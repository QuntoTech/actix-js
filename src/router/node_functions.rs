use actix_web::http::Method;
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use serde::Serialize;

use crate::request::RequestWrapper;
use crate::router::store::{add_new_route, cleanup_route};

// 定义请求数据结构
#[derive(Serialize)]
pub struct RequestData {
  pub method: String,
  pub path: String,
  pub query: String,
  pub params: serde_json::Map<String, serde_json::Value>,
}

// 使用RequestWrapper作为ThreadsafeFunction的类型，包装在Arc中以支持Clone
use std::sync::Arc;
pub type CallBackFunction = Arc<ThreadsafeFunction<RequestWrapper>>;

#[napi]
/// HTTP方法枚举
pub enum Methods {
  GET,
  POST,
  PUT,
  PATCH,
  DELETE,
}

impl Methods {
  #[inline(always)]
  pub fn convert_from_str(method: &str) -> Option<Self> {
    match method {
      "GET" => Some(Methods::GET),
      "POST" => Some(Methods::POST),
      "PUT" => Some(Methods::PUT),
      "PATCH" => Some(Methods::PATCH),
      "DELETE" => Some(Methods::DELETE),
      _ => None,
    }
  }

  #[inline(always)]
  pub fn convert_from_actix(method: Method) -> Option<Self> {
    match method {
      Method::GET => Some(Methods::GET),
      Method::POST => Some(Methods::POST),
      Method::PUT => Some(Methods::PUT),
      Method::PATCH => Some(Methods::PATCH),
      Method::DELETE => Some(Methods::DELETE),
      _ => None,
    }
  }
}

#[napi]
/// 注册新路由
pub fn new_route(
  route: String,
  method: Methods,
  callback: ThreadsafeFunction<RequestWrapper>,
) -> Result<()> {
  add_new_route(&route, method, Arc::new(callback))
}

#[napi]
/// 清理所有路由
pub fn cleanup_router() -> Result<()> {
  cleanup_route();
  Ok(())
}

#[napi]
/// 注册GET路由
pub fn get(route: String, callback: ThreadsafeFunction<RequestWrapper>) -> Result<()> {
  new_route(route, Methods::GET, callback)
}

#[napi]
/// 注册POST路由
pub fn post(route: String, callback: ThreadsafeFunction<RequestWrapper>) -> Result<()> {
  new_route(route, Methods::POST, callback)
}

#[napi]
/// 注册PUT路由
pub fn put(route: String, callback: ThreadsafeFunction<RequestWrapper>) -> Result<()> {
  new_route(route, Methods::PUT, callback)
}

#[napi]
/// 注册PATCH路由
pub fn patch(route: String, callback: ThreadsafeFunction<RequestWrapper>) -> Result<()> {
  new_route(route, Methods::PATCH, callback)
}

#[napi]
/// 注册DELETE路由
pub fn del(route: String, callback: ThreadsafeFunction<RequestWrapper>) -> Result<()> {
  new_route(route, Methods::DELETE, callback)
}

/// 执行JavaScript回调函数（带RequestWrapper）
pub fn execute_callback_with_request(callback: &CallBackFunction, request_wrapper: RequestWrapper) {
  // 添加调试信息
  println!(
    "尝试调用JavaScript回调，路径: {}, 方法: {}",
    request_wrapper.get_path(),
    request_wrapper.get_method()
  );

  // 使用正确的API调用ThreadsafeFunction
  match callback.call(Ok(request_wrapper), ThreadsafeFunctionCallMode::NonBlocking) {
    napi::Status::Ok => {
      println!("JavaScript回调调用成功");
    }
    status => {
      println!("JavaScript回调调用失败，状态: {:?}", status);
    }
  }
}

/// 执行JavaScript回调函数（兼容旧接口）
pub fn execute_callback(
  _callback: &CallBackFunction,
  _method: String,
  _path: String,
  _query: String,
) {
  // 创建一个临时的RequestWrapper（这里只是为了兼容，实际使用中应该传递真实的RequestWrapper）
  println!("警告：使用了旧的execute_callback接口，建议使用execute_callback_with_request");
  // 这里我们不能创建假的RequestWrapper，所以暂时保持空实现
}
