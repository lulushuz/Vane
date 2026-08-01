import type {
  AdvancedCapabilities,
  AdvancedConfig,
  AdvancedConfigCandidate,
  AdvancedParseDiagnostic,
  AdvancedParseResult,
  ValidatedPassthroughArg,
} from '../types/advanced';
import { DEFAULT_ADVANCED_CONFIG } from '../types/advanced';

const KNOWN_STRATEGIES = new Set([
  'none',
  'syndata',
  'rst',
  'rstack',
  'fake',
  'fakeknown',
  'split',
  'split2',
  'multisplit',
  'disorder',
  'multidisorder',
  'hostfake',
  'fakedsplit',
  'destopt',
  'ipfrag1',
  'ipfrag2',
  'udplen',
  'tamper',
]);

const FORBIDDEN_UNSUPPORTED_FLAGS = new Set([
  '--mss',
  '--dpi-desync-fake-tls-sni',
  '--bind-addr',
  '--ipset',
  '--socks',
  '--dpi-desync-fake-http',
  '--dpi-desync-fake-tls',
  '--dpi-desync-fake-quic',
  '--dpi-desync-http',
  '--dpi-desync-https',
  '--dpi-desync-quic',
  '--dpi-desync2',
  '--dpi-desync-ttl-ext',
  '--dpi-desync-split-pos-http-req',
  '--dpi-desync-split-pos-tls',
  '--tcp-window-size',
]);

const valueAfterEquals = (arg: string): string => {
  const separator = arg.indexOf('=');
  return separator >= 0 ? arg.slice(separator + 1) : '';
};

const parseIntegerStrict = (val: string): number | null => {
  const trimmed = val.trim();
  if (!/^-?\d+$/.test(trimmed)) return null;
  const num = Number(trimmed);
  return Number.isSafeInteger(num) ? num : null;
};

export function parseAdvancedArguments(
  args: readonly string[],
  _capabilities?: AdvancedCapabilities,
): AdvancedParseResult {
  const diagnostics: AdvancedParseDiagnostic[] = [];
  const passthrough: ValidatedPassthroughArg[] = [];

  let methods: string[] = [];
  let tcpPorts = '';
  let udpPorts = '';
  let ttlMode: 'default' | 'auto' | 'fixed' = 'default';
  let ttlValue: number | undefined;
  let repeats: number | undefined;
  let fooling: string[] = [];
  let splitPosition: string | undefined;
  let windowSize: string | undefined;

  for (const arg of args) {
    const trimmed = arg.trim();
    if (!trimmed) continue;

    const prefix = trimmed.split('=')[0];
    if (FORBIDDEN_UNSUPPORTED_FLAGS.has(prefix)) {
      diagnostics.push({
        argument: trimmed,
        code: 'UNSUPPORTED_ARGUMENT',
        message: `Argument ${prefix} is not supported by the bundled DPI engine and was removed`,
        severity: 'warning',
      });
      continue;
    }

    if (trimmed.startsWith('--wf-tcp=')) {
      tcpPorts = valueAfterEquals(trimmed);
    } else if (trimmed.startsWith('--wf-udp=')) {
      udpPorts = valueAfterEquals(trimmed);
    } else if (trimmed.startsWith('--wssize=')) {
      windowSize = valueAfterEquals(trimmed);
    } else if (trimmed.startsWith('--dpi-desync=')) {
      const rawVal = valueAfterEquals(trimmed);
      methods = rawVal
        .split(',')
        .map((s) => s.trim())
        .filter(Boolean);
    } else if (trimmed === '--dpi-desync-autottl') {
      ttlMode = 'auto';
    } else if (trimmed.startsWith('--dpi-desync-ttl=')) {
      const parsed = parseIntegerStrict(valueAfterEquals(trimmed));
      if (parsed === null) {
        diagnostics.push({
          argument: trimmed,
          code: 'MALFORMED_INTEGER',
          message: `Malformed TTL integer: ${trimmed}`,
          severity: 'error',
        });
      } else {
        ttlMode = 'fixed';
        ttlValue = parsed;
      }
    } else if (trimmed.startsWith('--dpi-desync-repeats=')) {
      const parsed = parseIntegerStrict(valueAfterEquals(trimmed));
      if (parsed === null) {
        diagnostics.push({
          argument: trimmed,
          code: 'MALFORMED_INTEGER',
          message: `Malformed repeats integer: ${trimmed}`,
          severity: 'error',
        });
      } else {
        repeats = parsed;
      }
    } else if (trimmed.startsWith('--dpi-desync-fooling=')) {
      fooling = valueAfterEquals(trimmed)
        .split(',')
        .map((s) => s.trim())
        .filter(Boolean);
    } else if (trimmed.startsWith('--dpi-desync-split-pos=')) {
      const val = valueAfterEquals(trimmed);
      if (!/^-?\d+$/.test(val) && !val.startsWith('sniext')) {
        diagnostics.push({
          argument: trimmed,
          code: 'MALFORMED_INTEGER',
          message: `Malformed split position: ${trimmed}`,
          severity: 'error',
        });
      } else {
        splitPosition = val;
      }
    } else {
      passthrough.push(trimmed as ValidatedPassthroughArg);
    }
  }

  const candidate: AdvancedConfigCandidate = {
    methods,
    tcpPorts,
    udpPorts,
    ttlMode,
    ttlValue,
    repeats,
    fooling,
    splitPosition,
    windowSize,
    passthrough: passthrough.map((p) => String(p)),
  };

  const lossless = diagnostics.length === 0;

  return {
    candidate,
    passthrough,
    diagnostics,
    lossless,
  };
}

// Backward compatibility legacy interface for UI components
export function parseArgsToConfig(args: string[]): AdvancedConfig {
  const config: AdvancedConfig = {
    ...DEFAULT_ADVANCED_CONFIG,
    desyncFooling: [],
    passthroughArgs: [],
    invalidArgs: [],
  };

  for (const arg of args) {
    if (arg.startsWith('--dpi-desync=')) {
      const val = valueAfterEquals(arg);
      if (KNOWN_STRATEGIES.has(val)) {
        config.desyncMethod = val;
      } else {
        config.desyncMethod = 'custom';
        config.customDesyncMethod = val;
      }
    } else if (
      arg.startsWith('--dpi-desync-http=') ||
      arg.startsWith('--dpi-desync-https=') ||
      arg.startsWith('--dpi-desync-quic=') ||
      arg.startsWith('--dpi-desync2=') ||
      arg.startsWith('--dpi-desync-ttl-ext=') ||
      arg.startsWith('--dpi-desync-split-pos-http-req=') ||
      arg.startsWith('--dpi-desync-split-pos-tls=') ||
      arg.startsWith('--dpi-desync-fake-tls-sni=') ||
      arg.startsWith('--mss=') ||
      arg.startsWith('--tcp-window-size=') ||
      arg.startsWith('--bind-addr=') ||
      arg.startsWith('--ipset=') ||
      arg.startsWith('--socks=')
    ) {
      config.invalidArgs.push(arg);
    } else if (arg.startsWith('--dpi-desync-cutoff=')) {
      config.desyncCutoff = valueAfterEquals(arg);
    } else if (arg.startsWith('--dpi-desync-split-pos=')) {
      const parsed = parseIntegerStrict(valueAfterEquals(arg));
      if (parsed === null) {
        config.invalidArgs.push(arg);
      } else {
        config.splitPosition = parsed;
      }
    } else if (arg.startsWith('--dpi-desync-repeats=')) {
      const parsed = parseIntegerStrict(valueAfterEquals(arg));
      if (parsed === null) {
        config.invalidArgs.push(arg);
      } else {
        config.desyncRepeats = parsed;
      }
    } else if (arg.startsWith('--dpi-desync-fooling=')) {
      config.desyncFooling = valueAfterEquals(arg)
        .split(',')
        .map((s) => s.trim())
        .filter(Boolean);
    } else if (arg.startsWith('--dpi-desync-ttl=')) {
      const parsed = parseIntegerStrict(valueAfterEquals(arg));
      if (parsed === null) {
        config.invalidArgs.push(arg);
      } else {
        config.fakeTtl = parsed;
        config.autoTtl = false;
      }
    } else if (arg === '--dpi-desync-autottl') {
      config.autoTtl = true;
    } else if (arg.startsWith('--dpi-desync-split-http-req=')) {
      config.splitHttpReq = valueAfterEquals(arg);
    } else if (arg.startsWith('--dpi-desync-split-tls=')) {
      config.splitTls = valueAfterEquals(arg);
    } else if (arg.startsWith('--dpi-desync-fake-http=')) {
      config.fakeHttpPayload = valueAfterEquals(arg);
    } else if (arg.startsWith('--dpi-desync-fake-tls=')) {
      config.fakeTlsPayload = valueAfterEquals(arg);
    } else if (arg.startsWith('--dpi-desync-fake-quic=')) {
      config.fakeQuicPayload = valueAfterEquals(arg);
    } else if (arg.startsWith('--wssize=')) {
      const parsed = parseIntegerStrict(valueAfterEquals(arg));
      if (parsed === null) {
        config.invalidArgs.push(arg);
      } else {
        config.tcpWindowSize = parsed;
      }
    } else if (arg.startsWith('--wf-tcp=')) {
      config.httpPorts = valueAfterEquals(arg).replace(/,/g, ', ');
    } else if (arg.startsWith('--wf-udp=')) {
      config.quicUdpHandling = true;
      config.udpPorts = valueAfterEquals(arg);
    } else if (arg === '--dpi-desync-any-protocol') {
      config.anyProtocol = true;
    } else {
      config.passthroughArgs.push(arg);
    }
  }

  return config;
}

export function serializeConfigToArgs(config?: Partial<AdvancedConfig>): string[] {
  if (!config) return [];
  const args: string[] = [];

  if (config.httpPorts) {
    args.push(`--wf-tcp=${config.httpPorts.replace(/\s+/g, '')}`);
  }
  if (config.udpPorts) {
    args.push(`--wf-udp=${config.udpPorts.replace(/\s+/g, '')}`);
  } else if (config.quicUdpHandling) {
    args.push('--wf-udp=443');
  }

  if (
    config.tcpWindowSize &&
    Number.isFinite(config.tcpWindowSize) &&
    Number.isInteger(config.tcpWindowSize) &&
    config.tcpWindowSize > 0
  ) {
    args.push(`--wssize=${config.tcpWindowSize}`);
  }

  const strategyVal =
    config.desyncMethod === 'custom'
      ? config.customDesyncMethod || 'multisplit'
      : config.desyncMethod;

  if (strategyVal && strategyVal !== 'none') {
    args.push(`--dpi-desync=${strategyVal}`);
  }

  if (config.anyProtocol) {
    args.push('--dpi-desync-any-protocol');
  }

  if (config.desyncCutoff) {
    args.push(`--dpi-desync-cutoff=${config.desyncCutoff}`);
  }

  if (
    config.splitPosition &&
    Number.isFinite(config.splitPosition) &&
    Number.isInteger(config.splitPosition) &&
    config.splitPosition > 0
  ) {
    args.push(`--dpi-desync-split-pos=${config.splitPosition}`);
  }

  if (config.splitHttpReq) {
    args.push(`--dpi-desync-split-http-req=${config.splitHttpReq}`);
  }
  if (config.splitTls) {
    args.push(`--dpi-desync-split-tls=${config.splitTls}`);
  }

  if (config.fakeHttpPayload) {
    args.push(`--dpi-desync-fake-http=${config.fakeHttpPayload}`);
  }
  if (config.fakeTlsPayload) {
    args.push(`--dpi-desync-fake-tls=${config.fakeTlsPayload}`);
  }
  if (config.fakeQuicPayload) {
    args.push(`--dpi-desync-fake-quic=${config.fakeQuicPayload}`);
  }

  if (
    config.desyncRepeats &&
    Number.isFinite(config.desyncRepeats) &&
    Number.isInteger(config.desyncRepeats) &&
    config.desyncRepeats > 1
  ) {
    args.push(`--dpi-desync-repeats=${config.desyncRepeats}`);
  }

  if (Array.isArray(config.desyncFooling) && config.desyncFooling.length > 0) {
    args.push(`--dpi-desync-fooling=${config.desyncFooling.join(',')}`);
  }

  if (config.autoTtl) {
    args.push('--dpi-desync-autottl');
  } else if (
    config.fakeTtl &&
    Number.isFinite(config.fakeTtl) &&
    Number.isInteger(config.fakeTtl) &&
    config.fakeTtl > 0
  ) {
    args.push(`--dpi-desync-ttl=${config.fakeTtl}`);
  }

  if (Array.isArray(config.passthroughArgs)) {
    const emittedExact = new Set(args);
    const emittedPrefixes = new Set(args.map((a) => a.split('=')[0]));

    for (const passthroughArg of config.passthroughArgs) {
      const prefix = passthroughArg.split('=')[0];
      if (!emittedExact.has(passthroughArg) && !emittedPrefixes.has(prefix)) {
        args.push(passthroughArg);
      }
    }
  }

  return args;
}
