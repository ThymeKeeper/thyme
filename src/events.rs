// src/events.rs

use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind, MouseEvent, KeyCode};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Tick,
}

pub struct EventHandler {
    receiver: Receiver<Event>,
    _thread_handle: thread::JoinHandle<()>,
}

impl EventHandler {
    pub fn new() -> Result<Self> {
        let (sender, receiver) = mpsc::channel();
        
        let thread_handle = thread::spawn(move || {
            let mut last_tick = Instant::now();
            let tick_rate = Duration::from_millis(100);
            let mut last_key_time = Instant::now();
            let key_repeat_delay = Duration::from_millis(30); // Minimum time between key repeats
            
            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or_else(|| Duration::from_secs(0));

                if event::poll(timeout).unwrap_or(false) {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key)) => {
                            // Skip release events for keys that shouldn't trigger actions on release
                            if key.kind == KeyEventKind::Release {
                                match key.code {
                                    // Skip release events for character input
                                    KeyCode::Char(_) => continue,
                                    
                                    // Skip release events for navigation keys
                                    KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right |
                                    KeyCode::PageUp | KeyCode::PageDown | 
                                    KeyCode::Home | KeyCode::End => continue,
                                    
                                    // Skip release events for editing keys
                                    KeyCode::Enter | KeyCode::Tab | KeyCode::BackTab |
                                    KeyCode::Backspace | KeyCode::Delete => continue,
                                    
                                    // Skip release events for function keys (they usually trigger on press)
                                    KeyCode::F(_) => continue,
                                    
                                    // Skip release for Escape
                                    KeyCode::Esc => continue,
                                    
                                    // Allow release events for modifier keys and any other special keys
                                    _ => {}
                                }
                            }
                            
                            // For Press and Repeat events, apply rate limiting
                            if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                                let now = Instant::now();
                                
                                // Rate limit key repeat events
                                if now.duration_since(last_key_time) >= key_repeat_delay {
                                    if sender.send(Event::Key(key)).is_err() {
                                        break;
                                    }
                                    last_key_time = now;
                                }
                            } else {
                                // Send release events without rate limiting (only for allowed keys)
                                if sender.send(Event::Key(key)).is_err() {
                                    break;
                                }
                            }
                        }
                        Ok(CrosstermEvent::Mouse(mouse)) => {
                            if sender.send(Event::Mouse(mouse)).is_err() {
                                break;
                            }
                        }
                        Ok(_) => {}
                        Err(_) => {}
                    }
                }

                if last_tick.elapsed() >= tick_rate {
                    if sender.send(Event::Tick).is_err() {
                        break;
                    }
                    last_tick = Instant::now();
                }
            }
        });

        Ok(Self {
            receiver,
            _thread_handle: thread_handle,
        })
    }

    pub async fn next(&self) -> Result<Option<Event>> {
        // For key events, we want to handle the queue differently
        // We'll collect all pending events and process them intelligently
        let mut events = Vec::new();
        
        // Collect all pending events
        loop {
            match self.receiver.try_recv() {
                Ok(event) => events.push(event),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return Ok(None),
            }
        }
        
        // If we have events, process them
        if !events.is_empty() {
            // Separate different types of events
            let mut last_movement_key = None;
            let mut has_tick = false;
            
            for event in events {
                match event {
                    Event::Key(key) => {
                        // Only coalesce movement keys that are Press or Repeat events
                        if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                            match key.code {
                                KeyCode::Up |
                                KeyCode::Down |
                                KeyCode::Left |
                                KeyCode::Right |
                                KeyCode::PageUp |
                                KeyCode::PageDown |
                                KeyCode::Home |
                                KeyCode::End => {
                                    last_movement_key = Some(Event::Key(key));
                                }
                                _ => {
                                    // For other keys, process immediately
                                    return Ok(Some(Event::Key(key)));
                                }
                            }
                        } else {
                            // Process release events immediately (these will only be for special keys like modifiers)
                            return Ok(Some(Event::Key(key)));
                        }
                    }
                    Event::Mouse(_) => {
                        // Return mouse events immediately
                        return Ok(Some(event));
                    }
                    Event::Tick => {
                        has_tick = true;
                    }
                }
            }
            
            // Return the last movement key event if we have one
            if let Some(event) = last_movement_key {
                return Ok(Some(event));
            }
            
            // Return tick if that's all we have
            if has_tick {
                return Ok(Some(Event::Tick));
            }
        }
        
        // No pending events, wait for the next one
        match self.receiver.recv() {
            Ok(event) => Ok(Some(event)),
            Err(_) => Ok(None),
        }
    }
}
