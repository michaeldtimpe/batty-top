use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use log::trace;

use crate::Result;
use crate::app::Config;

#[derive(Debug, Eq, PartialEq)]
pub enum Event {
    Exit,
    NextTab,
    PreviousTab,
    Tick,
}

#[derive(Debug)]
pub struct EventHandler {
    tick: Duration,
    last_tick: Instant,
    interrupted: Arc<AtomicBool>,
}

impl EventHandler {
    pub fn from_config(config: &Config) -> EventHandler {
        let interrupted = Arc::new(AtomicBool::new(false));
        let flag = interrupted.clone();
        // Best-effort SIGINT handler — registering twice in the same process panics,
        // so swallow the error in case something else already set one.
        let _ = ctrlc::set_handler(move || {
            flag.store(true, Ordering::SeqCst);
        });

        EventHandler {
            tick: *config.delay(),
            last_tick: Instant::now(),
            interrupted,
        }
    }

    pub fn next(&mut self) -> Result<Event> {
        loop {
            if self.interrupted.load(Ordering::SeqCst) {
                trace!("SIGINT received, exiting");
                return Ok(Event::Exit);
            }

            let elapsed = self.last_tick.elapsed();
            if elapsed >= self.tick {
                self.last_tick = Instant::now();
                return Ok(Event::Tick);
            }
            let remaining = self.tick - elapsed;

            if event::poll(remaining)? {
                match event::read()? {
                    CtEvent::Key(KeyEvent { code, modifiers, kind, .. })
                        if kind == KeyEventKind::Press || kind == KeyEventKind::Repeat =>
                    {
                        match (code, modifiers) {
                            (KeyCode::Left, _) => return Ok(Event::PreviousTab),
                            (KeyCode::Right, _) => return Ok(Event::NextTab),
                            (KeyCode::Char('q'), _) => return Ok(Event::Exit),
                            (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => {
                                return Ok(Event::Exit);
                            }
                            (KeyCode::Esc, _) => return Ok(Event::Exit),
                            _ => continue,
                        }
                    }
                    _ => continue,
                }
            }
            // poll() returned false; loop continues and the elapsed check above will fire Tick.
        }
    }
}
