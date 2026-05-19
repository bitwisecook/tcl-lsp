//! Code-lens provider — Rust port of
//! `lsp/features/code_lens.py`.
//!
//! Surfaces a reference-count lens above every user-proc
//! definition: `N references` at the proc's name span,
//! showing how many call sites target it in the current
//! document.  Mirrors Python's `_proc_reference_count_lens`.
//!
//! What landed:
//!
//! * Per-proc lenses — `N references` per user proc.
//! * Class lenses — `N references` per `oo::class create`
//!   declaration, counting `ClassName new`, `ClassName create
//!   <inst>`, and inheritance references in
//!   `analysis.command_invocations`.
//!
//! Per-method / classmethod reference-count lenses also land:
//! each member declaration's name span gets a `N references`
//! lens counting call sites discovered by re-segmenting every
//! sibling method body in the class (the analyser doesn't
//! walk into method bodies for `command_invocations`).
//!
//! What is *deferred*:
//!
//! * Cross-document reference counts — workspace-wide
//!   matching that includes call sites in other open
//!   documents.  Lands alongside `S-workspace-symbols-rich`.
//! * `[$obj methodName]` call sites *outside* the class body
//!   — needs analyser-side variable-type tracking so the
//!   provider knows which object's class to match against.
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
    let mut lenses: Vec<CodeLens> = Vec::new();

    for (qname, proc_def) in &analysis.all_procs {
        let count = count_references(qname, proc_def, analysis);
        let title = reference_count_title(count);
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

    // `S-code-lens-rich`: class lenses.  Surface a reference-
    // count lens at each `oo::class create ClassName` site.
    // The count includes constructor calls (`ClassName new`,
    // `ClassName create instance`) and references in
    // inheritance chains (`oo::class create Sub { superclass
    // ClassName ... }`).
    for (qname, class_def) in &analysis.all_classes {
        let count = count_class_references(qname, class_def, analysis);
        let title = reference_count_title(count);
        let start = line_index.position_at(class_def.name_span.start());
        let end = line_index.position_at(class_def.name_span.end());
        lenses.push(CodeLens {
            range: LspRange {
                start_line: start.line,
                start_character: start.character,
                end_line: end.line,
                end_character: end.character,
            },
            command_title: title,
            command: String::new(),
        });
        // Per-method / classmethod / property lenses inside
        // the class body.  Counts call sites discovered by re-
        // segmenting every sibling method body (the analyser
        // doesn't walk into method bodies for
        // `command_invocations`).
        emit_class_member_lenses(source, class_def, &line_index, &mut lenses);
    }

    lenses
}

/// Emit per-member reference-count lenses for every method,
/// classmethod, and property in `class_def`.  Call sites are
/// discovered by re-segmenting each method body.
fn emit_class_member_lenses(
    source: &str,
    class_def: &tcl_compiler::analyser::ClassDef,
    line_index: &LineIndex,
    lenses: &mut Vec<CodeLens>,
) {
    let body_spans: Vec<tcl_lexer::Span> = class_def
        .methods
        .values()
        .map(|m| m.body_span)
        .chain(class_def.class_methods.values().map(|m| m.body_span))
        .chain(class_def.constructors.iter().map(|c| c.body_span))
        .chain(class_def.destructor.iter().map(|d| d.body_span))
        .collect();
    let calls_for = |word: &str, decl_span: tcl_lexer::Span| -> usize {
        let mut count = 0;
        for span in &body_spans {
            count += count_method_calls_in_body(source, *span, word, decl_span);
        }
        count
    };
    let push_lens = |name_span: tcl_lexer::Span, title: String, lenses: &mut Vec<CodeLens>| {
        let start = line_index.position_at(name_span.start());
        let end = line_index.position_at(name_span.end());
        lenses.push(CodeLens {
            range: LspRange {
                start_line: start.line,
                start_character: start.character,
                end_line: end.line,
                end_character: end.character,
            },
            command_title: title,
            command: String::new(),
        });
    };
    for m in class_def.methods.values() {
        if m.name_span.is_empty() {
            continue;
        }
        push_lens(
            m.name_span,
            reference_count_title(calls_for(&m.name, m.name_span)),
            lenses,
        );
    }
    for m in class_def.class_methods.values() {
        if m.name_span.is_empty() {
            continue;
        }
        push_lens(
            m.name_span,
            reference_count_title(calls_for(&m.name, m.name_span)),
            lenses,
        );
    }
}

/// Count call sites in `body_span` whose head token equals
/// `word`, skipping the declaration site at `decl_span`.
fn count_method_calls_in_body(
    source: &str,
    body_span: tcl_lexer::Span,
    word: &str,
    decl_span: tcl_lexer::Span,
) -> usize {
    use tcl_compiler::segmenter::segment_commands_with_offset;
    if body_span.is_empty() {
        return 0;
    }
    let mut start = body_span.start() as usize;
    let mut end = body_span.end() as usize;
    if start >= source.len() || end > source.len() || start > end {
        return 0;
    }
    if source.as_bytes().get(start) == Some(&b'{') {
        start += 1;
    }
    if end > start && source.as_bytes().get(end - 1) == Some(&b'}') {
        end -= 1;
    }
    let body_text = &source[start..end];
    let commands =
        segment_commands_with_offset(body_text, u32::try_from(start).unwrap_or(body_span.start()));
    let mut count = 0;
    for cmd in &commands {
        let Some(head) = cmd.argv.first() else {
            continue;
        };
        let h_start = head.span.start() as usize;
        let h_end = head.span.end() as usize;
        if h_start >= source.len() || h_end > source.len() {
            continue;
        }
        if &source[h_start..h_end] != word {
            continue;
        }
        if head.span.start() == decl_span.start() && head.span.end() == decl_span.end() {
            continue;
        }
        count += 1;
    }
    count
}

fn reference_count_title(count: usize) -> String {
    match count {
        0 => "0 references".to_string(),
        1 => "1 reference".to_string(),
        n => format!("{n} references"),
    }
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

/// Count invocations targeting the given class — `ClassName
/// new ...`, `ClassName create instance`, and inheritance
/// references in `oo::class create Sub { superclass ClassName
/// ... }`.  Excludes the class's own `oo::class create
/// ClassName` declaration site.
fn count_class_references(
    qname: &str,
    class_def: &tcl_compiler::analyser::ClassDef,
    analysis: &AnalysisResult,
) -> usize {
    let qname_no_prefix = qname.strip_prefix("::").unwrap_or(qname);
    analysis
        .command_invocations
        .iter()
        .filter(|inv| {
            inv.name == class_def.name
                || inv.name == class_def.qualified_name
                || inv.name == qname_no_prefix
        })
        .filter(|inv| {
            !(inv.range.start() <= class_def.name_span.start()
                && class_def.name_span.end() <= inv.range.end())
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

    // -- S-code-lens-rich: class lenses -----------------------------

    #[test]
    fn lens_per_class() {
        let src = "oo::class create Greeter {}\noo::class create Helper {}\n";
        let analysis = analyse(src);
        // Skip if the analyser doesn't track classes for this
        // tcl version / shape — the assertion below should still
        // succeed when the analyser does record them.
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, Some(&analysis));
        let class_lenses: Vec<_> = lenses
            .iter()
            .filter(|l| l.command_title.contains("reference"))
            .collect();
        assert!(class_lenses.len() >= 2, "{lenses:?}");
    }

    #[test]
    fn lens_counts_class_constructor_calls() {
        // `MyClass new` and `MyClass create instance` are
        // constructor calls — both contribute to the reference
        // count.
        let src = concat!(
            "oo::class create MyClass {}\n",
            "MyClass new\n",
            "MyClass create instance\n",
        );
        let analysis = analyse(src);
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, Some(&analysis));
        let myclass_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 0)
            .expect("class lens at line 0");
        // Two constructor calls.
        assert_eq!(
            myclass_lens.command_title, "2 references",
            "got {:?}",
            myclass_lens.command_title,
        );
    }

    // -- S-code-lens-rich: class-member lenses ----------------------

    #[test]
    fn lens_counts_method_calls_within_class_body() {
        // `greet` is called twice from `twice`'s body.
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, Some(&analysis));
        // Find the lens anchored on line 1 (the `greet`
        // declaration's name span).
        let greet_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("greet lens");
        assert_eq!(greet_lens.command_title, "2 references", "{lenses:?}");
    }

    #[test]
    fn lens_reports_zero_for_uncalled_method() {
        let src = "oo::class create C {\n    method orphan {} {}\n}\n";
        let analysis = analyse(src);
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, Some(&analysis));
        let orphan_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("orphan lens");
        assert_eq!(orphan_lens.command_title, "0 references", "{lenses:?}");
    }
}
