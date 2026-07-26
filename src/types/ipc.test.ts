import { describe, expect, it } from 'vitest';
import { normalizeIpcError } from './ipc';

describe('normalizeIpcError', () => {
  it('preserves structured Rust engine errors', () => {
    expect(normalizeIpcError({
      code: 'DNS_FORWARDER_RESTART_FAILED',
      message: 'Forwarder unavailable',
      operation: 'sync_dns_settings',
      retryable: true,
    })).toEqual({
      code: 'DNS_FORWARDER_RESTART_FAILED',
      message: 'Forwarder unavailable',
      operation: 'sync_dns_settings',
      retryable: true,
      configRevision: undefined,
    });
  });

  it('normalizes unstructured command errors', () => {
    expect(normalizeIpcError('Forwarder unavailable')).toEqual({
      code: 'UNKNOWN',
      message: 'Forwarder unavailable',
    });
  });
});
