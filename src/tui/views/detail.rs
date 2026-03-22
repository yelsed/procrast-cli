use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::tui::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(5),    // content
            Constraint::Length(1), // status bar
        ])
        .split(frame.area());

    render_header(frame, chunks[0], app);
    render_content(frame, chunks[1], app);
    render_status_bar(frame, chunks[2], app);
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let title = app
        .current_idea()
        .and_then(|i| i.summary_title.as_deref())
        .unwrap_or("Idea Detail");

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", title),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, area);
}

fn render_content(frame: &mut Frame, area: Rect, app: &App) {
    let idea = match app.current_idea() {
        Some(idea) => idea,
        None => {
            frame.render_widget(Paragraph::new("No idea selected"), area);
            return;
        }
    };

    let mut lines: Vec<Line> = Vec::new();

    // Original idea
    lines.push(Line::from(Span::styled(
        "Original Idea",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    for line in idea.content.lines() {
        lines.push(Line::from(line.to_string()));
    }
    lines.push(Line::from(""));

    // Refined version
    if let Some(ref refined) = idea.refined_content {
        if !refined.is_empty() {
            lines.push(Line::from(Span::styled(
                "Refined Version",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            for line in refined.lines() {
                lines.push(Line::from(line.to_string()));
            }
            lines.push(Line::from(""));
        }
    }

    // Summary
    if let Some(ref summary) = idea.refinement_summary {
        if !summary.is_empty() {
            lines.push(Line::from(Span::styled(
                "Summary",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            for line in summary.lines() {
                lines.push(Line::from(line.to_string()));
            }
            lines.push(Line::from(""));
        }
    }

    // Action steps
    if let Some(ref steps) = idea.action_steps {
        if !steps.is_empty() {
            lines.push(Line::from(Span::styled(
                "Action Steps",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            for (i, step) in steps.iter().enumerate() {
                lines.push(Line::from(format!("  {}. {}", i + 1, step)));
            }
            lines.push(Line::from(""));
        }
    }

    // Metadata
    lines.push(Line::from(Span::styled(
        "─".repeat(40),
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(vec![
        Span::styled("Priority: ", Style::default().fg(Color::DarkGray)),
        Span::raw(idea.priority.as_deref().unwrap_or("None")),
        Span::styled("  Due: ", Style::default().fg(Color::DarkGray)),
        Span::raw(idea.due_date.as_deref().unwrap_or("None")),
        Span::styled("  Status: ", Style::default().fg(Color::DarkGray)),
        Span::raw(idea.refinement_status.as_deref().unwrap_or("-")),
    ]));
    lines.push(Line::from(vec![
        Span::styled("UUID: ", Style::default().fg(Color::DarkGray)),
        Span::raw(&idea.uuid),
        Span::styled("  Created: ", Style::default().fg(Color::DarkGray)),
        Span::raw(&idea.created_at[..10.min(idea.created_at.len())]),
    ]));
    if let Some(ref completed) = idea.completed_at {
        lines.push(Line::from(vec![
            Span::styled("Completed: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &completed[..completed.len().min(10)],
                Style::default().fg(Color::Green),
            ),
        ]));
    }

    let content = Paragraph::new(Text::from(lines))
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0))
        .block(Block::default().borders(Borders::NONE));

    frame.render_widget(content, area);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let msg = app.status_message.as_deref().unwrap_or("");

    let bar = Line::from(vec![
        Span::styled(
            " Esc:back  j/k:scroll  y:copy markdown  q:quit ",
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(msg, Style::default().fg(Color::Yellow)),
    ]);
    frame.render_widget(Paragraph::new(bar), area);
}
