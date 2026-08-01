#!/usr/bin/env node
import fs from 'fs';
import path from 'path';
import crypto from 'crypto';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const ROOT = path.resolve(__dirname, '../../');
const NATIVE_MANIFEST_PATH = path.join(ROOT, 'src-tauri/security/native-artifacts.json');
const CONTENT_MANIFEST_PATH = path.join(ROOT, 'src-tauri/security/content-artifacts.json');

const isWriteMode = process.argv.includes('--write');

function hashFile(filePath) {
  const fileBuffer = fs.readFileSync(filePath);
  const hashSum = crypto.createHash('sha256');
  hashSum.update(fileBuffer);
  return {
    size: fileBuffer.length,
    sha256: hashSum.digest('hex').toLowerCase(),
  };
}

console.log('=== Vane Artifact Integrity Manifest Audit ===\n');

// 1. Audit Native Artifacts
const nativeManifest = JSON.parse(fs.readFileSync(NATIVE_MANIFEST_PATH, 'utf8'));
let nativeModified = false;

for (const artifact of nativeManifest.artifacts) {
  const fullPath = path.join(ROOT, 'src-tauri', artifact.relativePath);
  if (!fs.existsSync(fullPath)) {
    console.error(`[ERROR] Native artifact missing: ${artifact.relativePath}`);
    process.exitCode = 1;
    continue;
  }

  const { size, sha256 } = hashFile(fullPath);
  if (artifact.size !== size || artifact.sha256.toLowerCase() !== sha256) {
    console.log(`[DIFF] ${artifact.id}:`);
    console.log(`  Size:   expected ${artifact.size}, actual ${size}`);
    console.log(`  SHA256: expected ${artifact.sha256}, actual ${sha256}`);
    artifact.size = size;
    artifact.sha256 = sha256;
    nativeModified = true;
  } else {
    console.log(`[OK] ${artifact.id} (${size} bytes, SHA-256 matches)`);
  }
}

// 2. Audit Content Artifacts
const contentManifest = JSON.parse(fs.readFileSync(CONTENT_MANIFEST_PATH, 'utf8'));
let contentModified = false;

for (const artifact of contentManifest.artifacts) {
  const fullPath = path.join(ROOT, artifact.relativePath);
  if (!fs.existsSync(fullPath)) {
    console.error(`[ERROR] Content artifact missing: ${artifact.relativePath}`);
    process.exitCode = 1;
    continue;
  }

  const { size, sha256 } = hashFile(fullPath);
  if (artifact.size !== size || artifact.sha256.toLowerCase() !== sha256) {
    console.log(`[DIFF] ${artifact.id}:`);
    console.log(`  Size:   expected ${artifact.size}, actual ${size}`);
    console.log(`  SHA256: expected ${artifact.sha256}, actual ${sha256}`);
    artifact.size = size;
    artifact.sha256 = sha256;
    contentModified = true;
  } else {
    console.log(`[OK] ${artifact.id} (${size} bytes, SHA-256 matches)`);
  }
}

if (isWriteMode) {
  if (nativeModified) {
    fs.writeFileSync(NATIVE_MANIFEST_PATH, JSON.stringify(nativeManifest, null, 2) + '\n');
    console.log('\n[WRITE] Updated native-artifacts.json');
  }
  if (contentModified) {
    fs.writeFileSync(CONTENT_MANIFEST_PATH, JSON.stringify(contentManifest, null, 2) + '\n');
    console.log('[WRITE] Updated content-artifacts.json');
  }
} else {
  if (nativeModified || contentModified) {
    console.log('\n[NOTICE] Manifest differences detected. Run with --write to update manifests.');
  } else {
    console.log('\n[PASS] All manifests are up to date.');
  }
}
