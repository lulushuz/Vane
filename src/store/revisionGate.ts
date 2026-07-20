export class MonotonicRevisionGate {
  private latest = 0;

  accept(revision: number): boolean {
    if (!Number.isSafeInteger(revision) || revision <= this.latest) return false;
    this.latest = revision;
    return true;
  }
}
