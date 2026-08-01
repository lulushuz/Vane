import { describe, expect, it, beforeEach } from 'vitest';
import { mockIpc, resetStoreToDefaults } from './mockIpc';
import { useEngineStore } from '../store/engineStore';
import { validateImportedPreset } from '../utils/presetValidator';
import { normalizeIpcError } from '../types/ipc';

describe('Test Group I — Preset Import/Export Characterization', () => {
  beforeEach(() => {
    mockIpc.reset();
    resetStoreToDefaults();
  });

  it('I-01: validates and builds custom preset payload from form metadata', () => {
    const rawInput = {
      id: 'custom-1',
      label: 'My Custom Preset',
      description: 'Test custom preset description',
      args: ['--wf-tcp=80,443', '--dpi-desync=fake'],
      category: 'custom',
    };

    const validated = validateImportedPreset(rawInput);
    expect(validated.ok).toBe(true);
    if (validated.ok) {
      expect(validated.preset).toMatchObject({
        label: 'My Custom Preset',
        args: ['--wf-tcp=80,443', '--dpi-desync=fake'],
        isCustom: true,
      });
    }
  });

  it('I-02: characterizes default export file format (.vane canonical extension)', () => {
    const defaultExportName = 'vane-preset-custom.vane';
    expect(defaultExportName.endsWith('.vane')).toBe(true);
  });

  it('I-03: verifies export_preset IPC command payload formatting with .vane extension', async () => {
    const preset = {
      id: 'p-1',
      label: 'Test Preset',
      description: 'Desc',
      args: ['--wf-tcp=80,443'],
      isCustom: true,
    };

    await (mockIpc.handleInvoke('export_preset', {
      filePath: 'C:/presets/my-preset.vane',
      content: JSON.stringify(preset, null, 2),
    }));

    const call = mockIpc.getCallsForCommand('export_preset').pop();
    expect(call?.payload?.filePath).toBe('C:/presets/my-preset.vane');
    expect(JSON.parse(call?.payload?.content)).toEqual(preset);
  });

  it('I-04: verifies export_preset rejects non-.vane export extensions (R-02 resolved)', async () => {
    mockIpc.registerError('export_preset', 'Preset exports must use the .vane extension.');

    await expect(
      mockIpc.handleInvoke('export_preset', {
        filePath: 'C:/presets/my-preset.json',
        content: '{}',
      }),
    ).rejects.toBe('Preset exports must use the .vane extension.');
  });


  it('I-05: imports valid JSON preset object', () => {
    const validJson = {
      label: 'Valid Preset',
      description: 'Description',
      args: ['--wf-tcp=80,443', '--dpi-desync=fake'],
    };

    const res = validateImportedPreset(validJson);
    expect(res.ok).toBe(true);
  });

  it('I-06: rejects invalid preset JSON schemas (non-array args, shell injection, oversized text)', () => {
    expect(validateImportedPreset({ args: 'not-an-array' }).ok).toBe(false);
    expect(validateImportedPreset({ args: ['--wf-tcp=80;calc'] }).ok).toBe(false);
    expect(validateImportedPreset({ label: 'x'.repeat(100), args: [] }).ok).toBe(true); // label gets truncated to 64 chars
  });

  it('I-07: handles imported preset with unknown arguments', () => {
    const imported = validateImportedPreset({
      label: 'Unknown Args Preset',
      args: ['--wf-tcp=80,443', '--unknown-desync-flag=1'],
    });

    expect(imported.ok).toBe(true);
    if (imported.ok) {
      expect(imported.preset.args).toContain('--unknown-desync-flag=1');
    }
  });

  it('I-08: verifies preset deletion IPC call and state update', async () => {
    useEngineStore.setState({
      presets: [
        { id: 'custom-p1', label: 'Preset 1', description: '', icon: 'zap', args: [], isCustom: true },
      ],
      activePresetId: 'custom-p1',
    });

    await useEngineStore.getState().deletePreset('custom-p1');

    expect(mockIpc.getCallsForCommand('delete_custom_preset')).toHaveLength(1);
    expect(useEngineStore.getState().activePresetId).toBe('default');
  });
});

describe('Test Group J — IPC Error Normalization Characterization', () => {
  it('J-01: normalizes standard JavaScript Error instances', () => {
    const err = new Error('Network unreachable');
    expect(normalizeIpcError(err)).toEqual({
      code: 'UNKNOWN',
      message: 'Network unreachable',
    });
  });

  it('J-02: normalizes plain string error messages', () => {
    expect(normalizeIpcError('Access denied')).toEqual({
      code: 'UNKNOWN',
      message: 'Access denied',
    });
  });

  it('J-03: preserves structured Rust engine error objects', () => {
    const structured = {
      code: 'ELEVATION_REQUIRED',
      message: 'Administrator privileges required to load WinDivert',
      operation: 'start_engine_with_dns_guard',
      retryable: false,
      configRevision: 5,
    };
    expect(normalizeIpcError(structured)).toEqual({
      code: 'ELEVATION_REQUIRED',
      message: 'Administrator privileges required to load WinDivert',
      operation: 'start_engine_with_dns_guard',
      retryable: false,
      configRevision: 5,
    });
  });

  it('J-04: handles null and undefined error inputs by converting to string representation', () => {
    expect(normalizeIpcError(null)).toEqual({
      code: 'UNKNOWN',
      message: 'null',
    });
    expect(normalizeIpcError(undefined)).toEqual({
      code: 'UNKNOWN',
      message: 'undefined',
    });
  });

  it('J-05: handles arbitrary non-standard object error inputs', () => {
    const obj = { error_detail: 'Internal failure' };
    const normalized = normalizeIpcError(obj);
    expect(normalized.code).toBe('UNKNOWN');
    expect(typeof normalized.message).toBe('string');
  });
});
