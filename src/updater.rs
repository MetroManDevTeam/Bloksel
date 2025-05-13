use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration
};
use tokio::{
    fs, io,
    time::timeout
};
use reqwest::Client;
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionManifest {
    pub version: String,
    pub exe_url: String,
    pub exe_size: u64,
    pub exe_sha256: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Hash mismatch")]
    HashMismatch,
    #[error("Size mismatch (expected {expected}, got {actual})")]
    SizeMismatch { expected: u64, actual: u64 },
    #[error("Update timeout")]
    Timeout,
    #[error("Update required")]
    UpdateRequired,
}

pub struct GameUpdater {
    client: Client,
    base_url: String,
    install_dir: PathBuf,
    current_exe: PathBuf,
}

impl GameUpdater {
    /// Creates a new updater instance
    pub fn new(base_url: impl Into<String>, install_dir: impl AsRef<Path>) -> Result<Self, UpdateError> {
        Ok(Self {
            client: Client::new(),
            base_url: base_url.into(),
            install_dir: install_dir.as_ref().to_path_buf(),
            current_exe: std::env::current_exe()?,
        })
    }

    /// Checks for updates and installs them if available
    pub async fn run(&self) -> Result<bool, UpdateError> {
        let local_ver = self.get_local_version().await.unwrap_or("0.0.0".into());
        let remote = self.fetch_manifest().await?;

        if local_ver == remote.version {
            return Ok(false); // No update needed
        }

        if remote.required {
            return Err(UpdateError::UpdateRequired);
        }

        self.install_update(&remote).await?;
        Ok(true)
    }

    async fn get_local_version(&self) -> Option<String> {
        fs::read_to_string(self.install_dir.join("version.json"))
            .await.ok()
            .and_then(|v| serde_json::from_str::<VersionManifest>(&v).ok())
            .map(|m| m.version)
    }

    async fn fetch_manifest(&self) -> Result<VersionManifest, UpdateError> {
        let url = format!("{}/version.json", self.base_url);
        timeout(
            Duration::from_secs(10),
            self.client.get(&url).send()
        )
        .await??
        .json()
        .await
    }

    async fn install_update(&self, manifest: &VersionManifest) -> Result<(), UpdateError> {
        // 1. Download to temp file
        let temp_path = self.install_dir.join("game_update.tmp");
        self.download_verified(&manifest.exe_url, &temp_path, manifest.exe_sha256.as_str(), manifest.exe_size).await?;

        // 2. Install
        #[cfg(windows)]
        self.install_windows(&temp_path).await?;

        #[cfg(not(windows))]
        self.install_unix(&temp_path).await?;

        // 3. Update version file
        fs::write(
            self.install_dir.join("version.json"),
            serde_json::to_string_pretty(manifest)?
        ).await?;

        Ok(())
    }

    async fn download_verified(
        &self,
        url: &str,
        dest: &Path,
        expected_hash: &str,
        expected_size: u64
    ) -> Result<(), UpdateError> {
        // Download
        let mut response = timeout(
            Duration::from_secs(60),
            self.client.get(url).send()
        ).await??;

        // Check size
        if let Some(len) = response.content_length() {
            if len != expected_size {
                return Err(UpdateError::SizeMismatch {
                    expected: expected_size,
                    actual: len
                });
            }
        }

        // Stream to file while hashing
        let mut file = fs::File::create(dest).await?;
        let mut hasher = Sha256::new();
        let mut downloaded = 0;

        while let Some(chunk) = timeout(Duration::from_secs(30), response.chunk()).await? {
            let chunk = chunk?;
            hasher.update(&chunk);
            io::copy(&mut chunk.as_ref(), &mut file).await?;
            downloaded += chunk.len() as u64;
        }

        // Verify hash
        if format!("{:x}", hasher.finalize()) != expected_hash {
            fs::remove_file(dest).await.ok();
            return Err(UpdateError::HashMismatch);
        }

        Ok(())
    }

    #[cfg(windows)]
    async fn install_windows(&self, new_exe: &Path) -> Result<(), UpdateError> {
        let script = format!(
            r#"
            @echo off
            timeout /t 1 /nobreak >nul
            del "{}"
            rename "{}" "{}"
            start "" "{}"
            del "%~f0"
            "#,
            self.current_exe.display(),
            new_exe.display(),
            self.current_exe.file_name().unwrap().to_string_lossy(),
            self.current_exe.display()
        );

        let script_path = self.install_dir.join("update.bat");
        fs::write(&script_path, script).await?;

        Command::new("cmd")
            .args(["/C", &script_path.to_string_lossy()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        Ok(())
    }

    #[cfg(not(windows))]
    async fn install_unix(&self, new_exe: &Path) -> Result<(), UpdateError> {
        fs::rename(new_exe, &self.current_exe).await?;
        Ok(())
    }
}

    /* USAGE

// In your main.rs or wherever needed:
mod updater;

#[tokio::main]
async fn main() {
    let updater = updater::GameUpdater::new(
        "https://your-site.com/updates",
        std::env::current_dir().unwrap()
    ).unwrap();

    match updater.run().await {
        Ok(updated) if updated => {
            println!("Restart to apply update!");
            std::process::exit(0);
        }
        Err(e) => eprintln!("Update failed: {}", e),
        _ => {} // No update needed
    }

    // Run your game...
}

*/
