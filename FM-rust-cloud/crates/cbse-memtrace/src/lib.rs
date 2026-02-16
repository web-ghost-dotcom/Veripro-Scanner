// SPDX-License-Identifier: AGPL-3.0

use colored::*;
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex, Once};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{Pid, System};

/// Format bytes into human-readable size
pub fn readable_size(num: u64) -> String {
    if num < 1024 {
        format!("{}B", num)
    } else if num < 1024 * 1024 {
        format!("{:.1}KiB", num as f64 / 1024.0)
    } else {
        format!("{:.1}MiB", num as f64 / (1024.0 * 1024.0))
    }
}

/// Format size with magenta color
pub fn pretty_size(num: u64) -> String {
    format!("[magenta]{}[/magenta]", readable_size(num))
}

/// Format count difference with color
pub fn pretty_count_diff(num: i64) -> String {
    if num > 0 {
        format!("[red]+{}[/red]", num)
    } else if num < 0 {
        format!("[green]{}[/green]", num)
    } else {
        "[gray]0[/gray]".to_string()
    }
}

/// Pretty format a source line
pub fn pretty_line(line: &str) -> String {
    if line.is_empty() {
        String::new()
    } else {
        format!("[white]    {}[/white]", line)
    }
}

/// Frame information for memory tracing
#[derive(Debug, Clone)]
pub struct Frame {
    pub filename: String,
    pub lineno: usize,
}

impl Frame {
    pub fn new(filename: String, lineno: usize) -> Self {
        Self { filename, lineno }
    }
}

/// Pretty format frame information
pub fn pretty_frame_info(frame: &Frame, result_number: Option<usize>) -> String {
    let result_str = if let Some(n) = result_number {
        format!("[grey37]# {}:[/grey37] ", n + 1)
    } else {
        String::new()
    };

    format!(
        "{}[grey37]{}:[/grey37][grey37]{}:[/grey37]",
        result_str, frame.filename, frame.lineno
    )
}

/// Statistic for memory usage
#[derive(Debug, Clone)]
pub struct Statistic {
    pub traceback: Vec<Frame>,
    pub size: u64,
    pub count: usize,
}

impl Statistic {
    pub fn new(traceback: Vec<Frame>, size: u64, count: usize) -> Self {
        Self {
            traceback,
            size,
            count,
        }
    }
}

/// Statistic difference between snapshots
#[derive(Debug, Clone)]
pub struct StatisticDiff {
    pub traceback: Vec<Frame>,
    pub size_diff: i64,
    pub count_diff: i64,
}

impl StatisticDiff {
    pub fn new(traceback: Vec<Frame>, size_diff: i64, count_diff: i64) -> Self {
        Self {
            traceback,
            size_diff,
            count_diff,
        }
    }
}

/// Memory snapshot
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub timestamp: Instant,
    pub memory: u64,
    pub statistics: Vec<Statistic>,
}

impl Snapshot {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let pid = Pid::from_u32(std::process::id());
        let memory = sys.process(pid).map(|p| p.memory()).unwrap_or(0);

        // Create dummy statistics - in production this would analyze actual memory allocations
        let statistics = vec![Statistic::new(
            vec![Frame::new("unknown".to_string(), 0)],
            memory,
            1,
        )];

        Self {
            timestamp: Instant::now(),
            memory,
            statistics,
        }
    }

    /// Compare this snapshot to another and return differences
    pub fn compare_to(&self, other: &Snapshot, key: &str, cumulative: bool) -> Vec<StatisticDiff> {
        // Simplified comparison - in production would do detailed line-by-line comparison
        let size_diff = self.memory as i64 - other.memory as i64;
        let count_diff = self.statistics.len() as i64 - other.statistics.len() as i64;

        vec![StatisticDiff::new(
            vec![Frame::new("comparison".to_string(), 1)],
            size_diff,
            count_diff,
        )]
    }

    /// Get statistics grouped by key
    pub fn statistics(&self, key: &str) -> Vec<Statistic> {
        self.statistics.clone()
    }
}

impl Default for Snapshot {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory tracer singleton
pub struct MemTracer {
    pub curr_snapshot: Option<Snapshot>,
    pub prev_snapshot: Option<Snapshot>,
    pub running: bool,
}

impl MemTracer {
    fn new() -> Self {
        Self {
            curr_snapshot: None,
            prev_snapshot: None,
            running: false,
        }
    }

    /// Get the singleton instance
    pub fn get() -> &'static Mutex<MemTracer> {
        static mut INSTANCE: Option<Mutex<MemTracer>> = None;
        static ONCE: Once = Once::new();

        unsafe {
            ONCE.call_once(|| {
                INSTANCE = Some(Mutex::new(MemTracer::new()));
            });
            INSTANCE.as_ref().unwrap()
        }
    }

    /// Take a memory snapshot
    pub fn take_snapshot(&mut self) {
        println!("memtracer: taking snapshot");
        self.prev_snapshot = self.curr_snapshot.clone();
        self.curr_snapshot = Some(Snapshot::new());
        self.display_stats();
    }

    /// Display statistics about the current memory snapshot
    pub fn display_stats(&self) {
        if !self.running {
            return;
        }

        if self.curr_snapshot.is_none() {
            println!("memtracer: no current snapshot");
            return;
        }

        let snapshot = self.curr_snapshot.as_ref().unwrap();
        let mut output = String::new();

        // Show top memory consumers by line
        output.push_str("[cyan][ Top memory consumers ][/cyan]\n");
        let stats = snapshot.statistics("lineno");
        for (i, stat) in stats.iter().take(10).enumerate() {
            if let Some(frame) = stat.traceback.first() {
                let line = self.get_line(&frame.filename, frame.lineno);
                output.push_str(&format!(
                    "{} {}\n",
                    pretty_frame_info(frame, Some(i)),
                    pretty_size(stat.size)
                ));
                output.push_str(&format!("{}\n", pretty_line(&line)));
            }
        }
        output.push_str("\n");

        // Get total memory usage
        let total: u64 = snapshot.statistics.iter().map(|s| s.size).sum();
        output.push_str(&format!(
            "Total memory used in snapshot: {}\n\n",
            pretty_size(total)
        ));

        println!("{}", output);
    }

    /// Start tracking memory usage at the specified interval
    pub fn start(&mut self, interval_seconds: u64) {
        self.running = true;
        self.take_snapshot();

        // Spawn background thread
        let tracer = Arc::new(Mutex::new(self.running));
        let tracer_clone = tracer.clone();

        thread::spawn(move || {
            let tracer_mutex = MemTracer::get();
            loop {
                thread::sleep(Duration::from_secs(interval_seconds));

                let should_run = {
                    let tracer = tracer_mutex.lock().unwrap();
                    tracer.running
                };

                if !should_run {
                    break;
                }

                let mut tracer = tracer_mutex.lock().unwrap();
                tracer.take_snapshot();
                tracer.display_differences();
            }
        });
    }

    /// Stop the memory tracer
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Display top memory differences between snapshots
    fn display_differences(&self) {
        if !self.running {
            return;
        }

        if self.prev_snapshot.is_none() || self.curr_snapshot.is_none() {
            println!("memtracer: no snapshots to compare");
            return;
        }

        let prev = self.prev_snapshot.as_ref().unwrap();
        let curr = self.curr_snapshot.as_ref().unwrap();

        let mut output = String::new();

        let top_stats = curr.compare_to(prev, "lineno", true);
        output.push_str("[cyan][ Top differences ][/cyan]\n");
        for (i, stat) in top_stats.iter().take(10).enumerate() {
            if let Some(frame) = stat.traceback.first() {
                let line = self.get_line(&frame.filename, frame.lineno);
                output.push_str(&format!(
                    "{} {} [{}]\n",
                    pretty_frame_info(frame, Some(i)),
                    pretty_size(stat.size_diff.unsigned_abs()),
                    pretty_count_diff(stat.count_diff)
                ));
                output.push_str(&format!("{}\n", pretty_line(&line)));
            }
        }

        let total_diff: i64 = top_stats.iter().map(|s| s.size_diff).sum();
        output.push_str(&format!(
            "Total size difference: {}\n",
            pretty_size(total_diff.unsigned_abs())
        ));

        println!("{}", output);
    }

    /// Get a line from a file
    fn get_line(&self, filename: &str, lineno: usize) -> String {
        if let Ok(contents) = fs::read_to_string(filename) {
            contents
                .lines()
                .nth(lineno.saturating_sub(1))
                .unwrap_or("")
                .trim()
                .to_string()
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_readable_size() {
        assert_eq!(readable_size(512), "512B");
        assert_eq!(readable_size(2048), "2.0KiB");
        assert_eq!(readable_size(2097152), "2.0MiB");
    }

    #[test]
    fn test_pretty_size() {
        let result = pretty_size(1024);
        assert!(result.contains("KiB"));
        assert!(result.contains("magenta"));
    }

    #[test]
    fn test_pretty_count_diff() {
        let positive = pretty_count_diff(10);
        assert!(positive.contains("+10"));
        assert!(positive.contains("red"));

        let negative = pretty_count_diff(-5);
        assert!(negative.contains("-5"));
        assert!(negative.contains("green"));

        let zero = pretty_count_diff(0);
        assert!(zero.contains("0"));
        assert!(zero.contains("gray"));
    }

    #[test]
    fn test_pretty_line() {
        assert_eq!(pretty_line(""), "");
        let line = pretty_line("test line");
        assert!(line.contains("test line"));
        assert!(line.contains("white"));
    }

    #[test]
    fn test_frame() {
        let frame = Frame::new("test.rs".to_string(), 42);
        assert_eq!(frame.filename, "test.rs");
        assert_eq!(frame.lineno, 42);
    }

    #[test]
    fn test_pretty_frame_info() {
        let frame = Frame::new("test.rs".to_string(), 10);
        let with_number = pretty_frame_info(&frame, Some(0));
        assert!(with_number.contains("# 1:"));
        assert!(with_number.contains("test.rs"));
        assert!(with_number.contains("10"));

        let without_number = pretty_frame_info(&frame, None);
        assert!(!without_number.contains("# 1:"));
        assert!(without_number.contains("test.rs"));
    }

    #[test]
    fn test_statistic() {
        let frames = vec![Frame::new("test.rs".to_string(), 1)];
        let stat = Statistic::new(frames, 1024, 1);
        assert_eq!(stat.size, 1024);
        assert_eq!(stat.count, 1);
        assert_eq!(stat.traceback.len(), 1);
    }

    #[test]
    fn test_statistic_diff() {
        let frames = vec![Frame::new("test.rs".to_string(), 1)];
        let diff = StatisticDiff::new(frames, 512, 2);
        assert_eq!(diff.size_diff, 512);
        assert_eq!(diff.count_diff, 2);
    }

    #[test]
    fn test_snapshot() {
        let snapshot = Snapshot::new();
        assert!(snapshot.memory > 0);
        assert!(!snapshot.statistics.is_empty());
    }

    #[test]
    fn test_snapshot_compare() {
        let snap1 = Snapshot::new();
        thread::sleep(Duration::from_millis(10));
        let snap2 = Snapshot::new();

        let diffs = snap2.compare_to(&snap1, "lineno", true);
        assert!(!diffs.is_empty());
    }

    #[test]
    fn test_snapshot_statistics() {
        let snapshot = Snapshot::new();
        let stats = snapshot.statistics("lineno");
        assert!(!stats.is_empty());
    }

    #[test]
    fn test_memtracer_singleton() {
        let tracer1 = MemTracer::get();
        let tracer2 = MemTracer::get();
        assert!(std::ptr::eq(tracer1, tracer2));
    }

    #[test]
    fn test_memtracer_snapshot() {
        let tracer = MemTracer::get();
        let mut tracer = tracer.lock().unwrap();
        tracer.running = true;
        tracer.take_snapshot();
        assert!(tracer.curr_snapshot.is_some());
    }

    #[test]
    fn test_memtracer_stop() {
        let tracer = MemTracer::get();
        let mut tracer = tracer.lock().unwrap();
        tracer.running = true;
        tracer.stop();
        assert!(!tracer.running);
    }
}
