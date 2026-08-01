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
    set({ isCheckingIntegrity: true, error: null });
    try {
      const status = await invoke<ArtifactIntegrityStatusDto>('get_artifact_integrity_status');
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
    set({ isHealthChecking: true, error: null });
    try {
      const snapshot = await invoke<SystemHealthSnapshot>('run_local_diagnostics');
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
    set({ isProbeRunning: true, error: null });
    try {
      const report = await invoke<TrafficProbeReport>('run_traffic_diagnostics', {
        targets,
      });
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
    set({ isExporting: true, error: null });
    try {
      const exportedPath = await invoke<string>('export_diagnostics_bundle', {
        exportPath: targetPath,
      });
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
      const nextEvents = [event, ...state.events].slice(0, 500);
      return { events: nextEvents };
    });
  },

  clearEvents: () => {
    set({ events: [] });
  },
}));
