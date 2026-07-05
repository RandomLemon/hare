use crate::tui::app::{App, ControlTab, FreqCol, MonitorTab, Page};
use crate::hardware::Value;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
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
    let labels = tab_labels(&MonitorTab::ALL, MonitorTab::name);
    let (sidebar, content) = sidebar_layout(area, &labels);
    draw_sidebar(frame, sidebar, &labels, monitor_tab_index(app));
    draw_monitor_content(frame, app, content);
}

fn monitor_tab_index(app: &App) -> usize {
    MonitorTab::ALL
        .iter()
        .position(|t| *t == app.monitor_tab)
        .unwrap_or(0)
}

// ----- Generic sidebar (shared by Monitor and Control pages) -----

/// Build a label list from an enum's `ALL` array.
fn tab_labels<T, F>(all: &[T], name: F) -> Vec<&'static str>
where
    F: Fn(&T) -> &'static str,
{
    all.iter().map(name).collect()
}

/// Split a body area into a left sidebar (vertical tabs) and a content area.
pub fn sidebar_layout(body: Rect, labels: &[&str]) -> (Rect, Rect) {
    let sidebar_w = sidebar_width(labels);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(sidebar_w), Constraint::Min(0)])
        .split(body);
    (chunks[0], chunks[1])
}

/// Width of the left sidebar: longest label + 2 chars of padding.
fn sidebar_width(labels: &[&str]) -> u16 {
    let max_label = labels.iter().map(|s| line_width(s)).max().unwrap_or(0);
    (max_label + 2) as u16
}

/// Per-tab hit-rectangles within the sidebar (index-aligned), for mouse clicks.
pub fn sidebar_tab_rects(body: Rect, labels: &[&str]) -> Vec<Rect> {
    let (sidebar, _content) = sidebar_layout(body, labels);
    labels
        .iter()
        .enumerate()
        .map(|(i, _)| Rect {
            x: sidebar.x,
            y: sidebar.y + i as u16,
            width: sidebar.width,
            height: 1,
        })
        .collect()
}

/// Typed Monitor sidebar rects (zips index -> `MonitorTab`).
pub fn monitor_tab_rects(body: Rect) -> Vec<(MonitorTab, Rect)> {
    let labels = tab_labels(&MonitorTab::ALL, MonitorTab::name);
    sidebar_tab_rects(body, &labels)
        .into_iter()
        .zip(MonitorTab::ALL.iter())
        .map(|(rect, tab)| (*tab, rect))
        .collect()
}

/// Typed Control sidebar rects (zips index -> `ControlTab`).
pub fn control_tab_rects(body: Rect) -> Vec<(ControlTab, Rect)> {
    let labels = tab_labels(&ControlTab::ALL, ControlTab::name);
    sidebar_tab_rects(body, &labels)
        .into_iter()
        .zip(ControlTab::ALL.iter())
        .map(|(rect, tab)| (*tab, rect))
        .collect()
}

/// Render the vertical tab list. Borderless so hit-rects line up exactly with
/// the rendered rows. The active tab is highlighted.
fn draw_sidebar(frame: &mut Frame, area: Rect, labels: &[&str], active: usize) {
    let lines: Vec<Line> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let text = format!(" {} ", label);
            let style = if i == active {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Line::from(vec![Span::styled(text, style)])
        })
        .collect();
    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

// ----- Monitor content -----

fn draw_monitor_content(frame: &mut Frame, app: &App, area: Rect) {
    match app.monitor_tab {
        MonitorTab::Overview => draw_overview_table(frame, app, area),
        MonitorTab::Cluster => draw_cluster_table(frame, app, area),
        // List-style pages: filter snapshot by the tab's metric prefix and
        // render each metric as a per-core list.
        tab if tab.metric_prefix().is_some() => {
            let prefix = tab.metric_prefix().unwrap();
            draw_metric_list(frame, app, prefix, area)
        }
        // Unreachable: every variant is either custom or has a prefix.
        _ => {}
    }
}

/// Render a list-style sub-page: metrics whose id starts with `prefix`,
/// each metric as a labelled header with per-core values expanded below.
fn draw_metric_list(frame: &mut Frame, app: &App, prefix: &str, area: Rect) {
    let entries: Vec<&crate::tui::app::SnapshotEntry> = app
        .snapshot
        .iter()
        .filter(|e| e.id.starts_with(prefix))
        .collect();

    let lines = if entries.is_empty() {
        vec![
            Line::from(format!("{} - no data", app.monitor_tab.name())),
            Line::from(""),
            Line::from("This sub-page is a placeholder for future work."),
        ]
    } else {
        render_metric_lines(&entries)
    };

    let paragraph = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(format!(" {} ", app.monitor_tab.name()))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL),
        )
        .scroll((app.scroll_offset.1, app.scroll_offset.0));

    frame.render_widget(paragraph, area);
}

/// Overview page: a per-core table aggregating current frequency and usage.
///
/// Columns: `CPU Number | Current Freq | Usage`. Data is pulled by metric id
/// from the snapshot; missing series (e.g. `cpu.usage` not implemented yet)
/// render as `—` so the table is future-proof — registering a `cpu.usage`
/// metric later fills the column automatically.
fn draw_overview_table(frame: &mut Frame, app: &App, area: Rect) {
    let rows = overview_rows(&app.snapshot);

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(vec![
            Cell::from("CPU Number"),
            Cell::from("Current Freq"),
            Cell::from("Usage"),
        ])
        .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .title(" Overview ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL),
    )
    .row_highlight_style(Style::default().bg(Color::DarkGray));

    let mut state = TableState::default();
    *state.offset_mut() = app.scroll_offset.1 as usize;
    frame.render_stateful_widget(table, area, &mut state);
}

/// Build the per-core rows for the Overview table from the current snapshot.
/// Pure (no `Frame`) so it can be unit-tested.
fn overview_rows(snapshot: &[crate::tui::app::SnapshotEntry]) -> Vec<Row<'static>> {
    overview_data(snapshot)
        .into_iter()
        .map(|(cpu, freq, usage)| {
            Row::new(vec![
                Cell::from(cpu),
                Cell::from(freq),
                Cell::from(usage),
            ])
        })
        .collect()
}

/// Per-core cell strings for the Overview table: `(cpu_number, current_freq, usage)`.
/// Missing series or NaN values yield `—`.
fn overview_data(snapshot: &[crate::tui::app::SnapshotEntry]) -> Vec<(String, String, String)> {
    let freq = series_for(snapshot, "cpu.freq.cur");
    let usage = series_for(snapshot, "cpu.usage");

    let core_count = freq
        .map(|s| s.len())
        .or_else(|| usage.map(|s| s.len()))
        .unwrap_or(0);

    let mut rows: Vec<(String, String, String)> = Vec::with_capacity(core_count);
    for i in 0..core_count {
        let cpu = format!("{}", i);
        let freq_cell = freq
            .and_then(|s| s.get(i))
            .map(cell_string)
            .unwrap_or_else(|| "—".to_string());
        let usage_cell = usage
            .and_then(|s| s.get(i))
            .map(cell_string)
            .unwrap_or_else(|| "—".to_string());
        rows.push((cpu, freq_cell, usage_cell));
    }

    if rows.is_empty() {
        rows.push(("—".to_string(), "—".to_string(), "—".to_string()));
    }
    rows
}

/// Cluster page: a per-core table showing each core's cluster id plus its
/// minimum and maximum scaling frequencies.
///
/// Columns: `CPU Number | Cluster | Min Freq | Max Freq`. Data is pulled by
/// metric id from the snapshot; missing series or NaN values render as `-`.
fn draw_cluster_table(frame: &mut Frame, app: &App, area: Rect) {
    let rows = cluster_table_rows(&app.snapshot);

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(vec![
            Cell::from("CPU Number"),
            Cell::from("Cluster"),
            Cell::from("Min Freq"),
            Cell::from("Max Freq"),
        ])
        .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .title(" Cluster ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL),
    )
    .row_highlight_style(Style::default().bg(Color::DarkGray));

    let mut state = TableState::default();
    *state.offset_mut() = app.scroll_offset.1 as usize;
    frame.render_stateful_widget(table, area, &mut state);
}

/// Build the per-core rows for the Cluster table from the current snapshot.
fn cluster_table_rows(snapshot: &[crate::tui::app::SnapshotEntry]) -> Vec<Row<'static>> {
    cluster_table_data(snapshot)
        .into_iter()
        .map(|(cpu, cluster, min, max)| {
            Row::new(vec![
                Cell::from(cpu),
                Cell::from(cluster),
                Cell::from(min),
                Cell::from(max),
            ])
        })
        .collect()
}

/// Per-core cell strings for the Cluster table: `(cpu, cluster, min, max)`.
/// Missing series or NaN values yield `-`.
fn cluster_table_data(
    snapshot: &[crate::tui::app::SnapshotEntry],
) -> Vec<(String, String, String, String)> {
    let cluster = series_for(snapshot, "cpu.topology.cluster");
    let min = series_for(snapshot, "cpu.freq.min");
    let max = series_for(snapshot, "cpu.freq.max");

    let core_count = cluster
        .map(|s| s.len())
        .or_else(|| min.map(|s| s.len()))
        .or_else(|| max.map(|s| s.len()))
        .unwrap_or(0);

    let mut rows: Vec<(String, String, String, String)> = Vec::with_capacity(core_count);
    for i in 0..core_count {
        let cpu = format!("{}", i);
        let cluster_cell = cluster
            .and_then(|s| s.get(i))
            .map(cluster_cell_string)
            .unwrap_or_else(|| "-".to_string());
        let min_cell = min
            .and_then(|s| s.get(i))
            .map(cluster_cell_string)
            .unwrap_or_else(|| "-".to_string());
        let max_cell = max
            .and_then(|s| s.get(i))
            .map(cluster_cell_string)
            .unwrap_or_else(|| "-".to_string());
        rows.push((cpu, cluster_cell, min_cell, max_cell));
    }

    if rows.is_empty() {
        rows.push((
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
        ));
    }
    rows
}

/// Format a single `Value` for a Cluster table cell, rendering NaN/empty as `-`.
fn cluster_cell_string(v: &Value) -> String {
    match v {
        Value::Freq(x) | Value::Temp(x) | Value::Percent(x) if x.is_nan() => "-".to_string(),
        Value::Raw(s) if s.trim().is_empty() => "-".to_string(),
        other => other.format(),
    }
}

/// Extract the `Value::Series` slice for a given metric id, if present.
fn series_for<'a>(snapshot: &'a [crate::tui::app::SnapshotEntry], id: &str) -> Option<&'a [Value]> {
    snapshot
        .iter()
        .find(|e| e.id == id)
        .and_then(|e| match &e.value {
            Value::Series(items) => Some(items.as_slice()),
            _ => None,
        })
}

/// Format a single `Value` for a table cell, rendering NaN as `—`.
fn cell_string(v: &Value) -> String {
    match v {
        Value::Freq(x) | Value::Temp(x) | Value::Percent(x) if x.is_nan() => "—".to_string(),
        other => other.format(),
    }
}

/// Render metrics as a hierarchical list: each metric's label is a sub-header,
/// and per-core `Series` values expand into one indented line per core.
fn render_metric_lines<'a>(
    entries: &[&'a crate::tui::app::SnapshotEntry],
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line<'a>> = Vec::new();

    for (i, entry) in entries.iter().enumerate() {
        if i > 0 {
            lines.push(Line::from(""));
        }
        lines.push(
            Line::from(entry.label.as_str())
                .style(Style::default().add_modifier(Modifier::BOLD)),
        );

        match &entry.value {
            Value::Series(items) => {
                for (idx, v) in items.iter().enumerate() {
                    lines.push(Line::from(format!("  #{}: {}", idx, v.format())));
                }
            }
            other => {
                lines.push(Line::from(format!("  {}", other.format())));
            }
        }
    }

    lines
}

fn draw_control(frame: &mut Frame, app: &App, area: Rect) {
    let labels = tab_labels(&ControlTab::ALL, ControlTab::name);
    let (sidebar, content) = sidebar_layout(area, &labels);
    let active = ControlTab::ALL
        .iter()
        .position(|t| *t == app.control_tab)
        .unwrap_or(0);
    draw_sidebar(frame, sidebar, &labels, active);
    draw_control_content(frame, app, content);
}

fn draw_control_content(frame: &mut Frame, app: &App, area: Rect) {
    // Split into the table area and a 1-line message/input strip below.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);
    match app.control_tab {
        ControlTab::Online => draw_online_control(frame, app, chunks[0]),
        ControlTab::FreqLimit => draw_freq_limit_control(frame, app, chunks[0]),
    }
    draw_control_message(frame, app, chunks[1]);
}

/// Online control: per-core `CPU | Status` table; Space/Enter toggles.
fn draw_online_control(frame: &mut Frame, app: &App, area: Rect) {
    let online = series_for(&app.snapshot, "cpu.topology.online");
    let core_count = online.map(|s| s.len()).unwrap_or(0);

    let mut rows: Vec<Row> = Vec::with_capacity(core_count);
    for i in 0..core_count {
        let cpu = format!("{}", i);
        let status = if app.online_lockable.get(i).copied().unwrap_or(false) {
            match online.and_then(|s| s.get(i)) {
                Some(Value::Bool(true)) => "online".to_string(),
                Some(Value::Bool(false)) => "offline".to_string(),
                _ => "-".to_string(),
            }
        } else {
            "locked".to_string()
        };
        rows.push(Row::new(vec![Cell::from(cpu), Cell::from(status)]));
    }
    if rows.is_empty() {
        rows.push(Row::new(vec![Cell::from("-"), Cell::from("-")]));
    }

    render_control_table(frame, app, area, rows, vec!["CPU Number", "Status"], " Online ");
}

/// Freq limit control: per-core `CPU | Min Freq | Max Freq` table; the focused
/// cell is highlighted, typing digits edits it, Enter commits.
fn draw_freq_limit_control(frame: &mut Frame, app: &App, area: Rect) {
    let min = series_for(&app.snapshot, "cpu.freq.min");
    let max = series_for(&app.snapshot, "cpu.freq.max");
    let core_count = min
        .map(|s| s.len())
        .or_else(|| max.map(|s| s.len()))
        .unwrap_or(0);

    let focused_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    let mut rows: Vec<Row> = Vec::with_capacity(core_count);
    for i in 0..core_count {
        let cpu = Cell::from(format!("{}", i));
        let min_cell = min
            .and_then(|s| s.get(i))
            .map(cell_string)
            .unwrap_or_else(|| "-".to_string());
        let max_cell = max
            .and_then(|s| s.get(i))
            .map(cell_string)
            .unwrap_or_else(|| "-".to_string());

        // Highlight the focused column on the selected row.
        let (min_styled, max_styled) = if i == app.selected_core {
            match app.freq_col {
                FreqCol::Min => (
                    Cell::new(min_cell).style(focused_style),
                    Cell::new(max_cell),
                ),
                FreqCol::Max => (
                    Cell::new(min_cell),
                    Cell::new(max_cell).style(focused_style),
                ),
            }
        } else {
            (Cell::new(min_cell), Cell::new(max_cell))
        };
        rows.push(Row::new(vec![cpu, min_styled, max_styled]));
    }
    if rows.is_empty() {
        rows.push(Row::new(vec![
            Cell::from("-"),
            Cell::from("-"),
            Cell::from("-"),
        ]));
    }

    render_control_table(
        frame,
        app,
        area,
        rows,
        vec!["CPU Number", "Min Freq", "Max Freq"],
        " Freq Limit ",
    );
}

/// Render a control `Table` with fixed column widths (12 each), a bold header,
/// and `selected_core` driving the highlight + scroll offset.
fn render_control_table(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    rows: Vec<Row>,
    header: Vec<&str>,
    title: &str,
) {
    let widths: Vec<Constraint> = (0..header.len()).map(|_| Constraint::Length(12)).collect();
    let table = Table::new(rows, widths)
        .header(
            Row::new(header.into_iter().map(Cell::from).collect::<Vec<_>>())
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(format!(" {} ", title))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL),
        )
        .row_highlight_style(Style::default().bg(Color::DarkGray));

    let mut state = TableState::default();
    state.select(Some(app.selected_core));
    frame.render_stateful_widget(table, area, &mut state);
}

/// Bottom strip: freq-edit input line when editing, otherwise the last
/// control message (if any).
fn draw_control_message(frame: &mut Frame, app: &App, area: Rect) {
    let line = if let Some(buf) = &app.freq_edit {
        let col = match app.freq_col {
            FreqCol::Min => "min",
            FreqCol::Max => "max",
        };
        Line::from(vec![
            Span::styled(
                format!(" cpu{} {} ", app.selected_core, col),
                Style::default().bg(Color::DarkGray).fg(Color::White),
            ),
            Span::raw(" "),
            Span::raw(buf.clone()),
            Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
            Span::raw("  [Enter=apply Esc=cancel]"),
        ])
    } else {
        Line::from(vec![Span::styled(
            app.control_message
                .clone()
                .unwrap_or_else(|| " Space toggle | m/edit? see help ".to_string()),
            Style::default().fg(Color::DarkGray),
        )])
    };
    frame.render_widget(Paragraph::new(line), area);
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
                " 1/2/3 or click tabs | Tab/Shift+Tab sub-tabs | {} metrics | {} | hjkl scroll ",
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
                .find(|(_, r)| col >= r.x && col < r.x + r.width && rect.y >= r.y && rect.y < r.y + r.height)
                .map(|(p, _)| *p);
            assert_eq!(hit, Some(*page));
        }
    }

    #[test]
    fn monitor_sidebar_tabs_stack_vertically_without_overlap() {
        use crate::tui::app::MonitorTab;
        let body = Rect {
            x: 0,
            y: 1,
            width: 80,
            height: 22,
        };
        let labels = tab_labels(&MonitorTab::ALL, MonitorTab::name);
        let (sidebar, content) = sidebar_layout(body, &labels);
        // Sidebar is on the left, content to its right.
        assert!(sidebar.x <= content.x);
        assert!(sidebar.width < body.width);

        let rects = monitor_tab_rects(body);
        assert_eq!(rects.len(), MonitorTab::ALL.len());

        let mut prev_bottom = None;
        for (i, (tab, rect)) in rects.iter().enumerate() {
            // Each tab fills the sidebar width and is one row tall.
            assert_eq!(rect.width, sidebar.width);
            assert_eq!(rect.height, 1);
            assert_eq!(rect.x, sidebar.x);
            // Stacked top-to-bottom in enum order.
            assert_eq!(rect.y, sidebar.y + i as u16);
            if let Some(bottom) = prev_bottom {
                assert!(rect.y >= bottom);
            }
            prev_bottom = Some(rect.y + rect.height);
            assert!(rect.y < sidebar.y + sidebar.height);
            assert_eq!(*tab, MonitorTab::ALL[i]);
        }
    }

    #[test]
    fn control_sidebar_tabs_stack_vertically_without_overlap() {
        use crate::tui::app::ControlTab;
        let body = Rect {
            x: 0,
            y: 1,
            width: 80,
            height: 22,
        };
        let labels = tab_labels(&ControlTab::ALL, ControlTab::name);
        let (sidebar, _content) = sidebar_layout(body, &labels);
        let rects = control_tab_rects(body);
        assert_eq!(rects.len(), ControlTab::ALL.len());
        for (i, (tab, rect)) in rects.iter().enumerate() {
            assert_eq!(rect.x, sidebar.x);
            assert_eq!(rect.width, sidebar.width);
            assert_eq!(rect.height, 1);
            assert_eq!(rect.y, sidebar.y + i as u16);
            assert_eq!(*tab, ControlTab::ALL[i]);
        }
    }

    #[test]
    fn overview_rows_match_freq_series_and_pad_missing_usage() {
        use crate::hardware::Value;
        use crate::tui::app::SnapshotEntry;

        let snapshot = vec![
            SnapshotEntry {
                id: "cpu.freq.cur".to_string(),
                label: "Current Frequency".to_string(),
                unit: "MHz".to_string(),
                value: Value::Series(vec![
                    Value::Freq(2400.0),
                    Value::Freq(f64::NAN),
                    Value::Freq(1800.0),
                ]),
            },
            // No `cpu.usage` entry -> Usage column should be `—`.
            SnapshotEntry {
                id: "cpu.governor".to_string(),
                label: "Scaling Governor".to_string(),
                unit: "".to_string(),
                value: Value::Series(vec![Value::Enum("powersave".into())]),
            },
        ];

        let rows = overview_data(&snapshot);
        assert_eq!(rows.len(), 3);

        // Row count tracks the freq series length; CPU number is the index.
        assert_eq!(rows[0].0, "0");
        assert_eq!(rows[2].0, "2");

        // Missing usage => every Usage cell is "—".
        for (_cpu, _freq, usage) in &rows {
            assert_eq!(usage, "—");
        }

        // NaN frequency should render as "—".
        assert_eq!(rows[1].1, "—");
        // A real frequency renders with its formatted value.
        assert!(rows[0].1.contains("MHz"));
    }

    #[test]
    fn cluster_table_data_per_core_with_freqs() {
        use crate::hardware::Value;
        use crate::tui::app::SnapshotEntry;

        let snapshot = vec![
            SnapshotEntry {
                id: "cpu.topology.cluster".to_string(),
                label: "Cluster".to_string(),
                unit: "".to_string(),
                value: Value::Series(vec![Value::Raw("0".into()), Value::Raw("1".into())]),
            },
            SnapshotEntry {
                id: "cpu.freq.min".to_string(),
                label: "Minimum Frequency".to_string(),
                unit: "MHz".to_string(),
                value: Value::Series(vec![Value::Freq(800.0), Value::Freq(f64::NAN)]),
            },
            SnapshotEntry {
                id: "cpu.freq.max".to_string(),
                label: "Maximum Frequency".to_string(),
                unit: "MHz".to_string(),
                value: Value::Series(vec![Value::Freq(3500.0), Value::Freq(4500.0)]),
            },
        ];

        let rows = cluster_table_data(&snapshot);
        assert_eq!(rows.len(), 2);

        // CPU number is the index; cluster sits right after it.
        assert_eq!(rows[0].0, "0");
        assert_eq!(rows[0].1, "0");
        assert_eq!(rows[1].0, "1");
        assert_eq!(rows[1].1, "1");

        // Real frequencies render with MHz; NaN frequency renders as `-`.
        assert!(rows[0].2.contains("MHz"));
        assert_eq!(rows[1].2, "-");
        assert!(rows[0].3.contains("MHz"));
        assert!(rows[1].3.contains("MHz"));
    }

    #[test]
    fn cluster_table_data_missing_series_renders_dash() {
        use crate::hardware::Value;
        use crate::tui::app::SnapshotEntry;

        // Only the cluster series is present; min/max columns should be `-`.
        let snapshot = vec![SnapshotEntry {
            id: "cpu.topology.cluster".to_string(),
            label: "Cluster".to_string(),
            unit: "".to_string(),
            value: Value::Series(vec![Value::Raw("-".into()), Value::Raw("0".into())]),
        }];

        let rows = cluster_table_data(&snapshot);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].1, "-"); // unsupported core -> `-`
        assert_eq!(rows[0].2, "-"); // missing min freq -> `-`
        assert_eq!(rows[0].3, "-"); // missing max freq -> `-`
    }
}
