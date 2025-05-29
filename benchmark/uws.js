const uWS = require('uWebSockets.js');
const port = 9001;

const app = uWS
  .App({})
  .get('/', (res, req) => {
    res.end('Hello World');
  })
  .get('/json', (res, req) => {
    res
      .writeHeader('Content-Type', 'application/json')
      .end(JSON.stringify({ message: 'Hello JSON', timestamp: Date.now() }));
  })
  .post('/echo', (res, req) => {
    res.onData((chunk, isLast) => {
      if (isLast) {
        const body = Buffer.from(chunk).toString();
        res.writeHeader('Content-Type', 'application/json').end(JSON.stringify({ echo: body, timestamp: Date.now() }));
      }
    });

    res.onAborted(() => {
      console.log('Request aborted');
    });
  })
  .post('/json', (res, req) => {
    res.onData((chunk, isLast) => {
      if (isLast) {
        try {
          const body = Buffer.from(chunk).toString();
          const data = JSON.parse(body);
          res
            .writeHeader('Content-Type', 'application/json')
            .end(JSON.stringify({ received: data, timestamp: Date.now() }));
        } catch (e) {
          res
            .writeStatus('400 Bad Request')
            .writeHeader('Content-Type', 'application/json')
            .end(JSON.stringify({ error: 'Invalid JSON' }));
        }
      }
    });

    res.onAborted(() => {
      console.log('Request aborted');
    });
  })
  .listen(port, socket => {
    console.log(`Server is running on port ${port}`);
  });
