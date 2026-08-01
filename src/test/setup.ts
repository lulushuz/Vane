if (typeof window === 'undefined') {
  (globalThis as any).window = globalThis;
}
if (!(globalThis as any).window.__TAURI_INTERNALS__) {
  (globalThis as any).window.__TAURI_INTERNALS__ = {
    invoke: (cmd: string, args: any) =>
      (globalThis as any).__mockIpc?.handleInvoke(cmd, args) ?? Promise.resolve(null),
    plugins: {},
  };
}
