const { Server, get, post, put, patch, del, cleanupRouter, sum } = require('../index.js');

console.log('🚀 开始简单测试...');

// 注册一个简单的路由
get('/', (err, requestWrapper) => {
  if (err) {
    console.log('❌ 回调出错:', err);
    return;
  }
  
  console.log('✅ 收到请求:', {
    method: requestWrapper.getMethod(),
    path: requestWrapper.getPath(),
    query: requestWrapper.getQueryString()
  });
});

console.log('✅ 路由注册完成');

const server = new Server({
  host: '127.0.0.1',
  port: 3002
});

get('/api/test/:id', (err, req) => {
  if (err) {
    console.log('❌ 回调出错:', err);
    return;
  }
  
  console.log('✅ 收到请求:', {
    method: req.getMethod(),
    path: req.getPath(),
    query: req.getQueryString(),
    pathParams: req.getPathParams(),
    id: req.getPathParam('id')
  });

  req.sendObject({
    id: req.getPathParam('id'),
  })
});

const result = server.start();
console.log('✅ 服务器启动结果:', result);

console.log('🌐 服务器运行在 http://127.0.0.1:3002');
console.log('📝 访问 http://127.0.0.1:3002/ 来测试路由');
console.log('⏹  按 Ctrl+C 停止服务器');

// 优雅停止
process.on('SIGINT', () => {
  console.log('\n🛑 收到停止信号，正在清理...');
  try {
    cleanupRouter();
    console.log('✅ 路由清理完成');
  } catch (error) {
    console.error('❌ 清理失败:', error.message);
  }
  process.exit(0);
});