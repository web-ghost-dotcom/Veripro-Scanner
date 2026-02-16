// SPDX-License-Identifier: AGPL-3.0

#[cfg(test)]
mod tests {
    use cbse_memtrace::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_readable_size_bytes() {
        assert_eq!(readable_size(0), "0B");
        assert_eq!(readable_size(512), "512B");
        assert_eq!(readable_size(1023), "1023B");
    }

    #[test]
    fn test_readable_size_kib() {
        assert_eq!(readable_size(1024), "1.0KiB");
        assert_eq!(readable_size(2048), "2.0KiB");
        assert_eq!(readable_size(1536), "1.5KiB");
        assert_eq!(readable_size(10240), "10.0KiB");
    }

    #[test]
    fn test_readable_size_mib() {
        assert_eq!(readable_size(1024 * 1024), "1.0MiB");
        assert_eq!(readable_size(2 * 1024 * 1024), "2.0MiB");
        assert_eq!(readable_size(1536 * 1024), "1.5MiB");
        assert_eq!(readable_size(100 * 1024 * 1024), "100.0MiB");
    }

    #[test]
    fn test_pretty_size_format() {
        let result = pretty_size(1024);
        assert!(result.contains("1.0KiB"));
        assert!(result.contains("magenta"));
    }

    #[test]
    fn test_pretty_count_diff_positive() {
        let result = pretty_count_diff(10);
        assert!(result.contains("+10"));
        assert!(result.contains("red"));
    }

    #[test]
    fn test_pretty_count_diff_negative() {
        let result = pretty_count_diff(-5);
        assert!(result.contains("-5"));
        assert!(result.contains("green"));
    }

    #[test]
    fn test_pretty_count_diff_zero() {
        let result = pretty_count_diff(0);
        assert!(result.contains("0"));
        assert!(result.contains("gray"));
    }

    #[test]
    fn test_pretty_line_empty() {
        let result = pretty_line("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_pretty_line_with_content() {
        let result = pretty_line("test content");
        assert!(result.contains("test content"));
        assert!(result.contains("white"));
    }

    #[test]
    fn test_frame_creation() {
        let frame = Frame::new("src/main.rs".to_string(), 42);
        assert_eq!(frame.filename, "src/main.rs");
        assert_eq!(frame.lineno, 42);
    }

    #[test]
    fn test_pretty_frame_info_with_number() {
        let frame = Frame::new("src/test.rs".to_string(), 10);
        let result = pretty_frame_info(&frame, Some(0));

        assert!(result.contains("# 1:"));
        assert!(result.contains("src/test.rs"));
        assert!(result.contains("10"));
    }

    #[test]
    fn test_pretty_frame_info_without_number() {
        let frame = Frame::new("src/test.rs".to_string(), 10);
        let result = pretty_frame_info(&frame, None);

        assert!(!result.contains("# 1:"));
        assert!(result.contains("src/test.rs"));
        assert!(result.contains("10"));
    }

    #[test]
    fn test_statistic_creation() {
        let frames = vec![Frame::new("test.rs".to_string(), 1)];
        let stat = Statistic::new(frames.clone(), 1024, 5);

        assert_eq!(stat.size, 1024);
        assert_eq!(stat.count, 5);
        assert_eq!(stat.traceback.len(), 1);
    }

    #[test]
    fn test_statistic_diff_creation() {
        let frames = vec![Frame::new("test.rs".to_string(), 1)];
        let diff = StatisticDiff::new(frames, 512, -2);

        assert_eq!(diff.size_diff, 512);
        assert_eq!(diff.count_diff, -2);
    }

    #[test]
    fn test_snapshot_creation() {
        let snapshot = Snapshot::new();
        assert!(snapshot.memory > 0);
        assert!(!snapshot.statistics.is_empty());
    }

    #[test]
    fn test_snapshot_comparison() {
        let snap1 = Snapshot::new();
        thread::sleep(Duration::from_millis(10));
        let snap2 = Snapshot::new();

        let diffs = snap2.compare_to(&snap1, "lineno", true);
        assert!(!diffs.is_empty());

        if let Some(diff) = diffs.first() {
            assert!(!diff.traceback.is_empty());
        }
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
    fn test_memtracer_take_snapshot() {
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
        assert!(tracer.running);

        tracer.stop();
        assert!(!tracer.running);
    }

    #[test]
    fn test_memtracer_multiple_snapshots() {
        let tracer = MemTracer::get();
        let mut tracer = tracer.lock().unwrap();
        // Reset state first
        tracer.curr_snapshot = None;
        tracer.prev_snapshot = None;
        tracer.running = true;

        tracer.take_snapshot();
        assert!(tracer.curr_snapshot.is_some());
        assert!(tracer.prev_snapshot.is_none());

        thread::sleep(Duration::from_millis(10));
        tracer.take_snapshot();
        assert!(tracer.curr_snapshot.is_some());
        assert!(tracer.prev_snapshot.is_some());
    }

    #[test]
    fn test_snapshot_with_timestamp() {
        let snap1 = Snapshot::new();
        thread::sleep(Duration::from_millis(10));
        let snap2 = Snapshot::new();

        assert!(snap2.timestamp > snap1.timestamp);
    }
}
