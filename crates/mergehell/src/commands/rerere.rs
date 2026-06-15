use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RerereCache {
    root: PathBuf,
}

impl RerereCache {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn store(&self, conflict: &str, resolution: &str) -> io::Result<PathBuf> {
        fs::create_dir_all(&self.root)?;
        let path = self.path_for(conflict);
        fs::write(&path, resolution)?;
        Ok(path)
    }

    pub fn lookup(&self, conflict: &str) -> io::Result<Option<String>> {
        let path = self.path_for(conflict);
        match fs::read_to_string(path) {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn path_for(&self, conflict: &str) -> PathBuf {
        self.root
            .join(format!("{:016x}.resolution", stable_hash(conflict)))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn stable_hash(value: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_cache() -> RerereCache {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        RerereCache::new(std::env::temp_dir().join(format!("mergehell_rerere_{unique}")))
    }

    #[test]
    fn stores_and_reuses_resolution() {
        let cache = temp_cache();
        let path = cache.store("conflict", "resolution").unwrap();

        assert!(path.starts_with(cache.root()));
        assert_eq!(
            cache.lookup("conflict").unwrap(),
            Some("resolution".to_string())
        );
    }

    #[test]
    fn lookup_returns_none_when_missing() {
        let cache = temp_cache();

        assert_eq!(cache.lookup("missing").unwrap(), None);
    }

    #[test]
    fn path_for_same_conflict_is_stable() {
        let cache = temp_cache();

        assert_eq!(cache.path_for("same"), cache.path_for("same"));
        assert_ne!(cache.path_for("same"), cache.path_for("different"));
    }
}
