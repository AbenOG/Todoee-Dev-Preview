use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Terminal events
#[derive(Clone, Debug)]
#[allow(dead_code)] // Mouse and Resize variants are part of the API
pub enum Event {
    /// Terminal tick (for animations/updates)
    Tick,
    /// Key press
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Terminal resize
    Resize(u16, u16),
}

/// Handles terminal events
pub struct EventHandler {
    /// Event receiver
    receiver: mpsc::Receiver<Event>,
    /// Event handler thread
    _handler: thread::JoinHandle<()>,
}

impl EventHandler {
    /// Create a new event handler with the given tick rate
    pub fn new(tick_rate: u64) -> Self {
        let tick_rate = Duration::from_millis(tick_rate);
        let (sender, receiver) = mpsc::channel();
        let handler_sender = sender.clone();

        let handler = thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or(tick_rate);

                match event::poll(timeout) {
                    Ok(true) => {
                        match event::read() {
                            Ok(event) => match event {
                                CrosstermEvent::Key(e) => {
                                    if handler_sender.send(Event::Key(e)).is_err() {
                                        break;
                                    }
                                }
                                CrosstermEvent::Mouse(e) => {
                                    if handler_sender.send(Event::Mouse(e)).is_err() {
                                        break;
                                    }
                                }
                                CrosstermEvent::Resize(w, h) => {
                                    if handler_sender.send(Event::Resize(w, h)).is_err() {
                                        break;
                                    }
                                }
                                _ => {}
                            },
                            Err(_) => continue,
                        }
                    }
                    Ok(false) => {}
                    Err(_) => continue,
                }

                if last_tick.elapsed() >= tick_rate {
                    if handler_sender.send(Event::Tick).is_err() {
                        break;
                    }
                    last_tick = Instant::now();
                }
            }
        });

        Self {
            receiver,
            _handler: handler,
        }
    }

    /// Receive the next event
    pub fn next(&self) -> Result<Event> {
        Ok(self.receiver.recv()?)
    }
}
