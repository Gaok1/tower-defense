use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use std::time::Duration;

use crate::app::{App, ButtonId, HoverAction, MapSelectAction, Screen, TowerKind};

pub fn pump(app: &mut App) -> Result<()> {
    if !event::poll(Duration::from_millis(12))? {
        return Ok(());
    }

    match event::read()? {
        Event::Key(k) => {
            if k.kind != KeyEventKind::Press {
                return Ok(());
            }
            match app.screen {
                Screen::MapSelect => match k.code {
                    KeyCode::Esc | KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Left | KeyCode::Char('a') => app.select_prev_map(),
                    KeyCode::Right | KeyCode::Char('d') => app.select_next_map(),
                    KeyCode::Enter | KeyCode::Char(' ') => app.start_selected_map(),
                    _ => {}
                },
                Screen::Game => match k.code {
                    KeyCode::Esc | KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Char(' ') => app.handle_button(ButtonId::StartPause),
                    KeyCode::Char('b') => app.handle_button(ButtonId::Build),
                    KeyCode::Char('u') => app.handle_button(ButtonId::Upgrade),
                    KeyCode::Char('s') => app.handle_button(ButtonId::Sell),
                    KeyCode::Char('f') => app.handle_button(ButtonId::Speed),
                    KeyCode::Char('1') => app.toggle_build_kind(TowerKind::Basic),
                    KeyCode::Char('2') => app.toggle_build_kind(TowerKind::Sniper),
                    KeyCode::Char('3') => app.toggle_build_kind(TowerKind::Rapid),
                    KeyCode::Char('4') => app.toggle_build_kind(TowerKind::Cannon),
                    KeyCode::Char('5') => app.toggle_build_kind(TowerKind::Tesla),
                    KeyCode::Char('6') => app.toggle_build_kind(TowerKind::Frost),
                    KeyCode::Char('+') | KeyCode::Char('=') => app.cycle_zoom(1),
                    KeyCode::Char('-') => app.cycle_zoom(-1),
                    _ => {}
                },
            }
        }
        Event::Resize(_, _) => { /* draw recalcula tudo */ }
        Event::Mouse(m) => match m.kind {
            MouseEventKind::Moved => {
                app.ui.hover_button = hit_test_button(app, m.column, m.row);
                app.ui.hover_cell = map_cell_at(app, m.column, m.row);
                app.ui.hover_action = hit_test_action(app, m.column, m.row);
                app.ui.hover_build_kind = hit_test_build(app, m.column, m.row);
                app.ui.hover_map_select = hit_test_map_select(app, m.column, m.row);
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if app.screen == Screen::MapSelect {
                    if let Some(action) = hit_test_map_select(app, m.column, m.row) {
                        match action {
                            MapSelectAction::Prev => app.select_prev_map(),
                            MapSelectAction::Next => app.select_next_map(),
                            MapSelectAction::Start => app.start_selected_map(),
                        }
                    }
                } else if let Some(btn) = hit_test_button(app, m.column, m.row) {
                    app.handle_button(btn);
                } else if let Some(act) = hit_test_action(app, m.column, m.row) {
                    match act {
                        HoverAction::UpgradePreview => app.handle_button(ButtonId::Upgrade),
                    }
                } else if let Some(kind) = hit_test_build(app, m.column, m.row) {
                    app.toggle_build_kind(kind);
                } else if let Some(cell) = map_cell_at(app, m.column, m.row) {
                    if let Some(kind) = app.game.build_kind {
                        if app.build_at(cell.0, cell.1, kind) {
                            app.game.build_kind = None;
                            app.game.selected_cell = Some(cell);
                        }
                    } else {
                        // Toggle apenas para torre: clicar de novo deseleciona (e some o range).
                        if app.game.selected_cell == Some(cell)
                            && app.tower_index_at(cell.0, cell.1).is_some()
                        {
                            app.game.selected_cell = None;
                        } else {
                            app.game.selected_cell = Some(cell);
                        }
                    }
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
    let ids = App::available_towers();
    for (i, id) in ids.iter().enumerate() {
        if contains(rects[i], x, y) {
            return Some(*id);
        }
    }
    None
}

fn map_cell_at(app: &App, x: u16, y: u16) -> Option<(u16, u16)> {
    if app.screen != Screen::Game {
        return None;
    }
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

fn hit_test_map_select(app: &App, x: u16, y: u16) -> Option<MapSelectAction> {
    if app.screen != Screen::MapSelect {
        return None;
    }
    if contains(app.ui.hit.map_select_left, x, y) {
        return Some(MapSelectAction::Prev);
    }
    if contains(app.ui.hit.map_select_right, x, y) {
        return Some(MapSelectAction::Next);
    }
    if contains(app.ui.hit.map_select_start, x, y) {
        return Some(MapSelectAction::Start);
    }
    None
}
