//! Explorer view metadata: the tab table and severity vocabulary.
//!
//! The front-ends key off the `id`
//! strings, so this table is part of the de-facto explorer contract.

/// One explorer view tab: `(id, label, group)`.
///
/// `group` is informational (`compiler` / `optimiser` / `codegen`); the
/// CLI's `--show` grouping uses the separate `VIEW_GROUPS` mapping.
pub type ViewMeta = (&'static str, &'static str, &'static str);

/// The ordered view-tab table. Diverges intentionally from Python's
/// `_VIEW_META`: Rust's parser produces a single red-green CST (no separate
/// legacy "green tree"), so the `greentree` tab is dropped and `cst` is the
/// sole parse-tree view.
pub const VIEW_META: &[ViewMeta] = &[
    ("cst", "CST", "compiler"),
    ("segments", "Segments", "compiler"),
    ("structuralIndex", "Structural Index", "compiler"),
    ("sourceMap", "Source Map", "compiler"),
    ("ir", "IR", "compiler"),
    ("cfg", "CFG (pre-SSA)", "compiler"),
    ("ssa", "CFG (post-SSA)", "compiler"),
    ("loops", "Loops", "compiler"),
    ("types", "Types", "compiler"),
    ("intervals", "Intervals", "compiler"),
    ("bounds", "Bounds", "compiler"),
    ("dataflow", "Data Flow", "compiler"),
    ("interproc", "Interprocedural", "compiler"),
    ("rendered", "Rendered Props", "compiler"),
    ("opt", "Optimisations", "optimiser"),
    ("optimiserPasses", "Pass Pipeline", "optimiser"),
    ("gvn", "GVN", "optimiser"),
    ("shimmer", "Shimmer", "optimiser"),
    ("taint", "Taint", "optimiser"),
    ("irules", "iRules Flow", "optimiser"),
    ("eventOrder", "Event Order", "optimiser"),
    ("callouts", "Source Callouts", "optimiser"),
    ("asm", "Tcl ASM", "codegen"),
    ("asmOpt", "Tcl ASM (opt)", "codegen"),
    ("wasm", "WASM", "codegen"),
    ("wasmOpt", "WASM (opt)", "codegen"),
];

/// Renderer-agnostic severity classification.
///
/// The string value is what each renderer (CLI ANSI, GUI CSS class) keys
/// off.; order matches Python's enum
/// declaration so `meta.severities` lists `[error, warning, info]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// An error-level annotation.
    Error,
    /// A warning-level annotation.
    Warning,
    /// An informational annotation.
    Info,
}

impl Severity {
    /// All severities in Python declaration order.
    pub const ALL: [Severity; 3] = [Severity::Error, Severity::Warning, Severity::Info];

    /// The contract string the renderers key off.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}
