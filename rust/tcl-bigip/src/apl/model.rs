//! Structured model for F5 iApp APL (Application Presentation Language).

use crate::range::Range;

/// A single form field declared in APL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AplField {
    /// Leaf name, e.g. `"addr"`.
    pub name: String,
    /// Dotted qualified name, e.g. `"basic.addr"`.
    pub qualified_name: String,
    /// Field type keyword, e.g. `"string"` / `"choice"`.
    pub field_type: String,
    /// Whether the field carries the `required` attribute.
    pub is_required: bool,
    /// Source range of the field name.
    pub range: Range,
}

/// A named `section` block.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AplSection {
    /// Section name.
    pub name: String,
    /// Qualified name (the section name).
    pub qualified_name: String,
    /// Fields declared in the section, in source order.
    pub fields: Vec<(String, AplField)>,
    /// Source range of the section name.
    pub range: Option<Range>,
}

/// A named `table` block with column fields.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AplTable {
    /// Table name.
    pub name: String,
    /// Qualified name (parent-section-qualified when nested).
    pub qualified_name: String,
    /// Column fields, in source order.
    pub columns: Vec<(String, AplField)>,
    /// Source range of the table name.
    pub range: Option<Range>,
}

/// An `#include` directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AplInclude {
    /// The path string from the directive.
    pub path: String,
    /// 0-based source line number.
    pub line: u32,
    /// Whether the included file was found / resolved.
    pub resolved: bool,
}

/// Complete parsed APL presentation model.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AplModel {
    /// `section` blocks, in source order.
    pub sections: Vec<(String, AplSection)>,
    /// `table` blocks, in source order.
    pub tables: Vec<(String, AplTable)>,
    /// `define` blocks: name -> underlying type.
    pub defines: Vec<(String, String)>,
    /// `#include` directives.
    pub includes: Vec<AplInclude>,
    /// Flattened `qualified_name -> AplField` for quick lookup.
    pub all_fields: Vec<(String, AplField)>,
}

impl AplModel {
    /// The set of Tcl global variable names this presentation exports —
    /// APL `section.field` becomes `::section__field`.
    #[must_use]
    pub fn tcl_variable_names(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .all_fields
            .iter()
            .map(|(q, _)| format!("::{}", q.replace('.', "__")))
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

/// Convert a Tcl iApp variable name to its APL qualified name
/// (`::basic__addr` -> `basic.addr`); `None` when it isn't the iApp
/// convention.
#[must_use]
pub fn tcl_var_to_apl_name(tcl_var: &str) -> Option<String> {
    let name = tcl_var.trim_start_matches(':');
    if !name.contains("__") {
        return None;
    }
    Some(name.replace("__", "."))
}

/// Convert an APL qualified name to the Tcl global variable name
/// (`basic.addr` -> `::basic__addr`).
#[must_use]
pub fn apl_name_to_tcl_var(apl_name: &str) -> String {
    format!("::{}", apl_name.replace('.', "__"))
}
