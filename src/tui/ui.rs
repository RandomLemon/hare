use crate::tui::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &App) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    draw_metric_view(frame, app, main_layout[0]);
    draw_status_bar(frame, app, main_layout[1]);
}

fn draw_metric_view(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let mut lines: Vec<Line> = Vec::new();

    if app.snapshot.is_empty() {
        lines.push(Line::from("No metrics available."));
    } else {
        let label_width = app
            .snapshot
            .iter()
            .map(|e| e.label.chars().count())
            .max()
            .unwrap_or(0)
            .max("Metric".len());

        lines.push(
            Line::from(format!(
                "{:<width$}  Value",
                "Metric",
                width = label_width
            ))
            .style(Style::default().add_modifier(Modifier::BOLD)),
        );

        for entry in &app.snapshot {
            lines.push(Line::from(format!(
                "{:<width$}  {}",
                entry.label,
                entry.value.format(),
                width = label_width
            )));
        }
    }

    let paragraph = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(" Hardware Monitor ")
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
                " {} metrics | {} | h/j/k/l scroll ",
                app.snapshot.len(),
                refresh_text
            ),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    frame.render_widget(Paragraph::new(status), area);
}
