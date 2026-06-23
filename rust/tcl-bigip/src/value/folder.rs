//! `Folder` + `ObjectPath` value types — F5 admin folders and full
//! object paths.

use super::error::ValueError;
use super::partition::{Partition, is_partition_valid_char};
use super::port::py_repr;
use std::fmt;

/// Folder segments share the partition's valid character set.
fn is_folder_segment_valid_char(c: char) -> bool {
    is_partition_valid_char(c)
}

/// Split on `/`, dropping empty segments — mirrors Python
/// `text.strip("/").split("/")` filtered for truthiness.
fn split_path_segments(text: &str) -> Vec<&str> {
    text.split('/').filter(|p| !p.is_empty()).collect()
}

/// An F5 admin folder path: a partition + zero or more nested segments.
///
/// Stored canonically with the leading slash and all segments joined with
/// `/`. `Folder("/Common")` is the partition root with no folder nesting;
/// `Folder("/Common/Application_X")` is the `Application_X` folder under
/// `/Common`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Folder {
    /// The leading partition segment.
    pub partition: Partition,
    /// Zero or more nested folder segments below the partition.
    pub segments: Vec<String>,
}

impl Folder {
    /// Construct a folder directly from a partition and its segments.
    #[must_use]
    pub fn new(partition: Partition, segments: Vec<String>) -> Self {
        Self {
            partition,
            segments,
        }
    }

    /// Parse `text` as a folder path.
    ///
    /// Accepts `"/Common"` (partition only), `"/Common/Application_X"`,
    /// and arbitrarily-deep nesting. Trailing slashes are tolerated.
    ///
    /// # Errors
    /// Empty input, input without a leading `/`, only-slashes input, and
    /// invalid folder-segment characters return [`ValueError`].
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(ValueError("Folder: empty input".to_owned()));
        }
        if !text.starts_with('/') {
            return Err(ValueError(format!(
                "Folder: must start with '/' (partition prefix), got {}",
                py_repr(text)
            )));
        }
        let parts = split_path_segments(text);
        if parts.is_empty() {
            return Err(ValueError(format!(
                "Folder: only slashes ({})",
                py_repr(text)
            )));
        }
        let partition = Partition::parse(parts[0])?;
        let segments: Vec<String> = parts[1..].iter().map(|s| (*s).to_owned()).collect();
        for seg in &segments {
            if !seg.chars().all(is_folder_segment_valid_char) {
                return Err(ValueError(format!(
                    "Folder: invalid character in folder segment {}",
                    py_repr(seg)
                )));
            }
        }
        Ok(Self {
            partition,
            segments,
        })
    }

    /// Like [`Folder::parse`] but returns `None` instead of erroring.
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        Self::parse(text).ok()
    }

    /// `true` when this folder is the partition root (no nesting).
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }

    /// Number of folder segments below the partition (0 for root).
    #[must_use]
    pub fn depth(&self) -> usize {
        self.segments.len()
    }

    /// Return a new [`Folder`] with `segment` appended.
    ///
    /// # Errors
    /// Returns [`ValueError`] when `segment` contains invalid characters.
    pub fn child(&self, segment: &str) -> Result<Self, ValueError> {
        if !segment.chars().all(is_folder_segment_valid_char) {
            return Err(ValueError(format!(
                "Folder.child: invalid segment {}",
                py_repr(segment)
            )));
        }
        let mut segments = self.segments.clone();
        segments.push(segment.to_owned());
        Ok(Self {
            partition: self.partition.clone(),
            segments,
        })
    }
}

impl fmt::Display for Folder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_root() {
            return write!(f, "{}", self.partition);
        }
        write!(f, "{}/{}", self.partition, self.segments.join("/"))
    }
}

/// A full BIG-IP object path: `<folder>/<leaf>`.
///
/// The leaf is the object's name (`web_pool`, `vs1`, ...) and can contain
/// the same characters folder segments do. Together the folder + leaf
/// form what the rest of the codebase calls the object's `full_path`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectPath {
    /// The containing folder.
    pub folder: Folder,
    /// The leaf object name.
    pub name: String,
}

impl ObjectPath {
    /// Parse `text` as a full BIG-IP object path.
    ///
    /// # Errors
    /// Empty input, input without a leading `/`, fewer than two segments,
    /// or invalid leaf characters return [`ValueError`].
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(ValueError("ObjectPath: empty input".to_owned()));
        }
        if !text.starts_with('/') {
            return Err(ValueError(format!(
                "ObjectPath: must start with '/' (partition prefix), got {}",
                py_repr(text)
            )));
        }
        let parts = split_path_segments(text);
        if parts.len() < 2 {
            return Err(ValueError(format!(
                "ObjectPath: must have at least partition + leaf, got {}",
                py_repr(text)
            )));
        }
        let leaf = parts[parts.len() - 1];
        if !leaf.chars().all(is_folder_segment_valid_char) {
            return Err(ValueError(format!(
                "ObjectPath: invalid character in leaf {}",
                py_repr(leaf)
            )));
        }
        let folder_text = format!("/{}", parts[..parts.len() - 1].join("/"));
        Ok(Self {
            folder: Folder::parse(&folder_text)?,
            name: leaf.to_owned(),
        })
    }

    /// Like [`ObjectPath::parse`] but returns `None` instead of erroring.
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        Self::parse(text).ok()
    }

    /// The partition portion of the folder.
    #[must_use]
    pub fn partition(&self) -> &Partition {
        &self.folder.partition
    }

    /// `true` when the object lives at the partition root (no folders).
    #[must_use]
    pub fn in_root_folder(&self) -> bool {
        self.folder.is_root()
    }
}

impl fmt::Display for ObjectPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.folder.is_root() {
            write!(f, "{}/{}", self.folder.partition, self.name)
        } else {
            write!(f, "{}/{}", self.folder, self.name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_parse_and_display() {
        let f = Folder::parse("/Common").unwrap();
        assert!(f.is_root());
        assert_eq!(f.to_string(), "/Common");
        assert_eq!(f.depth(), 0);

        let f = Folder::parse("/Common/Application_X").unwrap();
        assert!(!f.is_root());
        assert_eq!(f.segments, vec!["Application_X".to_owned()]);
        assert_eq!(f.to_string(), "/Common/Application_X");

        let f = Folder::parse("/Common/iApps/Tenant.app/").unwrap();
        assert_eq!(f.to_string(), "/Common/iApps/Tenant.app");
        assert_eq!(f.depth(), 2);
    }

    #[test]
    fn folder_child() {
        let f = Folder::parse("/Common").unwrap().child("App").unwrap();
        assert_eq!(f.to_string(), "/Common/App");
        assert!(f.child("bad/seg").is_err());
    }

    #[test]
    fn folder_rejects() {
        assert_eq!(Folder::parse("").unwrap_err().0, "Folder: empty input");
        assert_eq!(
            Folder::parse("Common").unwrap_err().0,
            "Folder: must start with '/' (partition prefix), got 'Common'"
        );
    }

    #[test]
    fn object_path_parse_and_display() {
        let p = ObjectPath::parse("/Common/web_pool").unwrap();
        assert!(p.in_root_folder());
        assert_eq!(p.name, "web_pool");
        assert_eq!(p.partition().name, "/Common");
        assert_eq!(p.to_string(), "/Common/web_pool");

        let p = ObjectPath::parse("/Common/Application_X/web_pool").unwrap();
        assert!(!p.in_root_folder());
        assert_eq!(p.to_string(), "/Common/Application_X/web_pool");

        let p = ObjectPath::parse("/Common/iApps/Tenant.app/pool_1").unwrap();
        assert_eq!(p.folder.to_string(), "/Common/iApps/Tenant.app");
        assert_eq!(p.name, "pool_1");
        assert_eq!(p.to_string(), "/Common/iApps/Tenant.app/pool_1");
    }

    #[test]
    fn object_path_rejects() {
        assert_eq!(
            ObjectPath::parse("/Common").unwrap_err().0,
            "ObjectPath: must have at least partition + leaf, got '/Common'"
        );
    }
}
