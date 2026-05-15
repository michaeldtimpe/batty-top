use std::io::{self, Stdout};
use std::rc::Rc;
use std::sync::Arc;

use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use super::{Context, Painter, TabBar, View};
use crate::Result;
use crate::app::Config;

#[cfg(target_os = "macos")]
use crate::app::Extras;

pub fn init(
    config: Arc<Config>,
    views: Vec<View>,
    #[cfg(target_os = "macos")] extras: Option<Extras>,
) -> Result<Interface> {
    debug_assert!(!views.is_empty());

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    install_panic_hook();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let tab_titles = views.iter().map(|view| view.title()).collect::<Vec<_>>();
    let tabs = TabBar::new(tab_titles);

    Ok(Interface {
        config,
        terminal,
        views,
        tabs,
        #[cfg(target_os = "macos")]
        extras,
    })
}

fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default(info);
    }));
}

pub struct Interface {
    config: Arc<Config>,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    views: Vec<View>,
    tabs: TabBar,
    #[cfg(target_os = "macos")]
    extras: Option<Extras>,
}

impl Interface {
    pub fn draw(&mut self) -> Result<()> {
        let context = Rc::new(Context {
            tabs: &self.tabs,
            view: &self.views[self.tabs.index()],
            #[cfg(target_os = "macos")]
            extras: self.extras.as_ref(),
        });
        self.terminal.draw(|frame| {
            Painter::from_context(context.clone()).draw(frame);
        })?;
        Ok(())
    }

    pub fn views_mut(&mut self) -> &mut [View] {
        self.views.as_mut()
    }

    pub fn tabs_mut(&mut self) -> &mut TabBar {
        &mut self.tabs
    }
}

impl std::fmt::Debug for Interface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interface").field("config", &self.config).finish()
    }
}

impl Drop for Interface {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}
