use std::rc::Rc;
use std::time::Duration;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Block, Borders, Cell, Chart, Dataset, Gauge, Paragraph, Row, Table, Tabs};

use starship_battery::State;
use starship_battery::units::Unit;
use starship_battery::units::electric_potential::volt;
use starship_battery::units::energy::{joule, watt_hour};
use starship_battery::units::power::watt;
use starship_battery::units::ratio::{percent, ratio};
use starship_battery::units::thermodynamic_temperature::{degree_celsius, kelvin};
use starship_battery::units::time::second;

use super::{ChartData, TabBar, Units, View};

#[cfg(target_os = "macos")]
use crate::app::Extras;

#[derive(Debug)]
pub struct Context<'i> {
    pub tabs: &'i TabBar,
    pub view: &'i View,
    #[cfg(target_os = "macos")]
    pub extras: Option<&'i Extras>,
}

#[derive(Debug)]
pub struct Painter<'i>(Rc<Context<'i>>);

impl<'i> Painter<'i> {
    pub fn from_context(context: Rc<Context<'i>>) -> Painter<'i> {
        Painter(context)
    }

    pub fn draw(&self, frame: &mut Frame<'_>) {
        let main = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(10)])
            .split(frame.area());

        let main_columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(40), Constraint::Min(20)])
            .split(main[1]);

        let left_column = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // SoC gauge
                Constraint::Length(12), // Information (expanded for extras)
                Constraint::Length(9),  // Energy
                Constraint::Length(5),  // Timings
                Constraint::Min(10),    // Environment + lifetime stats
            ])
            .split(main_columns[0]);

        let right_column = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
            .split(main_columns[1]);

        self.draw_tabs(frame, main[0]);
        self.draw_state_of_charge_bar(frame, left_column[0]);
        self.draw_common_info(frame, left_column[1]);
        self.draw_energy_info(frame, left_column[2]);
        self.draw_timing_info(frame, left_column[3]);
        self.draw_environment_info(frame, left_column[4]);
        self.draw_chart(self.0.view.voltage(), frame, right_column[0]);
        self.draw_chart(self.0.view.energy_rate(), frame, right_column[1]);
        self.draw_chart(self.0.view.temperature(), frame, right_column[2]);
    }

    fn draw_tabs(&self, frame: &mut Frame<'_>, area: Rect) {
        let titles: Vec<Line> = self.0.tabs.titles().iter().map(|t| Line::from(t.as_str())).collect();

        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title(" Batteries "))
            .select(self.0.tabs.index())
            .style(Style::default().fg(Color::Cyan))
            .highlight_style(Style::default().fg(Color::White));
        frame.render_widget(tabs, area);
    }

    fn draw_state_of_charge_bar(&self, frame: &mut Frame<'_>, area: Rect) {
        let value = f64::from(self.0.view.battery().state_of_charge().get::<ratio>());
        let value_label = f64::from(self.0.view.battery().state_of_charge().get::<percent>());

        let gauge_block = Block::default()
            .title(" State of charge ")
            .borders(Borders::ALL & !Borders::RIGHT);
        let text_block = Block::default().borders(Borders::ALL & !Borders::LEFT);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length("|100.00 %|".len() as u16)])
            .split(area);

        let gauge_color = if value > 0.3 {
            Color::Green
        } else if value > 0.15 {
            Color::Yellow
        } else {
            Color::Red
        };
        let text_color = if gauge_color == Color::Green { Color::Gray } else { gauge_color };

        let gauge = Gauge::default()
            .block(gauge_block)
            .ratio(value.clamp(0.0, 1.0))
            .gauge_style(Style::default().bg(Color::Black).fg(gauge_color))
            .label("");

        let text_line = Line::from(vec![
            Span::raw(" "),
            Span::styled(format!("{:>6.2} %", value_label), Style::default().fg(text_color)),
        ]);
        let paragraph = Paragraph::new(text_line).block(text_block).alignment(Alignment::Right);

        frame.render_widget(gauge, chunks[0]);
        frame.render_widget(paragraph, chunks[1]);
    }

    fn draw_chart(&self, data: &ChartData, frame: &mut Frame<'_>, area: Rect) {
        let title = format!(" {} ", data.title());
        let block = Block::default().title(title).borders(Borders::ALL);
        let value = data.current();
        let x_axis = Axis::default()
            .title(value)
            .style(Style::default().fg(Color::Reset))
            .bounds(data.x_bounds());
        let y_labels: Vec<Span> = data.y_labels().into_iter().map(Span::from).collect();
        let y_axis = Axis::default()
            .title(data.y_title().to_string())
            .labels(y_labels)
            .bounds(data.y_bounds());

        let datasets = vec![
            Dataset::default()
                .marker(Marker::Braille)
                .style(Style::default().fg(Color::Green))
                .data(data.points()),
        ];

        let chart = Chart::new(datasets).block(block).x_axis(x_axis).y_axis(y_axis);
        frame.render_widget(chart, area);
    }

    fn draw_common_info(&self, frame: &mut Frame<'_>, area: Rect) {
        let block = Block::default()
            .title(" Information ")
            .borders(Borders::LEFT | Borders::TOP | Borders::RIGHT);

        let battery = self.0.view.battery();
        let tech_string;
        let tech: &str = {
            #[cfg(target_os = "macos")]
            {
                if let Some(extras) = self.0.extras {
                    extras.technology
                } else {
                    tech_string = format!("{}", battery.technology());
                    tech_string.as_str()
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                tech_string = format!("{}", battery.technology());
                tech_string.as_str()
            }
        };
        let state = format!("{}", battery.state());
        let cycles = match battery.cycle_count() {
            Some(cycles) => format!("{}", cycles),
            None => "N/A".to_string(),
        };

        #[cfg(target_os = "macos")]
        let vendor_display: String = match (
            battery.vendor(),
            self.0.extras.and_then(|e| e.vendor),
            self.0.extras.and_then(|e| e.pack_lot_code.as_deref()),
        ) {
            (Some(v), _, _) => v.to_string(),       // starship-battery (Intel path)
            (_, Some(v), _) => v.to_string(),       // defensive supplier-code lookup
            (_, _, Some(lot)) => format!("lot {}", lot), // honest pack-lot fallback
            _ => "N/A".to_string(),
        };
        #[cfg(not(target_os = "macos"))]
        let vendor_display: String = battery.vendor().unwrap_or("N/A").to_string();
        let vendor: &str = vendor_display.as_str();

        #[cfg(target_os = "macos")]
        let firmware = self
            .0
            .extras
            .and_then(|e| e.firmware_version.as_deref())
            .unwrap_or("N/A");
        #[cfg(target_os = "macos")]
        let cell_rev = self.0.extras.and_then(|e| e.cell_revision.as_deref()).unwrap_or("N/A");

        let mut items: Vec<[&str; 2]> = vec![
            ["Vendor", vendor],
            ["Model", battery.model().unwrap_or("N/A")],
            ["S/N", battery.serial_number().unwrap_or("N/A")],
            ["Technology", tech],
        ];
        #[cfg(target_os = "macos")]
        {
            items.push(["Firmware", firmware]);
            items.push(["Cell rev", cell_rev]);
        }
        items.push(["Charge state", &state]);
        items.push(["Cycles count", &cycles]);

        self.draw_info_table(["Device", ""], &items, block, frame, area);
    }

    fn draw_energy_info(&self, frame: &mut Frame<'_>, area: Rect) {
        let block = Block::default().borders(Borders::LEFT | Borders::RIGHT);
        let battery = self.0.view.battery();
        let config = self.0.view.config();

        let consumption = format!("{:.2} {}", battery.energy_rate().get::<watt>(), watt::abbreviation());
        let voltage = format!("{:.2} {}", battery.voltage().get::<volt>(), volt::abbreviation());
        let capacity = format!(
            "{:.2} {}",
            battery.state_of_health().get::<percent>(),
            percent::abbreviation()
        );
        let current = match config.units() {
            Units::Human => format!(
                "{:.2} {}",
                battery.energy().get::<watt_hour>(),
                watt_hour::abbreviation()
            ),
            Units::Si => format!("{:.2} {}", battery.energy().get::<joule>(), joule::abbreviation()),
        };
        let last_full = match config.units() {
            Units::Human => format!(
                "{:.2} {}",
                battery.energy_full().get::<watt_hour>(),
                watt_hour::abbreviation()
            ),
            Units::Si => format!(
                "{:.2} {}",
                battery.energy_full().get::<joule>(),
                joule::abbreviation()
            ),
        };
        let full_design = match config.units() {
            Units::Human => format!(
                "{:.2} {}",
                battery.energy_full_design().get::<watt_hour>(),
                watt_hour::abbreviation()
            ),
            Units::Si => format!(
                "{:.2} {}",
                battery.energy_full_design().get::<joule>(),
                joule::abbreviation()
            ),
        };
        let consumption_label = match battery.state() {
            State::Charging => "Charging with",
            State::Discharging => "Discharging with",
            _ => "Consumption",
        };

        let items = vec![
            [consumption_label, consumption.as_str()],
            ["Voltage", voltage.as_str()],
            ["Capacity", capacity.as_str()],
            ["Current", current.as_str()],
            ["Last full", last_full.as_str()],
            ["Full design", full_design.as_str()],
        ];
        self.draw_info_table(["Energy", ""], &items, block, frame, area);
    }

    fn draw_timing_info(&self, frame: &mut Frame<'_>, area: Rect) {
        let block = Block::default().borders(Borders::LEFT | Borders::RIGHT);
        let battery = self.0.view.battery();

        let time_to_full = match battery.time_to_full() {
            Some(time) => humantime::format_duration(Duration::from_secs(time.get::<second>() as u64)).to_string(),
            None => "N/A".to_string(),
        };
        let time_to_empty = match battery.time_to_empty() {
            Some(time) => humantime::format_duration(Duration::from_secs(time.get::<second>() as u64)).to_string(),
            None => "N/A".to_string(),
        };

        let items = vec![
            ["Time to full", time_to_full.as_str()],
            ["Time to empty", time_to_empty.as_str()],
        ];
        self.draw_info_table(["Time", ""], &items, block, frame, area);
    }

    fn draw_environment_info(&self, frame: &mut Frame<'_>, area: Rect) {
        let block = Block::default().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM);
        let battery = self.0.view.battery();
        let config = self.0.view.config();

        let temperature = match battery.temperature() {
            Some(temp) => match config.units() {
                Units::Human => format!("{:.2} {}", temp.get::<degree_celsius>(), degree_celsius::abbreviation()),
                Units::Si => format!("{:.2} {}", temp.get::<kelvin>(), kelvin::abbreviation()),
            },
            None => "N/A".to_string(),
        };

        let mut items: Vec<[&str; 2]> = vec![["Temperature", temperature.as_str()]];

        #[cfg(target_os = "macos")]
        let (operating, first_use_str, health, max_temp, peak_charge, peak_voltage, disconnects);
        #[cfg(target_os = "macos")]
        {
            if let Some(extras) = self.0.extras {
                operating = match extras.total_operating_time_hours {
                    Some(h) => format!("{} h", h),
                    None => "N/A".to_string(),
                };
                first_use_str = match extras.first_use {
                    Some(t) => format_system_time(t),
                    None => "N/A".to_string(),
                };
                health = extras
                    .battery_health_metric
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "N/A".to_string());
                max_temp = extras
                    .lifetime_max_temperature_c
                    .map(|v| format!("{:.1} °C", v))
                    .unwrap_or_else(|| "N/A".to_string());
                peak_charge = extras
                    .lifetime_max_charge_current_ma
                    .map(|v| format!("{} mA", v))
                    .unwrap_or_else(|| "N/A".to_string());
                peak_voltage = extras
                    .lifetime_max_pack_voltage_mv
                    .map(|v| format!("{} mV", v))
                    .unwrap_or_else(|| "N/A".to_string());
                disconnects = extras
                    .system_disconnect_count
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "N/A".to_string());
                items.push(["Operating", operating.as_str()]);
                items.push(["First use", first_use_str.as_str()]);
                items.push(["Health score", health.as_str()]);
                items.push(["Max temp ever", max_temp.as_str()]);
                items.push(["Peak charge", peak_charge.as_str()]);
                items.push(["Peak voltage", peak_voltage.as_str()]);
                items.push(["Disconnects", disconnects.as_str()]);
            }
        }

        self.draw_info_table(["Environment", ""], &items, block, frame, area);
    }

    fn draw_info_table(
        &self,
        header: [&str; 2],
        items: &[[&str; 2]],
        block: Block<'_>,
        frame: &mut Frame<'_>,
        area: Rect,
    ) {
        let header_row = Row::new(vec![Cell::from(header[0]), Cell::from(header[1])])
            .style(Style::default().add_modifier(Modifier::BOLD));

        let rows: Vec<Row> = items
            .iter()
            .map(|item| Row::new(vec![Cell::from(item[0]), Cell::from(item[1])]))
            .collect();

        let widths = [Constraint::Min(14), Constraint::Min(18)];
        let table = Table::new(rows, widths).header(header_row).block(block);
        frame.render_widget(table, area);
    }
}

#[cfg(target_os = "macos")]
fn format_system_time(t: std::time::SystemTime) -> String {
    use std::time::UNIX_EPOCH;
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => {
            let secs = d.as_secs() as i64;
            let (year, month, day) = ymd_from_unix(secs);
            format!("{}-{:02}-{:02}", year, month, day)
        }
        Err(_) => "N/A".to_string(),
    }
}

/// Civil-date conversion (Howard Hinnant's algorithm, public domain).
#[cfg(target_os = "macos")]
fn ymd_from_unix(unix_seconds: i64) -> (i32, u32, u32) {
    let days = unix_seconds.div_euclid(86_400);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i32 + era as i32 * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
