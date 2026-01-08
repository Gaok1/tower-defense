use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::*,
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph, Widget, Wrap},
};

use crate::{
    app::{App, ButtonId, HoverAction, LayoutMode, MapViewport, ParticleKind, TowerKind},
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

    match app.ui.mode {
        LayoutMode::Wide => draw_wide(f, app, area),
        LayoutMode::Compact => draw_compact(f, app, area),
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
            Constraint::Length(10),
        ])
        .split(area);

    draw_header(f, app, rows[0]);
    draw_map_panel(f, app, rows[1]);

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
        .constraints([Constraint::Length(24), Constraint::Min(10), Constraint::Length(22)])
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
                    Style::default().fg(panel_title()).add_modifier(Modifier::BOLD),
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
        Span::styled(format!("HP {}", app.game.lives), Style::default().fg(danger())),
        Span::raw("  "),
        Span::styled(
            format!("x{}", app.game.speed),
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
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
    let block = panel_block("Map");
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
            Span::styled("U", Style::default().fg(text_dim())),
            Span::raw(": upgrade  "),
            Span::styled("S", Style::default().fg(text_dim())),
            Span::raw(": sell  "),
            Span::styled("F", Style::default().fg(text_dim())),
            Span::raw(": speed  "),
            Span::styled("Q", Style::default().fg(text_dim())),
            Span::raw(": quit"),
        ]))
        .style(Style::default().fg(text_dim()).bg(bg()));
        f.render_widget(hint, hint_area);
    }
}

fn compute_viewport(app: &App, inner: Rect) -> MapViewport {
    let mut vp = MapViewport::default();
    vp.tile_w = 2;
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
        .constraints([
            Constraint::Length(7),
            Constraint::Length(6),
            Constraint::Min(10),
        ])
        .split(area);

    draw_stats_panel(f, app, rows[0]);
    draw_flow_panel(f, app, rows[1]);
    draw_inspector_panel(f, app, rows[2]);
}

fn draw_stats_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("Stats");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(vec![
            Span::styled("Money", Style::default().fg(text_dim())),
            Span::raw(": "),
            Span::styled(format!("$ {}", app.game.money), Style::default().fg(warn())),
        ]),
        Line::from(vec![
            Span::styled("Lives", Style::default().fg(text_dim())),
            Span::raw(": "),
            Span::styled(format!("{}", app.game.lives), Style::default().fg(danger())),
        ]),
        Line::from(vec![
            Span::styled("Towers", Style::default().fg(text_dim())),
            Span::raw(": "),
            Span::styled(format!("{}", app.game.towers.len()), Style::default().fg(accent())),
        ]),
        Line::from(vec![
            Span::styled("Enemies", Style::default().fg(text_dim())),
            Span::raw(": "),
            Span::styled(
                format!("{}", app.game.enemies.iter().filter(|e| e.hp > 0).count()),
                Style::default().fg(Color::Red),
            ),
        ]),
    ];

    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::White).bg(bg())),
        inner,
    );
}

fn draw_flow_panel(f: &mut Frame, app: &App, area: Rect) {
    let pct = app.wave_progress_percent();
    let g = Gauge::default()
        .block(panel_block("Path Progress"))
        .gauge_style(Style::default().fg(accent()))
        .percent(pct);
    f.render_widget(g, area);
}

fn draw_inspector_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let block = panel_block("Inspector");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(6), Constraint::Length(3)])
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
                match t.kind {
                    TowerKind::Basic => "Basic",
                },
                Style::default().fg(warn()).add_modifier(Modifier::BOLD),
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

        stats_lines.push(line_stat_i32("Attack", s.attack, upgrade_hover.then_some(d.attack)));
        stats_lines.push(line_stat_u16("Range", s.range, upgrade_hover.then_some(d.range)));
        stats_lines.push(line_stat_u16_inv("Fire CD", s.fire_cd, upgrade_hover.then_some(d.fire_cd)));

        stats_lines.push(Line::from(""));
        stats_lines.push(Line::from(vec![
            Span::styled("Build", Style::default().fg(text_dim())),
            Span::raw(": 50   "),
            Span::styled("Upgrade", Style::default().fg(text_dim())),
            Span::raw(": 30   "),
            Span::styled("Sell", Style::default().fg(text_dim())),
            Span::raw(": +20"),
        ]));
    } else {
        stats_lines.push(Line::from("No tower selected."));
        stats_lines.push(Line::from(Span::styled(
            "Select grass tile and press Build.",
            Style::default().fg(text_dim()),
        )));
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
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
        .split(actions);

    app.ui.hit.inspector_upgrade = action_rows[0];

    let upgrade_hover = app.ui.hover_action == Some(HoverAction::UpgradePreview)
        || app.ui.hover_button == Some(ButtonId::Upgrade);

    let upgrade_style = if upgrade_hover {
        Style::default().fg(Color::Black).bg(accent())
    } else {
        Style::default().fg(panel_title()).bg(bg())
    };

    let upgrade_text = if app.selected_tower().is_some() {
        "Upgrade [U] (30)  — hover preview"
    } else {
        "Upgrade [U] (30)  — select a tower"
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
        Paragraph::new("Build [B] (50) — grass")
            .style(Style::default().fg(good()).bg(bg()))
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
            "Wave {} | Lives {} | $ {} | Speed x{}",
            app.game.wave, app.game.lives, app.game.money, app.game.speed
        )),
        Line::from(sel),
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

                let mut sym = assets::GLYPH_GRASS;
                let mut style = Style::default().fg(Color::DarkGray).bg(bg());

                if app.is_path(cell_x, cell_y) {
                    sym = assets::GLYPH_PATH;
                    style = Style::default().fg(Color::Gray).bg(bg());
                }

                if let Some(ti) = app.tower_index_at(cell_x, cell_y) {
                    let t = &app.game.towers[ti];
                    sym = assets::GLYPH_TOWER_BASIC;
                    style = Style::default()
                        .fg(if t.level >= 4 { warn() } else { Color::Yellow })
                        .bg(bg())
                        .add_modifier(Modifier::BOLD);
                }

                if app.enemy_at(cell_x, cell_y) {
                    sym = assets::GLYPH_ENEMY;
                    style = Style::default().fg(Color::Red).bg(bg()).add_modifier(Modifier::BOLD);
                }

                // impacto grande por cima do tile
                if let Some(fx) = app
                    .game
                    .impacts
                    .iter()
                    .find(|fx| fx.x == cell_x && fx.y == cell_y)
                {
                    sym = assets::GLYPH_IMPACT_BIG;
                    style = match fx.ttl {
                        4 => Style::default().fg(danger()).bg(bg()).add_modifier(Modifier::BOLD),
                        3 => Style::default().fg(Color::Red).bg(bg()).add_modifier(Modifier::BOLD),
                        2 => Style::default().fg(Color::DarkGray).bg(bg()),
                        _ => Style::default().fg(text_dim()).bg(bg()),
                    };
                }

                if is_sel {
                    style = style.bg(Color::Blue).fg(Color::White);
                } else if is_hover {
                    style = style.bg(Color::DarkGray).fg(Color::Black);
                }

                // tile_w=2 -> glifo + padding
                if sx < area.right() && sy < area.bottom() {
                    buf.get_mut(sx, sy).set_symbol(sym).set_style(style);
                }
                if sx + 1 < area.right() && sy < area.bottom() {
                    buf.get_mut(sx + 1, sy).set_symbol(" ").set_style(style);
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

            let style = Style::default()
                .fg(Color::LightMagenta)
                .bg(bg())
                .add_modifier(Modifier::BOLD);
            if sx < area.right() && sy < area.bottom() {
                buf.get_mut(sx, sy)
                    .set_symbol(assets::GLYPH_PROJECTILE)
                    .set_style(style);
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

            let (sym, style) = particle_visual(p.kind, p.ttl);
            if sx < area.right() && sy < area.bottom() {
                buf.get_mut(sx, sy).set_symbol(sym).set_style(style);
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
        ParticleKind::Smoke => (
            assets::SMOKE[idx],
            Style::default().fg(text_dim()).bg(bg()),
        ),
    }
}
