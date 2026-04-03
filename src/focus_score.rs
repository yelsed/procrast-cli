use chrono::{DateTime, NaiveDate, Utc};
use clap::ValueEnum;

use crate::api::models::Idea;

#[derive(Debug, Clone, Copy, PartialEq, Default, ValueEnum)]
pub enum SortMode {
    #[default]
    Smart,
    Priority,
    Newest,
    Oldest,
}

impl SortMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Smart => "SMART",
            Self::Priority => "PRIORITY",
            Self::Newest => "NEWEST",
            Self::Oldest => "OLDEST",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Smart => Self::Priority,
            Self::Priority => Self::Newest,
            Self::Newest => Self::Oldest,
            Self::Oldest => Self::Smart,
        }
    }
}

impl std::fmt::Display for SortMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Smart => write!(f, "smart"),
            Self::Priority => write!(f, "priority"),
            Self::Newest => write!(f, "newest"),
            Self::Oldest => write!(f, "oldest"),
        }
    }
}

pub fn calculate_focus_score(idea: &Idea) -> u32 {
    priority_score(idea)
        + due_date_score(idea)
        + age_score(idea)
        + refinement_score(idea)
        + content_depth_score(idea)
}

fn priority_score(idea: &Idea) -> u32 {
    match idea.priority.as_deref() {
        Some("high") => 30,
        Some("medium") => 15,
        Some("low") => 5,
        _ => 0,
    }
}

fn due_date_score(idea: &Idea) -> u32 {
    let due = match idea.due_date.as_deref() {
        Some(s) => match NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => return 0,
        },
        None => return 0,
    };

    let today = Utc::now().date_naive();
    let days_until = (due - today).num_days();

    if days_until < 0 {
        25 // Overdue
    } else if days_until <= 3 {
        20
    } else if days_until <= 7 {
        15
    } else if days_until <= 30 {
        10
    } else {
        5
    }
}

fn age_score(idea: &Idea) -> u32 {
    let created = match DateTime::parse_from_rfc3339(&idea.created_at) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => return 0,
    };

    let age_days = (Utc::now() - created).num_days();

    if age_days < 3 {
        5
    } else if age_days <= 7 {
        12
    } else if age_days <= 14 {
        20 // Peak "don't forget" window
    } else if age_days <= 30 {
        15
    } else if age_days <= 60 {
        8
    } else if age_days <= 90 {
        4
    } else {
        2
    }
}

fn refinement_score(idea: &Idea) -> u32 {
    if idea.refinement_status.as_deref() != Some("completed") {
        return 0;
    }

    let has_steps = idea
        .action_steps
        .as_ref()
        .is_some_and(|steps| !steps.is_empty());

    if has_steps { 20 } else { 15 }
}

fn content_depth_score(idea: &Idea) -> u32 {
    let has_audio = idea
        .audio_path
        .as_ref()
        .is_some_and(|p| !p.is_empty());
    let is_long = idea.content.len() > 500;

    if has_audio || is_long { 5 } else { 0 }
}

fn priority_rank(idea: &Idea) -> u32 {
    match idea.priority.as_deref() {
        Some("high") => 0,
        Some("medium") => 1,
        Some("low") => 2,
        _ => 3,
    }
}

pub fn sort_ideas(ideas: &mut Vec<Idea>, mode: SortMode) {
    match mode {
        SortMode::Smart => {
            ideas.sort_unstable_by(|a, b| {
                calculate_focus_score(b).cmp(&calculate_focus_score(a))
            });
        }
        SortMode::Priority => {
            ideas.sort_unstable_by(|a, b| {
                let rank = priority_rank(a).cmp(&priority_rank(b));
                if rank != std::cmp::Ordering::Equal {
                    rank
                } else {
                    b.created_at.cmp(&a.created_at)
                }
            });
        }
        SortMode::Newest => {
            ideas.sort_unstable_by(|a, b| b.created_at.cmp(&a.created_at));
        }
        SortMode::Oldest => {
            ideas.sort_unstable_by(|a, b| a.created_at.cmp(&b.created_at));
        }
    }
}
