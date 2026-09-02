const CleanupService = require('../services/CleanupService');
const HttpError = require('../utils/httpError');

const cleanupService = new CleanupService();

async function cleanup(req, res, next) {
  try {
    if (req.body?.confirmed !== true) {
      throw new HttpError(400, 'Cleanup requires explicit confirmation with confirmed: true.');
    }
    res.json(await cleanupService.cleanup(req.body?.paths));
  } catch (error) {
    next(error);
  }
}

module.exports = { cleanup };
