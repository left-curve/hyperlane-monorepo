use {
    super::scope_child::ScopeChild,
    ansi_regex::ansi_regex,
    crossterm::event::{Event, KeyCode},
    ratatui::{
        layout::{Constraint, Direction, Layout},
        widgets::{Block, Borders, List, ListState},
        Terminal,
    },
    std::{
        io::BufRead,
        sync::{Arc, LazyLock, Mutex},
        thread::{self, sleep},
        time::Duration,
    },
};

pub static PRINTER: LazyLock<Printer> = LazyLock::new(|| Printer::new());

pub struct Printer {
    pub handle: thread::JoinHandle<()>,
    messages: Arc<Mutex<Vec<String>>>,
    pub agent: Arc<Mutex<Option<ScopeChild>>>,
    pub dango: Arc<Mutex<Option<ScopeChild>>>,
    search_agent: Arc<Mutex<Option<SearchMessage>>>,
}

impl Printer {
    fn new() -> Self {
        let messages: Arc<Mutex<Vec<String>>> = Default::default();

        let thread_messages = messages.clone();

        let agent = Arc::new(Mutex::new(None::<ScopeChild>));
        let dango = Arc::new(Mutex::new(None::<ScopeChild>));

        let thread_agent = agent.clone();
        let thread_dango = dango.clone();

        let search_agent: Arc<Mutex<Option<SearchMessage>>> = Default::default();
        let thread_search_message = search_agent.clone();

        let temp_messages = messages.clone();

        let _handle = thread::spawn(move || {
            let mut terminal = ratatui::init();

            let agent_ok_messages: Arc<Mutex<Vec<String>>> = Default::default();
            let agent_err_messages: Arc<Mutex<Vec<String>>> = Default::default();
            let dango_messages: Arc<Mutex<Vec<String>>> = Default::default();

            let thread_agent_ok_messages = agent_ok_messages.clone();
            let thread_agent_err_messages = agent_err_messages.clone();
            let thread_dango_messages = dango_messages.clone();

            // Agent
            thread::spawn(move || {
                while thread_agent.lock().unwrap().is_none() {
                    sleep(Duration::from_millis(50));
                }

                thread_agent_ok_messages
                    .lock()
                    .unwrap()
                    .push("build agent bin...".to_string());

                let mut agent = thread_agent.lock().unwrap().take().unwrap();

                let std_out = agent.stdout.take().unwrap();
                let std_err = agent.stderr.take().unwrap();

                // StdOut - Ok
                thread::spawn(move || {
                    let regex = ansi_regex();

                    for line in std::io::BufReader::new(std_out).lines() {
                        let line = line.unwrap();
                        let clear_line = regex.replace_all(&line, "");
                        thread_agent_ok_messages
                            .lock()
                            .unwrap()
                            .push(clear_line.clone().to_string());

                        if let Some(search) = thread_search_message.lock().unwrap().as_mut() {
                            if line.contains(&search.search_text) {
                                search.message = Some(clear_line.to_string());
                            }
                        }
                    }
                });

                // StdErr - Err
                thread::spawn(move || {
                    let regex = ansi_regex();

                    for line in std::io::BufReader::new(std_err).lines() {
                        let line = line.unwrap();
                        let clear_line = regex.replace_all(&line, "");
                        thread_agent_err_messages
                            .lock()
                            .unwrap()
                            .push(clear_line.to_string());
                    }

                    match agent.try_wait() {
                        Ok(code) => match code {
                            Some(status) => temp_messages.lock().unwrap().push(format!(
                                "agent.try_wait() is ok, code is some and status is: {}",
                                status
                            )),

                            None => temp_messages
                                .lock()
                                .unwrap()
                                .push(format!("agent.try_wait() is ok but code is None")),
                        },
                        Err(err) => temp_messages
                            .lock()
                            .unwrap()
                            .push(format!("agent.try_wait() is error:{}", err)),
                    };
                });
            });

            // Dango
            thread::spawn(move || {
                while thread_dango.lock().unwrap().is_none() {
                    sleep(Duration::from_millis(50));
                }

                let mut agent = thread_dango.lock().unwrap().take().unwrap();
                let std_out = agent.stdout.take().expect("Dango std_out not sett!");
                let regex = ansi_regex();

                for line in std::io::BufReader::new(std_out).lines() {
                    let line = line.unwrap();
                    let clear_line = regex.replace_all(&line, "");
                    thread_dango_messages
                        .lock()
                        .unwrap()
                        .push(clear_line.to_string());
                }
            });

            let scroll: Arc<Mutex<usize>> = Default::default();

            let scroll_thread = scroll.clone();

            // scroll thread
            thread::spawn(move || loop {
                if let Event::Key(k) = crossterm::event::read().unwrap() {
                    match k.code {
                        KeyCode::Up => {
                            let mut scroll = scroll_thread.lock().unwrap();
                            *scroll += 1;
                        }

                        KeyCode::Down => {
                            let mut scroll = scroll_thread.lock().unwrap();
                            *scroll = scroll.saturating_sub(1);
                        }
                        _ => {}
                    }
                }
            });

            loop {
                for i in [
                    &thread_messages,
                    &agent_ok_messages,
                    &agent_err_messages,
                    &dango_messages,
                ] {
                    resize_messages(i);
                }

                draw(
                    &mut terminal,
                    thread_messages.lock().unwrap().clone(),
                    dango_messages.lock().unwrap().clone(),
                    agent_ok_messages.lock().unwrap().clone(),
                    agent_err_messages.lock().unwrap().clone(),
                    *scroll.lock().unwrap(),
                );

                sleep(Duration::from_millis(50));
            }
        });

        Self {
            handle: _handle,
            messages,
            agent,
            dango,
            search_agent,
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

    pub fn block_for_agent_submsg(&self, msg: &str) -> String {
        *self.search_agent.lock().unwrap() = Some(SearchMessage {
            search_text: msg.to_string(),
            message: None,
        });

        let response = loop {
            if let Some(search) = self.search_agent.lock().unwrap().as_ref() {
                if let Some(message) = &search.message {
                    // response = Some(message.clone());
                    break message.clone();
                }
            }
        };

        // reset search
        *self.search_agent.lock().unwrap() = None;
        response
    }
}

impl Drop for Printer {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

fn draw<B: ratatui::prelude::Backend>(
    terminal: &mut Terminal<B>,
    main_messages: Vec<String>,
    dango_messages: Vec<String>,
    agent_ok_messages: Vec<String>,
    agent_err_messages: Vec<String>,
    scroll: usize,
) {
    terminal
        .draw(|f| {
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(f.area());

            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[0]);

            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[1]);

            let mut main_state = ListState::default().with_selected(select_message(&main_messages));

            let mut dango_state =
                ListState::default().with_selected(select_message(&dango_messages));

            let mut agent_ok_state =
                ListState::default().with_selected(select_message(&agent_ok_messages));

            agent_ok_state.scroll_up_by(scroll as u16);

            let mut agent_err_state =
                ListState::default().with_selected(select_message(&agent_err_messages));

            let top_left_panel = List::new(main_messages)
                .block(Block::default().title("Main").borders(Borders::ALL));

            let bottom_left_panel = List::new(agent_err_messages)
                .block(Block::default().title("Agent Err").borders(Borders::ALL));

            let top_right_panel = List::new(dango_messages)
                .block(Block::default().title("Dango").borders(Borders::ALL));

            let bottom_right_panel = List::new(agent_ok_messages)
                .block(Block::default().title("Agent Ok").borders(Borders::ALL));

            f.render_stateful_widget(top_left_panel, left_chunks[0], &mut main_state);
            f.render_stateful_widget(bottom_left_panel, left_chunks[1], &mut agent_err_state);
            f.render_stateful_widget(top_right_panel, right_chunks[0], &mut dango_state);
            f.render_stateful_widget(bottom_right_panel, right_chunks[1], &mut agent_ok_state);
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

fn resize_messages(messages: &Arc<Mutex<Vec<String>>>) {
    let mut messages = messages.lock().unwrap();
    let len = messages.len();
    if len > 500 {
        messages.drain(..len - 500);
    }
}

#[macro_export]
macro_rules! dprintln {
    ($($arg:tt)*) => {
        crate::utils::printer::PRINTER.add_message(&format!($($arg)*));
    };
}

struct SearchMessage {
    pub search_text: String,
    pub message: Option<String>,
}
