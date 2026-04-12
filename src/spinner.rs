use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// A simple terminal spinner that runs in a background thread
pub struct Spinner {
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Spinner {
    /// Start a spinner with the given message
    pub fn new(message: &str) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let msg = message.to_string();
        let flag = running.clone();

        let handle = thread::spawn(move || {
            let mut i = 0;
            while flag.load(Ordering::Relaxed) {
                eprint!("\r  {} {}", FRAMES[i], msg);
                i = (i + 1) % FRAMES.len();
                thread::sleep(Duration::from_millis(80));
            }
            eprint!("\r\x1b[K\r"); // clear the line
        });

        Spinner {
            running,
            handle: Some(handle),
        }
    }

    /// Stop the spinner and clear the line
    pub fn stop(mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            h.join().ok();
        }
    }
}
