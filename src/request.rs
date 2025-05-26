use crate::response::{InnerResp, JsResponse};
use actix_web::HttpRequest;
use bytes::Bytes;
#[allow(unused_imports)]
use napi::bindgen_prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use tokio::sync::oneshot;

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
