use crate::tui::app::{App, Page};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

/// App title shown on the left of the title bar.
const APP_TITLE: &str = " Hardware Monitor ";

/// Compute the three-region layout (title bar / body / status bar) for the
/// given screen area. Kept as a pure function so the event loop can reuse the
/// exact same geometry for hit-testing.
pub fn layout(screen: Rect) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(screen);
    (chunks[0], chunks[1], chunks[2])
}

/// Build the tab label line (with the active page highlighted) and return it
/// together with each tab's hit-rectangle, computed relative to `tabs_area`.
fn tabs(tabs_area: Rect, active: Page) -> (Line<'static>, Vec<(Page, Rect)>) {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut rects: Vec<(Page, Rect)> = Vec::new();
    let mut x = tabs_area.x;

    for (i, page) in Page::ALL.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
            x += 2;
        }

        let label = format!(" {}:{}", page.digit(), page.name());
        let width = line_width(&label) as u16;

        let style = if *page == active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(label, style));
        rects.push((
            *page,
            Rect {
                x,
                y: tabs_area.y,
                width,
                height: 1,
            },
        ));
        x = x.saturating_add(width);
    }

    (Line::from(spans), rects)
}

/// Display width of a string (CJK characters count as 2 columns), matching
/// ratatui's own width accounting.
fn line_width(s: &str) -> usize {
    Line::from(s.to_string()).width()
}

/// Per-tab hit-rectangles for the given screen. Used by the event loop for
/// mouse click handling.
pub fn tab_rects(screen: Rect, active: Page) -> Vec<(Page, Rect)> {
    let (header, _body, _status) = layout(screen);
    let tabs_area = tabs_area(header);
    let (_line, rects) = tabs(tabs_area, active);
    rects
}

fn tabs_area(header: Rect) -> Rect {
    let total: usize = Page::ALL
        .iter()
        .map(|p| line_width(&format!(" {}:{}", p.digit(), p.name())))
        .sum::<usize>()
        + 2 * (Page::ALL.len() - 1);
    let total = total as u16;
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(total)])
        .split(header)[1]
}

pub fn draw(frame: &mut Frame, app: &App) {
    let screen = frame.area();
    let (header, body, status) = layout(screen);

    draw_title_bar(frame, app, header);
    draw_body(frame, app, body);
    draw_status_bar(frame, app, status);
}

fn draw_title_bar(frame: &mut Frame, app: &App, area: Rect) {
    let tabs_rect = tabs_area(area);
    let (tabs_line, _) = tabs(tabs_rect, app.current_page);

    // Left: app title (dark background for a "title bar" look).
    let title = Span::styled(
        APP_TITLE,
        Style::default().fg(Color::White).bg(Color::DarkGray),
    );

    let left = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(tabs_rect.width)])
        .split(area)[0];

    let left_line = Line::from(vec![title, Span::raw(" ")]);
    frame.render_widget(
        Paragraph::new(left_line).style(Style::default().bg(Color::DarkGray)),
        left,
    );
    frame.render_widget(
        Paragraph::new(tabs_line).alignment(Alignment::Right),
        tabs_rect,
    );
}

fn draw_body(frame: &mut Frame, app: &App, area: Rect) {
    match app.current_page {
        Page::Monitor => draw_monitor(frame, app, area),
        Page::Control => draw_control(frame, app, area),
        Page::Preset => draw_preset(frame, app, area),
    }
}

fn draw_monitor(frame: &mut Frame, app: &App, area: Rect) {
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
            Line::from(format!("{:<width$}  Value", "Metric", width = label_width))
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
                .title(format!(" {} ", app.current_page.name()))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL),
        )
        // Paragraph::scroll takes (y, x), so we map our (x, y) offset accordingly.
        .scroll((app.scroll_offset.1, app.scroll_offset.0));

    frame.render_widget(paragraph, area);
}

fn draw_control(frame: &mut Frame, app: &App, area: Rect) {
    let lines = vec![Line::from("CPU Control - TODO"), Line::from("")];
    let paragraph = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(format!(" {} ", app.current_page.name()))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL),
        )
        .scroll((app.scroll_offset.1, app.scroll_offset.0));
    frame.render_widget(paragraph, area);
}

fn draw_preset(frame: &mut Frame, app: &App, area: Rect) {
    let lines = vec![Line::from("Preset - TODO"), Line::from("")];
    let paragraph = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(format!(" {} ", app.current_page.name()))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL),
        )
        .scroll((app.scroll_offset.1, app.scroll_offset.0));
    frame.render_widget(paragraph, area);
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let refresh_text = app
        .last_refresh
        .map(|instant| format!("Updated: {:.1}s ago", instant.elapsed().as_secs_f64()))
        .unwrap_or_else(|| "Updated: never".to_string());

    let status = Line::from(vec![
        Span::styled(
            " q/Esc quit ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
        Span::raw("  "),
        Span::styled(
            format!(
                " 1/2/3 or click tabs | {} metrics | {} | hjkl scroll ",
                app.snapshot.len(),
                refresh_text
            ),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    frame.render_widget(Paragraph::new(status), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_rects_are_contiguous_and_within_header() {
        let screen = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 24,
        };
        let (header, _body, _status) = layout(screen);
        let rects = tab_rects(screen, Page::Monitor);

        assert_eq!(rects.len(), Page::ALL.len());

        let mut prev_end = None;
        for (page, rect) in &rects {
            // All tabs sit on the title-bar row.
            assert_eq!(rect.y, header.y);
            assert_eq!(rect.height, 1);
            // Tab sits within the header width and to the right side.
            assert!(rect.x >= header.x);
            assert!(rect.x + rect.width <= header.x + header.width);
            // Tabs are laid out left-to-right without overlap.
            if let Some(end) = prev_end {
                assert!(
                    rect.x >= end,
                    "page {:?} overlaps previous (x={} < {})",
                    page,
                    rect.x,
                    end
                );
            }
            prev_end = Some(rect.x + rect.width);
        }

        // The last tab should reach the right edge of the header (right-aligned).
        let last_end = rects.last().unwrap().1.x + rects.last().unwrap().1.width;
        assert_eq!(last_end, header.x + header.width);
    }

    #[test]
    fn clicking_a_tab_column_maps_to_that_page() {
        let screen = Rect {
            x: 0,
            y: 0,
            width: 60,
            height: 20,
        };
        let rects = tab_rects(screen, Page::Monitor);

        // Click in the middle of each tab should resolve to that page.
        for (page, rect) in &rects {
            let col = rect.x + rect.width / 2;
            let hit = rects
                .iter()
                .find(|(_, r)| {
                    col >= r.x && col < r.x + r.width && rect.y >= r.y && rect.y < r.y + r.height
                })
                .map(|(p, _)| *p);
            assert_eq!(hit, Some(*page));
        }
    }
}
