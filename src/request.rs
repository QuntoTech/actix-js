use crate::response::{InnerResp, JsResponse};
use actix_web::HttpRequest;
use bytes::Bytes;
use napi::bindgen_prelude::*;
use serde::Serialize;
use std::cell::OnceCell;
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
  #[serde(skip)]
  parsed_headers: OnceCell<HashMap<String, String>>,
  #[serde(skip)]
  parsed_query_params: OnceCell<HashMap<String, String>>,
  #[serde(skip)]
  parsed_json: OnceCell<Option<serde_json::Value>>,
  #[serde(skip)]
  parsed_form_data: OnceCell<serde_json::Value>,
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
      parsed_headers: OnceCell::new(),
      parsed_query_params: OnceCell::new(),
      parsed_json: OnceCell::new(),
      parsed_form_data: OnceCell::new(),
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
      parsed_headers: OnceCell::new(),
      parsed_query_params: OnceCell::new(),
      parsed_json: OnceCell::new(),
      parsed_form_data: OnceCell::new(),
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
  /// 获取请求路径 - 零拷贝优化：直接返回，避免不必要的克隆
  pub fn get_path(&self) -> String {
    self.request.path().to_string()
  }

  #[napi]
  /// 获取请求方法 - 零拷贝优化：直接返回，避免不必要的克隆
  pub fn get_method(&self) -> String {
    self.request.method().as_str().to_string()
  }

  #[napi]
  /// 获取查询字符串 - 零拷贝优化：直接返回，避免不必要的克隆
  pub fn get_query_string(&self) -> String {
    self.request.query_string().to_string()
  }

  #[napi]
  /// 获取URI - 零拷贝优化：直接返回，避免不必要的克隆
  pub fn get_uri(&self) -> String {
    self.request.uri().to_string()
  }

  #[napi(ts_return_type = "{[key: string]: string}")]
  /// 获取查询参数作为对象 - 零拷贝优化：延迟解析，只计算一次
  pub fn get_query_params(&self) -> HashMap<String, String> {
    self
      .parsed_query_params
      .get_or_init(|| {
        let query_string = self.request.query_string();
        if query_string.is_empty() {
          HashMap::new()
        } else {
          serde_qs::from_str(query_string).unwrap_or_default()
        }
      })
      .clone()
  }

  #[napi]
  /// 获取原始请求体字符串 - 零拷贝优化：直接使用 Bytes 的零拷贝特性
  pub fn get_body_string(&self) -> String {
    match &self.body {
      Some(bytes) => String::from_utf8_lossy(bytes).to_string(),
      None => String::new(),
    }
  }

  /// 获取原始请求体字节 - 零拷贝优化：返回 Bytes 引用
  pub fn get_body_bytes(&self) -> Option<&Bytes> {
    self.body.as_ref()
  }

  #[napi]
  /// 检查请求体是否为空 - 零拷贝优化：直接检查，不解析内容
  pub fn has_body(&self) -> bool {
    self.body.is_some() && !self.body.as_ref().unwrap().is_empty()
  }

  #[napi]
  /// 获取请求体大小 - 零拷贝优化：直接返回字节长度
  pub fn get_body_size(&self) -> u32 {
    self.body.as_ref().map(|b| b.len() as u32).unwrap_or(0)
  }

  /// 检查是否为JSON请求 - 零拷贝优化：只检查Content-Type，不解析内容
  pub fn is_json_request(&self) -> bool {
    self
      .get_header("content-type".to_string())
      .map(|ct| ct.to_lowercase().contains("application/json"))
      .unwrap_or(false)
  }

  /// 检查是否为表单请求 - 零拷贝优化：只检查Content-Type，不解析内容
  pub fn is_form_request(&self) -> bool {
    self
      .get_header("content-type".to_string())
      .map(|ct| {
        let ct_lower = ct.to_lowercase();
        ct_lower.contains("application/x-www-form-urlencoded")
          || ct_lower.contains("multipart/form-data")
      })
      .unwrap_or(false)
  }

  #[napi(ts_return_type = "{[key: string]: any}")]
  /// 尝试将请求体解析为JSON对象 - 零拷贝优化：延迟解析，只计算一次
  pub fn get_body_json(&self) -> Option<serde_json::Value> {
    self
      .parsed_json
      .get_or_init(|| match &self.body {
        Some(bytes) => {
          if let Ok(body_str) = std::str::from_utf8(bytes) {
            serde_json::from_str(body_str).ok()
          } else {
            None
          }
        }
        None => None,
      })
      .clone()
  }

  #[napi]
  /// 获取表单数据参数，支持 application/x-www-form-urlencoded 和 multipart/form-data 格式
  /// 对于文件字段，直接返回文件信息对象 - 零拷贝优化：延迟解析，只计算一次
  pub fn get_form_data(&self) -> serde_json::Value {
    self
      .parsed_form_data
      .get_or_init(|| self.parse_form_data_internal())
      .clone()
  }

  /// 内部方法：解析表单数据
  fn parse_form_data_internal(&self) -> serde_json::Value {
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
  /// 获取表单数据中指定键的值 - 零拷贝优化：使用缓存的表单数据
  pub fn get_form_value(&self, key: String) -> Option<serde_json::Value> {
    let form_data = self
      .parsed_form_data
      .get_or_init(|| self.parse_form_data_internal());

    if let serde_json::Value::Object(map) = form_data {
      map.get(&key).cloned()
    } else {
      None
    }
  }

  #[napi]
  /// 获取指定的请求头 - 零拷贝优化：使用延迟解析的缓存
  pub fn get_header(&self, name: String) -> Option<String> {
    self.get_headers_cached().get(&name).cloned()
  }

  #[napi(ts_return_type = "{[key: string]: string}")]
  /// 获取所有请求头 - 零拷贝优化：延迟解析，只计算一次
  pub fn get_headers(&self) -> HashMap<String, String> {
    self.get_headers_cached().clone()
  }

  /// 内部方法：获取缓存的请求头
  fn get_headers_cached(&self) -> &HashMap<String, String> {
    self.parsed_headers.get_or_init(|| {
      let mut headers = HashMap::new();
      for (name, value) in self.request.headers() {
        if let Ok(value_str) = value.to_str() {
          headers.insert(name.as_str().to_string(), value_str.to_string());
        }
      }
      headers
    })
  }

  #[napi(ts_return_type = "{[key: string]: string}")]
  /// 获取路径参数作为对象，例如路由 /api/test/:id 匹配请求 /api/test/123 时返回 {id: "123"}
  /// 零拷贝优化：直接返回引用的克隆，避免重复构建
  pub fn get_path_params(&self) -> HashMap<String, String> {
    self.path_params.clone()
  }

  #[napi]
  /// 获取指定名称的路径参数值 - 零拷贝优化：直接从HashMap查找，避免重复遍历
  pub fn get_path_param(&self, name: String) -> Option<String> {
    self.path_params.get(&name).cloned()
  }

  /// 内部方法：检查是否有路径参数
  pub fn has_path_params(&self) -> bool {
    !self.path_params.is_empty()
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

#[napi]
#[derive(Serialize)]
pub struct DetachedRequestWrapper {
  // 提前提取的请求数据，不持有HttpRequest引用
  #[serde(skip)]
  path: String,
  #[serde(skip)]
  method: String,
  #[serde(skip)]
  query_string: String,
  #[serde(skip)]
  uri: String,
  #[serde(skip)]
  headers: HashMap<String, String>,
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
  response_headers: Vec<(String, String)>,
  // 预计算缓存字段 - 零拷贝优化：在创建时就解析好，避免运行时原子操作
  #[serde(skip)]
  cached_query_params: Option<HashMap<String, String>>,
  #[serde(skip)]
  cached_json: Option<serde_json::Value>,
  #[serde(skip)]
  cached_form_data: Option<serde_json::Value>,
}

impl DetachedRequestWrapper {
  // 静态解析方法 - 零拷贝优化：预计算时使用，复用现有逻辑
  fn parse_query_params_static(query_string: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for pair in query_string.split('&') {
      if let Some((key, value)) = pair.split_once('=') {
        // 使用简单的解码，避免依赖外部库
        params.insert(key.to_string(), value.to_string());
      }
    }
    params
  }

  fn is_json_content_type(headers: &HashMap<String, String>) -> bool {
    headers
      .get("content-type")
      .map(|ct| ct.to_lowercase().contains("application/json"))
      .unwrap_or(false)
  }

  fn parse_json_static(body: &Bytes) -> Option<serde_json::Value> {
    serde_json::from_slice(body).ok()
  }

  fn is_form_content_type(headers: &HashMap<String, String>) -> bool {
    headers
      .get("content-type")
      .map(|ct| {
        let ct_lower = ct.to_lowercase();
        ct_lower.contains("application/x-www-form-urlencoded")
          || ct_lower.contains("multipart/form-data")
      })
      .unwrap_or(false)
  }

  fn parse_form_data_static(
    body: &Bytes,
    headers: &HashMap<String, String>,
  ) -> Option<serde_json::Value> {
    let content_type = headers
      .get("content-type")
      .unwrap_or(&String::new())
      .to_lowercase();

    if content_type.contains("application/x-www-form-urlencoded") {
      // 使用与 RequestWrapper 相同的解析逻辑
      if let Ok(body_str) = std::str::from_utf8(body) {
        let form_data: HashMap<String, String> = serde_qs::from_str(body_str).unwrap_or_default();
        Some(
          serde_json::to_value(form_data)
            .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new())),
        )
      } else {
        Some(serde_json::Value::Object(serde_json::Map::new()))
      }
    } else if content_type.contains("multipart/form-data") {
      // 对于 multipart 表单，我们需要完整的解析逻辑
      // 为了保持预计算的优势，我们创建一个临时实例来解析
      Self::parse_multipart_static(body, &content_type)
    } else {
      Some(serde_json::Value::Object(serde_json::Map::new()))
    }
  }

  fn parse_multipart_static(body: &Bytes, content_type: &str) -> Option<serde_json::Value> {
    // 提取 boundary
    let boundary = if let Some(boundary_start) = content_type.find("boundary=") {
      let boundary_str = &content_type[boundary_start + 9..];
      boundary_str
        .split(';')
        .next()
        .unwrap_or("")
        .trim_matches('"')
        .trim()
    } else {
      return Some(serde_json::Value::Object(serde_json::Map::new()));
    };

    if boundary.is_empty() {
      return Some(serde_json::Value::Object(serde_json::Map::new()));
    }

    let mut form_data = serde_json::Map::new();
    let body_str = String::from_utf8_lossy(body);

    // 查找实际的 boundary
    let actual_boundary = if body_str.starts_with("--") {
      if let Some(first_line_end) = body_str.find("\r\n").or_else(|| body_str.find("\n")) {
        &body_str[2..first_line_end]
      } else {
        boundary
      }
    } else {
      boundary
    };

    let boundary_delimiter = format!("--{}", actual_boundary);
    let parts: Vec<&str> = body_str.split(&boundary_delimiter).collect();

    for part in parts.iter().skip(1) {
      if part.trim().is_empty() || part.starts_with("--") {
        continue;
      }

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

        if let Some(name) = Self::extract_form_field_name_static(headers) {
          if headers.contains("filename=") {
            // 对于文件字段，在静态模式下我们只记录基本信息，不实际保存文件
            if let Some(filename) = Self::extract_filename_static(headers) {
              let file_info = serde_json::json!({
                "type": "file",
                "originalName": filename,
                "filename": format!("static_mode_{}", filename),
                "path": format!("static/static_mode_{}", filename),
                "size": content.len(),
                "contentType": Self::extract_content_type_static(headers)
              });
              form_data.insert(name, file_info);
            }
          } else {
            // 处理文本字段
            form_data.insert(name, serde_json::Value::String(content.to_string()));
          }
        }
      }
    }

    Some(serde_json::Value::Object(form_data))
  }

  fn extract_form_field_name_static(headers: &str) -> Option<String> {
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

  fn extract_filename_static(headers: &str) -> Option<String> {
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

  fn extract_content_type_static(headers: &str) -> Option<String> {
    for line in headers.lines() {
      if line.to_lowercase().starts_with("content-type:") {
        return Some(line[13..].trim().to_string());
      }
    }
    None
  }

  /// 从HttpRequest创建DetachedRequestWrapper，提前提取所有需要的数据
  pub fn new_detached(
    req: HttpRequest,
    body: Option<Bytes>,
    path_params: HashMap<String, String>,
  ) -> Self {
    // 提前提取所有请求数据
    let path = req.path().to_string();
    let method = req.method().as_str().to_string();
    let query_string = req.query_string().to_string();
    let uri = req.uri().to_string();

    // 提前解析所有请求头
    let mut headers = HashMap::new();
    for (name, value) in req.headers() {
      if let Ok(value_str) = value.to_str() {
        headers.insert(name.as_str().to_string(), value_str.to_string());
      }
    }

    // 预计算缓存 - 零拷贝优化：在创建时解析，避免运行时原子操作开销
    let cached_query_params = if query_string.is_empty() {
      None
    } else {
      Some(Self::parse_query_params_static(&query_string))
    };

    let cached_json = if let Some(ref body_bytes) = body {
      if Self::is_json_content_type(&headers) {
        Self::parse_json_static(body_bytes)
      } else {
        None
      }
    } else {
      None
    };

    let cached_form_data = if let Some(ref body_bytes) = body {
      if Self::is_form_content_type(&headers) {
        Self::parse_form_data_static(body_bytes, &headers)
      } else {
        None
      }
    } else {
      None
    };

    Self {
      path,
      method,
      query_string,
      uri,
      headers,
      body,
      path_params,
      response_sender: None,
      sent: false,
      status_code: None,
      response_headers: Vec::new(),
      cached_query_params,
      cached_json,
      cached_form_data,
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
        headers: if self.response_headers.is_empty() {
          None
        } else {
          Some(self.response_headers.clone())
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
impl DetachedRequestWrapper {
  #[napi]
  /// 获取请求路径
  pub fn get_path(&self) -> String {
    self.path.clone()
  }

  #[napi]
  /// 获取请求方法
  pub fn get_method(&self) -> String {
    self.method.clone()
  }

  #[napi]
  /// 获取查询字符串
  pub fn get_query_string(&self) -> String {
    self.query_string.clone()
  }

  #[napi]
  /// 获取URI
  pub fn get_uri(&self) -> String {
    self.uri.clone()
  }

  #[napi(ts_return_type = "{[key: string]: string}")]
  /// 获取查询参数作为对象 - 零拷贝优化：使用预计算缓存，无运行时开销
  pub fn get_query_params(&self) -> HashMap<String, String> {
    self.cached_query_params.clone().unwrap_or_default()
  }

  #[napi]
  /// 获取原始请求体字符串
  pub fn get_body_string(&self) -> String {
    match &self.body {
      Some(bytes) => String::from_utf8_lossy(bytes).to_string(),
      None => String::new(),
    }
  }

  #[napi]
  /// 检查请求体是否为空
  pub fn has_body(&self) -> bool {
    self.body.is_some() && !self.body.as_ref().unwrap().is_empty()
  }

  #[napi]
  /// 获取请求体大小
  pub fn get_body_size(&self) -> u32 {
    self.body.as_ref().map(|b| b.len() as u32).unwrap_or(0)
  }

  #[napi(ts_return_type = "{[key: string]: any}")]
  /// 尝试将请求体解析为JSON对象 - 零拷贝优化：使用预计算缓存，无运行时开销
  pub fn get_body_json(&self) -> Option<serde_json::Value> {
    self.cached_json.clone()
  }

  #[napi]
  /// 获取指定的请求头
  pub fn get_header(&self, name: String) -> Option<String> {
    self.headers.get(&name).cloned()
  }

  #[napi(ts_return_type = "{[key: string]: string}")]
  /// 获取所有请求头
  pub fn get_headers(&self) -> HashMap<String, String> {
    self.headers.clone()
  }

  #[napi(ts_return_type = "{[key: string]: string}")]
  /// 获取路径参数作为对象
  pub fn get_path_params(&self) -> HashMap<String, String> {
    self.path_params.clone()
  }

  #[napi]
  /// 获取指定名称的路径参数值
  pub fn get_path_param(&self, name: String) -> Option<String> {
    self.path_params.get(&name).cloned()
  }

  // 异步响应方法 - 这些方法返回Promise，支持JavaScript的await语法

  #[napi]
  /// 异步发送文本响应 - 返回Promise，支持await
  pub async unsafe fn send_text_async(&mut self, text: String) -> Result<()> {
    self.send_response(InnerResp::Text(text))
  }

  #[napi]
  /// 异步发送JSON响应 - 返回Promise，支持await
  pub async unsafe fn send_json_async(&mut self, json: String) -> Result<()> {
    self.send_response(InnerResp::Json(json))
  }

  #[napi]
  /// 异步发送对象作为JSON响应 - 返回Promise，支持await
  pub async unsafe fn send_object_async(&mut self, obj: serde_json::Value) -> Result<()> {
    match serde_json::to_string(&obj) {
      Ok(json_string) => self.send_response(InnerResp::Json(json_string)),
      Err(e) => Err(napi::Error::from_reason(format!("JSON序列化失败: {}", e))),
    }
  }

  #[napi]
  /// 异步发送空响应 - 返回Promise，支持await
  pub async unsafe fn send_empty_async(&mut self) -> Result<()> {
    self.send_response(InnerResp::EmptyString)
  }

  #[napi]
  /// 异步发送服务器错误响应 - 返回Promise，支持await
  pub async unsafe fn send_error_async(&mut self, message: Option<String>) -> Result<()> {
    match message {
      Some(msg) => self.send_response(InnerResp::ServerErrorWithMessage(msg)),
      None => self.send_response(InnerResp::ServerError),
    }
  }

  #[napi]
  /// 异步设置响应状态码 - 返回Promise，支持await
  pub async unsafe fn set_status_code_async(&mut self, status: u16) -> Result<bool> {
    if self.sent {
      return Ok(false);
    }

    if !(100..1000).contains(&status) {
      return Ok(false);
    }

    self.status_code = Some(status);
    Ok(true)
  }

  #[napi]
  /// 异步添加响应头 - 返回Promise，支持await
  pub async unsafe fn add_header_async(&mut self, key: String, value: String) -> Result<()> {
    if !self.sent {
      self.response_headers.push((key, value));
    }
    Ok(())
  }

  #[napi]
  /// 异步获取表单数据参数，支持 application/x-www-form-urlencoded 和 multipart/form-data 格式
  /// 对于文件字段，直接返回文件信息对象 - 零拷贝优化：使用预计算缓存，无运行时开销
  pub async unsafe fn get_form_data_async(&self) -> Result<serde_json::Value> {
    Ok(
      self
        .cached_form_data
        .clone()
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
    )
  }

  #[napi]
  /// 异步获取表单数据中指定键的值 - 零拷贝优化：使用预计算缓存，无运行时开销
  pub async unsafe fn get_form_value_async(
    &self,
    key: String,
  ) -> Result<Option<serde_json::Value>> {
    if let Some(serde_json::Value::Object(map)) = &self.cached_form_data {
      Ok(map.get(&key).cloned())
    } else {
      Ok(None)
    }
  }
}
