import type { EngineStatus, Preset } from './engine';

export const DEFAULT_HEALTH_CHECK_TARGET = 'example.com';

/** `start_engine` komutunun parametresi */
export interface StartEnginePayload {
  presetId: string;
}

/** `save_custom_preset` komutunun parametresi */
export interface SaveCustomPresetPayload {
  preset: Preset;
}

/** `delete_custom_preset` komutunun parametresi */
export interface DeleteCustomPresetPayload {
  presetId: string;
}

/** `get_engine_status` dönüş tipi */
export type GetEngineStatusResponse = EngineStatus;

/** `list_presets` dönüş tipi */
export type ListPresetsResponse = Preset[];

export interface IpcErrorPayload {
  code: string;
  message: string;
  operation?: string;
  retryable?: boolean;
  configRevision?: number;
}

export interface BypassConfigStatus {
  mode: 'all' | 'whitelist' | 'blacklist';
  domainCount: number;
  configRevision: number;
  stage: 'prepared' | 'process_started';
  engineRestarted: boolean;
  engineRunning: boolean;
  whitelistDomains: string[];
  blacklistDomains: string[];
  activePresetId: string;
}

export interface DnsConfigStatus {
  protocol: 'doh' | 'dot';
  adblock: boolean;
  cache: boolean;
  socks5Proxy: string;
  forwarderActive: boolean;
  configRevision: number;
  stage: 'persisted' | 'applied' | 'superseded' | 'disabled' | 'rolled_back' | 'forwarder_started';
  superseded?: boolean;
}

export function normalizeIpcError(error: unknown): IpcErrorPayload {
  if (typeof error === 'object' && error !== null) {
    const payload = error as Record<string, unknown>;
    return {
      code: typeof payload.code === 'string' ? payload.code : 'UNKNOWN',
      message: typeof payload.message === 'string' ? payload.message : String(error),
      operation: typeof payload.operation === 'string' ? payload.operation : undefined,
      retryable: typeof payload.retryable === 'boolean' ? payload.retryable : undefined,
      configRevision: typeof payload.configRevision === 'number' ? payload.configRevision : undefined,
    };
  }

  return {
    code: 'UNKNOWN',
    message: error instanceof Error ? error.message : String(error),
  };
}

export type DiagnosticSeverity = 'debug' | 'info' | 'warning' | 'error' | 'critical';
export type DiagnosticComponent = 'engine' | 'config' | 'dns' | 'firewall' | 'optimizer' | 'security' | 'system';
export type HealthState = 'healthy' | 'degraded' | 'unhealthy' | 'unknown';
export type DpiBypassAssessment = 'inconclusive' | 'unknown';

export interface SubsystemHealth {
  name: string;
  state: HealthState;
  message: string;
  lastCheckedMs: number;
}

export interface SystemHealthSnapshot {
  overall: HealthState;
  subsystems: SubsystemHealth[];
  timestampMs: number;
}

export interface TargetProbeResult {
  targetId: string;
  success: boolean;
  statusCode?: number;
  latencyMs?: number;
  error?: string;
}

export interface TrafficProbeReport {
  targets: TargetProbeResult[];
  successRatio: number;
  medianLatencyMs?: number;
  assessment: DpiBypassAssessment;
  timestampMs: number;
}

export interface DiagnosticEvent {
  sequence: number;
  timestampEpochMs: number;
  monotonicNs: number;
  component: DiagnosticComponent;
  code: string;
  severity: DiagnosticSeverity;
  fields: Record<string, string | number | boolean>;
}

export interface ArtifactIntegrityStatusDto {
  status: 'verified' | 'missing' | 'modified' | 'invalid_manifest' | 'unsupported_target' | string;
  target: string;
  verifiedArtifacts: number;
  failedArtifactId?: string;
  errorCode?: string;
  lastVerifiedAt?: string;
}

export interface DiagnosticsBundle {
  schemaVersion: string;
  appVersion: string;
  platform: string;
  timestampMs: number;
  healthSnapshot: SystemHealthSnapshot;
  events: DiagnosticEvent[];
  droppedEventCount: number;
  truncated: boolean;
  secretScannerPassed: boolean;
}

