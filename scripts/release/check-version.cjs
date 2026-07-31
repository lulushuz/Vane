const fs = require('fs');
const path = require('path');

const repoRoot = path.resolve(__dirname, '../..');

function readJson(relPath) {
  const fullPath = path.join(repoRoot, relPath);
  if (!fs.existsSync(fullPath)) {
    throw new Error(`File not found: ${relPath}`);
  }
  return JSON.parse(fs.readFileSync(fullPath, 'utf8'));
}

function getCargoVersion() {
  const cargoPath = path.join(repoRoot, 'src-tauri/Cargo.toml');
  const content = fs.readFileSync(cargoPath, 'utf8');
  const match = content.match(/^version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error('Failed to parse version from src-tauri/Cargo.toml');
  }
  return match[1];
}

function checkVersionConsistency() {
  console.log('=== Checking Package & Engine Version Consistency ===');
  
  const pkgVersion = readJson('package.json').version;
  const tauriVersion = readJson('src-tauri/tauri.conf.json').version;
  const cargoVersion = getCargoVersion();

  console.log(`package.json version:          ${pkgVersion}`);
  console.log(`src-tauri/tauri.conf.json:    ${tauriVersion}`);
  console.log(`src-tauri/Cargo.toml:          ${cargoVersion}`);

  if (pkgVersion !== tauriVersion || pkgVersion !== cargoVersion) {
    console.error(`❌ VERSION DRIFT DETECTED! package=${pkgVersion}, tauri=${tauriVersion}, cargo=${cargoVersion}`);
    process.exit(1);
  }

  // Verify Native Artifacts Manifest Application Version
  const nativeManifest = readJson('src-tauri/security/native-artifacts.json');
  console.log(`native-artifacts.json appVersion: ${nativeManifest.applicationVersion}`);

  if (nativeManifest.applicationVersion !== pkgVersion) {
    console.error(`❌ NATIVE ARTIFACT MANIFEST VERSION MISMATCH! Expected ${pkgVersion}, got ${nativeManifest.applicationVersion}`);
    process.exit(1);
  }

  console.log(`✅ Version consistency verified: ${pkgVersion}`);
}

checkVersionConsistency();
