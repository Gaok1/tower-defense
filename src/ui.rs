use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::*,
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph, Widget, Wrap},
};

use crate::{
    app::{
        App, ButtonId, HoverAction, LayoutMode, MapSelectAction, MapSpec, MapViewport,
        ParticleKind, Screen, TOWER_KIND_COUNT, TowerKind,
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
        1 => Color::Rgb(8, 22, 10),
        2 => Color::Rgb(12, 28, 12),
        3 => Color::Rgb(16, 34, 16),
        _ => Color::Rgb(20, 40, 20),
    }
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

fn compute_viewport(app: &App, inner: Rect) -> MapViewport {
    let mut vp = MapViewport::default();
    vp.tile_w = 2 * app.ui.zoom.max(1);
    vp.tile_h = 1;

    vp.vis_w = (inner.width / vp.tile_w).max(1).min(app.game.grid_w);
    vp.vis_h = inner.height.max(1).min(app.game.grid_h);

    let mut vx = app.ui.viewport.view_x;
    let mut vy = app.ui.viewport.view_y;

    let max_x = app.game.grid_w.saturating_sub(vp.vis_w);
    let max_y = app.game.grid_h.saturating_sub(vp.vis_h);
    vx = vx.min(max_x);
    vy = vy.min(max_y);

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
            "Wave {} | Lives {} | $ {} | Speed x{} | Zoom {}",
            app.game.wave, app.game.lives, app.game.money, app.game.speed, app.ui.zoom
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

struct MapWidget<'a> {
    app: &'a App,
}

impl<'a> Widget for MapWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let app = self.app;
        let vp = app.ui.viewport;
        let range_focus = range_focus(app);
        let goal = app.game.path.last().copied();
        let build_preview = app
            .game
            .build_kind
            .and_then(|kind| app.game.selected_cell.map(|cell| (cell, kind)))
            .filter(|((x, y), kind)| app.can_build_at(*x, *y, *kind));

        let cell_bg = |cell_x: u16, cell_y: u16| -> Color {
            if app.game.selected_cell == Some((cell_x, cell_y)) {
                Color::Blue
            } else if app.ui.hover_cell == Some((cell_x, cell_y)) {
                Color::DarkGray
            } else if let Some((rx, ry, range, shape)) = range_focus {
                if range_match(shape, cell_x, cell_y, rx, ry, range) {
                    Color::Blue
                } else {
                    map_bg(app.ui.zoom)
                }
            } else {
                map_bg(app.ui.zoom)
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
                    .set_style(Style::default().bg(map_bg(app.ui.zoom)));
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
                let mut style = Style::default().fg(Color::Green).bg(map_bg(app.ui.zoom));

                if app.is_path(cell_x, cell_y) {
                    tile = assets::GLYPH_PATH[(cell_x as usize + cell_y as usize) % 2];
                    style = Style::default()
                        .fg(Color::LightYellow)
                        .bg(map_bg(app.ui.zoom));
                }

                if goal == Some((cell_x, cell_y)) {
                    tile = assets::GLYPH_GOAL;
                    style = Style::default()
                        .fg(Color::LightMagenta)
                        .bg(map_bg(app.ui.zoom))
                        .add_modifier(Modifier::BOLD);
                }

                if let Some(ti) = app.tower_index_at(cell_x, cell_y) {
                    let t = &app.game.towers[ti];
                    tile = match t.kind {
                        TowerKind::Basic => assets::GLYPH_TOWER_BASIC,
                        TowerKind::Sniper => assets::GLYPH_TOWER_SNIPER,
                        TowerKind::Rapid => assets::GLYPH_TOWER_RAPID,
                        TowerKind::Cannon => assets::GLYPH_TOWER_CANNON,
                        TowerKind::Tesla => assets::GLYPH_TOWER_TESLA,
                        TowerKind::Frost => assets::GLYPH_TOWER_FROST,
                    };
                    style = Style::default()
                        .fg(if t.level >= 4 {
                            warn()
                        } else {
                            tower_kind_color(t.kind)
                        })
                        .bg(map_bg(app.ui.zoom))
                        .add_modifier(Modifier::BOLD);
                }

                if let Some(((px, py), preview_kind)) = build_preview {
                    if (cell_x, cell_y) == (px, py) {
                        tile = match preview_kind {
                            TowerKind::Basic => assets::GLYPH_TOWER_BASIC,
                            TowerKind::Sniper => assets::GLYPH_TOWER_SNIPER,
                            TowerKind::Rapid => assets::GLYPH_TOWER_RAPID,
                            TowerKind::Cannon => assets::GLYPH_TOWER_CANNON,
                            TowerKind::Tesla => assets::GLYPH_TOWER_TESLA,
                            TowerKind::Frost => assets::GLYPH_TOWER_FROST,
                        };
                        style = Style::default()
                            .fg(tower_kind_color(preview_kind))
                            .bg(map_bg(app.ui.zoom))
                            .add_modifier(Modifier::DIM);
                    }
                }

                if app.enemy_at(cell_x, cell_y) {
                    tile = assets::GLYPH_ENEMY;
                    style = Style::default()
                        .fg(Color::Red)
                        .bg(map_bg(app.ui.zoom))
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
                    let base_color = tower_kind_color(fx.kind);
                    style = match fx.ttl {
                        4 => Style::default()
                            .fg(base_color)
                            .bg(map_bg(app.ui.zoom))
                            .add_modifier(Modifier::BOLD),
                        3 => Style::default()
                            .fg(base_color)
                            .bg(map_bg(app.ui.zoom))
                            .add_modifier(Modifier::DIM),
                        2 => Style::default().fg(Color::DarkGray).bg(map_bg(app.ui.zoom)),
                        _ => Style::default().fg(text_dim()).bg(map_bg(app.ui.zoom)),
                    };
                }

                if is_sel {
                    style = style.bg(Color::Blue).fg(Color::White);
                } else if is_hover {
                    style = style.bg(Color::DarkGray).fg(Color::Black);
                } else if let Some((rx, ry, range, shape)) = range_focus {
                    if range_match(shape, cell_x, cell_y, rx, ry, range) {
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
                        let glyph = if pad % 2 == 0 { tile.left } else { tile.right };
                        buf.get_mut(sx + pad, sy).set_symbol(glyph).set_style(style);
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
                TowerKind::Cannon => (assets::GLYPH_PROJECTILE_CANNON, Color::LightRed),
                TowerKind::Tesla => (assets::GLYPH_PROJECTILE_TESLA, Color::LightBlue),
                TowerKind::Frost => (assets::GLYPH_PROJECTILE_FROST, Color::LightBlue),
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
                    .set_style(Style::default().bg(map_bg(self.zoom)));
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
                let mut style = Style::default().fg(Color::Green).bg(map_bg(self.zoom));

                if self
                    .map
                    .path
                    .iter()
                    .any(|&(px, py)| px == cell_x && py == cell_y)
                {
                    tile = assets::GLYPH_PATH[(cell_x as usize + cell_y as usize) % 2];
                    style = Style::default()
                        .fg(Color::LightYellow)
                        .bg(map_bg(self.zoom));
                }

                if goal == Some((cell_x, cell_y)) {
                    tile = assets::GLYPH_GOAL;
                    style = Style::default()
                        .fg(Color::LightMagenta)
                        .bg(map_bg(self.zoom))
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
                        let glyph = if pad % 2 == 0 { tile.left } else { tile.right };
                        buf.get_mut(sx + pad, sy).set_symbol(glyph).set_style(style);
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
        ParticleKind::Arc => (
            assets::ARC[idx],
            Style::default()
                .fg(Color::LightBlue)
                .bg(bg())
                .add_modifier(Modifier::BOLD),
        ),
        ParticleKind::Shard => (
            assets::SHARD[idx],
            Style::default().fg(Color::LightBlue).bg(bg()),
        ),
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
        TowerKind::Frost => {
            let (slow_percent, slow_ticks) = App::frost_slow(level);
            let label = format!("Slow {slow_percent}% / {slow_ticks}t");
            let next = if upgrade_hover {
                let next_level = (level + 1).min(9);
                if next_level > level {
                    let (next_percent, next_ticks) = App::frost_slow(next_level);
                    Some(format!("Slow {next_percent}% / {next_ticks}t"))
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
