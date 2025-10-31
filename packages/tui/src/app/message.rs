//! Message-passing pattern for application state updates
//!
//! This module provides types and utilities for implementing the Elm Architecture
//! pattern in TUI applications.

use std::sync::mpsc::{channel, Receiver, Sender};

/// Message channel for sending updates from widgets to the application
pub struct MessageChannel<M> {
    sender: Sender<M>,
    receiver: Receiver<M>,
}

impl<M> MessageChannel<M> {
    /// Create a new message channel
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self { sender, receiver }
    }

    /// Get a sender for this channel
    pub fn sender(&self) -> Sender<M> {
        self.sender.clone()
    }

    /// Try to receive a message (non-blocking)
    pub fn try_recv(&self) -> Option<M> {
        self.receiver.try_recv().ok()
    }

    /// Receive all pending messages
    pub fn recv_all(&self) -> Vec<M> {
        let mut messages = Vec::new();
        while let Ok(msg) = self.receiver.try_recv() {
            messages.push(msg);
        }
        messages
    }
}

impl<M> Default for MessageChannel<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for types that can send messages
pub trait MessageSender<M> {
    /// Send a message
    fn send(&self, message: M);
}

impl<M> MessageSender<M> for Sender<M> {
    fn send(&self, message: M) {
        let _ = Sender::send(self, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    enum TestMessage {
        Increment,
        Decrement,
        SetValue(i32),
    }

    #[test]
    fn test_message_channel() {
        let channel = MessageChannel::new();
        let sender = channel.sender();

        sender.send(TestMessage::Increment).unwrap();
        sender.send(TestMessage::Decrement).unwrap();
        sender.send(TestMessage::SetValue(42)).unwrap();

        let messages = channel.recv_all();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0], TestMessage::Increment);
        assert_eq!(messages[1], TestMessage::Decrement);
        assert_eq!(messages[2], TestMessage::SetValue(42));
    }

    #[test]
    fn test_try_recv_empty() {
        let channel: MessageChannel<TestMessage> = MessageChannel::new();
        assert!(channel.try_recv().is_none());
    }

    #[test]
    fn test_recv_all_empty() {
        let channel: MessageChannel<TestMessage> = MessageChannel::new();
        let messages = channel.recv_all();
        assert!(messages.is_empty());
    }
}
