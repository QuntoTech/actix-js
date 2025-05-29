const fastify = require('fastify');
const cluster = require('cluster');
const os = require('os');

// 配置 worker 数量
const WORKER_COUNT = 20;

if (cluster.isMaster) {
  console.log(`Master ${process.pid} is running`);
  console.log(`Starting ${WORKER_COUNT} Fastify workers...`);

  // 启动 worker 进程
  for (let i = 0; i < WORKER_COUNT; i++) {
    cluster.fork();
  }

  // 监听 worker 退出事件
  cluster.on('exit', (worker, code, signal) => {
    console.log(`Worker ${worker.process.pid} died with code ${code} and signal ${signal}`);
    console.log('Starting a new worker...');
    cluster.fork();
  });

  // 优雅关闭处理
  const gracefulShutdown = () => {
    console.log('\nShutting down all workers...');
    for (const id in cluster.workers) {
      cluster.workers[id].kill();
    }
    process.exit(0);
  };

  process.on('SIGINT', gracefulShutdown);
  process.on('SIGTERM', gracefulShutdown);
} else {
  // Worker 进程
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

    try {
      const PORT = 3000;
      const HOST = '127.0.0.1';

      await server.listen({ port: PORT, host: HOST });
      console.log(`Worker ${process.pid} started Fastify server on http://${HOST}:${PORT}`);
    } catch (err) {
      console.error(`Worker ${process.pid} error:`, err);
      process.exit(1);
    }
  };

  start();
}
