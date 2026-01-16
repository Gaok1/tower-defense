use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::*,
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph, Widget, Wrap},
};

use crate::{
    app::{
        App, ButtonId, HoverAction, LayoutMode, LoadMenuFocus, MapSelectAction, MapSpec,
        MapViewport, ParticleKind, Screen, TOWER_KIND_COUNT, TowerKind,
    },
    assets,
};

// ------------------------------------------------------------
// Theme (discreto / "profissional")
// ------------------------------------------------------------

#[inline]
fn bg() -> Color {
    Color::Rgb(10, 14, 10)
}

#[inline]
fn panel_border() -> Color {
    Color::DarkGray
}

#[inline]
fn panel_title() -> Color {
    Color::LightCyan
}

#[inline]
fn text_dim() -> Color {
    Color::Gray
}

#[inline]
fn accent() -> Color {
    Color::LightCyan
}

#[inline]
fn danger() -> Color {
    Color::LightRed
}

#[inline]
fn warn() -> Color {
    Color::LightYellow
}

#[inline]
fn good() -> Color {
    Color::LightGreen
}

#[inline]
fn map_bg(zoom: u16) -> Color {
    match zoom {
        0 | 1 => Color::Rgb(8, 22, 10),
        2 => Color::Rgb(12, 28, 12),
        3 => Color::Rgb(16, 34, 16),
        _ => Color::Rgb(20, 40, 20),
    }
}

#[inline]
fn path_bg(zoom: u16) -> Color {
    match zoom {
        0 | 1 => Color::Rgb(20, 20, 10),
        2 => Color::Rgb(26, 26, 12),
        3 => Color::Rgb(32, 32, 14),
        _ => Color::Rgb(38, 38, 16),
    }
}

#[inline]
fn goal_bg(zoom: u16) -> Color {
    match zoom {
        0 | 1 => Color::Rgb(20, 12, 26),
        2 => Color::Rgb(26, 16, 32),
        3 => Color::Rgb(32, 20, 38),
        _ => Color::Rgb(38, 24, 44),
    }
}

#[inline]
fn tile_bg(zoom: u16, is_path: bool, is_goal: bool) -> Color {
    if is_goal {
        goal_bg(zoom)
    } else if is_path {
        path_bg(zoom)
    } else {
        map_bg(zoom)
    }
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    app.set_layout_mode_from_size(area);

    f.render_widget(Block::default().style(Style::default().bg(bg())), area);

    match app.screen {
        Screen::MainMenu => draw_main_menu(f, app, area),
        Screen::MapSelect => draw_map_select(f, app, area),
        Screen::LoadGame => draw_load_game(f, app, area),
        Screen::Game => match app.ui.mode {
            LayoutMode::Wide => draw_wide(f, app, area),
            LayoutMode::Compact => draw_compact(f, app, area),
        },
    }
}

fn draw_wide(f: &mut Frame, app: &mut App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    draw_header(f, app, rows[0]);

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(52), Constraint::Length(42)])
        .split(rows[1]);

    draw_map_panel(f, app, main[0]);
    draw_sidebar(f, app, main[1]);
    draw_footer_buttons(f, app, rows[2]);
}

fn draw_compact(f: &mut Frame, app: &mut App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(11),
        ])
        .split(area);

    draw_header(f, app, rows[0]);
    draw_map_panel(f, app, rows[1]);
    app.ui.hit.build_options = [Rect::new(0, 0, 0, 0); TOWER_KIND_COUNT];

    let bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Length(3)])
        .split(rows[2]);

    draw_compact_info(f, app, bottom[0]);
    draw_footer_buttons(f, app, bottom[1]);
}

fn panel_block(title: &str) -> Block<'static> {
    Block::default()
        .title(Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(panel_title())
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(panel_border()))
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(24),
            Constraint::Min(10),
            Constraint::Length(22),
        ])
        .split(area);

    // Brand
    let mut brand = vec![
        Span::styled(
            " TOWER TD ",
            Style::default()
                .fg(Color::Black)
                .bg(accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            if app.game.running { "RUN" } else { "PAUSE" },
            Style::default()
                .fg(if app.game.running { good() } else { warn() })
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if app.dev_mode {
        brand.push(Span::raw("  "));
        brand.push(Span::styled(
            "DEV",
            Style::default()
                .fg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let title = Paragraph::new(Line::from(brand))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(panel_border())),
        )
        .style(Style::default().bg(bg()));
    f.render_widget(title, cols[0]);

    // Center: wave progress
    let pct = app.wave_progress_percent();
    let wave = Gauge::default()
        .block(
            Block::default()
                .title(Span::styled(
                    format!(" Wave {} ", app.game.wave),
                    Style::default()
                        .fg(panel_title())
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(panel_border())),
        )
        .gauge_style(Style::default().fg(accent()))
        .percent(pct);
    f.render_widget(wave, cols[1]);

    // Right: stats
    let stats = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(
                "$ {}",
                if app.dev_mode {
                    "∞".to_string()
                } else {
                    app.game.money.to_string()
                }
            ),
            Style::default().fg(warn()),
        ),
        Span::raw("  "),
        Span::styled(
            format!(
                "HP {}",
                if app.dev_mode {
                    "∞".to_string()
                } else {
                    app.game.lives.to_string()
                }
            ),
            Style::default().fg(danger()),
        ),
        Span::raw("  "),
        Span::styled(
            format!("x{}", app.game.speed),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(panel_border())),
    )
    .style(Style::default().bg(bg()));
    f.render_widget(stats, cols[2]);
}

fn draw_map_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let title = format!("Map — {}  Zoom {}", app.game.map_name, app.ui.zoom);
    let block = panel_block(&title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Reserva 1 linha pro help (não sobrepõe o mapa)
    let (map_area, hint_area) = if inner.height >= 2 {
        (
            Rect {
                x: inner.x,
                y: inner.y,
                width: inner.width,
                height: inner.height - 1,
            },
            Rect {
                x: inner.x,
                y: inner.y + inner.height - 1,
                width: inner.width,
                height: 1,
            },
        )
    } else {
        (inner, Rect::new(0, 0, 0, 0))
    };

    app.ui.hit.map_inner = map_area;
    app.ui.viewport = compute_viewport(app, map_area);

    f.render_widget(MapWidget { app }, map_area);

    if hint_area.height == 1 {
        let hint = Paragraph::new(Line::from(vec![
            Span::styled("Mouse", Style::default().fg(text_dim())),
            Span::raw(": select / place  "),
            Span::styled("Right drag", Style::default().fg(text_dim())),
            Span::raw(": pan  "),
            Span::styled("Space", Style::default().fg(text_dim())),
            Span::raw(": start/pause  "),
            Span::styled("B", Style::default().fg(text_dim())),
            Span::raw(": build  "),
            Span::styled("1-6", Style::default().fg(text_dim())),
            Span::raw(": select tower  "),
            Span::styled("U", Style::default().fg(text_dim())),
            Span::raw(": upgrade  "),
            Span::styled("S", Style::default().fg(text_dim())),
            Span::raw(": sell  "),
            Span::styled("F", Style::default().fg(text_dim())),
            Span::raw(": speed  "),
            Span::styled("+/-", Style::default().fg(text_dim())),
            Span::raw(": zoom  "),
            Span::styled("Q", Style::default().fg(text_dim())),
            Span::raw(": quit"),
        ]))
        .style(Style::default().fg(text_dim()).bg(map_bg(app.ui.zoom)));
        f.render_widget(hint, hint_area);
    }
}

fn compute_viewport(app: &mut App, inner: Rect) -> MapViewport {
    let mut vp = MapViewport::default();

    let z = app.ui.zoom;
    let tile = if z == 0 { 2 } else { 4 * z };
    vp.tile_w = tile;
    vp.tile_h = tile;

    vp.vis_w = (inner.width / vp.tile_w).max(1).min(app.game.grid_w);
    vp.vis_h = (inner.height / vp.tile_h).max(1).min(app.game.grid_h);

    let max_x = app.game.grid_w.saturating_sub(vp.vis_w);
    let max_y = app.game.grid_h.saturating_sub(vp.vis_h);

    // Se mudou o zoom, tenta manter “âncora” no hover (ou selected)
    if app.ui.last_zoom != app.ui.zoom {
        app.ui.last_zoom = app.ui.zoom;
        app.ui.manual_pan = true;

        if let Some((ax, ay)) = app.ui.hover_cell.or(app.game.selected_cell) {
            let nx = ax.saturating_sub(vp.vis_w / 2).min(max_x);
            let ny = ay.saturating_sub(vp.vis_h / 2).min(max_y);
            vp.view_x = nx;
            vp.view_y = ny;
            return vp;
        }
    }

    let mut vx = app.ui.viewport.view_x.min(max_x);
    let mut vy = app.ui.viewport.view_y.min(max_y);

    if !app.ui.manual_pan {
        if app.ui.viewport.view_x == 0 && app.ui.viewport.view_y == 0 {
            vx = max_x / 2;
            vy = max_y / 2;
        } else if let Some((cx, cy)) = app.game.selected_cell {
            if cx < vx {
                vx = cx;
            } else if cx >= vx + vp.vis_w {
                vx = cx.saturating_sub(vp.vis_w - 1);
            }
            if cy < vy {
                vy = cy;
            } else if cy >= vy + vp.vis_h {
                vy = cy.saturating_sub(vp.vis_h - 1);
            }
        }
    }

    vp.view_x = vx.min(max_x);
    vp.view_y = vy.min(max_y);
    vp
}

fn draw_sidebar(f: &mut Frame, app: &mut App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Min(8),
            Constraint::Length(9),
        ])
        .split(area);

    draw_build_panel(f, app, rows[0]);
    draw_inspector_panel(f, app, rows[1]);
    draw_evolutions_panel(f, rows[2]);
}

fn draw_build_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let block = panel_block("Build Selector");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let towers = App::available_towers();
    let row_constraints = vec![Constraint::Length(1); towers.len()];
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(inner);
    for (i, kind) in towers.iter().enumerate() {
        app.ui.hit.build_options[i] = rows[i];

        let is_active = app.game.build_kind == Some(*kind);
        let is_hover = app.ui.hover_build_kind == Some(*kind);
        let label = format!(
            "{}. {:<6} ${}",
            i + 1,
            tower_kind_label(*kind),
            App::tower_cost(*kind)
        );

        let style = if is_active {
            Style::default()
                .fg(Color::Black)
                .bg(tower_kind_color(*kind))
        } else if is_hover {
            Style::default().fg(Color::Black).bg(accent())
        } else {
            Style::default().fg(Color::White).bg(bg())
        };

        f.render_widget(
            Paragraph::new(label)
                .alignment(Alignment::Left)
                .style(style),
            rows[i],
        );
    }
}

fn draw_inspector_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let block = panel_block("Inspector");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(inner);

    let sel = app
        .game
        .selected_cell
        .map(|(x, y)| format!("Selected: ({x},{y})"))
        .unwrap_or_else(|| "Selected: -".to_string());

    let top = Paragraph::new(vec![
        Line::from(sel),
        Line::from(Span::styled(
            "Tip: hover Upgrade to preview",
            Style::default().fg(text_dim()),
        )),
    ])
    .style(Style::default().fg(Color::White).bg(bg()));
    f.render_widget(top, rows[0]);

    // Stats
    let mut stats_lines: Vec<Line> = Vec::new();
    if let Some(t) = app.selected_tower() {
        let s = App::tower_stats(t);
        stats_lines.push(Line::from(vec![
            Span::styled("Type", Style::default().fg(text_dim())),
            Span::raw(": "),
            Span::styled(
                tower_kind_label(t.kind),
                Style::default()
                    .fg(tower_kind_color(t.kind))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled(
                format!("Lv {}", t.level),
                Style::default().fg(accent()).add_modifier(Modifier::BOLD),
            ),
        ]));

        stats_lines.push(Line::from(""));

        let upgrade_hover = app.ui.hover_button == Some(ButtonId::Upgrade)
            || app.ui.hover_action == Some(HoverAction::UpgradePreview);
        let d = app.upgrade_delta(t);

        stats_lines.push(line_stat_i32(
            "Attack",
            s.attack,
            upgrade_hover.then_some(d.attack),
        ));
        stats_lines.push(line_stat_u16(
            "Range",
            s.range,
            upgrade_hover.then_some(d.range),
        ));
        stats_lines.push(line_stat_u16_inv(
            "Fire CD",
            s.fire_cd,
            upgrade_hover.then_some(d.fire_cd),
        ));

        let effect_line = tower_effect_line(t.kind, t.level, upgrade_hover);
        stats_lines.push(Line::from(""));
        stats_lines.push(effect_line);

        stats_lines.push(Line::from(""));
        stats_lines.push(Line::from(vec![
            Span::styled("Build", Style::default().fg(text_dim())),
            Span::raw(format!(
                ": {}   ",
                app.game
                    .build_kind
                    .map(App::tower_cost)
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "-".to_string())
            )),
            Span::styled("Upgrade", Style::default().fg(text_dim())),
            Span::raw(format!(": {}   ", App::tower_upgrade_cost(t.kind))),
            Span::styled("Sell", Style::default().fg(text_dim())),
            Span::raw(": +20"),
        ]));
    } else {
        stats_lines.push(Line::from("No tower selected."));
        if let Some(kind) = app.game.build_kind {
            stats_lines.push(Line::from(vec![
                Span::styled("Build", Style::default().fg(text_dim())),
                Span::raw(": "),
                Span::styled(
                    tower_kind_label(kind),
                    Style::default()
                        .fg(tower_kind_color(kind))
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            if let Some(preview) = app.build_preview_stats() {
                stats_lines.push(Line::from(""));
                stats_lines.push(Line::from(vec![
                    Span::styled("Attack", Style::default().fg(text_dim())),
                    Span::raw(format!(": {}", preview.attack)),
                    Span::raw("   "),
                    Span::styled("Range", Style::default().fg(text_dim())),
                    Span::raw(format!(": {}", preview.range)),
                    Span::raw("   "),
                    Span::styled("CD", Style::default().fg(text_dim())),
                    Span::raw(format!(": {}", preview.fire_cd)),
                ]));
                stats_lines.push(Line::from(""));
                stats_lines.push(tower_effect_line(kind, 1, false));
            }

            stats_lines.push(Line::from(Span::styled(
                "Click twice on a tile or press Build.",
                Style::default().fg(text_dim()),
            )));
        } else {
            stats_lines.push(Line::from(Span::styled(
                "Select a tower type (1-6) to build.",
                Style::default().fg(text_dim()),
            )));
        }
    }

    f.render_widget(
        Paragraph::new(stats_lines)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::White).bg(bg())),
        rows[1],
    );

    // Actions
    let actions = rows[2];
    let action_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(actions);

    app.ui.hit.inspector_upgrade = action_rows[0];

    let upgrade_hover = app.ui.hover_action == Some(HoverAction::UpgradePreview)
        || app.ui.hover_button == Some(ButtonId::Upgrade);

    let upgrade_style = if upgrade_hover {
        Style::default().fg(Color::Black).bg(accent())
    } else {
        Style::default().fg(panel_title()).bg(bg())
    };

    let upgrade_cost = app
        .selected_tower()
        .map(|t| App::tower_upgrade_cost(t.kind))
        .unwrap_or(0);
    let upgrade_text = if app.selected_tower().is_some() {
        format!("Upgrade [U] ({upgrade_cost})  — hover preview")
    } else {
        "Upgrade [U] (—)  — select a tower".to_string()
    };

    f.render_widget(
        Paragraph::new(upgrade_text)
            .style(upgrade_style)
            .alignment(Alignment::Left),
        action_rows[0],
    );

    f.render_widget(
        Paragraph::new("Sell [S] (+20)")
            .style(Style::default().fg(warn()).bg(bg()))
            .alignment(Alignment::Left),
        action_rows[1],
    );

    let build_label = if let Some(kind) = app.game.build_kind {
        if let Some((x, y)) = app.game.selected_cell {
            if app.can_build_at(x, y, kind) {
                format!(
                    "Build [B] ({}) — double click or press Build",
                    App::tower_cost(kind)
                )
            } else if app.is_path(x, y) || app.tower_index_at(x, y).is_some() {
                format!("Build [B] ({}) — blocked tile", App::tower_cost(kind))
            } else {
                format!("Build [B] ({}) — need more $", App::tower_cost(kind))
            }
        } else {
            format!("Build [B] ({}) — select tile", App::tower_cost(kind))
        }
    } else {
        "Build [B] (—) — select tower type".to_string()
    };

    let build_style = if app.game.build_kind.is_some() {
        Style::default().fg(good()).bg(bg())
    } else {
        Style::default().fg(text_dim()).bg(bg())
    };

    f.render_widget(
        Paragraph::new(build_label)
            .style(build_style)
            .alignment(Alignment::Left),
        action_rows[2],
    );
}

fn line_stat_i32(label: &str, value: i32, delta: Option<i32>) -> Line<'static> {
    let mut spans = vec![
        Span::styled(format!("{label}: "), Style::default().fg(text_dim())),
        Span::styled(format!("{value}"), Style::default().fg(Color::White)),
    ];
    if let Some(d) = delta {
        if d != 0 {
            spans.push(Span::styled(
                format!(" + {d}"),
                Style::default().fg(good()).add_modifier(Modifier::BOLD),
            ));
        }
    }
    Line::from(spans)
}

fn line_stat_u16(label: &str, value: u16, delta: Option<i16>) -> Line<'static> {
    let mut spans = vec![
        Span::styled(format!("{label}: "), Style::default().fg(text_dim())),
        Span::styled(format!("{value}"), Style::default().fg(Color::White)),
    ];
    if let Some(d) = delta {
        if d != 0 {
            let sign = if d > 0 { "+" } else { "-" };
            spans.push(Span::styled(
                format!(" {sign} {}", d.abs()),
                Style::default().fg(good()).add_modifier(Modifier::BOLD),
            ));
        }
    }
    Line::from(spans)
}

// Fire CD: delta negativo é buff (atira mais rápido), mas continua verde.
fn line_stat_u16_inv(label: &str, value: u16, delta: Option<i16>) -> Line<'static> {
    let mut spans = vec![
        Span::styled(format!("{label}: "), Style::default().fg(text_dim())),
        Span::styled(format!("{value}"), Style::default().fg(Color::White)),
    ];
    if let Some(d) = delta {
        if d != 0 {
            let sign = if d > 0 { "+" } else { "-" };
            spans.push(Span::styled(
                format!(" {sign} {}", d.abs()),
                Style::default().fg(good()).add_modifier(Modifier::BOLD),
            ));
        }
    }
    Line::from(spans)
}

fn draw_compact_info(f: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("Info");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let sel = app
        .game
        .selected_cell
        .map(|(x, y)| format!("Selected: ({x},{y})"))
        .unwrap_or_else(|| "Selected: -".to_string());

    let txt = vec![
        Line::from(format!(
            "Wave {} | Lives {} | $ {} | Speed x{} | Zoom {}{}",
            app.game.wave,
            if app.dev_mode {
                "∞".to_string()
            } else {
                app.game.lives.to_string()
            },
            if app.dev_mode {
                "∞".to_string()
            } else {
                app.game.money.to_string()
            },
            app.game.speed,
            app.ui.zoom,
            if app.dev_mode { " | DEV" } else { "" }
        )),
        Line::from(sel),
        Line::from(format!(
            "Build: {} ({}). Double click or press B. Switch: 1 Basic • 2 Sniper • 3 Rapid • 4 Cannon • 5 Tesla • 6 Frost",
            app.game.build_kind.map(tower_kind_label).unwrap_or("-"),
            app.game
                .build_kind
                .map(App::tower_cost)
                .map(|c| c.to_string())
                .unwrap_or_else(|| "-".to_string())
        )),
        Line::from(Span::styled(
            "Space start/pause • B build • U upgrade • S sell • F speed • Q quit",
            Style::default().fg(text_dim()),
        )),
        Line::from(Span::styled(
            "Evoluções: Cannon • Tesla • Frost • Mapas temáticos • FX por tipo",
            Style::default().fg(text_dim()),
        )),
    ];

    f.render_widget(
        Paragraph::new(txt)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::White).bg(bg())),
        inner,
    );
}

fn draw_evolutions_panel(f: &mut Frame, area: Rect) {
    let block = panel_block("Evoluções");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(Span::styled(
            "Próximos passos sugeridos",
            Style::default().fg(panel_title()),
        )),
        Line::from("• Torre Canhão: tiro pesado e FX de impacto"),
        Line::from("• Torre Tesla: arco elétrico com faíscas"),
        Line::from("• Torre Frost: estilhaços e controle"),
        Line::from("• Mapas temáticos com rotas largas"),
        Line::from("• Modo infinito + placar de tempo"),
    ];

    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::White).bg(bg())),
        inner,
    );
}

fn draw_footer_buttons(f: &mut Frame, app: &mut App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .split(area);

    let defs: [(ButtonId, String); 6] = [
        (
            ButtonId::StartPause,
            if app.game.running {
                "Pause [Space]".to_string()
            } else {
                "Start [Space]".to_string()
            },
        ),
        (ButtonId::Build, "Build [B]".to_string()),
        (ButtonId::Upgrade, "Upgrade [U]".to_string()),
        (ButtonId::Sell, "Sell [S]".to_string()),
        (ButtonId::Speed, format!("Speed [F] x{}", app.game.speed)),
        (ButtonId::Quit, "Quit [Q]".to_string()),
    ];

    for (i, (id, label)) in defs.iter().enumerate() {
        let hovered = app.ui.hover_button == Some(*id);
        let active = *id == ButtonId::StartPause && app.game.running;

        let base = if hovered {
            Style::default().fg(Color::Black).bg(accent())
        } else if active {
            Style::default().fg(Color::Black).bg(good())
        } else {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        };

        let w = Paragraph::new(label.clone())
            .alignment(Alignment::Center)
            .style(base)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(panel_border())),
            );

        f.render_widget(w, cols[i]);
        app.ui.hit.buttons[i] = cols[i];
    }
}

// ------------------------------------------------------------
// Map Widget
// ------------------------------------------------------------
#[inline]
fn hash32(mut v: u32) -> u32 {
    // hash simples e rápido (bom o suficiente pra textura procedural)
    v ^= v >> 16;
    v = v.wrapping_mul(0x7feb352d);
    v ^= v >> 15;
    v = v.wrapping_mul(0x846ca68b);
    v ^= v >> 16;
    v
}

#[inline]
fn tex_pick(cell_x: u16, cell_y: u16, dx: u16, dy: u16, salt: u32, m: u32) -> u32 {
    let v = (cell_x as u32)
        ^ ((cell_y as u32) << 11)
        ^ ((dx as u32) << 21)
        ^ ((dy as u32) << 27)
        ^ salt;
    hash32(v) % m
}

fn draw_sprite(
    buf: &mut Buffer,
    tile_x: u16,
    tile_y: u16,
    tile_w: u16,
    tile_h: u16,
    sprite: assets::Sprite,
    style: Style,
) {
    let h = sprite.h.min(tile_h) as usize;
    let w = sprite.w.min(tile_w) as usize;
    for sy in 0..h {
        let row = sprite.row(sy);
        for (sx, ch) in row.chars().take(w).enumerate() {
            if ch == ' ' {
                continue;
            }
            let x = tile_x + sx as u16;
            let y = tile_y + sy as u16;
            if x >= tile_x + tile_w || y >= tile_y + tile_h {
                continue;
            }
            let mut tmp = [0u8; 4];
            let sym = ch.encode_utf8(&mut tmp);
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol(sym).set_style(style);
            }
        }
    }
}

struct MapWidget<'a> {
    app: &'a App,
}

impl<'a> Widget for MapWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let app = self.app;
        let vp = app.ui.viewport;

        if area.width == 0 || area.height == 0 || vp.vis_w == 0 || vp.vis_h == 0 {
            return;
        }

        let range_focus = range_focus(app);
        let goal = app.game.path.last().copied();

        // fundo do painel do mapa
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_symbol(" ")
                        .set_style(Style::default().bg(map_bg(app.ui.zoom)));
                }
            }
        }

        // tiles + entidades
        for gy in 0..vp.vis_h {
            for gx in 0..vp.vis_w {
                let cell_x = vp.view_x + gx;
                let cell_y = vp.view_y + gy;

                let tile_x = area.x + gx * vp.tile_w;
                let tile_y = area.y + gy * vp.tile_h;

                let is_goal = goal == Some((cell_x, cell_y));
                let is_path = app.is_path(cell_x, cell_y);

                let base_bg = tile_bg(app.ui.zoom, is_path, is_goal);

                // highlight (seleção/hover/range)
                let mut hl_bg = base_bg;
                if app.game.selected_cell == Some((cell_x, cell_y)) {
                    hl_bg = Color::Blue;
                } else if app.ui.hover_cell == Some((cell_x, cell_y)) {
                    hl_bg = Color::DarkGray;
                } else if let Some((rx, ry, range, shape)) = range_focus {
                    if range_match(shape, cell_x, cell_y, rx, ry, range) {
                        hl_bg = Color::Blue;
                    }
                }

                // textura procedural do terreno (preenche o retângulo inteiro do tile)
                for dy in 0..vp.tile_h {
                    for dx in 0..vp.tile_w {
                        let (sym, fg) = if is_goal {
                            let cx = vp.tile_w / 2;
                            let cy = vp.tile_h / 2;
                            if dx == cx && dy == cy {
                                ("◎", Color::LightMagenta)
                            } else {
                                let k = tex_pick(cell_x, cell_y, dx, dy, 0xBEEF_u32, 3);
                                (["░", "▒", "▓"][k as usize], Color::Magenta)
                            }
                        } else if is_path {
                            let k = tex_pick(cell_x, cell_y, dx, dy, 0xCAFE_u32, 5);
                            (["·", "░", "▒", "░", "·"][k as usize], Color::LightYellow)
                        } else {
                            let k = tex_pick(cell_x, cell_y, dx, dy, 0x1234_u32, 7);
                            (
                                [" ", "·", "˙", "✿", "˙", "·", " "][k as usize],
                                Color::Green,
                            )
                        };

                        let x = tile_x + dx;
                        let y = tile_y + dy;
                        if x < area.right() && y < area.bottom() {
                            if let Some(cell) = buf.cell_mut((x, y)) {
                                cell.set_symbol(sym)
                                    .set_style(Style::default().fg(fg).bg(hl_bg));
                            }
                        }
                    }
                }

                // build preview (só no tile alvo)
                if let Some(kind) = app.game.build_kind {
                    if let Some((sx, sy)) = app.game.selected_cell {
                        if (sx, sy) == (cell_x, cell_y) && app.can_build_at(cell_x, cell_y, kind) {
                            let spr = assets::tower_sprite(kind, app.ui.zoom);
                            let st = Style::default()
                                .fg(tower_kind_color(kind))
                                .bg(hl_bg)
                                .add_modifier(Modifier::DIM);
                            draw_sprite(buf, tile_x, tile_y, vp.tile_w, vp.tile_h, spr, st);
                        }
                    }
                }

                // torre real
                if let Some(ti) = app.tower_index_at(cell_x, cell_y) {
                    let t = &app.game.towers[ti];
                    let spr = assets::tower_sprite(t.kind, app.ui.zoom);

                    let st = Style::default()
                        .fg(if t.level >= 4 {
                            warn()
                        } else {
                            tower_kind_color(t.kind)
                        })
                        .bg(hl_bg)
                        .add_modifier(Modifier::BOLD);

                    draw_sprite(buf, tile_x, tile_y, vp.tile_w, vp.tile_h, spr, st);
                }

                // inimigo por cima
                if app.enemy_at(cell_x, cell_y) {
                    let spr = assets::enemy_sprite(app.ui.zoom);
                    let st = Style::default()
                        .fg(Color::Red)
                        .bg(hl_bg)
                        .add_modifier(Modifier::BOLD);
                    draw_sprite(buf, tile_x, tile_y, vp.tile_w, vp.tile_h, spr, st);
                }

                // impacto por cima de tudo
                if let Some(fx) = app
                    .game
                    .impacts
                    .iter()
                    .find(|fx| fx.x == cell_x && fx.y == cell_y)
                {
                    let spr = assets::impact_sprite(app.ui.zoom);
                    let base_color = tower_kind_color(fx.kind);
                    let st = match fx.ttl {
                        4 => Style::default()
                            .fg(base_color)
                            .bg(hl_bg)
                            .add_modifier(Modifier::BOLD),
                        3 => Style::default()
                            .fg(base_color)
                            .bg(hl_bg)
                            .add_modifier(Modifier::DIM),
                        2 => Style::default().fg(Color::DarkGray).bg(hl_bg),
                        _ => Style::default().fg(text_dim()).bg(hl_bg),
                    };
                    draw_sprite(buf, tile_x, tile_y, vp.tile_w, vp.tile_h, spr, st);
                }
            }
        }

        // projéteis (agora no “meio” do tile em X e Y)
        for p in &app.game.projectiles {
            if p.x < 0 || p.y < 0 {
                continue;
            }
            let cx = p.x as u16;
            let cy = p.y as u16;
            if cx < vp.view_x || cy < vp.view_y {
                continue;
            }
            let gx = cx - vp.view_x;
            let gy = cy - vp.view_y;
            if gx >= vp.vis_w || gy >= vp.vis_h {
                continue;
            }

            let tile_x = area.x + gx * vp.tile_w;
            let tile_y = area.y + gy * vp.tile_h;

            let mid_x = tile_x + (vp.tile_w / 2).min(vp.tile_w.saturating_sub(1));
            let mid_y = tile_y + (vp.tile_h / 2).min(vp.tile_h.saturating_sub(1));

            let dx = (p.tx - p.x).signum();
            let dy = (p.ty - p.y).signum();

            let (sym, color) = match p.kind {
                TowerKind::Sniper => {
                    // Sniper: traçante direcional.
                    let s = match (dx, dy) {
                        (1, 0) | (-1, 0) => assets::GLYPH_TRACER_H,
                        (0, 1) | (0, -1) => assets::GLYPH_TRACER_V,
                        (1, 1) | (-1, -1) => assets::GLYPH_TRACER_D1,
                        (1, -1) | (-1, 1) => assets::GLYPH_TRACER_D2,
                        _ => assets::GLYPH_PROJECTILE_SNIPER,
                    };
                    (s, Color::LightCyan)
                }
                TowerKind::Rapid => (assets::GLYPH_PROJECTILE_RAPID, Color::Yellow),
                TowerKind::Cannon => (assets::GLYPH_PROJECTILE_CANNON, Color::LightRed),
                TowerKind::Tesla => (assets::GLYPH_PROJECTILE_TESLA, Color::LightBlue),
                TowerKind::Frost => (assets::GLYPH_PROJECTILE_FROST, Color::LightBlue),
                TowerKind::Basic => (assets::GLYPH_PROJECTILE_BASIC, Color::LightMagenta),
            };

            if mid_x < area.right() && mid_y < area.bottom() {
                if let Some(cell) = buf.cell_mut((mid_x, mid_y)) {
                    let style = cell.style().fg(color).add_modifier(Modifier::BOLD);
                    cell.set_symbol(sym).set_style(style);
                }
            }
        }

        // partículas (também centralizadas em X/Y)
        for p in &app.game.particles {
            if p.x < 0 || p.y < 0 {
                continue;
            }
            let cx = p.x as u16;
            let cy = p.y as u16;
            if cx < vp.view_x || cy < vp.view_y {
                continue;
            }
            let gx = cx - vp.view_x;
            let gy = cy - vp.view_y;
            if gx >= vp.vis_w || gy >= vp.vis_h {
                continue;
            }

            let tile_x = area.x + gx * vp.tile_w;
            let tile_y = area.y + gy * vp.tile_h;

            let mid_x = tile_x + (vp.tile_w / 2).min(vp.tile_w.saturating_sub(1));
            let mid_y = tile_y + (vp.tile_h / 2).min(vp.tile_h.saturating_sub(1));

            let (sym, color, modifier) = particle_visual(p.kind, p.ttl);

            if mid_x < area.right() && mid_y < area.bottom() {
                if let Some(cell) = buf.cell_mut((mid_x, mid_y)) {
                    let style = cell.style().fg(color).add_modifier(modifier);
                    cell.set_symbol(sym).set_style(style);
                }
            }
        }
    }
}

fn draw_main_menu(f: &mut Frame, app: &mut App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    let header = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            " TOWER TD ",
            Style::default()
                .fg(Color::Black)
                .bg(accent())
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(Span::styled(
            "Terminal Tower Defense",
            Style::default().fg(text_dim()),
        )),
    ])
    .alignment(Alignment::Center)
    .block(panel_block("Menu"))
    .style(Style::default().bg(bg()));
    f.render_widget(header, rows[0]);

    let block = panel_block("Start");
    let inner = block.inner(rows[1]);
    f.render_widget(block, rows[1]);

    let options = ["New game", "Load saved game"];
    let mut lines: Vec<Line> = Vec::new();
    for (i, label) in options.iter().enumerate() {
        let selected = app.main_menu_index == i;
        let style = if selected {
            Style::default()
                .fg(Color::Black)
                .bg(accent())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).bg(bg())
        };
        lines.push(Line::from(Span::styled(format!("  {label}  "), style)));
    }

    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(Style::default().bg(bg())),
        inner,
    );

    let footer = Paragraph::new(Line::from(Span::styled(
        "Up/Down select  •  Enter confirm  •  Q quit",
        Style::default().fg(text_dim()),
    )))
    .alignment(Alignment::Center)
    .block(panel_block("Help"))
    .style(Style::default().bg(bg()));
    f.render_widget(footer, rows[2]);
}

fn draw_load_game(f: &mut Frame, app: &mut App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " LOAD GAME ",
            Style::default()
                .fg(Color::Black)
                .bg(accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("↑/↓", Style::default().fg(text_dim())),
        Span::raw(" move  "),
        Span::styled("←/→", Style::default().fg(text_dim())),
        Span::raw(" focus  "),
        Span::styled("Enter", Style::default().fg(text_dim())),
        Span::raw(" load"),
    ]))
    .alignment(Alignment::Center)
    .block(panel_block("Saves"))
    .style(Style::default().bg(bg()));
    f.render_widget(header, rows[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(rows[1]);

    let focus_slots = app.load_menu.focus == LoadMenuFocus::Slots;
    let focus_waves = app.load_menu.focus == LoadMenuFocus::Waves;

    // ---------------------- slots ----------------------
    let slots_title = if focus_slots {
        "Save Slots *"
    } else {
        "Save Slots"
    };
    let block = panel_block(slots_title);
    let inner = block.inner(cols[0]);
    f.render_widget(block, cols[0]);

    let mut slot_lines: Vec<Line> = Vec::new();
    if app.load_menu.slots.is_empty() {
        slot_lines.push(Line::from(Span::styled(
            "No saves found",
            Style::default().fg(text_dim()),
        )));
    } else {
        for (i, slot) in app.load_menu.slots.iter().enumerate() {
            let selected = i == app.load_menu.selected_slot;
            let style = if selected && focus_slots {
                Style::default()
                    .fg(Color::Black)
                    .bg(accent())
                    .add_modifier(Modifier::BOLD)
            } else if selected {
                Style::default().fg(Color::Black).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White).bg(bg())
            };

            let last_wave = slot.waves.last().copied().unwrap_or(0);
            let dev_tag = if slot.dev_mode { " DEV" } else { "" };
            let short_id = &slot.id[..slot.id.len().min(8)];

            slot_lines.push(Line::from(Span::styled(
                format!(" {short_id}  {}  wave {last_wave}{dev_tag}", slot.map_name),
                style,
            )));
        }
    }

    f.render_widget(
        Paragraph::new(slot_lines)
            .wrap(Wrap { trim: true })
            .style(Style::default().bg(bg())),
        inner,
    );

    // ---------------------- waves ----------------------
    let waves_title = if focus_waves { "Waves *" } else { "Waves" };
    let block = panel_block(waves_title);
    let inner = block.inner(cols[1]);
    f.render_widget(block, cols[1]);

    let mut wave_lines: Vec<Line> = Vec::new();
    let selected_slot = app.load_menu.slots.get(app.load_menu.selected_slot);
    let waves = selected_slot.map(|s| s.waves.as_slice()).unwrap_or(&[]);

    if waves.is_empty() {
        wave_lines.push(Line::from(Span::styled(
            "No wave checkpoints",
            Style::default().fg(text_dim()),
        )));
    } else {
        for (i, w) in waves.iter().enumerate() {
            let selected = i == app.load_menu.selected_wave;
            let style = if selected && focus_waves {
                Style::default()
                    .fg(Color::Black)
                    .bg(accent())
                    .add_modifier(Modifier::BOLD)
            } else if selected {
                Style::default().fg(Color::Black).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White).bg(bg())
            };

            wave_lines.push(Line::from(Span::styled(format!(" Wave {w} "), style)));
        }
    }

    f.render_widget(
        Paragraph::new(wave_lines)
            .wrap(Wrap { trim: true })
            .style(Style::default().bg(bg())),
        inner,
    );

    // ---------------------- footer ----------------------
    let footer_text = if let Some(err) = app.load_menu.error.as_ref() {
        Line::from(vec![
            Span::styled(
                "Error: ",
                Style::default().fg(danger()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(err.clone(), Style::default().fg(danger())),
        ])
    } else {
        Line::from(Span::styled(
            "Esc back  •  R refresh  •  Tab focus  •  Enter load  •  Q quit",
            Style::default().fg(text_dim()),
        ))
    };

    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .block(panel_block("Help"))
        .style(Style::default().bg(bg()));
    f.render_widget(footer, rows[2]);
}

fn draw_map_select(f: &mut Frame, app: &mut App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(5),
        ])
        .split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " SELECT MAP ",
            Style::default()
                .fg(Color::Black)
                .bg(accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("←/→", Style::default().fg(text_dim())),
        Span::raw(" navigate  "),
        Span::styled("Enter", Style::default().fg(text_dim())),
        Span::raw(" play"),
    ]))
    .alignment(Alignment::Center)
    .block(panel_block("Map Browser"))
    .style(Style::default().bg(bg()));
    f.render_widget(header, rows[0]);

    let preview = panel_block("Preview");
    let inner = preview.inner(rows[1]);
    f.render_widget(preview, rows[1]);

    f.render_widget(
        MapPreviewWidget {
            map: app.selected_map(),
        },
        inner,
    );

    let footer_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(12),
            Constraint::Min(10),
            Constraint::Length(12),
        ])
        .split(rows[2]);

    let center_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(2)])
        .split(footer_cols[1]);

    let left_style = if app.ui.hover_map_select == Some(MapSelectAction::Prev) {
        Style::default().fg(Color::Black).bg(accent())
    } else {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    };
    let right_style = if app.ui.hover_map_select == Some(MapSelectAction::Next) {
        Style::default().fg(Color::Black).bg(accent())
    } else {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    };
    let start_style = if app.ui.hover_map_select == Some(MapSelectAction::Start) {
        Style::default().fg(Color::Black).bg(good())
    } else {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    };

    f.render_widget(
        Paragraph::new(" ◀ Prev ")
            .alignment(Alignment::Center)
            .style(left_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(panel_border())),
            ),
        footer_cols[0],
    );
    app.ui.hit.map_select_left = footer_cols[0];

    let map = app.selected_map();
    let info = Paragraph::new(vec![
        Line::from(Span::styled(
            map.name,
            Style::default()
                .fg(panel_title())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "{}x{} • path {} tiles • map {} of {}",
            map.grid_w,
            map.grid_h,
            map.path.len(),
            app.selected_map_index() + 1,
            app.maps_len()
        )),
    ])
    .alignment(Alignment::Center)
    .style(Style::default().bg(bg()));
    f.render_widget(info, center_rows[0]);

    f.render_widget(
        Paragraph::new(" Play ▶ ")
            .alignment(Alignment::Center)
            .style(start_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(panel_border())),
            ),
        center_rows[1],
    );
    app.ui.hit.map_select_start = center_rows[1];

    f.render_widget(
        Paragraph::new(" Next ▶ ")
            .alignment(Alignment::Center)
            .style(right_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(panel_border())),
            ),
        footer_cols[2],
    );
    app.ui.hit.map_select_right = footer_cols[2];
}

struct MapPreviewWidget<'a> {
    map: &'a MapSpec,
}

impl<'a> Widget for MapPreviewWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        #[cfg(any())]
        {
        let z = self.zoom.max(1);
        let tile_w = 4 * z;
        let tile_h = 4 * z;

        let vis_w = (area.width / tile_w).max(1).min(self.map.grid_w);
        let vis_h = (area.height / tile_h).max(1).min(self.map.grid_h);

        let view_x = self.map.grid_w.saturating_sub(vis_w) / 2;
        let view_y = self.map.grid_h.saturating_sub(vis_h) / 2;

        let goal = self.map.path.last().copied();

        // fundo
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_symbol(" ")
                        .set_style(Style::default().bg(map_bg(z)));
                }
            }
        }

        for gy in 0..vis_h {
            for gx in 0..vis_w {
                let cell_x = view_x + gx;
                let cell_y = view_y + gy;

                let tile_x = area.x + gx * tile_w;
                let tile_y = area.y + gy * tile_h;

                let is_goal = goal == Some((cell_x, cell_y));
                let is_path = self
                    .map
                    .path
                    .iter()
                    .any(|&(px, py)| px == cell_x && py == cell_y);

                let base_bg = tile_bg(z, is_path, is_goal);

                for dy in 0..tile_h {
                    for dx in 0..tile_w {
                        let (sym, fg) = if is_goal {
                            let cx = tile_w / 2;
                            let cy = tile_h / 2;
                            if dx == cx && dy == cy {
                                ("◎", Color::LightMagenta)
                            } else {
                                ("▓", Color::Magenta)
                            }
                        } else if is_path {
                            ("▚", Color::LightYellow)
                        } else {
                            let k = tex_pick(cell_x, cell_y, dx, dy, 0x1234_u32, 5);
                            ([" ", "·", "˙", "·", " "][k as usize], Color::Green)
                        };

                        let x = tile_x + dx;
                        let y = tile_y + dy;
                        if x < area.right() && y < area.bottom() {
                            if let Some(cell) = buf.cell_mut((x, y)) {
                                cell.set_symbol(sym)
                                    .set_style(Style::default().fg(fg).bg(base_bg));
                            }
                        }
                    }
                }
            }
        }
        }

        // Preview do mapa precisa sempre caber na tela. Aqui renderiza um minimap
        // do formato do caminho (o que importa na seleção), escalando para `area`.
        let map_w = self.map.grid_w.max(1);
        let map_h = self.map.grid_h.max(1);

        // Mantém aspecto aproximado do grid (sem distorcer demais).
        let mut draw_w = area.width.max(1);
        let mut draw_h = area.height.max(1);
        // tenta usar a largura toda; se estourar altura, ajusta pela altura
        let fit_h = ((draw_w as u32 * map_h as u32) / map_w as u32).max(1) as u16;
        if fit_h <= draw_h {
            draw_h = fit_h;
        } else {
            draw_w = ((draw_h as u32 * map_w as u32) / map_h as u32).max(1) as u16;
        }

        draw_w = draw_w.min(area.width).max(1);
        draw_h = draw_h.min(area.height).max(1);

        let off_x = area.x + (area.width.saturating_sub(draw_w) / 2);
        let off_y = area.y + (area.height.saturating_sub(draw_h) / 2);

        let z = 1u16;
        let start = self.map.path.first().copied();
        let goal = self.map.path.last().copied();

        // fundo
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_symbol(" ")
                        .set_style(Style::default().bg(map_bg(z)));
                }
            }
        }

        // Pré-computa máscara do caminho no grid (barato: no máx ~80*40).
        let map_w_usize = map_w as usize;
        let map_h_usize = map_h as usize;
        let mut is_path_cell = vec![false; map_w_usize.saturating_mul(map_h_usize)];
        for &(x, y) in &self.map.path {
            if x < map_w && y < map_h {
                is_path_cell[y as usize * map_w_usize + x as usize] = true;
            }
        }

        // Renderiza 1 caractere por "pixel" do minimap.
        let dw = draw_w as u32;
        let dh = draw_h as u32;
        let mw = map_w as u32;
        let mh = map_h as u32;

        for sy in 0..draw_h {
            for sx in 0..draw_w {
                // região no grid que este pixel cobre (usa ceil para nunca dar range vazio)
                let mx0 = (sx as u32 * mw) / dw;
                let my0 = (sy as u32 * mh) / dh;
                let mx1 = (((sx as u32 + 1) * mw + dw - 1) / dw).min(mw);
                let my1 = (((sy as u32 + 1) * mh + dh - 1) / dh).min(mh);

                let mut has_path = false;
                let mut has_start = false;
                let mut has_goal = false;
                for my in my0..my1 {
                    for mx in mx0..mx1 {
                        let cell = (mx as u16, my as u16);
                        if start == Some(cell) {
                            has_start = true;
                        }
                        if goal == Some(cell) {
                            has_goal = true;
                        }
                        let idx = my as usize * map_w_usize + mx as usize;
                        if is_path_cell.get(idx).copied().unwrap_or(false) {
                            has_path = true;
                        }
                    }
                }

                let (sym, fg, bgc) = if has_goal {
                    ("G", Color::LightMagenta, goal_bg(z))
                } else if has_start {
                    ("S", Color::LightGreen, path_bg(z))
                } else if has_path {
                    ("█", Color::LightYellow, path_bg(z))
                } else {
                    (" ", Color::Reset, map_bg(z))
                };

                let x = off_x + sx;
                let y = off_y + sy;
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_symbol(sym)
                        .set_style(Style::default().fg(fg).bg(bgc));
                }
            }
        }
    }
}

fn particle_visual(kind: ParticleKind, ttl: u8) -> (&'static str, Color, Modifier) {
    let t = ttl.max(1).min(6) as usize;
    // idx: ttl alto -> mais forte/cheio
    let idx = 6 - t;

    match kind {
        ParticleKind::TrailBasic => (assets::TRAIL_BASIC[idx], Color::LightMagenta, Modifier::empty()),
        ParticleKind::TrailSniper => (assets::TRAIL_SNIPER[idx], Color::LightCyan, Modifier::BOLD),
        ParticleKind::TrailRapid => (assets::TRAIL_RAPID[idx], Color::Yellow, Modifier::empty()),
        ParticleKind::TrailCannon => (assets::TRAIL_CANNON[idx], Color::LightRed, Modifier::empty()),
        ParticleKind::TrailTesla => (assets::TRAIL_TESLA[idx], Color::LightBlue, Modifier::BOLD),
        ParticleKind::TrailFrost => (assets::TRAIL_FROST[idx], Color::Cyan, Modifier::empty()),

        ParticleKind::Spark => (
            assets::SPARK[idx.min(3)],
            if ttl >= 4 { warn() } else { Color::Yellow },
            Modifier::BOLD,
        ),
        ParticleKind::Smoke => (assets::SMOKE[idx.min(3)], text_dim(), Modifier::empty()),
        ParticleKind::Arc => (assets::ARC[idx.min(3)], Color::LightBlue, Modifier::BOLD),
        ParticleKind::Shard => (assets::SHARD[idx.min(3)], Color::LightBlue, Modifier::empty()),
        ParticleKind::Bolt => (assets::BOLT[idx.min(3)], Color::LightBlue, Modifier::BOLD),
        ParticleKind::Frost => (assets::FROST[idx.min(3)], Color::Cyan, Modifier::empty()),
        ParticleKind::Wave => (assets::WAVE[idx.min(3)], Color::LightRed, Modifier::BOLD),
    }
}

fn tower_kind_label(kind: TowerKind) -> &'static str {
    match kind {
        TowerKind::Basic => "Basic",
        TowerKind::Sniper => "Sniper",
        TowerKind::Rapid => "Rapid",
        TowerKind::Cannon => "Cannon",
        TowerKind::Tesla => "Tesla",
        TowerKind::Frost => "Frost",
    }
}

fn tower_kind_color(kind: TowerKind) -> Color {
    match kind {
        TowerKind::Basic => warn(),
        TowerKind::Sniper => Color::LightCyan,
        TowerKind::Rapid => Color::Yellow,
        TowerKind::Cannon => Color::LightRed,
        TowerKind::Tesla => Color::LightBlue,
        TowerKind::Frost => Color::LightBlue,
    }
}

#[derive(Debug, Clone, Copy)]
enum RangeShape {
    Diamond,
    Hex,
}

fn range_focus(app: &App) -> Option<(u16, u16, u16, RangeShape)> {
    // Torre em foco: mostra range dela.
    if let Some(t) = app.selected_tower() {
        let stats = App::tower_stats(t);
        return Some((t.x, t.y, stats.range, range_shape(t.kind)));
    }

    let (x, y) = app.game.selected_cell?;
    if app.is_path(x, y) || app.tower_index_at(x, y).is_some() {
        return None;
    }

    let preview = app.build_preview_stats()?;
    Some((x, y, preview.range, range_shape(app.game.build_kind?)))
}

fn manhattan(x1: u16, y1: u16, x2: u16, y2: u16) -> u16 {
    x1.abs_diff(x2) + y1.abs_diff(y2)
}

fn hex_distance(x1: u16, y1: u16, x2: u16, y2: u16) -> u16 {
    let dx = x1 as i32 - x2 as i32;
    let dy = y1 as i32 - y2 as i32;
    let dz = -dx - dy;
    ((dx.abs() + dy.abs() + dz.abs()) / 2) as u16
}

fn range_shape(kind: TowerKind) -> RangeShape {
    match kind {
        TowerKind::Sniper | TowerKind::Frost => RangeShape::Hex,
        _ => RangeShape::Diamond,
    }
}

fn range_match(shape: RangeShape, x: u16, y: u16, cx: u16, cy: u16, range: u16) -> bool {
    match shape {
        RangeShape::Diamond => manhattan(x, y, cx, cy) == range,
        RangeShape::Hex => hex_distance(x, y, cx, cy) == range,
    }
}

fn tower_effect_line(kind: TowerKind, level: u8, upgrade_hover: bool) -> Line<'static> {
    let (label, color, next_label) = tower_effect_labels(kind, level, upgrade_hover);
    let mut spans = vec![
        Span::styled("Effect: ", Style::default().fg(text_dim())),
        Span::styled(
            label,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ];
    if let Some(next) = next_label {
        spans.push(Span::raw(" "));
        spans.push(Span::styled("→", Style::default().fg(text_dim())));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            next,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    }
    Line::from(spans)
}

fn tower_effect_labels(
    kind: TowerKind,
    level: u8,
    upgrade_hover: bool,
) -> (String, Color, Option<String>) {
    match kind {
        TowerKind::Tesla => {
            let (radius, targets, percent) = App::tesla_chain_params(level);
            let label = format!("Chain {percent}% r{radius} x{targets}");
            let next = if upgrade_hover {
                let next_level = (level + 1).min(9);
                if next_level > level {
                    let (nr, nt, np) = App::tesla_chain_params(next_level);
                    Some(format!("Chain {np}% r{nr} x{nt}"))
                } else {
                    None
                }
            } else {
                None
            };
            (label, Color::LightBlue, next)
        }
        TowerKind::Cannon => {
            let (radius, percent) = App::cannon_splash_params(level);
            let label = format!("Splash {percent}% r{radius}");
            let next = if upgrade_hover {
                let next_level = (level + 1).min(9);
                if next_level > level {
                    let (nr, np) = App::cannon_splash_params(next_level);
                    Some(format!("Splash {np}% r{nr}"))
                } else {
                    None
                }
            } else {
                None
            };
            (label, Color::LightRed, next)
        }
        TowerKind::Frost => {
            let (slow_percent, slow_ticks) = App::frost_slow(level);
            let (burst_radius, burst_slow, _burst_ticks) = App::frost_burst_params(level);
            let label =
                format!("Slow {slow_percent}%/{slow_ticks}t + Chill {burst_slow}% r{burst_radius}");
            let next = if upgrade_hover {
                let next_level = (level + 1).min(9);
                if next_level > level {
                    let (next_percent, next_ticks) = App::frost_slow(next_level);
                    let (nr, nb, _nt) = App::frost_burst_params(next_level);
                    Some(format!(
                        "Slow {next_percent}%/{next_ticks}t + Chill {nb}% r{nr}"
                    ))
                } else {
                    None
                }
            } else {
                None
            };
            (label, Color::LightBlue, next)
        }
        _ => ("—".to_string(), text_dim(), None),
    }
}
