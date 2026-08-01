const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const repoRoot = path.resolve(__dirname, '../..');

function computeSha256(filePath) {
  const buf = fs.readFileSync(filePath);
  return crypto.createHash('sha256').update(buf).digest('hex');
}

function generateChecksums(targetDir, outputFile) {
  console.log(`=== Generating Checksums for artifacts in ${targetDir} ===`);
  if (!fs.existsSync(targetDir)) {
    console.warn(`Target directory does not exist: ${targetDir}. Skipping checksum generation.`);
    return;
  }

  const lines = [];
  const entries = fs.readdirSync(targetDir, { withFileTypes: true });

  for (const entry of entries) {
    if (entry.isFile() && !entry.name.endsWith('.sha256') && entry.name !== 'SHA256SUMS') {
      const fullPath = path.join(targetDir, entry.name);
      const hash = computeSha256(fullPath);
      lines.push(`${hash}  ${entry.name}`);
    }
  }

  const outputContent = lines.join('\n') + '\n';
  const outPath = outputFile || path.join(targetDir, 'SHA256SUMS');
  fs.writeFileSync(outPath, outputContent, 'utf8');
  console.log(`✅ Generated checksums file at ${outPath} (${lines.length} artifacts hashed)`);
}

const targetDirArg = process.argv[2] || path.join(repoRoot, 'artifacts');
const outArg = process.argv[3];
generateChecksums(targetDirArg, outArg);
