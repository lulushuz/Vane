import { beforeEach, describe, expect, it } from 'vitest';
import { useDiagnosticsStore } from '../store/diagnosticsStore';
import { mockIpc } from './mockIpc';

describe('P14 Frontend — Diagnostics & Observability Test Suite', () => {
  beforeEach(() => {
    mockIpc.reset();
    useDiagnosticsStore.setState({
      healthSnapshot: null,
      trafficReport: null,
      events: [],
      isHealthChecking: false,
      isProbeRunning: false,
      isExporting: false,
      lastExportPath: null,
      error: null,
    });
  });

  it('FE-01: runLocalDiagnostics updates health snapshot', async () => {
    mockIpc.registerHandler('run_local_diagnostics', () => ({
      overall: 'healthy',
      subsystems: [
        { name: 'Engine', state: 'healthy', message: 'OK', lastCheckedMs: 1000 },
      ],
      timestampMs: 1000,
    }));

    const res = await useDiagnosticsStore.getState().runLocalDiagnostics();
    expect(res?.overall).toBe('healthy');
    expect(useDiagnosticsStore.getState().healthSnapshot?.overall).toBe('healthy');
  });

  it('FE-02: runTrafficDiagnostics returns inconclusive assessment', async () => {
    mockIpc.registerHandler('run_traffic_diagnostics', () => ({
      targets: [
        { target: 'youtube.com', success: true, statusCode: 200, latencyMs: 45 },
      ],
      successRatio: 1.0,
      medianLatencyMs: 45,
      assessment: 'inconclusive',
      timestampMs: 1000,
    }));

    const report = await useDiagnosticsStore.getState().runTrafficDiagnostics();
    expect(report?.assessment).toBe('inconclusive');
    expect(report?.successRatio).toBe(1.0);
  });

  it('FE-03: cancelTrafficDiagnostics updates state', async () => {
    mockIpc.registerHandler('cancel_traffic_diagnostics', () => true);

    useDiagnosticsStore.setState({ isProbeRunning: true });
    const canceled = await useDiagnosticsStore.getState().cancelTrafficDiagnostics();

    expect(canceled).toBe(true);
    expect(useDiagnosticsStore.getState().isProbeRunning).toBe(false);
  });

  it('FE-04: exportDiagnosticsBundle sets last export path', async () => {
    mockIpc.registerHandler('export_diagnostics_bundle', () => '/tmp/diag.vane-diag.json');

    const path = await useDiagnosticsStore.getState().exportDiagnosticsBundle('/tmp/diag.vane-diag.json');
    expect(path).toBe('/tmp/diag.vane-diag.json');
    expect(useDiagnosticsStore.getState().lastExportPath).toBe('/tmp/diag.vane-diag.json');
  });

  it('FE-05: pushEvent and clearEvents manage diagnostic event stream', () => {
    const store = useDiagnosticsStore.getState();
    store.pushEvent({
      sequence: 1,
      timestampEpochMs: 1000,
      monotonicNs: 100,
      component: 'engine',
      code: 'ENG_START_INIT',
      severity: 'info',
      fields: { preset: 'tr-1' },
    });

    expect(useDiagnosticsStore.getState().events.length).toBe(1);

    store.clearEvents();
    expect(useDiagnosticsStore.getState().events.length).toBe(0);
  });
});
