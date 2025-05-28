# 🚀 性能基准测试

本目录包含三个不同实现的HTTP服务器，用于性能对比测试：

## 服务器列表

### 1. Actix-JS 服务器 (actix-server.js)
- **实现**: 使用我们的Actix-JS框架（Rust + Node.js绑定）
- **端口**: 3001
- **特点**: 
  - Rust性能 + JavaScript开发体验
  - 包含LRU缓存路由优化
  - 异步处理能力

### 2. Fastify 服务器 (native-server.js)
- **实现**: 纯Node.js + Fastify框架
- **端口**: 3000
- **特点**: 
  - 高性能Node.js框架
  - 原生JavaScript实现
  - 作为Node.js生态的基准

### 3. 原生Actix-Web服务器 (rust-native/)
- **实现**: 纯Rust + Actix-Web框架
- **端口**: 3002
- **特点**: 
  - 原生Rust性能
  - 无JavaScript桥接开销
  - 理论性能上限参考

## 实现的接口

所有服务器都实现了相同的HTTP接口：

### GET 接口
- `GET /` - 返回 "Hello World" 文本
- `GET /json` - 返回JSON响应: `{"message": "Hello JSON", "timestamp": <时间戳>}`

### POST 接口
- `POST /echo` - 回显请求体: `{"echo": <请求体>, "timestamp": <时间戳>}`
- `POST /json` - 处理JSON请求: `{"received": <请求体>, "timestamp": <时间戳>}`

## 运行方式

### 启动 Actix-JS 服务器
```bash
# 在项目根目录
node benchmark/actix-server.js
# 访问: http://127.0.0.1:3001
```

### 启动 Fastify 服务器
```bash
# 确保安装了依赖
npm install fastify @fastify/cors

# 运行服务器
node benchmark/native-server.js
# 访问: http://127.0.0.1:3000
```

### 启动原生Actix-Web服务器
```bash
# 进入rust-native目录
cd benchmark/rust-native

# 开发模式运行
cargo run

# 或者编译优化版本运行
cargo build --release
./target/release/actix-native-benchmark

# 访问: http://127.0.0.1:3002
```

## 性能测试建议

### 使用 wrk 进行压力测试
```bash
# 安装wrk (Linux/macOS)
# Ubuntu: sudo apt install wrk
# macOS: brew install wrk

# 测试各个接口
wrk -t4 -c100 -d30s http://127.0.0.1:3001/
wrk -t4 -c100 -d30s http://127.0.0.1:3000/
wrk -t4 -c100 -d30s http://127.0.0.1:3002/

# POST请求测试
wrk -t4 -c100 -d30s -s post.lua http://127.0.0.1:3001/json
```

### 使用 hey 进行测试（跨平台）
```bash
# 安装hey
go install github.com/rakyll/hey@latest

# GET测试
hey -n 10000 -c 100 http://127.0.0.1:3001/
hey -n 10000 -c 100 http://127.0.0.1:3000/
hey -n 10000 -c 100 http://127.0.0.1:3002/

# POST测试
hey -n 10000 -c 100 -m POST -H "Content-Type: application/json" -d '{"test": "data"}' http://127.0.0.1:3001/json
```

### 使用 Apache Bench (ab)
```bash
# GET测试
ab -n 10000 -c 100 http://127.0.0.1:3001/
ab -n 10000 -c 100 http://127.0.0.1:3000/
ab -n 10000 -c 100 http://127.0.0.1:3002/

# POST测试 (创建测试文件 test.json)
echo '{"test": "data"}' > test.json
ab -n 10000 -c 100 -T "application/json" -p test.json http://127.0.0.1:3001/json
```

## 预期性能对比

### 理论分析
1. **原生Actix-Web**: 最高性能，无桥接开销
2. **Actix-JS**: 接近原生性能，有少量FFI开销，但有LRU路由缓存优化
3. **Fastify**: Node.js生态中的高性能选择，但受限于V8引擎

### 关注指标
- **请求处理量 (Requests/sec)**
- **延迟分布 (Latency percentiles)**
- **内存使用量**
- **CPU使用率**
- **错误率**

## 优化建议

### 系统级优化
- 关闭不必要的系统服务
- 调整TCP内核参数
- 使用专用测试机器

### 测试环境
- 相同的硬件配置
- 相同的网络环境
- 避免其他程序干扰

## 注意事项

1. **端口隔离**: 三个服务器使用不同端口，避免冲突
2. **预热**: 建议先进行少量请求预热，再进行正式测试
3. **多轮测试**: 进行多轮测试取平均值，减少偶然因素
4. **资源监控**: 测试过程中监控CPU、内存、网络使用情况

这个基准测试可以帮助我们了解：
- Actix-JS相对于原生Rust的性能开销
- LRU缓存优化的实际效果
- 与Node.js生态的性能对比
- 在不同负载下的表现差异 