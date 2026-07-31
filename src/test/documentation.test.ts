import { describe, expect, it } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';

describe('Test Group DOC — Documentation Capability Claims Alignment (P05-C)', () => {
  const readmePath = path.resolve(__dirname, '../../README.md');
  const readmeTrPath = path.resolve(__dirname, '../../README.tr.md');

  const readme = fs.readFileSync(readmePath, 'utf-8');
  const readmeTr = fs.readFileSync(readmeTrPath, 'utf-8');

  it('DOC-01: verifies README does not present DoQ as supported', () => {
    expect(readme).not.toContain('| DNS-over-QUIC | ✅ |');
    expect(readmeTr).not.toContain('| DNS-over-QUIC | ✅ |');
  });

  it('DOC-02: verifies README states DoQ as not supported / removed', () => {
    expect(readme).toContain('DNS-over-QUIC | ❌ Not supported');
    expect(readmeTr).toContain('DNS-over-QUIC | ❌ Desteklenmiyor');
  });

  it('DOC-03: verifies README does not falsely claim WFP callout driver usage', () => {
    expect(readme).not.toContain('Windows Filtering Platform (WFP) callout driver');
    expect(readmeTr).not.toContain('Windows Filtreleme Platformu (WFP)');
  });

  it('DOC-04: verifies README specifies canonical .vane extension for presets', () => {
    expect(readme).toContain('.vane');
    expect(readmeTr).toContain('.vane');
  });
});
