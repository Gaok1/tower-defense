use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use std::time::Duration;

use crate::app::{App, ButtonId, HoverAction, TowerKind};

pub fn pump(app: &mut App) -> Result<()> {
    if !event::poll(Duration::from_millis(12))? {
        return Ok(());
    }

    match event::read()? {
        Event::Key(k) => {
            if k.kind != KeyEventKind::Press {
                return Ok(());
            }
            match k.code {
                KeyCode::Esc | KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Char(' ') => app.handle_button(ButtonId::StartPause),
                KeyCode::Char('b') => app.handle_button(ButtonId::Build),
                KeyCode::Char('u') => app.handle_button(ButtonId::Upgrade),
                KeyCode::Char('s') => app.handle_button(ButtonId::Sell),
                KeyCode::Char('f') => app.handle_button(ButtonId::Speed),
                KeyCode::Char('1') => app.game.build_kind = TowerKind::Basic,
                KeyCode::Char('2') => app.game.build_kind = TowerKind::Sniper,
                KeyCode::Char('3') => app.game.build_kind = TowerKind::Rapid,
                _ => {}
            }
        }
        Event::Resize(_, _) => { /* draw recalcula tudo */ }
        Event::Mouse(m) => match m.kind {
            MouseEventKind::Moved => {
                app.ui.hover_button = hit_test_button(app, m.column, m.row);
                app.ui.hover_cell = map_cell_at(app, m.column, m.row);
                app.ui.hover_action = hit_test_action(app, m.column, m.row);
                app.ui.hover_build_kind = hit_test_build(app, m.column, m.row);
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(btn) = hit_test_button(app, m.column, m.row) {
                    app.handle_button(btn);
                } else if let Some(act) = hit_test_action(app, m.column, m.row) {
                    match act {
                        HoverAction::UpgradePreview => app.handle_button(ButtonId::Upgrade),
                    }
                } else if let Some(kind) = hit_test_build(app, m.column, m.row) {
                    app.game.build_kind = kind;
                } else if let Some(cell) = map_cell_at(app, m.column, m.row) {
                    app.game.selected_cell = Some(cell);
                }
            }
            _ => {}
        },
        _ => {}
    }

    Ok(())
}

fn hit_test_button(app: &App, x: u16, y: u16) -> Option<ButtonId> {
    let rects = app.ui.hit.buttons;
    let ids = [
        ButtonId::StartPause,
        ButtonId::Build,
        ButtonId::Upgrade,
        ButtonId::Sell,
        ButtonId::Speed,
        ButtonId::Quit,
    ];
    for (i, id) in ids.iter().enumerate() {
        if contains(rects[i], x, y) {
            return Some(*id);
        }
    }
    None
}

fn hit_test_action(app: &App, x: u16, y: u16) -> Option<HoverAction> {
    if contains(app.ui.hit.inspector_upgrade, x, y) {
        return Some(HoverAction::UpgradePreview);
    }
    None
}

fn hit_test_build(app: &App, x: u16, y: u16) -> Option<TowerKind> {
    let rects = app.ui.hit.build_options;
    let ids = [TowerKind::Basic, TowerKind::Sniper, TowerKind::Rapid];
    for (i, id) in ids.iter().enumerate() {
        if contains(rects[i], x, y) {
            return Some(*id);
        }
    }
    None
}

fn map_cell_at(app: &App, x: u16, y: u16) -> Option<(u16, u16)> {
    let inner = app.ui.hit.map_inner;
    if inner.width == 0 || inner.height == 0 {
        return None;
    }
    if x < inner.x || y < inner.y || x >= inner.x + inner.width || y >= inner.y + inner.height {
        return None;
    }

    let vp = app.ui.viewport;
    if vp.vis_w == 0 || vp.vis_h == 0 {
        return None;
    }

    let local_x = x - inner.x;
    let local_y = y - inner.y;

    let cx = vp.view_x + (local_x / vp.tile_w);
    let cy = vp.view_y + (local_y / vp.tile_h);

    if cx < app.game.grid_w && cy < app.game.grid_h {
        Some((cx, cy))
    } else {
        None
    }
}

fn contains(r: ratatui::layout::Rect, x: u16, y: u16) -> bool {
    x >= r.x && y >= r.y && x < r.x + r.width && y < r.y + r.height
}
