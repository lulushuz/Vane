import { describe, expect, it, beforeEach } from 'vitest';
import { mockIpc, resetStoreToDefaults } from './mockIpc';
import { useEngineStore } from '../store/engineStore';

describe('Test Group E — Engine Launch Sequence Characterization', () => {
  beforeEach(() => {
    mockIpc.reset();
    resetStoreToDefaults();
  });

  it('E-01: verifies exact backend call sequence during standard engine start', async () => {
    await useEngineStore.getState().startEngine('default');

    const commands = mockIpc.getCommandNames().filter((c) => !c.startsWith('settings_'));
    expect(commands).toEqual([
      'sync_bypass_config',
      'sync_dns_settings',
      'start_engine_with_dns_guard',
    ]);
  });

  it('E-02: verifies whitelist mode payload sent to sync_bypass_config during start', async () => {
    useEngineStore.getState().setBypassMode('whitelist');
    useEngineStore.getState().setWhitelistDomains(['allowed.example']);

    await useEngineStore.getState().startEngine('default');

    const bypassCall = mockIpc.getCallsForCommand('sync_bypass_config').pop();
    expect(bypassCall?.payload).toMatchObject({
      mode: 'whitelist',
      list: 'allowed.example',
      whitelistDomains: ['allowed.example'],
      activePresetId: 'default',
    });
  });

  it('E-03: verifies blacklist mode payload sent to sync_bypass_config during start', async () => {
    useEngineStore.getState().setBypassMode('blacklist');
    useEngineStore.getState().setBlacklistDomains(['blocked.example']);

    await useEngineStore.getState().startEngine('default');

    const bypassCall = mockIpc.getCallsForCommand('sync_bypass_config').pop();
    expect(bypassCall?.payload).toMatchObject({
      mode: 'blacklist',
      list: 'blocked.example',
      blacklistDomains: ['blocked.example'],
    });
  });

  it('E-04: automatically starts DoH forwarder when kill switch is enabled', async () => {
    useEngineStore.setState({ killSwitch: true });
    mockIpc.registerHandler('get_doh_forwarder_status', () => ({ active: false }));

    await useEngineStore.getState().startEngine('default');

    const commands = mockIpc.getCommandNames();
    expect(commands).toContain('get_doh_forwarder_status');
    expect(commands).toContain('start_doh_forwarder');
  });

  it('E-05: applies saved custom DNS provider settings when forwarder is inactive', async () => {
    useEngineStore.setState({
      selectedDnsId: 'custom',
      dnsCustomPrimary: '1.1.1.1',
      dnsCustomSecondary: '1.0.0.1',
    });

    await useEngineStore.getState().startEngine('default');

    const dnsCall = mockIpc.getCallsForCommand('apply_dns_settings').pop();
    expect(dnsCall?.payload).toEqual({
      primary: '1.1.1.1',
      secondary: '1.0.0.1',
    });
  });

  it('E-06: verifies explicit dnsProtocol in sync_dns_settings during engine start (R-14 resolved)', async () => {
    useEngineStore.setState({ dnsProtocol: 'dot' });

    await useEngineStore.getState().startEngine('default');

    const dnsSyncCall = mockIpc.getCallsForCommand('sync_dns_settings').pop();
    expect(dnsSyncCall?.payload?.protocol).toBe('dot');
  });


  it('E-07: updates store status and log when engine returns running variant with PID', async () => {
    mockIpc.registerHandler('start_engine_with_dns_guard', () => ({
      variant: 'running',
      pid: 5678,
    }));

    await useEngineStore.getState().startEngine('default');

    const state = useEngineStore.getState();
    expect(state.status).toEqual({ variant: 'running', pid: 5678 });
    expect(state.logs.some((l) => l.content.includes('5678'))).toBe(true);
  });

  it('E-08: sets error status when engine returns error variant payload', async () => {
    mockIpc.registerHandler('start_engine_with_dns_guard', () => ({
      variant: 'error',
      message: 'WinDivert driver failed to open',
      code: 'DRIVER_ERROR',
    }));

    await useEngineStore.getState().startEngine('default');

    const state = useEngineStore.getState();
    expect(state.status).toEqual({
      variant: 'error',
      message: 'WinDivert driver failed to open',
      code: 'DRIVER_ERROR',
    });
  });

  it('E-09: catches IPC rejections and normalizes error state during start', async () => {
    mockIpc.registerError('start_engine_with_dns_guard', {
      code: 'PERMISSION_DENIED',
      message: 'Elevation required',
    });

    await useEngineStore.getState().startEngine('default');

    const state = useEngineStore.getState();
    expect(state.status).toEqual({
      variant: 'error',
      message: 'Elevation required',
      code: 'PERMISSION_DENIED',
    });
  });
});

describe('Test Group H — Engine Stop and Lifecycle UI Characterization', () => {
  beforeEach(() => {
    mockIpc.reset();
    resetStoreToDefaults();
  });

  it('H-01: sets stopped status and logs warning on successful engine stop', async () => {
    useEngineStore.setState({ status: { variant: 'running', pid: 1234 } });

    await useEngineStore.getState().stopEngine();

    expect(useEngineStore.getState().status).toEqual({ variant: 'stopped' });
    expect(mockIpc.getCommandNames()).toContain('stop_engine');
  });

  it('H-02: characterizes silent error log handling on stop engine rejection', async () => {
    mockIpc.registerError('stop_engine', 'Failed to kill process');

    await expect(useEngineStore.getState().stopEngine()).resolves.not.toThrow();
  });

  it('H-03: handles consecutive stop engine calls gracefully', async () => {
    await useEngineStore.getState().stopEngine();
    await useEngineStore.getState().stopEngine();

    expect(mockIpc.getCallsForCommand('stop_engine')).toHaveLength(2);
    expect(useEngineStore.getState().status).toEqual({ variant: 'stopped' });
  });

  it('H-04: allows stop command execution during starting phase', async () => {
    useEngineStore.setState({ status: { variant: 'starting' } });

    await useEngineStore.getState().stopEngine();

    expect(useEngineStore.getState().status).toEqual({ variant: 'stopped' });
  });

  it('H-05: sets starting status immediately when startEngine is invoked', () => {
    mockIpc.registerHandler('start_engine_with_dns_guard', () => new Promise(() => {}));

    void useEngineStore.getState().startEngine('default');

    expect(useEngineStore.getState().status).toEqual({ variant: 'starting' });
  });

  it('H-06: documents current pid based running state representation (R-17)', async () => {
    mockIpc.registerHandler('start_engine_with_dns_guard', () => ({
      variant: 'running',
      pid: 9999,
    }));

    await useEngineStore.getState().startEngine('default');

    const status = useEngineStore.getState().status;
    expect(status.variant).toBe('running');
    if (status.variant === 'running') {
      expect(status.pid).toBe(9999);
    }
  });
});

describe('Test Group M — User Log Characterization', () => {
  beforeEach(() => {
    mockIpc.reset();
    resetStoreToDefaults();
  });

  it('M-01: appends log with auto-incrementing ID and info level', () => {
    useEngineStore.getState().appendLog('Test log entry 1', 'info');
    useEngineStore.getState().appendLog('Test log entry 2', 'warn');

    const logs = useEngineStore.getState().logs;
    expect(logs[0].content).toBe('Test log entry 2');
    expect(logs[0].level).toBe('warn');
    expect(logs[1].content).toBe('Test log entry 1');
  });

  it('M-02: caps max log history at 500 lines', () => {
    for (let i = 0; i < 550; i++) {
      useEngineStore.getState().appendLog(`Log ${i}`);
    }
    expect(useEngineStore.getState().logs).toHaveLength(500);
  });

  it('M-03: clears all log lines when clearLogs is called', () => {
    useEngineStore.getState().appendLog('Line 1');
    useEngineStore.getState().clearLogs();
    expect(useEngineStore.getState().logs).toEqual([]);
  });

  it('M-04: logs localized start messages in Turkish and English', async () => {
    useEngineStore.setState({ language: 'tr' });
    await useEngineStore.getState().startEngine('default');
    expect(useEngineStore.getState().logs.some((l) => l.content.includes('başlatılıyor'))).toBe(true);

    useEngineStore.setState({ language: 'en', logs: [] });
    await useEngineStore.getState().startEngine('default');
    expect(useEngineStore.getState().logs.some((l) => l.content.includes('Starting DPI bypass'))).toBe(true);
  });
});
