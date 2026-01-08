use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::*,
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph, Widget, Wrap},
};

use crate::{
    app::{
        App, ButtonId, HoverAction, LayoutMode, MapSelectAction, MapSpec, MapViewport,
        ParticleKind, Screen, TowerKind,
    },
    assets,
};

// ------------------------------------------------------------
// Theme (discreto / "profissional")
// ------------------------------------------------------------

#[inline]
fn bg() -> Color {
    Color::Black
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

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.size();
    app.set_layout_mode_from_size(area);

    f.render_widget(Block::default().style(Style::default().bg(bg())), area);

    match app.screen {
        Screen::MapSelect => draw_map_select(f, app, area),
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
    app.ui.hit.build_options = [Rect::new(0, 0, 0, 0); 3];

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
    let title = Paragraph::new(Line::from(vec![
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
    ]))
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
        Span::styled(format!("$ {}", app.game.money), Style::default().fg(warn())),
        Span::raw("  "),
        Span::styled(
            format!("HP {}", app.game.lives),
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
            Span::raw(": select / click  "),
            Span::styled("Space", Style::default().fg(text_dim())),
            Span::raw(": start/pause  "),
            Span::styled("B", Style::default().fg(text_dim())),
            Span::raw(": build  "),
            Span::styled("1-3", Style::default().fg(text_dim())),
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
        .style(Style::default().fg(text_dim()).bg(bg()));
        f.render_widget(hint, hint_area);
    }
}

fn compute_viewport(app: &App, inner: Rect) -> MapViewport {
    let mut vp = MapViewport::default();
    vp.tile_w = 2 * app.ui.zoom.max(1);
    vp.tile_h = 1;

    vp.vis_w = (inner.width / vp.tile_w).max(1).min(app.game.grid_w);
    vp.vis_h = inner.height.max(1).min(app.game.grid_h);

    let (cx, cy) = app.game.selected_cell.unwrap_or((0, 0));
    let mut vx = app.ui.viewport.view_x;
    let mut vy = app.ui.viewport.view_y;

    let max_x = app.game.grid_w.saturating_sub(vp.vis_w);
    let max_y = app.game.grid_h.saturating_sub(vp.vis_h);
    vx = vx.min(max_x);
    vy = vy.min(max_y);

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

    vp.view_x = vx.min(max_x);
    vp.view_y = vy.min(max_y);
    vp
}

fn draw_sidebar(f: &mut Frame, app: &mut App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(8)])
        .split(area);

    draw_build_panel(f, app, rows[0]);
    draw_inspector_panel(f, app, rows[1]);
}

fn draw_build_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let block = panel_block("Build Selector");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .split(inner);

    let towers = App::available_towers();
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
            }

            stats_lines.push(Line::from(Span::styled(
                "Select grass tile and press Build.",
                Style::default().fg(text_dim()),
            )));
        } else {
            stats_lines.push(Line::from(Span::styled(
                "Select a tower type (1/2/3) to build.",
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

    f.render_widget(
        Paragraph::new(format!(
            "Build [B] ({}) — grass",
            app.game
                .build_kind
                .map(App::tower_cost)
                .map(|c| c.to_string())
                .unwrap_or_else(|| "-".to_string())
        ))
        .style(if app.game.build_kind.is_some() {
            Style::default().fg(good()).bg(bg())
        } else {
            Style::default().fg(text_dim()).bg(bg())
        })
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
            "Wave {} | Lives {} | $ {} | Speed x{} | Zoom {}",
            app.game.wave, app.game.lives, app.game.money, app.game.speed, app.ui.zoom
        )),
        Line::from(sel),
        Line::from(format!(
            "Build: {} ({}). Switch: 1 Basic • 2 Sniper • 3 Rapid",
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
    ];

    f.render_widget(
        Paragraph::new(txt)
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

struct MapWidget<'a> {
    app: &'a App,
}

impl<'a> Widget for MapWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let app = self.app;
        let vp = app.ui.viewport;
        let range_focus = range_focus(app);
        let goal = app.game.path.last().copied();

        let cell_bg = |cell_x: u16, cell_y: u16| -> Color {
            if app.game.selected_cell == Some((cell_x, cell_y)) {
                Color::Blue
            } else if app.ui.hover_cell == Some((cell_x, cell_y)) {
                Color::DarkGray
            } else if let Some((rx, ry, range)) = range_focus {
                if manhattan(cell_x, cell_y, rx, ry) == range {
                    Color::Blue
                } else {
                    bg()
                }
            } else {
                bg()
            }
        };

        if area.width == 0 || area.height == 0 || vp.vis_w == 0 || vp.vis_h == 0 {
            return;
        }

        // fundo
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf.get_mut(x, y)
                    .set_symbol(" ")
                    .set_style(Style::default().bg(bg()));
            }
        }

        // tiles + entidades
        for gy in 0..vp.vis_h {
            for gx in 0..vp.vis_w {
                let cell_x = vp.view_x + gx;
                let cell_y = vp.view_y + gy;

                let sx = area.x + gx * vp.tile_w;
                let sy = area.y + gy * vp.tile_h;

                let is_hover = app.ui.hover_cell == Some((cell_x, cell_y));
                let is_sel = app.game.selected_cell == Some((cell_x, cell_y));

                let mut tile = assets::GLYPH_GRASS[(cell_x as usize + cell_y as usize) % 4];
                let mut style = Style::default().fg(Color::Green).bg(bg());

                if app.is_path(cell_x, cell_y) {
                    tile = assets::GLYPH_PATH[(cell_x as usize + cell_y as usize) % 2];
                    style = Style::default().fg(Color::LightYellow).bg(bg());
                }

                if goal == Some((cell_x, cell_y)) {
                    tile = assets::GLYPH_GOAL;
                    style = Style::default()
                        .fg(Color::LightMagenta)
                        .bg(bg())
                        .add_modifier(Modifier::BOLD);
                }

                if let Some(ti) = app.tower_index_at(cell_x, cell_y) {
                    let t = &app.game.towers[ti];
                    tile = match t.kind {
                        TowerKind::Basic => assets::GLYPH_TOWER_BASIC,
                        TowerKind::Sniper => assets::GLYPH_TOWER_SNIPER,
                        TowerKind::Rapid => assets::GLYPH_TOWER_RAPID,
                    };
                    style = Style::default()
                        .fg(if t.level >= 4 {
                            warn()
                        } else {
                            tower_kind_color(t.kind)
                        })
                        .bg(bg())
                        .add_modifier(Modifier::BOLD);
                }

                if app.enemy_at(cell_x, cell_y) {
                    tile = assets::GLYPH_ENEMY;
                    style = Style::default()
                        .fg(Color::Red)
                        .bg(bg())
                        .add_modifier(Modifier::BOLD);
                }

                // impacto grande por cima do tile
                if let Some(fx) = app
                    .game
                    .impacts
                    .iter()
                    .find(|fx| fx.x == cell_x && fx.y == cell_y)
                {
                    tile = assets::GLYPH_IMPACT_BIG;
                    style = match fx.ttl {
                        4 => Style::default()
                            .fg(danger())
                            .bg(bg())
                            .add_modifier(Modifier::BOLD),
                        3 => Style::default()
                            .fg(Color::Red)
                            .bg(bg())
                            .add_modifier(Modifier::BOLD),
                        2 => Style::default().fg(Color::DarkGray).bg(bg()),
                        _ => Style::default().fg(text_dim()).bg(bg()),
                    };
                }

                if is_sel {
                    style = style.bg(Color::Blue).fg(Color::White);
                } else if is_hover {
                    style = style.bg(Color::DarkGray).fg(Color::Black);
                } else if let Some((rx, ry, range)) = range_focus {
                    if manhattan(cell_x, cell_y, rx, ry) == range {
                        style = style.bg(Color::Blue);
                    }
                }

                // tile_w>=2 -> glifos duplos + padding
                if sx < area.right() && sy < area.bottom() {
                    buf.get_mut(sx, sy).set_symbol(tile.left).set_style(style);
                }
                if sx + 1 < area.right() && sy < area.bottom() {
                    buf.get_mut(sx + 1, sy)
                        .set_symbol(tile.right)
                        .set_style(style);
                }
                for pad in 2..vp.tile_w {
                    if sx + pad < area.right() && sy < area.bottom() {
                        buf.get_mut(sx + pad, sy).set_symbol(" ").set_style(style);
                    }
                }
            }
        }

        // projéteis
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
            let sx = area.x + gx * vp.tile_w;
            let sy = area.y + gy * vp.tile_h;
            let bg_color = cell_bg(cx, cy);

            let (sym, color) = match p.kind {
                TowerKind::Basic => (assets::GLYPH_PROJECTILE_BASIC, Color::LightMagenta),
                TowerKind::Sniper => (assets::GLYPH_PROJECTILE_SNIPER, Color::LightCyan),
                TowerKind::Rapid => (assets::GLYPH_PROJECTILE_RAPID, Color::Yellow),
            };
            let style = Style::default()
                .fg(color)
                .bg(bg_color)
                .add_modifier(Modifier::BOLD);
            let mid = sx + (vp.tile_w.saturating_sub(1) / 2).max(1);
            if mid < area.right() && sy < area.bottom() {
                buf.get_mut(mid, sy).set_symbol(sym).set_style(style);
            }
        }

        // partículas
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
            let sx = area.x + gx * vp.tile_w;
            let sy = area.y + gy * vp.tile_h;
            let bg_color = cell_bg(cx, cy);

            let (sym, style) = particle_visual(p.kind, p.ttl);
            let style = style.bg(bg_color);
            let mid = sx + (vp.tile_w.saturating_sub(1) / 2).max(1);
            if mid < area.right() && sy < area.bottom() {
                buf.get_mut(mid, sy).set_symbol(sym).set_style(style);
            }
        }
    }
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
            zoom: app.ui.zoom,
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
    zoom: u16,
}

impl<'a> Widget for MapPreviewWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let tile_w = 2 * self.zoom.max(1);
        let tile_h = 1;

        let vis_w = (area.width / tile_w).max(1).min(self.map.grid_w);
        let vis_h = area.height.max(1).min(self.map.grid_h);

        let view_x = self.map.grid_w.saturating_sub(vis_w) / 2;
        let view_y = self.map.grid_h.saturating_sub(vis_h) / 2;

        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf.get_mut(x, y)
                    .set_symbol(" ")
                    .set_style(Style::default().bg(bg()));
            }
        }

        let goal = self.map.path.last().copied();

        for gy in 0..vis_h {
            for gx in 0..vis_w {
                let cell_x = view_x + gx;
                let cell_y = view_y + gy;

                let sx = area.x + gx * tile_w;
                let sy = area.y + gy * tile_h;

                let mut tile = assets::GLYPH_GRASS[(cell_x as usize + cell_y as usize) % 4];
                let mut style = Style::default().fg(Color::Green).bg(bg());

                if self
                    .map
                    .path
                    .iter()
                    .any(|&(px, py)| px == cell_x && py == cell_y)
                {
                    tile = assets::GLYPH_PATH[(cell_x as usize + cell_y as usize) % 2];
                    style = Style::default().fg(Color::LightYellow).bg(bg());
                }

                if goal == Some((cell_x, cell_y)) {
                    tile = assets::GLYPH_GOAL;
                    style = Style::default()
                        .fg(Color::LightMagenta)
                        .bg(bg())
                        .add_modifier(Modifier::BOLD);
                }

                if sx < area.right() && sy < area.bottom() {
                    buf.get_mut(sx, sy).set_symbol(tile.left).set_style(style);
                }
                if sx + 1 < area.right() && sy < area.bottom() {
                    buf.get_mut(sx + 1, sy)
                        .set_symbol(tile.right)
                        .set_style(style);
                }
                for pad in 2..tile_w {
                    if sx + pad < area.right() && sy < area.bottom() {
                        buf.get_mut(sx + pad, sy).set_symbol(" ").set_style(style);
                    }
                }
            }
        }
    }
}

fn particle_visual(kind: ParticleKind, ttl: u8) -> (&'static str, Style) {
    let t = ttl.max(1).min(4) as usize;
    let idx = 4 - t; // ttl 4 -> idx0 (forte), ttl 1 -> idx3 (fraco)

    match kind {
        ParticleKind::Trail => (
            assets::TRAIL[idx],
            Style::default().fg(Color::LightMagenta).bg(bg()),
        ),
        ParticleKind::Spark => (
            assets::SPARK[idx],
            Style::default()
                .fg(if ttl >= 3 { warn() } else { Color::Yellow })
                .bg(bg())
                .add_modifier(Modifier::BOLD),
        ),
        ParticleKind::Smoke => (assets::SMOKE[idx], Style::default().fg(text_dim()).bg(bg())),
    }
}

fn tower_kind_label(kind: TowerKind) -> &'static str {
    match kind {
        TowerKind::Basic => "Basic",
        TowerKind::Sniper => "Sniper",
        TowerKind::Rapid => "Rapid",
    }
}

fn tower_kind_color(kind: TowerKind) -> Color {
    match kind {
        TowerKind::Basic => warn(),
        TowerKind::Sniper => Color::LightCyan,
        TowerKind::Rapid => Color::Yellow,
    }
}

fn range_focus(app: &App) -> Option<(u16, u16, u16)> {
    // Torre em foco: mostra range dela.
    if let Some(t) = app.selected_tower() {
        let stats = App::tower_stats(t);
        return Some((t.x, t.y, stats.range));
    }

    let (x, y) = app.game.selected_cell?;
    if app.is_path(x, y) || app.tower_index_at(x, y).is_some() {
        return None;
    }

    let preview = app.build_preview_stats()?;
    Some((x, y, preview.range))
}

fn manhattan(x1: u16, y1: u16, x2: u16, y2: u16) -> u16 {
    x1.abs_diff(x2) + y1.abs_diff(y2)
}
