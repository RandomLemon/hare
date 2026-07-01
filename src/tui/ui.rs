use crate::tui::app::{App, MonitorTab, Page};
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
    let (sidebar, content) = monitor_layout(area);
    draw_monitor_sidebar(frame, app, sidebar);
    draw_monitor_content(frame, app, content);
}

/// Split the Monitor body into a left sidebar (vertical tabs) and content area.
pub fn monitor_layout(body: Rect) -> (Rect, Rect) {
    let sidebar_w = sidebar_width();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(sidebar_w), Constraint::Min(0)])
        .split(body);
    (chunks[0], chunks[1])
}

/// Width of the left sidebar: longest tab label + padding for breathing room.
fn sidebar_width() -> u16 {
    let max_label = MonitorTab::ALL
        .iter()
        .map(|t| line_width(t.name()))
        .max()
        .unwrap_or(0);
    // 1 leading space + label + 1 trailing space.
    (max_label + 2) as u16
}

/// Per-sub-tab hit-rectangles within the sidebar, for mouse click handling.
pub fn monitor_tab_rects(body: Rect) -> Vec<(MonitorTab, Rect)> {
    let (sidebar, _content) = monitor_layout(body);
    MonitorTab::ALL
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            (
                *tab,
                Rect {
                    x: sidebar.x,
                    y: sidebar.y + i as u16,
                    width: sidebar.width,
                    height: 1,
                },
            )
        })
        .collect()
}

fn draw_monitor_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let lines: Vec<Line> = MonitorTab::ALL
        .iter()
        .map(|tab| {
            let label = format!(" {} ", tab.name());
            let style = if *tab == app.monitor_tab {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Line::from(vec![Span::styled(label, style)])
        })
        .collect();

    // Borderless sidebar so hit-rects (computed from the same area) line up
    // exactly with the rendered rows. The content block provides the visual
    // frame on the right.
    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn draw_monitor_content(frame: &mut Frame, app: &App, area: Rect) {
    match app.monitor_tab {
        MonitorTab::Overview => draw_overview_table(frame, app, area),
        MonitorTab::Cluster => draw_cluster_grouped(frame, app, area),
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

/// Cluster page: cores grouped by their `cluster_id`.
///
/// Reads the `cpu.topology.cluster` series (one `Value::Raw` per core) and
/// groups core indices by cluster id, rendering one block per cluster with its
/// member cores. Cores lacking `cluster_id` (unsupported kernel) fall into a
/// `"-"` cluster.
fn draw_cluster_grouped(frame: &mut Frame, app: &App, area: Rect) {
    let lines = match series_for(&app.snapshot, "cpu.topology.cluster") {
        None => vec![
            Line::from("Cluster - no data"),
            Line::from(""),
            Line::from("This sub-page is a placeholder for future work."),
        ],
        Some(series) => render_cluster_groups(cluster_groups(series)),
    };

    let paragraph = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(" Cluster ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL),
        )
        .scroll((app.scroll_offset.1, app.scroll_offset.0));
    frame.render_widget(paragraph, area);
}

/// Group core indices by their cluster id string, sorted by cluster id.
/// Pure so it can be unit-tested.
fn cluster_groups(series: &[Value]) -> Vec<(String, Vec<usize>)> {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, v) in series.iter().enumerate() {
        let cluster = match v {
            Value::Raw(s) => s.trim().to_string(),
            other => other.format(),
        };
        groups.entry(cluster).or_default().push(i);
    }
    groups.into_iter().collect()
}

/// Render grouped cores as: a bold `Cluster <id>` header line, followed by one
/// indented line listing the member cores (`cpu0, cpu1, ...`).
fn render_cluster_groups(groups: Vec<(String, Vec<usize>)>) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    for (i, (cluster, cores)) in groups.iter().enumerate() {
        if i > 0 {
            lines.push(Line::from(""));
        }
        lines.push(
            Line::from(format!("Cluster {}", cluster))
                .style(Style::default().add_modifier(Modifier::BOLD)),
        );
        let members: Vec<String> = cores.iter().map(|c| format!("cpu{}", c)).collect();
        lines.push(Line::from(format!("  {}", members.join(", "))));
    }
    lines
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
        let (sidebar, content) = monitor_layout(body);
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
    fn cluster_groups_group_cores_by_id_sorted() {
        use crate::hardware::Value;
        // cpu0 -> cluster 0, cpu1 -> cluster 4, cpu2 -> cluster 0, cpu3 -> "-"
        let series = vec![
            Value::Raw("0".to_string()),
            Value::Raw("4".to_string()),
            Value::Raw("0".to_string()),
            Value::Raw("-".to_string()),
        ];
        let groups = cluster_groups(&series);
        // BTreeMap orders keys: "-", "0", "4".
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].0, "-");
        assert_eq!(groups[0].1, vec![3]);
        assert_eq!(groups[1].0, "0");
        assert_eq!(groups[1].1, vec![0, 2]);
        assert_eq!(groups[2].0, "4");
        assert_eq!(groups[2].1, vec![1]);
    }
}
