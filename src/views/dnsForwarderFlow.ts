export interface DnsApplyResult {
  success: boolean;
  error: string | null;
}

export interface DnsForwarderStatus {
  active: boolean;
  port: number;
  endpoint: string;
  protocol: 'doh' | 'dot';
  adblock: boolean;
  cache: boolean;
  watchdogEnabled: boolean;
}

type Invoke = (command: string, args?: Record<string, unknown>) => Promise<unknown>;

export class DnsTransactionGate {
  private active = false;

  tryEnter(): boolean {
    if (this.active) return false;
    this.active = true;
    return true;
  }

  leave(): void {
    this.active = false;
  }

  isBusy(): boolean {
    return this.active;
  }
}

export async function applyProviderAndRefresh(
  invokeCommand: Invoke,
  args: { primary: string; secondary: string },
): Promise<{ result?: DnsApplyResult; status: DnsForwarderStatus; error?: unknown }> {
  let result: DnsApplyResult | undefined;
  let error: unknown;
  try {
    result = await invokeCommand('apply_dns_settings', args) as DnsApplyResult;
  } catch (caught) {
    error = caught;
  }
  const status = await invokeCommand('get_doh_forwarder_status') as DnsForwarderStatus;
  return { result, status, error };
}

export function shouldIssueForwarderStart(status: DnsForwarderStatus): boolean {
  return !status.active;
}
