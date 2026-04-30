//! Documentation and completion metadata for LSP features.

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
