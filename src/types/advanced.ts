export type DesyncMethod =
  | 'syndata'
  | 'rst'
  | 'rstack'
  | 'fake'
  | 'fakeknown'
  | 'split'
  | 'split2'
  | 'multisplit'
  | 'disorder'
  | 'multidisorder'
  | 'hostfake'
  | 'fakedsplit'
  | 'destopt'
  | 'ipfrag1'
  | 'ipfrag2'
  | 'udplen'
  | 'tamper'
  | 'none';

export type FoolingMethod =
  | 'badseq'
  | 'badsum'
  | 'md5sig'
  | 'datanoack'
  | 'ts';

export interface PortRange {
  start: number;
  end: number;
}

export interface CapabilityStatus {
  state: 'supported' | 'experimental' | 'unsupported';
  reason?: string;
}

export interface AdvancedCapabilities {
  platform: 'windows' | 'linux';
  methods: Record<string, CapabilityStatus>;
  traffic: {
    tcpFiltering: CapabilityStatus;
    udpFiltering: CapabilityStatus;
    customTcpPorts: CapabilityStatus;
    customUdpPorts: CapabilityStatus;
  };
  options: {
    autoTtl: CapabilityStatus;
    fixedTtl: CapabilityStatus;
    repeats: CapabilityStatus;
    fooling: CapabilityStatus;
    splitPosition: CapabilityStatus;
    windowSize: CapabilityStatus;
    mss: CapabilityStatus;
    fakePayload: CapabilityStatus;
    fakeTlsSni: CapabilityStatus;
    bindAddress: CapabilityStatus;
    ipset: CapabilityStatus;
    tpws: CapabilityStatus;
  };
}

export interface AdvancedConfigCandidate {
  methods: string[];
  tcpPorts: string;
  udpPorts: string;
  ttlMode: 'default' | 'auto' | 'fixed';
  ttlValue?: number;
  repeats?: number;
  fooling: string[];
  splitPosition?: string;
  windowSize?: string;
  passthrough: string[];
}

export type VerifiedTtl =
  | { mode: 'default' }
  | { mode: 'auto' }
  | { mode: 'fixed'; value: number };

export interface VerifiedTrafficFilter {
  tcp: PortRange[];
  udp: PortRange[];
}

export type SplitPosition =
  | { kind: 'absolute'; value: number }
  | { kind: 'selector'; selector: string; offset: number };

export interface VerifiedSplitConfig {
  position?: SplitPosition;
}

export interface VerifiedWindowSize {
  kind: 'bytes';
  value: number;
}

export type ValidatedPassthroughArg = string & {
  readonly __validatedPassthrough: unique symbol;
};

export interface VerifiedAdvancedConfig {
  methods: DesyncMethod[];
  traffic: VerifiedTrafficFilter;
  ttl: VerifiedTtl;
  repeats?: number;
  fooling: FoolingMethod[];
  split?: VerifiedSplitConfig;
  windowSize?: VerifiedWindowSize;
  passthrough: ValidatedPassthroughArg[];
}

export interface AdvancedValidationIssue {
  field?: string;
  message: string;
}

export type AdvancedValidationResult =
  | {
      valid: true;
      config: VerifiedAdvancedConfig;
      warnings: AdvancedValidationIssue[];
    }
  | {
      valid: false;
      errors: AdvancedValidationIssue[];
      warnings: AdvancedValidationIssue[];
    };

export interface AdvancedParseDiagnostic {
  argument: string;
  code: string;
  message: string;
  severity: 'error' | 'warning';
}

export interface AdvancedParseResult {
  candidate: AdvancedConfigCandidate;
  passthrough: ValidatedPassthroughArg[];
  diagnostics: AdvancedParseDiagnostic[];
  lossless: boolean;
}

export interface AdvancedConfig {
  httpPorts: string;
  quicUdpHandling: boolean;
  udpPorts?: string;
  desyncMethod: string;
  customDesyncMethod: string;
  autoTtl: boolean;
  fakeTtl: number;
  desyncRepeats: number;
  desyncFooling: string[];
  splitPosition: number;
  tcpWindowSize: number;
  anyProtocol: boolean;
  desyncCutoff: string;
  splitHttpReq: string;
  splitTls: string;
  fakeHttpPayload: string;
  fakeTlsPayload: string;
  fakeQuicPayload: string;
  passthroughArgs: string[];
  invalidArgs: string[];

  // Unsupported phantom fields preserved as optional for UI contract compatibility
  desyncHttp?: string;
  desyncHttps?: string;
  desyncQuic?: string;
  desync2?: string;
  splitPosHttpReq?: string;
  splitPosTls?: string;
  fakeTtlExt?: string;
  mssFix?: number;
  fakeTlsSni?: string;
  bindInterface?: string;
}

export const DEFAULT_ADVANCED_CONFIG: AdvancedConfig = {
  httpPorts: '80, 443',
  quicUdpHandling: true,
  udpPorts: '',
  desyncMethod: 'fake,multidisorder',
  customDesyncMethod: '',
  autoTtl: true,
  fakeTtl: 0,
  desyncRepeats: 0,
  desyncFooling: ['badseq'],
  splitPosition: 1,
  tcpWindowSize: 0,
  anyProtocol: true,
  desyncCutoff: 'd3',
  splitHttpReq: '',
  splitTls: '',
  fakeHttpPayload: '',
  fakeTlsPayload: '',
  fakeQuicPayload: '',
  passthroughArgs: [],
  invalidArgs: [],
};
