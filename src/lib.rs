#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use napi::Result;
use parking_lot::Mutex;
use std::net::TcpListener;
use std::sync::Arc;

// 导入router模块
mod router;
pub use router::*;

// 导入request模块
mod request;
pub use request::*;

// 导入response模块
mod response;
pub use response::*;

// 使用系统默认分配器
// #[global_allocator]
// static GLOBAL: MiMalloc = MiMalloc;

// 服务器句柄类型
type ServerHandle = Option<actix_web::dev::ServerHandle>;

#[napi(object)]
pub struct ServerOptions {
  pub host: String,
  pub port: u16,
}

#[napi]
pub struct Server {
  options: ServerOptions,
  // 使用Arc<Mutex>来存储服务器句柄，这样可以在多线程间安全共享
  handle: Arc<Mutex<ServerHandle>>,
}

#[napi]
impl Server {
  #[napi(constructor)]
  pub fn new(options: ServerOptions) -> Self {
    Server {
      options,
      handle: Arc::new(Mutex::new(None)),
    }
  }

  #[napi]
  pub fn start(&self) -> Result<String> {
    let host = self.options.host.clone();
    let port = self.options.port;

    // 检查端口是否可用
    if let Err(e) = TcpListener::bind(format!("{}:{}", &host, port)) {
      return Err(napi::Error::from_reason(format!(
        "无法绑定到 {}:{}：{}",
        &host, port, e
      )));
    }

    // 初始化路由读取器
    router::store::initialise_reader();

    let handle_clone = self.handle.clone();
    let host_clone = host.clone();

    // 使用tokio运行时启动服务器
    napi::tokio::spawn(async move {
      let server = HttpServer::new(|| {
        App::new()
          .wrap(middleware::Logger::default())
          // 所有路由都通过动态路由处理器处理
          .default_service(web::route().to(handle_dynamic_route))
      })
      .bind(format!("{}:{}", host_clone, port))
      .unwrap()
      .run();

      // 存储服务器句柄
      {
        let mut handle_lock = handle_clone.lock();
        *handle_lock = Some(server.handle());
      }

      println!("✅ 服务器已启动：http://{}:{}", host_clone, port);

      // 运行服务器
      if let Err(e) = server.await {
        eprintln!("❌ 服务器错误: {}", e);
      }
    });

    Ok(format!("服务器已启动：http://{}:{}", host, port))
  }

  #[napi]
  pub fn stop(&self) -> Result<String> {
    let mut handle_lock = self.handle.lock();
    if let Some(handle) = handle_lock.take() {
      napi::tokio::spawn(async move {
        handle.stop(true).await;
      });
      Ok("服务器已停止".to_string())
    } else {
      Err(napi::Error::from_reason("服务器未运行"))
    }
  }
}

// 动态路由处理函数
async fn handle_dynamic_route(req: HttpRequest, body: web::Bytes) -> HttpResponse {
  let path = req.path();
  let method = req.method().clone();

  // 尝试从动态路由中查找回调函数
  if let Some(callback) = router::read_only::get_route(path, method.clone()) {
    // 获取路径参数并转换为std::collections::HashMap
    let path_params = router::read_only::get_params(path, method.clone())
      .map(|params| {
        params
          .into_iter()
          .collect::<std::collections::HashMap<String, String>>()
      })
      .unwrap_or_default();

    // 创建oneshot channel用于接收响应
    let (tx, rx) = tokio::sync::oneshot::channel::<JsResponse>();

    // 创建带路径参数和响应发送器的RequestWrapper
    let mut request_wrapper = RequestWrapper::new_with_params(req, Some(body), path_params);
    request_wrapper.set_response_sender(tx);

    // 执行JavaScript回调，传递RequestWrapper
    router::node_functions::execute_callback_with_request(callback, request_wrapper);

    // 等待JavaScript回调发送响应，设置5秒超时
    match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
      Ok(Ok(js_response)) => {
        // 将JsResponse转换为HttpResponse
        js_response.into_http_response()
      }
      Ok(Err(_)) => {
        // 发送器被丢弃，说明JavaScript代码没有发送响应
        HttpResponse::InternalServerError()
          .content_type("application/json")
          .body(r#"{"error": "JavaScript callback did not send response"}"#)
      }
      Err(_) => {
        // 超时
        HttpResponse::RequestTimeout()
          .content_type("application/json")
          .body(r#"{"error": "Request timeout - JavaScript callback took too long"}"#)
      }
    }
  } else {
    // 路由未找到
    HttpResponse::NotFound()
      .content_type("application/json")
      .body(format!(
        r#"{{"error": "Route not found", "path": "{}"}}"#,
        path
      ))
  }
}

// 简单测试函数
#[napi]
pub fn sum(a: i32, b: i32) -> i32 {
  a + b
}
