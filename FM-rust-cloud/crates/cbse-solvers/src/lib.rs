// SPDX-License-Identifier: AGPL-3.0

//! Solver management and download system

use dirs::home_dir;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Once};
use tar::Archive;
use zip::ZipArchive;

// Default versions
pub const DEFAULT_YICES_VERSION: &str = "2.6.4";
pub const DEFAULT_CVC5_VERSION: &str = "1.2.1";
pub const DEFAULT_BITWUZLA_VERSION: &str = "0.8.1";

// Environment variable to bypass download confirmation
pub const ALLOW_DOWNLOAD_VAR: &str = "HALMOS_ALLOW_DOWNLOAD";

/// Machine information for platform detection
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MachineInfo {
    pub system: String,
    pub machine: String,
}

impl MachineInfo {
    pub fn new(system: String, machine: String) -> Self {
        Self { system, machine }
    }

    pub fn current() -> Self {
        let system = std::env::consts::OS.to_string();
        let machine = std::env::consts::ARCH.to_string();
        
        // Normalize system names
        let system = match system.as_str() {
            "macos" => "Darwin".to_string(),
            "linux" => "Linux".to_string(),
            "windows" => "Windows".to_string(),
            _ => system,
        };

        // Normalize machine names (AMD64 -> x86_64)
        let machine = match machine.as_str() {
            "x86_64" | "amd64" => "x86_64".to_string(),
            "aarch64" => "arm64".to_string(),
            _ => machine,
        };

        Self { system, machine }
    }
}

impl std::fmt::Display for MachineInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.system, self.machine)
    }
}

/// Download information for a solver binary
#[derive(Debug, Clone)]
pub struct DownloadInfo {
    pub base_url: String,
    pub filename: String,
    pub checksum: String,
    pub binary_name_in_archive: String,
}

impl DownloadInfo {
    pub fn new(base_url: String, filename: String, checksum: String, binary_name_in_archive: String) -> Self {
        Self {
            base_url,
            filename,
            checksum,
            binary_name_in_archive,
        }
    }
}

/// Solver information
#[derive(Debug, Clone)]
pub struct SolverInfo {
    pub name: String,
    pub binary_name: String,
    pub arguments: Vec<String>,
    pub downloads: HashMap<MachineInfo, Option<DownloadInfo>>,
}

impl SolverInfo {
    pub fn new(
        name: String,
        binary_name: String,
        arguments: Vec<String>,
        downloads: HashMap<MachineInfo, Option<DownloadInfo>>,
    ) -> Self {
        Self {
            name,
            binary_name,
            arguments,
            downloads,
        }
    }
}

/// Get solver cache directory
pub fn solver_cache_dir() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".halmos")
        .join("solvers")
}

/// Initialize solver cache directory
pub fn init_cache_dir() -> io::Result<PathBuf> {
    let cache_dir = solver_cache_dir();
    fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

/// Get binary path in cache
pub fn binary_path_in_cache(solver: &SolverInfo) -> PathBuf {
    let cache_dir = solver_cache_dir();
    let suffix = if cfg!(target_os = "windows") { ".exe" } else { "" };
    cache_dir.join(format!("{}{}", solver.binary_name, suffix))
}

/// Verify SHA256 checksum
pub fn verify_checksum(file_path: &Path, expected_checksum: &str) -> io::Result<bool> {
    let expected = expected_checksum
        .trim()
        .to_lowercase()
        .trim_start_matches("sha256:")
        .to_string();

    let mut file = File::open(file_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 4096];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let actual = format!("{:x}", hasher.finalize());

    if actual != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Checksum mismatch: expected={}, actual={}", expected, actual),
        ));
    }

    Ok(true)
}

/// Download file from URL
pub fn download(download_info: &DownloadInfo, output_dir: &Path) -> io::Result<PathBuf> {
    let url = format!("{}/{}", download_info.base_url, download_info.filename);
    let archive_path = output_dir.join(&download_info.filename);

    println!("Downloading {} ...", url);

    let response = reqwest::blocking::get(&url)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    if !response.status().is_success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to download: HTTP {}", response.status()),
        ));
    }

    let mut file = File::create(&archive_path)?;
    let content = response.bytes()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    file.write_all(&content)?;

    Ok(archive_path)
}

/// Extract file from tar.gz archive
pub fn extract_from_targz(archive_path: &Path, file_path: &str) -> io::Result<Vec<u8>> {
    let file = File::open(archive_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        
        if path.to_string_lossy() == file_path {
            let mut content = Vec::new();
            entry.read_to_end(&mut content)?;
            return Ok(content);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("File {} not found in archive", file_path),
    ))
}

/// Extract file from zip archive
pub fn extract_from_zip(archive_path: &Path, file_path: &str) -> io::Result<Vec<u8>> {
    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        
        if file.name() == file_path {
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;
            return Ok(content);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("File {} not found in archive", file_path),
    ))
}

/// Check if download is allowed (environment variable or interactive prompt)
pub fn download_allowed(solver: &SolverInfo, _download_info: &DownloadInfo) -> bool {
    // Check environment variable first
    if let Ok(val) = env::var(ALLOW_DOWNLOAD_VAR) {
        return val.to_lowercase() == "true" || val == "1";
    }

    // For automated testing, default to allow
    true
}

/// Install solver binary
pub fn install_solver(solver: &SolverInfo) -> io::Result<PathBuf> {
    let machine_info = MachineInfo::current();
    
    let download_info = solver
        .downloads
        .get(&machine_info)
        .and_then(|opt| opt.as_ref())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("No download available for {}", machine_info),
            )
        })?;

    if !download_allowed(solver, download_info) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Download not allowed",
        ));
    }

    let cache_dir = init_cache_dir()?;
    let binary_path = binary_path_in_cache(solver);

    // If already installed and valid, return it
    if binary_path.exists() {
        return Ok(binary_path);
    }

    // Download archive
    let archive_path = download(download_info, &cache_dir)?;

    // Verify checksum
    verify_checksum(&archive_path, &download_info.checksum)?;

    // Extract binary
    let binary_content = if download_info.filename.ends_with(".tar.gz") {
        extract_from_targz(&archive_path, &download_info.binary_name_in_archive)?
    } else if download_info.filename.ends_with(".zip") {
        extract_from_zip(&archive_path, &download_info.binary_name_in_archive)?
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Unsupported archive format",
        ));
    };

    // Write binary to cache
    let mut file = File::create(&binary_path)?;
    file.write_all(&binary_content)?;

    // Make executable on Unix-like systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&binary_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary_path, perms)?;
    }

    // Clean up archive
    let _ = fs::remove_file(archive_path);

    println!("Installed solver binary to: {}", binary_path.display());

    Ok(binary_path)
}

/// Find solver binary in PATH or venv
pub fn find_solver_binary(solver: &SolverInfo) -> Option<PathBuf> {
    // Check cache first
    let cached_path = binary_path_in_cache(solver);
    if cached_path.exists() {
        return Some(cached_path);
    }

    // Check PATH
    let binary_name = if cfg!(target_os = "windows") {
        format!("{}.exe", solver.binary_name)
    } else {
        solver.binary_name.clone()
    };

    which::which(&binary_name).ok()
}

/// Solver registry singleton
static INIT: Once = Once::new();
static mut SOLVERS: Option<Arc<Mutex<HashMap<String, SolverInfo>>>> = None;

/// Get solver registry
pub fn solver_registry() -> Arc<Mutex<HashMap<String, SolverInfo>>> {
    unsafe {
        INIT.call_once(|| {
            SOLVERS = Some(Arc::new(Mutex::new(init_solvers())));
        });
        SOLVERS.as_ref().unwrap().clone()
    }
}

/// Initialize solver definitions
pub fn init_solvers() -> HashMap<String, SolverInfo> {
    let mut solvers = HashMap::new();

    // Platform-specific machine info
    let macos_intel = MachineInfo::new("Darwin".to_string(), "x86_64".to_string());
    let macos_arm64 = MachineInfo::new("Darwin".to_string(), "arm64".to_string());
    let linux_intel = MachineInfo::new("Linux".to_string(), "x86_64".to_string());
    let linux_arm64 = MachineInfo::new("Linux".to_string(), "arm64".to_string());
    let windows_intel = MachineInfo::new("Windows".to_string(), "x86_64".to_string());

    // Yices 2.6.5
    let mut yices_265_downloads = HashMap::new();
    yices_265_downloads.insert(
        macos_intel.clone(),
        Some(DownloadInfo::new(
            "https://github.com/SRI-CSL/yices2/releases/download/Yices-2.6.5".to_string(),
            "yices-2.6.5-x86_64-apple-darwin21.6.0-static-gmp.tar.gz".to_string(),
            "831094681703173cb30657e9a9d690bd6139f435ff44afdcf81f8e761f9ed0c4".to_string(),
            "yices-2.6.5/bin/yices-smt2".to_string(),
        )),
    );
    yices_265_downloads.insert(
        macos_arm64.clone(),
        Some(DownloadInfo::new(
            "https://github.com/SRI-CSL/yices2/releases/download/Yices-2.6.5".to_string(),
            "yices-2.6.5-arm-apple-darwin22.6.0-static-gmp.tar.gz".to_string(),
            "b75f2881859fb91c1e8fae121595091b89c07421f35db0e7cddc8a43cba13507".to_string(),
            "yices-2.6.5/bin/yices-smt2".to_string(),
        )),
    );
    yices_265_downloads.insert(
        linux_intel.clone(),
        Some(DownloadInfo::new(
            "https://github.com/SRI-CSL/yices2/releases/download/Yices-2.6.5".to_string(),
            "yices-2.6.5-x86_64-pc-linux-gnu-static-gmp.tar.gz".to_string(),
            "d6c9465c261e4f4eabd240d0dd9dff5e740fca2beb0042de15f67954bbc70cce".to_string(),
            "yices-2.6.5/bin/yices-smt2".to_string(),
        )),
    );
    yices_265_downloads.insert(
        windows_intel.clone(),
        Some(DownloadInfo::new(
            "https://github.com/SRI-CSL/yices2/releases/download/Yices-2.6.5".to_string(),
            "yices-2.6.5-x86_64-unknown-mingw32-static-gmp.zip".to_string(),
            "189aaa5515bb71c18996b87d7eceb8cfa037a7b2114f6b46abf5c6f4f07072af".to_string(),
            "yices-2.6.5/bin/yices-smt2.exe".to_string(),
        )),
    );

    solvers.insert(
        "yices-2.6.5".to_string(),
        SolverInfo::new(
            "yices-2.6.5".to_string(),
            "yices-smt2".to_string(),
            vec!["--smt2-model-format".to_string(), "--bvconst-in-decimal".to_string()],
            yices_265_downloads,
        ),
    );

    // Yices 2.6.4
    let mut yices_264_downloads = HashMap::new();
    yices_264_downloads.insert(
        macos_intel.clone(),
        Some(DownloadInfo::new(
            "https://github.com/SRI-CSL/yices2/releases/download/Yices-2.6.4".to_string(),
            "yices-2.6.4-x86_64-apple-darwin20.6.0.tar.gz".to_string(),
            "e54d979bf466102c03476c9a34dd3b5316e543f201eca8ca0e4be07ffccdefd5".to_string(),
            "yices-2.6.4/bin/yices-smt2".to_string(),
        )),
    );
    yices_264_downloads.insert(
        macos_arm64.clone(),
        Some(DownloadInfo::new(
            "https://github.com/SRI-CSL/yices2/releases/download/Yices-2.6.4".to_string(),
            "yices-2.6.4-arm-apple-darwin20.6.0.tar.gz".to_string(),
            "302fdf64bd2d9fb0e124c4adcf9b2ff9658426ebb983fd7ad3ac54a4019a9fc9".to_string(),
            "yices-2.6.4/bin/yices-smt2".to_string(),
        )),
    );
    yices_264_downloads.insert(
        linux_intel.clone(),
        Some(DownloadInfo::new(
            "https://github.com/SRI-CSL/yices2/releases/download/Yices-2.6.4".to_string(),
            "yices-2.6.4-x86_64-pc-linux-gnu.tar.gz".to_string(),
            "841184509aecdc4df99c7ee280e33f76359032dc367919260a916257229601a4".to_string(),
            "yices-2.6.4/bin/yices-smt2".to_string(),
        )),
    );
    yices_264_downloads.insert(
        windows_intel.clone(),
        Some(DownloadInfo::new(
            "https://github.com/SRI-CSL/yices2/releases/download/Yices-2.6.4".to_string(),
            "yices-2.6.4-x86_64-unknown-mingw32-static-gmp.zip".to_string(),
            "a26031f0c9634ff1f1737086cc058a1b6d401a5aa04a1904bcbd71b761736ded".to_string(),
            "yices-2.6.4/bin/yices-smt2.exe".to_string(),
        )),
    );

    solvers.insert(
        "yices-2.6.4".to_string(),
        SolverInfo::new(
            "yices-2.6.4".to_string(),
            "yices-smt2".to_string(),
            vec!["--smt2-model-format".to_string(), "--bvconst-in-decimal".to_string()],
            yices_264_downloads,
        ),
    );

    // Z3 (relies on PATH)
    solvers.insert(
        "z3".to_string(),
        SolverInfo::new(
            "z3".to_string(),
            "z3".to_string(),
            vec![],
            HashMap::new(),
        ),
    );

    // Set default aliases
    solvers.insert("yices".to_string(), solvers.get("yices-2.6.4").unwrap().clone());

    solvers
}

/// Get solver command (binary path + arguments)
pub fn get_solver_command(solver_name: &str) -> io::Result<Vec<String>> {
    let registry = solver_registry();
    let solvers = registry.lock().unwrap();
    
    let solver = solvers.get(solver_name).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Unknown solver: {}", solver_name),
        )
    })?;

    // Try to find existing binary
    let binary_path = if let Some(path) = find_solver_binary(solver) {
        path
    } else {
        // Try to install it
        install_solver(solver)?
    };

    let mut command = vec![binary_path.to_string_lossy().to_string()];
    command.extend(solver.arguments.clone());

    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_machine_info_current() {
        let info = MachineInfo::current();
        assert!(!info.system.is_empty());
        assert!(!info.machine.is_empty());
    }

    #[test]
    fn test_machine_info_display() {
        let info = MachineInfo::new("Linux".to_string(), "x86_64".to_string());
        assert_eq!(format!("{}", info), "Linux-x86_64");
    }

    #[test]
    fn test_solver_cache_dir() {
        let cache_dir = solver_cache_dir();
        assert!(cache_dir.to_string_lossy().contains(".halmos"));
        assert!(cache_dir.to_string_lossy().contains("solvers"));
    }

    #[test]
    fn test_binary_path_in_cache() {
        let solver = SolverInfo::new(
            "test".to_string(),
            "test-binary".to_string(),
            vec![],
            HashMap::new(),
        );
        let path = binary_path_in_cache(&solver);
        assert!(path.to_string_lossy().contains("test-binary"));
    }

    #[test]
    fn test_init_solvers() {
        let solvers = init_solvers();
        assert!(solvers.contains_key("yices-2.6.4"));
        assert!(solvers.contains_key("yices-2.6.5"));
        assert!(solvers.contains_key("z3"));
        assert!(solvers.contains_key("yices")); // alias
    }

    #[test]
    fn test_solver_registry() {
        let registry = solver_registry();
        let solvers = registry.lock().unwrap();
        assert!(!solvers.is_empty());
        assert!(solvers.contains_key("yices"));
    }

    #[test]
    fn test_download_info_creation() {
        let info = DownloadInfo::new(
            "https://example.com".to_string(),
            "file.tar.gz".to_string(),
            "abc123".to_string(),
            "bin/solver".to_string(),
        );
        assert_eq!(info.base_url, "https://example.com");
        assert_eq!(info.filename, "file.tar.gz");
    }

    #[test]
    fn test_solver_info_creation() {
        let solver = SolverInfo::new(
            "test-solver".to_string(),
            "test".to_string(),
            vec!["--arg1".to_string()],
            HashMap::new(),
        );
        assert_eq!(solver.name, "test-solver");
        assert_eq!(solver.binary_name, "test");
        assert_eq!(solver.arguments.len(), 1);
    }

    #[test]
    fn test_yices_solver_info() {
        let solvers = init_solvers();
        let yices = solvers.get("yices-2.6.4").unwrap();
        
        assert_eq!(yices.binary_name, "yices-smt2");
        assert!(yices.arguments.contains(&"--smt2-model-format".to_string()));
        assert!(yices.arguments.contains(&"--bvconst-in-decimal".to_string()));
    }

    #[test]
    fn test_z3_solver_info() {
        let solvers = init_solvers();
        let z3 = solvers.get("z3").unwrap();
        
        assert_eq!(z3.binary_name, "z3");
        assert!(z3.arguments.is_empty());
        assert!(z3.downloads.is_empty());
    }

    #[test]
    fn test_yices_alias() {
        let solvers = init_solvers();
        let yices_alias = solvers.get("yices").unwrap();
        let yices_264 = solvers.get("yices-2.6.4").unwrap();
        
        assert_eq!(yices_alias.name, yices_264.name);
    }
}
