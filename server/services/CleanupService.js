const fs = require('node:fs/promises');
const DirectorySizeService = require('./DirectorySizeService');
const { assertSafeNodeModulesPath } = require('../utils/pathSafety');
const HttpError = require('../utils/httpError');

class CleanupService {
  constructor(directorySizeService = new DirectorySizeService()) {
    this.directorySizeService = directorySizeService;
  }

  async cleanup(paths) {
    if (!Array.isArray(paths) || paths.length === 0) {
      throw new HttpError(400, 'paths must be a non-empty array.');
    }
    // Validate the complete batch before deleting anything.
    const targets = [];
    for (const targetPath of paths) targets.push(await assertSafeNodeModulesPath(targetPath));
    const canonicalPaths = targets.map((target) => target.canonical);
    if (new Set(canonicalPaths).size !== canonicalPaths.length) {
      throw new HttpError(400, 'paths must not contain duplicates.');
    }

    const deleted = [];
    for (const target of targets) {
      const targetPath = target.resolved;
      const sizeBefore = await this.directorySizeService.calculate(targetPath);
      // Narrow the time-of-check/time-of-use window immediately before fs.rm.
      const revalidated = await assertSafeNodeModulesPath(targetPath);
      if (revalidated.canonical !== target.canonical) {
        throw new HttpError(409, `Cleanup target changed during the operation: ${targetPath}`);
      }
      await fs.rm(targetPath, { recursive: true, force: false });
      let sizeAfter = 0;
      try {
        sizeAfter = await this.directorySizeService.calculate(targetPath);
      } catch (error) {
        if (error.code !== 'ENOENT') throw error;
      }
      deleted.push({ path: targetPath, freedBytes: Math.max(0, sizeBefore - sizeAfter) });
    }

    return { deleted, totalFreedBytes: deleted.reduce((sum, item) => sum + item.freedBytes, 0) };
  }
}

module.exports = CleanupService;
