use actix_web::HttpRequest;
use bytes::Bytes;
#[allow(unused_imports)]
use napi::bindgen_prelude::*;
use serde::Serialize;
use std::collections::HashMap;

#[napi]
#[derive(Serialize)]
pub struct RequestWrapper {
  #[serde(skip)]
  request: HttpRequest,
  #[serde(skip)]
  body: Option<Bytes>,
  #[serde(skip)]
  path_params: HashMap<String, String>,
}

impl RequestWrapper {
  pub fn new(request: HttpRequest, body: Option<Bytes>) -> Self {
    Self {
      request,
      body,
      path_params: HashMap::new(),
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
    }
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
}
