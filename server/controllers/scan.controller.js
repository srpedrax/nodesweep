const ScanService = require('../services/ScanService');

const scanService = new ScanService();

async function scan(req, res, next) {
  try {
    res.json(await scanService.scan(req.body?.rootPath));
  } catch (error) {
    next(error);
  }
}

module.exports = { scan };
