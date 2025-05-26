import test from 'ava';
import axios from 'axios';

import { Server, cleanupRouter, forceCleanup, forceExit, get, post } from '../index';

const server = new Server({
  host: '127.0.0.1',
  port: 3002,
});

test.before(async t => {
  // 首先注册测试路由
  get('/', (err, req) => {
    if (err) {
      req.setStatusCode(500);
      req.sendError('error');
    }
    req.sendText('hello world');
  });

  get('/json', (err, req) => {
    if (err) {
      req.setStatusCode(500);
      req.sendError('error');
    }
    req.sendJson('{"message": "hello json"}');
  });

  post('/echo', (err, req) => {
    if (err) {
      req.setStatusCode(500);
      req.sendError('error');
    }
    const body = req.getBodyString();
    req.sendText(`Echo: ${body}`);
  });

  // 然后启动服务器
  server.start();

  // 等待服务器完全启动
  await new Promise(resolve => setTimeout(resolve, 3000));
});

test.serial('GET / should return hello world', async t => {
  try {
    const res = await axios.get('http://127.0.0.1:3002/', { timeout: 5000 });
    t.is(res.data, 'hello world');
  } catch (error) {
    console.log('Error in GET /', error);
    t.fail(`Request failed: ${error}`);
  }
});

test.serial('GET /json should return json', async t => {
  try {
    const res = await axios.get('http://127.0.0.1:3002/json', { timeout: 5000 });
    t.is(res.data.message, 'hello json');
  } catch (error) {
    console.log('Error in GET /json', error);
    t.fail(`Request failed: ${error}`);
  }
});

test.serial('POST /echo should echo request body', async t => {
  try {
    const res = await axios.post('http://127.0.0.1:3002/echo', 'test message', {
      headers: { 'Content-Type': 'text/plain' },
      timeout: 5000,
    });
    t.is(res.data, 'Echo: test message');
  } catch (error) {
    console.log('Error in POST /echo', error);
    t.fail(`Request failed: ${error}`);
  }
});

test.serial('404 for unknown routes', async t => {
  try {
    await axios.get('http://127.0.0.1:3002/unknown');
    t.fail('Should have thrown 404');
  } catch (error: any) {
    t.is(error.response.status, 404);
  }
});

test.after(async t => {
  cleanupRouter();
  await server.stop(); // 添加await等待服务器真正停止

  // 强制清理所有资源
  try {
    forceCleanup();
  } catch (e) {
    console.log('Force cleanup failed:', e);
  }

  // 强制垃圾收集
  if (global.gc) {
    global.gc();
  }

  // 等待清理完成
  await new Promise(resolve => setTimeout(resolve, 200));

  // 强制退出进程（如果其他方法不能让进程正常退出）
  forceExit();
});
