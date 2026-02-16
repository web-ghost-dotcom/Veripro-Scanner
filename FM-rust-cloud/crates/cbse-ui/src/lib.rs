// SPDX-License-Identifier: AGPL-3.0

//! UI module for CBSE - status management and user interaction

use console::{style, Term};
use dialoguer::Confirm;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::{Arc, Mutex};

/// Status handle for suspending/resuming
pub struct StatusHandle {
    progress: Arc<Mutex<Option<ProgressBar>>>,
}

impl StatusHandle {
    /// Suspend the status (stop updates)
    pub fn suspend(&self) {
        if let Ok(guard) = self.progress.lock() {
            if let Some(ref pb) = *guard {
                pb.set_draw_target(indicatif::ProgressDrawTarget::hidden());
            }
        }
    }

    /// Resume the status (restart updates)
    pub fn resume(&self) {
        if let Ok(guard) = self.progress.lock() {
            if let Some(ref pb) = *guard {
                pb.set_draw_target(indicatif::ProgressDrawTarget::stderr());
            }
        }
    }
}

/// Context manager for suspending status
pub struct SuspendStatus {
    handle: StatusHandle,
}

impl SuspendStatus {
    pub fn new(handle: StatusHandle) -> Self {
        handle.suspend();
        Self { handle }
    }
}

impl Drop for SuspendStatus {
    fn drop(&mut self) {
        self.handle.resume();
    }
}

/// Main UI handler for CBSE
#[derive(Clone)]
pub struct UI {
    progress: Arc<Mutex<Option<ProgressBar>>>,
    term: Term,
}

impl UI {
    /// Create a new UI instance
    pub fn new() -> Self {
        Self {
            progress: Arc::new(Mutex::new(None)),
            term: Term::stderr(),
        }
    }

    /// Check if the terminal is interactive
    pub fn is_interactive(&self) -> bool {
        self.term.is_term()
    }

    /// Clear live display
    pub fn clear_live(&self) {
        if let Ok(guard) = self.progress.lock() {
            if let Some(ref pb) = *guard {
                pb.finish_and_clear();
            }
        }
    }

    /// Start status with a message
    pub fn start_status(&self, message: &str) {
        self.clear_live();

        if self.is_interactive() {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap_or_else(|_| ProgressStyle::default_spinner()),
            );
            pb.set_message(message.to_string());
            pb.enable_steady_tick(std::time::Duration::from_millis(100));

            if let Ok(mut guard) = self.progress.lock() {
                *guard = Some(pb);
            }
        }
    }

    /// Update status message
    pub fn update_status(&self, message: &str) {
        if let Ok(guard) = self.progress.lock() {
            if let Some(ref pb) = *guard {
                pb.set_message(message.to_string());
            }
        }
    }

    /// Stop status updates
    pub fn stop_status(&self) {
        if let Ok(mut guard) = self.progress.lock() {
            if let Some(pb) = guard.take() {
                pb.finish_and_clear();
            }
        }
    }

    /// Get a handle for suspending status
    pub fn status_handle(&self) -> StatusHandle {
        StatusHandle {
            progress: Arc::clone(&self.progress),
        }
    }

    /// Prompt user for confirmation (returns false if non-interactive)
    pub fn prompt(&self, prompt: &str) -> bool {
        // Non-interactive sessions will block on input, so return false
        if !self.is_interactive() {
            return false;
        }

        // Suspend status during prompt
        let _suspend = SuspendStatus::new(self.status_handle());

        Confirm::new()
            .with_prompt(prompt)
            .interact()
            .unwrap_or(false)
    }

    /// Print message to console
    pub fn print(&self, message: &str) {
        if let Ok(guard) = self.progress.lock() {
            if let Some(ref pb) = *guard {
                pb.suspend(|| {
                    println!("{}", message);
                });
            } else {
                println!("{}", message);
            }
        } else {
            println!("{}", message);
        }
    }

    /// Print styled message
    pub fn print_styled(&self, message: &str, color: &str) {
        let styled = match color {
            "green" => style(message).green(),
            "red" => style(message).red(),
            "yellow" => style(message).yellow(),
            "cyan" => style(message).cyan(),
            "magenta" => style(message).magenta(),
            _ => style(message).white(),
        };
        self.print(&format!("{}", styled));
    }

    /// Print to stderr
    pub fn eprint(&self, message: &str) {
        if let Ok(guard) = self.progress.lock() {
            if let Some(ref pb) = *guard {
                pb.suspend(|| {
                    eprintln!("{}", message);
                });
            } else {
                eprintln!("{}", message);
            }
        } else {
            eprintln!("{}", message);
        }
    }

    /// Write message (same as print for convenience)
    pub fn write(&self, text: &str) {
        self.print(text);
    }

    /// Write line (same as print for convenience)
    pub fn write_line(&self, text: &str) {
        self.print(text);
    }

    /// Flush output (no-op, kept for API compatibility)
    pub fn flush(&self) {
        // No-op since println! auto-flushes
    }
}

impl Default for UI {
    fn default() -> Self {
        Self::new()
    }
}

/// Global UI instance
static GLOBAL_UI: once_cell::sync::Lazy<UI> = once_cell::sync::Lazy::new(UI::new);

/// Get the global UI instance
pub fn ui() -> &'static UI {
    &GLOBAL_UI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_creation() {
        let _ui = UI::new();
        // Should not panic
        assert!(true);
    }

    #[test]
    fn test_status_operations() {
        let ui = UI::new();
        ui.start_status("Testing");
        ui.update_status("Updated");
        ui.stop_status();
    }

    #[test]
    fn test_print() {
        let ui = UI::new();
        ui.print("Test message");
    }

    #[test]
    fn test_print_styled() {
        let ui = UI::new();
        ui.print_styled("Success", "green");
        ui.print_styled("Error", "red");
        ui.print_styled("Warning", "yellow");
    }

    #[test]
    fn test_eprint() {
        let ui = UI::new();
        ui.eprint("Error message");
    }

    #[test]
    fn test_clear_live() {
        let ui = UI::new();
        ui.clear_live();
    }

    #[test]
    fn test_status_handle() {
        let ui = UI::new();
        let handle = ui.status_handle();
        handle.suspend();
        handle.resume();
    }

    #[test]
    fn test_suspend_status() {
        let ui = UI::new();
        ui.start_status("Testing");
        {
            let _suspend = SuspendStatus::new(ui.status_handle());
            // Status should be suspended here
        }
        // Status should be resumed here
        ui.stop_status();
    }

    #[test]
    fn test_prompt_non_interactive() {
        let ui = UI::new();
        // Should return false for non-interactive
        // (can't test interactive prompt in unit tests)
        let result = ui.prompt("Test?");
        // Result depends on whether terminal is interactive
        let _ = result;
    }

    #[test]
    fn test_global_ui() {
        let ui1 = ui();
        let ui2 = ui();
        // Should be the same instance
        ui1.print("Test");
        ui2.print("Test");
    }

    #[test]
    fn test_default() {
        let ui = UI::default();
        ui.print("Default UI");
    }

    #[test]
    fn test_clone() {
        let ui1 = UI::new();
        let ui2 = ui1.clone();
        ui1.print("UI 1");
        ui2.print("UI 2");
    }

    #[test]
    fn test_multiple_status_updates() {
        let ui = UI::new();
        ui.start_status("Step 1");
        ui.update_status("Step 2");
        ui.update_status("Step 3");
        ui.stop_status();
    }

    #[test]
    fn test_status_without_start() {
        let ui = UI::new();
        ui.update_status("Should not crash");
        ui.stop_status();
    }

    #[test]
    fn test_clear_without_status() {
        let ui = UI::new();
        ui.clear_live();
    }
}
