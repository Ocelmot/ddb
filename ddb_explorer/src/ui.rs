use std::io;

use crossbeam::channel::{Receiver, Sender};
use crossterm::{
    event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use tui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    text::{Span, Text},
    widgets::{List, ListItem, Paragraph},
};

pub enum UiMessage {
    Char(char),
    Message(String),
}

pub fn ui_thread(rx: Receiver<UiMessage>, tx: Sender<String>) -> Result<(), io::Error> {
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        // EnableMouseCapture,
        EnableBracketedPaste
    )
    .unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut command = String::new();
    let mut history = Vec::<String>::new();

    loop {
        terminal
            .draw(|f| {
                let constraints = vec![Constraint::Min(5), Constraint::Length(1)];
                let areas = Layout::default()
                    .constraints(constraints)
                    .direction(Direction::Vertical)
                    .split(f.size());

                let history_items = history
                    .iter()
                    .rev()
                    .take(areas[0].height.into())
                    .rev()
                    .map(|line| ListItem::new(Span::raw(line)));

                f.render_widget(List::new(history_items.collect::<Vec<_>>()), areas[0]);
                f.render_widget(Paragraph::new(Text::raw(format!(">>{command}"))), areas[1]);
            })
            .unwrap();

        let msg = rx.recv();
        let Ok(msg) = msg else {
            break;
        };
        match msg {
            UiMessage::Char(new_char) => match new_char {
                '\x08' => {
                    command.pop();
                }
                '\r' | '\n' => {
                    history.push(format!(">{command}"));
					tx.send(command);
                    command = String::new()
                }
                _ => {
                    command.push(new_char);
                }
            },
            UiMessage::Message(s) => history.push(s),
        }
    }
    Ok(())
}
