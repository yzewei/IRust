mod art;
mod cargo_cmds;
mod events;
mod format;
mod help;
pub mod highlight;
mod history;
mod irust_error;
mod known_paths;
pub mod options;
mod parser;
pub mod printer;
mod racer;
mod repl;
use crossterm::event::*;
use crossterm::style::Color;
use history::History;
pub use irust_error::IRustError;
use options::Options;
use racer::Racer;
use repl::Repl;
pub mod buffer;
use buffer::Buffer;
use highlight::theme::Theme;
use known_paths::KnownPaths;

const IN: &str = "In: ";
const OUT: &str = "Out: ";
pub const CTRL_KEYMODIFIER: crossterm::event::KeyModifiers =
    crossterm::event::KeyModifiers::CONTROL;
const ALT_KEYMODIFIER: crossterm::event::KeyModifiers = crossterm::event::KeyModifiers::ALT;
const SHIFT_KEYMODIFIER: crossterm::event::KeyModifiers = crossterm::event::KeyModifiers::SHIFT;
pub const NO_MODIFIER: crossterm::event::KeyModifiers = crossterm::event::KeyModifiers::empty();

pub struct IRust {
    buffer: Buffer,
    repl: Repl,
    printer: printer::Printer<std::io::Stdout>,
    pub options: Options,
    racer: Result<Racer, IRustError>,
    known_paths: KnownPaths,
    theme: Theme,
    history: History,
}

impl Default for IRust {
    fn default() -> Self {
        let printer = printer::Printer::default();
        let known_paths = KnownPaths::new();

        let repl = Repl::new();
        let options = Options::new().unwrap_or_default();
        let racer = if options.enable_racer {
            Racer::start()
        } else {
            Err(IRustError::RacerDisabled)
        };
        let buffer = Buffer::new();
        let theme = highlight::theme::theme().unwrap_or_default();
        let history = History::new().unwrap_or_default();

        IRust {
            repl,
            printer,
            options,
            racer,
            buffer,
            known_paths,
            theme,
            history,
        }
    }
}

impl IRust {
    fn prepare(&mut self) -> Result<(), IRustError> {
        // title is optional
        self.printer
            .writer
            .raw
            .set_title(&format!("IRust: {}", self.known_paths.get_cwd().display()))?;
        self.repl.prepare_ground(self.options.toolchain)?;
        self.welcome()?;
        self.printer.write_from_terminal_start(IN, Color::Yellow)?;

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), IRustError> {
        self.prepare()?;

        let (tx, rx) = channel();
        watch(tx.clone())?;
        let input_thread = input_read(tx)?;

        use std::io::Write;
        loop {
            // flush queued output after each key
            // some events that have an inner input loop like ctrl-r/ ctrl-d require flushing inside their respective handler function
            self.printer.writer.raw.flush()?;

            match rx.recv() {
                Ok(IRustEvent::Input(ev)) => {
                    let exit = self.handle_input_event(ev)?;
                    if exit {
                        break;
                    }
                    input_thread.thread().unpark();
                }
                Ok(IRustEvent::Notify(_ev)) => {
                    self.sync()?;
                }
                Ok(IRustEvent::Exit(e)) => return Err(e),
                // tx paniced ?
                Err(e) => {
                    return Err(
                        format!("Error while trying to receive events, error: {}", e).into(),
                    )
                }
            }
        }
        Ok(())
    }

    fn handle_input_event(&mut self, ev: crossterm::event::Event) -> Result<bool, IRustError> {
        // handle input event
        match ev {
            Event::Mouse(_) => (),
            Event::Resize(width, height) => {
                self.printer.update_dimensions(width, height);
                //Hack
                self.handle_ctrl_c()?;
            }
            Event::Key(key_event) => match key_event {
                KeyEvent {
                    code: KeyCode::Char(c),
                    modifiers: NO_MODIFIER,
                }
                | KeyEvent {
                    code: KeyCode::Char(c),
                    modifiers: SHIFT_KEYMODIFIER,
                } => {
                    self.handle_character(c)?;
                }
                KeyEvent {
                    code: KeyCode::Char('e'),
                    modifiers: CTRL_KEYMODIFIER,
                } => {
                    self.handle_ctrl_e()?;
                }
                KeyEvent {
                    code: KeyCode::Enter,
                    ..
                } => {
                    self.handle_enter(false)?;
                }
                KeyEvent {
                    code: KeyCode::Tab, ..
                } => {
                    self.handle_tab()?;
                }
                KeyEvent {
                    code: KeyCode::BackTab,
                    ..
                } => {
                    self.handle_back_tab()?;
                }
                KeyEvent {
                    code: KeyCode::Left,
                    modifiers: NO_MODIFIER,
                } => {
                    self.handle_left()?;
                }
                KeyEvent {
                    code: KeyCode::Right,
                    modifiers: NO_MODIFIER,
                } => {
                    self.handle_right()?;
                }
                KeyEvent {
                    code: KeyCode::Up, ..
                } => {
                    self.handle_up()?;
                }
                KeyEvent {
                    code: KeyCode::Down,
                    ..
                } => {
                    self.handle_down()?;
                }
                KeyEvent {
                    code: KeyCode::Backspace,
                    ..
                } => {
                    self.handle_backspace()?;
                }
                KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: CTRL_KEYMODIFIER,
                } => {
                    self.handle_ctrl_c()?;
                }
                KeyEvent {
                    code: KeyCode::Char('d'),
                    modifiers: CTRL_KEYMODIFIER,
                } => {
                    return self.handle_ctrl_d();
                }
                KeyEvent {
                    code: KeyCode::Char('z'),
                    modifiers: CTRL_KEYMODIFIER,
                } => {
                    self.handle_ctrl_z()?;
                }
                KeyEvent {
                    code: KeyCode::Char('l'),
                    modifiers: CTRL_KEYMODIFIER,
                } => {
                    self.handle_ctrl_l()?;
                }
                KeyEvent {
                    code: KeyCode::Char('r'),
                    modifiers: CTRL_KEYMODIFIER,
                } => {
                    self.handle_ctrl_r()?;
                }
                KeyEvent {
                    code: KeyCode::Char('\r'),
                    modifiers: ALT_KEYMODIFIER,
                } => {
                    self.handle_alt_enter()?;
                }
                KeyEvent {
                    code: KeyCode::Home,
                    ..
                } => {
                    self.handle_home_key()?;
                }
                KeyEvent {
                    code: KeyCode::End, ..
                } => {
                    self.handle_end_key()?;
                }
                KeyEvent {
                    code: KeyCode::Left,
                    modifiers: CTRL_KEYMODIFIER,
                } => {
                    self.handle_ctrl_left();
                }
                KeyEvent {
                    code: KeyCode::Right,
                    modifiers: CTRL_KEYMODIFIER,
                } => {
                    self.handle_ctrl_right()?;
                }
                KeyEvent {
                    code: KeyCode::Delete,
                    ..
                } => {
                    self.handle_del()?;
                }
                keyevent => {
                    // Handle AltGr on windows
                    if keyevent
                        .modifiers
                        .contains(CTRL_KEYMODIFIER | ALT_KEYMODIFIER)
                    {
                        if let KeyCode::Char(c) = keyevent.code {
                            self.handle_character(c)?;
                        }
                    }
                }
            },
        }
        Ok(false)
    }
}

impl Drop for IRust {
    fn drop(&mut self) {
        // ignore errors on drop with let _
        let _ = self.exit();
        if std::thread::panicking() {
            let _ = self.printer.writer.raw.write("IRust panicked, to log the error you can redirect stderror to a file, example irust 2>log");
        }
    }
}

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::time::Duration;

fn watch(tx: Sender<IRustEvent>) -> Result<(), std::io::Error> {
    std::thread::Builder::new()
        .name("Watcher".into())
        .spawn(move || loop {
            let _g = Guard(tx.clone());
            let (local_tx, local_rx) = channel();
            let mut watcher: RecommendedWatcher =
                Watcher::new(local_tx, Duration::from_secs(2)).expect("Watcher Thread paniced");

            watcher
                .watch(&*cargo_cmds::MAIN_FILE_EXTERN, RecursiveMode::Recursive)
                .expect("Error while trying to watch main_extern file for changes");

            if let Ok(ev) = local_rx.recv() {
                tx.send(IRustEvent::Notify(ev))
                    .expect("Error sending notify event to IRust main thread");
            }
        })?;
    Ok(())
}

fn input_read(tx: Sender<IRustEvent>) -> Result<std::thread::JoinHandle<()>, std::io::Error> {
    std::thread::Builder::new()
        .name("Input".into())
        .spawn(move || {
            let _g = Guard(tx.clone());
            let try_read = || -> Result<(), IRustError> {
                let ev = read()?;
                tx.send(IRustEvent::Input(ev))
                    .map_err(|e| format!("Could not send input event, error: {}", e))?;
                std::thread::park();
                Ok(())
            };

            loop {
                if let Err(e) = try_read() {
                    tx.send(IRustEvent::Exit(e))
                        .expect("Could not send input event");
                }
            }
        })
}

struct Guard(Sender<IRustEvent>);
impl Drop for Guard {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let t = std::thread::current();
            let name = t.name().unwrap_or("???");
            let msg = format!("\n\rThread {} paniced, to log the error you can redirect stderror to a file, exp: irust 2>log", name);

            // try clean exit
            match self.0.send(IRustEvent::Exit(msg.clone().into())) {
                Ok(_) => (),
                Err(_) => {
                    // last resort
                    // ignore errors on drop with let _
                    let _ = crossterm::terminal::disable_raw_mode();
                    eprintln!("{}", msg);
                    std::process::exit(1);
                }
            }
        }
    }
}

pub enum IRustEvent {
    Input(crossterm::event::Event),
    Notify(notify::DebouncedEvent),
    Exit(IRustError),
}
