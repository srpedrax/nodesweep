const express = require('express');
const path = require('node:path');
const scanRoutes = require('./routes/scan.routes');
const cleanupRoutes = require('./routes/cleanup.routes');

function createApp() {
  const app = express();
  app.disable('x-powered-by');
  app.use(express.json({ limit: '32kb' }));
  app.use('/api/scan', scanRoutes);
  app.use('/api/cleanup', cleanupRoutes);
  const clientDist = path.join(__dirname, '..', 'client', 'dist');
  app.use(express.static(clientDist));
  app.get(/^(?!\/api(?:\/|$)).*/, (req, res, next) => {
    res.sendFile(path.join(clientDist, 'index.html'), (error) => {
      if (error) next();
    });
  });
  app.use((req, res) => res.status(404).json({ error: 'Route not found.' }));
  app.use((error, req, res, next) => {
    if (res.headersSent) return next(error);
    if (error instanceof SyntaxError && error.status === 400 && 'body' in error) {
      return res.status(400).json({ error: 'Invalid JSON body.' });
    }
    const status = Number.isInteger(error.status) ? error.status : 500;
    const body = { error: status === 500 ? 'Internal server error.' : error.message };
    if (error.details) body.details = error.details;
    return res.status(status).json(body);
  });
  return app;
}

if (require.main === module) {
  const port = Number(process.env.PORT) || 3000;
  createApp().listen(port, () => console.log(`NodeSweep listening on http://localhost:${port}`));
}

module.exports = { createApp };
