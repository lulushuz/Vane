const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const repoRoot = path.resolve(__dirname, '../..');
const args = process.argv.slice(2);
function option(name) { const index = args.indexOf(name); return index >= 0 ? args[index + 1] : undefined; }
function hash(file) { return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex'); }
function main() {
  const installer = option('--installer');
  if (!installer || !fs.existsSync(installer)) throw new Error('Existing --installer is required');
  const artifacts = path.join(repoRoot, 'artifacts');
  const candidates = [path.resolve(installer), path.join(artifacts, `sbom-${require(path.join(repoRoot, 'package.json')).version}.spdx.json`),
    path.join(artifacts, 'release-readiness.json'), path.join(artifacts, 'evidence-manifest.json')];
  const updaterDir = path.dirname(path.resolve(installer));
  for (const entry of fs.readdirSync(updaterDir)) if (/\.(?:zip|tar\.gz|sig)$/i.test(entry)) candidates.push(path.join(updaterDir, entry));
  const existing = [...new Set(candidates.filter((file) => fs.existsSync(file)).map((file) => path.resolve(file)))];
  if (!existing.includes(path.resolve(installer))) throw new Error('Installer omitted from checksum input');
  const names = new Set();
  const records = existing.map((file) => {
    const name = file.startsWith(artifacts) ? path.relative(artifacts, file).replaceAll('\\', '/') : `installer/${path.basename(file)}`;
    if (names.has(name)) throw new Error(`Duplicate checksum name: ${name}`); names.add(name);
    return { name, file };
  }).sort((a, b) => a.name.localeCompare(b.name));
  fs.mkdirSync(artifacts, { recursive: true });
  fs.writeFileSync(path.join(artifacts, 'SHA256SUMS'), `${records.map(({ name, file }) => `${hash(file)}  ${name}`).join('\n')}\n`);
  const evidenceDir = path.join(artifacts, 'evidence'); fs.mkdirSync(evidenceDir, { recursive: true });
  const commit = require('child_process').execFileSync('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, encoding: 'utf8' }).trim();
  fs.writeFileSync(path.join(evidenceDir, 'installer-checksum.json'), `${JSON.stringify({ schemaVersion: 1, gate: 'installer-checksum', status: 'passed', command: 'generate-checksums --installer', exitCode: 0, completedAt: new Date().toISOString(), commit, artifactSha256: hash(installer), files: records.map((item) => item.name) }, null, 2)}\n`);
}
try { main(); } catch (error) { console.error(error.message); process.exit(1); }
