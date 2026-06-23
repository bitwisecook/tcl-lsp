//! Generic typed list value for BIG-IP list-shaped properties.
//!
//! `BigipList` carries items as [`ListItem`] records, each pairing the
//! typed item value with its lexical metadata (`key` / `body` / `raw` /
//! source ranges). [`ListSyntax`] discriminates the four list shapes.

use super::attachments::{PersistenceAttachment, ProfileAttachment};
use super::cert_key_chain::CertKeyChain;
use super::data_group_record::DataGroupRecord;
use super::firewall_rule::FirewallRule;
use super::gtm_region_member::GtmRegionMember;
use super::monitor_expression::MonitorExpression;

/// Half-open byte span paired with absolute offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    /// Start byte offset (inclusive).
    pub start: usize,
    /// End byte offset (exclusive — half-open).
    pub end: usize,
}

impl SourceSpan {
    /// Construct a span from start/end byte offsets.
    #[must_use]
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// The lexical shape of a [`BigipList`]. [`ListSyntax::as_str`] returns the
/// Python string literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListSyntax {
    /// Unbraced scalar list (`ip-protocol icmp tcp`).
    SpaceSeparated,
    /// `{ /Common/a /Common/b }`.
    BracedSpaceSeparated,
    /// `{ /Common/clientssl { context clientside } ... }`.
    KeyedBlock,
    /// `{ name { action accept ... } }`.
    NamedRuleBlock,
    /// `{ replace-all-with { /Common/a } }`.
    OperatorPrefixed,
}

impl ListSyntax {
    /// The Python string literal for this syntax.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ListSyntax::SpaceSeparated => "space-separated",
            ListSyntax::BracedSpaceSeparated => "braced-space-separated",
            ListSyntax::KeyedBlock => "keyed-block",
            ListSyntax::NamedRuleBlock => "named-rule-block",
            ListSyntax::OperatorPrefixed => "operator-prefixed",
        }
    }
}

/// The heterogeneous value of a [`ListItem`] — a plain string for scalar
/// lists, or one of the typed attachment / record values for the
/// structured shapes, as a value union.
#[derive(Debug, Clone, PartialEq)]
pub enum ListItemValue {
    /// A plain string path or scalar.
    Str(String),
    /// A profile attachment.
    Profile(ProfileAttachment),
    /// A persistence attachment.
    Persistence(PersistenceAttachment),
    /// A cert-key-chain entry.
    CertKeyChain(CertKeyChain),
    /// A data-group record.
    DataGroupRecord(DataGroupRecord),
    /// A GTM region member.
    GtmRegionMember(GtmRegionMember),
    /// A firewall (or NAT) rule.
    FirewallRule(FirewallRule),
    /// A monitor expression.
    MonitorExpression(MonitorExpression),
    /// A typed pool member (`ltm pool`/`snatpool` members) — an intra-crate
    /// reference to the model struct (Rust allows the module cycle).
    PoolMember(crate::model::BigipPoolMember),
}

impl ListItemValue {
    /// The path this value references: typed values with a `full_path` /
    /// `path` attribute return it; plain strings return themselves
    /// verbatim. `None` when there is no path-like attribute to surface.
    #[must_use]
    fn path(&self) -> Option<String> {
        match self {
            ListItemValue::Str(s) => Some(s.clone()),
            ListItemValue::Profile(p) if !p.full_path().is_empty() => Some(p.path.clone()),
            ListItemValue::Persistence(p) if !p.full_path().is_empty() => Some(p.path.clone()),
            ListItemValue::FirewallRule(_)
            | ListItemValue::CertKeyChain(_)
            | ListItemValue::DataGroupRecord(_)
            | ListItemValue::GtmRegionMember(_)
            | ListItemValue::MonitorExpression(_)
            | ListItemValue::PoolMember(_)
            | ListItemValue::Profile(_)
            | ListItemValue::Persistence(_) => None,
        }
    }
}

impl From<String> for ListItemValue {
    fn from(s: String) -> Self {
        ListItemValue::Str(s)
    }
}

impl From<&str> for ListItemValue {
    fn from(s: &str) -> Self {
        ListItemValue::Str(s.to_owned())
    }
}

/// One element of a [`BigipList`].
///
/// `key` and `body` carry the lexical halves of a keyed-block item; `raw`
/// is the per-item source text. The three range fields locate the item
/// span, key token, and body span; each is `None` when not available.
#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    /// The typed item value.
    pub value: ListItemValue,
    /// Key (path or rule name) preceding `{ ... }`; empty for non-keyed.
    pub key: String,
    /// Brace-body content (without braces); empty for non-keyed.
    pub body: String,
    /// Per-item source text.
    pub raw: String,
    /// Span of the whole item.
    pub range: Option<SourceSpan>,
    /// Span of the key token.
    pub key_range: Option<SourceSpan>,
    /// Span of the body.
    pub body_range: Option<SourceSpan>,
}

impl ListItem {
    /// Construct a bare list item from a value, leaving lexical metadata
    /// at its defaults.
    #[must_use]
    pub fn new(value: impl Into<ListItemValue>) -> Self {
        Self {
            value: value.into(),
            key: String::new(),
            body: String::new(),
            raw: String::new(),
            range: None,
            key_range: None,
            body_range: None,
        }
    }

    /// The leaf name of `key`: `/Common/clientssl` -> `clientssl`, the key
    /// verbatim when it has no `/`, or empty when `key` is empty.
    #[must_use]
    pub fn name(&self) -> String {
        if self.key.is_empty() {
            return String::new();
        }
        if self.key.contains('/') {
            return self.key.rsplit('/').next().unwrap_or(&self.key).to_owned();
        }
        self.key.clone()
    }
}

/// A list-shaped BIG-IP property value.
///
/// `items` is the typed per-element view. `syntax` discriminates the
/// lexical shape. `operator` carries the tmsh edit verb when present.
/// `raw` preserves the original brace body; `range` is the absolute span.
#[derive(Debug, Clone, PartialEq)]
pub struct BigipList {
    /// The typed item records.
    pub items: Vec<ListItem>,
    /// The lexical shape.
    pub syntax: ListSyntax,
    /// The tmsh edit verb (`add` / `replace-all-with` / ...), if any.
    pub operator: Option<String>,
    /// Original brace body (without the surrounding braces).
    pub raw: String,
    /// Absolute span of the list value.
    pub range: Option<SourceSpan>,
}

impl Default for BigipList {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            syntax: ListSyntax::BracedSpaceSeparated,
            operator: None,
            raw: String::new(),
            range: None,
        }
    }
}

impl BigipList {
    /// Construct a list from items + syntax, leaving the other fields at
    /// their defaults.
    #[must_use]
    pub fn new(items: Vec<ListItem>, syntax: ListSyntax) -> Self {
        Self {
            items,
            syntax,
            ..Default::default()
        }
    }

    /// Number of items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// `true` when the list has no items (Python `__bool__` is the inverse).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Iterate the typed item values.
    pub fn iter(&self) -> impl Iterator<Item = &ListItemValue> {
        self.items.iter().map(|item| &item.value)
    }

    /// The typed item values as an owned vector.
    #[must_use]
    pub fn values(&self) -> Vec<ListItemValue> {
        self.items.iter().map(|item| item.value.clone()).collect()
    }

    /// The string paths the items reference.
    ///
    /// Typed values with a `full_path` / `path` return it; plain strings
    /// return themselves; otherwise the item `key` is used, falling back
    /// to the empty string.
    #[must_use]
    pub fn paths(&self) -> Vec<String> {
        let mut out = Vec::with_capacity(self.items.len());
        for item in &self.items {
            if let Some(p) = item.value.path() {
                out.push(p);
                continue;
            }
            if !item.key.is_empty() {
                out.push(item.key.clone());
                continue;
            }
            out.push(String::new());
        }
        out
    }

    /// `true` when `needle` matches an item's path (Python `__contains__`
    /// for the string case).
    #[must_use]
    pub fn contains_path(&self, needle: &str) -> bool {
        self.paths().iter().any(|p| p == needle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_as_str() {
        assert_eq!(ListSyntax::KeyedBlock.as_str(), "keyed-block");
        assert_eq!(
            ListSyntax::BracedSpaceSeparated.as_str(),
            "braced-space-separated"
        );
        assert_eq!(ListSyntax::OperatorPrefixed.as_str(), "operator-prefixed");
    }

    #[test]
    fn list_item_name() {
        let mut item = ListItem::new("/Common/clientssl");
        item.key = "/Common/clientssl".to_owned();
        assert_eq!(item.name(), "clientssl");
        item.key = "plain_rule".to_owned();
        assert_eq!(item.name(), "plain_rule");
        item.key = String::new();
        assert_eq!(item.name(), "");
    }

    #[test]
    fn paths_for_plain_strings() {
        let list = BigipList::new(
            vec![ListItem::new("/Common/a"), ListItem::new("/Common/b")],
            ListSyntax::BracedSpaceSeparated,
        );
        assert_eq!(
            list.paths(),
            vec!["/Common/a".to_owned(), "/Common/b".to_owned()]
        );
        assert!(list.contains_path("/Common/a"));
        assert!(!list.contains_path("/Common/z"));
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn paths_for_profile_attachments() {
        let mut item = ListItem::new(ListItemValue::Profile(ProfileAttachment::from_raw(
            "/Common/clientssl",
            "context clientside",
        )));
        item.key = "/Common/clientssl".to_owned();
        let list = BigipList::new(vec![item], ListSyntax::KeyedBlock);
        assert_eq!(list.paths(), vec!["/Common/clientssl".to_owned()]);
    }

    #[test]
    fn paths_falls_back_to_key() {
        let mut item = ListItem::new(ListItemValue::FirewallRule(FirewallRule::from_raw(
            "allow",
            "action accept",
        )));
        item.key = "allow".to_owned();
        let list = BigipList::new(vec![item], ListSyntax::NamedRuleBlock);
        assert_eq!(list.paths(), vec!["allow".to_owned()]);
    }
}
