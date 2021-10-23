use std::{
    io::{Read, Write},
    process::{ChildStdin, Command, Stdio},
    sync::mpsc,
};

use rscript::{scripting::Scripter, Hook, ScriptType, VersionReq};

struct IPython {
    stdin: ChildStdin,
    stdout: mpsc::Receiver<String>,
}

impl Scripter for IPython {
    fn script_type() -> ScriptType {
        ScriptType::Daemon
    }

    fn name() -> &'static str {
        "IPython"
    }

    fn hooks() -> &'static [&'static str] {
        &[
            irust_api::SetTitle::NAME,
            irust_api::SetWelcomeMsg::NAME,
            irust_api::OutputEvent::NAME,
            irust_api::Startup::NAME,
            irust_api::Shutdown::NAME,
        ]
    }
    fn version_requirement() -> VersionReq {
        VersionReq::parse(">=1.30.6").expect("correct version requirement")
    }
}

fn main() {
    let mut ipython = IPython::new();
    IPython::execute(&mut |hook_name| IPython::run(&mut ipython, hook_name));
}

impl IPython {
    fn run(&mut self, hook_name: &str) {
        match hook_name {
            irust_api::OutputEvent::NAME => {
                let hook: irust_api::OutputEvent = Self::read();
                let output = self.handle_output_event(hook);
                Self::write::<irust_api::OutputEvent>(&output);
            }
            irust_api::SetTitle::NAME => {
                let _hook: irust_api::SetTitle = Self::read();
                Self::write::<irust_api::SetTitle>(&Some("IPython".to_string()));
            }
            irust_api::SetWelcomeMsg::NAME => {
                let _hook: irust_api::SetWelcomeMsg = Self::read();
                Self::write::<irust_api::SetWelcomeMsg>(&Some("IPython".to_string()));
            }
            irust_api::Startup::NAME => {
                let _hook: irust_api::Startup = Self::read();
                self.clean_up();
                *self = Self::new();
                Self::write::<irust_api::Shutdown>(&None);
            }
            irust_api::Shutdown::NAME => {
                let _hook: irust_api::Shutdown = Self::read();
                self.clean_up();
                Self::write::<irust_api::Shutdown>(&None);
            }
            _ => unreachable!(),
        }
    }

    pub(crate) fn handle_output_event(&mut self, hook: irust_api::OutputEvent) -> Option<String> {
        if hook.1.starts_with(':') {
            return None;
        }

        let input = hook.1 + "\n";
        self.stdin.write_all(input.as_bytes()).unwrap();
        self.stdin.flush().unwrap();
        let now = std::time::Instant::now();
        while now.elapsed() < std::time::Duration::from_millis(200) {
            if let Ok(out) = self.stdout.try_recv() {
                // Expression
                return Some(out);
            }
        }
        // Statement
        Some("()".to_string())
    }

    pub(crate) fn clean_up(&mut self) {
        // IPython could have already exited
        // So we ignore errors
        let _ = self.stdin.write_all(b"exit\n");
        let _ = self.stdin.flush();
    }

    fn new() -> IPython {
        let mut p = Command::new("ipython")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let stdin = p.stdin.take().unwrap();
        let mut stdout = p.stdout.take().unwrap();

        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            let mut buf = [0; 512];
            let _ = stdout.read(&mut buf).unwrap();
            let _ = stdout.read(&mut buf).unwrap();
            tx.send(String::new()).unwrap();

            loop {
                let n = stdout.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                let out = String::from_utf8(buf[..n].to_vec()).unwrap();
                // Ignore In prompt
                if out.starts_with("\nIn ") {
                    continue;
                }
                // Clean Error output
                let out = {
                    let mut out = out.lines().collect::<Vec<_>>();
                    if matches!(out.last().map(|l| l.starts_with("In ")), Some(true)) {
                        out.pop();
                        out.pop();
                    }
                    out.join("\n")
                };
                tx.send(out).unwrap();
            }
        });
        // Wait for IPython to start
        rx.recv().unwrap();

        IPython { stdin, stdout: rx }
    }
}
