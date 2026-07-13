use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter};
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::prelude::*;
use tracing_subscriber::Layer;

static APP_HANDLE: Mutex<Option<AppHandle>> = Mutex::new(None);

// Guarantees the subscriber is only ever installed once, even if run() is
// somehow invoked more than once.  A second call to try_init() in release
// builds running as Administrator causes a silent crash; OnceLock prevents it.
static LOG_INIT: OnceLock<()> = OnceLock::new();

pub fn set_app_handle(handle: AppHandle) {
    if let Ok(mut guard) = APP_HANDLE.lock() {
        *guard = Some(handle);
    }
}

struct FrontendTracingLayer;

impl<S> Layer<S> for FrontendTracingLayer
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        struct StringVisitor(String);
        impl tracing::field::Visit for StringVisitor {
            fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                if field.name() == "message" {
                    self.0 = value.to_string();
                }
            }
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.0 = format!("{:?}", value);
                }
            }
        }

        let mut visitor = StringVisitor(String::new());
        event.record(&mut visitor);

        let metadata = event.metadata();
        let target = metadata.target();

        // Module path'e göre Türkçe kategori etiketi belirle
        let tag = if target.contains("engine") || target.contains("manager") || target.contains("process") {
            "MOTOR"
        } else if target.contains("adblock") {
            "ADBLOCK"
        } else if target.contains("dns") || target.contains("forwarder") || target.contains("resolver") {
            "DNS"
        } else if target.contains("sanitizer") || target.contains("privilege") {
            "GÜVENLİK"
        } else if target.contains("updater") || target.contains("update") {
            "GÜNCELLEME"
        } else if target.contains("autostart") || target.contains("watcher") || target.contains("network") {
            "SİSTEM"
        } else {
            match *metadata.level() {
                tracing::Level::ERROR => "HATA",
                tracing::Level::WARN  => "UYARI",
                _                     => "SİSTEM",
            }
        };

        if !visitor.0.is_empty() {
            let log_line = format!("[{}] {}", tag, visitor.0);
            if let Ok(guard) = APP_HANDLE.lock() {
                if let Some(handle) = &*guard {
                    let _ = handle.emit("log_batch", vec![log_line]);
                }
            }
        }
    }
}

/// Installs the global tracing subscriber exactly once.
///
/// This is called by `tauri_plugin_log`'s setup via the `setup` callback.
/// The `OnceLock` guard prevents a second call from panicking / crashing in
/// release builds where `set_global_default` returns an error instead of a
/// no-op.
pub fn init_logging() {
    LOG_INIT.get_or_init(|| {
        let _ = tracing_subscriber::registry()
            .with(FrontendTracingLayer)
            .with(tracing_subscriber::fmt::layer())
            .try_init();
    });
}
