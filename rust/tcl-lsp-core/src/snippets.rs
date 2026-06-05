//! Context-aware snippet completion templates (GAP-A9).
//!
//! Port of `lsp/features/snippet_templates.py` (the Tcl-core templates).
//! Generates VS Code snippet-format completion items (tabstops `${1:…}`
//! / choices `${2|a,b|}` / final `$0`) that adapt to the formatter's
//! indent unit and the variables in scope.  The iRules event templates
//! (`RULE_INIT` / `HTTP_REQUEST` / …) — which depend on the enclosing
//! `when` event and the events declared in the file — are a follow-up.

use crate::completion::{CompletionItem, CompletionKind};

/// Immutable context for snippet generation.  Mirrors Python's
/// `SnippetContext` (brace style is always K&R, i.e. a bare `{`).
pub struct SnippetContext<'a> {
    /// Dialect string (`tcl8.6`, `f5-irules`, …).
    pub dialect: &'a str,
    /// One indent level (e.g. `"    "` or `"\t"`).
    pub indent_unit: &'a str,
    /// Variable names accessible at the cursor (for `${n|choices|}`).
    pub scope_vars: &'a [String],
    /// Text typed so far (the `tcl-…` prefix filter).
    pub partial: &'a str,
}

/// A registered snippet template.
struct Template {
    prefix: &'static str,
    label: &'static str,
    detail: &'static str,
    /// `f5-irules`-only when `true` (Tcl-core templates are all-dialect).
    irules_only: bool,
    generator: fn(&SnippetContext) -> String,
}

/// Return context-aware snippet completion items.  Mirrors
/// `get_snippet_completions`: filter each template by dialect and the
/// typed prefix, then emit a `Snippet`-kind item whose `insert_text` is
/// the generated body and whose `filter_text` is the `tcl-…` prefix.
#[must_use]
pub fn snippet_completions(ctx: &SnippetContext) -> Vec<CompletionItem> {
    let mut out = Vec::new();
    for tmpl in TEMPLATES {
        if tmpl.irules_only && ctx.dialect != "f5-irules" {
            continue;
        }
        if !ctx.partial.is_empty() && !tmpl.prefix.starts_with(ctx.partial) {
            continue;
        }
        out.push(CompletionItem {
            label: tmpl.label.to_string(),
            insert_text: (tmpl.generator)(ctx),
            kind: CompletionKind::Snippet,
            detail: Some(tmpl.detail.to_string()),
            // `Z0_…` sorts snippets after real symbols.
            sort_text: Some(format!("Z0_{}", tmpl.prefix)),
            is_snippet: true,
            filter_text: Some(tmpl.prefix.to_string()),
        });
    }
    out
}

/// Build a snippet placeholder offering the in-scope variables as
/// choices, or a plain default.  Mirrors `_var_choices`.
fn var_choices(ctx: &SnippetContext, tabstop: u32, default: &str) -> String {
    if ctx.scope_vars.is_empty() {
        return format!("${{{tabstop}:{default}}}");
    }
    let choices = ctx
        .scope_vars
        .iter()
        .take(10)
        .map(|v| {
            let escaped = v.replace(',', "\\,").replace('|', "\\|");
            format!("\\${escaped}")
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("${{{tabstop}|{choices}|}}")
}

// -- Tcl-core generators (mirror `_gen_*`) --------------------------

fn gen_proc(ctx: &SnippetContext) -> String {
    let i = ctx.indent_unit;
    format!("proc ${{1:name}} {{${{2:args}}}} {{\n{i}$0\n}}")
}

fn gen_namespace(ctx: &SnippetContext) -> String {
    let i = ctx.indent_unit;
    format!("namespace eval ${{1:::ns}} {{\n{i}$0\n}}")
}

fn gen_package(ctx: &SnippetContext) -> String {
    let i = ctx.indent_unit;
    format!(
        "package require Tcl ${{1:8.6}}\n\
         package provide ${{2:pkgname}} ${{3:1.0}}\n\
         \n\
         namespace eval ${{4:::${{2}}}} {{\n\
         {i}namespace export ${{5:*}}\n\
         }}\n\
         \n\
         $0"
    )
}

fn gen_class(ctx: &SnippetContext) -> String {
    let i = ctx.indent_unit;
    let ii = format!("{i}{i}");
    format!(
        "oo::class create ${{1:ClassName}} {{\n\
         {i}constructor {{${{2:args}}}} {{\n\
         {ii}${{3:# init}}\n\
         {i}}}\n\
         \n\
         {i}method ${{4:methodName}} {{${{5:args}}}} {{\n\
         {ii}$0\n\
         {i}}}\n\
         }}"
    )
}

fn gen_if(ctx: &SnippetContext) -> String {
    let i = ctx.indent_unit;
    format!("if {{${{1:condition}}}} {{\n{i}${{2:# then}}\n}} else {{\n{i}$0\n}}")
}

fn gen_foreach(ctx: &SnippetContext) -> String {
    let i = ctx.indent_unit;
    let list_ph = var_choices(ctx, 2, "listVar");
    format!("foreach ${{1:item}} {list_ph} {{\n{i}$0\n}}")
}

fn gen_for(ctx: &SnippetContext) -> String {
    let i = ctx.indent_unit;
    format!("for {{set ${{1:i}} 0}} {{\\$${{1:i}} < ${{2:10}}}} {{incr ${{1:i}}}} {{\n{i}$0\n}}")
}

fn gen_switch(ctx: &SnippetContext) -> String {
    let i = ctx.indent_unit;
    let ii = format!("{i}{i}");
    format!(
        "switch -- ${{1:value}} {{\n\
         {i}${{2:pattern}} {{\n\
         {ii}${{3:# body}}\n\
         {i}}}\n\
         {i}default {{\n\
         {ii}$0\n\
         {i}}}\n\
         }}"
    )
}

fn gen_catch(ctx: &SnippetContext) -> String {
    let i = ctx.indent_unit;
    format!(
        "if {{[catch {{\n\
         {i}${{1:# risky call}}\n\
         }} result opts]}} {{\n\
         {i}puts stderr \\$result\n\
         {i}return -code error -options \\$opts \\$result\n\
         }}\n\
         $0"
    )
}

fn gen_try(ctx: &SnippetContext) -> String {
    let i = ctx.indent_unit;
    format!(
        "try {{\n\
         {i}${{1:# body}}\n\
         }} trap {{${{2:TCL}} ${{3:*}}}} {{result opts}} {{\n\
         {i}$0\n\
         }}"
    )
}

fn gen_dict_for(ctx: &SnippetContext) -> String {
    let i = ctx.indent_unit;
    let dict_ph = var_choices(ctx, 3, "dictVar");
    format!("dict for {{${{1:key}} ${{2:value}}}} {dict_ph} {{\n{i}$0\n}}")
}

const TEMPLATES: &[Template] = &[
    Template {
        prefix: "tcl-proc",
        label: "Tcl Proc",
        detail: "Create a Tcl procedure",
        irules_only: false,
        generator: gen_proc,
    },
    Template {
        prefix: "tcl-namespace",
        label: "Namespace Eval",
        detail: "Create a namespace eval block",
        irules_only: false,
        generator: gen_namespace,
    },
    Template {
        prefix: "tcl-package",
        label: "Package Boilerplate",
        detail: "Create package provide/require boilerplate",
        irules_only: false,
        generator: gen_package,
    },
    Template {
        prefix: "tcl-class",
        label: "OO Class",
        detail: "Create an oo::class",
        irules_only: false,
        generator: gen_class,
    },
    Template {
        prefix: "tcl-if",
        label: "If Else",
        detail: "Create braced if/else block",
        irules_only: false,
        generator: gen_if,
    },
    Template {
        prefix: "tcl-foreach",
        label: "Foreach",
        detail: "Create a foreach loop",
        irules_only: false,
        generator: gen_foreach,
    },
    Template {
        prefix: "tcl-for",
        label: "For Loop",
        detail: "Create a for loop with braced expressions",
        irules_only: false,
        generator: gen_for,
    },
    Template {
        prefix: "tcl-switch",
        label: "Switch",
        detail: "Create a switch block with -- option terminator",
        irules_only: false,
        generator: gen_switch,
    },
    Template {
        prefix: "tcl-catch",
        label: "Catch with Result",
        detail: "Create a catch pattern that preserves result and options",
        irules_only: false,
        generator: gen_catch,
    },
    Template {
        prefix: "tcl-try",
        label: "Try Trap",
        detail: "Create a try/trap block",
        irules_only: false,
        generator: gen_try,
    },
    Template {
        prefix: "tcl-dict-for",
        label: "Dict For",
        detail: "Iterate key/value pairs in a dict",
        irules_only: false,
        generator: gen_dict_for,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx<'a>(partial: &'a str, vars: &'a [String]) -> SnippetContext<'a> {
        SnippetContext {
            dialect: "tcl8.6",
            indent_unit: "    ",
            scope_vars: vars,
            partial,
        }
    }

    #[test]
    fn emits_all_tcl_core_templates_with_no_prefix() {
        let items = snippet_completions(&ctx("", &[]));
        assert_eq!(items.len(), TEMPLATES.len());
        assert!(items.iter().all(|i| i.is_snippet));
        assert!(items.iter().all(|i| i.kind == CompletionKind::Snippet));
    }

    #[test]
    fn filters_by_prefix() {
        let items = snippet_completions(&ctx("tcl-fo", &[]));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["Foreach", "For Loop"]);
    }

    #[test]
    fn proc_snippet_uses_indent_and_tabstops() {
        let items = snippet_completions(&ctx("tcl-proc", &[]));
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].insert_text,
            "proc ${1:name} {${2:args}} {\n    $0\n}"
        );
        assert_eq!(items[0].filter_text.as_deref(), Some("tcl-proc"));
    }

    #[test]
    fn foreach_offers_scope_var_choices() {
        let vars = vec!["items".to_string(), "list".to_string()];
        let items = snippet_completions(&ctx("tcl-foreach", &vars));
        // The list placeholder becomes a choice list of in-scope vars.
        assert!(items[0].insert_text.contains("${2|\\$items,\\$list|}"));
    }
}
