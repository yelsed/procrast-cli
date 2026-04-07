use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::DefaultTerminal;
use std::time::{Duration, Instant};

use crate::api::client::{ApiClient, ApiError};
use crate::api::models::Idea;
use crate::cache::Cache;
use crate::focus_score::{self, SortMode};

use super::views::{detail, list, search};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Tab {
    #[default]
    Open,
    Done,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    List,
    Detail(usize), // index into ideas vec
    Search,
}

pub struct App {
    pub screen: Screen,
    pub ideas: Vec<Idea>,
    pub selected: usize,
    pub selected_done: usize,
    pub active_tab: Tab,
    pub scroll_offset: u16,
    pub search_query: String,
    pub search_results: Vec<Idea>,
    pub search_selected: usize,
    pub is_offline: bool,
    pub is_unauthorized: bool,
    pub cache_age_text: Option<String>,
    pub status_message: Option<String>,
    pub sort_mode: SortMode,
    pub should_quit: bool,
    pub quit_to_login: bool,
    pending_refresh: bool,
    pending_complete: Option<usize>,
    last_retry: Option<Instant>,
    client: ApiClient,
    cache: Option<Cache>,
}

impl App {
    pub fn new(client: ApiClient) -> Self {
        Self {
            screen: Screen::List,
            ideas: Vec::new(),
            selected: 0,
            selected_done: 0,
            active_tab: Tab::Open,
            scroll_offset: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
            is_offline: false,
            is_unauthorized: false,
            cache_age_text: None,
            status_message: None,
            sort_mode: SortMode::Smart,
            should_quit: false,
            quit_to_login: false,
            pending_refresh: false,
            pending_complete: None,
            last_retry: None,
            client,
            cache: Cache::open().ok(),
        }
    }

    pub fn apply_sort(&mut self) {
        focus_score::sort_ideas(&mut self.ideas, self.sort_mode);
    }

    pub fn filtered_indices(&self) -> Vec<usize> {
        self.ideas
            .iter()
            .enumerate()
            .filter(|(_, idea)| match self.active_tab {
                Tab::Open => idea.completed_at.is_none(),
                Tab::Done => idea.completed_at.is_some(),
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn tab_selected(&self) -> usize {
        match self.active_tab {
            Tab::Open => self.selected,
            Tab::Done => self.selected_done,
        }
    }

    pub fn tab_selected_mut(&mut self) -> &mut usize {
        match self.active_tab {
            Tab::Open => &mut self.selected,
            Tab::Done => &mut self.selected_done,
        }
    }

    pub fn selected_idea_index(&self) -> Option<usize> {
        let indices = self.filtered_indices();
        indices.get(self.tab_selected()).copied()
    }

    fn clamp_selections(&mut self) {
        let open_count = self.ideas.iter().filter(|i| i.completed_at.is_none()).count();
        let done_count = self.ideas.iter().filter(|i| i.completed_at.is_some()).count();
        self.selected = if open_count > 0 { self.selected.min(open_count - 1) } else { 0 };
        self.selected_done = if done_count > 0 { self.selected_done.min(done_count - 1) } else { 0 };
    }

    pub async fn fetch_ideas(&mut self) {
        match self.client.list_ideas().await {
            Ok(ideas) => {
                if let Some(ref cache) = self.cache {
                    let _ = cache.upsert_ideas(&ideas);
                    let _ = cache.set_last_synced();
                }
                self.ideas = ideas;
                self.is_offline = false;
                self.is_unauthorized = false;
                self.cache_age_text = None;
                self.apply_sort();
                self.selected = 0;
                self.selected_done = 0;
                self.status_message = Some(format!("{} ideas loaded", self.ideas.len()));
            }
            Err(e) => {
                let unauthorized = e.downcast_ref::<ApiError>()
                    .is_some_and(|ae| matches!(ae, ApiError::Unauthorized));
                self.is_unauthorized = unauthorized;

                // Try cache
                if let Some(ref cache) = self.cache {
                    if let Ok(cached) = cache.get_all_ideas() {
                        if !cached.is_empty() {
                            let was_retry = !self.ideas.is_empty();
                            self.ideas = cached;
                            self.is_offline = true;
                            self.cache_age_text = cache.last_synced_age_text();
                            self.apply_sort();
                            self.status_message = if was_retry {
                                if unauthorized {
                                    Some("Re-login required — run 'procrast login'".to_string())
                                } else {
                                    Some("Could not connect — still offline".to_string())
                                }
                            } else {
                                None
                            };
                            return;
                        }
                    }
                }
                self.status_message = Some("Failed to fetch ideas".to_string());
            }
        }
    }

    pub async fn search_api(&mut self) {
        if self.search_query.is_empty() {
            return;
        }
        match self.client.search_ideas(&self.search_query).await {
            Ok(results) => {
                self.search_results = results;
                self.search_selected = 0;
            }
            Err(_) => {
                // Local fallback
                if let Some(ref cache) = self.cache {
                    if let Ok(results) = cache.search_ideas(&self.search_query) {
                        self.search_results = results;
                        self.search_selected = 0;
                        self.is_offline = true;
                        return;
                    }
                }
                self.status_message = Some("Search failed".to_string());
            }
        }
    }

    pub fn current_idea(&self) -> Option<&Idea> {
        match &self.screen {
            Screen::Detail(idx) => self.ideas.get(*idx),
            _ => None,
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<bool> {
        self.fetch_ideas().await;

        loop {
            terminal.draw(|frame| {
                match &self.screen {
                    Screen::List => list::render(frame, &self),
                    Screen::Detail(_) => detail::render(frame, &self),
                    Screen::Search => search::render(frame, &self),
                }
            })?;

            if self.should_quit {
                break;
            }

            if self.pending_refresh {
                self.pending_refresh = false;
                self.fetch_ideas().await;
                continue; // re-render immediately with results
            }

            if let Some(idx) = self.pending_complete.take() {
                if let Some(idea) = self.ideas.get(idx) {
                    let uuid = idea.uuid.clone();
                    match self.client.toggle_complete(&uuid).await {
                        Ok(updated) => {
                            let is_done = updated.completed_at.is_some();
                            if let Some(ref cache) = self.cache {
                                let _ = cache.upsert_ideas(&[updated.clone()]);
                            }
                            self.ideas[idx] = updated;
                            self.clamp_selections();
                            self.status_message = Some(if is_done {
                                "✓ Marked as done".to_string()
                            } else {
                                "☐ Unmarked".to_string()
                            });
                        }
                        Err(e) => {
                            if e.downcast_ref::<ApiError>()
                                .is_some_and(|ae| matches!(ae, ApiError::Unauthorized))
                            {
                                self.status_message =
                                    Some("Session expired — run 'procrast login'".to_string());
                            } else {
                                self.status_message =
                                    Some(format!("Failed to toggle: {}", e));
                            }
                        }
                    }
                }
                continue;
            }

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    // Global quit: Ctrl+C
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        break;
                    }

                    match &self.screen.clone() {
                        Screen::List => {
                            self.handle_list_input(key.code).await;
                        }
                        Screen::Detail(_) => {
                            self.handle_detail_input(key.code).await;
                        }
                        Screen::Search => {
                            self.handle_search_input(key.code).await;
                        }
                    }
                }
            }
        }

        Ok(self.quit_to_login)
    }

    async fn handle_list_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Tab => {
                self.active_tab = match self.active_tab {
                    Tab::Open => Tab::Done,
                    Tab::Done => Tab::Open,
                };
                self.status_message = None;
            }
            KeyCode::Char('1') => {
                self.active_tab = Tab::Open;
                self.status_message = None;
            }
            KeyCode::Char('2') => {
                self.active_tab = Tab::Done;
                self.status_message = None;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let count = self.filtered_indices().len();
                if count > 0 {
                    let sel = self.tab_selected_mut();
                    *sel = (*sel + 1).min(count - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let sel = self.tab_selected_mut();
                *sel = sel.saturating_sub(1);
            }
            KeyCode::Enter => {
                if let Some(real_idx) = self.selected_idea_index() {
                    self.scroll_offset = 0;
                    self.screen = Screen::Detail(real_idx);
                }
            }
            KeyCode::Char('/') => {
                self.search_query.clear();
                self.search_results.clear();
                self.search_selected = 0;
                self.screen = Screen::Search;
            }
            KeyCode::Char('r') => {
                let cooldown = Duration::from_secs(3);
                if self.last_retry.is_none_or(|t| t.elapsed() >= cooldown) {
                    self.last_retry = Some(Instant::now());
                    self.status_message = Some("Retrying...".to_string());
                    self.pending_refresh = true;
                }
            }
            KeyCode::Char('d') if !self.is_offline => {
                if let Some(real_idx) = self.selected_idea_index() {
                    self.pending_complete = Some(real_idx);
                }
            }
            KeyCode::Char('s') => {
                self.sort_mode = self.sort_mode.next();
                self.apply_sort();
                self.selected = 0;
                self.selected_done = 0;
                self.status_message = Some(format!("Sort: {}", self.sort_mode.label()));
            }
            KeyCode::Char('l') if self.is_offline => {
                self.quit_to_login = true;
                self.should_quit = true;
            }
            _ => {}
        }
    }

    async fn handle_detail_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.screen = Screen::List;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            KeyCode::Char('d') if !self.is_offline => {
                if let Screen::Detail(idx) = self.screen {
                    self.pending_complete = Some(idx);
                }
            }
            KeyCode::Char('y') => {
                if let Some(idea) = self.current_idea() {
                    let md = crate::markdown::idea_to_markdown(idea);
                    match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(md)) {
                        Ok(()) => {
                            self.status_message =
                                Some("Copied markdown to clipboard".to_string());
                        }
                        Err(_) => {
                            self.status_message =
                                Some("Failed to copy to clipboard".to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    async fn handle_search_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.screen = Screen::List;
            }
            KeyCode::Enter => {
                if !self.search_query.is_empty() && self.search_results.is_empty() {
                    self.search_api().await;
                } else if !self.search_results.is_empty() {
                    // Open selected search result — find it in main ideas list
                    if let Some(result) = self.search_results.get(self.search_selected) {
                        let uuid = result.uuid.clone();
                        // Find in ideas or add it
                        let idx = self
                            .ideas
                            .iter()
                            .position(|i| i.uuid == uuid)
                            .unwrap_or_else(|| {
                                self.ideas
                                    .insert(0, self.search_results[self.search_selected].clone());
                                0
                            });
                        self.scroll_offset = 0;
                        self.screen = Screen::Detail(idx);
                    }
                }
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.search_results.clear();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.search_results.clear();
            }
            KeyCode::Down => {
                if !self.search_results.is_empty() {
                    self.search_selected =
                        (self.search_selected + 1).min(self.search_results.len() - 1);
                }
            }
            KeyCode::Up => {
                self.search_selected = self.search_selected.saturating_sub(1);
            }
            _ => {}
        }
    }
}
