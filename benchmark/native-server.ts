import * as http from 'http';
import * as url from 'url';

// 创建 HTTP 服务器
const server = http.createServer((req: http.IncomingMessage, res: http.ServerResponse) => {
  const parsedUrl = url.parse(req.url || '', true);
  const path = parsedUrl.pathname;
  const method = req.method;

  // 设置 CORS 头
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

  // 处理 OPTIONS 请求
  if (method === 'OPTIONS') {
    res.writeHead(200);
    res.end();
    return;
  }

  // 路由处理
  if (method === 'GET' && path === '/') {
    res.writeHead(200, { 'Content-Type': 'text/plain' });
    res.end('Hello World');
  } else if (method === 'GET' && path === '/json') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ message: 'Hello JSON', timestamp: Date.now() }));
  } else if (method === 'POST' && path === '/echo') {
    let body = '';
    req.on('data', (chunk: Buffer) => {
      body += chunk.toString();
    });
    req.on('end', () => {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ echo: body, timestamp: Date.now() }));
    });
  } else if (method === 'POST' && path === '/json') {
    let body = '';
    req.on('data', (chunk: Buffer) => {
      body += chunk.toString();
    });
    req.on('end', () => {
      try {
        const data = JSON.parse(body);
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ received: data, timestamp: Date.now() }));
      } catch (e) {
        res.writeHead(400, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'Invalid JSON' }));
      }
    });
  } else {
    // 404 处理
    res.writeHead(404, { 'Content-Type': 'text/plain' });
    res.end('Not Found');
  }
});

// 启动服务器
const PORT = 3000;
const HOST = '127.0.0.1';

server.listen(PORT, HOST, () => {
  console.log(`Starting Native HTTP server on http://${HOST}:${PORT}`);
});

// 优雅关闭处理
process.on('SIGINT', () => {
  console.log('\nShutting down Native HTTP server...');
  server.close(() => {
    process.exit(0);
  });
});

process.on('SIGTERM', () => {
  console.log('\nShutting down Native HTTP server...');
  server.close(() => {
    process.exit(0);
  });
});
