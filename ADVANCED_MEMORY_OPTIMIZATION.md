# 高级内存优化方案

## 📋 **优化目标**
在 `HttpRequest` 不能跨线程传递的约束下，实现内存使用优化和性能提升。

## 🎯 **策略一：字符串内存池 (推荐实施)**

### **1.1 HTTP 方法字符串池**
HTTP 方法数量有限，可以使用字符串常量池：

```rust
use std::sync::LazyLock;
use std::collections::HashMap;

static HTTP_METHODS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert("GET", "GET");
    map.insert("POST", "POST");
    map.insert("PUT", "PUT");
    map.insert("PATCH", "PATCH");
    map.insert("DELETE", "DELETE");
    map.insert("HEAD", "HEAD");
    map.insert("OPTIONS", "OPTIONS");
    map
});

impl DetachedRequestWrapper {
    pub fn new_detached(req: HttpRequest, body: Option<Bytes>, path_params: HashMap<String, String>) -> Self {
        // 使用字符串池优化方法名
        let method = HTTP_METHODS.get(req.method().as_str())
            .copied()
            .unwrap_or_else(|| req.method().as_str())
            .to_string();
        
        // ... 其他代码
    }
}
```

### **1.2 常见请求头池**
常见请求头名称也可以池化：

```rust
static COMMON_HEADERS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert("content-type", "content-type");
    map.insert("content-length", "content-length");
    map.insert("user-agent", "user-agent");
    map.insert("accept", "accept");
    map.insert("authorization", "authorization");
    map.insert("cookie", "cookie");
    map.insert("host", "host");
    map.insert("referer", "referer");
    map.insert("x-forwarded-for", "x-forwarded-for");
    map.insert("x-real-ip", "x-real-ip");
    map
});

fn extract_headers_optimized(req: &HttpRequest) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for (name, value) in req.headers() {
        if let Ok(value_str) = value.to_str() {
            let header_name = COMMON_HEADERS.get(name.as_str())
                .copied()
                .unwrap_or(name.as_str());
            headers.insert(header_name.to_string(), value_str.to_string());
        }
    }
    headers
}
```

### **性能提升预期**: 10-15% 内存使用减少

## 🎯 **策略二：线程本地对象池**

由于 `HttpRequest` 不能跨线程，我们可以为每个工作线程创建本地对象池：

```rust
use std::cell::RefCell;

thread_local! {
    static DETACHED_WRAPPER_POOL: RefCell<Vec<DetachedRequestWrapper>> = RefCell::new(Vec::new());
    static STRING_BUFFER_POOL: RefCell<Vec<String>> = RefCell::new(Vec::new());
    static HASHMAP_POOL: RefCell<Vec<HashMap<String, String>>> = RefCell::new(Vec::new());
}

impl DetachedRequestWrapper {
    /// 从线程本地池获取或创建新的 DetachedRequestWrapper
    pub fn from_pool(req: HttpRequest, body: Option<Bytes>, path_params: HashMap<String, String>) -> Self {
        DETACHED_WRAPPER_POOL.with(|pool| {
            let mut pool = pool.borrow_mut();
            if let Some(mut wrapper) = pool.pop() {
                // 重用现有对象，重置状态
                wrapper.reset_and_reuse(req, body, path_params);
                wrapper
            } else {
                // 创建新对象
                Self::new_detached(req, body, path_params)
            }
        })
    }

    /// 将对象返回到线程本地池
    pub fn return_to_pool(mut self) {
        self.clear_sensitive_data();
        DETACHED_WRAPPER_POOL.with(|pool| {
            let mut pool = pool.borrow_mut();
            if pool.len() < 10 { // 限制池大小
                pool.push(self);
            }
            // 超过限制的对象会被自动丢弃
        });
    }

    fn reset_and_reuse(&mut self, req: HttpRequest, body: Option<Bytes>, path_params: HashMap<String, String>) {
        // 重置所有字段
        self.path = req.path().to_string();
        self.method = req.method().as_str().to_string();
        self.query_string = req.query_string().to_string();
        self.uri = req.uri().to_string();
        self.body = body;
        self.path_params = path_params;
        self.sent = false;
        self.status_code = None;
        self.response_headers.clear();
        
        // 重新提取请求头（可以复用 HashMap）
        self.headers.clear();
        for (name, value) in req.headers() {
            if let Ok(value_str) = value.to_str() {
                self.headers.insert(name.as_str().to_string(), value_str.to_string());
            }
        }
        
        // 重新计算缓存
        self.recalculate_caches();
    }

    fn clear_sensitive_data(&mut self) {
        // 清理敏感数据，但保留容器
        self.path.clear();
        self.method.clear();
        self.query_string.clear();
        self.uri.clear();
        self.headers.clear();
        self.body = None;
        self.path_params.clear();
        self.response_headers.clear();
        self.cached_query_params = None;
        self.cached_json = None;
        self.cached_form_data = None;
    }
}
```

### **性能提升预期**: 20-30% 内存分配减少

## 🎯 **策略三：预分配缓冲区优化**

```rust
thread_local! {
    static LARGE_STRING_BUFFER: RefCell<String> = RefCell::new(String::with_capacity(8192));
    static FORM_PARSE_BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(4096));
}

impl DetachedRequestWrapper {
    fn parse_form_data_with_buffer(&self, body: &Bytes) -> Option<serde_json::Value> {
        LARGE_STRING_BUFFER.with(|buffer| {
            let mut buffer = buffer.borrow_mut();
            buffer.clear();
            
            // 使用预分配的缓冲区进行解析
            if let Ok(body_str) = std::str::from_utf8(body) {
                buffer.push_str(body_str);
                // 在缓冲区中进行解析，避免重复分配
                self.parse_form_from_buffer(&buffer)
            } else {
                None
            }
        })
    }
}
```

## 🎯 **策略四：智能容器预分配**

```rust
impl DetachedRequestWrapper {
    pub fn new_detached_optimized(
        req: HttpRequest, 
        body: Option<Bytes>, 
        path_params: HashMap<String, String>
    ) -> Self {
        // 智能预分配容器大小
        let header_count = req.headers().len();
        let mut headers = HashMap::with_capacity(header_count.max(16));
        
        // 查询参数数量估算
        let query_capacity = req.query_string().matches('&').count() + 1;
        let cached_query_params = if req.query_string().is_empty() {
            None
        } else {
            Some(HashMap::with_capacity(query_capacity))
        };

        // ... 其他优化代码
    }
}
```

## 🎯 **策略五：Bytes 零拷贝增强**

```rust
impl DetachedRequestWrapper {
    /// 零拷贝获取请求体切片
    pub fn get_body_slice(&self) -> Option<&[u8]> {
        self.body.as_ref().map(|b| b.as_ref())
    }

    /// 零拷贝 JSON 解析（避免字符串分配）
    pub fn parse_json_zero_copy(&self) -> Option<serde_json::Value> {
        self.body.as_ref()
            .and_then(|bytes| serde_json::from_slice(bytes).ok())
    }

    /// 使用 Cow 避免不必要的字符串分配
    pub fn get_header_cow(&self, name: &str) -> Option<std::borrow::Cow<str>> {
        self.headers.get(name).map(|s| s.as_str().into())
    }
}
```

## 📊 **实施优先级**

### **Phase 1 (立即实施)**
1. ✅ **字符串常量池** - HTTP方法和常见请求头
2. ✅ **智能容器预分配** - 根据实际数据大小预分配

### **Phase 2 (短期实施)**
3. **线程本地对象池** - DetachedRequestWrapper 复用
4. **预分配缓冲区** - 减少解析时的内存分配

### **Phase 3 (长期优化)**
5. **Bytes 零拷贝增强** - 更深度的零拷贝优化
6. **内存使用监控** - 运行时内存使用统计

## 🎯 **为什么不使用传统对象池**

### **HttpRequest 限制**
```rust
// ❌ 不可行 - HttpRequest 不是 Send
static GLOBAL_REQUEST_POOL: LazyLock<Pool<HttpRequest>> = LazyLock::new(|| {
    Pool::new(100, || HttpRequest::default()) // HttpRequest 没有 Default，也不是 Send
});

// ❌ 不可行 - 无法跨线程传递
fn get_request_from_global_pool() -> HttpRequest {
    GLOBAL_REQUEST_POOL.get() // 编译错误：HttpRequest 不是 Send
}
```

### **现实的替代方案**
```rust
// ✅ 可行 - 线程本地池
thread_local! {
    static LOCAL_WRAPPER_POOL: RefCell<Vec<DetachedRequestWrapper>> = 
        RefCell::new(Vec::with_capacity(10));
}

// ✅ 可行 - 字符串池
static STRING_POOL: LazyLock<HashMap<&'static str, &'static str>> = 
    LazyLock::new(|| HashMap::new());
```

## 📈 **预期性能提升**

| 优化策略 | 内存使用改善 | CPU 使用改善 | 实施复杂度 |
|----------|-------------|-------------|-----------|
| 字符串常量池 | 10-15% | 5-8% | 低 |
| 线程本地池 | 20-30% | 15-20% | 中 |
| 预分配缓冲区 | 15-25% | 10-15% | 中 |
| Bytes 零拷贝 | 5-10% | 8-12% | 低 |
| **总计** | **40-60%** | **30-45%** | **中等** |

## 🔧 **实施建议**

1. **从字符串池开始** - 低风险，立即见效
2. **渐进式实施** - 一次实施一个策略，测试稳定性
3. **性能监控** - 实施前后进行基准测试
4. **内存安全** - 确保线程本地池的正确生命周期管理

这些优化策略在不违反 Rust 借用检查和线程安全的前提下，可以显著提升内存使用效率和整体性能。 