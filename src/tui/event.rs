use crate::tui::app::App;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseEventKind};
use std::time::Duration;

pub fn handle_events(app: &mut App, timeout: Duration) -> Result<()> {
    if event::poll(timeout)? {
        match event::read()? {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    handle_key_event(app, key.code);
                }
            }
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => app.scroll_up(3),
                MouseEventKind::ScrollDown => app.scroll_down(3),
                _ => {}
            },
            _ => {}
        }
    }
    Ok(())
}

fn handle_key_event(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => app.quit(),
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
