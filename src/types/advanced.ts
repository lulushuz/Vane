export interface AdvancedConfig {
  desyncMethod: string;
  customDesyncMethod: string;
  splitPosition: number;
  desyncRepeats: number;
  desyncFooling: string[];
  anyProtocol: boolean;
  autoTtl: boolean;
  fakeTtl: number;
  mssFix: number;
  quicUdpHandling: boolean;
  httpPorts: string;
  desyncHttp: string;
  desyncHttps: string;
  desyncQuic: string;
  desyncCutoff: string;
  splitHttpReq: string;
  splitPosHttpReq: number;
  splitTls: string;
  splitPosTls: number;
  fakeTtlExt: number;
  fakeTlsSni: string;
  fakeHttpPayload: string;
  fakeTlsPayload: string;
  fakeQuicPayload: string;
  desync2: string;
  tcpWindowSize: number;
  ipsetPath: string;
  tpwsMode: boolean;
  bindInterface: string;
  passthroughArgs: string[];
  invalidArgs: string[];
}

export const DEFAULT_ADVANCED_CONFIG: AdvancedConfig = {
  desyncMethod: 'custom',
  customDesyncMethod: 'fake,multidisorder',
  splitPosition: 1,
  desyncRepeats: 1,
  desyncFooling: ['badseq'],
  anyProtocol: true,
  autoTtl: true,
  fakeTtl: 4,
  mssFix: 1300,
  quicUdpHandling: true,
  httpPorts: '80, 443',
  desyncHttp: 'none',
  desyncHttps: 'none',
  desyncQuic: 'none',
  desyncCutoff: 'd3',
  splitHttpReq: 'none',
  splitPosHttpReq: 0,
  splitTls: 'none',
  splitPosTls: 0,
  fakeTtlExt: 0,
  fakeTlsSni: '',
  fakeHttpPayload: '',
  fakeTlsPayload: '',
  fakeQuicPayload: '',
  desync2: 'none',
  tcpWindowSize: 0,
  ipsetPath: '',
  tpwsMode: false,
  bindInterface: '',
  passthroughArgs: [],
  invalidArgs: [],
};
