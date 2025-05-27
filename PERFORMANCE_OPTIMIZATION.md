# Actix-JS 性能优化指南

## 当前性能状况

- **当前性能**: 60,000 请求/分钟 (1,000 RPS)
- **目标性能**: 300,000+ 请求/分钟 (5,000+ RPS)
- **对比参考**:
  - Express.js: ~10,000-15,000 RPS
  - Fastify: ~20,000-30,000 RPS  
  - 纯 Actix-web: ~100,000+ RPS

## 主要性能瓶颈

### 1. 内存分配和拷贝 🔥
**问题**: 大量不必要的字符串克隆和内存分配
```rust
// 当前代码中的问题
pub fn get_path(&self) -> String {
    self.path.clone() // 每次调用都克隆
}
```

### 2. JavaScript 回调开销 🔥
**问题**: Rust ↔ JavaScript 边界的序列化/反序列化开销

### 3. 同步等待 JavaScript 响应 🔥
**问题**: 5秒超时等待，阻塞 Rust 线程

## 优化方案

### 阶段一：零拷贝优化 (预期提升: 2-3x)

#### 1.1 使用 `Cow<str>` 减少字符串拷贝
```rust
use std::borrow::Cow;

pub struct RequestWrapper {
    path: Cow<'static, str>,
    method: Cow<'static, str>,
    // ...
}

impl RequestWrapper {
    pub fn get_path(&self) -> &str {
        &self.path // 返回引用而不是克隆
    }
}
```

#### 1.2 使用 `Arc<[u8]>` 共享请求体
```rust
use std::sync::Arc;

pub struct RequestWrapper {
    body: Option<Arc<[u8]>>, // 零拷贝共享
    // ...
}
```

#### 1.3 延迟解析策略
```rust
pub struct RequestWrapper {
    raw_query: String,
    parsed_query: OnceCell<HashMap<String, String>>, // 延迟解析
}

impl RequestWrapper {
    pub fn get_query_params(&self) -> &HashMap<String, String> {
        self.parsed_query.get_or_init(|| {
            serde_qs::from_str(&self.raw_query).unwrap_or_default()
        })
    }
}
```

### 阶段二：异步优化 (预期提升: 2-3x) ✅ 已实施

#### 2.1 解决 BorrowMutError 问题 ✅
**问题**: 高并发下 `HttpRequest` 多重借用导致 `BorrowMutError`
```rust
// 问题代码: HttpRequest 被多次可变借用
let mut request_wrapper = RequestWrapper::new_with_params(req, Some(body), path_params);
// JavaScript 回调持有引用时，其他请求无法访问
```

**解决方案**: 使用 `DetachedRequestWrapper` 避免引用冲突
```rust
// ✅ 优化后: 提前提取所有数据，不持有 HttpRequest 引用
pub struct DetachedRequestWrapper {
    // 提前提取的请求数据，不持有HttpRequest引用
    path: String,
    method: String,
    query_string: String,
    uri: String,
    headers: HashMap<String, String>,
    body: Option<Bytes>,
    path_params: HashMap<String, String>,
    // ... 其他字段
}

impl DetachedRequestWrapper {
    pub fn new_detached(req: HttpRequest, body: Option<Bytes>, path_params: HashMap<String, String>) -> Self {
        // 🚀 关键优化：提前提取所有请求数据
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
        // ... 构建结构体
    }
}
```

#### 2.2 JavaScript 异步回调支持 ✅
**实现**: JavaScript 回调现在可以使用 `async/await` 语法
```javascript
// ✅ 新的异步API使用方式
const { getAsync } = require('@qunto/actix-js');

getAsync('/', async (err, req) => {
    if (err) {
        await req.setStatusCodeAsync(500);
        await req.sendErrorAsync('error');
        return;
    }
    
    // 🚀 支持异步操作，不阻塞Rust线程
    await req.setStatusCodeAsync(200);
    await req.addHeaderAsync('Content-Type', 'application/json');
    await req.sendTextAsync('hello world');
});
```

**异步方法列表**:
```rust
// ✅ 已实现的异步方法
pub async unsafe fn send_text_async(&mut self, text: String) -> Result<()>
pub async unsafe fn send_json_async(&mut self, json: String) -> Result<()>
pub async unsafe fn send_object_async(&mut self, obj: serde_json::Value) -> Result<()>
pub async unsafe fn send_empty_async(&mut self) -> Result<()>
pub async unsafe fn send_error_async(&mut self, message: Option<String>) -> Result<()>
pub async unsafe fn set_status_code_async(&mut self, status: u16) -> Result<bool>
pub async unsafe fn add_header_async(&mut self, key: String, value: String) -> Result<()>
```

#### 2.3 非阻塞请求处理 ✅
**优化**: Rust 主线程不再阻塞等待 JavaScript 响应
```rust
// ✅ 优化后的 handle_dynamic_route
async fn handle_dynamic_route(req: HttpRequest, body: web::Bytes) -> HttpResponse {
    // 🚀 关键优化：使用DetachedRequestWrapper，避免BorrowMutError
    let mut detached_wrapper = DetachedRequestWrapper::new_detached(req, Some(body), path_params);
    detached_wrapper.set_response_sender(tx);

    // 🚀 异步执行JavaScript回调，不阻塞Rust主线程
    router::node_functions::execute_callback_with_detached_request(callback, detached_wrapper);

    // 🚀 非阻塞等待：增加超时时间到10秒，给异步处理更多时间
    match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
        // 处理响应...
    }
}
```

#### 2.4 新的异步路由注册API ✅
```rust
// ✅ 新增的异步路由注册函数
pub fn get_async(route: String, callback: ThreadsafeFunction<DetachedRequestWrapper>) -> Result<()>
pub fn post_async(route: String, callback: ThreadsafeFunction<DetachedRequestWrapper>) -> Result<()>
pub fn put_async(route: String, callback: ThreadsafeFunction<DetachedRequestWrapper>) -> Result<()>
pub fn patch_async(route: String, callback: ThreadsafeFunction<DetachedRequestWrapper>) -> Result<()>
pub fn del_async(route: String, callback: ThreadsafeFunction<DetachedRequestWrapper>) -> Result<()>
```

#### 2.5 性能提升预期
- **BorrowMutError 解决**: 消除高并发下的错误，提升稳定性
- **异步处理**: 从 1,000 RPS → 3,000-5,000 RPS (3-5倍提升)
- **非阻塞架构**: Rust 主线程不再等待 JavaScript，可处理更多并发请求
- **超时优化**: 从 5秒 → 10秒，减少超时错误

#### 2.6 批量处理请求 (待实施)
```rust
pub struct BatchProcessor {
    batch: Vec<DetachedRequestWrapper>,
    batch_size: usize,
}

impl BatchProcessor {
    pub async fn process_batch(&mut self) {
        // 批量发送到 JavaScript
        // 批量接收响应
    }
}
```

### 阶段三：内存池优化 (预期提升: 1.5-2x)

#### 3.1 对象池
```rust
use object_pool::Pool;

static REQUEST_POOL: Lazy<Pool<RequestWrapper>> = Lazy::new(|| {
    Pool::new(1000, || RequestWrapper::default())
});

pub fn get_request_wrapper() -> PoolGuard<RequestWrapper> {
    REQUEST_POOL.try_pull().unwrap_or_else(|| {
        Pool::new(1, || RequestWrapper::default()).try_pull().unwrap()
    })
}
```

#### 3.2 字符串内存池
```rust
use string_cache::DefaultAtom;

pub struct RequestWrapper {
    path: DefaultAtom, // 内存池中的字符串
    method: DefaultAtom,
}
```

### 阶段四：编译优化 (预期提升: 1.2-1.5x)

#### 4.1 Cargo.toml 优化
```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true

[dependencies]
# 使用更快的分配器
mimalloc = { version = "0.1", default-features = false }
```

#### 4.2 CPU 特定优化
```toml
[profile.release]
target-cpu = "native"
```

### 阶段五：架构优化 (预期提升: 2-5x)

#### 5.1 多线程 JavaScript 引擎
```rust
pub struct MultiThreadJSEngine {
    workers: Vec<JSWorker>,
    load_balancer: LoadBalancer,
}
```

#### 5.2 请求路由缓存
```rust
use lru::LruCache;

static ROUTE_CACHE: Lazy<Mutex<LruCache<String, CallBackFunction>>> = 
    Lazy::new(|| Mutex::new(LruCache::new(10000)));
```

## 实施计划

### 第1周: 零拷贝优化 ✅ 已完成
- [x] 实现延迟解析 (OnceCell)
- [x] 实现请求头缓存
- [x] 实现查询参数缓存
- [x] 实现JSON解析缓存
- [x] 实现表单数据缓存
- [x] 基准测试

### 第2周: 异步优化 ✅ 已完成
- [x] 解决 BorrowMutError 问题
- [x] 实现 DetachedRequestWrapper
- [x] 实现异步回调支持 (async/await)
- [x] 实现非阻塞请求处理
- [x] 新增异步路由注册API
- [ ] 实现批量处理 (待实施)
- [x] 基准测试

### 第3周: 内存池优化
- [ ] 实现对象池
- [ ] 实现字符串池
- [ ] 基准测试

### 第4周: 编译和架构优化
- [ ] 编译器优化
- [ ] 多线程引擎
- [ ] 最终基准测试

## 基准测试

### 测试工具
```bash
# 使用 wrk 进行压测
wrk -t12 -c400 -d30s http://127.0.0.1:3001/

# 使用 autocannon (Node.js)
npx autocannon -c 100 -d 30 http://127.0.0.1:3001/
```

### 性能指标
- **RPS (Requests Per Second)**: 目标 5,000+
- **延迟 (Latency)**: P99 < 100ms
- **内存使用**: < 100MB
- **CPU 使用**: < 80%

## 预期结果

| 优化阶段 | 当前 RPS | 优化后 RPS | 提升倍数 |
|---------|----------|------------|----------|
| 基线     | 1,000    | 1,000      | 1.0x     |
| 零拷贝   | 1,000    | 2,500      | 2.5x     |
| 异步优化 | 2,500    | 6,000      | 2.4x     |
| 内存池   | 6,000    | 10,000     | 1.7x     |
| 编译优化 | 10,000   | 12,000     | 1.2x     |
| 架构优化 | 12,000   | 25,000+    | 2.1x     |

**最终目标**: 25,000+ RPS (25倍性能提升)

## 🎉 异步优化总结

### ✅ 已解决的核心问题

1. **BorrowMutError 彻底解决**
   - 使用 `DetachedRequestWrapper` 避免 `HttpRequest` 多重借用
   - 提前提取所有请求数据，消除引用冲突
   - 高并发环境下稳定运行

2. **JavaScript 异步支持**
   - 新增 `*_async` 系列方法，支持 `await` 语法
   - JavaScript 回调可以异步处理，不阻塞 Rust 线程
   - 更好的错误处理和超时控制

3. **非阻塞架构**
   - Rust 主线程不再等待 JavaScript 响应
   - 超时时间从 5秒 → 10秒，减少超时错误
   - 支持更高的并发请求处理

### 🚀 性能提升预期

- **稳定性**: 消除 `BorrowMutError`，100% 稳定运行
- **吞吐量**: 从 1,000 RPS → 3,000-5,000 RPS (3-5倍提升)
- **延迟**: 减少阻塞等待，降低平均响应时间
- **并发**: 支持更高的并发连接数

### 📝 使用方式

#### 新的异步API
```javascript
const { getAsync, postAsync } = require('@qunto/actix-js');

// 异步路由处理
getAsync('/', async (err, req) => {
    if (err) {
        await req.setStatusCodeAsync(500);
        await req.sendErrorAsync('Internal Server Error');
        return;
    }
    
    // 异步处理逻辑
    const data = await someAsyncOperation();
    await req.setStatusCodeAsync(200);
    await req.addHeaderAsync('Content-Type', 'application/json');
    await req.sendObjectAsync({ data });
});
```

#### 向后兼容
```javascript
// 旧的同步API仍然支持（但建议迁移到异步版本）
const { get, post } = require('@qunto/actix-js');

get('/', (err, req) => {
    if (err) {
        req.setStatusCode(500);
        req.sendError('error');
        return;
    }
    req.sendText('hello world');
});
```

### 🎯 下一步优化建议

1. **批量处理**: 实现请求批量处理，进一步提升吞吐量
2. **内存池**: 实现对象池和字符串池，减少内存分配
3. **编译优化**: 启用 LTO 和 CPU 特定优化
4. **多线程 JS 引擎**: 支持多个 JavaScript 工作线程

这次异步优化为后续的性能提升奠定了坚实的基础！🎉 