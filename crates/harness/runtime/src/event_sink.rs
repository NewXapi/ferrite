//! Event sink for [`AgentRunEvent`]s.

use chrono::Utc;
use serde_json::Value;
use tokio::sync::mpsc;

use harness_core::{AgentRunEvent, AgentRunEventLevel};

/// Receives run events.
pub trait EventSink: Send {
    fn emit(&mut self, event: AgentRunEvent);
}

/// Collects events in memory.
#[derive(Debug, Default)]
pub struct VecEventSink {
    pub events: Vec<AgentRunEvent>,
}

impl EventSink for VecEventSink {
    fn emit(&mut self, event: AgentRunEvent) {
        self.events.push(event);
    }
}

/// Forwards events over an unbounded channel.
#[derive(Debug, Clone)]
pub struct MpscEventSink {
    sender: mpsc::UnboundedSender<AgentRunEvent>,
}

impl MpscEventSink {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<AgentRunEvent>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (Self { sender }, receiver)
    }
}

impl EventSink for MpscEventSink {
    fn emit(&mut self, event: AgentRunEvent) {
        let _ = self.sender.send(event);
    }
}

/// Sequential event factory for one run.
#[derive(Debug)]
pub struct EventFactory {
    run_id: String,
    seq: u64,
}

impl EventFactory {
    pub fn new(run_id: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            seq: 0,
        }
    }

    pub fn next(
        &mut self,
        event_type: impl Into<String>,
        level: AgentRunEventLevel,
        payload: Value,
    ) -> AgentRunEvent {
        self.seq += 1;
        AgentRunEvent {
            seq: self.seq,
            id: format!("evt_{}", self.seq),
            run_id: self.run_id.clone(),
            timestamp: Utc::now(),
            level,
            event_type: event_type.into(),
            payload,
        }
    }
}
