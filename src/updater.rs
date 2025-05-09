use std::path::{Path, PathBuf};
use reqwest::Client;
use tokio::{fs, io};
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct GameManifest {
    version: String,
    files: Vec<GameFile>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GameFile {
    path: String,
    sha256: String,
    size: u64,
}

pub struct GameUpdater {
    client: Client,
    repo: String,
    branch: String,
    game_dir: PathBuf,
}

impl GameUpdater {
    pub fn new(repo: String, branch: String, game_dir: PathBuf) -> Self {
        Self {
            client: Client::new(),
            repo,
            branch,
            game_dir,
        }
    }

    pub async fn detect_changes(&self) -> Result<Vec<String>, String> {
        let manifest = match self.fetch_manifest().await {
            Ok(m) => m,
            Err(e) => return Err(format!("Failed to fetch manifest: {}", e)),
        };

        let mut changed_files = Vec::new();

        for file in manifest.files {
            let file_path = self.game_dir.join(&file.path);
            
            // Skip protected files
            if self.is_protected(&file.path) {
                continue;
            }

            // Check if file exists and matches hash
            let needs_update = match fs::read(&file_path).await {
                Ok(content) => {
                    let local_hash = format!("{:x}", Sha256::digest(&content));
                    local_hash != file.sha256
                },
                Err(_) => true, // File doesn't exist
            };

            if needs_update {
                changed_files.push(file.path);
            }
        }

        Ok(changed_files)
    }

    pub async fn update_game(&self, files_to_update: &[String]) -> Result<usize, String> {
        let mut updated_count = 0;

        for file_path in files_to_update {
            if self.is_protected(file_path) {
                continue;
            }

            let download_url = format!(
                "https://raw.githubusercontent.com/{}/{}/{}",
                self.repo, self.branch, file_path
            );

            let dest_path = self.game_dir.join(file_path);
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).await
                    .map_err(|e| format!("Failed to create directories: {}", e))?;
            }

            // Download file
            let content = self.client.get(&download_url)
                .send().await
                .map_err(|e| format!("Download failed: {}", e))?
                .bytes().await
                .map_err(|e| format!("Failed to read response: {}", e))?;

            // Verify hash before writing
            let downloaded_hash = format!("{:x}", Sha256::digest(&content));
            let expected_hash = self.get_expected_hash(file_path).await?;
            
            if downloaded_hash != expected_hash {
                return Err(format!("Hash mismatch for {}", file_path));
            }

            fs::write(&dest_path, &content).await
                .map_err(|e| format!("Failed to write file: {}", e))?;

            updated_count += 1;
        }

        Ok(updated_count)
    }

    async fn fetch_manifest(&self) -> Result<GameManifest, String> {
        let url = format!(
            "https://raw.githubusercontent.com/{}/{}/manifest.json",
            self.repo, self.branch
        );

        let response = self.client.get(&url)
            .send().await
            .map_err(|e| format!("Request failed: {}", e))?;

        response.json().await
            .map_err(|e| format!("JSON parse failed: {}", e))
    }

    async fn get_expected_hash(&self, file_path: &str) -> Result<String, String> {
        let manifest = self.fetch_manifest().await?;
        manifest.files.iter()
            .find(|f| f.path == file_path)
            .map(|f| f.sha256.clone())
            .ok_or_else(|| format!("File not in manifest: {}", file_path))
    }

    fn is_protected(&self, path: &str) -> bool {
        let protected = [
            "user_data/",
            "config/settings.toml",
            "saves/",
            "custom_",
        ];
        protected.iter().any(|p| path.starts_with(p))
    }
}



/*
USAGE EXAMPLE 

#[tokio::main]
async fn main() {
    let updater = GameUpdater::new(
        "your_github/repo".to_string(),
        "main".to_string(),
        PathBuf::from("./game_files")
    );

    // Detect what needs updating
    match updater.detect_changes().await {
        Ok(changed) => {
            if changed.is_empty() {
                println!("Game is up-to-date!");
                return;
            }
            
            println!("Files to update: {:?}", changed);
            
            // Perform the update
            match updater.update_game(&changed).await {
                Ok(count) => println!("Successfully updated {} files", count),
                Err(e) => eprintln!("Update failed: {}", e),
            }
        },
        Err(e) => eprintln!("Error detecting changes: {}", e),
    }
} 
*/
