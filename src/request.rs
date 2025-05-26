use crate::response::{InnerResp, JsResponse};
use actix_web::HttpRequest;
use bytes::Bytes;
use napi::bindgen_prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tokio::sync::oneshot;
use uuid::Uuid;

#[napi(object)]
#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
  pub r#type: String,
  #[napi(js_name = "originalName")]
  #[serde(rename = "originalName")]
  pub original_name: String,
  pub filename: String,
  pub path: String,
  #[napi(js_name = "contentType")]
  #[serde(rename = "contentType")]
  pub content_type: Option<String>,
  pub size: u32,
}

#[napi]
#[derive(Serialize)]
pub struct RequestWrapper {
  #[serde(skip)]
  request: HttpRequest,
  #[serde(skip)]
  body: Option<Bytes>,
  #[serde(skip)]
  path_params: HashMap<String, String>,
  #[serde(skip)]
  response_sender: Option<oneshot::Sender<JsResponse>>,
  #[serde(skip)]
  sent: bool,
  #[serde(skip)]
  status_code: Option<u16>,
  #[serde(skip)]
  headers: Vec<(String, String)>,
}

impl RequestWrapper {
  pub fn new(request: HttpRequest, body: Option<Bytes>) -> Self {
    Self {
      request,
      body,
      path_params: HashMap::new(),
      response_sender: None,
      sent: false,
      status_code: None,
      headers: Vec::new(),
    }
  }

  pub fn new_with_params(
    request: HttpRequest,
    body: Option<Bytes>,
    path_params: HashMap<String, String>,
  ) -> Self {
    Self {
      request,
      body,
      path_params,
      response_sender: None,
      sent: false,
      status_code: None,
      headers: Vec::new(),
    }
  }

  /// 设置响应发送器，用于异步响应
  pub fn set_response_sender(&mut self, sender: oneshot::Sender<JsResponse>) {
    self.response_sender = Some(sender);
  }

  /// 发送响应
  fn send_response(&mut self, inner: InnerResp) -> Result<()> {
    if self.sent {
      return Err(napi::Error::from_reason("响应已经发送"));
    }

    self.sent = true;

    if let Some(sender) = self.response_sender.take() {
      let response = JsResponse {
        inner,
        status_code: self.status_code,
        headers: if self.headers.is_empty() {
          None
        } else {
          Some(self.headers.clone())
        },
      };

      if sender.send(response).is_err() {
        eprintln!("警告：发送响应失败，接收器可能已经被丢弃");
      }
    }

    Ok(())
  }
}

#[napi]
impl RequestWrapper {
  #[napi]
  /// 获取请求路径
  pub fn get_path(&self) -> String {
    self.request.path().to_string()
  }

  #[napi]
  /// 获取请求方法
  pub fn get_method(&self) -> String {
    self.request.method().as_str().to_string()
  }

  #[napi]
  /// 获取查询字符串
  pub fn get_query_string(&self) -> String {
    self.request.query_string().to_string()
  }

  #[napi(ts_return_type = "{[key: string]: string}")]
  /// 获取查询参数作为对象
  pub fn get_query_params(&self) -> HashMap<String, String> {
    let query_string = self.request.query_string();
    if query_string.is_empty() {
      return HashMap::new();
    }

    serde_qs::from_str(query_string).unwrap_or_default()
  }

  #[napi]
  /// 获取原始请求体字符串
  pub fn get_body_string(&self) -> String {
    match &self.body {
      Some(bytes) => String::from_utf8_lossy(bytes).to_string(),
      None => String::new(),
    }
  }

  #[napi(ts_return_type = "{[key: string]: any}")]
  /// 尝试将请求体解析为JSON对象
  pub fn get_body_json(&self) -> Option<serde_json::Value> {
    match &self.body {
      Some(bytes) => {
        if let Ok(body_str) = std::str::from_utf8(bytes) {
          serde_json::from_str(body_str).ok()
        } else {
          None
        }
      }
      None => None,
    }
  }

  #[napi]
  /// 获取表单数据参数，支持 application/x-www-form-urlencoded 和 multipart/form-data 格式
  /// 对于文件字段，直接返回文件信息对象
  pub fn get_form_data(&self) -> serde_json::Value {
    // 检查 Content-Type
    let content_type = self
      .get_header("content-type".to_string())
      .unwrap_or_default()
      .to_lowercase();

    match &self.body {
      Some(bytes) => {
        if content_type.contains("application/x-www-form-urlencoded") {
          // 处理 URL 编码的表单数据
          if let Ok(body_str) = std::str::from_utf8(bytes) {
            let form_data: HashMap<String, String> =
              serde_qs::from_str(body_str).unwrap_or_default();
            // 转换为 JSON Value
            serde_json::to_value(form_data)
              .unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
          } else {
            serde_json::Value::Object(serde_json::Map::new())
          }
        } else if content_type.contains("multipart/form-data") {
          // 处理 multipart 表单数据，包括文件字段
          serde_json::to_value(self.parse_multipart_with_files(bytes, &content_type))
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
        } else {
          serde_json::Value::Object(serde_json::Map::new())
        }
      }
      None => serde_json::Value::Object(serde_json::Map::new()),
    }
  }

  /// 解析 multipart 数据，包括文本字段和文件字段
  fn parse_multipart_with_files(
    &self,
    bytes: &Bytes,
    content_type: &str,
  ) -> HashMap<String, serde_json::Value> {
    // 提取 boundary（保持原始大小写）
    let boundary = if let Some(boundary_start) = content_type.find("boundary=") {
      let boundary_str = &content_type[boundary_start + 9..];
      // 移除可能的引号和分号后的内容
      boundary_str
        .split(';')
        .next()
        .unwrap_or("")
        .trim_matches('"')
        .trim()
    } else {
      return HashMap::new();
    };

    if boundary.is_empty() {
      return HashMap::new();
    }

    let mut form_data: HashMap<String, serde_json::Value> = HashMap::new();
    let body_str = String::from_utf8_lossy(bytes);

    // 查找请求体中实际的 boundary（从第一行提取）
    let actual_boundary = if body_str.starts_with("--") {
      if let Some(first_line_end) = body_str.find("\r\n").or_else(|| body_str.find("\n")) {
        &body_str[2..first_line_end] // 去掉开头的 "--"
      } else {
        boundary
      }
    } else {
      boundary
    };

    let boundary_delimiter = format!("--{}", actual_boundary);

    // 分割各个部分
    let parts: Vec<&str> = body_str.split(&boundary_delimiter).collect();

    for part in parts.iter().skip(1) {
      if part.trim().is_empty() || part.starts_with("--") {
        continue;
      }

      // 尝试不同的换行符格式
      let header_end = part.find("\r\n\r\n").or_else(|| part.find("\n\n"));

      if let Some(header_end) = header_end {
        let headers = &part[..header_end];
        let content_start = if part[header_end..].starts_with("\r\n\r\n") {
          header_end + 4
        } else {
          header_end + 2
        };
        let content = &part[content_start..]
          .trim_end_matches("\r\n")
          .trim_end_matches("\n");

        // 解析 Content-Disposition 头
        if let Some(name) = self.extract_form_field_name(headers) {
          if headers.contains("filename=") {
            // 处理文件字段，保存到本地并返回文件信息
            if let Some(file_info) = self.save_uploaded_file(headers, content) {
              if let Ok(file_value) = serde_json::to_value(&file_info) {
                form_data.insert(name, file_value);
              }
            }
          } else {
            // 处理文本字段
            form_data.insert(name, serde_json::Value::String(content.to_string()));
          }
        }
      }
    }
    form_data
  }

  /// 从 Content-Disposition 头中提取字段名
  fn extract_form_field_name(&self, headers: &str) -> Option<String> {
    for line in headers.lines() {
      if line.to_lowercase().starts_with("content-disposition:") {
        if let Some(name_start) = line.find("name=\"") {
          let name_part = &line[name_start + 6..];
          if let Some(name_end) = name_part.find('"') {
            return Some(name_part[..name_end].to_string());
          }
        }
      }
    }
    None
  }

  /// 从 Content-Disposition 头中提取文件名
  fn extract_filename(&self, headers: &str) -> Option<String> {
    for line in headers.lines() {
      if line.to_lowercase().starts_with("content-disposition:") {
        if let Some(filename_start) = line.find("filename=\"") {
          let filename_part = &line[filename_start + 10..];
          if let Some(filename_end) = filename_part.find('"') {
            return Some(filename_part[..filename_end].to_string());
          }
        }
      }
    }
    None
  }

  /// 从头部中提取 Content-Type
  fn extract_content_type(&self, headers: &str) -> Option<String> {
    for line in headers.lines() {
      if line.to_lowercase().starts_with("content-type:") {
        return Some(line[13..].trim().to_string());
      }
    }
    None
  }

  /// 保存上传的文件到本地并返回文件信息
  fn save_uploaded_file(&self, headers: &str, content: &str) -> Option<FileInfo> {
    let original_filename = self.extract_filename(headers)?;
    let content_type = self.extract_content_type(headers);
    let file_size = content.len();

    // 确保 static 目录存在
    let static_dir = Path::new("static");
    if !static_dir.exists() {
      if let Err(e) = fs::create_dir_all(static_dir) {
        eprintln!("创建 static 目录失败: {}", e);
        return None;
      }
    }

    // 生成唯一文件名，保留原始扩展名
    let file_extension = Path::new(&original_filename)
      .extension()
      .and_then(|ext| ext.to_str())
      .unwrap_or("");

    let unique_filename = if file_extension.is_empty() {
      format!("{}", Uuid::new_v4())
    } else {
      format!("{}.{}", Uuid::new_v4(), file_extension)
    };

    let file_path = static_dir.join(&unique_filename);
    let relative_path = format!("static/{}", unique_filename);

    // 保存文件
    if let Err(e) = fs::write(&file_path, content.as_bytes()) {
      eprintln!("保存文件失败: {}", e);
      return None;
    }

    // 返回文件信息对象
    let file_info = FileInfo {
      r#type: "file".to_string(),
      original_name: original_filename,
      filename: unique_filename,
      path: relative_path,
      content_type,
      size: file_size as u32,
    };

    Some(file_info)
  }

  #[napi]
  /// 获取表单数据中指定键的值
  pub fn get_form_value(&self, key: String) -> Option<serde_json::Value> {
    if let serde_json::Value::Object(map) = self.get_form_data() {
      map.get(&key).cloned()
    } else {
      None
    }
  }

  #[napi]
  /// 获取指定的请求头
  pub fn get_header(&self, name: String) -> Option<String> {
    self
      .request
      .headers()
      .get(&name)
      .and_then(|value| value.to_str().ok())
      .map(|s| s.to_string())
  }

  #[napi(ts_return_type = "{[key: string]: string}")]
  /// 获取所有请求头
  pub fn get_headers(&self) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for (name, value) in self.request.headers() {
      if let Ok(value_str) = value.to_str() {
        headers.insert(name.as_str().to_string(), value_str.to_string());
      }
    }
    headers
  }

  #[napi(ts_return_type = "{[key: string]: string}")]
  /// 获取路径参数作为对象，例如路由 /api/test/:id 匹配请求 /api/test/123 时返回 {id: "123"}
  pub fn get_path_params(&self) -> HashMap<String, String> {
    self.path_params.clone()
  }

  #[napi]
  /// 获取指定名称的路径参数值
  pub fn get_path_param(&self, name: String) -> Option<String> {
    self.path_params.get(&name).cloned()
  }

  #[napi]
  /// 发送文本响应
  pub fn send_text(&mut self, text: String) -> Result<()> {
    self.send_response(InnerResp::Text(text))
  }

  #[napi]
  /// 发送JSON响应
  pub fn send_json(&mut self, json: String) -> Result<()> {
    self.send_response(InnerResp::Json(json))
  }

  #[napi]
  /// 发送对象作为JSON响应
  pub fn send_object(&mut self, obj: serde_json::Value) -> Result<()> {
    match serde_json::to_string(&obj) {
      Ok(json_string) => self.send_response(InnerResp::Json(json_string)),
      Err(e) => Err(napi::Error::from_reason(format!("JSON序列化失败: {}", e))),
    }
  }

  #[napi]
  /// 发送空响应
  pub fn send_empty(&mut self) -> Result<()> {
    self.send_response(InnerResp::EmptyString)
  }

  #[napi]
  /// 发送服务器错误响应
  pub fn send_error(&mut self, message: Option<String>) -> Result<()> {
    match message {
      Some(msg) => self.send_response(InnerResp::ServerErrorWithMessage(msg)),
      None => self.send_response(InnerResp::ServerError),
    }
  }

  #[napi]
  /// 设置响应状态码
  pub fn set_status_code(&mut self, status: u16) -> bool {
    if self.sent {
      return false;
    }

    if !(100..1000).contains(&status) {
      return false;
    }

    self.status_code = Some(status);
    true
  }

  #[napi]
  /// 添加响应头
  pub fn add_header(&mut self, key: String, value: String) {
    if !self.sent {
      self.headers.push((key, value));
    }
  }
}
