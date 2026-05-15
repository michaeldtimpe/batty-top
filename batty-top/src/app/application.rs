use std::sync::Arc;

use log::{error, trace};
use starship_battery as battery;

use super::config::Config;
use super::events::{Event, EventHandler};
use super::ui;
use crate::{Error, Result};

#[cfg(target_os = "macos")]
use super::Extras;

pub fn init(config: Arc<Config>) -> Result<Application> {
    let manager = battery::Manager::new()?;

    let batteries = manager
        .batteries()?
        .flatten()
        .map(|battery| ui::View::new(config.clone(), battery))
        .collect::<Vec<_>>();

    if batteries.is_empty() {
        error!("Unable to find any batteries in system, exiting");
        return Err(Error::NoBatteries);
    }
    trace!("Found {} batteries during initialization", batteries.len());

    #[cfg(target_os = "macos")]
    let extras: Option<Extras> = match batty_mac_extras::read() {
        Ok(e) => Some(e),
        Err(e) => {
            log::warn!("Failed to read macOS battery extras: {}", e);
            None
        }
    };

    let events = EventHandler::from_config(&config);
    let interface = ui::init(
        config.clone(),
        batteries,
        #[cfg(target_os = "macos")]
        extras,
    )?;

    Ok(Application { manager, config, events, interface })
}

pub struct Application {
    manager: battery::Manager,
    #[allow(dead_code)]
    config: Arc<Config>,
    events: EventHandler,
    interface: ui::Interface,
}

impl Application {
    pub fn run(&mut self) -> Result<()> {
        loop {
            self.interface.draw()?;
            self.handle_event()?;
        }
    }

    fn handle_event(&mut self) -> Result<()> {
        match self.events.next()? {
            Event::Exit => Err(Error::UserExit),
            Event::PreviousTab => {
                self.interface.tabs_mut().previous();
                Ok(())
            }
            Event::NextTab => {
                self.interface.tabs_mut().next();
                Ok(())
            }
            Event::Tick => {
                for view in self.interface.views_mut() {
                    view.update(&mut self.manager)?;
                }
                Ok(())
            }
        }
    }
}

impl std::fmt::Debug for Application {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Application").field("config", &self.config).finish()
    }
}
