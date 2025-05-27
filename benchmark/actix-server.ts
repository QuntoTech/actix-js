import { RequestWrapper, Server, cleanupRouter, get, post } from '../index';

// 清理之前的路由
cleanupRouter();

// 创建服务器实例
const server = new Server({
  host: '127.0.0.1',
  port: 3001,
});

// 注册路由
get('/', (err: Error | null, req: RequestWrapper) => {
  if (err) {
    req.setStatusCode(500);
    req.sendError('Internal Server Error');
    return;
  }
  req.sendText('Hello World');
});

get('/json', (err: Error | null, req: RequestWrapper) => {
  if (err) {
    req.setStatusCode(500);
    req.sendError('Internal Server Error');
    return;
  }
  req.sendObject({ message: 'Hello JSON', timestamp: Date.now() });
});

post('/echo', (err: Error | null, req: RequestWrapper) => {
  if (err) {
    req.setStatusCode(500);
    req.sendError('Internal Server Error');
    return;
  }
  const body = req.getBodyString();
  req.sendObject({ echo: body, timestamp: Date.now() });
});

post('/json', (err: Error | null, req: RequestWrapper) => {
  if (err) {
    req.setStatusCode(500);
    req.sendError('Internal Server Error');
    return;
  }
  try {
    const data = req.getBodyJson();
    req.sendObject({ received: data, timestamp: Date.now() });
  } catch (e) {
    req.setStatusCode(400);
    req.sendError('Invalid JSON');
  }
});

// 启动服务器
console.log('Starting Actix-JS server on http://127.0.0.1:3001');
server.start();

// 优雅关闭处理
process.on('SIGINT', async () => {
  console.log('\nShutting down Actix-JS server...');
  await server.stop();
  process.exit(0);
});

process.on('SIGTERM', async () => {
  console.log('\nShutting down Actix-JS server...');
  await server.stop();
  process.exit(0);
});
