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

//! The tool table: each tool's name, description, parameters and handler, and the input schema rendered from a row.

use super::diagnostics::{analyze, code_actions, find_legacy, review, validate};
use super::docstrings::{generate_docstring, read_proc_docs, update_docstrings};
use super::editor_queries::{
    command_info, complete, event_info, find_references, goto_definition, hover, rename,
};
use super::refactorings::{
    brace_expr, extract_datagroup, extract_variable, if_to_switch, inline_variable, refactor,
    switch_to_dict,
};
use super::source_structure::{
    call_graph, dataflow_graph, def_use_chains, detect_dialect, diagram, event_order,
    memory_aliases, symbol_graph, symbols,
};
use super::transforms::{compile_wasm, format_source, optimize};
use super::{Param, ToolDef, help, set_dialect, unminify_error};
use serde_json::{Map, Value, json};

/// The property name every dialect-taking tool spells its dialect argument
/// with. [`input_schema`] keys the catalogue enum off it, so a tool advertises
/// the full dialect list by naming its parameter rather than by repeating the
/// catalogue per tool.
pub(super) const DIALECT_PARAM: &str = "dialect";

/// [`spectcl_check`](crate::spectcl::spectcl_check)'s discovery-tier
/// parameter. [`input_schema`] gives it a closed `enum`, the four
/// [`tcl_spectcl::Tier`] spellings.
pub(super) const TIER_PARAM: &str = "tier";

/// [`spectcl_check`](crate::spectcl::spectcl_check)'s Workspace Trust
/// parameter. [`input_schema`] gives it a closed `enum`, the two
/// [`tcl_dialect::model::WorkspaceTrust`] spellings.
pub(super) const TRUST_PARAM: &str = "trust";

/// Every selectable environment's canonical id, in selectable order, read from
/// the live registry.
pub(super) fn dialect_names() -> Vec<String> {
    tcl_registry::model::selectable_environments()
        .iter()
        .map(|environment| environment.id.to_string())
        .collect()
}

/// The schema for a dialect-valued property: the selectable environments as a
/// JSON-Schema `enum`, plus each `name: description` pair appended to `desc` so
/// a model reading only the description still sees what each name means.
///
/// The `enum` lists canonical ids only; the runtime ingress keeps accepting
/// every declared name.
fn dialect_schema(desc: &str) -> Value {
    let selectable = tcl_registry::model::selectable_environments();
    let pairs: Vec<String> = selectable
        .iter()
        .map(|environment| format!("{}: {}", environment.id, environment.description()))
        .collect();
    json!({
        "type": "string",
        "description": format!(
            "{desc}. Canonical names: {}. Registered aliases are accepted too.",
            pairs.join("; ")
        ),
        "enum": dialect_names(),
    })
}

pub(super) const SRC: Param = ("source", "string", "Tcl or iRules source code");

pub(super) const DISABLE: Param = (
    "disable",
    "string",
    "Comma-separated diagnostic codes to turn off for this call — the invocation \
     layer, over the user's global config.ini",
);

pub(super) const ENABLE: Param = (
    "enable",
    "string",
    "Comma-separated diagnostic codes to turn on for this call, including a \
     default-off code such as W242",
);

pub(super) const OPT_DISABLE: Param = (
    "disable",
    "string",
    "Comma-separated optimisation codes to turn off on top of the profile",
);

pub(super) const OPT_ENABLE: Param = (
    "enable",
    "string",
    "Comma-separated optimisation codes to turn on that the profile turns off",
);

pub(super) const DIALECT: Param = (
    DIALECT_PARAM,
    "string",
    "Language dialect; auto-detected if empty",
);

pub(super) const TIER: Param = (
    TIER_PARAM,
    "string",
    "Discovery tier to preview this pack loading at — bundled, user, workspace, or \
     studio-override; defaults to workspace, the tier a .tclspec file actually installs \
     at. Governs untrusted_tier_refusal (never raised for bundled or user, which no trust \
     state makes untrusted) and, with trust, dormant_hooks and stamp_refusals; never which \
     commands, notices, or hooks are reported — the pack is always evaluated as trusted",
);

pub(super) const TRUST: Param = (
    TRUST_PARAM,
    "string",
    "Workspace Trust state to preview this pack loading under — trusted or untrusted; \
     defaults to trusted. With tier, decides dormant_hooks (a workspace hook body stays \
     dormant only when untrusted)",
);

pub(super) const LINE: Param = ("line", "integer", "0-based line of the cursor");

pub(super) const CHAR: Param = ("character", "integer", "0-based character of the cursor");

pub(super) const START_LINE: Param = (
    "start_line",
    "integer",
    "0-based start line of the selection",
);

pub(super) const START_CHAR: Param = (
    "start_character",
    "integer",
    "0-based start character of the selection",
);

pub(super) const END_LINE: Param = ("end_line", "integer", "0-based end line of the selection");

pub(super) const END_CHAR: Param = (
    "end_character",
    "integer",
    "0-based end character of the selection",
);

pub(super) const STYLE: Param = (
    "style",
    "string",
    "Docstring style: 'doxygen' (default) or 'plain'",
);

pub(super) const DECORATION: Param = (
    "decoration",
    "boolean",
    "Add '# ....' rule lines above/below the stub",
);

pub(super) const TOOLS: &[ToolDef] = &[
    ToolDef {
        name: "call_graph",
        description: "Proc caller→callee graph with call sites, roots, and leaf procs.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: |args| Ok(call_graph(args)),
    },
    ToolDef {
        name: "symbol_graph",
        description: "Scope hierarchy with proc/variable definitions, references, and package requires.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: |args| Ok(symbol_graph(args)),
    },
    ToolDef {
        name: "dataflow_graph",
        description: "Taint warnings, tainted variables, and per-proc side-effect classification.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: |args| Ok(dataflow_graph(args)),
    },
    ToolDef {
        name: "def_use_chains",
        description: "SSA def-use chains + memory-SSA aliases; optional variable filter.",
        params: &[
            SRC,
            DIALECT,
            (
                "variable",
                "string",
                "Filter to a specific variable name (optional)",
            ),
        ],
        required: &["source"],
        handler: |args| Ok(def_use_chains(args)),
    },
    ToolDef {
        name: "memory_aliases",
        description: "Memory-SSA alias sets (upvar/global/variable) with reasons and locations.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: |args| Ok(memory_aliases(args)),
    },
    ToolDef {
        name: "diagram",
        description: "Control-flow diagram data ({events, procedures}) from the IR.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: |args| Ok(diagram(args)),
    },
    ToolDef {
        name: "detect_dialect",
        description: "Detect the Tcl dialect from source (+ optional filename).",
        params: &[
            SRC,
            (
                "filename",
                "string",
                "File name for extension-based detection (optional)",
            ),
        ],
        required: &["source"],
        handler: |args| Ok(detect_dialect(args)),
    },
    ToolDef {
        name: "event_order",
        description: "iRule events in canonical firing order with multiplicity.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: |args| Ok(event_order(args)),
    },
    ToolDef {
        name: "format_source",
        description: "Format Tcl/iRules source.",
        params: &[
            SRC,
            DIALECT,
            ("indent_size", "integer", "Spaces per indent (default 4)"),
            ("indent_style", "string", "'spaces' or 'tabs'"),
            (
                "max_line_length",
                "integer",
                "Max line length (default 120)",
            ),
        ],
        required: &["source"],
        handler: |args| Ok(format_source(args)),
    },
    ToolDef {
        name: "optimize",
        description: "Find optimisation opportunities and produce rewritten source. Only the \
                      rewrites the source's policy shows are applied: a `# noqa` on a command, a \
                      top-of-file `# tcl-lsp: disable=`, the profile and the global config.ini \
                      keep a rewrite off exactly as they keep a diagnostic off.",
        params: &[
            SRC,
            DIALECT,
            (
                "profile",
                "string",
                "off | readability | standard | full | aggressive. Named, it wins over the \
                 global config.ini's [optimiser] profile; omitted, that profile applies, \
                 else full",
            ),
            OPT_DISABLE,
            OPT_ENABLE,
        ],
        required: &["source"],
        handler: optimize,
    },
    ToolDef {
        name: "compile_wasm",
        description: "Compile through the canonical analysis-aware WebAssembly pipeline and \
                      return the per-script WAT module and counts.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: |args| Ok(compile_wasm(args)),
    },
    ToolDef {
        name: "inline_variable",
        description: "Inline a single-use variable at the cursor.",
        params: &[SRC, LINE, CHAR, DIALECT],
        required: &["source", "line", "character"],
        handler: |args| Ok(inline_variable(args)),
    },
    ToolDef {
        name: "if_to_switch",
        description: "Convert an if/elseif chain testing one variable to a switch.",
        params: &[SRC, LINE, CHAR, DIALECT],
        required: &["source", "line", "character"],
        handler: |args| Ok(if_to_switch(args)),
    },
    ToolDef {
        name: "switch_to_dict",
        description: "Convert a switch whose arms set one variable to a dict lookup.",
        params: &[SRC, LINE, CHAR, DIALECT],
        required: &["source", "line", "character"],
        handler: |args| Ok(switch_to_dict(args)),
    },
    ToolDef {
        name: "brace_expr",
        description: "Brace an unbraced expr argument for safety/performance.",
        params: &[SRC, LINE, CHAR, DIALECT],
        required: &["source", "line", "character"],
        handler: |args| Ok(brace_expr(args)),
    },
    ToolDef {
        name: "analyze",
        description: "Full analysis: diagnostics (+category), document symbols, detected events, and event firing order. \
                      Diagnostics are the source's shown set, including the compiler checks (S1xx, T1xx, \
                      IRULE1xxx–5xxx) and the source-style pass (W111, W112, W115, W118) — inline `# noqa`, \
                      top-of-file `# tcl-lsp: disable=`, the global config.ini and the call's disable/enable \
                      arguments all apply, as in the editor; and a `suppressed` array: every finding the policy \
                      hides, with its reason.",
        params: &[SRC, DIALECT, DISABLE, ENABLE],
        required: &["source"],
        handler: |args| Ok(analyze(args)),
    },
    ToolDef {
        name: "validate",
        description: "Diagnostics grouped by category (security, taint, thread-safety, control-flow, performance, style, …), \
                      from the source's shown set, including the compiler checks (S1xx, T1xx, IRULE1xxx–5xxx) and \
                      the source-style pass (W111, W112, W115, W118) (directives, the global config.ini and \
                      disable/enable apply); and a `suppressed` array: every finding the policy hides, with its \
                      reason.",
        params: &[SRC, DIALECT, DISABLE, ENABLE],
        required: &["source"],
        handler: |args| Ok(validate(args)),
    },
    ToolDef {
        name: "review",
        description: "Security, taint, and thread-safety diagnostics for a focused review, from the source's shown set, \
                      including the compiler checks (S1xx, T1xx, IRULE1xxx–5xxx) and the source-style pass (W111, \
                      W112, W115, W118) (directives, the global config.ini and disable/enable apply); and a \
                      `suppressed` array: every finding the policy hides, with its reason.",
        params: &[SRC, DIALECT, DISABLE, ENABLE],
        required: &["source"],
        handler: |args| Ok(review(args)),
    },
    ToolDef {
        name: "find-legacy",
        description: "Auto-convertible legacy patterns with a modernisation hint per finding, from the source's shown set, \
                      including the compiler checks (S1xx, T1xx, IRULE1xxx–5xxx) and the source-style pass (W111, \
                      W112, W115, W118) (directives, the global config.ini and disable/enable apply); and a \
                      `suppressed` array: every finding the policy hides, with its reason.",
        params: &[SRC, DIALECT, DISABLE, ENABLE],
        required: &["source"],
        handler: |args| Ok(find_legacy(args)),
    },
    ToolDef {
        name: "event_info",
        description: "iRules event metadata: multiplicity, side, transport, implied profiles, valid commands.",
        params: &[(
            "event_name",
            "string",
            "iRules event name, e.g. HTTP_REQUEST",
        )],
        required: &["event_name"],
        handler: |args| Ok(event_info(args)),
    },
    ToolDef {
        name: "command_info",
        description: "Registry metadata for a command: summary, synopsis, switches, valid events.",
        params: &[
            ("command_name", "string", "Command name, e.g. HTTP::uri"),
            (
                "bigip_version",
                "string",
                "Optional target BIG-IP release; reports whether the command exists there",
            ),
        ],
        required: &["command_name"],
        handler: |args| Ok(command_info(args)),
    },
    ToolDef {
        name: "symbols",
        description: "Document symbol tree (procs, classes, methods, variables) with ranges.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: |args| Ok(symbols(args)),
    },
    ToolDef {
        name: "hover",
        description: "Hover documentation (markdown) for the symbol at the cursor.",
        params: &[SRC, LINE, CHAR, DIALECT],
        required: &["source", "line", "character"],
        handler: |args| Ok(hover(args)),
    },
    ToolDef {
        name: "complete",
        description: "Completion items at the cursor (variables, procs, switches, subcommands, snippets).",
        params: &[SRC, LINE, CHAR, DIALECT],
        required: &["source", "line", "character"],
        handler: |args| Ok(complete(args)),
    },
    ToolDef {
        name: "goto_definition",
        description: "Definition location(s) for the symbol at the cursor.",
        params: &[SRC, LINE, CHAR, DIALECT],
        required: &["source", "line", "character"],
        handler: |args| Ok(goto_definition(args)),
    },
    ToolDef {
        name: "find_references",
        description: "All references to the symbol at the cursor (including its declaration).",
        params: &[SRC, LINE, CHAR, DIALECT],
        required: &["source", "line", "character"],
        handler: |args| Ok(find_references(args)),
    },
    ToolDef {
        name: "rename",
        description: "Text edits to rename the symbol at the cursor, or a refused object with a reason.",
        params: &[
            SRC,
            LINE,
            CHAR,
            ("new_name", "string", "New symbol name"),
            DIALECT,
        ],
        required: &["source", "line", "character", "new_name"],
        handler: |args| Ok(rename(args)),
    },
    ToolDef {
        name: "code_actions",
        description: "Code actions (quick-fixes + refactors) for a selection range: a fix for every finding the source \
                      shows — analyser, compiler-check and optimiser rewrite alike — and for no other.",
        params: &[
            SRC, START_LINE, START_CHAR, END_LINE, END_CHAR, DIALECT, DISABLE, ENABLE,
        ],
        required: &[
            "source",
            "start_line",
            "start_character",
            "end_line",
            "end_character",
        ],
        handler: |args| Ok(code_actions(args)),
    },
    ToolDef {
        name: "extract_variable",
        description: "Extract a selected expression into a new `set var …` binding.",
        params: &[
            SRC,
            START_LINE,
            START_CHAR,
            END_LINE,
            END_CHAR,
            DIALECT,
            ("var_name", "string", "New variable name (default 'result')"),
        ],
        required: &[
            "source",
            "start_line",
            "start_character",
            "end_line",
            "end_character",
        ],
        handler: |args| Ok(extract_variable(args)),
    },
    ToolDef {
        name: "extract_datagroup",
        description: "Extract an if/switch over literals into an iRules data-group + lookup.",
        params: &[
            SRC,
            DIALECT,
            LINE,
            CHAR,
            (
                "dg_name",
                "string",
                "Data-group name; auto-generated if empty",
            ),
        ],
        required: &["source", "line", "character"],
        handler: |args| Ok(extract_datagroup(args)),
    },
    ToolDef {
        name: "refactor",
        description: "List the refactorings available at a selection (probe of all refactors).",
        params: &[SRC, START_LINE, START_CHAR, END_LINE, END_CHAR, DIALECT],
        required: &[
            "source",
            "start_line",
            "start_character",
            "end_line",
            "end_character",
        ],
        handler: |args| Ok(refactor(args)),
    },
    ToolDef {
        name: "generate_docstring",
        description: "Generate a docstring stub for a named proc.",
        params: &[
            SRC,
            ("proc_name", "string", "Proc to document"),
            STYLE,
            DECORATION,
            DIALECT,
        ],
        required: &["source", "proc_name"],
        handler: |args| Ok(generate_docstring(args)),
    },
    ToolDef {
        name: "read_proc_docs",
        description: "Structured docs for every proc: params, parsed docstring, inferred param traits.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: |args| Ok(read_proc_docs(args)),
    },
    ToolDef {
        name: "update_docstrings",
        description: "Insert docstring stubs above every undocumented proc; returns rewritten source.",
        params: &[SRC, STYLE, DECORATION, DIALECT],
        required: &["source"],
        handler: |args| Ok(update_docstrings(args)),
    },
    ToolDef {
        name: "unminify_error",
        description: "Translate a minified error back to original symbol names / line numbers.",
        params: &[
            ("error_message", "string", "The error text to translate"),
            ("symbol_map", "string", "Minified→original symbol map"),
            (
                "minified_source",
                "string",
                "Minified source (optional, for line remapping)",
            ),
            (
                "original_source",
                "string",
                "Original source (optional, for line remapping)",
            ),
        ],
        required: &["error_message", "symbol_map"],
        handler: |args| Ok(unminify_error(args)),
    },
    ToolDef {
        name: "set_dialect",
        description: "Set the session default dialect used when a tool's dialect is auto-detected.",
        params: &[(
            DIALECT_PARAM,
            "string",
            "Dialect to make the session default (e.g. f5-irules, tcl9.0)",
        )],
        required: &["dialect"],
        handler: set_dialect,
    },
    ToolDef {
        name: "fakecmp_which_tmm",
        description: "Which TMM a connection 4-tuple hashes to under the fakeCMP disaggregator.",
        params: &[
            ("tmm_count", "integer", "Number of TMMs (>= 2)"),
            ("src_addr", "string", "Source IPv4"),
            ("src_port", "integer", "Source port (0-65535)"),
            ("dst_addr", "string", "Destination IPv4"),
            ("dst_port", "integer", "Destination port (0-65535)"),
        ],
        required: &["tmm_count", "src_addr", "src_port", "dst_addr", "dst_port"],
        handler: crate::fakecmp::which_tmm,
    },
    ToolDef {
        name: "fakecmp_suggest_sources",
        description: "Source tuples that spread test traffic across every TMM.",
        params: &[
            ("tmm_count", "integer", "Number of TMMs (>= 2)"),
            ("count", "integer", "Tuples per TMM (default 1)"),
            (
                "dst_addr",
                "string",
                "Destination IPv4 (default 192.168.1.100)",
            ),
            ("dst_port", "integer", "Destination port (default 443)"),
        ],
        required: &["tmm_count"],
        handler: crate::fakecmp::suggest_sources,
    },
    ToolDef {
        name: "irule_with_context",
        description: "Bundle each iRule in a BIG-IP config with the objects it references (pools, data-groups, profiles, monitors, …).",
        params: &[
            (
                "config_text",
                "string",
                "BIG-IP config text (bigip.conf/scf)",
            ),
            (
                "config_paths",
                "array",
                "Paths to config files to merge (optional)",
            ),
            (
                "rule_path",
                "string",
                "Full path of a single rule to bundle (optional)",
            ),
            ("format", "string", "'json' (default) or 'text'"),
            (
                "transitive",
                "boolean",
                "Follow transitive references (default true)",
            ),
        ],
        required: &[],
        handler: crate::bigip::irule_with_context,
    },
    ToolDef {
        name: "hud_chain",
        description: "HUD chain of each virtual server in a BIG-IP config: profiles as HUD filters (type, layer, side), \
                      attached iRules in rule-list order, and every `when` handler merged into one table per event, \
                      ordered by priority then rule position, with equal-priority ties and whether the profiles raise \
                      each event.",
        params: &[
            (
                "config_text",
                "string",
                "BIG-IP config text (bigip.conf/scf)",
            ),
            (
                "config_paths",
                "array",
                "Paths to config files to merge (optional)",
            ),
            (
                "virtual",
                "string",
                "Virtual server full path or name (optional; default all)",
            ),
            (
                "rule_path",
                "string",
                "Only virtual servers that attach this iRule (optional)",
            ),
        ],
        required: &[],
        handler: crate::bigip::hud_chain,
    },
    ToolDef {
        name: "help",
        description: "Search the KCS feature knowledge base (empty topic → full catalogue).",
        params: &[
            ("topic", "string", "Search query; empty lists all features"),
            (
                DIALECT_PARAM,
                "string",
                "Filter features by dialect (optional)",
            ),
            ("limit", "integer", "Max matches (default 20)"),
        ],
        required: &[],
        handler: |args| Ok(help(args)),
    },
    ToolDef {
        name: "xc_translate",
        description: "Translate an iRule to F5 Distributed Cloud (XC) config — terraform HCL, ves.io JSON, or pasteable XC Console documents, with per-command coverage.",
        params: &[
            SRC,
            (
                "output_format",
                "string",
                "'terraform', 'json', 'console' (one pasteable Console document per object), or 'both' (default: terraform + json)",
            ),
        ],
        required: &["source"],
        handler: |args| Ok(crate::xc::xc_translate(args)),
    },
    ToolDef {
        name: "explain_flow",
        description: "Narrate a captured session (pcap) against a BIG-IP config: matched VS, profiles, HTTP/TLS, ordered iRule events, pool decision, optional simulation.",
        params: &[
            ("pcap_path", "string", "Path to the pcap/pcapng capture"),
            ("config_text", "string", "Inline BIG-IP config (optional)"),
            (
                "config_paths",
                "array",
                "Config file paths to merge (optional)",
            ),
            (
                "use_tshark",
                "boolean",
                "Use the tshark overlay for L7/TLS decode",
            ),
            (
                "keylog_path",
                "string",
                "TLS keylog file for decryption (optional)",
            ),
            (
                "tshark_filter",
                "string",
                "Extra tshark display filter (optional)",
            ),
            (
                "max_event_body_lines",
                "integer",
                "Max iRule body lines per event (default 8)",
            ),
            ("simulate", "boolean", "Run the iRule simulation pass"),
        ],
        required: &["pcap_path"],
        handler: crate::bigip::explain_flow,
    },
    ToolDef {
        name: "tk_layout",
        description: "Build the shared, versioned static Tk UI model without executing source. Returns registry-resolved widgets, literal options, geometry evidence, source spans, conflicts, and explicit uncertainty.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: crate::tk::tk_layout,
    },
    ToolDef {
        name: "irule_cfg_paths",
        description: "Enumerate CFG paths through an iRule to terminal actions (pool/reject/redirect/…), each annotated with priority, relevant taint warnings, and test-generation questions.",
        params: &[SRC],
        required: &["source"],
        handler: |args| Ok(crate::irule_test::irule_cfg_paths(args)),
    },
    ToolDef {
        name: "suggest_datagroup_extractions",
        description: "Scan iRules source for if/switch patterns extractable to data-groups; returns per-candidate value type, CIDR flag, body-shape, confidence, and whether the static extractor applies.",
        params: &[SRC],
        required: &["source"],
        handler: |args| Ok(crate::datagroup::suggest_datagroup_extractions(args)),
    },
    ToolDef {
        name: "generate_irule_test",
        description: "Generate a runnable iRule test scaffold (per-event + per-CFG-path cases, multi-TMM scenario) with extracted events/profiles/commands/pools/datagroups.",
        params: &[SRC],
        required: &["source"],
        handler: |args| Ok(crate::irule_gen::generate_irule_test(args)),
    },
    ToolDef {
        name: "spectcl_check",
        description: "Validate a SpecTcl (.tclspec) pack by evaluating it in the deterministic sandbox (no clock/IO, budgets, transactional registration — safe on generated packs). Returns per-command draft fields, loader notices (dropped/unknown words), hooks with family and cacheability, hooks reading past their `-inputs`, and collisions with the shipped registry; plus `load_error` (determinism denial, budget, or Tcl error — nothing loads), `target_dependent` (`available?` was queried), `provenance` (what `tier`/`trust` resolve to), `untrusted_tier_refusal` (what an untrusted install at `tier` would refuse the pack for), `dormant_hooks` (hook bodies that stay dormant under that provenance — declarative facts install regardless; empty when that provenance refuses the pack, since nothing of it loads), and `stamp_refusals` (codegen_hook, inline_codegen_hook, or semantic_operation Intrinsic rows that install drops: only a bundled pack may stamp, and only as its `alias_of` target's own). Use spectcl_expand to see what a templated pack registered.",
        params: &[
            (
                "source",
                "string",
                "SpecTcl pack source text (the contents of a .tclspec file)",
            ),
            (
                DIALECT_PARAM,
                "string",
                "Dialect whose shipped registry to check names against; defaults to the session dialect",
            ),
            TIER,
            TRUST,
        ],
        required: &["source"],
        handler: |args| Ok(crate::spectcl::spectcl_check(args)),
    },
    ToolDef {
        name: "spectcl_expand",
        description: "Expand a SpecTcl (.tclspec) pack to canonical form: evaluate it in the sandbox and return the straight-line source of every registration it made (no proc/foreach/set). Review the expansion, not the loop, before shipping a templated pack and after every template edit; a straight-line pack comes back as itself. Also reports target-dependence (`available?` queried) and load notices. Hand the pack to spectcl_check for field-level validation.",
        params: &[(
            "source",
            "string",
            "SpecTcl pack source text (the contents of a .tclspec file)",
        )],
        required: &["source"],
        handler: |args| Ok(crate::spectcl::spectcl_expand(args)),
    },
    ToolDef {
        name: "spec_import",
        description: "Derive version ranges for a Tcl package's commands from LOCAL release snapshots (directories, .zip, or .tar.gz — never fetches): drafts each release, diffs them, and returns the rendered .tclspec pack (evidence in its header), per-command introduced/retired versions, and every contradiction. Obtain sources first with `tcl spec import --github OWNER/REPO` or git; hand the pack to spectcl_check.",
        params: &[
            (
                "snapshots",
                "array",
                "One entry per release, each {\"version\": \"1.2\", \"path\": \"/path/to/that/release\"} where path is a local directory, .zip or .tar.gz; order does not matter",
            ),
            (
                DIALECT_PARAM,
                "string",
                "Dialect every snapshot is analysed as; defaults to the session dialect",
            ),
            (
                "package",
                "string",
                "Pack name for the rendered speclib block; defaults to the name the sources `package provide`",
            ),
            (
                "complete_history",
                "boolean",
                "True only when the snapshots are EVERY release: presence in the earliest one is then an introduction. Defaults to false, which leaves introduced_version unset there",
            ),
        ],
        required: &["snapshots"],
        handler: crate::spec_import::spec_import,
    },
];

/// The four discovery-tier spellings [`TIER`] advertises — each tier's own
/// label, owned beside the argument's reader in `spectcl.rs`.
const TIER_VALUES: [&str; 4] = crate::spectcl::TIER_ARGUMENTS;

/// The two [`tcl_dialect::model::WorkspaceTrust`] spellings [`TRUST`]
/// advertises.
const TRUST_VALUES: [&str; 2] = ["trusted", "untrusted"];

/// The schema for a property whose value is one of a fixed, small word list —
/// a JSON-Schema `enum`, the same shape [`dialect_schema`] gives the dialect
/// property, without that property's dynamic catalogue lookup.
fn closed_string_schema(desc: &str, values: &[&str]) -> Value {
    json!({ "type": "string", "description": desc, "enum": values })
}

/// The JSON-Schema input-schema object (`{type, properties, required}`) for a
/// tool's parameters.
pub(super) fn input_schema(t: &ToolDef) -> Value {
    let mut properties = Map::new();
    for (name, ty, desc) in t.params {
        let schema = if *name == DIALECT_PARAM {
            dialect_schema(desc)
        } else if *name == TIER_PARAM {
            closed_string_schema(desc, &TIER_VALUES)
        } else if *name == TRUST_PARAM {
            closed_string_schema(desc, &TRUST_VALUES)
        } else {
            json!({ "type": ty, "description": desc })
        };
        properties.insert((*name).to_owned(), schema);
    }
    json!({
        "type": "object",
        "properties": Value::Object(properties),
        "required": t.required,
    })
}
