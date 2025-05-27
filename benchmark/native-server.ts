import fastify from 'fastify';

// 启动服务器
const start = async () => {
  // 创建 Fastify 实例
  const server = fastify({
    logger: false,
  });

  // 配置 CORS
  await server.register(require('@fastify/cors'), {
    origin: '*',
    methods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
  });

  // 路由处理
  server.get('/', async (request, reply) => {
    return 'Hello World';
  });

  server.get('/json', async (request, reply) => {
    reply.type('application/json');
    return { message: 'Hello JSON', timestamp: Date.now() };
  });

  server.post('/echo', async (request, reply) => {
    reply.type('application/json');
    return { echo: request.body, timestamp: Date.now() };
  });

  server.post('/json', async (request, reply) => {
    reply.type('application/json');
    return { received: request.body, timestamp: Date.now() };
  });

  // 优雅关闭处理
  const gracefulShutdown = async () => {
    console.log('\nShutting down Fastify server...');
    await server.close();
    process.exit(0);
  };

  process.on('SIGINT', gracefulShutdown);
  process.on('SIGTERM', gracefulShutdown);

  try {
    const PORT = 3000;
    const HOST = '127.0.0.1';

    await server.listen({ port: PORT, host: HOST });
    console.log(`Starting Fastify server on http://${HOST}:${PORT}`);
  } catch (err) {
    console.error(err);
    process.exit(1);
  }
};

start();
