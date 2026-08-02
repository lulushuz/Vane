/**
 * Rust tarafındaki `EngineStatus` enum'uyla birebir eşleşir.
 * `#[serde(tag = "variant", rename_all = "camelCase")]` sayesinde
 * Tauri bunu discriminated union olarak serialize eder.
 */
export type EngineStatus =
  | { variant: 'stopped' }
  | { variant: 'starting' }
  | { variant: 'waitingForReadiness'; pid: number }
  | { variant: 'ready'; pid: number; generation: number; revision: number; fingerprint: string }
  | { variant: 'running'; pid: number }
  | { variant: 'error'; message: string; code?: string };

/** Tek bir DPI bypass preset. Rust `Preset` struct'ıyla eşleşir. */
export interface Preset {
  id: string;
  label: string;
  description: string;
  icon: string;
  args: string[];
  isCustom: boolean;
  priority?: number;
  category?: string;
}

/** Log satırı — frontend'de oluşturulan meta-veri ile zenginleştirilir. */
export interface LogLine {
  id: string;          // benzersiz key (React rendering için)
  timestamp: Date;
  content: string;
  level: 'info' | 'warn' | 'error';
}
