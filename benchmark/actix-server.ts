import { Server, cleanupRouter, getAsync, postAsync } from '../index';

// 清理之前的路由
cleanupRouter();

// 创建服务器实例
const server = new Server({
  host: '127.0.0.1',
  port: 3001,
  // keepAlive: 30,
});

// 注册路由
getAsync('/', async (err, req) => {
  if (err) {
    await req.setStatusCodeAsync(500);
    await req.sendErrorAsync('Internal Server Error');
    return;
  }
  await req.sendTextAsync('Hello World');
});

getAsync('/json', async (err, req) => {
  if (err) {
    await req.setStatusCodeAsync(500);
    await req.sendErrorAsync('Internal Server Error');
    return;
  }
  await req.sendObjectAsync({ message: 'Hello JSON', timestamp: Date.now() });
});

postAsync('/echo', async (err, req) => {
  if (err) {
    await req.setStatusCodeAsync(500);
    await req.sendErrorAsync('Internal Server Error');
    return;
  }
  const body = req.getBodyString();
  await req.sendObjectAsync({ echo: body, timestamp: Date.now() });
});

postAsync('/json', async (err, req) => {
  if (err) {
    await req.setStatusCodeAsync(500);
    await req.sendErrorAsync('Internal Server Error');
    return;
  }
  try {
    const data = req.getBodyJson();
    await req.sendObjectAsync({ received: data, timestamp: Date.now() });
  } catch (e) {
    await req.setStatusCodeAsync(400);
    await req.sendErrorAsync('Invalid JSON');
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
