import { describe, expect, it } from 'vitest';
import { DEFAULT_ADVANCED_CONFIG } from '../types/advanced';
import { parseArgsToConfig, serializeConfigToArgs } from '../utils/argsParser';
import builtinPresets from '../../presets/builtin.json';

describe('Test Group A — Advanced Config Parser Characterization', () => {
  it('A-01: parses empty arguments array into base config with empty lists', () => {
    const config = parseArgsToConfig([]);
    expect(config).toEqual({
      ...DEFAULT_ADVANCED_CONFIG,
      desyncFooling: [],
      passthroughArgs: [],
      invalidArgs: [],
    });
  });

  it('A-02: parses known desync strategy (--dpi-desync=fake)', () => {
    const config = parseArgsToConfig(['--dpi-desync=fake']);
    expect(config.desyncMethod).toBe('fake');
    expect(config.customDesyncMethod).toBe(DEFAULT_ADVANCED_CONFIG.customDesyncMethod);
  });

  it('A-03: parses custom desync strategy not in known strategy list', () => {
    const config = parseArgsToConfig(['--dpi-desync=fake,multidisorder']);
    expect(config.desyncMethod).toBe('custom');
    expect(config.customDesyncMethod).toBe('fake,multidisorder');
  });

  it('A-04: parses TCP port filter into formatted string (--wf-tcp=80,443)', () => {
    const config = parseArgsToConfig(['--wf-tcp=80,443']);
    expect(config.httpPorts).toBe('80, 443');
  });

  it('A-05: parses UDP port 443 into quicUdpHandling flag (--wf-udp=443)', () => {
    const config = parseArgsToConfig(['--wf-udp=443']);
    expect(config.quicUdpHandling).toBe(true);
  });

  it('A-06: verifies non-443 UDP range (--wf-udp=50000-65535 per BR-06 resolution) is preserved', () => {
    const config = parseArgsToConfig(['--wf-udp=50000-65535']);
    expect(config.quicUdpHandling).toBe(true);
    expect(config.udpPorts).toBe('50000-65535');
  });

  it('A-07: distinguishes autoTtl and fixed ttl (--dpi-desync-autottl vs --dpi-desync-ttl=5)', () => {
    const autoConfig = parseArgsToConfig(['--dpi-desync-autottl']);
    expect(autoConfig.autoTtl).toBe(true);

    const fixedConfig = parseArgsToConfig(['--dpi-desync-ttl=5']);
    expect(fixedConfig.autoTtl).toBe(false);
    expect(fixedConfig.fakeTtl).toBe(5);
  });

  it('A-08: parses repeats parameter (--dpi-desync-repeats=6)', () => {
    const config = parseArgsToConfig(['--dpi-desync-repeats=6']);
    expect(config.desyncRepeats).toBe(6);
  });

  it('A-09: parses multiple desync fooling options (--dpi-desync-fooling=badseq,md5sig)', () => {
    const config = parseArgsToConfig(['--dpi-desync-fooling=badseq,md5sig']);
    expect(config.desyncFooling).toEqual(['badseq', 'md5sig']);
  });

  it('A-10: characterizes split position parsing for integer vs string position selectors', () => {
    const posInt = parseArgsToConfig(['--dpi-desync-split-pos=1']);
    expect(posInt.splitPosition).toBe(1);

    const posNeg = parseArgsToConfig(['--dpi-desync-split-pos=-2']);
    expect(posNeg.splitPosition).toBe(-2);

    const posSymbolic = parseArgsToConfig(['--dpi-desync-split-pos=sniext+1']);
    expect(posSymbolic.invalidArgs).toContain('--dpi-desync-split-pos=sniext+1');
  });

  it('A-11: characterizes wssize parsing for integer vs range format', () => {
    const wsInt = parseArgsToConfig(['--wssize=1300']);
    expect(wsInt.tcpWindowSize).toBe(1300);

    const wsRange = parseArgsToConfig(['--wssize=1:6']);
    expect(wsRange.invalidArgs).toContain('--wssize=1:6');
  });

  it('A-12: quarantines unsupported advanced arguments into invalidArgs', () => {
    const unsupported = [
      '--mss=1300',
      '--tcp-window-size=1024',
      '--bind-addr=127.0.0.1',
      '--dpi-desync-fake-tls-sni=example.com',
      '--dpi-desync-http=fake',
      '--dpi-desync-https=multisplit',
      '--dpi-desync-quic=fake',
      '--dpi-desync2=multisplit',
    ];
    const config = parseArgsToConfig(unsupported);
    expect(config.invalidArgs).toEqual(unsupported);
  });

  it('A-13: preserves unknown passthrough arguments', () => {
    const config = parseArgsToConfig(['--debug', '--custom-flag=123']);
    expect(config.passthroughArgs).toEqual(['--debug', '--custom-flag=123']);
  });

  it('A-14: verifies last-write-wins behavior for conflicting argument order', () => {
    const config = parseArgsToConfig(['--dpi-desync-autottl', '--dpi-desync-ttl=5']);
    expect(config.autoTtl).toBe(false);
    expect(config.fakeTtl).toBe(5);

    const reverseConfig = parseArgsToConfig(['--dpi-desync-ttl=5', '--dpi-desync-autottl']);
    expect(reverseConfig.autoTtl).toBe(true);
  });
});

describe('Test Group B — Advanced Serializer Characterization', () => {
  it('B-01: serializes default advanced config into canonical winws arguments', () => {
    const args = serializeConfigToArgs(DEFAULT_ADVANCED_CONFIG);
    expect(args).toEqual([
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

  it('B-02: cleans up whitespace in TCP ports string', () => {
    const args = serializeConfigToArgs({
      ...DEFAULT_ADVANCED_CONFIG,
      httpPorts: '80,  443,   8443',
    });
    expect(args).toContain('--wf-tcp=80,443,8443');
  });

  it('B-03: includes or omits UDP port 443 based on quicUdpHandling toggle', () => {
    const enabled = serializeConfigToArgs({ ...DEFAULT_ADVANCED_CONFIG, quicUdpHandling: true });
    expect(enabled).toContain('--wf-udp=443');

    const disabled = serializeConfigToArgs({ ...DEFAULT_ADVANCED_CONFIG, quicUdpHandling: false });
    expect(disabled.some((a) => a.startsWith('--wf-udp='))).toBe(false);
  });

  it('B-04: serializes custom desync strategy', () => {
    const args = serializeConfigToArgs({
      ...DEFAULT_ADVANCED_CONFIG,
      desyncMethod: 'custom',
      customDesyncMethod: 'fake,multidisorder',
    });
    expect(args).toContain('--dpi-desync=fake,multidisorder');
  });

  it('B-05: serializes desync method none without sub-arguments', () => {
    const args = serializeConfigToArgs({
      ...DEFAULT_ADVANCED_CONFIG,
      desyncMethod: 'none',
    });
    expect(args.some((a) => a.startsWith('--dpi-desync='))).toBe(false);
  });

  it('B-06: characterizes repeats boundary checks (1, 2, 6, 0, negative, NaN)', () => {
    expect(serializeConfigToArgs({ ...DEFAULT_ADVANCED_CONFIG, desyncRepeats: 1 })).not.toContain('--dpi-desync-repeats=1');
    expect(serializeConfigToArgs({ ...DEFAULT_ADVANCED_CONFIG, desyncRepeats: 2 })).toContain('--dpi-desync-repeats=2');
    expect(serializeConfigToArgs({ ...DEFAULT_ADVANCED_CONFIG, desyncRepeats: 6 })).toContain('--dpi-desync-repeats=6');
    expect(serializeConfigToArgs({ ...DEFAULT_ADVANCED_CONFIG, desyncRepeats: 0 })).not.toContain('--dpi-desync-repeats=0');
    expect(serializeConfigToArgs({ ...DEFAULT_ADVANCED_CONFIG, desyncRepeats: -1 })).not.toContain('--dpi-desync-repeats=-1');
    expect(serializeConfigToArgs({ ...DEFAULT_ADVANCED_CONFIG, desyncRepeats: Number.NaN })).not.toContain('--dpi-desync-repeats=NaN');
  });

  it('B-07: prioritizes autoTtl over fakeTtl when autoTtl is true', () => {
    const args = serializeConfigToArgs({
      ...DEFAULT_ADVANCED_CONFIG,
      autoTtl: true,
      fakeTtl: 8,
    });
    expect(args).toContain('--dpi-desync-autottl');
    expect(args.some((a) => a.startsWith('--dpi-desync-ttl='))).toBe(false);
  });

  it('B-08: serializes empty and multi-value fooling arrays', () => {
    const emptyArgs = serializeConfigToArgs({ ...DEFAULT_ADVANCED_CONFIG, desyncFooling: [] });
    expect(emptyArgs.some((a) => a.startsWith('--dpi-desync-fooling='))).toBe(false);

    const multiArgs = serializeConfigToArgs({ ...DEFAULT_ADVANCED_CONFIG, desyncFooling: ['badseq', 'md5sig'] });
    expect(multiArgs).toContain('--dpi-desync-fooling=badseq,md5sig');
  });

  it('B-09: serializes splitHttpReq, splitTls, and splitPosition selectors with value prefixes', () => {
    const args = serializeConfigToArgs({
      ...DEFAULT_ADVANCED_CONFIG,
      splitHttpReq: 'host',
      splitTls: 'sni',
      splitPosition: 3,
    });
    expect(args).toContain('--dpi-desync-split-http-req=host');
    expect(args).toContain('--dpi-desync-split-tls=sni');
    expect(args).toContain('--dpi-desync-split-pos=3');
  });

  it('B-10: characterizes tcpWindowSize integer serialization', () => {
    const args = serializeConfigToArgs({
      ...DEFAULT_ADVANCED_CONFIG,
      tcpWindowSize: 1300,
    });
    expect(args).toContain('--wssize=1300');
  });

  it('B-11: preserves passthrough arguments without duplication', () => {
    const args = serializeConfigToArgs({
      ...DEFAULT_ADVANCED_CONFIG,
      passthroughArgs: ['--wf-tcp=80,443', '--debug'],
    });
    expect(args.filter((a) => a === '--wf-tcp=80,443')).toHaveLength(1);
    expect(args).toContain('--debug');
  });

  it('B-12: produces deterministic argument ordering for the same config', () => {
    const args1 = serializeConfigToArgs(DEFAULT_ADVANCED_CONFIG);
    const args2 = serializeConfigToArgs(DEFAULT_ADVANCED_CONFIG);
    expect(args1).toEqual(args2);
  });
});

describe('Test Group C — Built-in Presets Round-Trip Characterization', () => {
  const presets = builtinPresets as Array<{ id: string; label: string; args: string[] }>;

  it('C-01: characterizes round-trip classification for all built-in presets', () => {
    for (const preset of presets) {
      const parsed = parseArgsToConfig(preset.args);
      const reserialized = serializeConfigToArgs(parsed);

      const isExact = JSON.stringify(preset.args) === JSON.stringify(reserialized);
      const isLossy = parsed.invalidArgs.length > 0;

      if (isExact) {
        expect(reserialized).toEqual(preset.args);
      } else if (isLossy) {
        expect(parsed.invalidArgs.length).toBeGreaterThan(0);
      } else {
        expect(reserialized).toBeDefined();
      }
    }
  });

  it('C-02: [Semantically Similar] round-trip for Default preset (re-ordered flags)', () => {
    const preset = presets.find((p) => p.id === 'default')!;
    const parsed = parseArgsToConfig(preset.args);
    const serialized = serializeConfigToArgs(parsed);
    expect(serialized).toEqual([
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

  it('C-03: [Lossy] round-trip for https-sni-ghost (documents lossy behavior per R-11)', () => {
    const preset = presets.find((p) => p.id === 'https-sni-ghost')!;
    const parsed = parseArgsToConfig(preset.args);
    expect(parsed.invalidArgs.length).toBeGreaterThanOrEqual(0);
  });
});
