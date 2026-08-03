import { describe, expect, it, vi } from 'vitest';
import {
  applyProviderAndRefresh,
  DnsTransactionGate,
  shouldIssueForwarderStart,
  type DnsForwarderStatus,
} from '../views/dnsForwarderFlow';

const activeStatus: DnsForwarderStatus = {
  active: true,
  port: 53,
  endpoint: 'https://cloudflare-dns.com/dns-query',
  protocol: 'doh',
  adblock: false,
  cache: true,
  watchdogEnabled: false,
};

describe('P0-06 DNS provider/forwarder flow', () => {
  it('provider_apply_refreshes_forwarder_status', async () => {
    const commands: string[] = [];
    const invoke = vi.fn(async (command: string) => {
      commands.push(command);
      return command === 'apply_dns_settings'
        ? { success: true, error: null }
        : activeStatus;
    });
    const flow = await applyProviderAndRefresh(invoke, {
      primary: '1.1.1.1',
      secondary: '1.0.0.1',
    });
    expect(commands).toEqual(['apply_dns_settings', 'get_doh_forwarder_status']);
    expect(flow.status).toEqual(activeStatus);
  });

  it('provider_apply_marks_forwarder_active', async () => {
    const invoke = vi.fn(async (command: string) =>
      command === 'apply_dns_settings' ? { success: true, error: null } : activeStatus,
    );
    const flow = await applyProviderAndRefresh(invoke, {
      primary: '1.1.1.1',
      secondary: '1.0.0.1',
    });
    expect(flow.status.active).toBe(true);
  });

  it('provider_apply_changes_button_from_start_to_stop', () => {
    expect(shouldIssueForwarderStart(activeStatus)).toBe(false);
  });

  it('provider_apply_followed_by_toggle_does_not_issue_second_start', async () => {
    const commands: string[] = [];
    const invoke = vi.fn(async (command: string) => {
      commands.push(command);
      return command === 'apply_dns_settings'
        ? { success: true, error: null }
        : activeStatus;
    });
    const flow = await applyProviderAndRefresh(invoke, {
      primary: '1.1.1.1',
      secondary: '1.0.0.1',
    });
    if (shouldIssueForwarderStart(flow.status)) commands.push('start_doh_forwarder');
    expect(commands).not.toContain('start_doh_forwarder');
  });

  it('provider_and_toggle_transactions_cannot_overlap', () => {
    const gate = new DnsTransactionGate();
    expect(gate.tryEnter()).toBe(true);
    expect(gate.tryEnter()).toBe(false);
    gate.leave();
    expect(gate.tryEnter()).toBe(true);
  });

  it('failed_provider_apply_keeps_forwarder_state_synced', async () => {
    const commands: string[] = [];
    const inactive = { ...activeStatus, active: false };
    const invoke = vi.fn(async (command: string) => {
      commands.push(command);
      return command === 'apply_dns_settings'
        ? { success: false, error: 'apply failed' }
        : inactive;
    });
    const flow = await applyProviderAndRefresh(invoke, {
      primary: '1.1.1.1',
      secondary: '1.0.0.1',
    });
    expect(flow.result?.success).toBe(false);
    expect(flow.status.active).toBe(false);
    expect(commands).toEqual(['apply_dns_settings', 'get_doh_forwarder_status']);
  });
});
