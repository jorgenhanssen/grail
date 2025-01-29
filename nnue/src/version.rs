use std::fs;
use std::io;
use std::path::PathBuf;

pub struct VersionManager {
    root_path: PathBuf,
}

impl VersionManager {
    pub fn new() -> io::Result<Self> {
        let root_path = PathBuf::from("nnue/versions");
        fs::create_dir_all(&root_path)?;
        Ok(Self { root_path })
    }

    pub fn get_all_versions(&self) -> io::Result<Vec<u32>> {
        let mut versions = Vec::new();
        for entry in fs::read_dir(&self.root_path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Some(stripped) = name.strip_prefix('v') {
                    if let Ok(num) = stripped.parse::<u32>() {
                        versions.push(num);
                    }
                }
            }
        }
        versions.sort_unstable();
        Ok(versions)
    }

    pub fn get_latest_version(&self) -> io::Result<Option<u32>> {
        let versions = self.get_all_versions()?;
        Ok(versions.last().cloned())
    }

    pub fn version_exists(&self, version: u32) -> bool {
        self.version_path(version).exists()
    }

    pub fn create_version(&self, version: u32) -> io::Result<()> {
        let path = self.version_path(version);
        if path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("Version v{} already exists", version),
            ));
        }
        fs::create_dir_all(&path)?;
        Ok(())
    }

    pub fn create_next_version(&self) -> io::Result<u32> {
        let next = match self.get_latest_version()? {
            Some(latest) => latest + 1,
            None => 0,
        };
        self.create_version(next)?;
        Ok(next)
    }

    pub fn version_path(&self, version: u32) -> PathBuf {
        self.root_path.join(format!("v{}", version))
    }

    pub fn file_path(&self, version: u32, filename: &str) -> PathBuf {
        self.version_path(version).join(filename)
    }
}
