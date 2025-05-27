import test from 'ava';
import axios from 'axios';

import { FileInfo, Server, cleanupRouter, forceCleanup, forceExit, getAsync, postAsync } from '../index';

const server = new Server({
  host: '127.0.0.1',
  port: 3002,
});

test.before(async t => {
  // 首先注册测试路由
  getAsync('/', async (err, req) => {
    if (err) {
      await req.setStatusCodeAsync(500);
      await req.sendErrorAsync('error');
    }
    await req.sendTextAsync('hello world');
  });

  getAsync('/json', async (err, req) => {
    if (err) {
      await req.setStatusCodeAsync(500);
      await req.sendErrorAsync('error');
    }
    await req.sendJsonAsync('{"message": "hello json"}');
  });

  postAsync('/json', async (err, req) => {
    if (err) {
      await req.setStatusCodeAsync(500);
      await req.sendErrorAsync('error');
    }
    await req.sendObjectAsync({ message: req.getBodyJson().message });
  });

  postAsync('/echo', async (err, req) => {
    if (err) {
      await req.setStatusCodeAsync(500);
      await req.sendErrorAsync('error');
    }
    const body = req.getBodyString();
    await req.sendTextAsync(`Echo: ${body}`);
  });

  // 增加一个处理表单的接口
  postAsync('/form', async (err, req) => {
    if (err) {
      await req.setStatusCodeAsync(500);
      await req.sendErrorAsync('error');
    }
    const formData = await req.getFormDataAsync();
    await req.sendObjectAsync(formData);
  });

  // 增加处理文件上传的接口
  postAsync('/upload', async (err, req) => {
    if (err) {
      await req.setStatusCodeAsync(500);
      await req.sendErrorAsync('error');
    }
    const fields = await req.getFormDataAsync();

    // 检查是否有文件上传
    if (fields['file']) {
      const fileField = fields['file'];

      // 检查是否为文件对象
      if (typeof fileField === 'object' && fileField && 'type' in fileField && fileField.type === 'file') {
        // 直接使用文件信息对象，具有完整的类型支持
        const fileInfo = fileField as FileInfo;
        await req.sendObjectAsync({
          originalName: fileInfo.originalName,
          filename: fileInfo.filename,
          path: fileInfo.path,
          size: fileInfo.size,
          contentType: fileInfo.contentType,
          type: fileInfo.type,
        });
      } else {
        // 如果是普通文本字段
        await req.sendObjectAsync({
          message: 'Not a file field',
          value: fileField,
        });
      }
    } else {
      await req.sendObjectAsync({
        error: 'No file uploaded',
      });
    }
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

// 测试post请求，传参为json格式的场景
test.serial('POST /json should return json', async t => {
  try {
    const res = await axios.post(
      'http://127.0.0.1:3002/json',
      { message: 'hello json' },
      {
        headers: { 'Content-Type': 'application/json' },
        timeout: 5000,
      },
    );
    t.is(res.data.message, 'hello json');
  } catch (error) {
    console.log('Error in POST /json', error);
    t.fail(`Request failed: ${error}`);
  }
});

// 测试post请求，传参为表单的场景
test.serial('POST /form should return form data', async t => {
  try {
    const res = await axios.post(
      'http://127.0.0.1:3002/form',
      { message: 'hello application/x-www-form-urlencoded' },
      {
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        timeout: 5000,
      },
    );
    t.is(res.data.message, 'hello application/x-www-form-urlencoded');
  } catch (error) {
    console.log('Error in POST /form application/x-www-form-urlencoded', error);
    t.fail(`Request failed: ${error}`);
  }

  // 再测试下form-data的场景
  const formData = new FormData();
  formData.append('message', 'hello multipart/form-data');
  try {
    const res = await axios.post('http://127.0.0.1:3002/form', formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
      timeout: 5000,
    });
    t.is(res.data.message, 'hello multipart/form-data');
  } catch (error) {
    console.log('Error in POST /form multipart/form-data', error);
    t.fail(`Request failed: ${error}`);
  }
});

// 测试文件上传
test.serial('POST /upload should handle file upload', async t => {
  try {
    // 创建一个简单的文本文件内容
    const fileContent = 'This is a test file content for upload';
    const formData = new FormData();

    // 创建一个 Blob 模拟文件
    const blob = new Blob([fileContent], { type: 'text/plain' });
    formData.append('file', blob, 'test.txt');
    formData.append('description', 'Test file upload');

    const res = await axios.post('http://127.0.0.1:3002/upload', formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
      timeout: 5000,
    });

    // 验证响应
    t.is(res.data.type, 'file');
    t.is(res.data.originalName, 'test.txt');
    t.is(res.data.size, fileContent.length);
    t.is(res.data.contentType, 'text/plain');
    t.truthy(res.data.filename); // UUID 生成的文件名
    t.truthy(res.data.path); // 文件路径
    t.is(res.data.path.startsWith('static/'), true);
  } catch (error) {
    console.log('Error in POST /upload', error);
    t.fail(`Request failed: ${error}`);
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
