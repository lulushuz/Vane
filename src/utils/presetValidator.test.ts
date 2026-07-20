import { describe, expect, it } from 'vitest';
import { validateImportedPreset } from './presetValidator';

describe('imported preset validation', () => {
  it('accepts a bounded structural preset and sanitizes display metadata', () => {
    const result = validateImportedPreset({
      label: 'x'.repeat(80),
      description: 'd'.repeat(300),
      icon: '123456789',
      args: ['--wf-tcp=80,443', '--dpi-desync=fake'],
      priority: 500,
    });

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.preset.label).toHaveLength(64);
    expect(result.preset.description).toHaveLength(256);
    expect(result.preset.icon).toHaveLength(8);
    expect(result.preset.priority).toBe(100);
    expect(result.preset.isCustom).toBe(true);
  });

  it.each(['--wf-tcp=443;calc', '--arg|next', '--arg`next', '--arg$next', '../file\\name'])(
    'rejects shell metacharacters in %s',
    (arg) => {
      expect(validateImportedPreset({ args: [arg] }).ok).toBe(false);
    },
  );

  it('rejects oversized, empty, non-string, and excessive argument lists', () => {
    expect(validateImportedPreset({ args: [''] }).ok).toBe(false);
    expect(validateImportedPreset({ args: [7] }).ok).toBe(false);
    expect(validateImportedPreset({ args: ['x'.repeat(129)] }).ok).toBe(false);
    expect(validateImportedPreset({ args: Array.from({ length: 31 }, () => '--foo') }).ok)
      .toBe(false);
  });
});
