use std::sync::Arc;

use clap::Parser;
use log::{error, trace};

mod app;
mod errors;

pub use self::errors::{Error, Result};

fn main() -> Result<()> {
    let config = Arc::new(app::config::Config::parse());

    let log_level = match config.verbosity() {
        0 => log::LevelFilter::Error,
        1 => log::LevelFilter::Warn,
        2 => log::LevelFilter::Info,
        3 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    env_logger::Builder::new().filter_level(log_level).try_init()?;

    trace!("Starting with {:?}", &config);
    let mut app = app::init(config)?;

    match app.run() {
        Err(Error::UserExit) => {
            trace!("Exit was requested by user, terminating");
            Ok(())
        }
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Error occurred: {:?}", e);
            Err(e)
        }
    }
}
