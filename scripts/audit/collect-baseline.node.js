import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { execSync } from 'node:child_process';

function getFileHash(filePath) {
  if (!fs.existsSync(filePath)) return null;
  const content = fs.readFileSync(filePath);
  return crypto.createHash('sha256').update(content).digest('hex');
}

function runGit(cmd) {
  try {
    return execSync(`git ${cmd}`, { encoding: 'utf8' }).trim();
  } catch (err) {
    return 'UNKNOWN';
  }
}

function main() {
  const rootDir = path.resolve(process.cwd());
  const packageJsonPath = path.join(rootDir, 'package.json');
  const cargoTomlPath = path.join(rootDir, 'src-tauri', 'Cargo.toml');
  const tauriConfPath = path.join(rootDir, 'src-tauri', 'tauri.conf.json');

  const pkg = fs.existsSync(packageJsonPath) ? JSON.parse(fs.readFileSync(packageJsonPath, 'utf8')) : {};
  const tauriConf = fs.existsSync(tauriConfPath) ? JSON.parse(fs.readFileSync(tauriConfPath, 'utf8')) : {};

  const cargoTomlContent = fs.existsSync(cargoTomlPath) ? fs.readFileSync(cargoTomlPath, 'utf8') : '';
  const cargoVersionMatch = cargoTomlContent.match(/^version\s*=\s*"([^"]+)"/m);
  const cargoVersion = cargoVersionMatch ? cargoVersionMatch[1] : 'UNKNOWN';

  const branch = runGit('rev-parse --abbrev-ref HEAD');
  const commitSha = runGit('rev-parse HEAD');
  const commitDate = runGit('log -1 --format=%cd --date=iso');
  const commitMessage = runGit('log -1 --format=%s');

  const filesToHash = [
    'src-tauri/binaries/winws-x86_64-pc-windows-msvc.exe',
    'src-tauri/binaries/WinDivert64.sys',
    'src-tauri/binaries/WinDivert.dll',
    'src-tauri/binaries/cygwin1.dll',
    'src-tauri/binaries/nfqws-x86_64-unknown-linux-gnu',
    'presets/builtin.json',
    'presets/remote_template.json',
    'src-tauri/tauri.conf.json',
    'package.json',
    'src-tauri/Cargo.toml',
    'package-lock.json',
    'src-tauri/Cargo.lock'
  ];

  const hashManifest = {};
  filesToHash.forEach((relPath) => {
    const fullPath = path.join(rootDir, relPath);
    const hash = getFileHash(fullPath);
    hashManifest[relPath] = hash || 'FILE_NOT_FOUND';
  });

  const report = {
    schemaVersion: 1,
    purpose: 'P00 Production Hardening Baseline Audit Report',
    collectedAtUtc: new Date().toISOString(),
    repository: {
      name: pkg.name || 'vane-dpi',
      branch,
      commitSha,
      commitDate,
      commitMessage,
      license: 'GPL-3.0',
      visibility: 'public'
    },
    versions: {
      packageJson: pkg.version || 'UNKNOWN',
      cargoToml: cargoVersion,
      tauriConf: tauriConf.version || 'UNKNOWN',
      consistent: pkg.version === cargoVersion && pkg.version === tauriConf.version
    },
    suggestedBaselineTag: `baseline-${pkg.version || '2.1.4'}-2026-07-29`,
    hashManifest,
    workflows: [
      '.github/workflows/ci.yml',
      '.github/workflows/releases.yml',
      '.github/workflows/windows-acceptance-build.yml'
    ],
    keys: {
      tauriUpdaterPubkey: tauriConf.plugins?.updater?.pubkey || 'NONE',
      minisignSecurityPubkey: 'RWTo0iw8Ib18KoSGwlXjG4Hlz+oMjaFhN6077H5nNlTH6KuJogHeUra1'
    }
  };

  const outputDir = path.join(rootDir, 'artifacts', 'baseline');
  fs.mkdirSync(outputDir, { recursive: true });
  const outputPath = path.join(outputDir, 'baseline-report.json');
  fs.writeFileSync(outputPath, JSON.stringify(report, null, 2), 'utf8');

  console.log(`Baseline report written to: ${outputPath}`);
}

main();
