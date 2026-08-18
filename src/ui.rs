use crate::app::App;
use crate::model::HopStats;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Sparkline, Table, Wrap};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, app: &App) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(frame.area());
    draw_header(frame, app, areas[0]);
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(areas[1]);
    draw_hops(frame, app, body[0]);
    draw_visuals(frame, app, body[1]);
    draw_footer(frame, app, areas[2]);
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(area);
    let reached = app
        .model
        .reached_hop
        .map(|hop| format!("reached in {hop} hops"))
        .unwrap_or_else(|| "discovering path".to_owned());
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", app.model.target),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.model.target_address.to_string(),
            Style::default().fg(Color::LightBlue),
        ),
        Span::raw(format!(" · {reached} · cycles {}", app.model.cycles)),
    ]))
    .block(Block::default().title(" ReMTR ").borders(Borders::ALL));
    frame.render_widget(title, columns[0]);

    let sent: u64 = app.model.hops.iter().map(|hop| hop.sent).sum();
    let received: u64 = app.model.hops.iter().map(|hop| hop.received).sum();
    let response = if sent == 0 {
        0.0
    } else {
        received as f64 * 100.0 / sent as f64
    };
    let bar_color = loss_color(100.0 - response);
    let label_color = if response >= 50.0 {
        Color::Black
    } else {
        Color::White
    };
    let gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Probe responses ")
                .borders(Borders::ALL),
        )
        .gauge_style(Style::default().fg(bar_color).bg(Color::Rgb(38, 38, 38)))
        .percent(response.clamp(0.0, 100.0) as u16)
        .label(Span::styled(
            format!("{response:.1}% · {received}/{sent}"),
            Style::default()
                .fg(label_color)
                .add_modifier(Modifier::BOLD),
        ))
        .use_unicode(true);
    frame.render_widget(gauge, columns[1]);
}

fn draw_hops(frame: &mut Frame, app: &App, area: Rect) {
    let height = area.height.saturating_sub(3) as usize;
    let visible = app.model.visible_hops();
    let scroll = if app.selected >= height {
        app.selected + 1 - height
    } else {
        0
    };
    let show_trend = area.width >= 88;
    let show_detailed = area.width >= 118;
    let rows = visible
        .iter()
        .enumerate()
        .skip(scroll)
        .take(height)
        .map(|(index, hop)| {
            let address = hop
                .address
                .map(|address| address.to_string())
                .unwrap_or_else(|| "*".to_owned());
            let (status, status_color) = hop_status(hop, app.model.reached_hop);
            let mut cells = vec![
                Cell::from(status).style(Style::default().fg(status_color)),
                Cell::from(hop.ttl.to_string()).style(Style::default().fg(Color::Blue)),
                Cell::from(address).style(Style::default().fg(if hop.address.is_some() {
                    Color::White
                } else {
                    Color::DarkGray
                })),
                Cell::from(format!("{}/{}", hop.received, hop.sent))
                    .style(Style::default().fg(Color::Cyan)),
                Cell::from(format!("{:5.1}%", hop.loss_percent()))
                    .style(Style::default().fg(loss_color(hop.loss_percent()))),
                metric_cell(hop.last_ms),
                metric_cell(hop.average_ms),
                metric_cell(hop.percentile_ms(0.95)),
                metric_cell(hop.jitter_ms),
            ];
            if show_detailed {
                cells.extend([
                    metric_cell(hop.best_ms),
                    metric_cell(hop.worst_ms),
                    metric_cell(hop.standard_deviation_ms()),
                    Cell::from(hop.route_changes.to_string()).style(Style::default().fg(
                        if hop.route_changes > 0 {
                            Color::Yellow
                        } else {
                            Color::DarkGray
                        },
                    )),
                ]);
            }
            if show_trend {
                cells.push(
                    Cell::from(inline_trend(hop, 12)).style(Style::default().fg(Color::Cyan)),
                );
            }
            let row = Row::new(cells);
            if index == app.selected {
                row.style(Style::default().bg(Color::Blue).fg(Color::White))
            } else {
                row
            }
        });

    let mut headers = vec![
        "",
        "Hop",
        "Host",
        "Recv/Sent",
        "Loss",
        "Last",
        "Avg",
        "P95",
        "Jitter",
    ];
    let mut widths = vec![
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Min(15),
        Constraint::Length(9),
        Constraint::Length(7),
        Constraint::Length(7),
        Constraint::Length(7),
        Constraint::Length(7),
        Constraint::Length(7),
    ];
    if show_detailed {
        headers.extend(["Best", "Worst", "StdDev", "Chg"]);
        widths.extend([
            Constraint::Length(7),
            Constraint::Length(7),
            Constraint::Length(7),
            Constraint::Length(4),
        ]);
    }
    if show_trend {
        headers.push("Recent RTT");
        widths.push(Constraint::Length(12));
    }
    let table = Table::new(rows, widths)
        .header(
            Row::new(headers).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(
            Block::default()
                .title(" Network path · ◆ target · ↺ changed · ! loss · × no reply · milliseconds ")
                .borders(Borders::ALL),
        )
        .column_spacing(1);
    frame.render_widget(table, area);
}

fn draw_visuals(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Length(8),
            Constraint::Min(6),
        ])
        .split(area);
    let details = if let Some(hop) = app.selected_hop() {
        vec![
            Line::from(format!(
                " Host: {}",
                hop.address
                    .map(|address| address.to_string())
                    .unwrap_or_else(|| "no reply".to_owned())
            )),
            Line::from(format!(
                " Replies: {}/{} · timeouts {}",
                hop.received,
                hop.sent,
                hop.timeouts()
            )),
            Line::from(format!(
                " P95 {} · StdDev {}",
                metric_text(hop.percentile_ms(0.95)),
                metric_text(hop.standard_deviation_ms())
            )),
            Line::from(format!(
                " Range {}–{} · jitter {}",
                metric_text(hop.best_ms),
                metric_text(hop.worst_ms),
                metric_text(hop.jitter_ms)
            )),
            Line::from(format!(
                " Route changes: {} · known paths {}",
                hop.route_changes,
                hop.alternate_addresses.len() + usize::from(hop.address.is_some())
            )),
        ]
    } else {
        vec![Line::from(" Waiting for probes")]
    };
    frame.render_widget(
        Paragraph::new(details).block(
            Block::default()
                .title(" Selected hop ")
                .borders(Borders::ALL),
        ),
        rows[0],
    );
    let history: Vec<u64> = app
        .selected_hop()
        .map(|hop| {
            hop.history
                .iter()
                .map(|sample| sample.unwrap_or(0.0).round() as u64)
                .collect()
        })
        .unwrap_or_default();
    let max = history.iter().copied().max().unwrap_or(1).max(1);
    let selected_title = app
        .selected_hop()
        .map(|hop| format!(" Hop {} latency · max {max} ms ", hop.ttl))
        .unwrap_or_else(|| " Latency history ".to_owned());
    frame.render_widget(
        Sparkline::default()
            .block(Block::default().title(selected_title).borders(Borders::ALL))
            .data(&history)
            .max(max)
            .style(Style::default().fg(Color::Cyan)),
        rows[1],
    );

    let mut lines = Vec::new();
    let route_width = usize::from(area.width.saturating_sub(2));
    for hop in app.model.visible_hops() {
        let color = hop_color(hop);
        let address = hop
            .address
            .map(|address| address.to_string())
            .unwrap_or_else(|| "no reply".to_owned());
        let (status, _) = hop_status(hop, app.model.reached_hop);
        let path = format!("{:>2} {status} {address}", hop.ttl);
        let loss = format!("{:>5.1}%", hop.loss_percent());
        let trend = inline_trend(hop, 10);
        let padding = route_width
            .saturating_sub(path.chars().count() + loss.chars().count() + trend.chars().count() + 2)
            .max(1);
        lines.push(Line::from(vec![
            Span::styled(path, Style::default().fg(color)),
            Span::raw(" ".repeat(padding)),
            Span::styled(
                format!("{loss} "),
                Style::default().fg(loss_color(hop.loss_percent())),
            ),
            Span::styled(trend, Style::default().fg(Color::Cyan)),
        ]));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Route graph · loss · RTT → ")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        rows[2],
    );
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let message = if let Some(error) = &app.error {
        Line::from(Span::styled(
            format!(" error: {error} "),
            Style::default().bg(Color::Red).fg(Color::White),
        ))
    } else {
        Line::from(vec![
            Span::styled(" ↑↓/jk ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("select  "),
            Span::styled("p/space ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(if app.paused { "resume  " } else { "pause  " }),
            Span::styled("r ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("reset  "),
            Span::styled("q ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("quit"),
        ])
    };
    frame.render_widget(Paragraph::new(message), area);
}

fn metric_cell(value: Option<f64>) -> Cell<'static> {
    match value {
        Some(value) => {
            Cell::from(format!("{value:6.1}")).style(Style::default().fg(latency_color(value)))
        }
        None => Cell::from("     -").style(Style::default().fg(Color::DarkGray)),
    }
}

fn metric_text(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.1} ms"))
}

fn hop_status(hop: &HopStats, reached_hop: Option<u8>) -> (&'static str, Color) {
    if reached_hop == Some(hop.ttl) {
        ("◆", Color::Cyan)
    } else if hop.sent == 0 {
        ("○", Color::DarkGray)
    } else if matches!(hop.history.back(), Some(None)) {
        ("×", Color::Red)
    } else if hop.loss_percent() >= 20.0 {
        ("!", Color::Red)
    } else if hop.route_changes > 0 {
        ("↺", Color::Yellow)
    } else {
        ("●", Color::Green)
    }
}

fn inline_trend(hop: &HopStats, width: usize) -> String {
    const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let samples: Vec<Option<f64>> = hop.history.iter().rev().take(width).copied().collect();
    let max = samples
        .iter()
        .flatten()
        .copied()
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let mut trend: String = samples
        .into_iter()
        .rev()
        .map(|sample| match sample {
            Some(value) => {
                let index = ((value / max) * (BARS.len() - 1) as f64).round() as usize;
                BARS[index.min(BARS.len() - 1)]
            }
            None => '·',
        })
        .collect();
    while trend.chars().count() < width {
        trend.insert(0, ' ');
    }
    trend
}

fn latency_color(milliseconds: f64) -> Color {
    if milliseconds >= 150.0 {
        Color::Red
    } else if milliseconds >= 60.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn loss_color(loss: f64) -> Color {
    if loss >= 20.0 {
        Color::Red
    } else if loss >= 5.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn hop_color(hop: &HopStats) -> Color {
    if hop.address.is_none() {
        Color::DarkGray
    } else if hop.loss_percent() >= 20.0 {
        Color::Red
    } else if !hop.alternate_addresses.is_empty() {
        Color::Yellow
    } else {
        Color::Green
    }
}
