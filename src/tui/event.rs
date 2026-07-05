use crate::tui::app::{App, ControlTab, MonitorTab, Page};
use crate::tui::ui;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::layout::Rect;
use std::time::Duration;

pub fn handle_events(app: &mut App, timeout: Duration) -> Result<()> {
    if event::poll(timeout)? {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                handle_key_event(app, key.code);
            }
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => app.scroll_up(3),
                MouseEventKind::ScrollDown => app.scroll_down(3),
                MouseEventKind::Down(MouseButton::Left) => {
                    handle_click(app, mouse.column, mouse.row);
                }
                _ => {}
            },
            _ => {}
        }
    }
    Ok(())
}

fn handle_key_event(app: &mut App, code: KeyCode) {
    // Global keys.
    match code {
        KeyCode::Char('q') => {
            app.quit();
            return;
        }
        KeyCode::Esc => {
            app.escape();
            return;
        }
        KeyCode::Char('1') => {
            app.set_page(Page::Monitor);
            return;
        }
        KeyCode::Char('2') => {
            app.set_page(Page::Control);
            return;
        }
        KeyCode::Char('3') => {
            app.set_page(Page::Preset);
            return;
        }
        KeyCode::Tab => {
            app.cycle_sub_tab_next();
            return;
        }
        KeyCode::BackTab => {
            app.cycle_sub_tab_prev();
            return;
        }
        _ => {}
    }

    // Page-specific keys.
    match app.current_page {
        Page::Monitor => handle_monitor_key(app, code),
        Page::Control => handle_control_key(app, code),
        Page::Preset => {}
    }
}

fn handle_monitor_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up | KeyCode::Char('k') => app.scroll_up(1),
        KeyCode::Down | KeyCode::Char('j') => app.scroll_down(1),
        KeyCode::Left | KeyCode::Char('h') => app.scroll_left(3),
        KeyCode::Right | KeyCode::Char('l') => app.scroll_right(3),
        KeyCode::PageUp => app.scroll_up(10),
        KeyCode::PageDown => app.scroll_down(10),
        KeyCode::Home => app.scroll_up(u16::MAX),
        KeyCode::End => app.scroll_down(u16::MAX),
        _ => {}
    }
}

fn handle_control_key(app: &mut App, code: KeyCode) {
    match app.control_tab {
        ControlTab::Online => handle_online_control_key(app, code),
        ControlTab::FreqLimit => handle_freq_limit_control_key(app, code),
    }
}

fn handle_online_control_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up | KeyCode::Char('k') => app.move_cursor_vertical(false),
        KeyCode::Down | KeyCode::Char('j') => app.move_cursor_vertical(true),
        KeyCode::Char(' ') | KeyCode::Enter => app.toggle_online(),
        _ => {}
    }
}

fn handle_freq_limit_control_key(app: &mut App, code: KeyCode) {
    // While editing, only digits / Backspace / Enter / Esc are active.
    if app.freq_edit.is_some() {
        match code {
            KeyCode::Char(c) if c.is_ascii_digit() => app.freq_input(c),
            KeyCode::Backspace => app.freq_backspace(),
            KeyCode::Enter => app.freq_commit(),
            // Esc handled globally (cancels edit).
            _ => {}
        }
        return;
    }

    match code {
        KeyCode::Up | KeyCode::Char('k') => app.move_cursor_vertical(false),
        KeyCode::Down | KeyCode::Char('j') => app.move_cursor_vertical(true),
        KeyCode::Left | KeyCode::Char('h') => app.move_cursor_horizontal(true),
        KeyCode::Right | KeyCode::Char('l') => app.move_cursor_horizontal(false),
        KeyCode::Char(c) if c.is_ascii_digit() => app.freq_input(c),
        KeyCode::Enter => app.freq_commit(),
        _ => {}
    }
}

/// Handle a left-click: top-level title-bar tabs first, then (per page) the
/// left sidebar sub-tabs.
fn handle_click(app: &mut App, column: u16, row: u16) {
    let (w, h) = crossterm::terminal::size().unwrap_or((0, 0));
    let screen = Rect {
        x: 0,
        y: 0,
        width: w,
        height: h,
    };

    for (page, rect) in ui::tab_rects(screen, app.current_page) {
        if hit(rect, column, row) {
            app.set_page(page);
            return;
        }
    }

    let (_header, body, _status) = ui::layout(screen);
    let sidebar_rects: Vec<(usize, Rect)> = match app.current_page {
        Page::Monitor => index_rects(ui::monitor_tab_rects(body)),
        Page::Control => index_rects(ui::control_tab_rects(body)),
        Page::Preset => Vec::new(),
    };
    // Mouse-click switching of sub-tabs is disabled while a freq edit is active.
    if app.freq_edit.is_some() {
        return;
    }
    for (i, rect) in sidebar_rects {
        if hit(rect, column, row) {
            match app.current_page {
                Page::Monitor => app.set_monitor_tab(MonitorTab::ALL[i]),
                Page::Control => app.set_control_tab(ControlTab::ALL[i]),
                Page::Preset => {}
            }
            return;
        }
    }
}

fn index_rects<T>(rects: Vec<(T, Rect)>) -> Vec<(usize, Rect)> {
    rects.into_iter().map(|(_, r)| r).enumerate().collect()
}

fn hit(rect: Rect, column: u16, row: u16) -> bool {
    row >= rect.y
        && row < rect.y.saturating_add(rect.height)
        && column >= rect.x
        && column < rect.x.saturating_add(rect.width)
}
