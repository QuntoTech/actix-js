use actix_cors::Cors;
use actix_web::{
    middleware, web, App, HttpResponse, HttpServer, Result,
};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
struct JsonResponse {
    message: String,
    timestamp: u64,
}

#[derive(Serialize)]
struct EchoResponse {
    echo: serde_json::Value,
    timestamp: u64,
}

#[derive(Serialize)]
struct ReceivedResponse {
    received: serde_json::Value,
    timestamp: u64,
}

fn get_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

// GET / - 返回 "Hello World"
async fn hello() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        .body("Hello World"))
}

// GET /json - 返回JSON响应
async fn json_handler() -> Result<HttpResponse> {
    let response = JsonResponse {
        message: "Hello JSON".to_string(),
        timestamp: get_timestamp(),
    };
    
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .json(response))
}

// POST /echo - 回显请求体
async fn echo_handler(body: web::Bytes) -> Result<HttpResponse> {
    let body_value = if body.is_empty() {
        serde_json::Value::Null
    } else {
        // 尝试解析为JSON，如果失败则作为字符串处理
        match serde_json::from_slice::<serde_json::Value>(&body) {
            Ok(json) => json,
            Err(_) => {
                // 如果不是有效JSON，将其作为字符串返回
                match String::from_utf8(body.to_vec()) {
                    Ok(s) => serde_json::Value::String(s),
                    Err(_) => serde_json::Value::Null,
                }
            }
        }
    };

    let response = EchoResponse {
        echo: body_value,
        timestamp: get_timestamp(),
    };
    
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .json(response))
}

// POST /json - 处理JSON请求
async fn json_post_handler(payload: web::Json<serde_json::Value>) -> Result<HttpResponse> {
    let response = ReceivedResponse {
        received: payload.into_inner(),
        timestamp: get_timestamp(),
    };
    
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .json(response))
}

// 错误处理函数
async fn not_found() -> Result<HttpResponse> {
    Ok(HttpResponse::NotFound()
        .content_type("application/json")
        .json(serde_json::json!({
            "error": "Not Found",
            "timestamp": get_timestamp()
        })))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let host = "127.0.0.1";
    let port = 3002;

    println!("Starting Native Actix-Web server on http://{}:{}", host, port);

    HttpServer::new(|| {
        // 配置CORS
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            // .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .route("/", web::get().to(hello))
            .route("/json", web::get().to(json_handler))
            .route("/echo", web::post().to(echo_handler))
            .route("/json", web::post().to(json_post_handler))
            .default_service(web::route().to(not_found))
    })
    // .workers(1)
    .bind((host, port))?
    .run()
    .await
} 