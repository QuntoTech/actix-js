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

// 🚀 导入 JSON 优化模块
mod json_optimizer;
pub use json_optimizer::*;

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

    // 使用napi的runtime检查来确保在正确的上下文中运行
    napi::bindgen_prelude::within_runtime_if_available(|| {
      napi::tokio::spawn(async move {
        let server = HttpServer::new(|| {
          App::new()
            // .wrap(middleware::Logger::default())
            // 所有路由都通过动态路由处理器处理
            .default_service(web::route().to(handle_dynamic_route))
        })
        // .workers(1)
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
    });

    Ok(format!("服务器已启动：http://{}:{}", host, port))
  }

  #[napi]
  pub async fn stop(&self) -> Result<String> {
    // 先取出handle，避免在持有锁时await
    let handle = {
      let mut handle_lock = self.handle.lock();
      handle_lock.take()
    };

    if let Some(handle) = handle {
      // 直接await服务器停止，确保完全停止后才返回
      handle.stop(true).await;
      println!("✅ 服务器已完全停止");
      Ok("服务器已停止".to_string())
    } else {
      Err(napi::Error::from_reason("服务器未运行"))
    }
  }
}

// 动态路由处理函数 - 异步优化版本
async fn handle_dynamic_route(req: HttpRequest, body: web::Bytes) -> HttpResponse {
  let path = req.path();
  let method = req.method().clone();

  // 🚀 优化：一次性获取回调函数和路径参数，避免重复路由匹配
  if let Some((callback, path_params)) = router::read_only::get_route_with_params(path, method) {
    // 创建oneshot channel用于接收响应
    let (tx, rx) = tokio::sync::oneshot::channel::<JsResponse>();

    // 🚀 关键优化：使用DetachedRequestWrapper，避免BorrowMutError
    // 提前提取所有请求数据，不持有HttpRequest引用
    let mut detached_wrapper = DetachedRequestWrapper::new_detached(req, Some(body), path_params);
    detached_wrapper.set_response_sender(tx);

    // 🚀 异步执行JavaScript回调，不阻塞Rust主线程
    // JavaScript回调现在可以使用async/await语法
    router::node_functions::execute_callback_with_detached_request(callback, detached_wrapper);

    // 🚀 非阻塞等待：Rust主线程立即返回，JavaScript异步处理
    // 设置合理的超时时间，但不阻塞其他请求
    match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
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
        // 超时 - 增加到10秒，给异步处理更多时间
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

// 强制清理所有资源的函数
#[napi]
pub fn force_cleanup() -> Result<()> {
  // 清理所有路由
  router::store::cleanup_route();

  // 等待一小段时间让清理完成
  std::thread::sleep(std::time::Duration::from_millis(100));

  Ok(())
}

// 强制退出进程（最后手段）
#[napi]
pub fn force_exit() -> Result<()> {
  // 在新线程中延迟退出，给当前函数返回的时间
  std::thread::spawn(|| {
    std::thread::sleep(std::time::Duration::from_millis(100));
    std::process::exit(0);
  });
  Ok(())
}

// 简单测试函数
#[napi]
pub fn sum(a: i32, b: i32) -> i32 {
  a + b
}
