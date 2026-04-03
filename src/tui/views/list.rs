use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};

use crate::tui::app::App;

fn logo_lines() -> Vec<Line<'static>> {
    let orange = Color::Rgb(231, 144, 65); // #E79041
    let brown = Color::Rgb(139, 94, 60);   // #8B5E3C
    let o = Style::default().fg(orange).add_modifier(Modifier::BOLD);
    let b = Style::default().fg(brown).add_modifier(Modifier::BOLD);

    vec![
        Line::from(Span::styled(r"                                _   ", o)),
        Line::from(vec![
            Span::styled(r" _ __ _ _ ___  __ _ _ __ _ __", o),
            Span::styled(r"| |_ ", o),
        ]),
        Line::from(vec![
            Span::styled(r"| '_ \ '_/ _ \/ _| '_/ _` (_-<", o),
            Span::styled(r"  _|", o),
        ]),
        Line::from(vec![
            Span::styled(r"| .__/_| \___/\__|_| \__,_/__/\__|", o),
        ]),
        Line::from(vec![
            Span::styled("|_|", o),
            Span::styled("  ✦", b),
        ]),
    ]
}

pub fn render(frame: &mut Frame, app: &App) {
    let header_height = if app.is_offline { 9 } else { 7 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height), // header with logo (+ offline banner)
            Constraint::Min(5),               // table
            Constraint::Length(1),            // status bar
        ])
        .split(frame.area());

    render_header(frame, chunks[0], app);
    render_table(frame, chunks[1], app);
    render_status_bar(frame, chunks[2], app);
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines = logo_lines();

    // Empty line
    lines.push(Line::from(""));

    // Status line
    if app.is_offline {
        let age_suffix = match &app.cache_age_text {
            Some(age) => format!(" (last synced: {age})"),
            None => String::new(),
        };

        let (badge, hint) = if app.is_unauthorized {
            (
                " SESSION EXPIRED ",
                "   Press 'l' to login · 'r' to retry",
            )
        } else {
            (
                " OFFLINE ",
                "   Press 'r' to retry · Check your network connection · 'l' to login",
            )
        };

        lines.push(Line::from(vec![
            Span::styled(
                badge,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" showing {} cached ideas{}", app.ideas.len(), age_suffix),
                Style::default().fg(Color::Yellow),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            hint,
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            format!("  {} ideas · Sort: {}", app.ideas.len(), app.sort_mode.label()),
            Style::default().fg(Color::DarkGray),
        )));
    }

    let header = Paragraph::new(lines)
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, area);
}

fn render_table(frame: &mut Frame, area: Rect, app: &App) {
    let orange = Color::Rgb(231, 144, 65);

    let header = Row::new(vec!["UUID", "Title", "Priority", "Status", "Created"])
        .style(Style::default().fg(orange).add_modifier(Modifier::BOLD))
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
                    let mut end = 50.min(idea.content.len());
                    while end > 0 && !idea.content.is_char_boundary(end) {
                        end -= 1;
                    }
                    &idea.content[..end]
                });

            let uuid_short = &idea.uuid[..8.min(idea.uuid.len())];
            let priority = idea.priority.as_deref().unwrap_or("-");
            let (status_text, status_style) = if idea.completed_at.is_some() {
                ("done".to_string(), Style::default().fg(Color::Green))
            } else {
                (match idea.refinement_status.as_deref() {
                    Some("completed") => "refined".to_string(),
                    Some(s) => s.to_string(),
                    None => "-".to_string(),
                }, Style::default())
            };
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
                Cell::from(status_text).style(status_style),
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
    let help_text = if app.is_offline {
        " j/k:navigate  Enter:view  s:sort  /:search  r:retry  l:login  q:quit "
    } else {
        " j/k:navigate  Enter:view  d:done  s:sort  /:search  r:refresh  q:quit "
    };

    let right_text = match (&app.status_message, app.is_offline) {
        (Some(msg), _) => msg.clone(),
        (None, true) => match &app.cache_age_text {
            Some(age) => format!("Last synced {age}"),
            None => String::new(),
        },
        (None, false) => String::new(),
    };

    let bar = Line::from(vec![
        Span::styled(
            help_text,
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            right_text,
            Style::default().fg(Color::Yellow),
        ),
    ]);
    frame.render_widget(Paragraph::new(bar), area);
}
