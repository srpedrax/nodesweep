const fs = require('node:fs/promises');
const path = require('node:path');

class DirectorySizeService {
  async calculate(rootPath) {
    let total = 0;
    const pending = [rootPath];

    while (pending.length) {
      const current = pending.pop();
      let entries;
      try {
        entries = await fs.readdir(current, { withFileTypes: true });
      } catch (error) {
        if (error.code === 'ENOENT') continue;
        throw error;
      }

      for (const entry of entries) {
        const entryPath = path.join(current, entry.name);
        if (entry.isDirectory()) pending.push(entryPath);
        else if (entry.isFile()) total += (await fs.stat(entryPath)).size;
      }
    }

    return total;
  }
}

module.exports = DirectorySizeService;
