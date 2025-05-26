const { Server, get, post, put, patch, del, cleanupRouter, sum } = require('../index.js');
const http = require('http');

console.log('🚀 开始Actix-JS完整功能测试...\n');

// =============================================================================
// 1. 基础功能测试
// =============================================================================
console.log('📋 1. 基础功能测试');
const result = sum(2, 3);
console.log(`   ✅ 基础函数测试 sum(2, 3) = ${result}`);

// =============================================================================
// 2. 路由注册测试 - 展示用户自定义路由
// =============================================================================
console.log('\n📋 2. 用户自定义路由注册测试');
try {
  // 首页路由 - 用户自定义
  get('/', (err, requestWrapper) => {
    createRouteHandler('首页')(err, requestWrapper);
  });
  console.log('   ✅ 首页路由注册成功');

  // API测试路由
  get('/api/test', (err, requestWrapper) => {
    createRouteHandler('API测试')(err, requestWrapper);
  });
  console.log('   ✅ API测试路由注册成功');

  // 健康检查路由 - 用户自定义
  get('/health', (err, requestWrapper) => {
    createRouteHandler('健康检查')(err, requestWrapper);
  });
  console.log('   ✅ 健康检查路由注册成功');

  // 用户管理路由
  post('/api/users', (err, requestWrapper) => {
    createRouteHandler('创建用户')(err, requestWrapper);
  });
  console.log('   ✅ 创建用户路由注册成功');

  // 带参数的路由
  put('/api/users/:id', (err, requestWrapper) => {
    createRouteHandler('更新用户')(err, requestWrapper);
  });
  console.log('   ✅ 更新用户路由注册成功');

  // 删除用户路由
  del('/api/users/:id', (err, requestWrapper) => {
    createRouteHandler('删除用户')(err, requestWrapper);
  });
  console.log('   ✅ 删除用户路由注册成功');

} catch (error) {
  console.log(`   ❌ 路由注册失败: ${error.message}`);
}

// =============================================================================
// 3. 服务器创建和启动测试
// =============================================================================
console.log('\n📋 3. 服务器创建和启动测试');
const server = new Server({
  host: '127.0.0.1',
  port: 3001
});

try {
  const result = server.start();
  console.log(`   ✅ 服务器启动成功: ${result}`);
  console.log('   🌐 服务器运行在 http://127.0.0.1:3001');
} catch (error) {
  console.log(`   ❌ 服务器启动失败: ${error.message}`);
}

// =============================================================================
// 4. HTTP请求测试 - 测试用户自定义的路由
// =============================================================================
console.log('\n📋 4. HTTP请求测试（2秒后开始）');
setTimeout(() => {
  
  // 测试用户自定义的首页路由
  console.log('   🔗 测试首页路由 GET /');
  const req1 = http.get('http://127.0.0.1:3001/', (res) => {
    let data = '';
    res.on('data', chunk => data += chunk);
    res.on('end', () => {
      console.log(`   ✅ 首页路由响应: ${data}`);
    });
  });
  
  req1.on('error', (err) => {
    console.error(`   ❌ 首页路由请求失败: ${err.message}`);
  });
  
  // 测试用户自定义的健康检查路由
  setTimeout(() => {
    console.log('   🔗 测试健康检查路由 GET /health');
    const req2 = http.get('http://127.0.0.1:3001/health', (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        console.log(`   ✅ 健康检查路由响应: ${data}`);
      });
    });
    
    req2.on('error', (err) => {
      console.error(`   ❌ 健康检查路由请求失败: ${err.message}`);
    });
  }, 500);
  
  // 测试API测试路由
  setTimeout(() => {
    console.log('   🔗 测试API路由 GET /api/test');
    const req3 = http.get('http://127.0.0.1:3001/api/test', (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        console.log(`   ✅ API测试路由响应: ${data}`);
      });
    });
    
    req3.on('error', (err) => {
      console.error(`   ❌ API测试路由请求失败: ${err.message}`);
    });
  }, 1000);

  // 测试POST路由
  setTimeout(() => {
    console.log('   🔗 测试POST路由 POST /api/users');
    const postData = JSON.stringify({ name: 'John', age: 30 });
    
    const options = {
      hostname: '127.0.0.1',
      port: 3001,
      path: '/api/users',
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(postData)
      }
    };
    
    const req4 = http.request(options, (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        console.log(`   ✅ POST路由响应: ${data}`);
      });
    });
    
    req4.on('error', (err) => {
      console.error(`   ❌ POST路由请求失败: ${err.message}`);
    });
    
    req4.write(postData);
    req4.end();
  }, 1500);

  // 测试404情况 - 未注册的路由
  setTimeout(() => {
    console.log('   🔗 测试404路由 GET /nonexistent');
    const req5 = http.get('http://127.0.0.1:3001/nonexistent', (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        console.log(`   ✅ 404路由响应 (${res.statusCode}): ${data}`);
      });
    });
    
    req5.on('error', (err) => {
      console.error(`   ❌ 404路由请求失败: ${err.message}`);
    });
  }, 2000);

}, 2000);

// =============================================================================
// 5. 服务器停止和清理测试
// =============================================================================
console.log('\n📋 5. 服务器停止测试（12秒后执行）');
setTimeout(() => {
  console.log('   🛑 停止服务器...');
  try {
    const stopResult = server.stop();
    console.log(`   ✅ 服务器停止成功: ${stopResult}`);
  } catch (error) {
    console.error(`   ❌ 服务器停止失败: ${error.message}`);
  }
  
  try {
    cleanupRouter();
    console.log('   ✅ 路由清理成功');
  } catch (error) {
    console.error(`   ❌ 路由清理失败: ${error.message}`);
  }
  
  console.log('\n🎉 测试完成！');
  console.log('📝 说明：这是一个通用的HTTP服务器框架，所有路由都由用户自定义。');
  process.exit(0);
}, 12000);

// 异常处理
process.on('unhandledRejection', (reason, promise) => {
  console.error('❌ 未处理的Promise拒绝:', reason);
});

process.on('uncaughtException', (error) => {
  console.error('❌ 未捕获的异常:', error);
  process.exit(1);
});

console.log('\n⏳ 测试进行中，请等待...');

// 注册路由的回调函数 - 使用新的RequestWrapper响应功能
function createRouteHandler(routeName) {
  return (err, requestWrapper) => {
    if (err) {
      console.log(`   ❌ ${routeName}回调出错:`, err);
      return;
    }
    
    if (!requestWrapper) {
      console.log(`   ❌ ${routeName}回调接收到null参数`);
      return;
    }
    
    try {
      // 使用RequestWrapper的方法获取请求数据
      const method = requestWrapper.getMethod();
      const path = requestWrapper.getPath();
      const queryString = requestWrapper.getQueryString();
      const queryParams = requestWrapper.getQueryParams();
      const body = requestWrapper.getBodyString();
      const headers = requestWrapper.getHeaders();
      const pathParams = requestWrapper.getPathParams();
      
      console.log(`   ✅ ${routeName}回调被调用:`);
      console.log(`      方法: ${method}`);
      console.log(`      路径: ${path}`);
      console.log(`      查询字符串: "${queryString}"`);
      console.log(`      查询参数:`, queryParams);
      console.log(`      路径参数:`, pathParams);
      console.log(`      请求体: "${body}"`);
      console.log(`      请求头数量: ${Object.keys(headers).length}`);
      
      // 根据不同路由返回不同响应
      if (path === '/') {
        // 首页路由
        requestWrapper.setStatusCode(200);
        requestWrapper.addHeader('X-Custom-Header', 'Hello from Actix-JS');
        requestWrapper.sendJson(JSON.stringify({
          message: "欢迎使用 Actix-JS！",
          path: path,
          method: method,
          timestamp: new Date().toISOString()
        }));
      } else if (path === '/health') {
        // 健康检查路由
        requestWrapper.setStatusCode(200);
        requestWrapper.sendObject({
          status: "healthy",
          uptime: process.uptime(),
          timestamp: new Date().toISOString()
        });
      } else if (path === '/api/test') {
        // API测试路由
        requestWrapper.setStatusCode(200);
        requestWrapper.addHeader('Content-Type', 'application/json');
        requestWrapper.sendObject({
          success: true,
          data: {
            queryParams: queryParams,
            headers: Object.keys(headers).length,
            method: method
          }
        });
      } else if (path === '/api/users' && method === 'POST') {
        // 创建用户路由
        try {
          const userData = body ? JSON.parse(body) : {};
          requestWrapper.setStatusCode(201);
          requestWrapper.sendObject({
            success: true,
            message: "用户创建成功",
            user: {
              id: Math.floor(Math.random() * 1000),
              ...userData,
              createdAt: new Date().toISOString()
            }
          });
        } catch (e) {
          requestWrapper.setStatusCode(400);
          requestWrapper.sendObject({
            success: false,
            error: "无效的JSON数据"
          });
        }
      } else if (path.startsWith('/api/users/') && (method === 'PUT' || method === 'DELETE')) {
        // 更新或删除用户路由
        const userId = pathParams.id;
        if (method === 'PUT') {
          requestWrapper.setStatusCode(200);
          requestWrapper.sendObject({
            success: true,
            message: `用户 ${userId} 更新成功`,
            userId: userId
          });
        } else if (method === 'DELETE') {
          requestWrapper.setStatusCode(204);
          requestWrapper.sendEmpty();
        }
      } else {
        // 其他路由
        requestWrapper.setStatusCode(200);
        requestWrapper.sendText(`${routeName} 处理完成 - ${path}`);
      }
    } catch (error) {
      console.log(`   ❌ ${routeName}回调处理出错:`, error);
      try {
        requestWrapper.setStatusCode(500);
        requestWrapper.sendError(`服务器内部错误: ${error.message}`);
      } catch (e) {
        console.log(`   ❌ 发送错误响应失败:`, e);
      }
    }
  };
} 