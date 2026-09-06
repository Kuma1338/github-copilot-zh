import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

test('macOS package contains runnable installer scripts and excludes its checksum file', async () => {
  const source = await readFile(new URL('../scripts/package-macos.sh', import.meta.url), 'utf8');

  assert.match(source, /mkdir -p .*app_root.*scripts/);
  assert.match(source, /cp "scripts\/install-macos\.sh" "scripts\/uninstall-macos\.sh".*\$\{artifact_root\}\/scripts\//);
  assert.match(source, /find \. -type f .*SHA256SUMS\.txt/);
  assert.match(source, /-not -path ['"]\.\/SHA256SUMS\.txt['"]/);
});

test('macOS release builds pin the supported deployment target and architectures', async () => {
  const packageSource = await readFile(new URL('../scripts/package-macos.sh', import.meta.url), 'utf8');
  const workflowSource = await readFile(new URL('../.github/workflows/build-release.yml', import.meta.url), 'utf8');

  assert.match(packageSource, /MACOSX_DEPLOYMENT_TARGET=.*12\.0/);
  assert.match(workflowSource, /runner: macos-15\s+target: aarch64-apple-darwin/);
  assert.match(workflowSource, /runner: macos-15-intel\s+target: x86_64-apple-darwin/);
});
