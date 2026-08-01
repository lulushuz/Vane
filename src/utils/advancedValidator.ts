import type {
  AdvancedCapabilities,
  AdvancedConfigCandidate,
  AdvancedValidationIssue,
  AdvancedValidationResult,
  DesyncMethod,
  FoolingMethod,
  PortRange,
  VerifiedAdvancedConfig,
  VerifiedTrafficFilter,
  VerifiedTtl,
  ValidatedPassthroughArg,
} from '../types/advanced';

const PHASE_MAP: Record<string, number> = {
  syndata: 0,
  rst: 0,
  rstack: 0,
  fake: 1,
  fakeknown: 1,
  split: 1,
  split2: 1,
  multisplit: 1,
  disorder: 1,
  multidisorder: 1,
  hostfake: 1,
  fakedsplit: 1,
  destopt: 2,
  ipfrag1: 2,
  ipfrag2: 2,
  udplen: 2,
  tamper: 2,
  none: 2,
};

const VALID_DESYNC_METHODS = new Set<string>(Object.keys(PHASE_MAP));
const VALID_FOOLING_METHODS = new Set<string>([
  'badseq',
  'badsum',
  'md5sig',
  'datanoack',
  'ts',
]);

const FORBIDDEN_SHELL_CHARS = /[;&|`$()<>\\]/;

export function parsePortRanges(input: string): PortRange[] {
  if (!input || !input.trim()) return [];
  const tokens = input
    .split(',')
    .map((t) => t.trim())
    .filter(Boolean);
  const ranges: PortRange[] = [];
  for (const token of tokens) {
    if (token.includes('-')) {
      const parts = token.split('-');
      if (parts.length !== 2) continue;
      const start = parseInt(parts[0], 10);
      const end = parseInt(parts[1], 10);
      if (
        Number.isInteger(start) &&
        Number.isInteger(end) &&
        start >= 1 &&
        end <= 65535 &&
        start <= end
      ) {
        ranges.push({ start, end });
      }
    } else {
      const port = parseInt(token, 10);
      if (Number.isInteger(port) && port >= 1 && port <= 65535) {
        ranges.push({ start: port, end: port });
      }
    }
  }
  return ranges;
}

export function formatPortRanges(ranges: PortRange[]): string {
  return ranges
    .map((r) => (r.start === r.end ? `${r.start}` : `${r.start}-${r.end}`))
    .join(',');
}

export function validateAdvancedConfig(
  candidate: AdvancedConfigCandidate,
  capabilities: AdvancedCapabilities,
): AdvancedValidationResult {
  const errors: AdvancedValidationIssue[] = [];
  const warnings: AdvancedValidationIssue[] = [];

  // 1. Methods & Phase Sequence
  const methods: DesyncMethod[] = [];
  const seenMethods = new Set<string>();
  let lastPhase = -1;

  for (const rawMethod of candidate.methods) {
    const method = rawMethod.trim().toLowerCase();
    if (!method) continue;

    if (!VALID_DESYNC_METHODS.has(method)) {
      errors.push({
        field: 'methods',
        message: `Unknown desync method: ${method}`,
      });
      continue;
    }

    if (seenMethods.has(method)) {
      errors.push({
        field: 'methods',
        message: `Duplicate desync method: ${method}`,
      });
      continue;
    }
    seenMethods.add(method);

    const phase = PHASE_MAP[method];
    if (phase < lastPhase) {
      errors.push({
        field: 'methods',
        message: `Invalid desync phase sequence: ${method} (Phase ${phase}) follows a higher phase (Phase ${lastPhase})`,
      });
    }
    lastPhase = phase;
    methods.push(method as DesyncMethod);
  }

  if (methods.length > 3) {
    errors.push({
      field: 'methods',
      message: `Maximum 3 desync methods allowed per strategy (got ${methods.length})`,
    });
  }

  if (methods.includes('none' as DesyncMethod) && methods.length > 1) {
    errors.push({
      field: 'methods',
      message: `'none' desync method cannot be combined with other desync methods`,
    });
  }

  // 2. Traffic Ports
  const tcpRanges = parsePortRanges(candidate.tcpPorts);
  const udpRanges = parsePortRanges(candidate.udpPorts);

  const traffic: VerifiedTrafficFilter = {
    tcp: tcpRanges,
    udp: udpRanges,
  };

  if (
    capabilities.platform === 'linux' &&
    udpRanges.length > 0 &&
    capabilities.traffic.udpFiltering.state === 'unsupported'
  ) {
    warnings.push({
      field: 'udpPorts',
      message: capabilities.traffic.udpFiltering.reason || 'UDP filtering is unsupported on Linux',
    });
  }

  // 3. TTL
  let ttl: VerifiedTtl = { mode: 'default' };
  if (candidate.ttlMode === 'auto') {
    ttl = { mode: 'auto' };
  } else if (candidate.ttlMode === 'fixed') {
    const val = candidate.ttlValue;
    if (val === undefined || !Number.isInteger(val) || val < 1 || val > 255) {
      errors.push({
        field: 'ttlValue',
        message: 'Fixed TTL value must be an integer between 1 and 255',
      });
    } else {
      ttl = { mode: 'fixed', value: val };
    }
  }

  // 4. Repeats
  let repeats: number | undefined;
  if (candidate.repeats !== undefined && candidate.repeats !== null) {
    if (
      !Number.isInteger(candidate.repeats) ||
      candidate.repeats < 1 ||
      candidate.repeats > 20
    ) {
      errors.push({
        field: 'repeats',
        message: 'Desync repeats must be an integer between 1 and 20',
      });
    } else {
      repeats = candidate.repeats;
    }
  }

  // 5. Fooling
  const fooling: FoolingMethod[] = [];
  const seenFooling = new Set<string>();
  for (const rawF of candidate.fooling) {
    const f = rawF.trim().toLowerCase();
    if (!f) continue;
    if (!VALID_FOOLING_METHODS.has(f)) {
      errors.push({
        field: 'fooling',
        message: `Unknown fooling method: ${f}`,
      });
      continue;
    }
    if (!seenFooling.has(f)) {
      seenFooling.add(f);
      fooling.push(f as FoolingMethod);
    }
  }

  // 6. Split Position
  let split: VerifiedAdvancedConfig['split'];
  if (candidate.splitPosition && candidate.splitPosition.trim()) {
    const isSplitMethodPresent = methods.some((m) =>
      ['split', 'split2', 'multisplit', 'disorder', 'multidisorder', 'fakedsplit'].includes(m),
    );

    if (!isSplitMethodPresent) {
      errors.push({
        field: 'splitPosition',
        message: 'Split position argument requires a split-based desync method (e.g. split, multisplit, disorder)',
      });
    }

    const rawPos = candidate.splitPosition.trim();
    if (/^-?\d+$/.test(rawPos)) {
      const val = parseInt(rawPos, 10);
      split = { position: { kind: 'absolute', value: val } };
    } else if (rawPos.startsWith('sniext')) {
      let offset = 0;
      if (rawPos.includes('+')) {
        offset = parseInt(rawPos.split('+')[1], 10) || 0;
      } else if (rawPos.includes('-')) {
        offset = -(parseInt(rawPos.split('-')[1], 10) || 0);
      }
      split = { position: { kind: 'selector', selector: 'sniext', offset } };
    } else {
      errors.push({
        field: 'splitPosition',
        message: `Invalid split position format: ${rawPos}`,
      });
    }
  }

  // 7. Window Size
  let windowSize: VerifiedAdvancedConfig['windowSize'];
  if (candidate.windowSize && candidate.windowSize.trim()) {
    const val = parseInt(candidate.windowSize.trim(), 10);
    if (!Number.isInteger(val) || val < 1 || val > 65535) {
      errors.push({
        field: 'windowSize',
        message: 'Window size must be an integer between 1 and 65535',
      });
    } else {
      windowSize = { kind: 'bytes', value: val };
    }
  }

  // 8. Passthrough
  const passthrough: ValidatedPassthroughArg[] = [];
  for (const rawArg of candidate.passthrough) {
    const arg = rawArg.trim();
    if (!arg) continue;
    if (FORBIDDEN_SHELL_CHARS.test(arg)) {
      errors.push({
        field: 'passthrough',
        message: `Forbidden shell character in passthrough argument: ${arg}`,
      });
      continue;
    }
    if (arg.startsWith('--hostlist=') || arg.startsWith('--hostlist-exclude=')) {
      errors.push({
        field: 'passthrough',
        message: `Hostlist path injection in passthrough argument is forbidden: ${arg}`,
      });
      continue;
    }
    passthrough.push(arg as ValidatedPassthroughArg);
  }

  if (errors.length > 0) {
    return { valid: false, errors, warnings };
  }

  const verifiedConfig: VerifiedAdvancedConfig = {
    methods,
    traffic,
    ttl,
    repeats,
    fooling,
    split,
    windowSize,
    passthrough,
  };

  return { valid: true, config: verifiedConfig, warnings };
}
