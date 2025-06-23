// src/events.rs

use anyhow::Result;
use crossterm::event::{Event as CrosstermEvent, KeyEvent};
use futures::{FutureExt, StreamExt};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Lsp(LspEvent),
    Tick,
}

#[derive(Debug)]
pub enum LspEvent {
    Notification(String, serde_json::Value),
    Response(serde_json::Value),
    Error(String),
}

pub struct EventHandler {
    receiver: mpsc::UnboundedReceiver<Event>,
    _handles: Vec<tokio::task::JoinHandle<()>>,
}

impl EventHandler {
    pub fn new() -> Result<Self> {
        let (sender, receiver) = mpsc::unbounded_channel();

        // Keyboard event handler
        let keyboard_sender = sender.clone();
        let keyboard_handle = tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            loop {
                let delay = tokio::time::sleep(Duration::from_millis(16)).fuse();
                let event = reader.next().fuse();

                tokio::select! {
                    _ = delay => {
                        if keyboard_sender.send(Event::Tick).is_err() {
                            break;
                        }
                    }
                    maybe_event = event => {
                        if let Some(Ok(CrosstermEvent::Key(key))) = maybe_event {
                            if keyboard_sender.send(Event::Key(key)).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            receiver,
            _handles: vec![keyboard_handle],
        })
    }

    pub async fn next(&mut self) -> Result<Option<Event>> {
        Ok(self.receiver.recv().await)
    }
}
