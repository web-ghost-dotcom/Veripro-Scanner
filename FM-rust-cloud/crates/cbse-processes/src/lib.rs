// SPDX-License-Identifier: AGPL-3.0

use std::collections::HashSet;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, Once, Weak};
use std::thread;
use std::time::{Duration, Instant};

/// Executor registry for managing executors (singleton pattern)
pub struct ExecutorRegistry {
    executors: Mutex<Vec<Weak<Mutex<PopenExecutor>>>>,
}

impl ExecutorRegistry {
    fn new() -> Self {
        Self {
            executors: Mutex::new(Vec::new()),
        }
    }

    pub fn instance() -> &'static ExecutorRegistry {
        static mut INSTANCE: Option<ExecutorRegistry> = None;
        static ONCE: Once = Once::new();

        unsafe {
            ONCE.call_once(|| {
                INSTANCE = Some(ExecutorRegistry::new());
            });
            INSTANCE.as_ref().unwrap()
        }
    }

    pub fn register(&self, executor: Weak<Mutex<PopenExecutor>>) {
        if let Ok(mut executors) = self.executors.lock() {
            executors.push(executor);
        }
    }

    pub fn shutdown_all(&self) {
        if let Ok(executors) = self.executors.lock() {
            for weak_executor in executors.iter() {
                if let Some(executor) = weak_executor.upgrade() {
                    if let Ok(mut ex) = executor.lock() {
                        ex.shutdown(false, false);
                    }
                }
            }
        }
    }
}

/// Future for a subprocess execution
pub struct PopenFuture {
    pub cmd: Vec<String>,
    pub timeout: Option<Duration>,
    pub process: Option<Child>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub returncode: Option<i32>,
    pub start_time: Option<Instant>,
    pub end_time: Option<Instant>,
    exception: Option<String>,
    done: bool,
}

impl PopenFuture {
    pub fn new(cmd: Vec<String>, timeout: Option<f64>) -> Self {
        Self {
            cmd,
            timeout: timeout.map(Duration::from_secs_f64),
            process: None,
            stdout: None,
            stderr: None,
            returncode: None,
            start_time: None,
            end_time: None,
            exception: None,
            done: false,
        }
    }

    /// Start the subprocess and immediately return
    pub fn start(&mut self) -> Result<(), String> {
        if self.cmd.is_empty() {
            return Err("Empty command".to_string());
        }

        self.start_time = Some(Instant::now());

        let program = &self.cmd[0];
        let args = &self.cmd[1..];

        match Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => {
                self.process = Some(child);
                Ok(())
            }
            Err(e) => Err(format!("Failed to spawn process: {}", e)),
        }
    }

    /// Cancel/terminate the process
    pub fn cancel(&mut self) {
        if let Some(ref mut process) = self.process {
            let _ = process.kill();
            let _ = process.wait();
        }
    }

    /// Check if the process is currently running
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut process) = self.process {
            match process.try_wait() {
                Ok(None) => true,
                Ok(Some(status)) => {
                    if self.returncode.is_none() {
                        self.returncode = status.code();
                        self.end_time = Some(Instant::now());
                        self.done = true;
                    }
                    false
                }
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Wait for the process to complete and get the result
    pub fn result(&mut self) -> Result<(String, String, i32), String> {
        if self.done {
            return Ok((
                self.stdout.clone().unwrap_or_default(),
                self.stderr.clone().unwrap_or_default(),
                self.returncode.unwrap_or(-1),
            ));
        }

        if let Some(mut process) = self.process.take() {
            let start = Instant::now();

            // Check for timeout
            loop {
                match process.try_wait() {
                    Ok(Some(status)) => {
                        // Process finished
                        if let Some(stdout) = process.stdout.take() {
                            use std::io::Read;
                            let mut buf = String::new();
                            if std::io::BufReader::new(stdout).read_to_string(&mut buf).is_ok() {
                                self.stdout = Some(buf);
                            }
                        }

                        if let Some(stderr) = process.stderr.take() {
                            use std::io::Read;
                            let mut buf = String::new();
                            if std::io::BufReader::new(stderr).read_to_string(&mut buf).is_ok() {
                                self.stderr = Some(buf);
                            }
                        }

                        self.returncode = status.code();
                        self.end_time = Some(Instant::now());
                        self.done = true;

                        return Ok((
                            self.stdout.clone().unwrap_or_default(),
                            self.stderr.clone().unwrap_or_default(),
                            self.returncode.unwrap_or(-1),
                        ));
                    }
                    Ok(None) => {
                        // Still running
                        if let Some(timeout) = self.timeout {
                            if start.elapsed() > timeout {
                                // Timeout - kill the process
                                let _ = process.kill();
                                let _ = process.wait();
                                self.exception = Some(format!("Timeout after {:?}", timeout));
                                self.end_time = Some(Instant::now());
                                self.done = true;
                                return Err("Timeout".to_string());
                            }
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => {
                        return Err(format!("Error waiting for process: {}", e));
                    }
                }
            }
        }

        Err("No process to wait for".to_string())
    }

    /// Check if the future is done
    pub fn done(&self) -> bool {
        self.done
    }

    /// Get any exception that occurred
    pub fn exception(&self) -> Option<&String> {
        self.exception.as_ref()
    }
}

/// Error raised when submitting to a shutdown executor
#[derive(Debug)]
pub struct ShutdownError;

impl std::fmt::Display for ShutdownError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Executor has been shutdown")
    }
}

impl std::error::Error for ShutdownError {}

/// Executor for running subprocesses
pub struct PopenExecutor {
    futures: Vec<PopenFuture>,
    shutdown: bool,
    max_workers: usize,
}

impl PopenExecutor {
    pub fn new(max_workers: usize) -> Arc<Mutex<Self>> {
        let executor = Arc::new(Mutex::new(Self {
            futures: Vec::new(),
            shutdown: false,
            max_workers,
        }));

        // Register with the global registry
        ExecutorRegistry::instance().register(Arc::downgrade(&executor));

        executor
    }

    pub fn futures(&self) -> &Vec<PopenFuture> {
        &self.futures
    }

    pub fn submit(&mut self, mut future: PopenFuture) -> Result<usize, ShutdownError> {
        if self.shutdown {
            return Err(ShutdownError);
        }

        if let Err(e) = future.start() {
            eprintln!("Failed to start future: {}", e);
            return Err(ShutdownError);
        }

        self.futures.push(future);
        Ok(self.futures.len() - 1)
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    pub fn shutdown(&mut self, wait: bool, cancel_futures: bool) {
        self.shutdown = true;

        if wait {
            // Wait for all futures to complete
            for future in self.futures.iter_mut() {
                let _ = future.result();
            }
        } else if cancel_futures {
            // Cancel all running futures
            for future in self.futures.iter_mut() {
                future.cancel();
            }
        }
    }

    fn join(&mut self) {
        for future in self.futures.iter_mut() {
            let _ = future.result();
        }
    }
}

impl Drop for PopenExecutor {
    fn drop(&mut self) {
        if !self.shutdown {
            self.shutdown(false, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_registry_singleton() {
        let registry1 = ExecutorRegistry::instance();
        let registry2 = ExecutorRegistry::instance();
        assert!(std::ptr::eq(registry1, registry2));
    }

    #[test]
    fn test_popen_future_new() {
        let cmd = vec!["echo".to_string(), "hello".to_string()];
        let future = PopenFuture::new(cmd.clone(), Some(5.0));
        assert_eq!(future.cmd, cmd);
        assert!(future.timeout.is_some());
        assert_eq!(future.timeout.unwrap(), Duration::from_secs(5));
    }

    #[test]
    fn test_popen_future_no_timeout() {
        let cmd = vec!["echo".to_string(), "test".to_string()];
        let future = PopenFuture::new(cmd, None);
        assert!(future.timeout.is_none());
    }

    #[test]
    fn test_popen_future_start() {
        let cmd = vec!["echo".to_string(), "test".to_string()];
        let mut future = PopenFuture::new(cmd, None);
        let result = future.start();
        assert!(result.is_ok());
        assert!(future.process.is_some());
        assert!(future.start_time.is_some());
    }

    #[test]
    fn test_popen_future_result() {
        let cmd = vec!["echo".to_string(), "hello".to_string()];
        let mut future = PopenFuture::new(cmd, None);
        future.start().unwrap();
        
        let result = future.result();
        assert!(result.is_ok());
        
        let (stdout, stderr, code) = result.unwrap();
        assert!(stdout.contains("hello") || !stdout.is_empty());
        assert_eq!(code, 0);
    }

    #[test]
    fn test_popen_future_cancel() {
        let cmd = vec!["sleep".to_string(), "10".to_string()];
        let mut future = PopenFuture::new(cmd, None);
        future.start().unwrap();
        
        thread::sleep(Duration::from_millis(50));
        future.cancel();
        
        // Process should be terminated
        assert!(!future.is_running());
    }

    #[test]
    fn test_popen_future_timeout() {
        let cmd = vec!["sleep".to_string(), "5".to_string()];
        let mut future = PopenFuture::new(cmd, Some(0.1));
        future.start().unwrap();
        
        let result = future.result();
        assert!(result.is_err());
        assert!(future.exception().is_some());
    }

    #[test]
    fn test_executor_new() {
        let executor = PopenExecutor::new(4);
        let ex = executor.lock().unwrap();
        assert_eq!(ex.max_workers, 4);
        assert!(!ex.is_shutdown());
    }

    #[test]
    fn test_executor_submit() {
        let executor = PopenExecutor::new(4);
        let mut ex = executor.lock().unwrap();
        
        let cmd = vec!["echo".to_string(), "test".to_string()];
        let future = PopenFuture::new(cmd, None);
        
        let result = ex.submit(future);
        assert!(result.is_ok());
        assert_eq!(ex.futures().len(), 1);
    }

    #[test]
    fn test_executor_shutdown() {
        let executor = PopenExecutor::new(4);
        let mut ex = executor.lock().unwrap();
        
        assert!(!ex.is_shutdown());
        ex.shutdown(false, false);
        assert!(ex.is_shutdown());
    }

    #[test]
    fn test_executor_shutdown_with_cancel() {
        let executor = PopenExecutor::new(4);
        let mut ex = executor.lock().unwrap();
        
        let cmd = vec!["sleep".to_string(), "10".to_string()];
        let future = PopenFuture::new(cmd, None);
        ex.submit(future).unwrap();
        
        ex.shutdown(false, true);
        assert!(ex.is_shutdown());
    }

    #[test]
    fn test_executor_submit_after_shutdown() {
        let executor = PopenExecutor::new(4);
        let mut ex = executor.lock().unwrap();
        
        ex.shutdown(false, false);
        
        let cmd = vec!["echo".to_string(), "test".to_string()];
        let future = PopenFuture::new(cmd, None);
        let result = ex.submit(future);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_executor_registry_shutdown_all() {
        let executor1 = PopenExecutor::new(2);
        let executor2 = PopenExecutor::new(2);
        
        ExecutorRegistry::instance().shutdown_all();
        
        let ex1 = executor1.lock().unwrap();
        let ex2 = executor2.lock().unwrap();
        assert!(ex1.is_shutdown());
        assert!(ex2.is_shutdown());
    }

    #[test]
    fn test_popen_future_done() {
        let cmd = vec!["echo".to_string(), "test".to_string()];
        let mut future = PopenFuture::new(cmd, None);
        
        assert!(!future.done());
        future.start().unwrap();
        let _ = future.result();
        assert!(future.done());
    }

    #[test]
    fn test_multiple_futures() {
        let executor = PopenExecutor::new(4);
        let mut ex = executor.lock().unwrap();
        
        for i in 0..3 {
            let cmd = vec!["echo".to_string(), format!("test{}", i)];
            let future = PopenFuture::new(cmd, None);
            ex.submit(future).unwrap();
        }
        
        assert_eq!(ex.futures().len(), 3);
    }
}
