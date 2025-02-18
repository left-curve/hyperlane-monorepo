use std::{
    io::{stdout, BufRead},
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::Duration,
};

use crossterm::{execute, terminal::EnterAlternateScreen};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    widgets::{Block, Borders, List, ListState},
    Terminal,
};

use super::scope_child::ScopeChild;

pub struct Printer {
    _handle: thread::JoinHandle<()>,
    messages: Arc<Mutex<Vec<String>>>,
    agent: Arc<Mutex<Option<ScopeChild>>>,
    dango: Arc<Mutex<Option<ScopeChild>>>,
}

impl Printer {
    pub fn new() -> Self {
        let messages: Arc<Mutex<Vec<String>>> = Default::default();

        let thread_messages = messages.clone();

        let agent = Arc::new(Mutex::new(None::<ScopeChild>));
        let dango = Arc::new(Mutex::new(None::<ScopeChild>));

        let thread_agent = agent.clone();
        let thread_dango = dango.clone();

        let _handle = thread::spawn(move || {
            let mut stdout = stdout();
            execute!(stdout, EnterAlternateScreen).unwrap();
            let backend = CrosstermBackend::new(stdout);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut agent_messages = vec![];
            let mut dango_messages = vec![];

            loop {
                let messages: Vec<String> = thread_messages.lock().unwrap().clone();

                if let Some(agent) = thread_agent.lock().unwrap().as_deref_mut() {
                    if let Some(std_out) = agent.stdout.take() {
                        let new_lines = std::io::BufReader::new(std_out)
                            .lines()
                            .collect::<Result<Vec<_>, _>>()
                            .unwrap();

                        agent_messages.extend(new_lines);
                    }
                }

                if let Some(dango) = thread_dango.lock().unwrap().as_deref_mut() {
                    if let Some(std_out) = dango.stdout.take() {
                        let new_lines = std::io::BufReader::new(std_out)
                            .lines()
                            .collect::<Result<Vec<_>, _>>()
                            .unwrap();

                        dango_messages.extend(new_lines);
                    }
                }

                draw(
                    &mut terminal,
                    messages,
                    agent_messages.clone(),
                    dango_messages.clone(),
                );

                sleep(Duration::from_millis(50));
            }
        });

        Self {
            _handle,
            messages,
            agent,
            dango,
        }
    }

    pub fn add_message(&self, message: &str) {
        self.messages.lock().unwrap().push(message.to_string());
    }

    pub fn set_agent(&self, agent: ScopeChild) {
        *self.agent.lock().unwrap() = Some(agent);
    }

    pub fn set_dango(&self, dango: ScopeChild) {
        *self.dango.lock().unwrap() = Some(dango);
    }
}

fn draw<B: ratatui::prelude::Backend>(
    terminal: &mut Terminal<B>,
    main_messages: Vec<String>,
    dango_messages: Vec<String>,
    agent_messages: Vec<String>,
) {
    terminal
        .draw(|f| {
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(f.area());

            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[1]);

            let mut main_state = ListState::default().with_selected(select_message(&main_messages));
            let mut dango_state =
                ListState::default().with_selected(select_message(&dango_messages));
            let mut agent_state =
                ListState::default().with_selected(select_message(&agent_messages));

            let left_panel = List::new(main_messages)
                .block(Block::default().title("Main").borders(Borders::ALL));

            let top_right_panel = List::new(dango_messages)
                .block(Block::default().title("Dango").borders(Borders::ALL));

            let bottom_right_panel = List::new(agent_messages)
                .block(Block::default().title("Agent").borders(Borders::ALL));

            f.render_stateful_widget(left_panel, main_chunks[0], &mut main_state);
            f.render_stateful_widget(top_right_panel, right_chunks[0], &mut dango_state);
            f.render_stateful_widget(bottom_right_panel, right_chunks[1], &mut agent_state);
        })
        .unwrap();
}

fn select_message(messages: &[String]) -> Option<usize> {

    if messages.len() == 0 {
        return None;
    } else {
        return Some(messages.len() - 1);
    }
}