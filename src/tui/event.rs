use crate::tui::app::{App, Page};
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
                    handle_tab_click(app, mouse.column, mouse.row);
                }
                _ => {}
            },
            _ => {}
        }
    }
    Ok(())
}

fn handle_key_event(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Esc => app.escape(),
        KeyCode::Char('1') => app.set_page(Page::Monitor),
        KeyCode::Char('2') => app.set_page(Page::Control),
        KeyCode::Char('3') => app.set_page(Page::Preset),
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

/// Handle a left-click: if it lands on a title-bar tab, switch to that page.
fn handle_tab_click(app: &mut App, column: u16, row: u16) {
    let (w, h) = crossterm::terminal::size().unwrap_or((0, 0));
    let screen = Rect {
        x: 0,
        y: 0,
        width: w,
        height: h,
    };

    for (page, rect) in ui::tab_rects(screen, app.current_page) {
        if row >= rect.y
            && row < rect.y.saturating_add(rect.height)
            && column >= rect.x
            && column < rect.x.saturating_add(rect.width)
        {
            app.set_page(page);
            break;
        }
    }
}
