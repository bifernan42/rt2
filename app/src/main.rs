//! # [Ratatui] User Input example
//!
//! The latest version of this example is available in the [examples] folder in the repository.
//!
//! Please note that the examples are designed to be run against the `main` branch of the Github
//! repository. This means that you may not be able to compile with the latest release version on
//! crates.io, or the one that you have installed locally.
//!
//! See the [examples readme] for more information on finding examples that match the version of the
//! library you are using.
//!
//! [Ratatui]: https://github.com/ratatui/ratatui
//! [examples]: https://github.com/ratatui/ratatui/blob/main/examples
//! [examples readme]: https://github.com/ratatui/ratatui/blob/main/examples/README.md

use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use color_eyre::Result;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Position},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, List, ListItem, Paragraph},
};

fn main() -> Result<()> {
    let server_addr = "127.0.0.1:42069";

    let (network_tx, network_rx) = mpsc::channel::<String>();
    let (message_tx, message_rx) = mpsc::channel::<String>();

    let network_handle = thread::spawn(move || {
        let mut stream = TcpStream::connect(server_addr).expect("connect failed");
        stream
            .set_nonblocking(true)
            .expect("failed to set non blocking");

        let mut read_buf = Vec::new();
        let mut buf = [0u8; 2048];

        loop {
            while let Ok(msg) = network_rx.try_recv() {
                let mut line = msg;
                if !line.ends_with('\n') {
                    line.push('\n');
                }
                let _ = stream.write_all(line.as_bytes());
            }

            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(size) => {
                    read_buf.extend_from_slice(&buf[..size]);
                    loop {
                        if let Some(pos) = read_buf.iter().position(|&b| b == b'\n') {
                            let line: Vec<u8> = read_buf.drain(..=pos).collect();
                            if let Ok(mut s) = String::from_utf8(line) {
                                if s.ends_with('\n') {
                                    s.pop();
                                }
                                if s.ends_with('\r') {
                                    s.pop();
                                }
                                let _ = message_tx.send(s);
                            }
                        } else {
                            break;
                        }
                    }
                }
                Err(_) => {}
            }
            thread::sleep(Duration::from_millis(10));
        }
    });

    color_eyre::install()?;
    let terminal = ratatui::init();
    let app_result = App::new(network_tx, message_rx).run(terminal);
    ratatui::restore();
    network_handle.join().ok();
    app_result
}

struct App {
    input: String,
    network_tx: Sender<String>,
    message_rx: Receiver<String>,
    character_index: usize,
    input_mode: InputMode,
    messages: Vec<String>,
}

enum InputMode {
    Normal,
    Editing,
}

impl App {
    const fn new(
        network_tx: Sender<String>,
        message_rx: Receiver<String>,
    ) -> Self {
        Self {
            input: String::new(),
            network_tx: network_tx,
            message_rx: message_rx,
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            character_index: 0,
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn delete_char(&mut self) {
        if self.character_index != 0 {
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(current_index);

            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    fn submit_message(&mut self) {
        let msg = self.input.clone();
        self.network_tx.send(msg).ok();
        self.input.clear();
        self.reset_cursor();
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            while let Ok(msg) = self.message_rx.try_recv() {
                self.messages.push(msg);
            }
            terminal.draw(|frame| self.draw(frame))?;
            if let Event::Key(key) = event::read()? {
                match self.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('e') => self.input_mode = InputMode::Editing,
                        KeyCode::Char('q') => return Ok(()),
                        _ => {}
                    },
                    InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Enter => self.submit_message(),
                        KeyCode::Char(to_insert) => self.enter_char(to_insert),
                        KeyCode::Backspace => self.delete_char(),
                        KeyCode::Left => self.move_cursor_left(),
                        KeyCode::Right => self.move_cursor_right(),
                        KeyCode::Esc => self.input_mode = InputMode::Normal,
                        _ => {}
                    },
                    InputMode::Editing => {}
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let vertical = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(3),
        ]);

        // ✅ ORDRE MODIFIÉ ICI
        let [messages_area, help_area, input_area] = vertical.areas(frame.area());

        let (msg, style) = match self.input_mode {
            InputMode::Normal => (
                vec![
                    "Press ".into(),
                    "q".bold(),
                    " to exit, ".into(),
                    "e".bold(),
                    " to start editing.".bold(),
                ],
                Style::default().add_modifier(Modifier::RAPID_BLINK),
            ),
            InputMode::Editing => (
                vec![
                    "Press ".into(),
                    "Esc".bold(),
                    " to stop editing, ".into(),
                    "Enter".bold(),
                    " to record the message".into(),
                ],
                Style::default(),
            ),
        };

        let text = Text::from(Line::from(msg)).patch_style(style);
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, help_area);

        let input = Paragraph::new(self.input.as_str())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::bordered().title("Input"));
        frame.render_widget(input, input_area);

        match self.input_mode {
            InputMode::Normal => {}
            InputMode::Editing => frame.set_cursor_position(Position::new(
                input_area.x + self.character_index as u16 + 1,
                input_area.y + 1,
            )),
        }

        let messages: Vec<ListItem> = self
            .messages
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let content = Line::from(Span::raw(format!("{i}: {m}")));
                ListItem::new(content)
            })
            .collect();

        let messages = List::new(messages).block(Block::bordered().title("Messages"));
        frame.render_widget(messages, messages_area);
    }
}
