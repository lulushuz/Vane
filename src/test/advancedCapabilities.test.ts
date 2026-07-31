import { describe, expect, it } from 'vitest';
import { parseAdvancedArguments } from '../utils/argsParser';
import { formatPortRanges, parsePortRanges, validateAdvancedConfig } from '../utils/advancedValidator';
import { serializeVerifiedAdvancedConfig } from '../utils/advancedSerializer';
import type { AdvancedCapabilities, AdvancedConfigCandidate } from '../types/advanced';

const mockCapabilities: AdvancedCapabilities = {
  platform: 'windows',
  methods: {
    syndata: { state: 'supported' },
    fake: { state: 'supported' },
    split: { state: 'supported' },
    multisplit: { state: 'supported' },
  },
  traffic: {
    tcpFiltering: { state: 'supported' },
    udpFiltering: { state: 'supported' },
    customTcpPorts: { state: 'supported' },
    customUdpPorts: { state: 'supported' },
  },
  options: {
    autoTtl: { state: 'supported' },
    fixedTtl: { state: 'supported' },
    repeats: { state: 'supported' },
    fooling: { state: 'supported' },
    splitPosition: { state: 'supported' },
    windowSize: { state: 'supported' },
    mss: { state: 'unsupported', reason: 'Not supported' },
    fakePayload: { state: 'unsupported', reason: 'Not supported' },
    fakeTlsSni: { state: 'unsupported', reason: 'Not supported' },
    bindAddress: { state: 'unsupported', reason: 'Not supported' },
    ipset: { state: 'unsupported', reason: 'Not supported' },
    tpws: { state: 'unsupported', reason: 'Not supported' },
  },
};

describe('P09 Typed Advanced Configuration Contract & BR-06 Resolution', () => {
  it('BR-06 resolved: non-443 UDP port ranges survive Advanced parse and serialization', () => {
    const inputArgs = ['--wf-udp=50000-65535', '--dpi-desync=fake'];
    const parseResult = parseAdvancedArguments(inputArgs, mockCapabilities);

    expect(parseResult.candidate.udpPorts).toBe('50000-65535');
    const validated = validateAdvancedConfig(parseResult.candidate, mockCapabilities);
    expect(validated.valid).toBe(true);

    if (validated.valid) {
      const outputArgs = serializeVerifiedAdvancedConfig(validated.config);
      expect(outputArgs).toEqual(['--wf-udp=50000-65535', '--dpi-desync=fake']);
    }
  });

  it('parses and formats single and multi-range port specifications', () => {
    const parsed = parsePortRanges('80, 443, 50000-65535');
    expect(parsed).toEqual([
      { start: 80, end: 80 },
      { start: 443, end: 443 },
      { start: 50000, end: 65535 },
    ]);
    expect(formatPortRanges(parsed)).toBe('80,443,50000-65535');
  });

  it('rejects invalid port range specifications', () => {
    expect(parsePortRanges('0')).toEqual([]);
    expect(parsePortRanges('65536')).toEqual([]);
    expect(parsePortRanges('443-80')).toEqual([]);
    expect(parsePortRanges('80,,443')).toEqual([
      { start: 80, end: 80 },
      { start: 443, end: 443 },
    ]);
  });

  it('validates desync strategy phase ordering and rejects descending phases', () => {
    const candidate: AdvancedConfigCandidate = {
      methods: ['fake', 'syndata'], // Phase 1 then Phase 0 (invalid)
      tcpPorts: '80,443',
      udpPorts: '',
      ttlMode: 'default',
      fooling: [],
      passthrough: [],
    };
    const result = validateAdvancedConfig(candidate, mockCapabilities);
    expect(result.valid).toBe(false);
  });

  it('validates TTL discriminated union and rejects invalid fixed TTL value', () => {
    const candidate: AdvancedConfigCandidate = {
      methods: ['fake'],
      tcpPorts: '443',
      udpPorts: '',
      ttlMode: 'fixed',
      ttlValue: 300, // Invalid > 255
      fooling: [],
      passthrough: [],
    };
    const result = validateAdvancedConfig(candidate, mockCapabilities);
    expect(result.valid).toBe(false);
  });

  it('rejects dangling split position without split method', () => {
    const candidate: AdvancedConfigCandidate = {
      methods: ['fake'],
      tcpPorts: '443',
      udpPorts: '',
      ttlMode: 'default',
      fooling: [],
      splitPosition: '2',
      passthrough: [],
    };
    const result = validateAdvancedConfig(candidate, mockCapabilities);
    expect(result.valid).toBe(false);
  });

  it('filters out unsupported phantom fields during parse and serialization', () => {
    const inputArgs = ['--wf-tcp=443', '--dpi-desync=fake', '--mss=1400', '--ipset=blocklist'];
    const parseResult = parseAdvancedArguments(inputArgs, mockCapabilities);

    expect(parseResult.diagnostics).toHaveLength(2);
    expect(parseResult.diagnostics[0].code).toBe('UNSUPPORTED_ARGUMENT');

    const validated = validateAdvancedConfig(parseResult.candidate, mockCapabilities);
    expect(validated.valid).toBe(true);

    if (validated.valid) {
      const outputArgs = serializeVerifiedAdvancedConfig(validated.config);
      expect(outputArgs).toEqual(['--wf-tcp=443', '--dpi-desync=fake']);
    }
  });

  it('verifies round-trip for built-in TR-1 preset', () => {
    const tr1 = ['--wf-tcp=80,443', '--dpi-desync=fake,split', '--dpi-desync-repeats=2', '--dpi-desync-fooling=md5sig'];
    const parseResult = parseAdvancedArguments(tr1, mockCapabilities);
    const validated = validateAdvancedConfig(parseResult.candidate, mockCapabilities);
    expect(validated.valid).toBe(true);

    if (validated.valid) {
      const output = serializeVerifiedAdvancedConfig(validated.config);
      expect(output).toEqual(tr1);
    }
  });
});
