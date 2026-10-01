//! The wait, made visible.
//!
//! Commit-message generation against a reasoning model can take tens of
//! seconds. Silence for that long reads as a hang, so we show elapsed time and,
//! past 15 seconds, say what is probably going on.

use std::io::{self, IsTerminal, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use super::DIM;

pub struct Spinner {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Spinner {
    /// `None` when stderr is not a terminal, so scripts and hooks stay quiet.
    pub fn start(label: &'static str) -> Option<Self> {
        if !io::stderr().is_terminal() {
            return None;
        }

        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            const FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let started = Instant::now();
            let mut frame = 0;
            let mut hinted = false;
            let mut err = io::stderr();

            while !flag.load(Ordering::Relaxed) {
                let seconds = started.elapsed().as_secs_f32();
                let _ = write!(
                    err,
                    "\r\x1b[2K{DIM}{} {label} {seconds:.1}s{reset}   ",
                    FRAMES[frame % FRAMES.len()],
                    reset = super::RESET
                );
                let _ = err.flush();
                frame += 1;

                if seconds > 15.0 && !hinted {
                    hinted = true;
                    let _ = write!(
                        err,
                        "\n{DIM}│  still waiting — reasoning models are slow at this. \
                         Use `--model` with a faster one, or set \
                         params.reasoning_effort = \"low\" in your provider config{reset}\n",
                        reset = super::RESET
                    );
                    let _ = err.flush();
                }
                std::thread::sleep(Duration::from_millis(90));
            }

            let _ = write!(err, "\r\x1b[2K");
            let _ = err.flush();
        });

        Some(Self {
            stop,
            handle: Some(handle),
        })
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
