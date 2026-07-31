const fs = require('fs');
const path = require('path');

const repoRoot = path.resolve(__dirname, '../..');

function generateSbom() {
  console.log('=== Generating Machine-Readable SBOM (SPDX JSON format) ===');
  const pkg = JSON.parse(fs.readFileSync(path.join(repoRoot, 'package.json'), 'utf8'));
  const nativeManifest = JSON.parse(
    fs.readFileSync(path.join(repoRoot, 'src-tauri/security/native-artifacts.json'), 'utf8')
  );

  const packages = [
    {
      SPDXID: 'SPDXRef-RootPackage',
      name: pkg.name,
      versionInfo: pkg.version,
      downloadLocation: 'NOASSERTION',
      licenseConcluded: pkg.license || 'MIT',
      supplier: 'Organization: Archey DPI Engineering Team',
    },
  ];

  for (const artifact of nativeManifest.artifacts) {
    packages.push({
      SPDXID: `SPDXRef-NativeArtifact-${artifact.id}`,
      name: artifact.id,
      versionInfo: artifact.componentVersion,
      downloadLocation: 'NOASSERTION',
      licenseConcluded: artifact.license || 'Proprietary / Embedded',
      checksums: [
        {
          algorithm: 'SHA256',
          checksumValue: artifact.sha256,
        },
      ],
    });
  }

  const sbom = {
    spdxVersion: 'SPDX-2.3',
    dataLicense: 'CC0-1.0',
    SPDXID: 'SPDXRef-DOCUMENT',
    name: `${pkg.name}-${pkg.version}-SBOM`,
    documentNamespace: `https://vane.app/spdx/${pkg.name}/${pkg.version}`,
    creationInfo: {
      creators: ['Tool: Vane SBOM Generator 1.0'],
      created: new Date().toISOString(),
    },
    packages,
  };

  const outDir = path.join(repoRoot, 'artifacts');
  if (!fs.existsSync(outDir)) {
    fs.mkdirSync(outDir, { recursive: true });
  }

  const outFile = path.join(outDir, `sbom-${pkg.version}.spdx.json`);
  fs.writeFileSync(outFile, JSON.stringify(sbom, null, 2), 'utf8');
  console.log(`✅ Generated SBOM at ${outFile}`);
}

generateSbom();
