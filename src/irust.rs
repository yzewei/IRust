use crossterm::{
    Crossterm, InputEvent, KeyEvent, Terminal, TerminalColor, TerminalCursor, TerminalInput,
};

use crate::history::History;
use crate::racer::Racer;
use crate::repl::Repl;
mod art;
mod cursor;
mod events;
mod format;
mod help;
pub mod options;
mod parser;
mod printer;
mod writer;
use cursor::Cursor;
use options::Options;
use printer::Printer;

const IN: &str = "In: ";
const OUT: &str = "Out: ";

pub struct IRust {
    cursor: TerminalCursor,
    terminal: Terminal,
    input: TerminalInput,
    printer: Printer,
    color: TerminalColor,
    buffer: String,
    repl: Repl,
    internal_cursor: Cursor,
    history: History,
    pub options: Options,
    racer: Racer,
}

impl IRust {
    pub fn new() -> Self {
        let crossterm = Crossterm::new();
        let cursor = crossterm.cursor();
        let terminal = crossterm.terminal();
        let input = crossterm.input();
        let printer = Printer::default();
        let color = crossterm.color();
        let buffer = String::new();
        let repl = Repl::new();
        let history = History::default();
        let options = Options::new().unwrap_or_default();
        let internal_cursor = Cursor::new(0, 1);
        let racer = Racer::start().unwrap();

        IRust {
            cursor,
            terminal,
            input,
            printer,
            color,
            buffer,
            repl,
            history,
            options,
            internal_cursor,
            racer,
        }
    }

    fn prepare(&mut self) -> std::io::Result<()> {
        self.repl.prepare_ground()?;
        self.welcome()?;
        self.write_in()?;
        Ok(())
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        self.prepare()?;
        let mut stdin = self.input.read_sync();
        let _screen = crossterm::RawScreen::into_raw_mode()?;

        loop {
            if let Some(key_event) = stdin.next() {
                match key_event {
                    InputEvent::Keyboard(KeyEvent::Char(c)) => {
                        self.handle_character(c)?;
                    }
                    InputEvent::Keyboard(KeyEvent::Left) => {
                        self.handle_left()?;
                    }
                    InputEvent::Keyboard(KeyEvent::Right) => {
                        self.handle_right()?;
                    }
                    InputEvent::Keyboard(KeyEvent::Up) => {
                        self.handle_up()?;
                    }
                    InputEvent::Keyboard(KeyEvent::Down) => {
                        self.handle_down()?;
                    }
                    InputEvent::Keyboard(KeyEvent::Backspace) => {
                        self.handle_backspace()?;
                    }
                    InputEvent::Keyboard(KeyEvent::Ctrl('c')) => {
                        self.handle_ctrl_c()?;
                    }
                    InputEvent::Keyboard(KeyEvent::Ctrl('d')) => {
                        self.handle_ctrl_d()?;
                    }
                    InputEvent::Keyboard(KeyEvent::Ctrl('z')) => {
                        self.handle_ctrl_z()?;
                    }
                    InputEvent::Keyboard(KeyEvent::Ctrl('l')) => {
                        self.clear()?;
                    }
                    InputEvent::Keyboard(KeyEvent::Home) => {
                        self.go_to_start()?;
                    }
                    InputEvent::Keyboard(KeyEvent::End) => {
                        self.go_to_end()?;
                    }
                    InputEvent::Keyboard(KeyEvent::BackTab) => {
                        self.show_suggestions()?;
                    }
                    _ => {}
                }
            }
        }
    }
}
