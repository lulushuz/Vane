import type { AdvancedConfig } from '../store/engineStore';
import type { AdvancedConfigCandidate, VerifiedAdvancedConfig } from '../types/advanced';
import { DEFAULT_ADVANCED_CONFIG } from '../store/engineStore';
import { formatPortRanges } from './advancedValidator';

export function uiConfigToAdvancedCandidate(config: AdvancedConfig): AdvancedConfigCandidate {
  const selectedMethods = config.desyncMethod === 'custom'
    ? config.customDesyncMethod
    : config.desyncMethod;

  return {
    methods: selectedMethods.split(',').map((value) => value.trim()).filter(Boolean),
    tcpPorts: config.httpPorts,
    udpPorts: config.quicUdpHandling ? (config.udpPorts?.trim() || '443') : '',
    ttlMode: config.autoTtl ? 'auto' : config.fakeTtl > 0 ? 'fixed' : 'default',
    ttlValue: config.autoTtl || config.fakeTtl <= 0 ? undefined : config.fakeTtl,
    repeats: config.desyncRepeats > 0 ? config.desyncRepeats : undefined,
    fooling: [...config.desyncFooling],
    splitPosition: config.splitPosition > 0 ? String(config.splitPosition) : undefined,
    windowSize: config.tcpWindowSize > 0 ? String(config.tcpWindowSize) : undefined,
    passthrough: [...config.passthroughArgs],
  };
}

export function verifiedAdvancedConfigToUi(config: VerifiedAdvancedConfig): AdvancedConfig {
  return {
    ...DEFAULT_ADVANCED_CONFIG,
    httpPorts: formatPortRanges(config.traffic.tcp),
    quicUdpHandling: config.traffic.udp.length > 0,
    udpPorts: formatPortRanges(config.traffic.udp),
    desyncMethod: config.methods.join(','),
    customDesyncMethod: '',
    autoTtl: config.ttl.mode === 'auto',
    fakeTtl: config.ttl.mode === 'fixed' ? config.ttl.value : 0,
    desyncRepeats: config.repeats ?? 0,
    desyncFooling: [...config.fooling],
    splitPosition: config.split?.position?.kind === 'absolute'
      ? config.split.position.value
      : DEFAULT_ADVANCED_CONFIG.splitPosition,
    tcpWindowSize: config.windowSize?.value ?? 0,
    passthroughArgs: config.passthrough.map(String),
    invalidArgs: [],
  };
}
