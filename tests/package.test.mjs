import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const projectRoot = fileURLToPath(new URL('..', import.meta.url));

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

test('tracked files exclude local user and custom installation paths', async () => {
  const { stdout } = await execFileAsync('git', ['ls-files', '-z'], {
    cwd: projectRoot,
    encoding: 'utf8',
  });
  const offenders = [];

  for (const relativePath of stdout.split('\0').filter(Boolean)) {
    const contents = await readFile(join(projectRoot, relativePath));
    if (contents.includes(0)) continue;
    const text = contents.toString('utf8');
    if (/C:\\Users\\(?!<)|\/Users\/(?!<)|D:\\GitHub Copilot/i.test(text)) {
      offenders.push(relativePath);
    }
  }

  assert.deepEqual(offenders, []);
});

test('Windows installer discovers custom installs through the official shortcut', async () => {
  const source = await readFile(new URL('../scripts/install.ps1', import.meta.url), 'utf8');

  assert.match(source, /GitHub Copilot\.lnk/);
  assert.match(source, /CreateShortcut/);
});

test('release checksum file uses downloadable asset names', async () => {
  const source = await readFile(new URL('../.github/workflows/build-release.yml', import.meta.url), 'utf8');

  assert.match(source, /cd release-assets\s+sha256sum \*\.zip > SHA256SUMS\.txt/);
  assert.doesNotMatch(source, /sha256sum "\$\{assets\[@\]\}"/);
});

test('Windows package carries its release version into the install manifest', async () => {
  const packageSource = await readFile(new URL('../scripts/package.ps1', import.meta.url), 'utf8');
  const installerSource = await readFile(new URL('../scripts/install.ps1', import.meta.url), 'utf8');

  assert.match(packageSource, /Join-Path \$artifactRoot 'VERSION'/);
  assert.match(installerSource, /Join-Path \$SourceRoot 'VERSION'/);
  assert.match(installerSource, /version = \$packageVersion/);
});

test('macOS installer stages, configures, signs, and atomically replaces the launcher', async () => {
  const source = await readFile(new URL('../scripts/install-macos.sh', import.meta.url), 'utf8');
  const stagePosition = source.indexOf('"$source_app" "$staging_root"');
  const configPosition = source.indexOf('> "$config_path"');
  const signPosition = source.indexOf('codesign --force --deep --sign - "$staging_root"');
  const installPosition = source.indexOf('mv -- "$staging_root" "$install_root"');

  assert.match(source, /backup_root=/);
  assert.ok(stagePosition >= 0);
  assert.ok(stagePosition < configPosition);
  assert.ok(configPosition < signPosition);
  assert.ok(signPosition < installPosition);
});

test('release metadata describes the macOS limitation without an absolute ownership claim', async () => {
  const workflowSource = await readFile(new URL('../.github/workflows/build-release.yml', import.meta.url), 'utf8');
  const readmeSource = await readFile(new URL('../README.md', import.meta.url), 'utf8');

  assert.match(workflowSource, /macOS[^\n]*实验/);
  assert.doesNotMatch(readmeSource, /不包含任何 GitHub 专有资源/);
  assert.match(readmeSource, /不含官方 Copilot 可执行文件/);
});
