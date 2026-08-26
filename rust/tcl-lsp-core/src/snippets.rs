// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Context-aware snippet completion templates.
//!
//! Generates VS Code
//! snippet-format completion items (tabstops `${1:…}` / choices
//! `${2|a,b|}` / final `$0`) that adapt to the formatter's indent unit
//! and the variables in scope.  The iRules event templates (`RULE_INIT`
//! / `HTTP_REQUEST` / …) additionally depend on the enclosing `when`
//! event (offered only at the top level) and the events declared in the
//! file (declined when their event is already present).

use crate::completion::{CompletionItem, CompletionKind};

/// Immutable context for snippet generation (brace style is always
/// K&R, i.e. a bare `{`).
pub struct SnippetContext<'a> {
    /// Canonical dialect profile controlling template availability.
    pub profile: &'static tcl_dialect::DialectProfile,
    /// One indent level (e.g. `"    "` or `"\t"`).
    pub indent_unit: &'a str,
    /// Variable names accessible at the cursor (for `${n|choices|}`).
    pub scope_vars: &'a [String],
    /// Text typed so far (the `tcl-…` prefix filter).
    pub partial: &'a str,
    /// The enclosing `when` event at the cursor, or `None` at the top
    /// level. iRules event templates only offer at the top level.
    pub current_event: Option<&'a str>,
    /// `when` events already declared in the file — iRules event
    /// templates decline when their event is already present (avoids
    /// offering a duplicate `when HTTP_REQUEST`).
    pub file_events: &'a [String],
}

/// A registered snippet template.
struct Template {
    prefix: &'static str,
    label: &'static str,
    detail: &'static str,
    /// `f5-irules`-only when `true` (Tcl-core templates are all-dialect).
    irules_only: bool,
    /// Offered only outside any `when` block.  iRules event templates
    /// set this.
    requires_top_level: bool,
    /// Returns the snippet body, or `""` to decline (e.g. when the
    /// event is already in the file).
    generator: fn(&SnippetContext) -> String,
}

/// Return context-aware snippet completion items.  Filter each
/// template by dialect and the
/// typed prefix, then emit a `Snippet`-kind item whose `insert_text` is
/// the generated body and whose `filter_text` is the `tcl-…` prefix.
#[must_use]
pub fn snippet_completions(ctx: &SnippetContext) -> Vec<CompletionItem> {
    let mut out = Vec::new();
    for tmpl in TEMPLATES {
        if tmpl.irules_only && !ctx.profile.is_irules() {
            continue;
        }
        if tmpl.requires_top_level && ctx.current_event.is_some() {
            continue;
        }
        if !ctx.partial.is_empty() && !tmpl.prefix.starts_with(ctx.partial) {
            continue;
        }
        // An empty body means the generator declined (e.g. its `when`
        // event is already declared).
        let body = (tmpl.generator)(ctx);
        if body.is_empty() {
            continue;
        }
        out.push(CompletionItem {
            label: tmpl.label.to_string(),
            insert_text: body,
            kind: CompletionKind::Snippet,
            detail: Some(tmpl.detail.to_string()),
            // `Z0_…` sorts snippets after real symbols.
            sort_text: Some(format!("Z0_{}", tmpl.prefix)),
            is_snippet: true,
            filter_text: Some(tmpl.prefix.to_string()),
            text_edit: None,
            documentation: None,
        });
    }
    out
}

/// Build a snippet placeholder offering the in-scope variables as
/// choices, or a plain default.
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

// Tcl-core generators

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

// -- iRules event generators (declining via `""`) ----

/// `true` when `event` is already declared in the file.
fn has_event(ctx: &SnippetContext, event: &str) -> bool {
    ctx.file_events.iter().any(|e| e == event)
}

fn gen_rule_init(ctx: &SnippetContext) -> String {
    if has_event(ctx, "RULE_INIT") {
        return String::new();
    }
    let i = ctx.indent_unit;
    format!("when RULE_INIT {{\n{i}$0\n}}")
}

fn gen_http_request(ctx: &SnippetContext) -> String {
    if has_event(ctx, "HTTP_REQUEST") {
        return String::new();
    }
    let i = ctx.indent_unit;
    let ii = format!("{i}{i}");
    format!(
        "when HTTP_REQUEST {{\n\
         {i}set host [string tolower [HTTP::host]]\n\
         {i}set path [HTTP::path]\n\
         {i}set uri [HTTP::uri]\n\
         \n\
         {i}if {{\\$debug}} {{\n\
         {ii}log local0.debug \"HTTP_REQUEST host=\\$host path=\\$path\"\n\
         {i}}}\n\
         \n\
         {i}$0\n\
         }}"
    )
}

fn gen_redirect_https(ctx: &SnippetContext) -> String {
    if has_event(ctx, "HTTP_REQUEST") {
        return String::new();
    }
    let i = ctx.indent_unit;
    let ii = format!("{i}{i}");
    format!(
        "when HTTP_REQUEST {{\n\
         {i}if {{[TCP::local_port] == 80}} {{\n\
         {ii}HTTP::redirect \"https://[HTTP::host][HTTP::uri]\"\n\
         {ii}return\n\
         {i}}}\n\
         \n\
         {i}$0\n\
         }}"
    )
}

fn gen_collect_release(ctx: &SnippetContext) -> String {
    let has_req = has_event(ctx, "HTTP_REQUEST");
    let has_data = has_event(ctx, "HTTP_REQUEST_DATA");
    if has_req && has_data {
        return String::new();
    }
    let i = ctx.indent_unit;
    let ii = format!("{i}{i}");
    let mut parts: Vec<String> = Vec::new();
    if !has_req {
        parts.push(format!(
            "when HTTP_REQUEST {{\n\
             {i}if {{[HTTP::method] eq \"POST\"}} {{\n\
             {ii}HTTP::collect ${{1:1024}}\n\
             {ii}return\n\
             {i}}}\n\
             \n\
             {i}${{2:# non-body handling}}\n\
             }}"
        ));
    }
    if !has_data {
        parts.push(format!(
            "when HTTP_REQUEST_DATA {{\n\
             {i}set payload [HTTP::payload]\n\
             {i}HTTP::release\n\
             {i}$0\n\
             }}"
        ));
    }
    parts.join("\n\n")
}

fn gen_class_lookup(ctx: &SnippetContext) -> String {
    if has_event(ctx, "HTTP_REQUEST") {
        return String::new();
    }
    let i = ctx.indent_unit;
    let ii = format!("{i}{i}");
    format!(
        "when HTTP_REQUEST {{\n\
         {i}set host [string tolower [HTTP::host]]\n\
         {i}set pool_name [class match -value \\$host equals ${{1:host_to_pool_dg}}]\n\
         {i}if {{\\$pool_name ne \"\"}} {{\n\
         {ii}pool \\$pool_name\n\
         {ii}return\n\
         {i}}}\n\
         \n\
         {i}$0\n\
         }}"
    )
}

const TEMPLATES: &[Template] = &[
    Template {
        prefix: "tcl-proc",
        label: "Tcl Proc",
        detail: "Create a Tcl procedure",
        irules_only: false,
        requires_top_level: false,
        generator: gen_proc,
    },
    Template {
        prefix: "tcl-namespace",
        label: "Namespace Eval",
        detail: "Create a namespace eval block",
        irules_only: false,
        requires_top_level: false,
        generator: gen_namespace,
    },
    Template {
        prefix: "tcl-package",
        label: "Package Boilerplate",
        detail: "Create package provide/require boilerplate",
        irules_only: false,
        requires_top_level: false,
        generator: gen_package,
    },
    Template {
        prefix: "tcl-class",
        label: "OO Class",
        detail: "Create an oo::class",
        irules_only: false,
        requires_top_level: false,
        generator: gen_class,
    },
    Template {
        prefix: "tcl-if",
        label: "If Else",
        detail: "Create braced if/else block",
        irules_only: false,
        requires_top_level: false,
        generator: gen_if,
    },
    Template {
        prefix: "tcl-foreach",
        label: "Foreach",
        detail: "Create a foreach loop",
        irules_only: false,
        requires_top_level: false,
        generator: gen_foreach,
    },
    Template {
        prefix: "tcl-for",
        label: "For Loop",
        detail: "Create a for loop with braced expressions",
        irules_only: false,
        requires_top_level: false,
        generator: gen_for,
    },
    Template {
        prefix: "tcl-switch",
        label: "Switch",
        detail: "Create a switch block with -- option terminator",
        irules_only: false,
        requires_top_level: false,
        generator: gen_switch,
    },
    Template {
        prefix: "tcl-catch",
        label: "Catch with Result",
        detail: "Create a catch pattern that preserves result and options",
        irules_only: false,
        requires_top_level: false,
        generator: gen_catch,
    },
    Template {
        prefix: "tcl-try",
        label: "Try Trap",
        detail: "Create a try/trap block",
        irules_only: false,
        requires_top_level: false,
        generator: gen_try,
    },
    Template {
        prefix: "tcl-dict-for",
        label: "Dict For",
        detail: "Iterate key/value pairs in a dict",
        irules_only: false,
        requires_top_level: false,
        generator: gen_dict_for,
    },
    Template {
        prefix: "irule-rule-init",
        label: "iRule RULE_INIT",
        detail: "Initialise iRule static state",
        irules_only: true,
        requires_top_level: true,
        generator: gen_rule_init,
    },
    Template {
        prefix: "irule-http-request",
        label: "iRule HTTP_REQUEST",
        detail: "HTTP_REQUEST handler with safe defaults",
        irules_only: true,
        requires_top_level: true,
        generator: gen_http_request,
    },
    Template {
        prefix: "irule-redirect-https",
        label: "iRule Redirect HTTPS",
        detail: "Redirect HTTP traffic to HTTPS",
        irules_only: true,
        requires_top_level: true,
        generator: gen_redirect_https,
    },
    Template {
        prefix: "irule-collect-release",
        label: "iRule Collect/Release",
        detail: "Collect payload and release in HTTP_REQUEST_DATA",
        irules_only: true,
        requires_top_level: true,
        generator: gen_collect_release,
    },
    Template {
        prefix: "irule-class-lookup",
        label: "iRule Data Group Lookup",
        detail: "Data-group lookup and route",
        irules_only: true,
        requires_top_level: true,
        generator: gen_class_lookup,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx<'a>(partial: &'a str, vars: &'a [String]) -> SnippetContext<'a> {
        SnippetContext {
            profile: tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            indent_unit: "    ",
            scope_vars: vars,
            partial,
            current_event: None,
            file_events: &[],
        }
    }

    /// An `f5-irules` context at the top level with the given declared
    /// events.
    fn irule_ctx<'a>(partial: &'a str, events: &'a [String]) -> SnippetContext<'a> {
        SnippetContext {
            profile: tcl_dialect::DialectProfile::irules(),
            indent_unit: "    ",
            scope_vars: &[],
            partial,
            current_event: None,
            file_events: events,
        }
    }

    #[test]
    fn emits_all_tcl_core_templates_with_no_prefix() {
        let items = snippet_completions(&ctx("", &[]));
        // The `tcl8.6` dialect sees every all-dialect template but none
        // of the `f5-irules`-only ones.
        let tcl_core = TEMPLATES.iter().filter(|t| !t.irules_only).count();
        assert_eq!(items.len(), tcl_core);
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

    #[test]
    fn irules_templates_hidden_outside_irules_dialect() {
        let items = snippet_completions(&ctx("irule", &[]));
        assert!(items.is_empty());
    }

    #[test]
    fn irules_templates_offered_in_irules_dialect() {
        let items = snippet_completions(&irule_ctx("irule", &[]));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(
            labels,
            vec![
                "iRule RULE_INIT",
                "iRule HTTP_REQUEST",
                "iRule Redirect HTTPS",
                "iRule Collect/Release",
                "iRule Data Group Lookup",
            ]
        );
        assert_eq!(items[0].insert_text, "when RULE_INIT {\n    $0\n}");
    }

    #[test]
    fn rule_init_declines_when_event_already_declared() {
        let events = vec!["RULE_INIT".to_string()];
        let items = snippet_completions(&irule_ctx("irule-rule-init", &events));
        assert!(items.is_empty());
    }

    #[test]
    fn http_request_templates_decline_when_event_present() {
        let events = vec!["HTTP_REQUEST".to_string()];
        // Filter to the iRules templates (the `irule-` prefix) so the
        // always-offered Tcl-core ones don't enter the comparison.
        let items = snippet_completions(&irule_ctx("irule", &events));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        // Every template gated on HTTP_REQUEST drops out; RULE_INIT and
        // the HTTP_REQUEST_DATA half of collect/release remain.
        assert_eq!(labels, vec!["iRule RULE_INIT", "iRule Collect/Release"]);
        let collect = items
            .iter()
            .find(|i| i.label == "iRule Collect/Release")
            .unwrap();
        // Only the HTTP_REQUEST_DATA part survives.
        assert!(!collect.insert_text.contains("when HTTP_REQUEST {"));
        assert!(collect.insert_text.starts_with("when HTTP_REQUEST_DATA {"));
    }

    #[test]
    fn irules_event_templates_require_top_level() {
        let events: Vec<String> = Vec::new();
        let nested = SnippetContext {
            profile: tcl_dialect::DialectProfile::irules(),
            indent_unit: "    ",
            scope_vars: &[],
            partial: "irule",
            current_event: Some("HTTP_REQUEST"),
            file_events: &events,
        };
        // Inside a `when` block none of the event templates are offered.
        assert!(snippet_completions(&nested).is_empty());
    }
}
