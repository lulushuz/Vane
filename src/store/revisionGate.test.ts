import { describe, expect, it } from 'vitest';
import { MonotonicRevisionGate } from './revisionGate';

describe('IPC configuration revision gate', () => {
  it('accepts only newer backend snapshots', () => {
    const gate = new MonotonicRevisionGate();

    expect(gate.accept(2)).toBe(true);
    expect(gate.accept(1)).toBe(false);
    expect(gate.accept(2)).toBe(false);
    expect(gate.accept(3)).toBe(true);
  });

  it('rejects invalid revisions', () => {
    const gate = new MonotonicRevisionGate();

    expect(gate.accept(Number.NaN)).toBe(false);
    expect(gate.accept(1.5)).toBe(false);
    expect(gate.accept(Number.MAX_SAFE_INTEGER + 1)).toBe(false);
  });
});
