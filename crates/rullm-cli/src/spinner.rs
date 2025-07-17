use owo_colors::OwoColorize;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::time::interval;

pub struct Spinner {
    is_active: Arc<AtomicBool>,
    message: String,
}

impl Spinner {
    pub fn new(message: &str) -> Self {
        Self {
            is_active: Arc::new(AtomicBool::new(false)),
            message: message.to_string(),
        }
    }

    pub async fn start(&self) {
        self.is_active.store(true, Ordering::Relaxed);
        let is_active = Arc::clone(&self.is_active);
        let message = self.message.clone();

        tokio::spawn(async move {
            let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let mut interval = interval(Duration::from_millis(80));
            let mut frame_index = 0;

            // Print initial message
            print!("{} ", message.blue().bold());
            io::stdout().flush().unwrap_or(());

            while is_active.load(Ordering::Relaxed) {
                interval.tick().await;

                if is_active.load(Ordering::Relaxed) {
                    // Move cursor back and draw spinner
                    print!(
                        "\r{} {} ",
                        message.blue().bold(),
                        frames[frame_index].cyan()
                    );
                    io::stdout().flush().unwrap_or(());
                    frame_index = (frame_index + 1) % frames.len();
                }
            }
        });
    }

    pub fn stop(&self) {
        self.is_active.store(false, Ordering::Relaxed);
        // Clear the line
        print!("\r\x1b[K");
        io::stdout().flush().unwrap_or(());
    }

    pub fn stop_and_replace(&self, replacement: &str) {
        self.is_active.store(false, Ordering::Relaxed);
        // Clear the line and print replacement
        print!("\r\x1b[K{replacement}");
        io::stdout().flush().unwrap_or(());
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop();
    }
}
