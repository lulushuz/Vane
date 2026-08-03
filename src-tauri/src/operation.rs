#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExclusiveOperation {
    Optimizer(String),
    TrafficProbe(String),
    PatternTransaction(String),
}

#[derive(Default)]
pub struct ExclusiveOperationCoordinator {
    active: std::sync::Mutex<Option<ExclusiveOperation>>,
}

impl ExclusiveOperationCoordinator {
    pub fn try_acquire(
        self: &std::sync::Arc<Self>,
        operation: ExclusiveOperation,
    ) -> Result<ExclusiveOperationGuard, ExclusiveOperation> {
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(owner) = active.clone() {
            return Err(owner);
        }
        *active = Some(operation.clone());
        Ok(ExclusiveOperationGuard {
            coordinator: self.clone(),
            operation,
        })
    }

    pub fn active(&self) -> Option<ExclusiveOperation> {
        self.active
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
}

pub struct ExclusiveOperationGuard {
    coordinator: std::sync::Arc<ExclusiveOperationCoordinator>,
    operation: ExclusiveOperation,
}

impl Drop for ExclusiveOperationGuard {
    fn drop(&mut self) {
        let mut active = self
            .coordinator
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if active.as_ref() == Some(&self.operation) {
            *active = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conflicts_are_rejected_until_owner_drops() {
        let coordinator = std::sync::Arc::new(ExclusiveOperationCoordinator::default());
        let guard = coordinator
            .try_acquire(ExclusiveOperation::Optimizer("a".into()))
            .unwrap();
        assert!(coordinator
            .try_acquire(ExclusiveOperation::TrafficProbe("b".into()))
            .is_err());
        drop(guard);
        assert!(coordinator
            .try_acquire(ExclusiveOperation::TrafficProbe("b".into()))
            .is_ok());
    }
}
