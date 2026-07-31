import type { VerifiedAdvancedConfig } from '../types/advanced';
import { formatPortRanges } from './advancedValidator';

export function serializeVerifiedAdvancedConfig(
  config: VerifiedAdvancedConfig,
): string[] {
  const args: string[] = [];

  // 1. Traffic Filter Ports
  if (config.traffic.tcp.length > 0) {
    args.push(`--wf-tcp=${formatPortRanges(config.traffic.tcp)}`);
  }
  if (config.traffic.udp.length > 0) {
    args.push(`--wf-udp=${formatPortRanges(config.traffic.udp)}`);
  }

  // 2. Window Size
  if (config.windowSize) {
    args.push(`--wssize=${config.windowSize.value}`);
  }

  // 3. Desync Strategy
  if (config.methods.length > 0) {
    args.push(`--dpi-desync=${config.methods.join(',')}`);
  }

  // 4. TTL
  if (config.ttl.mode === 'auto') {
    args.push('--dpi-desync-autottl');
  } else if (config.ttl.mode === 'fixed') {
    args.push(`--dpi-desync-ttl=${config.ttl.value}`);
  }

  // 5. Repeats
  if (config.repeats !== undefined) {
    args.push(`--dpi-desync-repeats=${config.repeats}`);
  }

  // 6. Fooling
  if (config.fooling.length > 0) {
    args.push(`--dpi-desync-fooling=${config.fooling.join(',')}`);
  }

  // 7. Split Position
  if (config.split && config.split.position) {
    const pos = config.split.position;
    if (pos.kind === 'absolute') {
      args.push(`--dpi-desync-split-pos=${pos.value}`);
    } else if (pos.kind === 'selector') {
      if (pos.offset > 0) {
        args.push(`--dpi-desync-split-pos=${pos.selector}+${pos.offset}`);
      } else if (pos.offset < 0) {
        args.push(`--dpi-desync-split-pos=${pos.selector}${pos.offset}`);
      } else {
        args.push(`--dpi-desync-split-pos=${pos.selector}`);
      }
    }
  }

  // 8. Validated Passthrough
  for (const passthroughArg of config.passthrough) {
    args.push(passthroughArg);
  }

  return args;
}
