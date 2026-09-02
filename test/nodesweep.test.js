const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const ScanService = require('../server/services/ScanService');
const CleanupService = require('../server/services/CleanupService');
const DirectorySizeService = require('../server/services/DirectorySizeService');
const { createApp } = require('../server/app');

async function fixture() {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'nodesweep-'));
  const project = path.join(root, 'project-a');
  const modules = path.join(project, 'node_modules');
  await fs.mkdir(path.join(modules, 'dep'), { recursive: true });
  await fs.writeFile(path.join(project, 'package.json'), '{}');
  await fs.writeFile(path.join(modules, 'dep', 'a.txt'), '12345');
  return { root, project, modules };
}

test('DirectorySizeService sums nested regular files', async (t) => {
  const data = await fixture();
  t.after(() => fs.rm(data.root, { recursive: true, force: true }));
  assert.equal(await new DirectorySizeService().calculate(data.modules), 5);
});

test('ScanService finds projects, totals sizes, and ignores projects inside node_modules', async (t) => {
  const data = await fixture();
  t.after(() => fs.rm(data.root, { recursive: true, force: true }));
  const fakeNested = path.join(data.modules, 'dep', 'nested');
  await fs.mkdir(path.join(fakeNested, 'node_modules'), { recursive: true });
  await fs.writeFile(path.join(fakeNested, 'package.json'), '{}');
  await fs.writeFile(path.join(fakeNested, 'node_modules', 'hidden.txt'), 'hidden');
  await fs.mkdir(path.join(data.root, 'not-a-project', 'node_modules'), { recursive: true });

  const result = await new ScanService().scan(data.root);
  assert.equal(result.projects.length, 1);
  assert.equal(result.projects[0].name, 'project-a');
  assert.equal(result.projects[0].nodeModulesPath, data.modules);
  assert.equal(result.projects[0].sizeBytes, 13);
  assert.equal(result.totalSizeBytes, 13);
  assert.match(result.projects[0].lastModified, /^\d{4}-\d{2}-\d{2}T/);
});

test('ScanService validates missing and non-directory roots', async (t) => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'nodesweep-'));
  t.after(() => fs.rm(root, { recursive: true, force: true }));
  const file = path.join(root, 'file.txt');
  await fs.writeFile(file, 'x');
  await assert.rejects(() => new ScanService().scan(''), { status: 400 });
  await assert.rejects(() => new ScanService().scan(path.join(root, 'missing')), { status: 404 });
  await assert.rejects(() => new ScanService().scan(file), { status: 400 });
});

test('CleanupService deletes only a validated project node_modules and reports bytes', async (t) => {
  const data = await fixture();
  t.after(() => fs.rm(data.root, { recursive: true, force: true }));
  const result = await new CleanupService().cleanup([data.modules]);
  assert.deepEqual(result, { deleted: [{ path: data.modules, freedBytes: 5 }], totalFreedBytes: 5 });
  await assert.rejects(() => fs.access(data.modules));
  await fs.access(path.join(data.project, 'package.json'));
});

test('CleanupService validates the entire batch before deleting', async (t) => {
  const data = await fixture();
  t.after(() => fs.rm(data.root, { recursive: true, force: true }));
  await assert.rejects(
    () => new CleanupService().cleanup([data.modules, path.join(data.project, 'src')]),
    { status: 400 }
  );
  await fs.access(data.modules);
});

test('CleanupService rejects duplicates and node_modules without package.json', async (t) => {
  const data = await fixture();
  t.after(() => fs.rm(data.root, { recursive: true, force: true }));
  const orphan = path.join(data.root, 'orphan', 'node_modules');
  await fs.mkdir(orphan, { recursive: true });
  await assert.rejects(() => new CleanupService().cleanup([data.modules, data.modules]), { status: 400 });
  await assert.rejects(() => new CleanupService().cleanup([orphan]), { status: 400 });
});

test('CleanupService rejects a symbolic package manifest', async (t) => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'nodesweep-'));
  t.after(() => fs.rm(root, { recursive: true, force: true }));
  const project = path.join(root, 'project');
  const modules = path.join(project, 'node_modules');
  const manifest = path.join(root, 'manifest.json');
  await fs.mkdir(modules, { recursive: true });
  await fs.writeFile(manifest, '{}');
  try {
    await fs.symlink(manifest, path.join(project, 'package.json'), 'file');
  } catch (error) {
    if (error.code === 'EPERM') return t.skip('Creating symlinks requires additional Windows privileges.');
    throw error;
  }
  await assert.rejects(() => new CleanupService().cleanup([modules]), { status: 400 });
  await fs.access(modules);
});

test('HTTP API scans, requires confirmation, and cleans up', async (t) => {
  const data = await fixture();
  const server = createApp().listen(0);
  t.after(async () => {
    await new Promise((resolve) => server.close(resolve));
    await fs.rm(data.root, { recursive: true, force: true });
  });
  await new Promise((resolve) => server.once('listening', resolve));
  const base = `http://127.0.0.1:${server.address().port}`;

  const scanResponse = await fetch(`${base}/api/scan`, {
    method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ rootPath: data.root })
  });
  assert.equal(scanResponse.status, 200);
  assert.equal((await scanResponse.json()).totalSizeBytes, 5);

  const denied = await fetch(`${base}/api/cleanup`, {
    method: 'DELETE', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ paths: [data.modules] })
  });
  assert.equal(denied.status, 400);

  const cleanup = await fetch(`${base}/api/cleanup`, {
    method: 'DELETE', headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ paths: [data.modules], confirmed: true })
  });
  assert.equal(cleanup.status, 200);
  assert.equal((await cleanup.json()).totalFreedBytes, 5);
});
