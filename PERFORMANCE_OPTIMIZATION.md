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

#### 4.1 基础编译优化 (推荐配置)
```toml
[profile.release]
# 最高优化级别
opt-level = 3
# 启用链接时优化，显著提升性能
lto = "fat"
# 单个代码生成单元，更好的优化
codegen-units = 1
# 直接终止而不是展开panic，减少二进制大小
panic = "abort"
# 移除调试符号，减少二进制大小
strip = true
# 溢出检查在release模式下关闭（默认行为）
overflow-checks = false
```

#### 4.2 内存分配器优化 (条件性配置)

##### 4.2.1 推荐方案：jemalloc (跨平台兼容)
```toml
[dependencies]
# jemalloc: 更好的跨平台兼容性，无C++依赖
jemallocator = { version = "0.5", optional = true }

[features]
default = []
# 可选的内存分配器优化
jemalloc = ["jemallocator"]
mimalloc = ["dep:mimalloc"]

# 条件性依赖：只在需要时启用
[dependencies.mimalloc]
version = "0.1"
default-features = false
optional = true
```

##### 4.2.2 Rust代码中的条件编译
```rust
// src/lib.rs 或 main.rs 顶部
#[cfg(feature = "jemalloc")]
use jemallocator::Jemalloc;

#[cfg(feature = "mimalloc")]
use mimalloc::MiMalloc;

// 根据feature选择分配器
#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
```

#### 4.3 CPU优化 (环境感知配置)

##### 4.3.1 本地开发环境
```toml
# .cargo/config.toml (本地开发)
[build]
rustflags = ["-C", "target-cpu=native"]
```

##### 4.3.2 CI/CD 安全配置
```toml
[profile.release]
# 通用x86_64优化，避免native导致的兼容性问题
# target-cpu = "x86-64-v2"  # 注释掉，通过环境变量控制
```

#### 4.4 多环境编译脚本

##### 4.4.1 本地开发构建
```bash
#!/bin/bash
# scripts/build-dev.sh
echo "🚀 本地开发构建（启用原生CPU优化）"
export RUSTFLAGS="-C target-cpu=native"
cargo build --release --features jemalloc
```

##### 4.4.2 CI/CD 构建
```bash
#!/bin/bash
# scripts/build-ci.sh
echo "📦 CI/CD构建（兼容性优先）"

# 检测系统环境
if command -v gcc >/dev/null 2>&1 && command -v g++ >/dev/null 2>&1; then
    echo "✅ 检测到C++编译器，使用mimalloc"
    FEATURES="mimalloc"
else
    echo "⚠️  未检测到C++编译器，使用jemalloc"
    FEATURES="jemalloc"
fi

# 设置通用优化标志
export RUSTFLAGS="-C target-cpu=x86-64-v2"

# 构建
cargo build --release --features $FEATURES
```

##### 4.4.3 Docker多阶段构建
```dockerfile
# Dockerfile.optimized
FROM rust:1.75-bullseye as builder

# 安装C++编译器（用于mimalloc）
RUN apt-get update && apt-get install -y \
    build-essential \
    clang \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# 条件性构建：尝试mimalloc，失败则回退到jemalloc
RUN cargo build --release --features mimalloc || \
    cargo build --release --features jemalloc

# 运行时镜像（最小化）
FROM debian:bullseye-slim
COPY --from=builder /app/target/release/your-binary /usr/local/bin/
CMD ["your-binary"]
```

#### 4.5 性能监控和验证

##### 4.5.1 内存分配器性能测试
```rust
#[cfg(test)]
mod bench {
    use criterion::{criterion_group, criterion_main, Criterion};
    
    fn memory_allocation_benchmark(c: &mut Criterion) {
        c.bench_function("string_allocation", |b| {
            b.iter(|| {
                let mut vec = Vec::new();
                for i in 0..1000 {
                    vec.push(format!("test_string_{}", i));
                }
                vec
            })
        });
    }
    
    criterion_group!(benches, memory_allocation_benchmark);
    criterion_main!(benches);
}
```

##### 4.5.2 编译优化验证脚本
```bash
#!/bin/bash
# scripts/verify-optimizations.sh

echo "🔍 验证编译优化效果"

# 检查二进制大小
echo "📏 二进制文件大小:"
ls -lh target/release/

# 检查符号表（应该被strip掉）
echo "🔧 符号表检查:"
file target/release/your-binary

# 检查使用的分配器
echo "🧠 内存分配器检查:"
ldd target/release/your-binary | grep -E "(jemalloc|mimalloc)" || echo "使用默认分配器"

# 性能基准测试
echo "⚡ 性能测试:"
cargo bench
```

#### 4.6 CI/CD 集成配置

##### 4.6.1 GitHub Actions
```yaml
# .github/workflows/optimize-build.yml
name: 优化构建测试

on: [push, pull_request]

jobs:
  test-optimizations:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        allocator: [default, jemalloc, mimalloc]
        exclude:
          # mimalloc在某些环境下可能失败
          - os: ubuntu-latest
            allocator: mimalloc
    
    runs-on: ${{ matrix.os }}
    
    steps:
    - uses: actions/checkout@v3
    
    - name: 安装Rust工具链
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        profile: minimal
        override: true
    
    - name: 安装C++编译器 (Ubuntu)
      if: matrix.os == 'ubuntu-latest' && matrix.allocator == 'mimalloc'
      run: sudo apt-get update && sudo apt-get install -y build-essential
    
    - name: 构建（条件性特性）
      run: |
        if [ "${{ matrix.allocator }}" = "default" ]; then
          cargo build --release
        else
          cargo build --release --features ${{ matrix.allocator }}
        fi
    
    - name: 运行性能测试
      run: cargo test --release
```

#### 4.7 预期性能提升

| 优化项目 | 性能提升 | 兼容性 | 风险等级 |
|---------|----------|--------|----------|
| opt-level = 3 | +15% | ✅ 高 | 🟢 低 |
| LTO = "fat" | +10% | ✅ 高 | 🟢 低 |
| jemalloc | +8% | ✅ 高 | 🟢 低 |
| mimalloc | +12% | ⚠️ 中 | 🟡 中 |
| target-cpu=native | +5% | ❌ 低 | 🔴 高 |
| strip = true | 减少50%大小 | ✅ 高 | 🟢 低 |

**总计预期提升**: 1.2-1.5x (安全配置) 或 1.3-1.7x (激进配置)

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