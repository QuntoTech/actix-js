use actix_web::http::Method;
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use serde::Serialize;

use crate::request::{DetachedRequestWrapper, RequestWrapper};
use crate::router::read_only::clear_route_cache;
use crate::router::store::{add_new_route, cleanup_route};

// 定义请求数据结构
#[derive(Serialize)]
pub struct RequestData {
  pub method: String,
  pub path: String,
  pub query: String,
  pub params: serde_json::Map<String, serde_json::Value>,
}

// 使用DetachedRequestWrapper作为ThreadsafeFunction的类型，包装在Arc中以支持Clone
use std::sync::Arc;
pub type CallBackFunction = Arc<ThreadsafeFunction<DetachedRequestWrapper>>;
pub type LegacyCallBackFunction = Arc<ThreadsafeFunction<RequestWrapper>>;

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
/// 注册新路由（异步版本）
pub fn new_route_async(
  route: String,
  method: Methods,
  callback: ThreadsafeFunction<DetachedRequestWrapper>,
) -> Result<()> {
  add_new_route(&route, method, Arc::new(callback))
}

#[napi]
/// 注册新路由（兼容旧版本）
pub fn new_route(
  _route: String,
  _method: Methods,
  _callback: ThreadsafeFunction<RequestWrapper>,
) -> Result<()> {
  // 这里需要转换为新的异步版本，暂时保持空实现
  Err(napi::Error::from_reason(
    "请使用 new_route_async 或异步路由注册方法",
  ))
}

#[napi]
/// 清理所有路由
pub fn cleanup_router() -> Result<()> {
  cleanup_route();
  Ok(())
}

#[napi]
/// 注册GET路由（异步版本）
pub fn get_async(
  route: String,
  callback: ThreadsafeFunction<DetachedRequestWrapper>,
) -> Result<()> {
  new_route_async(route, Methods::GET, callback)
}

#[napi]
/// 注册POST路由（异步版本）
pub fn post_async(
  route: String,
  callback: ThreadsafeFunction<DetachedRequestWrapper>,
) -> Result<()> {
  new_route_async(route, Methods::POST, callback)
}

#[napi]
/// 注册PUT路由（异步版本）
pub fn put_async(
  route: String,
  callback: ThreadsafeFunction<DetachedRequestWrapper>,
) -> Result<()> {
  new_route_async(route, Methods::PUT, callback)
}

#[napi]
/// 注册PATCH路由（异步版本）
pub fn patch_async(
  route: String,
  callback: ThreadsafeFunction<DetachedRequestWrapper>,
) -> Result<()> {
  new_route_async(route, Methods::PATCH, callback)
}

#[napi]
/// 注册DELETE路由（异步版本）
pub fn del_async(
  route: String,
  callback: ThreadsafeFunction<DetachedRequestWrapper>,
) -> Result<()> {
  new_route_async(route, Methods::DELETE, callback)
}

#[napi]
/// 注册GET路由（兼容旧版本）
pub fn get(route: String, callback: ThreadsafeFunction<RequestWrapper>) -> Result<()> {
  new_route(route, Methods::GET, callback)
}

#[napi]
/// 注册POST路由（兼容旧版本）
pub fn post(route: String, callback: ThreadsafeFunction<RequestWrapper>) -> Result<()> {
  new_route(route, Methods::POST, callback)
}

#[napi]
/// 注册PUT路由（兼容旧版本）
pub fn put(route: String, callback: ThreadsafeFunction<RequestWrapper>) -> Result<()> {
  new_route(route, Methods::PUT, callback)
}

#[napi]
/// 注册PATCH路由（兼容旧版本）
pub fn patch(route: String, callback: ThreadsafeFunction<RequestWrapper>) -> Result<()> {
  new_route(route, Methods::PATCH, callback)
}

#[napi]
/// 注册DELETE路由（兼容旧版本）
pub fn del(route: String, callback: ThreadsafeFunction<RequestWrapper>) -> Result<()> {
  new_route(route, Methods::DELETE, callback)
}

/// 执行JavaScript回调函数（带DetachedRequestWrapper - 异步版本）
pub fn execute_callback_with_detached_request(
  callback: &CallBackFunction,
  request_wrapper: DetachedRequestWrapper,
) {
  // 使用正确的API调用ThreadsafeFunction
  match callback.call(Ok(request_wrapper), ThreadsafeFunctionCallMode::NonBlocking) {
    napi::Status::Ok => {
      // 回调调用成功
    }
    status => {
      eprintln!("JavaScript回调调用失败，状态: {:?}", status);
    }
  }
}

/// 执行JavaScript回调函数（带RequestWrapper - 兼容旧版本）
pub fn execute_callback_with_request(
  callback: &LegacyCallBackFunction,
  request_wrapper: RequestWrapper,
) {
  // 使用正确的API调用ThreadsafeFunction
  match callback.call(Ok(request_wrapper), ThreadsafeFunctionCallMode::NonBlocking) {
    napi::Status::Ok => {
      // 回调调用成功
    }
    status => {
      eprintln!("JavaScript回调调用失败，状态: {:?}", status);
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

// 🚀 新增：LRU缓存管理接口

#[napi]
/// 清理路由缓存 - 在需要强制刷新缓存时调用
pub fn clear_router_cache() -> Result<()> {
  clear_route_cache();
  Ok(())
}
