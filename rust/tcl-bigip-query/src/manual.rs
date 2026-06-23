//! The combined `f5 query --help-manual` surface.
//!
//! Concatenates the grammar reference, the builtins catalogue, and the
//! cookbook, composing the three sections from the [`crate::grammar`],
//! [`crate::builtins`], and [`crate::examples`] formatters. The builtins
//! section is the metadata-driven catalogue (see
//! [`crate::builtins::format_catalogue`]) rather than per-function
//! prose.

/// Render the comprehensive manual: grammar + builtins + cookbook.
#[must_use]
pub fn format_manual() -> String {
    let mut out = String::new();
    out.push_str(&crate::grammar::format_grammar());
    out.push('\n');
    out.push_str(&crate::builtins::format_catalogue(None));
    out.push('\n');
    out.push_str(&crate::examples::format_examples());
    out
}

#[cfg(test)]
mod tests {
    use super::format_manual;

    #[test]
    fn manual_composes_all_three_sections() {
        let m = format_manual();
        assert!(m.contains("F5 QUERY DSL — GRAMMAR"), "grammar section");
        assert!(
            m.contains("F5 QUERY DSL — BUILTIN FUNCTIONS"),
            "builtins section"
        );
        assert!(m.contains("F5 QUERY DSL — COOKBOOK"), "cookbook section");
        assert!(m.ends_with('\n'));
    }
}
