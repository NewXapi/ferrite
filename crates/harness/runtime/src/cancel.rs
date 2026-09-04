//! Cancellation token built on `tokio::sync::watch`.
//!
//! Cloning is cheap: each clone shares the same cancellation state.

use std::sync::{Arc, Mutex};

use tokio::sync::watch;

/// Reason describing why a run was cancelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancelReason {
    /// Cancellation requested by the caller.
    UserRequested,
    /// Provider reported an error and the loop cancelled remaining work.
    ProviderError(String),
    /// A tool result reported an error and the loop cancelled remaining work.
    ToolError(String),
}

impl CancelReason {
    fn describe(&self) -> String {
        match self {
            Self::UserRequested => "user requested".to_string(),
            Self::ProviderError(message) => format!("provider error: {message}"),
            Self::ToolError(message) => format!("tool error: {message}"),
        }
    }
}

/// Cloneable cancellation token.
#[derive(Clone)]
pub struct CancellationToken {
    inner: Arc<watch::Sender<bool>>,
    state: watch::Receiver<bool>,
    reason: Arc<Mutex<Option<CancelReason>>>,
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    /// Create a fresh, non-cancelled token.
    pub fn new() -> Self {
        let (sender, state) = watch::channel(false);
        Self {
            inner: Arc::new(sender),
            state,
            reason: Arc::new(Mutex::new(None)),
        }
    }

    /// Signal cancellation to every clone.
    pub fn cancel(&self, reason: CancelReason) {
        if let Ok(mut guard) = self.reason.lock() {
            *guard = Some(reason);
        }
        let _ = self.inner.send(true);
    }

    /// `true` once `cancel` has been called at least once.
    pub fn is_cancelled(&self) -> bool {
        *self.state.borrow()
    }

    /// Human-readable reason, if cancelled.
    pub fn reason(&self) -> Option<String> {
        self.reason
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(CancelReason::describe))
    }

    /// Future resolving the next time cancellation is observed.
    pub async fn cancelled(&self) {
        let mut receiver = self.state.clone();
        if *receiver.borrow_and_update() {
            return;
        }
        let _ = receiver.changed().await;
    }
}

impl std::fmt::Debug for CancellationToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CancellationToken")
            .field("cancelled", &self.is_cancelled())
            .field("reason", &self.reason())
            .finish()
    }
}
