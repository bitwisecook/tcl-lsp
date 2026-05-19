//! Code-lens provider — Rust port of
//! `lsp/features/code_lens.py`.
//!
//! Surfaces a reference-count lens above every user-proc
//! definition: `N references` at the proc's name span,
//! showing how many call sites target it in the current
//! document.  Mirrors Python's `_proc_reference_count_lens`.
//!
//! What is *deferred*:
//!
//! * Cross-document reference counts — workspace-wide
//!   matching that includes call sites in other open
//!   documents.  Lands alongside `S-workspace-symbols-rich`.
//! * Class / method reference lenses (Python's
//!   `_class_reference_count_lens` /
//!   `_method_reference_count_lens`).  Same shape as proc
//!   lenses but keyed on `ClassDef` / `MethodDef`; deferred
//!   until the analyser surfaces method-reference tracking
//!   (gated on `S-references-rich` follow-up).
//! * Inline command for "show references" jump-out — the
//!   minimal lens carries a static label; the editor's
//!   built-in references command can be invoked from the
//!   lens itself.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

use crate::definition::LspRange;

/// One code-lens entry — anchor range plus a command label
/// (Python uses commands like `Show Type`/`Show References`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeLens {
    /// Anchor range for the lens.
    pub range: LspRange,
    /// Command label shown to the user.
    pub command_title: String,
    /// Command identifier sent on click.
    pub command: String,
}

/// Compute code lenses for the document.
///
/// `analysis` is the analyser result for the document; when
/// `None`, returns an empty vector (preserves the stub call
/// shape for callers that haven't yet plumbed analysis
/// through).
#[must_use]
pub fn code_lenses(source: &str, analysis: Option<&AnalysisResult>) -> Vec<CodeLens> {
    let Some(analysis) = analysis else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    let mut lenses = Vec::new();

    for (qname, proc_def) in &analysis.all_procs {
        let count = count_references(qname, proc_def, analysis);
        let title = match count {
            0 => "0 references".to_string(),
            1 => "1 reference".to_string(),
            n => format!("{n} references"),
        };
        let start = line_index.position_at(proc_def.name_span.start());
        let end = line_index.position_at(proc_def.name_span.end());
        lenses.push(CodeLens {
            range: LspRange {
                start_line: start.line,
                start_character: start.character,
                end_line: end.line,
                end_character: end.character,
            },
            command_title: title,
            // Empty command — the lens is informational only.
            // Editors render the title as text; clicking is a
            // no-op until the references-jump command is wired
            // up in a follow-up.
            command: String::new(),
        });
    }

    lenses
}

/// Count the call sites in the analysis that target the given
/// proc.  Mirrors the matching logic in
/// [`crate::references`] / [`crate::call_hierarchy`].
fn count_references(
    qname: &str,
    proc_def: &tcl_compiler::analyser::ProcDef,
    analysis: &AnalysisResult,
) -> usize {
    let qname_no_prefix = qname.strip_prefix("::").unwrap_or(qname);
    analysis
        .command_invocations
        .iter()
        .filter(|inv| {
            inv.name == proc_def.name
                || inv.name == proc_def.qualified_name
                || inv.name == qname_no_prefix
                || inv
                    .resolved_qualified_name
                    .as_deref()
                    .is_some_and(|r| r == proc_def.qualified_name)
        })
        // Exclude the proc's own declaration site so a proc
        // with no callers shows `0 references`, not `1`.
        .filter(|inv| {
            !(inv.range.start() <= proc_def.name_span.start()
                && proc_def.name_span.end() <= inv.range.end())
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn empty_lenses_when_analysis_is_none() {
        assert!(code_lenses("proc foo {} {}\n", None).is_empty());
    }

    #[test]
    fn lens_per_user_proc() {
        let src = "proc foo {} {}\nproc bar {} {}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, Some(&analysis));
        assert_eq!(lenses.len(), 2, "{lenses:?}");
    }

    #[test]
    fn lens_shows_zero_references_for_unused_proc() {
        let src = "proc lonely {} {}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, Some(&analysis));
        assert_eq!(lenses.len(), 1);
        assert_eq!(lenses[0].command_title, "0 references");
    }

    #[test]
    fn lens_shows_singular_for_one_reference() {
        let src = "proc helper {} {}\nhelper\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, Some(&analysis));
        let helper = lenses
            .iter()
            .find(|l| l.range.start_line == 0)
            .expect("helper lens");
        assert_eq!(helper.command_title, "1 reference");
    }

    #[test]
    fn lens_counts_multiple_references() {
        let src = "proc tool {} {}\ntool\ntool\ntool\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, Some(&analysis));
        let tool = lenses
            .iter()
            .find(|l| l.range.start_line == 0)
            .expect("tool lens");
        assert_eq!(tool.command_title, "3 references");
    }

    #[test]
    fn lens_anchors_at_proc_name_span() {
        let src = "proc greet {} {}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, Some(&analysis));
        assert_eq!(lenses.len(), 1);
        // `greet` starts at column 5 (after `proc `).
        assert_eq!(lenses[0].range.start_character, 5);
    }
}
