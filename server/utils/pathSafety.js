const path = require('node:path');
const fs = require('node:fs/promises');
const HttpError = require('./httpError');

function normalized(value) {
  return path.resolve(value).replace(/[\\/]+$/, '').toLowerCase();
}

function dangerousPaths() {
  const values = [path.parse(process.cwd()).root];

  if (process.platform === 'win32') {
    const drive = path.parse(process.cwd()).root;
    values.push(
      path.join(drive, 'Users'),
      path.join(drive, 'Windows'),
      path.join(drive, 'Program Files'),
      path.join(drive, 'Program Files (x86)')
    );
  } else {
    values.push('/usr', '/etc', '/var', '/home');
  }

  return new Set(values.map(normalized));
}

function assertNonDangerousPath(targetPath) {
  if (typeof targetPath !== 'string' || targetPath.trim() === '') {
    throw new HttpError(400, 'Every cleanup path must be a non-empty string.');
  }

  const resolved = path.resolve(targetPath);
  if (dangerousPaths().has(normalized(resolved))) {
    throw new HttpError(400, `Refusing dangerous path: ${targetPath}`);
  }
  return resolved;
}

async function assertSafeNodeModulesPath(targetPath) {
  const resolved = assertNonDangerousPath(targetPath);
  if (path.basename(resolved).toLowerCase() !== 'node_modules') {
    throw new HttpError(400, `Only node_modules directories can be deleted: ${targetPath}`);
  }

  let stats;
  try {
    stats = await fs.lstat(resolved);
  } catch (error) {
    if (error.code === 'ENOENT') throw new HttpError(404, `Path does not exist: ${targetPath}`);
    throw error;
  }

  if (!stats.isDirectory() || stats.isSymbolicLink()) {
    throw new HttpError(400, `Cleanup target must be a real directory: ${targetPath}`);
  }

  const projectPath = path.dirname(resolved);
  const root = path.parse(resolved).root;
  const relativeParts = path.relative(root, resolved).split(path.sep).filter(Boolean);
  let current = root;
  for (const part of relativeParts) {
    current = path.join(current, part);
    const currentStats = await fs.lstat(current);
    if (currentStats.isSymbolicLink()) {
      throw new HttpError(400, `Cleanup paths cannot contain symbolic links or junctions: ${targetPath}`);
    }
  }

  try {
    const packageStats = await fs.lstat(path.join(projectPath, 'package.json'));
    if (!packageStats.isFile() || packageStats.isSymbolicLink()) throw new Error('not a regular file');
  } catch {
    throw new HttpError(400, `node_modules is not inside an identified Node.js project: ${targetPath}`);
  }

  const canonical = await fs.realpath(resolved);
  return { resolved, canonical: normalized(canonical) };
}

module.exports = { assertNonDangerousPath, assertSafeNodeModulesPath };
