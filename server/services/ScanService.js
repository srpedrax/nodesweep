const fs = require('node:fs/promises');
const path = require('node:path');
const DirectorySizeService = require('./DirectorySizeService');
const HttpError = require('../utils/httpError');

class ScanService {
  constructor(directorySizeService = new DirectorySizeService()) {
    this.directorySizeService = directorySizeService;
  }

  async scan(rootPath) {
    if (typeof rootPath !== 'string' || rootPath.trim() === '') {
      throw new HttpError(400, 'rootPath must be a non-empty string.');
    }

    const root = path.resolve(rootPath);
    let rootStats;
    try {
      rootStats = await fs.stat(root);
    } catch (error) {
      if (error.code === 'ENOENT') throw new HttpError(404, `Root path does not exist: ${rootPath}`);
      throw error;
    }
    if (!rootStats.isDirectory()) throw new HttpError(400, 'rootPath must point to a directory.');

    const projects = [];
    const pending = [root];

    while (pending.length) {
      const current = pending.pop();
      let entries;
      try {
        entries = await fs.readdir(current, { withFileTypes: true });
      } catch (error) {
        if (error.code === 'EACCES' || error.code === 'EPERM') continue;
        throw error;
      }

      const hasPackageJson = entries.some((entry) => entry.isFile() && entry.name === 'package.json');
      const nodeModules = entries.find((entry) => entry.isDirectory() && entry.name === 'node_modules');

      if (hasPackageJson && nodeModules) {
        const nodeModulesPath = path.join(current, nodeModules.name);
        const [sizeBytes, stats] = await Promise.all([
          this.directorySizeService.calculate(nodeModulesPath),
          fs.stat(nodeModulesPath)
        ]);
        projects.push({
          name: path.basename(current),
          path: current,
          nodeModulesPath,
          sizeBytes,
          lastModified: stats.mtime.toISOString()
        });
      }

      for (const entry of entries) {
        if (entry.isDirectory() && entry.name !== 'node_modules') {
          pending.push(path.join(current, entry.name));
        }
      }
    }

    projects.sort((a, b) => b.sizeBytes - a.sizeBytes || a.path.localeCompare(b.path));
    return { projects, totalSizeBytes: projects.reduce((sum, project) => sum + project.sizeBytes, 0) };
  }
}

module.exports = ScanService;
