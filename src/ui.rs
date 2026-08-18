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
    let gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Probe responses ")
                .borders(Borders::ALL),
        )
        .gauge_style(Style::default().fg(loss_color(100.0 - response)))
        .percent(response.clamp(0.0, 100.0) as u16)
        .label(format!("{response:.1}% · {received}/{sent}"));
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
            let route_changed = if hop.alternate_addresses.is_empty() {
                ""
            } else {
                " ↺"
            };
            let row = Row::new(vec![
                Cell::from(hop.ttl.to_string()).style(Style::default().fg(Color::Blue)),
                Cell::from(format!("{address}{route_changed}")).style(Style::default().fg(
                    if hop.address.is_some() {
                        Color::White
                    } else {
                        Color::DarkGray
                    },
                )),
                Cell::from(format!("{:5.1}%", hop.loss_percent()))
                    .style(Style::default().fg(loss_color(hop.loss_percent()))),
                Cell::from(hop.sent.to_string()).style(Style::default().fg(Color::Cyan)),
                metric_cell(hop.last_ms),
                metric_cell(hop.average_ms),
                metric_cell(hop.best_ms),
                metric_cell(hop.worst_ms),
                metric_cell(hop.jitter_ms),
            ]);
            if index == app.selected {
                row.style(Style::default().bg(Color::Blue).fg(Color::White))
            } else {
                row
            }
        });

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Min(16),
            Constraint::Length(8),
            Constraint::Length(7),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
        ],
    )
    .header(
        Row::new([
            "Hop", "Host", "Loss", "Sent", "Last", "Avg", "Best", "Worst", "Jitter",
        ])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .title(" Network path · milliseconds ")
            .borders(Borders::ALL),
    )
    .column_spacing(1);
    frame.render_widget(table, area);
}

fn draw_visuals(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(6)])
        .split(area);
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
        rows[0],
    );

    let mut lines = Vec::new();
    for hop in app.model.visible_hops() {
        let color = hop_color(hop);
        let address = hop
            .address
            .map(|address| address.to_string())
            .unwrap_or_else(|| "no reply".to_owned());
        lines.push(Line::from(vec![
            Span::styled(format!("{:>2} ● ", hop.ttl), Style::default().fg(color)),
            Span::styled(address, Style::default().fg(color)),
        ]));
        if hop.ttl < app.model.visible_hops().len() as u8 {
            lines.push(Line::from(Span::styled(
                "   │",
                Style::default().fg(Color::DarkGray),
            )));
        }
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Route graph ")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        rows[1],
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
