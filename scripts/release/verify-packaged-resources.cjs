const fs = require('fs');
const os = require('os');
const path = require('path');
const crypto = require('crypto');
const { execFileSync } = require('child_process');

const repoRoot = path.resolve(__dirname, '../..');
const args = process.argv.slice(2);
function option(name) { const index = args.indexOf(name); return index >= 0 ? args[index + 1] : undefined; }
function sha256(file) { return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex'); }
function walk(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isSymbolicLink()) throw new Error(`Package contains symbolic link: ${entry.name}`);
    if (entry.isDirectory()) walk(full, out); else if (entry.isFile()) out.push(full);
  }
  return out;
}
function findExtractor() {
  const requested = option('--extractor') || process.env.SEVEN_ZIP;
  const candidates = [requested, 'C:\\Program Files\\7-Zip\\7z.exe', 'C:\\Program Files (x86)\\7-Zip\\7z.exe', '7z'];
  for (const candidate of candidates.filter(Boolean)) {
    try { execFileSync(candidate, ['i'], { stdio: 'ignore' }); return candidate; } catch { /* continue */ }
  }
  throw new Error('Pinned 7-Zip extractor is required; pass --extractor or SEVEN_ZIP');
}
function verifyManifest(manifest, files, platform, extractDir) {
  const verified = [];
  for (const artifact of manifest.artifacts.filter((item) => item.required && (!item.platform || item.platform === platform))) {
    const normalized = artifact.relativePath.replaceAll('\\', '/').toLowerCase();
    const matches = files.filter((file) => {
      const relative = path.relative(extractDir, file).replaceAll('\\', '/').toLowerCase();
      return relative === normalized || relative.endsWith(`/${normalized}`);
    });
    if (matches.length !== 1) throw new Error(`${artifact.id}: expected one packaged occurrence, found ${matches.length}`);
    const stat = fs.lstatSync(matches[0]);
    if (!stat.isFile() || stat.size !== artifact.size) throw new Error(`${artifact.id}: size/type mismatch`);
    if (sha256(matches[0]) !== artifact.sha256) throw new Error(`${artifact.id}: SHA-256 mismatch`);
    verified.push({ id: artifact.id, relativePath: artifact.relativePath, status: 'verified', size: stat.size, sha256: artifact.sha256 });
  }
  return verified;
}
function main() {
  const installer = option('--installer');
  if (!installer || !fs.existsSync(installer) || path.extname(installer).toLowerCase() !== '.exe') {
    throw new Error('--installer must identify an existing NSIS .exe');
  }
  const extractor = findExtractor();
  const outputPath = path.resolve(option('--output') || path.join(repoRoot, 'artifacts/evidence/nsis-package-verification.json'));
  const startedAt = new Date().toISOString();
  const extractDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vane-nsis-'));
  try {
    execFileSync(extractor, ['x', '-y', `-o${extractDir}`, path.resolve(installer)], { stdio: 'pipe' });
    const files = walk(extractDir);
    if (!files.some((file) => /^(?:vane|vane-dpi)\.exe$/i.test(path.basename(file)))) throw new Error('Vane application executable missing');
    const forbidden = /\.(?:pdb|map|pem|key|pfx|env|log|rs|ts|tsx)$/i;
    const bad = files.filter((file) => forbidden.test(file) || file.replaceAll('\\', '/').includes('/node_modules/'));
    if (bad.length) throw new Error(`Unexpected package files: ${bad.map(path.basename).join(', ')}`);
    const nativeManifest = JSON.parse(fs.readFileSync(path.join(repoRoot, 'src-tauri/security/native-artifacts.json')));
    const contentManifest = JSON.parse(fs.readFileSync(path.join(repoRoot, 'src-tauri/security/content-artifacts.json')));
    const nativeArtifacts = verifyManifest(nativeManifest, files, 'windows-x86_64', extractDir);
    const contentArtifacts = verifyManifest(contentManifest, files, 'windows-x86_64', extractDir);
    for (const required of ['LICENSE', 'THIRD_PARTY_NOTICES.md']) {
      if (!files.some((file) => path.basename(file).toLowerCase() === required.toLowerCase())) throw new Error(`${required} missing`);
    }
    const sourceCommit = process.env.GITHUB_SHA || require('child_process').execFileSync('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, encoding: 'utf8' }).trim();
    const extractorVersion = execFileSync(extractor, ['i'], { encoding: 'utf8' }).match(/7-Zip\s+([\d.]+)/i)?.[1] || 'unknown';
    const evidence = { schemaVersion: 1, gate: 'nsis-package-verification', status: 'passed', sourceCommit,
      workflowRunId: process.env.GITHUB_RUN_ID || null, exitCode: 0, startedAt, completedAt: new Date().toISOString(),
      installer: { name: path.basename(installer), size: fs.statSync(installer).size, sha256: sha256(installer), authenticodeStatus: 'NotSigned' },
      extractor: { name: '7-Zip', version: extractorVersion, sha256: fs.existsSync(extractor) ? sha256(extractor) : null },
      nativeArtifacts, contentArtifacts, unexpectedFiles: [], extractedFileCount: files.length };
    fs.mkdirSync(path.dirname(outputPath), { recursive: true });
    fs.writeFileSync(outputPath, `${JSON.stringify(evidence, null, 2)}\n`);
  } finally { fs.rmSync(extractDir, { recursive: true, force: true }); }
}
try { main(); } catch (error) { console.error(error.message); process.exit(1); }
