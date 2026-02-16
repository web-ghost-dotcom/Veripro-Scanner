// SPDX-License-Identifier: AGPL-3.0

//! SSH connection handling with password authentication

use anyhow::{Context, Result};
use ssh2::Session;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;

/// SSH connection wrapper
pub struct SshConnection {
    session: Session,
    host: String,
}

impl SshConnection {
    /// Connect to SSH server with password authentication
    pub fn connect(host: &str, port: u16, username: &str, password: &str) -> Result<Self> {
        println!("🔌 Connecting to {}@{}:{}...", username, host, port);

        // Connect to TCP socket
        let tcp = TcpStream::connect(format!("{}:{}", host, port))
            .context(format!("Failed to connect to {}:{}", host, port))?;

        // Create SSH session
        let mut session = Session::new().context("Failed to create SSH session")?;

        session.set_tcp_stream(tcp);
        session.handshake().context("SSH handshake failed")?;

        // Authenticate with password
        session
            .userauth_password(username, password)
            .context("SSH password authentication failed")?;

        if !session.authenticated() {
            anyhow::bail!("SSH authentication failed - check username/password");
        }

        println!("✅ SSH connection established");

        Ok(Self {
            session,
            host: host.to_string(),
        })
    }

    /// Execute command on remote host
    pub fn exec(&self, cmd: &str) -> Result<(String, String, i32)> {
        let mut channel = self
            .session
            .channel_session()
            .context("Failed to open SSH channel")?;

        channel.exec(cmd).context("Failed to execute command")?;

        // Read stdout
        let mut stdout = String::new();
        channel
            .read_to_string(&mut stdout)
            .context("Failed to read stdout")?;

        // Read stderr
        let mut stderr = String::new();
        channel
            .stderr()
            .read_to_string(&mut stderr)
            .context("Failed to read stderr")?;

        // Wait for exit
        channel.wait_close().context("Failed to close channel")?;

        let exit_code = channel.exit_status()?;

        Ok((stdout, stderr, exit_code))
    }

    /// Upload file via SFTP
    pub fn upload_file(&self, local_path: &Path, remote_path: &str) -> Result<()> {
        let sftp = self
            .session
            .sftp()
            .context("Failed to create SFTP session")?;

        let local_data = std::fs::read(local_path).context(format!(
            "Failed to read local file: {}",
            local_path.display()
        ))?;

        let mut remote_file = sftp
            .create(Path::new(remote_path))
            .context(format!("Failed to create remote file: {}", remote_path))?;

        remote_file
            .write_all(&local_data)
            .context("Failed to write to remote file")?;

        Ok(())
    }

    /// Download file via SFTP
    pub fn download_file(&self, remote_path: &str, local_path: &Path) -> Result<()> {
        let sftp = self
            .session
            .sftp()
            .context("Failed to create SFTP session")?;

        let mut remote_file = sftp
            .open(Path::new(remote_path))
            .context(format!("Failed to open remote file: {}", remote_path))?;

        let mut contents = Vec::new();
        remote_file
            .read_to_end(&mut contents)
            .context("Failed to read remote file")?;

        std::fs::write(local_path, contents).context(format!(
            "Failed to write local file: {}",
            local_path.display()
        ))?;

        Ok(())
    }

    /// Check if remote path exists
    pub fn path_exists(&self, path: &str) -> Result<bool> {
        let (_, _, exit_code) = self.exec(&format!("test -e {}", path))?;
        Ok(exit_code == 0)
    }

    /// Create remote directory
    pub fn mkdir(&self, path: &str) -> Result<()> {
        let (_, stderr, exit_code) = self.exec(&format!("mkdir -p {}", path))?;

        if exit_code != 0 {
            anyhow::bail!("Failed to create directory: {}", stderr);
        }

        Ok(())
    }

    /// Remove remote path (file or directory)
    pub fn remove(&self, path: &str) -> Result<()> {
        let (_, stderr, exit_code) = self.exec(&format!("rm -rf {}", path))?;

        if exit_code != 0 {
            anyhow::bail!("Failed to remove path: {}", stderr);
        }

        Ok(())
    }

    pub fn get_host(&self) -> &str {
        &self.host
    }
}

impl Drop for SshConnection {
    fn drop(&mut self) {
        let _ = self.session.disconnect(None, "Closing connection", None);
    }
}
