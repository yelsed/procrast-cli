use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::DefaultTerminal;
use std::time::Duration;

use crate::api::client::ApiClient;
use crate::api::models::Idea;
use crate::cache::Cache;

use super::views::{detail, list, search};

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
    pub scroll_offset: u16,
    pub search_query: String,
    pub search_results: Vec<Idea>,
    pub search_selected: usize,
    pub is_offline: bool,
    pub status_message: Option<String>,
    pub should_quit: bool,
    client: ApiClient,
    cache: Option<Cache>,
}

impl App {
    pub fn new(client: ApiClient) -> Self {
        Self {
            screen: Screen::List,
            ideas: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
            is_offline: false,
            status_message: None,
            should_quit: false,
            client,
            cache: Cache::open().ok(),
        }
    }

    pub async fn fetch_ideas(&mut self) {
        match self.client.list_ideas().await {
            Ok(ideas) => {
                if let Some(ref cache) = self.cache {
                    let _ = cache.upsert_ideas(&ideas);
                }
                self.ideas = ideas;
                self.is_offline = false;
                self.status_message = Some(format!("{} ideas loaded", self.ideas.len()));
            }
            Err(_) => {
                // Try cache
                if let Some(ref cache) = self.cache {
                    if let Ok(cached) = cache.get_all_ideas() {
                        if !cached.is_empty() {
                            self.ideas = cached;
                            self.is_offline = true;
                            self.status_message = Some("Offline — showing cached data".to_string());
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

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
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
                            self.handle_detail_input(key.code);
                        }
                        Screen::Search => {
                            self.handle_search_input(key.code).await;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_list_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.ideas.is_empty() {
                    self.selected = (self.selected + 1).min(self.ideas.len() - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Enter => {
                if !self.ideas.is_empty() {
                    self.scroll_offset = 0;
                    self.screen = Screen::Detail(self.selected);
                }
            }
            KeyCode::Char('/') => {
                self.search_query.clear();
                self.search_results.clear();
                self.search_selected = 0;
                self.screen = Screen::Search;
            }
            KeyCode::Char('r') => {
                self.status_message = Some("Refreshing...".to_string());
                self.fetch_ideas().await;
            }
            _ => {}
        }
    }

    fn handle_detail_input(&mut self, key: KeyCode) {
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
