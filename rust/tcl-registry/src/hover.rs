//! Documentation and completion metadata for LSP features.

use crate::dialects::DialectSet;

/// Short hover content derived from man pages or vendor docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoverSnippet {
    /// One-line summary.
    pub summary: &'static str,
    /// Invocation synopsis lines (e.g. `"for start test next body"`).
    pub synopsis: &'static [&'static str],
    /// Extended description.
    pub snippet: &'static str,
    /// Documentation source (e.g. `"Tcl for(1)"`).
    pub source: &'static str,
    /// Usage examples.
    pub examples: &'static str,
    /// Return value description.
    pub return_value: &'static str,
}

impl HoverSnippet {
    /// A hover with only summary, synopsis, and source — the common case.
    #[must_use]
    pub const fn brief(
        summary: &'static str,
        synopsis: &'static [&'static str],
        source: &'static str,
    ) -> Self {
        Self {
            summary,
            synopsis,
            snippet: "",
            source,
            examples: "",
            return_value: "",
        }
    }
}

/// Completion and hover metadata for a positional argument value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentValueSpec {
    /// Completable value text.
    pub value: &'static str,
    /// Short description in the completion list.
    pub detail: &'static str,
}

/// Metadata for a switch-like option (`-nonewline`, `-nocase`, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionSpec {
    /// Option name (e.g. `"-nonewline"`).
    pub name: &'static str,
    /// Whether this option consumes a following value.
    pub takes_value: bool,
    /// Hint text for the value (e.g. `"channel"`).
    pub value_hint: &'static str,
    /// Short description.
    pub detail: &'static str,
    /// Dialect membership.  `None` means "inherit from the parent
    /// `CommandSpec` / `SubCommand` dialects" — the common case.
    /// Set this to restrict an option added in a specific Tcl
    /// version (e.g. `lsearch -stride` is Tcl 8.6+, `clock scan
    /// -validate` is Tcl 9.0+) so the option doesn't surface in
    /// older dialects.  Mirrors `OptionSpec.dialects` in
    /// `core/commands/registry/models.py` (PR #433).
    pub dialects: Option<DialectSet>,
}

impl OptionSpec {
    /// Check whether this option is available in *dialect*.
    ///
    /// If the option has its own `dialects` set, use it.  Otherwise
    /// inherit from *`parent_dialects`* (the parent `CommandSpec` or
    /// `SubCommand`).  When either side is `None`, the option is
    /// considered available (no restriction).
    #[must_use]
    pub fn supports_dialect(
        &self,
        dialect: Option<DialectSet>,
        parent_dialects: Option<DialectSet>,
    ) -> bool {
        let Some(active) = dialect else {
            return true;
        };
        if let Some(own) = self.dialects {
            return own.contains(active);
        }
        let Some(parent) = parent_dialects else {
            return true;
        };
        parent.contains(active)
    }
}

/// Classification of a command invocation form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormKind {
    /// Default form.
    Default,
    /// Getter form (read-only).
    Getter,
    /// Setter form (modifying).
    Setter,
}

/// A concrete invocation form of a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormSpec {
    /// Form classification.
    pub kind: FormKind,
    /// Human-readable invocation signature.
    pub synopsis: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_dialect_inherits_from_parent_when_unset() {
        let opt = OptionSpec {
            name: "-foo",
            takes_value: false,
            value_hint: "",
            detail: "",
            dialects: None,
        };
        // No parent: always available.
        assert!(opt.supports_dialect(Some(DialectSet::TCL84), None));
        // Parent allows everything: available.
        assert!(opt.supports_dialect(Some(DialectSet::TCL84), Some(DialectSet::ALL_TCL)));
        // Parent restricts: inherit the restriction.
        assert!(opt.supports_dialect(Some(DialectSet::TCL86), Some(DialectSet::TCL86_PLUS)));
        assert!(!opt.supports_dialect(Some(DialectSet::TCL85), Some(DialectSet::TCL86_PLUS)));
    }

    #[test]
    fn supports_dialect_own_set_overrides_parent() {
        // `lsearch -stride` is Tcl 8.6+ even though `lsearch` itself
        // is available since 8.4.  The option's own dialects field
        // wins.
        let opt = OptionSpec {
            name: "-stride",
            takes_value: true,
            value_hint: "int",
            detail: "",
            dialects: Some(DialectSet::TCL86_PLUS),
        };
        assert!(opt.supports_dialect(Some(DialectSet::TCL86), Some(DialectSet::ALL_TCL)));
        assert!(opt.supports_dialect(Some(DialectSet::TCL90), Some(DialectSet::ALL_TCL)));
        assert!(!opt.supports_dialect(Some(DialectSet::TCL84), Some(DialectSet::ALL_TCL)));
        assert!(!opt.supports_dialect(Some(DialectSet::TCL85), Some(DialectSet::ALL_TCL)));
    }

    #[test]
    fn supports_dialect_none_active_is_unrestricted() {
        // No active dialect = treat option as available (e.g.
        // unscoped completion).
        let opt = OptionSpec {
            name: "-x",
            takes_value: false,
            value_hint: "",
            detail: "",
            dialects: Some(DialectSet::TCL90),
        };
        assert!(opt.supports_dialect(None, Some(DialectSet::TCL90)));
    }
}
