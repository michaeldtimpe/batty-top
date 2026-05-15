use std::time::Duration;

use clap::Parser;

use crate::app::ui::Units;

fn parse_duration(raw: &str) -> Result<Duration, String> {
    match raw.parse::<u64>() {
        Ok(seconds) if seconds > 0 => Ok(Duration::from_secs(seconds)),
        _ => Err(format!("{} isn't a positive number", raw)),
    }
}

/// Interactive battery viewer.
///
/// Controls inside batty:
///   Right        next tab
///   Left         previous tab
///   Q, Ctrl+C, Esc   quit
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// Verbosity level, can be repeated up to 5 times (-vvvvv).
    /// Logs go to stderr.
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    verbose: u8,

    /// Delay between updates, in seconds.
    #[arg(short = 'd', long = "delay", default_value = "1", value_parser = parse_duration)]
    delay: Duration,

    /// Measurement units displayed (human or si).
    #[arg(short = 'u', long = "units", default_value = "human", ignore_case = true)]
    units: Units,
}

impl Config {
    pub fn verbosity(&self) -> u8 {
        self.verbose
    }

    pub fn delay(&self) -> &Duration {
        &self.delay
    }

    pub fn units(&self) -> Units {
        self.units
    }
}
