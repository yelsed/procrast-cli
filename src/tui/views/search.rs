use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::tui::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // search input
            Constraint::Min(5),    // results
            Constraint::Length(1), // status bar
        ])
        .split(frame.area());

    render_search_input(frame, chunks[0], app);
    render_results(frame, chunks[1], app);
    render_status_bar(frame, chunks[2], app);
}

fn render_search_input(frame: &mut Frame, area: Rect, app: &App) {
    let input = Paragraph::new(Line::from(vec![
        Span::styled(" Search: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(&app.search_query),
        Span::styled("_", Style::default().fg(Color::White).add_modifier(Modifier::SLOW_BLINK)),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(input, area);
}

fn render_results(frame: &mut Frame, area: Rect, app: &App) {
    if app.search_results.is_empty() {
        let msg = if app.search_query.is_empty() {
            "Type a search query and press Enter"
        } else {
            "Press Enter to search"
        };
        let p = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(Color::DarkGray),
        )));
        frame.render_widget(p, area);
        return;
    }

    let rows: Vec<Row> = app
        .search_results
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
            let style = if i == app.search_selected {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(uuid_short.to_string()),
                Cell::from(title.to_string()),
                Cell::from(idea.priority.as_deref().unwrap_or("-").to_string()),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Percentage(70),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["UUID", "Title", "Priority"])
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .bottom_margin(1),
        )
        .block(Block::default().borders(Borders::NONE));

    frame.render_widget(table, area);
}

fn render_status_bar(frame: &mut Frame, area: Rect, _app: &App) {
    let bar = Line::from(vec![Span::styled(
        " Esc:back  Enter:search/open  Up/Down:navigate ",
        Style::default().fg(Color::DarkGray),
    )]);
    frame.render_widget(Paragraph::new(bar), area);
}
