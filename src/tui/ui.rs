use crate::tui::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

const CPU_COLUMN_WIDTH: usize = 8;
const FREQ_COLUMN_WIDTH: usize = 15;

pub fn draw(frame: &mut Frame, app: &App) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    draw_frequency_view(frame, app, main_layout[0]);
    draw_status_bar(frame, app, main_layout[1]);
}

fn draw_frequency_view(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(
        Line::from(format!(
            "{:<width_cpu$} {:<width_freq$}",
            "CPU",
            "Frequency",
            width_cpu = CPU_COLUMN_WIDTH,
            width_freq = FREQ_COLUMN_WIDTH
        ))
        .style(Style::default().add_modifier(Modifier::BOLD)),
    );

    for (index, mhz) in app.frequencies.iter().enumerate() {
        let freq_text = if mhz.is_nan() {
            "NaN".to_string()
        } else {
            format!("{:.2} MHz", mhz)
        };

        lines.push(Line::from(format!(
            "{:<width_cpu$} {:<width_freq$}",
            index,
            freq_text,
            width_cpu = CPU_COLUMN_WIDTH,
            width_freq = FREQ_COLUMN_WIDTH
        )));
    }

    let paragraph = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(" CPU Frequency Monitor ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL),
        )
        // Paragraph::scroll takes (y, x), so we map our (x, y) offset accordingly.
        .scroll((app.scroll_offset.1, app.scroll_offset.0));

    frame.render_widget(paragraph, area);
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let refresh_text = app
        .last_refresh
        .map(|instant| format!("Updated: {:.1}s ago", instant.elapsed().as_secs_f64()))
        .unwrap_or_else(|| "Updated: never".to_string());

    let status = Line::from(vec![
        Span::styled(
            " Press 'q' or 'Esc' to quit ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
        Span::raw("  "),
        Span::styled(
            format!(
                " {} cores | {} | ↑/↓/j/k scroll ",
                app.frequencies.len(),
                refresh_text
            ),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    frame.render_widget(Paragraph::new(status), area);
}
