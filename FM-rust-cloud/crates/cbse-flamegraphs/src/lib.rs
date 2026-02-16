// SPDX-License-Identifier: AGPL-3.0

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Auto refresh interval for flamegraphs in seconds
pub const AUTO_REFRESH_INTERVAL_SECONDS: f64 = 0.250;

/// Timed thread for tracking execution time and exceptions
pub struct TimedThread {
    handle: Option<thread::JoinHandle<Result<(), String>>>,
    pub start_time: Option<Instant>,
    pub end_time: Arc<Mutex<Option<Instant>>>,
    pub exception: Arc<Mutex<Option<String>>>,
}

impl TimedThread {
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce() -> Result<(), String> + Send + 'static,
    {
        let start_time = Some(Instant::now());
        let end_time = Arc::new(Mutex::new(None));
        let exception = Arc::new(Mutex::new(None));

        let end_time_clone = end_time.clone();
        let exception_clone = exception.clone();

        let handle = thread::spawn(move || {
            let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
                Ok(r) => r,
                Err(e) => Err(format!("Thread panicked: {:?}", e)),
            };

            if let Ok(mut end) = end_time_clone.lock() {
                *end = Some(Instant::now());
            }

            if let Err(e) = &result {
                if let Ok(mut exc) = exception_clone.lock() {
                    *exc = Some(e.clone());
                }
            }

            result
        });

        Self {
            handle: Some(handle),
            start_time,
            end_time,
            exception,
        }
    }

    pub fn join(&mut self) -> Result<(), String> {
        if let Some(handle) = self.handle.take() {
            handle.join().map_err(|e| format!("{:?}", e))??;
        }
        Ok(())
    }

    pub fn is_alive(&self) -> bool {
        if let Some(handle) = &self.handle {
            !handle.is_finished()
        } else {
            false
        }
    }

    pub fn get_end_time(&self) -> Option<Instant> {
        self.end_time.lock().ok()?.clone()
    }

    pub fn get_exception(&self) -> Option<String> {
        self.exception.lock().ok()?.clone()
    }

    pub fn elapsed(&self) -> Option<Duration> {
        if let (Some(start), Some(end)) = (self.start_time, self.get_end_time()) {
            Some(end.duration_since(start))
        } else {
            None
        }
    }
}

impl std::fmt::Debug for TimedThread {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimedThread")
            .field("handle", &self.handle.is_some())
            .field("start_time", &self.start_time)
            .field("end_time", &self.end_time)
            .field("exception", &self.exception)
            .finish()
    }
}

/// Run command with temporary output
pub fn run_with_tmp_output(command: Vec<String>, out_filepath: &Path) -> Result<(), String> {
    // First write to a temporary file
    let tmp_filepath = out_filepath.with_extension("tmp");

    let output = Command::new(&command[0])
        .args(&command[1..])
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    if !output.status.success() {
        return Err(format!("Command failed with status: {}", output.status));
    }

    let mut tmp_file =
        File::create(&tmp_filepath).map_err(|e| format!("Failed to create temp file: {}", e))?;

    tmp_file
        .write_all(&output.stdout)
        .map_err(|e| format!("Failed to write to temp file: {}", e))?;

    // Rename temporary file to output file if command succeeded
    fs::rename(&tmp_filepath, out_filepath)
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;

    Ok(())
}

/// Flamegraph accumulator for collecting and generating flamegraphs
#[derive(Debug)]
pub struct FlamegraphAccumulator {
    pub title: String,
    pub src_filepath: Option<PathBuf>,
    pub out_filepath: Option<PathBuf>,
    pub colors: String,
    pub stacks: Arc<Mutex<Vec<String>>>,
    pub auto_flush: bool,
    pub debug: bool,
    pub bg_threads: Arc<Mutex<Vec<TimedThread>>>,
}

impl FlamegraphAccumulator {
    pub fn new(
        title: String,
        src_filepath: Option<PathBuf>,
        out_filepath: Option<PathBuf>,
    ) -> Self {
        // Clean up existing files
        if let Some(ref path) = src_filepath {
            if path.exists() {
                let _ = fs::remove_file(path);
            }
        }
        if let Some(ref path) = out_filepath {
            if path.exists() {
                let _ = fs::remove_file(path);
            }
        }

        Self {
            title,
            src_filepath,
            out_filepath,
            colors: "hot".to_string(),
            stacks: Arc::new(Mutex::new(Vec::new())),
            auto_flush: false,
            debug: false,
            bg_threads: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_colors(mut self, colors: String) -> Self {
        self.colors = colors;
        self
    }

    pub fn with_auto_flush(mut self, auto_flush: bool) -> Self {
        self.auto_flush = auto_flush;
        self
    }

    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    pub fn len(&self) -> usize {
        self.stacks.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Add a stack trace
    pub fn add_stack(&self, stack: String) {
        let mut stacks = self.stacks.lock().unwrap();
        stacks.push(stack);

        if self.auto_flush {
            drop(stacks); // Release lock before flushing
            self.flush(false);
        }
    }

    /// Add multiple stack traces
    pub fn add_stacks(&self, new_stacks: Vec<String>) {
        let mut stacks = self.stacks.lock().unwrap();
        stacks.extend(new_stacks);

        if self.auto_flush {
            drop(stacks); // Release lock before flushing
            self.flush(false);
        }
    }

    /// Flush stacks to file and generate flamegraph
    pub fn flush(&self, force: bool) {
        let mut stacks = self.stacks.lock().unwrap();

        if stacks.is_empty() {
            if self.debug {
                eprintln!("No stacks collected for {}, skipping", self.title);
            }
            return;
        }

        let Some(ref src_filepath) = self.src_filepath else {
            eprintln!("Missing src_filepath for {}", self.title);
            return;
        };

        let Some(ref out_filepath) = self.out_filepath else {
            eprintln!("Missing out_filepath for {}", self.title);
            return;
        };

        // Write stacks to file
        let mut src_file = match File::options().create(true).append(true).open(src_filepath) {
            Ok(f) => f,
            Err(e) => {
                eprintln!(
                    "Failed to open {} for writing: {}",
                    src_filepath.display(),
                    e
                );
                return;
            }
        };

        for stack in stacks.iter() {
            if let Err(e) = writeln!(src_file, "{} 1", stack) {
                eprintln!("Failed to write to {}: {}", src_filepath.display(), e);
                return;
            }
        }

        stacks.clear();
        drop(stacks); // Release lock
        drop(src_file); // Ensure file is closed

        // Check background threads
        let mut threads = self.bg_threads.lock().unwrap();

        if let Some(last_thread) = threads.last_mut() {
            // Don't start new thread if last one is still running
            if last_thread.is_alive() || last_thread.get_end_time().is_none() {
                if force {
                    let _ = last_thread.join();
                } else {
                    return;
                }
            }

            // Check if last thread failed
            if let Some(exception) = last_thread.get_exception() {
                eprintln!("Failed to generate flamegraph: {}", exception);
            }

            // Don't start new thread if last one finished recently
            if let Some(end_time) = last_thread.get_end_time() {
                let elapsed = end_time.elapsed().as_secs_f64();
                if elapsed < AUTO_REFRESH_INTERVAL_SECONDS && !force {
                    return;
                }
            }

            threads.pop();
        }

        // Start flamegraph generation in background thread
        let command = vec![
            "flamegraph.pl".to_string(),
            "--title".to_string(),
            self.title.clone(),
            "--colors".to_string(),
            self.colors.clone(),
            "--cp".to_string(),
            src_filepath.display().to_string(),
        ];

        let out_path = out_filepath.clone();
        let thread = TimedThread::new(move || run_with_tmp_output(command, &out_path));

        threads.push(thread);

        if force {
            if let Some(last_thread) = threads.last_mut() {
                let _ = last_thread.join();
            }
        }
    }
}

/// Call sequence flamegraph (extends FlamegraphAccumulator)
#[derive(Debug)]
pub struct CallSequenceFlamegraph {
    accumulator: FlamegraphAccumulator,
}

impl CallSequenceFlamegraph {
    pub fn new(
        title: String,
        src_filepath: Option<PathBuf>,
        out_filepath: Option<PathBuf>,
    ) -> Self {
        Self {
            accumulator: FlamegraphAccumulator::new(title, src_filepath, out_filepath)
                .with_colors("aqua".to_string())
                .with_auto_flush(true),
        }
    }

    /// Add stack trace with optional prefix and failure marking
    pub fn add_with_prefix(&self, stack: String, prefix: Option<String>, mark_as_fail: bool) {
        let identifier = if mark_as_fail {
            format!("[FAIL] {}", stack)
        } else {
            stack
        };

        let full_stack = if let Some(p) = prefix {
            format!("{};{}", p, identifier)
        } else {
            identifier
        };

        self.accumulator.add_stack(full_stack);
    }

    pub fn flush(&self, force: bool) {
        self.accumulator.flush(force);
    }

    pub fn len(&self) -> usize {
        self.accumulator.len()
    }

    pub fn is_empty(&self) -> bool {
        self.accumulator.is_empty()
    }
}

/// Global execution flamegraph (for stateless/single-function tests)
pub fn get_exec_flamegraph() -> FlamegraphAccumulator {
    FlamegraphAccumulator::new(
        "Execution Flamegraph".to_string(),
        Some(PathBuf::from("exec.stacks")),
        Some(PathBuf::from("exec-flamegraph.svg")),
    )
    .with_auto_flush(false)
}

/// Global call flamegraph (for invariant tests with auto-flush)
pub fn get_call_flamegraph() -> CallSequenceFlamegraph {
    CallSequenceFlamegraph::new(
        "Call Flamegraph".to_string(),
        Some(PathBuf::from("call.stacks")),
        Some(PathBuf::from("call-flamegraph.svg")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_timed_thread_success() {
        let mut thread = TimedThread::new(|| {
            thread::sleep(Duration::from_millis(100));
            Ok(())
        });

        assert!(thread.start_time.is_some());
        assert!(thread.join().is_ok());
        assert!(thread.elapsed().is_some());
    }

    #[test]
    fn test_timed_thread_error() {
        let mut thread = TimedThread::new(|| Err("Test error".to_string()));

        assert!(thread.join().is_err());
        assert!(thread.get_exception().is_some());
    }

    #[test]
    fn test_timed_thread_is_alive() {
        let thread = TimedThread::new(|| {
            thread::sleep(Duration::from_millis(500));
            Ok(())
        });

        assert!(thread.is_alive());
        thread::sleep(Duration::from_millis(600));
        assert!(!thread.is_alive());
    }

    #[test]
    fn test_flamegraph_accumulator_new() {
        let temp_dir = std::env::temp_dir();
        let src = temp_dir.join("test.stacks");
        let out = temp_dir.join("test.svg");

        let acc =
            FlamegraphAccumulator::new("Test".to_string(), Some(src.clone()), Some(out.clone()));

        assert_eq!(acc.title, "Test");
        assert_eq!(acc.colors, "hot");
        assert!(!acc.auto_flush);
        assert!(acc.is_empty());

        // Cleanup
        let _ = fs::remove_file(src);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn test_flamegraph_accumulator_add_stack() {
        let temp_dir = std::env::temp_dir();
        let acc = FlamegraphAccumulator::new(
            "Test".to_string(),
            Some(temp_dir.join("test2.stacks")),
            Some(temp_dir.join("test2.svg")),
        );

        acc.add_stack("A".to_string());
        acc.add_stack("A;B".to_string());
        acc.add_stack("A;B;C".to_string());

        assert_eq!(acc.len(), 3);
    }

    #[test]
    fn test_flamegraph_accumulator_add_stacks() {
        let temp_dir = std::env::temp_dir();
        let acc = FlamegraphAccumulator::new(
            "Test".to_string(),
            Some(temp_dir.join("test3.stacks")),
            Some(temp_dir.join("test3.svg")),
        );

        acc.add_stacks(vec![
            "A".to_string(),
            "A;B".to_string(),
            "A;B;C".to_string(),
        ]);

        assert_eq!(acc.len(), 3);
    }

    #[test]
    fn test_flamegraph_accumulator_with_colors() {
        let temp_dir = std::env::temp_dir();
        let acc = FlamegraphAccumulator::new(
            "Test".to_string(),
            Some(temp_dir.join("test4.stacks")),
            Some(temp_dir.join("test4.svg")),
        )
        .with_colors("aqua".to_string());

        assert_eq!(acc.colors, "aqua");
    }

    #[test]
    fn test_flamegraph_accumulator_with_auto_flush() {
        let temp_dir = std::env::temp_dir();
        let acc = FlamegraphAccumulator::new(
            "Test".to_string(),
            Some(temp_dir.join("test5.stacks")),
            Some(temp_dir.join("test5.svg")),
        )
        .with_auto_flush(true);

        assert!(acc.auto_flush);
    }

    #[test]
    fn test_call_sequence_flamegraph_new() {
        let temp_dir = std::env::temp_dir();
        let fg = CallSequenceFlamegraph::new(
            "Test Call".to_string(),
            Some(temp_dir.join("test6.stacks")),
            Some(temp_dir.join("test6.svg")),
        );

        assert_eq!(fg.accumulator.title, "Test Call");
        assert_eq!(fg.accumulator.colors, "aqua");
        assert!(fg.accumulator.auto_flush);
    }

    #[test]
    fn test_call_sequence_flamegraph_add_with_prefix() {
        let temp_dir = std::env::temp_dir();
        let fg = CallSequenceFlamegraph::new(
            "Test Call".to_string(),
            Some(temp_dir.join("test7.stacks")),
            Some(temp_dir.join("test7.svg")),
        )
        .accumulator
        .with_auto_flush(false);

        let fg = CallSequenceFlamegraph { accumulator: fg };

        fg.add_with_prefix("func1".to_string(), Some("prefix".to_string()), false);
        assert_eq!(fg.len(), 1);

        fg.add_with_prefix("func2".to_string(), None, true);
        assert_eq!(fg.len(), 2);
    }

    #[test]
    fn test_get_exec_flamegraph() {
        let fg = get_exec_flamegraph();
        assert_eq!(fg.title, "Execution Flamegraph");
        assert_eq!(fg.colors, "hot");
        assert!(!fg.auto_flush);
    }

    #[test]
    fn test_get_call_flamegraph() {
        let fg = get_call_flamegraph();
        assert_eq!(fg.accumulator.title, "Call Flamegraph");
        assert_eq!(fg.accumulator.colors, "aqua");
        assert!(fg.accumulator.auto_flush);
    }
}
