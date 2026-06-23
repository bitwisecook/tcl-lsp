//! `Partition` value type — F5 administrative partitions.

use super::error::ValueError;
use super::port::py_repr;
use std::fmt;

/// `true` when `c` is a valid partition / folder-segment character:
/// alphanumerics, underscore, hyphen, and dot. Slashes are excluded
/// (they are the path separator).
pub(crate) fn is_partition_valid_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')
}

/// An F5 administrative partition (`/Common`, `/Tenant_A`, ...).
///
/// Stored canonically with the leading `/` so [`Display`](fmt::Display)
/// matches the F5 spelling everywhere. Parsing accepts both
/// leading-slash (`/Common`) and bare (`Common`) input.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Partition {
    /// Canonical name: leading slash + the partition name.
    pub name: String,
}

impl Partition {
    /// Parse `text` as a partition name.
    ///
    /// Accepts `"/Common"`, `"Common"`, `"/Tenant_A"`.
    ///
    /// # Errors
    /// Empty, only-slashes, multi-segment, or invalid-character input
    /// returns [`ValueError`].
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(ValueError("Partition: empty input".to_owned()));
        }
        let bare = text.trim_start_matches('/');
        if bare.is_empty() {
            return Err(ValueError(format!(
                "Partition: only slashes ({})",
                py_repr(text)
            )));
        }
        if bare.contains('/') {
            return Err(ValueError(format!(
                "Partition: nested path not allowed ({}); partitions are flat",
                py_repr(text)
            )));
        }
        if !bare.chars().all(is_partition_valid_char) {
            return Err(ValueError(format!(
                "Partition: invalid character in {}",
                py_repr(text)
            )));
        }
        Ok(Self {
            name: format!("/{bare}"),
        })
    }

    /// Like [`Partition::parse`] but returns `None` instead of erroring.
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        Self::parse(text).ok()
    }

    /// The partition name without the leading slash.
    #[must_use]
    pub fn short_name(&self) -> &str {
        self.name.trim_start_matches('/')
    }

    /// `true` for the system-default `/Common` partition.
    #[must_use]
    pub fn is_common(&self) -> bool {
        self.short_name() == "Common"
    }

    /// `true` when an object in `self` may reference an object in `other`.
    ///
    /// Same partition is always visible; `/Common` is visible to every
    /// partition; any other cross-partition pairing is not visible.
    #[must_use]
    pub fn can_see(&self, other: &Partition) -> bool {
        if self == other {
            return true;
        }
        other.is_common()
    }
}

impl fmt::Display for Partition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_with_and_without_slash() {
        assert_eq!(Partition::parse("/Common").unwrap().name, "/Common");
        assert_eq!(Partition::parse("Common").unwrap().name, "/Common");
        assert_eq!(Partition::parse("/Tenant_A").unwrap().name, "/Tenant_A");
        assert_eq!(
            Partition::parse("  /Common  ").unwrap().to_string(),
            "/Common"
        );
    }

    #[test]
    fn predicates() {
        let common = Partition::parse("/Common").unwrap();
        let tenant = Partition::parse("/Tenant_A").unwrap();
        assert!(common.is_common());
        assert!(!tenant.is_common());
        assert_eq!(common.short_name(), "Common");
        assert!(tenant.can_see(&common));
        assert!(!common.can_see(&tenant));
        assert!(common.can_see(&common));
    }

    #[test]
    fn rejects_bad() {
        assert_eq!(
            Partition::parse("").unwrap_err().0,
            "Partition: empty input"
        );
        assert_eq!(
            Partition::parse("/").unwrap_err().0,
            "Partition: only slashes ('/')"
        );
        assert_eq!(
            Partition::parse("/Common/x").unwrap_err().0,
            "Partition: nested path not allowed ('/Common/x'); partitions are flat"
        );
        assert_eq!(
            Partition::parse("bad name").unwrap_err().0,
            "Partition: invalid character in 'bad name'"
        );
    }
}
