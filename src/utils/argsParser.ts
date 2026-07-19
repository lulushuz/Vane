import { type AdvancedConfig, DEFAULT_ADVANCED_CONFIG } from '../store/engineStore';

const KNOWN_STRATEGIES = [
  'none', 'synack', 'syndata', 'fake', 'fakeknown', 'rst', 'rstack',
  'hopbyhop', 'destopt', 'ipfrag1', 'multisplit', 'multidisorder',
  'fakedsplit', 'fakeddisorder', 'hostfakesplit', 'ipfrag2', 'udplen', 'tamper',
];

const valueAfterEquals = (arg: string): string => {
  const separator = arg.indexOf('=');
  return separator >= 0 ? arg.slice(separator + 1) : '';
};

const parseFiniteInteger = (value: string): number | null => {
  if (!/^-?\d+$/.test(value.trim())) return null;
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) ? parsed : null;
};

const isPositiveSafeInteger = (value: number, minimum = 1): boolean =>
  Number.isSafeInteger(value) && value >= minimum;

const assignInteger = (
  config: AdvancedConfig,
  key: keyof AdvancedConfig,
  arg: string,
): void => {
  const parsed = parseFiniteInteger(valueAfterEquals(arg));
  if (parsed === null) {
    config.invalidArgs.push(arg);
  } else {
    (config[key] as number) = parsed;
  }
};

/**
 * winws argüman dizisini AdvancedConfig objesine dönüştürür.
 * Unknown arguments are preserved for a lossless round trip and Rust performs
 * the authoritative validation before a preset can reach the engine.
 */
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
      if (KNOWN_STRATEGIES.includes(val)) {
        config.desyncMethod = val;
      } else {
        config.desyncMethod = 'custom';
        config.customDesyncMethod = val;
      }
    } else if (
      arg.startsWith('--dpi-desync-http=')
      || arg.startsWith('--dpi-desync-https=')
      || arg.startsWith('--dpi-desync-quic=')
      || arg.startsWith('--dpi-desync2=')
      || arg.startsWith('--dpi-desync-ttl-ext=')
      || arg.startsWith('--dpi-desync-split-pos-http-req=')
      || arg.startsWith('--dpi-desync-split-pos-tls=')
      || arg.startsWith('--dpi-desync-fake-tls-sni=')
      || arg.startsWith('--mss=')
      || arg.startsWith('--tcp-window-size=')
      || arg.startsWith('--bind-addr=')
    ) {
      config.invalidArgs.push(arg);
    } else if (arg.startsWith('--dpi-desync-cutoff=')) {
      config.desyncCutoff = valueAfterEquals(arg);
    } else if (arg.startsWith('--dpi-desync-split-pos=')) {
      assignInteger(config, 'splitPosition', arg);
    } else if (arg.startsWith('--dpi-desync-repeats=')) {
      assignInteger(config, 'desyncRepeats', arg);
    } else if (arg.startsWith('--dpi-desync-fooling=')) {
      config.desyncFooling = valueAfterEquals(arg).split(',').map(s => s.trim()).filter(Boolean);
    } else if (arg.startsWith('--dpi-desync-ttl=')) {
      assignInteger(config, 'fakeTtl', arg);
      config.autoTtl = false;
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
      assignInteger(config, 'tcpWindowSize', arg);
    } else if (arg.startsWith('--ipset=')) {
      config.invalidArgs.push(arg);
    } else if (arg.startsWith('--socks=')) {
      config.invalidArgs.push(arg);
    } else if (arg.startsWith('--wf-tcp=')) {
      config.httpPorts = valueAfterEquals(arg).replace(/,/g, ', ');
    } else if (arg.startsWith('--wf-udp=')) {
      config.quicUdpHandling = arg.includes('443');
    } else if (arg === '--dpi-desync-any-protocol') {
      config.anyProtocol = true;
    } else {
      config.passthroughArgs.push(arg);
    }
  }

  return config;
}

/**
 * AdvancedConfig objesini winws argüman dizisine dönüştürür.
 */
export function serializeConfigToArgs(config: AdvancedConfig): string[] {
  const args: string[] = [];

  if (config.httpPorts) {
    args.push(`--wf-tcp=${config.httpPorts.replace(/\s+/g, '')}`);
  }
  if (config.quicUdpHandling) {
    args.push('--wf-udp=443');
  }

  if (isPositiveSafeInteger(config.tcpWindowSize)) {
    args.push(`--wssize=${config.tcpWindowSize}`);
  }

  // Strategy
  const strategyVal = config.desyncMethod === 'custom'
    ? (config.customDesyncMethod || 'multisplit')
    : config.desyncMethod;

  if (strategyVal !== 'none') {
    args.push(`--dpi-desync=${strategyVal}`);

    if (config.anyProtocol) {
      args.push('--dpi-desync-any-protocol');
    }

    if (config.desyncCutoff) {
      args.push(`--dpi-desync-cutoff=${config.desyncCutoff}`);
    }

    // Bölme (split) konumları
    if (isPositiveSafeInteger(config.splitPosition)) {
      args.push(`--dpi-desync-split-pos=${config.splitPosition}`);
    }
    if (config.splitHttpReq && config.splitHttpReq !== 'none') {
      args.push(`--dpi-desync-split-http-req=${config.splitHttpReq}`);
    }
    if (config.splitTls && config.splitTls !== 'none') {
      args.push(`--dpi-desync-split-tls=${config.splitTls}`);
    }

    // Tekrar ve Evasion Fooling
    if (isPositiveSafeInteger(config.desyncRepeats, 2)) {
      args.push(`--dpi-desync-repeats=${config.desyncRepeats}`);
    }
    if (config.desyncFooling.length > 0) {
      args.push(`--dpi-desync-fooling=${config.desyncFooling.join(',')}`);
    }

    // TTL Evasion
    if (config.autoTtl) {
      args.push('--dpi-desync-autottl');
    } else if (isPositiveSafeInteger(config.fakeTtl)) {
      args.push(`--dpi-desync-ttl=${config.fakeTtl}`);
    }
  }

  for (const arg of config.passthroughArgs ?? []) {
    if (!args.includes(arg)) args.push(arg);
  }

  return args;
}
