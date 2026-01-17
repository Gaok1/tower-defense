use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use std::time::Duration;

use crate::app::{
    App, ButtonId, HoverAction, MapSelectAction, MultiplayerAction, MultiplayerFocus, Screen,
    TowerKind,
};

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
                Screen::MainMenu => match k.code {
                    KeyCode::Esc | KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Up | KeyCode::Char('w') => app.main_menu_prev(),
                    KeyCode::Down | KeyCode::Char('s') => app.main_menu_next(),
                    KeyCode::Enter | KeyCode::Char(' ') => app.main_menu_activate(),
                    _ => {}
                },
                Screen::Multiplayer => match k.code {
                    KeyCode::Esc => app.enter_main_menu(),
                    KeyCode::Backspace => {
                        if matches!(
                            app.multiplayer.focus,
                            MultiplayerFocus::PeerIp | MultiplayerFocus::Name
                        ) {
                            app.multiplayer_backspace();
                        }
                    }
                    KeyCode::Enter => {
                        if matches!(
                            app.multiplayer.focus,
                            MultiplayerFocus::PeerIp | MultiplayerFocus::Name
                        ) {
                            app.multiplayer.focus = MultiplayerFocus::Continue;
                        }
                    }
                    KeyCode::Char(c) => {
                        if matches!(
                            app.multiplayer.focus,
                            MultiplayerFocus::PeerIp | MultiplayerFocus::Name
                        ) {
                            app.multiplayer_input_char(c);
                        }
                    }
                    _ => {}
                },
                Screen::MapSelect => match k.code {
                    KeyCode::Esc => app.enter_main_menu(),
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Left | KeyCode::Char('a') => app.select_prev_map(),
                    KeyCode::Right | KeyCode::Char('d') => app.select_next_map(),
                    KeyCode::Enter | KeyCode::Char(' ') => app.start_selected_map(),
                    _ => {}
                },
                Screen::LoadGame => match k.code {
                    KeyCode::Esc => app.enter_main_menu(),
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Up | KeyCode::Char('w') => app.load_menu_prev(),
                    KeyCode::Down | KeyCode::Char('s') => app.load_menu_next(),
                    KeyCode::Left => app.load_menu_focus_left(),
                    KeyCode::Right => app.load_menu_focus_right(),
                    KeyCode::Tab => match app.load_menu.focus {
                        crate::app::LoadMenuFocus::Slots => app.load_menu_focus_right(),
                        crate::app::LoadMenuFocus::Waves => app.load_menu_focus_left(),
                    },
                    KeyCode::Enter => app.load_menu_activate(),
                    KeyCode::Char('r') => app.refresh_load_menu(),
                    _ => {}
                },
                Screen::Game => match k.code {
                    KeyCode::Esc | KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Char(' ') => app.handle_button(ButtonId::StartPause),
                    KeyCode::Char('r') => app.handle_button(ButtonId::StartWave),
                    KeyCode::Char('b') => app.handle_button(ButtonId::Build),
                    KeyCode::Char('u') => app.handle_button(ButtonId::Upgrade),
                    KeyCode::Char('s') => app.handle_button(ButtonId::Sell),
                    KeyCode::Char('f') => app.handle_button(ButtonId::Speed),
                    KeyCode::F(12) => app.toggle_dev_mode(),
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
                app.ui.hover_button = if app.screen == Screen::Game {
                    hit_test_button(app, m.column, m.row)
                } else {
                    None
                };
                app.ui.hover_cell = map_cell_at(app, m.column, m.row);
                app.ui.hover_action = if app.screen == Screen::Game {
                    hit_test_action(app, m.column, m.row)
                } else {
                    None
                };
                app.ui.hover_build_kind = if app.screen == Screen::Game {
                    hit_test_build(app, m.column, m.row)
                } else {
                    None
                };
                app.ui.hover_map_select = hit_test_map_select(app, m.column, m.row);
                app.ui.hover_multiplayer = if app.screen == Screen::Multiplayer {
                    hit_test_multiplayer(app, m.column, m.row)
                } else {
                    None
                };
                app.ui.hover_main_menu = hit_test_main_menu(app, m.column, m.row);
                app.ui.hover_load_slot = hit_test_load_slot(app, m.column, m.row);
                app.ui.hover_load_wave = hit_test_load_wave(app, m.column, m.row);
            }
            MouseEventKind::Down(MouseButton::Left) => {
                app.ui.drag_origin = None;
                app.ui.drag_view = None;
                if app.screen == Screen::MainMenu {
                    if let Some(index) = hit_test_main_menu(app, m.column, m.row) {
                        app.main_menu_index = index;
                        app.main_menu_activate();
                    }
                } else if app.screen == Screen::LoadGame {
                    if let Some(index) = hit_test_load_slot(app, m.column, m.row) {
                        let should_load = app.load_menu.focus == crate::app::LoadMenuFocus::Slots
                            && app.load_menu.selected_slot == index;
                        app.load_menu.focus = crate::app::LoadMenuFocus::Slots;
                        app.load_menu.selected_slot = index;
                        app.load_menu.selected_wave = app
                            .load_menu
                            .slots
                            .get(index)
                            .map(|s| s.waves.len().saturating_sub(1))
                            .unwrap_or(0);
                        if should_load {
                            app.load_menu_activate();
                        }
                    } else if let Some(index) = hit_test_load_wave(app, m.column, m.row) {
                        let should_load = app.load_menu.focus == crate::app::LoadMenuFocus::Waves
                            && app.load_menu.selected_wave == index;
                        app.load_menu.focus = crate::app::LoadMenuFocus::Waves;
                        app.load_menu.selected_wave = index;
                        if should_load {
                            app.load_menu_activate();
                        }
                    }
                } else if app.screen == Screen::MapSelect {
                    if let Some(action) = hit_test_map_select(app, m.column, m.row) {
                        match action {
                            MapSelectAction::Prev => app.select_prev_map(),
                            MapSelectAction::Next => app.select_next_map(),
                            MapSelectAction::Start => app.start_selected_map(),
                        }
                    }
                } else if app.screen == Screen::Game {
                    if let Some(btn) = hit_test_button(app, m.column, m.row) {
                        app.handle_button(btn);
                    } else if let Some(act) = hit_test_action(app, m.column, m.row) {
                        match act {
                            HoverAction::UpgradePreview => app.handle_button(ButtonId::Upgrade),
                        }
                    } else if let Some(kind) = hit_test_build(app, m.column, m.row) {
                        app.toggle_build_kind(kind);
                    } else if let Some(cell) = map_cell_at(app, m.column, m.row) {
                        if let Some(kind) = app.game.build_kind {
                            app.game.selected_cell = Some(cell);
                            if app.can_build_at(cell.0, cell.1, kind) {
                                if app.game.pending_build == Some(cell) {
                                    app.handle_button(ButtonId::Build);
                                } else {
                                    app.game.pending_build = Some(cell);
                                }
                            } else {
                                app.game.pending_build = None;
                            }
                        } else {
                            app.game.pending_build = None;
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
                } else if app.screen == Screen::Multiplayer {
                    if let Some(action) = hit_test_multiplayer(app, m.column, m.row) {
                        match action {
                            MultiplayerAction::CreateLobby => {
                                app.multiplayer_set_role(crate::app::MultiplayerRole::Host)
                            }
                            MultiplayerAction::JoinLobby => {
                                app.multiplayer_set_role(crate::app::MultiplayerRole::Peer)
                            }
                            MultiplayerAction::ToggleIpMode => app.multiplayer_toggle_ip_mode(),
                            MultiplayerAction::CopyStunIp => app.multiplayer_copy_public_ip(),
                            MultiplayerAction::RefreshStun => app.multiplayer_refresh_ip(),
                            MultiplayerAction::Connect => app.multiplayer_connect(),
                            MultiplayerAction::Continue => app.multiplayer_continue(),
                            MultiplayerAction::FocusPeerIp => {
                                app.multiplayer.focus = crate::app::MultiplayerFocus::PeerIp
                            }
                            MultiplayerAction::FocusName => {
                                app.multiplayer.focus = crate::app::MultiplayerFocus::Name
                            }
                            MultiplayerAction::KickPlayer(index) => {
                                app.multiplayer_kick_player(index)
                            }
                        }
                    }
                    else {
                        app.multiplayer.focus = MultiplayerFocus::Continue;
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Right) => {
                if app.screen == Screen::Game && map_cell_at(app, m.column, m.row).is_some() {
                    app.ui.drag_origin = Some((m.column, m.row));
                    app.ui.drag_view = Some((app.ui.viewport.view_x, app.ui.viewport.view_y));
                }
            }
            MouseEventKind::Drag(MouseButton::Right) => {
                if app.screen == Screen::Game {
                    if let (Some((ox, oy)), Some((vx, vy))) = (app.ui.drag_origin, app.ui.drag_view)
                    {
                        let vp = app.ui.viewport;
                        let dx = (ox as i16 - m.column as i16) / vp.tile_w.max(1) as i16;
                        let dy = (oy as i16 - m.row as i16) / vp.tile_h.max(1) as i16;
                        let max_x = app.game.grid_w.saturating_sub(vp.vis_w);
                        let max_y = app.game.grid_h.saturating_sub(vp.vis_h);
                        let next_x = (vx as i16 + dx).clamp(0, max_x as i16) as u16;
                        let next_y = (vy as i16 + dy).clamp(0, max_y as i16) as u16;
                        app.ui.viewport.view_x = next_x;
                        app.ui.viewport.view_y = next_y;
                        app.ui.manual_pan = true;
                    }
                }
            }
            MouseEventKind::Up(MouseButton::Right) => {
                app.ui.drag_origin = None;
                app.ui.drag_view = None;
            }
            MouseEventKind::ScrollUp => {
                if app.screen == Screen::Game && map_cell_at(app, m.column, m.row).is_some() {
                    app.cycle_zoom(1);
                }
            }
            MouseEventKind::ScrollDown => {
                if app.screen == Screen::Game && map_cell_at(app, m.column, m.row).is_some() {
                    app.cycle_zoom(-1);
                }
            }

            // (Opcional) pan com botão do meio também:
            MouseEventKind::Down(MouseButton::Middle) => {
                if app.screen == Screen::Game && map_cell_at(app, m.column, m.row).is_some() {
                    app.ui.drag_origin = Some((m.column, m.row));
                    app.ui.drag_view = Some((app.ui.viewport.view_x, app.ui.viewport.view_y));
                }
            }
            MouseEventKind::Drag(MouseButton::Middle) => {
                if app.screen == Screen::Game {
                    if let (Some((ox, oy)), Some((vx, vy))) = (app.ui.drag_origin, app.ui.drag_view)
                    {
                        let vp = app.ui.viewport;
                        let dx = (ox as i16 - m.column as i16) / vp.tile_w.max(1) as i16;
                        let dy = (oy as i16 - m.row as i16) / vp.tile_h.max(1) as i16;

                        let max_x = app.game.grid_w.saturating_sub(vp.vis_w);
                        let max_y = app.game.grid_h.saturating_sub(vp.vis_h);

                        app.ui.viewport.view_x = (vx as i16 + dx).clamp(0, max_x as i16) as u16;
                        app.ui.viewport.view_y = (vy as i16 + dy).clamp(0, max_y as i16) as u16;
                        app.ui.manual_pan = true;
                    }
                }
            }
            MouseEventKind::Up(MouseButton::Middle) => {
                app.ui.drag_origin = None;
                app.ui.drag_view = None;
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
        ButtonId::StartWave,
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

fn hit_test_multiplayer(app: &App, x: u16, y: u16) -> Option<MultiplayerAction> {
    if app.screen != Screen::Multiplayer {
        return None;
    }
    let hit = &app.ui.multiplayer_hit;
    if contains(hit.create_btn, x, y) {
        return Some(MultiplayerAction::CreateLobby);
    }
    if contains(hit.join_btn, x, y) {
        return Some(MultiplayerAction::JoinLobby);
    }
    if contains(hit.ip_mode_btn, x, y) {
        return Some(MultiplayerAction::ToggleIpMode);
    }
    if contains(hit.copy_ip_btn, x, y) {
        return Some(MultiplayerAction::CopyStunIp);
    }
    if contains(hit.refresh_ip_btn, x, y) {
        return Some(MultiplayerAction::RefreshStun);
    }
    if contains(hit.connect_btn, x, y) {
        return Some(MultiplayerAction::Connect);
    }
    if contains(hit.continue_btn, x, y) {
        return Some(MultiplayerAction::Continue);
    }
    if contains(hit.peer_ip_field, x, y) {
        return Some(MultiplayerAction::FocusPeerIp);
    }
    if contains(hit.name_field, x, y) {
        return Some(MultiplayerAction::FocusName);
    }
    for (idx, rect) in hit.kick_buttons.iter().enumerate() {
        if contains(*rect, x, y) {
            let target = hit.kick_targets.get(idx).copied().unwrap_or(1);
            return Some(MultiplayerAction::KickPlayer(target));
        }
    }
    None
}

fn hit_test_main_menu(app: &App, x: u16, y: u16) -> Option<usize> {
    if app.screen != Screen::MainMenu {
        return None;
    }
    for (idx, rect) in app.ui.main_menu_hit.options.iter().enumerate() {
        if contains(*rect, x, y) {
            return Some(idx);
        }
    }
    None
}

fn hit_test_load_slot(app: &App, x: u16, y: u16) -> Option<usize> {
    if app.screen != Screen::LoadGame {
        return None;
    }
    for (idx, rect) in app.ui.load_menu_hit.slot_items.iter().enumerate() {
        if contains(*rect, x, y) {
            return Some(idx);
        }
    }
    None
}

fn hit_test_load_wave(app: &App, x: u16, y: u16) -> Option<usize> {
    if app.screen != Screen::LoadGame {
        return None;
    }
    for (idx, rect) in app.ui.load_menu_hit.wave_items.iter().enumerate() {
        if contains(*rect, x, y) {
            return Some(idx);
        }
    }
    None
}
