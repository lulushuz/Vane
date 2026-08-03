import { invoke } from '@tauri-apps/api/core';
import { create } from 'zustand';
import type {
  ArtifactIntegrityStatusDto,
  DiagnosticEvent,
  SystemHealthSnapshot,
  TrafficProbeReport,
} from '../types/ipc';

interface DiagnosticsState {
  integrityStatus: ArtifactIntegrityStatusDto | null;
  isCheckingIntegrity: boolean;
  healthSnapshot: SystemHealthSnapshot | null;
  trafficReport: TrafficProbeReport | null;
  events: DiagnosticEvent[];
  isHealthChecking: boolean;
  isProbeRunning: boolean;
  isExporting: boolean;
  lastExportPath: string | null;
  error: string | null;

  fetchArtifactIntegrity: () => Promise<ArtifactIntegrityStatusDto | null>;
  runLocalDiagnostics: () => Promise<SystemHealthSnapshot | null>;
  runTrafficDiagnostics: (targets?: string[]) => Promise<TrafficProbeReport | null>;
  cancelTrafficDiagnostics: () => Promise<boolean>;
  exportDiagnosticsBundle: (targetPath: string) => Promise<string | null>;
  pushEvent: (event: DiagnosticEvent) => void;
  clearEvents: () => void;
}

let integrityRequestId = 0;
let localDiagnosticsRequestId = 0;
let trafficProbeRequestId = 0;
let exportRequestId = 0;

export const useDiagnosticsStore = create<DiagnosticsState>((set) => ({
  integrityStatus: null,
  isCheckingIntegrity: false,
  healthSnapshot: null,
  trafficReport: null,
  events: [],
  isHealthChecking: false,
  isProbeRunning: false,
  isExporting: false,
  lastExportPath: null,
  error: null,

  fetchArtifactIntegrity: async () => {
    const requestId = ++integrityRequestId;
    set({ isCheckingIntegrity: true, error: null });
    try {
      const status = await invoke<ArtifactIntegrityStatusDto>('get_artifact_integrity_status');
      if (requestId !== integrityRequestId) return null;
      set({ integrityStatus: status, isCheckingIntegrity: false });
      return status;
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        isCheckingIntegrity: false,
      });
      return null;
    }
  },


  runLocalDiagnostics: async () => {
    const requestId = ++localDiagnosticsRequestId;
    set({ isHealthChecking: true, error: null });
    try {
      const snapshot = await invoke<SystemHealthSnapshot>('run_local_diagnostics');
      if (requestId !== localDiagnosticsRequestId) return null;
      set({ healthSnapshot: snapshot, isHealthChecking: false });
      return snapshot;
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        isHealthChecking: false,
      });
      return null;
    }
  },

  runTrafficDiagnostics: async (targets?: string[]) => {
    const requestId = ++trafficProbeRequestId;
    set({ isProbeRunning: true, error: null });
    try {
      const report = await invoke<TrafficProbeReport>('run_traffic_diagnostics', {
        targets,
      });
      if (requestId !== trafficProbeRequestId) return null;
      set({ trafficReport: report, isProbeRunning: false });
      return report;
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        isProbeRunning: false,
      });
      return null;
    }
  },

  cancelTrafficDiagnostics: async () => {
    ++trafficProbeRequestId;
    try {
      const ok = await invoke<boolean>('cancel_traffic_diagnostics');
      set({ isProbeRunning: false });
      return ok;
    } catch (err) {
      set({ isProbeRunning: false });
      return false;
    }
  },

  exportDiagnosticsBundle: async (targetPath: string) => {
    const requestId = ++exportRequestId;
    set({ isExporting: true, error: null });
    try {
      const exportedPath = await invoke<string>('export_diagnostics_bundle', {
        exportPath: targetPath,
      });
      if (requestId !== exportRequestId) return null;
      set({ lastExportPath: exportedPath, isExporting: false });
      return exportedPath;
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        isExporting: false,
      });
      return null;
    }
  },

  pushEvent: (event: DiagnosticEvent) => {
    set((state) => {
      // Keep up to 500 events in memory stream
      const bySequence = new Map(state.events.map((item) => [item.sequence, item]));
      bySequence.set(event.sequence, event);
      const nextEvents = [...bySequence.values()].sort((a, b) => b.sequence - a.sequence).slice(0, 500);
      return { events: nextEvents };
    });
  },

  clearEvents: () => {
    set({ events: [] });
    void invoke('clear_diagnostic_events');
  },
}));
