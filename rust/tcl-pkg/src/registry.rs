//! tcltk-pkgs registry client with TTL-based caching.
//!
//! Faithful port of `tooling/tclpkg/registry.py`. Fetches the upstream
//! `packages.json` discovery metadata, caches it under
//! `<cache_dir>/tclpkg/registry/`, and respects a 24-hour TTL with conditional
//! GET (`If-None-Match` / `304 Not Modified`). Offline mode reads the cache
//! only.

use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::errors::TclPkgError;

const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/tcltk-pkgs/registry/main/packages.json";

const TTL: Duration = Duration::from_hours(24);
const MAX_BODY: u64 = 64 * 1024 * 1024;

/// One source entry inside a registry package.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegistrySource {
    pub url: String,
    pub method: String,
    pub web: String,
    pub author: String,
    pub license: String,
    pub extension: bool,
}

/// One package entry from `packages.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegistryEntry {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub sources: Vec<RegistrySource>,
}

fn parse_entries(data: &Value) -> Vec<RegistryEntry> {
    let Some(items) = data.as_array() else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    for item in items {
        if !item.is_object() {
            continue;
        }
        let mut sources = Vec::new();
        if let Some(arr) = item.get("sources").and_then(Value::as_array) {
            for s in arr {
                if s.is_object() {
                    sources.push(RegistrySource {
                        url: str_field(s, "url"),
                        method: str_field(s, "method"),
                        web: str_field(s, "web"),
                        author: str_field(s, "author"),
                        license: str_field(s, "license"),
                        extension: s.get("extension").and_then(Value::as_bool).unwrap_or(false),
                    });
                }
            }
        }
        entries.push(RegistryEntry {
            name: str_field(item, "name"),
            description: str_field(item, "description"),
            tags: item
                .get("tags")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|t| t.as_str().map(ToString::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            sources,
        });
    }
    entries
}

fn str_field(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

/// Fetch, cache, and query the tcltk-pkgs package registry.
pub struct RegistryClient {
    dir: PathBuf,
    url: String,
    offline: bool,
    entries: Option<Vec<RegistryEntry>>,
}

impl RegistryClient {
    /// Construct a client rooted at `cache_dir`, optionally pinned to offline.
    #[must_use]
    pub fn new(cache_dir: &Path, offline: bool) -> Self {
        Self {
            dir: cache_dir.join("tclpkg").join("registry"),
            url: DEFAULT_REGISTRY_URL.to_string(),
            offline,
            entries: None,
        }
    }

    /// Override the registry URL (used in tests).
    #[must_use]
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    fn cached_path(&self) -> PathBuf {
        self.dir.join("packages.json")
    }
    fn etag_path(&self) -> PathBuf {
        self.dir.join("packages.json.etag")
    }
    fn fetched_path(&self) -> PathBuf {
        self.dir.join("fetched_at")
    }

    fn is_fresh(&self) -> bool {
        let Ok(text) = std::fs::read_to_string(self.fetched_path()) else {
            return false;
        };
        let Ok(ts) = DateTime::parse_from_rfc3339(text.trim()) else {
            return false;
        };
        let elapsed = Utc::now().signed_duration_since(ts.with_timezone(&Utc));
        elapsed.to_std().is_ok_and(|d| d < TTL)
    }

    fn load_cached(&self) -> Result<Vec<RegistryEntry>, TclPkgError> {
        let cached = self.cached_path();
        if !cached.is_file() {
            return Err(TclPkgError::registry(
                "no cached registry and offline mode is active",
                Some("run 'tcl pkg search' with network access first".to_string()),
            ));
        }
        let text = std::fs::read_to_string(&cached)
            .map_err(|e| TclPkgError::registry(format!("cannot read registry cache: {e}"), None))?;
        let data: Value = serde_json::from_str(&text)
            .map_err(|e| TclPkgError::registry(format!("corrupt registry cache: {e}"), None))?;
        Ok(parse_entries(&data))
    }

    fn write_cache(&self, body: &[u8], etag: &str) {
        let _ = std::fs::create_dir_all(&self.dir);
        let _ = std::fs::write(self.cached_path(), body);
        let _ = std::fs::write(self.etag_path(), etag);
        let _ = std::fs::write(self.fetched_path(), Utc::now().to_rfc3339());
    }

    fn touch_fetched(&self) {
        let _ = std::fs::create_dir_all(&self.dir);
        let _ = std::fs::write(self.fetched_path(), Utc::now().to_rfc3339());
    }

    /// Return all registry entries, fetching/caching as needed.
    pub fn fetch(&mut self, force: bool) -> Result<Vec<RegistryEntry>, TclPkgError> {
        if let Some(entries) = &self.entries
            && !force
        {
            return Ok(entries.clone());
        }

        if self.offline {
            let entries = self.load_cached()?;
            self.entries = Some(entries.clone());
            return Ok(entries);
        }

        let cached = self.cached_path();
        if !force && self.is_fresh() && cached.is_file() {
            let entries = self.load_cached()?;
            self.entries = Some(entries.clone());
            return Ok(entries);
        }

        let entries = self.network_fetch()?;
        self.entries = Some(entries.clone());
        Ok(entries)
    }

    fn network_fetch(&self) -> Result<Vec<RegistryEntry>, TclPkgError> {
        let cached = self.cached_path();
        let agent = ureq::config::Config::builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .new_agent();

        let mut builder = agent.get(&self.url).header("User-Agent", "tclpkg/0.1");
        if let Ok(etag) = std::fs::read_to_string(self.etag_path()) {
            builder = builder.header("If-None-Match", etag.trim());
        }

        match builder.call() {
            Ok(mut resp) => {
                let status = resp.status().as_u16();
                if status == 304 {
                    self.touch_fetched();
                    return self.load_cached();
                }
                if status >= 400 {
                    if cached.is_file() {
                        return self.load_cached();
                    }
                    return Err(TclPkgError::registry(
                        format!("registry fetch failed: HTTP {status}"),
                        None,
                    ));
                }
                let new_etag = resp
                    .headers()
                    .get("ETag")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                let body = resp
                    .body_mut()
                    .with_config()
                    .limit(MAX_BODY)
                    .read_to_vec()
                    .map_err(|e| {
                        TclPkgError::registry(format!("registry read failed: {e}"), None)
                    })?;
                self.write_cache(&body, &new_etag);
                let data: Value = serde_json::from_slice(&body).map_err(|e| {
                    TclPkgError::registry(format!("invalid registry JSON: {e}"), None)
                })?;
                Ok(parse_entries(&data))
            }
            Err(exc) => {
                if cached.is_file() {
                    return self.load_cached();
                }
                Err(TclPkgError::registry(
                    format!("cannot reach registry at {}: {exc}", self.url),
                    Some("check your network connection or use --offline".to_string()),
                ))
            }
        }
    }

    /// Search cached entries by name/description/tag (case-insensitive).
    pub fn search(&mut self, query: &str) -> Result<Vec<RegistryEntry>, TclPkgError> {
        let entries = self.fetch(false)?;
        let q = query.to_lowercase();
        Ok(entries
            .into_iter()
            .filter(|e| {
                e.name.to_lowercase().contains(&q)
                    || e.description.to_lowercase().contains(&q)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect())
    }

    /// Find an entry by exact package name.
    pub fn lookup(&mut self, name: &str) -> Result<Option<RegistryEntry>, TclPkgError> {
        let entries = self.fetch(false)?;
        Ok(entries.into_iter().find(|e| e.name == name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries() {
        let data: Value = serde_json::from_str(
            r#"[
                {"name": "json", "description": "JSON for Tcl", "tags": ["data"],
                 "sources": [{"url": "https://x/json.git", "method": "git"}]},
                "not an object",
                {"name": "http", "description": "HTTP", "tags": []}
            ]"#,
        )
        .unwrap();
        let entries = parse_entries(&data);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "json");
        assert_eq!(entries[0].sources[0].method, "git");
        assert_eq!(entries[1].name, "http");
    }

    #[test]
    fn offline_without_cache_errors() {
        let dir = std::env::temp_dir().join(format!(
            "tclpkg-reg-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut client = RegistryClient::new(&dir, true);
        let err = client.fetch(false).unwrap_err();
        assert!(err.to_string().contains("offline"));
    }

    #[test]
    fn search_offline_cache() {
        let dir = std::env::temp_dir().join(format!(
            "tclpkg-reg2-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let reg_dir = dir.join("tclpkg").join("registry");
        std::fs::create_dir_all(&reg_dir).unwrap();
        std::fs::write(
            reg_dir.join("packages.json"),
            r#"[{"name": "json", "description": "JSON parser", "tags": ["data"]}]"#,
        )
        .unwrap();
        let mut client = RegistryClient::new(&dir, true);
        let results = client.search("json").unwrap();
        assert_eq!(results.len(), 1);
        assert!(client.search("nomatch").unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
