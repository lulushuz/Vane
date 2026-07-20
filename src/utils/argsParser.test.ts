import { describe, expect, it } from 'vitest';
import { DEFAULT_ADVANCED_CONFIG } from '../types/advanced';
import { parseArgsToConfig, serializeConfigToArgs } from './argsParser';

describe('Advanced argument serialization', () => {
  it('serializes the supported default controls into canonical winws arguments', () => {
    expect(serializeConfigToArgs(DEFAULT_ADVANCED_CONFIG)).toEqual([
      '--wf-tcp=80,443',
      '--wf-udp=443',
      '--dpi-desync=fake,multidisorder',
      '--dpi-desync-any-protocol',
      '--dpi-desync-cutoff=d3',
      '--dpi-desync-split-pos=1',
      '--dpi-desync-fooling=badseq',
      '--dpi-desync-autottl',
    ]);
  });

  it('uses a validated manual TTL and omits invalid numeric values', () => {
    const args = serializeConfigToArgs({
      ...DEFAULT_ADVANCED_CONFIG,
      autoTtl: false,
      fakeTtl: 7,
      splitPosition: Number.NaN,
      desyncRepeats: 1,
      tcpWindowSize: -1,
    });

    expect(args).toContain('--dpi-desync-ttl=7');
    expect(args.some((arg) => arg.includes('NaN'))).toBe(false);
    expect(args.some((arg) => arg.startsWith('--wssize='))).toBe(false);
    expect(args.some((arg) => arg.startsWith('--dpi-desync-repeats='))).toBe(false);
  });

  it('deduplicates passthrough arguments already emitted by modeled controls', () => {
    const args = serializeConfigToArgs({
      ...DEFAULT_ADVANCED_CONFIG,
      passthroughArgs: ['--wf-udp=443', '--debug'],
    });

    expect(args.filter((arg) => arg === '--wf-udp=443')).toHaveLength(1);
    expect(args).toContain('--debug');
  });
});

describe('Advanced argument parsing', () => {
  it('parses supported text, list, boolean, and numeric inputs', () => {
    const parsed = parseArgsToConfig([
      '--wf-tcp=80,443,8443',
      '--wf-udp=443',
      '--dpi-desync=multisplit',
      '--dpi-desync-cutoff=d5',
      '--dpi-desync-split-pos=3',
      '--dpi-desync-repeats=4',
      '--dpi-desync-fooling=badseq,md5sig',
      '--dpi-desync-ttl=6',
    ]);

    expect(parsed).toMatchObject({
      httpPorts: '80, 443, 8443',
      quicUdpHandling: true,
      desyncMethod: 'multisplit',
      desyncCutoff: 'd5',
      splitPosition: 3,
      desyncRepeats: 4,
      desyncFooling: ['badseq', 'md5sig'],
      autoTtl: false,
      fakeTtl: 6,
    });
    expect(parsed.invalidArgs).toEqual([]);
  });

  it('quarantines unsupported privileged fields instead of forwarding them', () => {
    const parsed = parseArgsToConfig([
      '--bind-addr=127.0.0.1',
      '--ipset=C:/outside.txt',
      '--socks=127.0.0.1:1080',
      '--dpi-desync-http=fake',
    ]);

    expect(parsed.invalidArgs).toEqual([
      '--bind-addr=127.0.0.1',
      '--ipset=C:/outside.txt',
      '--socks=127.0.0.1:1080',
      '--dpi-desync-http=fake',
    ]);
    expect(serializeConfigToArgs(parsed)).not.toEqual(
      expect.arrayContaining(parsed.invalidArgs),
    );
  });

  it('does not convert malformed integers into runtime values', () => {
    const parsed = parseArgsToConfig([
      '--dpi-desync-split-pos=2.5',
      '--dpi-desync-repeats=Infinity',
      '--dpi-desync-ttl=not-a-number',
    ]);

    expect(parsed.invalidArgs).toEqual([
      '--dpi-desync-split-pos=2.5',
      '--dpi-desync-repeats=Infinity',
      '--dpi-desync-ttl=not-a-number',
    ]);
    expect(Number.isFinite(parsed.splitPosition)).toBe(true);
    expect(Number.isFinite(parsed.desyncRepeats)).toBe(true);
    expect(Number.isFinite(parsed.fakeTtl)).toBe(true);
  });
});
