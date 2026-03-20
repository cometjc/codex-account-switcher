const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
async function captureDevEntrypointArgs(argv) {
  const Module = require('node:module');
  const devEntrypointPath = path.join(process.cwd(), 'bin/codex-auth-dev.cjs');
  const originalLoad = Module._load;
  const originalArgv = process.argv;
  let capturedArgs = null;

  Module._load = function(request, parent, isMain) {
    if (request === '@oclif/core') {
      return {
        execute: (options) => {
          capturedArgs = options.args ?? null;
          return Promise.resolve(undefined);
        },
      };
    }

    return originalLoad(request, parent, isMain);
  };

  try {
    process.argv = [process.execPath, devEntrypointPath, ...argv];
    delete require.cache[devEntrypointPath];
    require(devEntrypointPath);
    await new Promise((resolve) => setImmediate(resolve));
  } finally {
    Module._load = originalLoad;
    process.argv = originalArgv;
    delete require.cache[devEntrypointPath];
  }

  return capturedArgs;
}

test('shared argv router sends empty argv to root', async () => {
  const {routeCliArgv} = await import(path.join(process.cwd(), 'dist/lib/route-cli-argv.js'));

  assert.deepEqual(routeCliArgv([]), ['root']);
  assert.deepEqual(routeCliArgv(['current']), ['current']);
});

test('dev entrypoint uses shared argv routing before execute', async () => {
  assert.deepEqual(await captureDevEntrypointArgs([]), ['root']);
  assert.deepEqual(await captureDevEntrypointArgs(['current']), ['current']);
});

test('dev entrypoint imports the shared argv router', () => {
  const source = fs.readFileSync(path.join(process.cwd(), 'bin/codex-auth-dev.cjs'), 'utf8');

  assert.match(source, /routeCliArgv/);
  assert.match(source, /args:\s*routeCliArgv\(process\.argv\.slice\(2\)\)/);
});
