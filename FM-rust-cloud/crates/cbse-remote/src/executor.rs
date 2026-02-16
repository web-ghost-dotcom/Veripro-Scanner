// SPDX-License-Identifier: AGPL-3.0

//! Remote execution orchestrator

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::artifact::{JobArtifact, JobResult};
use crate::ssh::SshConnection;

/// Remote execution orchestrator
pub struct RemoteExecutor {
    connection: SshConnection,
    remote_workdir: String,
    remote_binary: String,
}

impl RemoteExecutor {
    /// Create a new remote executor
    pub fn new(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        remote_workdir: &str,
        remote_binary: &str,
    ) -> Result<Self> {
        let connection = SshConnection::connect(host, port, username, password)?;

        Ok(Self {
            connection,
            remote_workdir: remote_workdir.to_string(),
            remote_binary: remote_binary.to_string(),
        })
    }

    /// Execute a job on the remote node
    pub fn execute(&self, artifact: &JobArtifact) -> Result<JobResult> {
        let job_id = Uuid::new_v4().to_string();

        println!("🚀 Starting remote execution (Job ID: {})", job_id);

        // 1. Create local temp directory
        let local_workdir = format!("/tmp/cbse-local-{}", job_id);
        fs::create_dir_all(&local_workdir).context("Failed to create local working directory")?;

        let artifact_path = format!("{}/artifact.json", local_workdir);
        fs::write(&artifact_path, serde_json::to_string_pretty(&artifact)?)
            .context("Failed to write artifact file")?;

        // 2. Create remote job directory
        let remote_job_dir = format!("{}/{}", self.remote_workdir, job_id);
        self.connection.mkdir(&remote_job_dir)?;

        println!(
            "📤 Uploading artifacts to {}...",
            self.connection.get_host()
        );

        // 3. Upload artifact
        self.connection.upload_file(
            Path::new(&artifact_path),
            &format!("{}/artifact.json", remote_job_dir),
        )?;

        println!("⚙️  Executing CBSE on remote node...");

        // 4. Execute remotely
        let remote_cmd = format!(
            "cd {} && {} --worker-mode --input artifact.json --output result.json 2>&1",
            remote_job_dir, self.remote_binary
        );

        let start = std::time::Instant::now();
        let (stdout, stderr, exit_code) = self.connection.exec(&remote_cmd)?;
        let duration = start.elapsed();

        // Print remote output for debugging
        if !stdout.is_empty() {
            println!("\n📋 Remote output:");
            for line in stdout.lines() {
                println!("   {}", line);
            }
        }
        if !stderr.is_empty() && exit_code > 1 {
            eprintln!("\n⚠️  Remote stderr:");
            for line in stderr.lines() {
                eprintln!("   {}", line);
            }
        }

        // Exit code 0 = all passed, 1 = some failed (expected), >1 = error
        if exit_code > 1 {
            self.connection.remove(&remote_job_dir)?;
            fs::remove_dir_all(&local_workdir)?;
            anyhow::bail!("Remote execution failed with exit code {}", exit_code);
        }

        println!("📥 Downloading results...");

        // 5. Download results
        let result_path = format!("{}/result.json", local_workdir);
        self.connection.download_file(
            &format!("{}/result.json", remote_job_dir),
            Path::new(&result_path),
        )?;

        // 6. Parse results
        let result_content =
            fs::read_to_string(&result_path).context("Failed to read result file")?;
        let result: JobResult =
            serde_json::from_str(&result_content).context("Failed to parse result JSON")?;

        println!(
            "✅ Remote execution complete in {:.2}s",
            duration.as_secs_f64()
        );

        // 7. Cleanup
        self.connection.remove(&remote_job_dir)?;
        fs::remove_dir_all(&local_workdir)?;

        Ok(result)
    }

    /// Test connection and verify remote binary exists
    pub fn test_connection(&self) -> Result<()> {
        println!("🔍 Testing remote connection...");

        // Check if binary exists
        if !self.connection.path_exists(&self.remote_binary)? {
            anyhow::bail!(
                "Remote CBSE binary not found at: {}\nPlease run: ./scripts/setup-remote-node.sh {}",
                self.remote_binary,
                self.connection.get_host()
            );
        }

        // Check if working directory exists or can be created
        self.connection.mkdir(&self.remote_workdir)?;

        // Test execution
        let (stdout, _, exit_code) = self
            .connection
            .exec(&format!("{} --version", self.remote_binary))?;

        if exit_code == 0 {
            println!("✅ Remote CBSE version: {}", stdout.trim());
        } else {
            anyhow::bail!("Failed to execute remote binary");
        }

        println!("✅ Connection test successful");

        Ok(())
    }
}
