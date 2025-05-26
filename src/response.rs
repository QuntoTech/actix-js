use actix_web::{
  http::{header::HeaderValue, StatusCode},
  HttpResponse,
};
use bytes::Bytes;

/// 响应的内部表示，类似参考项目的InnerResp
#[derive(Debug)]
pub enum InnerResp {
  Text(String),
  Json(String),
  Raw(Bytes),
  EmptyString,
  ServerError,
  ServerErrorWithMessage(String),
}

/// JavaScript响应对象，包含响应数据、状态码和头部信息
#[derive(Debug)]
pub struct JsResponse {
  pub inner: InnerResp,
  pub status_code: Option<u16>,
  pub headers: Option<Vec<(String, String)>>,
}

impl JsResponse {
  /// 将JsResponse转换为actix-web的HttpResponse
  pub fn into_http_response(self) -> HttpResponse {
    let status = self.get_status_code();

    let mut builder = HttpResponse::build(status);

    // 设置内容类型和应用自定义头部
    match &self.inner {
      InnerResp::Text(_) | InnerResp::EmptyString => {
        builder.content_type("text/plain; charset=utf-8");
      }
      InnerResp::Json(_) => {
        builder.content_type("application/json; charset=utf-8");
      }
      InnerResp::Raw(_) => {
        builder.content_type("application/octet-stream");
      }
      InnerResp::ServerError | InnerResp::ServerErrorWithMessage(_) => {
        return HttpResponse::InternalServerError()
          .content_type("text/plain")
          .body(match self.inner {
            InnerResp::ServerError => "Internal Server Error".to_string(),
            InnerResp::ServerErrorWithMessage(msg) => msg,
            _ => unreachable!(),
          });
      }
    }

    // 应用自定义头部
    if let Some(headers) = &self.headers {
      for (key, value) in headers {
        if let Ok(header_value) = HeaderValue::from_str(value) {
          builder.append_header((key.as_str(), header_value));
        }
      }
    }

    // 根据响应类型创建响应体
    match self.inner {
      InnerResp::Text(text) => builder.body(text),
      InnerResp::Json(json) => builder.body(json),
      InnerResp::Raw(bytes) => builder.body(bytes),
      InnerResp::EmptyString => builder.body(""),
      _ => unreachable!(), // 这些情况在上面已经处理过了
    }
  }

  /// 获取状态码
  fn get_status_code(&self) -> StatusCode {
    match self.status_code {
      Some(code) => StatusCode::from_u16(code).unwrap_or(StatusCode::OK),
      None => StatusCode::OK,
    }
  }
}
