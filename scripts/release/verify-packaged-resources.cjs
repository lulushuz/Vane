const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const repoRoot = path.resolve(__dirname, '../..');

function getSha256(filePath) {
  const buf = fs.readFileSync(filePath);
  return crypto.createHash('sha256').update(buf).digest('hex');
}

function verifyPackagedResources(targetDir) {
  console.log(`=== Verifying Packaged Resources in: ${targetDir} ===`);
  const nativeManifest = JSON.parse(
    fs.readFileSync(path.join(repoRoot, 'src-tauri/security/native-artifacts.json'), 'utf8')
  );

  if (!fs.existsSync(targetDir)) {
    console.error(`❌ Target directory does not exist: ${targetDir}`);
    process.exit(1);
  }

  let errors = [];
  let debugFilesFound = [];
  let unexpectedExecutables = [];

  function scanDir(dir) {
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        scanDir(fullPath);
      } else if (entry.isFile()) {
        const ext = path.extname(entry.name).toLowerCase();
        
        // 1. Debug File Gate
        if (ext === '.pdb' || ext === '.map') {
          debugFilesFound.push(entry.name);
        }

        // 2. Executable Role Gate
        if (ext === '.exe' || ext === '.dll' || ext === '.sys') {
          const isKnownApp = entry.name.toLowerCase().includes('vane');
          const isManifestEntry = nativeManifest.artifacts.some(
            (a) => path.basename(a.relativePath).toLowerCase() === entry.name.toLowerCase()
          );

          if (!isKnownApp && !isManifestEntry) {
            unexpectedExecutables.push(entry.name);
          }
        }
      }
    }
  }

  scanDir(targetDir);

  if (debugFilesFound.length > 0) {
    errors.push(`Debug files detected in package: ${debugFilesFound.join(', ')}`);
  }

  if (unexpectedExecutables.length > 0) {
    errors.push(`Unexpected native executables in package: ${unexpectedExecutables.join(', ')}`);
  }

  if (errors.length > 0) {
    console.error('❌ PACKAGED RESOURCE VERIFICATION FAILED:');
    errors.forEach((e) => console.error(` - ${e}`));
    process.exit(1);
  }

  console.log('✅ Packaged resource verification passed. No debug artifacts or unexpected executables found.');
}

const targetDirArg = process.argv[2] || path.join(repoRoot, 'src-tauri/binaries');
verifyPackagedResources(targetDirArg);

const nsisBundleDir = path.join(repoRoot, 'src-tauri/target/release/bundle/nsis');
if (fs.existsSync(nsisBundleDir)) {
  verifyPackagedResources(nsisBundleDir);
}

