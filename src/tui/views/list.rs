use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};

use crate::tui::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(5),    // table
            Constraint::Length(1), // status bar
        ])
        .split(frame.area());

    render_header(frame, chunks[0], app);
    render_table(frame, chunks[1], app);
    render_status_bar(frame, chunks[2], app);
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let title = if app.is_offline {
        vec![
            Span::styled(" Procrast ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(" OFFLINE ", Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]
    } else {
        vec![
            Span::styled(" Procrast ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]
    };

    let header = Paragraph::new(Line::from(title))
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, area);
}

fn render_table(frame: &mut Frame, area: Rect, app: &App) {
    let header = Row::new(vec!["UUID", "Title", "Priority", "Status", "Created"])
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .bottom_margin(1);

    let rows: Vec<Row> = app
        .ideas
        .iter()
        .enumerate()
        .map(|(i, idea)| {
            let title = idea
                .summary_title
                .as_deref()
                .unwrap_or_else(|| {
                    if idea.content.len() > 50 {
                        &idea.content[..50]
                    } else {
                        &idea.content
                    }
                });

            let uuid_short = &idea.uuid[..8.min(idea.uuid.len())];
            let priority = idea.priority.as_deref().unwrap_or("-");
            let status = idea.refinement_status.as_deref().unwrap_or("-");
            let created = &idea.created_at[..10.min(idea.created_at.len())];

            let style = if i == app.selected {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(uuid_short.to_string()),
                Cell::from(title.to_string()),
                Cell::from(priority.to_string()),
                Cell::from(status.to_string()),
                Cell::from(created.to_string()),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Percentage(50),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::NONE));

    let mut state = TableState::default();
    state.select(Some(app.selected));
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let msg = app
        .status_message
        .as_deref()
        .unwrap_or("");

    let bar = Line::from(vec![
        Span::styled(
            " j/k:navigate  Enter:view  /:search  r:refresh  q:quit ",
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            msg,
            Style::default().fg(Color::Yellow),
        ),
    ]);
    frame.render_widget(Paragraph::new(bar), area);
}
