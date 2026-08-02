const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

const repoRoot = path.resolve(__dirname, '../..');
const evidenceDir = path.join(repoRoot, 'artifacts', 'evidence');
const requiredGates = [
  'frontend-tests', 'frontend-build', 'npm-audit', 'rust-lib-tests',
  'rust-all-targets', 'rust-all-features', 'cargo-fmt', 'clippy',
  'cargo-audit', 'version-parity', 'native-manifest', 'nsis-build',
  'nsis-package-verification', 'installer-checksum', 'secret-scan',
];

function sourceCommit() {
  return execFileSync('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, encoding: 'utf8' }).trim();
}

function readGate(gate, commit) {
  const file = path.join(evidenceDir, `${gate}.json`);
  if (!fs.existsSync(file)) return { gate, status: 'not-executed' };
  try {
    const value = JSON.parse(fs.readFileSync(file, 'utf8'));
    if (value.schemaVersion !== 1 || value.gate !== gate || typeof value.exitCode !== 'number'
      || typeof value.command !== 'string' || typeof value.completedAt !== 'string') {
      return { gate, status: 'invalid' };
    }
    if (value.commit !== commit) return { gate, status: 'stale' };
    if (value.exitCode !== 0 || value.status !== 'passed') return { gate, status: 'failed' };
    return { gate, status: 'passed', evidence: path.relative(repoRoot, file).replaceAll('\\', '/'), ...(value.artifactSha256 ? { artifactSha256: value.artifactSha256 } : {}) };
  } catch {
    return { gate, status: 'invalid' };
  }
}

function main() {
  const pkg = JSON.parse(fs.readFileSync(path.join(repoRoot, 'package.json'), 'utf8'));
  const commit = sourceCommit();
  const gates = Object.fromEntries(requiredGates.map((gate) => [gate, readGate(gate, commit)]));
  const unsignedReady = Object.values(gates).every((gate) => gate.status === 'passed');
  const manifest = {
    schemaVersion: 2,
    version: pkg.version,
    sourceCommit: commit,
    evidenceCommit: process.env.EVIDENCE_COMMIT || commit,
    workflowRunId: process.env.GITHUB_RUN_ID || null,
    artifactSha256: gates['installer-checksum'].artifactSha256 || null,
    generatedAt: new Date().toISOString(),
    gates,
    releaseDecision: unsignedReady ? 'READY FOR UNSIGNED WINDOWS TESTING' : 'BLOCKED',
    productionRelease: 'BLOCKED',
    remainingHumanBlockers: [
      'Windows Authenticode signing',
      'Tauri updater signing',
      'Windows 11 privileged VM acceptance',
    ],
  };
  fs.mkdirSync(path.join(repoRoot, 'artifacts'), { recursive: true });
  fs.writeFileSync(path.join(repoRoot, 'artifacts', 'release-readiness.json'), `${JSON.stringify(manifest, null, 2)}\n`);
  if (!unsignedReady) process.exitCode = 1;
}

main();
