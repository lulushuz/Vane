import { describe, expect, it } from 'vitest';
import { normalizeIpcError } from './ipc';

describe('normalizeIpcError', () => {
  it('preserves structured Rust engine errors', () => {
    expect(normalizeIpcError({ code: 'SPAWN_FAILED', message: 'Process start error' })).toEqual({
      code: 'SPAWN_FAILED',
      message: 'Process start error',
    });
  });

  it('normalizes unstructured command errors', () => {
    expect(normalizeIpcError('Forwarder unavailable')).toEqual({
      code: 'UNKNOWN',
      message: 'Forwarder unavailable',
    });
  });
});
