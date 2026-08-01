use crate::diagnostics::event::DiagnosticEvent;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

const DEFAULT_CAPACITY: usize = 2000;

#[derive(Clone)]
pub struct DiagnosticEventStore {
    capacity: usize,
    buffer: Arc<Mutex<VecDeque<DiagnosticEvent>>>,
    dropped_count: Arc<AtomicU64>,
}

impl Default for DiagnosticEventStore {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

impl DiagnosticEventStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            dropped_count: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn push(&self, event: DiagnosticEvent) {
        let mut guard = self.buffer.lock().await;
        if guard.len() >= self.capacity {
            guard.pop_front();
            self.dropped_count.fetch_add(1, Ordering::Relaxed);
        }
        guard.push_back(event);
    }

    pub async fn get_events(&self, limit: Option<usize>) -> Vec<DiagnosticEvent> {
        let guard = self.buffer.lock().await;
        let limit = limit.unwrap_or(self.capacity);
        let start = guard.len().saturating_sub(limit);
        guard.iter().skip(start).cloned().collect()
    }

    pub async fn clear(&self) {
        let mut guard = self.buffer.lock().await;
        guard.clear();
        self.dropped_count.store(0, Ordering::Relaxed);
    }

    pub fn dropped_count(&self) -> u64 {
        self.dropped_count.load(Ordering::Relaxed)
    }
}
