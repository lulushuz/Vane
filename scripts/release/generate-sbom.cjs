const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { execFileSync } = require('child_process');
const repoRoot = path.resolve(__dirname, '../..');
const cleanId = (value) => value.replace(/[^A-Za-z0-9.-]/g, '-');
function cargoPackages() {
  const text = fs.readFileSync(path.join(repoRoot, 'src-tauri/Cargo.lock'), 'utf8');
  return text.split('[[package]]').slice(1).map((block) => {
    const read = (key) => block.match(new RegExp(`^${key} = "([^"]+)"`, 'm'))?.[1];
    return { name: read('name'), version: read('version'), checksum: read('checksum') };
  }).filter((item) => item.name && item.version);
}
function npmPackages() {
  const lock = JSON.parse(fs.readFileSync(path.join(repoRoot, 'package-lock.json')));
  return Object.entries(lock.packages || {}).filter(([key]) => key.startsWith('node_modules/')).map(([key, value]) => ({ name: value.name || key.split('node_modules/').pop(), version: value.version, license: value.license, integrity: value.integrity }));
}
function main() {
  const pkg = JSON.parse(fs.readFileSync(path.join(repoRoot, 'package.json')));
  const commit = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, encoding: 'utf8' }).trim();
  const packages = [{ SPDXID: 'SPDXRef-Root', name: pkg.name, versionInfo: pkg.version, downloadLocation: 'NOASSERTION', licenseConcluded: pkg.license || 'NOASSERTION', externalRefs: [{ referenceCategory: 'OTHER', referenceType: 'vcs', referenceLocator: `git+https://github.com/lulushuz/Vane@${commit}` }] }];
  for (const item of cargoPackages()) packages.push({ SPDXID: `SPDXRef-Cargo-${cleanId(item.name)}-${cleanId(item.version)}`, name: item.name, versionInfo: item.version, downloadLocation: 'NOASSERTION', licenseConcluded: 'NOASSERTION', externalRefs: [{ referenceCategory: 'PACKAGE-MANAGER', referenceType: 'purl', referenceLocator: `pkg:cargo/${item.name}@${item.version}` }], ...(item.checksum ? { checksums: [{ algorithm: 'SHA256', checksumValue: item.checksum }] } : {}) });
  for (const item of npmPackages()) packages.push({ SPDXID: `SPDXRef-Npm-${cleanId(item.name)}-${cleanId(item.version || 'unknown')}`, name: item.name, versionInfo: item.version || 'NOASSERTION', downloadLocation: 'NOASSERTION', licenseConcluded: item.license || 'NOASSERTION', externalRefs: [{ referenceCategory: 'PACKAGE-MANAGER', referenceType: 'purl', referenceLocator: `pkg:npm/${encodeURIComponent(item.name)}@${item.version || 'unknown'}` }] });
  const native = JSON.parse(fs.readFileSync(path.join(repoRoot, 'src-tauri/security/native-artifacts.json')));
  for (const item of native.artifacts) packages.push({ SPDXID: `SPDXRef-Native-${cleanId(item.id)}`, name: item.component || item.id, versionInfo: item.componentVersion || 'NOASSERTION', downloadLocation: 'NOASSERTION', licenseConcluded: item.license || 'NOASSERTION', checksums: [{ algorithm: 'SHA256', checksumValue: item.sha256 }] });
  const relationships = packages.slice(1).map((item) => ({ spdxElementId: 'SPDXRef-Root', relationshipType: 'DEPENDS_ON', relatedSpdxElement: item.SPDXID }));
  const sbom = { spdxVersion: 'SPDX-2.3', dataLicense: 'CC0-1.0', SPDXID: 'SPDXRef-DOCUMENT', name: `${pkg.name}-${pkg.version}-SBOM`, documentNamespace: `https://vane.invalid/spdx/${pkg.version}/${commit}`, creationInfo: { creators: ['Tool: Vane lockfile SBOM generator 2'], created: new Date().toISOString() }, documentDescribes: ['SPDXRef-Root'], packages, relationships, annotations: [{ annotationType: 'OTHER', annotator: 'Tool: Vane lockfile SBOM generator 2', annotationDate: new Date().toISOString(), comment: 'Sources: package-lock.json, src-tauri/Cargo.lock, native-artifacts.json' }] };
  const out = path.join(repoRoot, 'artifacts', `sbom-${pkg.version}.spdx.json`); fs.mkdirSync(path.dirname(out), { recursive: true }); fs.writeFileSync(out, `${JSON.stringify(sbom, null, 2)}\n`);
  if (packages.length < 100) throw new Error(`SBOM dependency coverage unexpectedly small: ${packages.length}`);
}
try { main(); } catch (error) { console.error(error.message); process.exit(1); }
