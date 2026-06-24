//! Typed BIG-IP attachment values — profiles, persistence.

use std::fmt;

/// One `profiles { ... }` list item on an ltm virtual.
///
/// `path` is the profile's full-path reference. `context` is one of
/// `"clientside"` / `"serverside"` / `"all"` / `""` (empty when omitted).
/// `raw` carries the original `{ ... }` body so a round trip keeps the
/// user's spacing when no changes are made.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ProfileAttachment {
    /// Profile full-path reference.
    pub path: String,
    /// Direction context: `clientside` / `serverside` / `all` / empty.
    pub context: String,
    /// Original brace-body text (stripped).
    pub raw: String,
}

impl ProfileAttachment {
    /// Bare leaf name of the referenced profile.
    #[must_use]
    pub fn name(&self) -> String {
        leaf(&self.path)
    }

    /// Alias for `path` (`PathRef` back-compat).
    #[must_use]
    pub fn full_path(&self) -> &str {
        &self.path
    }

    /// Build an attachment from the keyed-block parser output.
    #[must_use]
    pub fn from_raw(path: &str, body: &str) -> Self {
        let mut context = String::new();
        let tokens = iter_top_level_tokens(body);
        for (idx, tok) in tokens.iter().enumerate() {
            if tok == "context" && idx + 1 < tokens.len() {
                let candidate = &tokens[idx + 1];
                if matches!(candidate.as_str(), "clientside" | "serverside" | "all") {
                    candidate.clone_into(&mut context);
                    break;
                }
            }
        }
        Self {
            path: path.to_owned(),
            context,
            raw: body.trim().to_owned(),
        }
    }
}

impl fmt::Display for ProfileAttachment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.raw.is_empty()
            && ProfileAttachment::from_raw(&self.path, &self.raw).context == self.context
        {
            return write!(f, "{} {{ {} }}", self.path, self.raw);
        }
        if !self.context.is_empty() {
            return write!(f, "{} {{ context {} }}", self.path, self.context);
        }
        write!(f, "{} {{ }}", self.path)
    }
}

/// One `persist { ... }` list item on an ltm virtual.
///
/// `path` is the persistence profile's full-path reference. `default` is
/// the `default yes` flag. `raw` preserves the original body text.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct PersistenceAttachment {
    /// Persistence profile full-path reference.
    pub path: String,
    /// The `default yes` flag.
    pub default: bool,
    /// Original brace-body text (stripped).
    pub raw: String,
}

impl PersistenceAttachment {
    /// Bare leaf name of the referenced profile.
    #[must_use]
    pub fn name(&self) -> String {
        leaf(&self.path)
    }

    /// Alias for `path` (`PathRef` back-compat).
    #[must_use]
    pub fn full_path(&self) -> &str {
        &self.path
    }

    /// Build an attachment from the keyed-block parser output.
    #[must_use]
    pub fn from_raw(path: &str, body: &str) -> Self {
        let mut default = false;
        let tokens = iter_top_level_tokens(body);
        for (idx, tok) in tokens.iter().enumerate() {
            if tok == "default" && idx + 1 < tokens.len() {
                let value = tokens[idx + 1].to_lowercase();
                default = matches!(value.as_str(), "yes" | "true");
                break;
            }
        }
        Self {
            path: path.to_owned(),
            default,
            raw: body.trim().to_owned(),
        }
    }
}

impl fmt::Display for PersistenceAttachment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.raw.is_empty()
            && PersistenceAttachment::from_raw(&self.path, &self.raw).default == self.default
        {
            return write!(f, "{} {{ {} }}", self.path, self.raw);
        }
        if self.default {
            return write!(f, "{} {{ default yes }}", self.path);
        }
        write!(f, "{} {{ }}", self.path)
    }
}

/// Leaf name of a TMSH full-path: `/Common/clientssl` -> `clientssl`,
/// or the empty string for an empty path.
fn leaf(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    path.rsplit('/').next().unwrap_or(path).to_owned()
}

/// Return `body`'s top-level tokens, skipping nested `{ ... }`. `{` and
/// `}` are themselves emitted so callers can see brace markers.
pub(crate) fn iter_top_level_tokens(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    for tok in body.split_whitespace() {
        if tok == "{" {
            depth += 1;
            out.push(tok.to_owned());
            continue;
        }
        if tok == "}" {
            if depth > 0 {
                depth -= 1;
            }
            out.push(tok.to_owned());
            continue;
        }
        if depth > 0 {
            continue;
        }
        out.push(tok.to_owned());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_context() {
        let p = ProfileAttachment::from_raw("/Common/clientssl", "context clientside");
        assert_eq!(p.context, "clientside");
        assert_eq!(p.name(), "clientssl");
        assert_eq!(p.to_string(), "/Common/clientssl { context clientside }");

        let p = ProfileAttachment::from_raw("/Common/http", "");
        assert_eq!(p.context, "");
        assert_eq!(p.to_string(), "/Common/http { }");
    }

    #[test]
    fn profile_stale_raw_guard() {
        let p = ProfileAttachment {
            path: "/Common/clientssl".to_owned(),
            context: "serverside".to_owned(),
            raw: "context clientside".to_owned(),
        };
        // raw re-parses to clientside != serverside, so render structured.
        assert_eq!(p.to_string(), "/Common/clientssl { context serverside }");
    }

    #[test]
    fn persistence_default() {
        let p = PersistenceAttachment::from_raw("/Common/cookie", "default yes");
        assert!(p.default);
        assert_eq!(p.to_string(), "/Common/cookie { default yes }");

        let p = PersistenceAttachment::from_raw("/Common/source_addr", "");
        assert!(!p.default);
        assert_eq!(p.to_string(), "/Common/source_addr { }");
    }
}
