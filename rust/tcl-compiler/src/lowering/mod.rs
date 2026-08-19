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

//! Lower Tcl source to structured analysis IR.
//!
//! Translates a flat token stream (via the segmenter) into the tree of
//! IR nodes defined in [`crate::ir`]. Each Tcl command is pattern-matched by
//! name and converted to a typed IR statement.

use std::collections::{HashMap, HashSet};

use tcl_lexer::TokenType;
use tcl_registry::events::{IrulesCommandPlacement, IrulesExecutionContext};
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::prelude::DialectSet;
use tcl_registry::{ArgRole, CommandRegistry};

use crate::alias::{CommandAliasMap, detect_interp_alias, detect_rename, resolve_alias};
use crate::ir::{
    CommandTokens, ForeachIterator, MethodDef, MethodKind, Module, Procedure, Script, Statement,
};
use crate::lowering_hooks::{ArgTokenKind, LoweringCommand, try_lower_hook};
use crate::naming::normalise_var_name;
use crate::segmenter::{
    SegmentedCommand, segment_commands, segment_commands_with_offset_and_config,
};

pub(crate) mod hooks;
mod structured;

/// Stand-in `Script` for a body past [`MAX_LOWER_NEST_DEPTH`]: a single
/// [`Statement::Barrier`] spanning `[base_offset, base_offset + len)`, so
/// downstream passes treat the unanalysed region as having unknown effects
/// (matching how a dynamic `eval`/`uplevel` body is already modelled)
/// rather than as empty/dead code.
fn over_depth_script(base_offset: u32, len: usize) -> Script {
    let end = base_offset.saturating_add(u32::try_from(len).unwrap_or(u32::MAX));
    Script::from_statements(vec![Statement::Barrier {
        span: tcl_lexer::Span::new(base_offset, end),
        reason: "nesting depth exceeds analysis limit".to_owned(),
        command: String::new(),
        canonical_command: None,
        args: Vec::new(),
        tokens: None,
    }])
}

/// Map token kind to the simplified `ArgTokenKind` used by lowering hooks.
fn arg_token_kind(kind: TokenType) -> ArgTokenKind {
    match kind {
        TokenType::Str => ArgTokenKind::Str,
        TokenType::Esc => ArgTokenKind::Esc,
        TokenType::Cmd => ArgTokenKind::Cmd,
        TokenType::Var => ArgTokenKind::Var,
        _ => ArgTokenKind::Other,
    }
}

/// Join a parent namespace with a child name.
fn join_namespace(parent: &str, child: &str) -> String {
    tcl_syntax::naming::qualify(parent, child)
}

/// The namespace a procedure body lowers in — everything up to the
/// last `::` of its qualified name, or `::` for a global proc.
fn proc_namespace(qname: &str) -> String {
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qname);
    if holder.is_empty() {
        "::".to_string()
    } else {
        holder.to_string()
    }
}

/// One statically-extractable definer invocation, classified from the
/// command's **registry spec** (issue #1172) — never a spelling match: the
/// definer's member grammar plus which argv words carry the definition
/// target and the static braced body.
///
/// Covers every command whose spec hangs a
/// [`tcl_registry::definer::DefinitionBodyGrammar`] off
/// `CommandSpec::definition_body`: the `TclOO` metaclasses (`oo::class`,
/// `oo::configurable`, `oo::abstract`, `oo::singleton` — issue #797), the
/// `oo::define` / `oo::objdefine` script forms, snit's `type` / `widget` /
/// `widgetadaptor`, and itcl's `itcl::class`.  A new definer added to the
/// registry is picked up here with no lowering change.
#[derive(Clone, Copy)]
struct DefinerCall {
    /// The definer's member grammar (registry data).
    grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
    /// Full-argv index of the class / type / object-handle word.
    name_idx: usize,
    /// Full-argv index of the static braced definition body.
    body_idx: usize,
    /// `oo::objdefine`: the target is an *object handle*, so members and
    /// `variable` declarations are **per-object** state (oracle case K of
    /// issue #1129: `oo::objdefine $k { variable z }` then
    /// `my variable z; info exists z` → `0` — per-object, not per-class).
    /// They home under a synthetic `::@objdefine@::…` class name and never
    /// join the real class's cross-block `class_instance_vars` union.
    per_object: bool,
}

impl Lowerer<'_> {
    /// Whether this concrete invocation is declared to initialise an unset
    /// target under the lowerer's active dialect profile.
    ///
    /// This is registry-driven: command form and Tcl release decide the
    /// result, not a consumer-side command-name or read-modify-write rule.
    fn safe_on_uninit(&self, command: &str, args: &[String]) -> bool {
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let dialect = self.registry.own_availability_mask();
        // A profile-less registry is an intentionally dialect-blind union.
        // `safe_on_uninit` is a release/runtime guarantee, so the union cannot
        // prove it: in particular, `incr` differs between Tcl 8.4 and 8.5.
        // Abstain until a concrete profile selects the applicable fact.
        if dialect.is_empty() {
            return false;
        }
        self.registry
            .resolve_invocation(command, &arg_refs, dialect)
            .and_then(|resolved| resolved.semantics.safe_on_uninit)
            .is_some_and(|allowed| allowed.is_empty() || allowed.intersects(dialect))
    }

    /// Classify one command as a statically-extractable definer call, from
    /// its registry spec (see [`DefinerCall`]).  `None` when the command is
    /// not a definer, uses a non-`create` metaclass form, or does not carry
    /// a static braced body at the expected position (the single-member
    /// `oo::define Cls method m {…} {…}` inline form and dynamic-body forms
    /// stay with the default lowering).
    fn classify_definer_call(
        &self,
        command: &str,
        canonical: Option<&str>,
        texts: &[String],
        kinds: &[TokenType],
        single: &[bool],
    ) -> Option<DefinerCall> {
        let spec = self.registry.get(canonical.unwrap_or(command))?;
        let grammar = spec.definition_body?;
        // A metaclass's registry manufacturer descriptor supplies both the
        // static class-name word and definition-body word. Every other
        // definer is `DEFINER TARGET {body}`. Auto-naming manufacturers and
        // forms without a definition body stay with the default lowering.
        let (name_idx, body_idx) = if spec
            .traits
            .contains(tcl_registry::prelude::Traits::IS_OO_METACLASS)
        {
            let method = self
                .registry
                .exported_manufacturer_method(canonical.unwrap_or(command), texts.get(1)?)?;
            (
                usize::from(method.names_instance_at?) + 1,
                usize::from(method.definition_body_at?) + 1,
            )
        } else {
            (1usize, 2usize)
        };
        if texts.len() <= body_idx || !word_is_static_braced(kinds, single, body_idx) {
            return None;
        }
        // Per-object vs per-class is dispatched on the spec's typed analyser
        // hook ID — the same registry datum the analyser dispatches on —
        // never the spelling.
        let per_object =
            spec.analyser_hook == Some(tcl_registry::hooks::AnalyserHookId::OoObjdefine);
        Some(DefinerCall {
            grammar,
            name_idx,
            body_idx,
            per_object,
        })
    }
}

/// True iff `command` (`::`-stripped) is `namespace` and the args are
/// the `eval CHILD {static-braced-body}` shape — the form whose body
/// the Rust lowerer evaluates inline and discards (it emits a
/// `Barrier`), so the OO post-pass must re-segment it to find any
/// classes defined directly inside the namespace.
fn is_namespace_eval_shape(
    command: &str,
    texts: &[String],
    kinds: &[TokenType],
    single: &[bool],
) -> bool {
    command.strip_prefix("::").unwrap_or(command) == "namespace"
        && texts.len() >= 4
        && texts.get(1).is_some_and(|s| s == "eval")
        && word_is_static_braced(kinds, single, 3)
}

/// True iff word `idx` (full argv index) is a single braced-literal
/// (`Str`) token.
fn word_is_static_braced(kinds: &[TokenType], single: &[bool], idx: usize) -> bool {
    idx < kinds.len() && idx < single.len() && single[idx] && kinds[idx] == TokenType::Str
}

/// `SegmentedCommand` variant of [`word_is_static_braced`]: word
/// `idx` is a single braced-literal (`Str`) token.
fn seg_word_is_static_braced(seg: &SegmentedCommand, idx: usize) -> bool {
    seg.single_token_word.get(idx).copied().unwrap_or(false)
        && seg.argv.get(idx).is_some_and(|t| t.kind == TokenType::Str)
}

/// Word `idx` is a single substitution-free literal: a `{braced}` (`Str`) word
/// or a plain bareword (`Esc`) — exactly the body words C's `TclCompileIfCmd`
/// inlines. A word carrying `$var` / `[cmd]` substitution, or a multi-token
/// concatenation like `$x1$x2`, is not (it must be substituted then evaluated
/// as a script by the runtime `if` command).
fn seg_word_is_static_literal(seg: &SegmentedCommand, idx: usize) -> bool {
    seg.single_token_word.get(idx).copied().unwrap_or(false)
        && seg
            .argv
            .get(idx)
            .is_some_and(|t| matches!(t.kind, TokenType::Str | TokenType::Esc))
}

/// Whether word `idx` runs a `$var` / `[cmd]` substitution when it is
/// evaluated — so its value is run-time data rather than the text as written.
///
/// Weaker than the negation of [`seg_word_is_static_literal`], and
/// deliberately so: a **compound** literal word (`{body}x`, two literal tokens
/// welded together) is still spelled out in the source, so a body walk that
/// clamps it with [`crate::segmenter::body_text_in_region`] rebases honest
/// spans (issue #1325). `{*}$n` under an expansionless grammar looks the same
/// from the representative token alone — braced, multi-token — yet its second
/// fragment substitutes, so its value appears nowhere in the document.
///
/// Falls back to the representative token kind when the per-word fragments are
/// unavailable, matching the coarse view the other word gates here take.
fn seg_word_substitutes(seg: &SegmentedCommand, idx: usize) -> bool {
    match seg.word_fragments.get(idx) {
        Some(fragments) if !fragments.is_empty() => fragments
            .iter()
            .any(|fragment| matches!(fragment.token.kind, TokenType::Var | TokenType::Cmd)),
        _ => seg
            .argv
            .get(idx)
            .is_some_and(|tok| matches!(tok.kind, TokenType::Var | TokenType::Cmd)),
    }
}

/// True iff `nm` is a static instance-variable name (not a `$var` /
/// `[cmd]` substitution and not an option flag). Dynamic names are
/// skipped — conservative.
fn is_instance_var_name(nm: &str) -> bool {
    !nm.is_empty() && !nm.contains('$') && !nm.contains('[') && !nm.starts_with('-')
}

/// True iff a statement's command (preferring its canonical form,
/// `::`-stripped) equals `bare`, tolerating the IR's
/// `canonical_command == None` for un-aliased commands.
fn canonical_matches(command: &str, canonical: Option<&str>, bare: &str) -> bool {
    let c = canonical.unwrap_or(command);
    c.strip_prefix("::").unwrap_or(c) == bare
}

/// Qualify a procedure name relative to a namespace.
fn qualify_proc_name(namespace: &str, proc_name: &str) -> String {
    tcl_syntax::naming::qualify(namespace, proc_name)
}

/// The namespace a `proc`'s **body** resolves names against: the namespace the
/// proc is *defined in* (the qualifier prefix of its own qualified name), not
/// the lexical namespace the `proc` command was written in.
///
/// The two agree for every proc whose name word carries no qualifier — the
/// overwhelmingly common case, including `proc outer {} { proc inner … }` at
/// top level — and diverge exactly when the name word is itself qualified, so
/// the definition site and the defining namespace are different namespaces.
///
/// Oracle, identical on tclsh 9.0.4 and 8.6.16 (`self class`-style
/// introspection is not involved; this is plain `info commands` after
/// running the outer proc):
///
/// ```tcl
/// namespace eval ::a {}
/// proc a::outer {} { proc helper {x} { return $x } ; return [namespace current] }
/// a::outer                        ;# -> ::a           (the body's frame namespace)
/// info commands ::helper          ;# -> {}            (NOT the lexical ::)
/// info commands ::a::helper       ;# -> ::a::helper
///
/// namespace eval ::b::c {}
/// proc b::c::o2 {} { proc h2 {y} { return $y } }
/// b::c::o2 ; info commands ::b::c::h2   ;# -> ::b::c::h2, and ::h2 is empty
///
/// # The *lexical* enclosing `namespace eval` loses to the name's own qualifier:
/// namespace eval ::e {}
/// namespace eval ::d { proc ::e::o3 {} { proc h3 {z} { return $z } } }
/// ::e::o3 ; info commands ::e::h3       ;# -> ::e::h3, ::d::h3 and ::h3 empty
///
/// # Nesting composes — each level re-homes to its own defining namespace:
/// namespace eval ::g {}
/// proc g::o7 {} { proc o7b {} { proc h7 {u} { return $u } } }
/// g::o7 ; ::g::o7b ; info commands ::g::h7   ;# -> ::g::h7, ::h7 empty
/// ```
///
/// An absolutely-written nested name (`proc ::abs {q} {…}`) is unaffected —
/// [`qualify_proc_name`] already short-circuits on the leading `::`.
///
/// `qualified` is always rooted (it comes from [`qualify_proc_name`], which
/// routes every result through `normalise_qualified_name`), so the holder is
/// never the empty "relative" marker [`tcl_syntax::naming::key_holder_and_tail`]
/// returns for a bare simple name; `lexical_fallback` covers that
/// impossible-by-construction case rather than silently producing `""`.
fn proc_body_namespace(qualified: &str, lexical_fallback: &str) -> String {
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qualified);
    if holder.is_empty() {
        lexical_fallback.to_string()
    } else {
        holder.to_string()
    }
}

/// Collapse the one substitution a braced word permits before the word's text
/// is parsed as a list.
///
/// Both list words below arrive as brace-quoted source, so a `\<newline>` line
/// continuation is still spelled out in the text the segmenter hands over —
/// without collapsing it a wrapped var list (`foreach {a b\<nl>  c} …`) or a
/// wrapped `proc` parameter list would keep the `\` glued to the preceding name
/// (`b\`) and bind the wrong variable.
fn collapse_list_word(word_text: &str) -> std::borrow::Cow<'_, str> {
    tcl_syntax::backslash::collapse_brace_continuations_str(word_text)
}

/// Split a Tcl *variable list* word into the variable names it binds.
///
/// This is the `foreach` / `lmap` varList word, and the identically-shaped
/// varLists of `dict for|map`, `array for`, and a `try on|trap` handler. Such a
/// word is a plain Tcl list, so grouping binds *one* variable whose name
/// contains the separator: tclsh 9.0 binds the single name `a b` for both
/// `foreach {{a b}} …` and `foreach {a\ b} …`, where a whitespace split would
/// invent two names (or drop one). [`tcl_syntax::list::split_list`] owns that
/// grammar, and it is what the analyser's `define_vars_from_list` already uses
/// for the same word, so lowering and analysis agree by construction.
///
/// `None` means the word is not a well-formed list (an unmatched brace or
/// quote). Callers fall back to the runtime command, which raises Tcl's own
/// error, rather than guessing at a binding.
fn parse_var_list_names(list_text: &str) -> Option<Vec<String>> {
    let collapsed = collapse_list_word(list_text);
    let names = tcl_syntax::list::split_list(&collapsed).ok()?;
    Some(
        names
            .into_iter()
            .map(std::borrow::Cow::into_owned)
            .collect(),
    )
}

/// Split a Tcl *formal parameter* list word into its parameter names.
///
/// This is the `proc` / `apply`-lambda / TclOO-method parameter word, a
/// different grammar from [`parse_var_list_names`]: it has two list levels, the
/// outer one holding a specifier per parameter and each specifier itself a
/// `name ?default?` list. So `{x {y 2} args}` is the three parameters `x`, `y`
/// (defaulting to `2`), and `args`; and `proc p {a\ b} {}` is the *single*
/// parameter `a` defaulting to `b` (tclsh 9.0: `info args p` → `a`), not the
/// two names a whitespace split would produce.
/// [`tcl_syntax::formal_params::parse_formal_parameters`] owns that grammar and
/// is what the VM, the WASM runtime, and `signature_scan` all decode with.
///
/// Only the names reach the IR — [`Procedure::params`] is name-only, and
/// `params_raw` keeps the source text for consumers that need the defaults.
///
/// `None` means Tcl itself would refuse to create the procedure (a malformed
/// list, a specifier with three or more fields, an array-element or qualified
/// name), so the caller defers to the runtime command that reports it.
fn parse_formal_param_names(param_text: &str) -> Option<Vec<String>> {
    let collapsed = collapse_list_word(param_text);
    let parameters = tcl_syntax::formal_params::parse_formal_parameters(&collapsed).ok()?;
    Some(
        parameters
            .into_iter()
            .map(|parameter| parameter.name)
            .collect(),
    )
}

/// Return `Some(true)` / `Some(false)` if *`expr_text`* is a
/// literal Tcl boolean, or `None` when the condition is not a
/// simple literal. Tolerates surrounding whitespace and is
/// case-insensitive (matches `Tcl_GetBoolean`).
pub(crate) fn static_bool(expr_text: &str) -> Option<bool> {
    let stripped = expr_text.trim();
    match stripped {
        "0" => Some(false),
        "1" => Some(true),
        // Boolean words, incl. unique-prefix spellings (`tr`, `ye`, `of`).
        _ => tcl_syntax::boolean::parse_boolean_word(stripped),
    }
}

/// An `uplevel` level argument, parsed.
///
/// The two forms address different frames and must not be conflated:
/// `#N` counts *down* from the global frame, `N` counts *up* from the
/// current one. They coincide only when the current frame is the
/// global frame, which static analysis cannot assume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UplevelLevel {
    /// The level number as written, without the `#`.
    shift: i32,
    /// `true` for the `#N` absolute form, `false` for the relative form.
    absolute: bool,
}

/// Parse the level argument of an `uplevel` call. Accepts the
/// canonical relative form (`uplevel 1 body`, `uplevel 3 body`) and
/// the absolute form (`#0` / `#N`), returning `None` when the argument
/// is dynamic (`$lvl`, `[expr {...}]`) or otherwise unparseable.
///
/// `#0` and `0` both yield a `shift` of `0` and are told apart by
/// `absolute` — `uplevel #0 {…}` runs the body in the global frame
/// while `uplevel 0 {…}` runs it in the current one
/// (tclsh8.6/9.0-confirmed).
fn parse_uplevel_level(text: &str) -> Option<UplevelLevel> {
    if let Some(rest) = text.strip_prefix('#') {
        return rest.parse::<i32>().ok().map(|shift| UplevelLevel {
            shift,
            absolute: true,
        });
    }
    text.parse::<i32>().ok().map(|shift| UplevelLevel {
        shift,
        absolute: false,
    })
}

/// Return `(name, literal)` when *seg* is `set name {literal}`.
///
/// The LHS must be a plain bareword `Esc` token (no substitutions,
/// array index, or namespace qualifier) so we only ever track
/// proc-local scalars. The RHS must be a single brace-string `Str`
/// token.
fn set_literal_body(seg: &SegmentedCommand) -> Option<(String, String)> {
    if seg.name() != "set" || seg.args().len() != 2 {
        return None;
    }
    let arg_tokens = seg.arg_tokens();
    if arg_tokens.len() < 2 {
        return None;
    }
    if !seg.single_token_word.iter().take(3).all(|&b| b) {
        return None;
    }
    let name_tok = arg_tokens[0];
    let value_tok = arg_tokens[1];
    if value_tok.kind != TokenType::Str {
        return None;
    }
    if name_tok.kind != TokenType::Esc {
        return None;
    }
    let name = &seg.args()[0];
    if name.is_empty()
        || name.contains('$')
        || name.contains('[')
        || name.contains('(')
        || name.contains("::")
    {
        return None;
    }
    let value = seg.args()[1].clone();
    Some((normalise_var_name(name).to_string(), value))
}

/// If *`cmd_text`* is `list lit1 lit2 ...` with all-literal
/// arguments, return the body text the list would evaluate to —
/// otherwise `None`.
///
/// Literal means `Esc` / `Str` token only: no `$var` substitution,
/// no nested command substitution. `Str` (`{...}`) tokens are
/// re-braced in the synthesised body so list-canonicalisation
/// stays correct (we trust the segmenter's `single_token_word`
/// flag plus the absence of `$` / `[` in `Esc` text).
fn eval_list_literal_body(cmd_text: &str) -> Option<String> {
    let inner = segment_commands(cmd_text);
    if inner.len() != 1 {
        return None;
    }
    let inner_cmd = &inner[0];
    if inner_cmd.texts.is_empty() || inner_cmd.texts[0] != "list" {
        return None;
    }
    let argv = inner_cmd.arg_tokens();
    let texts = inner_cmd.args();
    let single = &inner_cmd.single_token_word;
    // Each element after ``list`` must be a single-token literal.
    for (i, tok) in argv.iter().enumerate() {
        // ``single`` is per-word over the full argv (including
        // the command word at index 0) — argv here starts at
        // index 1 of the original argv, so the matching single-
        // token-word index is ``i + 1``.
        if !single.get(i + 1).copied().unwrap_or(false) {
            return None;
        }
        if !matches!(tok.kind, TokenType::Esc | TokenType::Str) {
            return None;
        }
        if tok.kind == TokenType::Esc {
            let text = &texts[i];
            if text.contains('$') || text.contains('[') {
                return None;
            }
        }
    }
    let mut parts: Vec<String> = Vec::with_capacity(argv.len());
    for (i, tok) in argv.iter().enumerate() {
        let text = &texts[i];
        if tok.kind == TokenType::Str {
            parts.push(format!("{{{text}}}"));
        } else {
            parts.push(text.clone());
        }
    }
    Some(parts.join(" "))
}

/// Drop const-map entries that *stmt* may have overwritten.
/// Straight-line assignments pop just the named variable;
/// structured IR and barriers conservatively clear the whole map
/// because their child scopes (or runtime side effects) could
/// touch any tracked name.
/// Token-level check that a relaxed-eval /
/// relaxed-uplevel body is free of nested dynamic-shape barriers.
///
/// When the eval/uplevel hooks consider relaxing a braced-literal
/// body to inline IR, they first walk the body's command words and
/// reject any nested `eval`/`uplevel` whose own body argument is
/// substitution-bearing (`$var` / `[cmd]` / multi-token).  Without
/// this gate, a static braced `eval {uplevel 1 $x}` would relax to
/// IR that runs a compiled `uplevel` with a still-dynamic body.
///
/// The walk is deliberately shallow and token-based:
///
/// 1. Segment the body into commands.
/// 2. For each command the registry marks as evaluating code, ask the
///    registry which words its script is made of and poison unless
///    every one of them is a braced literal (`Str`) word.
/// 3. Recurse into nested braced bodies and into braced-arg shapes
///    of non-barrier commands so a nested
///    `if { … } { eval $x }` still trips the gate.
///
/// Which word is the script is **never** re-derived here (issue #1055):
/// [`CommandRegistry::arg_indices_for_role`] answers it, so `uplevel`'s
/// optional-level shape is resolved by the one contract-tested
/// `uplevel_arg_roles` resolver instead of a second, subtly different
/// level-word sniff.  The barrier test likewise composes subcommand
/// traits ([`CommandRegistry::invocation_traits`]), so the compound
/// eval-family members — `namespace eval`, `namespace inscope`,
/// `interp eval`, all of which carry the eval-family bits on the
/// *subcommand* — flow through the same path a bare `eval` does.
///
/// Returns `true` when the body contains a nested dynamic-shape
/// barrier (the caller should fall back to [`Statement::Barrier`]); `false`
/// when the body is safe to relax.
fn body_has_dynamic_barrier(body_text: &str, registry: &CommandRegistry) -> bool {
    use tcl_lexer::TokenType;
    use tcl_registry::prelude::Traits;
    let commands = segment_commands(body_text);
    for sc in &commands {
        if sc.argv.is_empty() || sc.texts.is_empty() {
            continue;
        }
        let raw_name = sc.texts[0].as_str();
        // Strip any leading `::` so the fully-qualified spellings resolve too.
        let name = raw_name.strip_prefix("::").unwrap_or(raw_name);
        let args: Vec<&str> = sc.texts[1..].iter().map(String::as_str).collect();
        // A "dynamic barrier" command evaluates a script (`eval`,
        // `uplevel`, `namespace eval`, `interp eval`, …).  Sourced from
        // the registry's `EVALUATES_CODE` trait rather than a hardcoded
        // name list.  `DialectSet::empty()` because the question is the
        // command's *shape*, not its availability: a barrier is a barrier
        // whichever dialect the file is analysed as.
        let traits = registry.invocation_traits(name, &args, DialectSet::empty());
        if !traits.contains(Traits::EVALUATES_CODE) {
            // Recurse into braced args of non-barrier commands so
            // nested barriers still trip the gate.
            for (i, tok) in sc.argv.iter().enumerate() {
                if i == 0 {
                    continue;
                }
                if tok.kind != TokenType::Str {
                    continue;
                }
                let arg_text = sc.texts.get(i).map_or("", String::as_str);
                if body_has_dynamic_barrier(arg_text, registry) {
                    return true;
                }
            }
            continue;
        }
        // Name is a barrier — inspect the words its script is made of.
        let body_indices = registry.arg_indices_for_role(name, &args, ArgRole::Body);
        let Some(&first_body) = body_indices.first() else {
            // The registry can point at no script word: a malformed call
            // (`uplevel 1` — a wrong-#args error) or an evaluator whose
            // code argument is a command *prefix* rather than a script
            // (`coroprobe` / `coroinject`).  Poison so the outer hook
            // falls back to Statement::Barrier and the runtime decides.
            return true;
        };
        // `SCRIPT_CONCATENATES_ARGS`: the trailing words concatenate into
        // the one script Tcl evaluates, so `uplevel 1 {set x 1} $tail`
        // is dynamic even though the marked body word is a literal.  Check
        // the whole tail from the first script word, not just the marked
        // ones.
        let script_words: Vec<usize> = if traits.contains(Traits::SCRIPT_CONCATENATES_ARGS) {
            (first_body..args.len()).collect()
        } else {
            body_indices
        };
        for idx in script_words {
            // `sc.argv` / `sc.texts` include the command name at 0, so the
            // arg-relative index shifts by one.
            let (Some(tok), Some(text)) = (sc.argv.get(idx + 1), sc.texts.get(idx + 1)) else {
                return true;
            };
            if tok.kind != TokenType::Str {
                return true;
            }
            // Recurse into the literal nested script word.
            if body_has_dynamic_barrier(text, registry) {
                return true;
            }
        }
    }
    false
}

fn invalidate_const_map_for(stmt: &Statement, scope: &mut HashMap<String, String>) {
    match stmt {
        Statement::AssignConst { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::Incr { name, .. } => {
            scope.remove(normalise_var_name(name));
        }
        Statement::Call { defs, .. } => {
            for v in defs {
                scope.remove(normalise_var_name(v));
            }
        }
        Statement::Barrier { .. }
        | Statement::Block { .. }
        | Statement::UpFrame { .. }
        | Statement::If { .. }
        | Statement::For { .. }
        | Statement::While { .. }
        | Statement::Foreach { .. }
        | Statement::Catch { .. }
        | Statement::Try { .. }
        | Statement::Switch { .. } => {
            scope.clear();
        }
        Statement::Return { .. } | Statement::ExprEval { .. } => {}
    }
}

/// A memoised per-procedure body-lowering callback: `(offset-0 body text,
/// namespace) → offset-0 body [`Script`]`.  See [`Lowerer::with_body_cache`].
type BodyCacheFn<'a> = dyn Fn(&str, &str) -> Script + 'a;

/// Which lowering pass a [`Lowerer`] performs.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CompileTarget {
    /// The analysis IR: structured control flow kept everywhere, every
    /// registry-driven hook active. What LSP consumers (diagnostics,
    /// navigation) lower against.
    #[default]
    Analysis,
    /// The VM-faithful bytecode pass ([`lower_to_ir_for_bytecode`]):
    /// constructs the bytecode backend can't compile correctly (a bare
    /// `try`, a directly-nested `foreach`/`lmap`) barrier to a runtime
    /// dispatch instead of their structured IR; every other hook active.
    Bytecode,
    /// The bytecode pass with every registry-driven inline/structured
    /// lowering hook ALSO suppressed ([`lower_to_ir_traced`]): every command
    /// falls straight to [`Lowerer::lower_default`] — a plain runtime
    /// dispatch (`Statement::Call`/`Statement::Barrier` with a non-empty
    /// `command`, both of which [`crate::codegen`] emits as `push words;
    /// INVOKE`), never a typed statement (`AssignConst`/`Incr`/`Return`/…)
    /// or a structured control-flow block (`If`/`While`/`Foreach`/`Switch`/
    /// `Catch`/`Try`/…). Used to recompile a proc/script body so execution
    /// traces (`enterstep`/`leavestep`) observe every command in it — the
    /// compiler-side half of issue #946's step-trace parity fix; see
    /// [`tcl_runtime_api::CompileService::compile_traced`]. Never set for
    /// the primary module compile — only for a standalone "compile this body
    /// text as a script" call, so a proc's own `proc name args body`
    /// DEFINITION always module-compiles normally; only its *body* is
    /// affected when recompiled under this target.
    BytecodeTraced,
}

impl CompileTarget {
    /// Whether this pass targets the bytecode backend (barriers
    /// backend-incompatible structured forms) — [`Self::Bytecode`] or
    /// [`Self::BytecodeTraced`].
    pub(crate) fn is_bytecode(self) -> bool {
        matches!(self, Self::Bytecode | Self::BytecodeTraced)
    }

    /// Whether this pass suppresses every inline/structured hook —
    /// [`Self::BytecodeTraced`] only.
    pub(crate) fn is_trace_visible(self) -> bool {
        self == Self::BytecodeTraced
    }
}

/// The lowering engine — accumulates procedures and IR statements.
pub struct Lowerer<'r> {
    /// Output module being built.
    pub module: Module,
    /// Command alias table built during lowering.
    aliases: CommandAliasMap,
    /// `class qualified name -> every instance variable declared for it`,
    /// accumulated across **all** of that class's definition blocks (issue
    /// #1096/#1097 review, finding 1).
    ///
    /// A class's state is not confined to the block that created it: `oo::class
    /// create C { method m … }` followed by `oo::define C { variable x }`
    /// makes `x` instance state for `m`, which was extracted before the
    /// declaration was seen.  Each block's [`Self::extract_oo_methods`] run
    /// computes only its *own* block's declarations, so the union is
    /// accumulated here and merged into every method of the class by
    /// [`Self::extract_oo_methods_pass`] once all blocks have been walked.
    /// Declaring `variable x` anywhere makes it instance state for every
    /// method of that class, so the union is order-free.
    class_instance_vars: HashMap<String, HashSet<String>>,
    /// Event handler occurrence counts (for `when` numbering).
    when_counts: HashMap<String, u32>,
    /// The registry-owned iRules execution placement of the script currently
    /// being lowered.  This is deliberately distinct from `proc_depth`:
    /// control-flow bodies inherit an event/procedure context, whereas a body
    /// opened by a top-level executable statement is invalid declaration
    /// territory and must not manufacture `when` / `proc` procedures.
    irules_execution_context: IrulesExecutionContext,
    /// Priority inherited by subsequent top-level iRules event declarations.
    irules_priority: u16,
    /// Monotonic counter for synthetic *body unit* names (`apply` lambdas and
    /// `namespace eval` bodies), so each registers under a unique qualified
    /// name in [`Module::body_units`]. Deterministic in source order, so the
    /// fresh and incremental lowerings produce identical names.
    body_unit_count: u32,
    /// Whether we're inside a `namespace eval` body.
    in_namespace_eval: bool,
    /// Command registry for arg-role queries.
    registry: &'r CommandRegistry,
    /// Per-script const-map stack. Each scope tracks
    /// proc-local variables assigned a brace-string literal so
    /// later `eval $var` / `uplevel 1 $var` calls can fold the
    /// body in at lowering time. Active only when
    /// `proc_depth > 0` — top-level / `namespace eval` scopes
    /// write globals or namespace vars whose values can be
    /// observed and mutated by other code, so const-propagating
    /// them is unsound.
    const_map_stack: Vec<HashMap<String, String>>,
    /// Depth of `proc` / `when` body lowerings currently in
    /// flight. A positive value enables the const-map.
    proc_depth: u32,
    /// `namespace import` directives observed at lowering time.
    /// Recorded as `(context_namespace, absolute_pattern)`
    /// pairs and copied into `Module::namespace_imports` at the
    /// end of lowering. Order is preserved.
    namespace_imports: Vec<(String, String)>,
    /// `namespace export` directives observed at lowering time.
    /// Recorded as `(context_namespace, pattern)` pairs
    /// and copied into `Module::namespace_exports` at the end of
    /// lowering. Order is preserved.
    namespace_exports: Vec<(String, String)>,
    /// Depth of statically-dead branches currently being lowered.
    /// `if {0} {…}` / `if {1} {…} else {…}` bump this
    /// around the dead body so any `namespace import` /
    /// `namespace export` directives found inside don't register
    /// with the module-level tables. The IR for the dead code is
    /// still produced so consumers that walk the tree by syntactic
    /// offset see the original structure.
    pub(crate) dead_code_depth: u32,
    /// `true` while lowering a `TclOO` method body. A `proc`
    /// (or `namespace eval`-lifted proc) defined inside a method
    /// body is created at method-call time in the global namespace,
    /// NOT at class-definition time — so it must not be lifted into
    /// `module.procedures` (codegen would otherwise emit it
    /// unconditionally at script load). The method body is still
    /// lowered for analysis; only the global registration is
    /// suppressed.
    suppress_proc_register: bool,
    /// Lexer config for the document's dialect, threaded into every
    /// body re-segmentation so `{*}` expansion (off for Tcl 8.4 /
    /// iRules) and the iRules `}{` ghost SEP are honoured rather than
    /// always assuming the Tcl-8.5+ default.  Defaults to
    /// `LexerConfig::default()`; production callers thread the active
    /// dialect via [`Lowerer::with_config`] / [`lower_to_ir_with_config`].
    config: tcl_lexer::LexerConfig,
    /// The document's analysis dialect (`None` for plain Tcl), threaded into
    /// every `expr` / condition parse so a dialect-only operator — an iRules
    /// word operator such as `contains` or `starts_with` — is parsed as that
    /// operator instead of degrading to
    /// [`crate::expr_ast::ExprNode::Raw`], which no downstream fold can
    /// evaluate.  Separate from `config`: the lexer config carries the
    /// dialect's *word* tokenisation, this carries its *expression* grammar.
    /// Set by [`Lowerer::with_dialect`] / [`lower_to_ir_with_dialect`].
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    /// Which lowering pass this instance performs — folds what would
    /// otherwise be two related bool fields (`for_bytecode`, `trace_visible`)
    /// into one three-state enum (`clippy::struct_excessive_bools`); see
    /// [`CompileTarget`].
    pub(crate) target: CompileTarget,
    /// Optional memoised per-procedure body lowering (SRV-INCREMENTAL Task 3).
    /// When set, a **top-level** `proc`'s static literal body is lowered through
    /// this callback `(offset-0 body text, namespace) -> offset-0 body Script`
    /// (the caller rebases by the body offset) instead of `lower_body`, so an
    /// unchanged proc's body IR is reused across edits.  Set only for
    /// **context-free** files (no `namespace eval`/`import`/`export`, alias,
    /// `oo::`, `when`, or nested `proc`) where isolated lowering is byte-identical;
    /// `None` ⇒ the normal whole-file lowering.  See [`lower_to_ir_with_body_cache`].
    body_cache: Option<&'r BodyCacheFn<'r>>,
    /// Current `lower_script` / `lower_body` recursion depth, bounded by
    /// [`MAX_LOWER_NEST_DEPTH`] so deeply-nested bodies cannot overflow the
    /// stack. Mirrors [`crate::cfg_builder::CfgBuilder`]'s `depth` field —
    /// lowering runs *before* CFG construction, so an unguarded recursion
    /// here reaches the same crash first and makes the CFG builder's own
    /// cap moot (issue #996).
    nest_depth: u32,
    /// The buffer every span this lowering emits indexes into — the document
    /// text for an ordinary lowering, and the *materialised literal* while a
    /// synthesised script is lowered at offset `0` inside one (the `eval
    /// $const` / factory-body paths that re-enter [`Self::lower_script`]).
    /// [`Self::lower_script`] swaps it for the duration of each such
    /// re-entry, because that is exactly where a new span space begins;
    /// [`Self::lower_body`] does not, since a body's spans are rebased into
    /// the space it inherits.
    ///
    /// Held so the lowerer can *check* a body word's rebase against the text
    /// it claims to slice ([`crate::segmenter::body_text_in_region`] in
    /// [`Self::lower_body_from_tok`]) rather than trusting the arithmetic —
    /// the guard the analyser has carried since issue #1325.  Empty until a
    /// script is lowered, which the guard reads as "nothing to compare
    /// against" and leaves every body text alone.
    source: String,
}

/// Maximum nesting depth for the recursive `lower_script` / `lower_body`
/// descent (`lower_script` ↔ `lower_body` ↔ `lower_segmented` ↔
/// `lower_command` are mutually recursive with one Rust frame group per
/// braced-body nesting level). Matches
/// [`crate::cfg_builder::MAX_LOWER_DEPTH`] and
/// `tcl_compiler::analyser::commands::MAX_BODY_DEPTH` — all three
/// independently-recursive walkers over the same source cap at the same
/// depth, so no consumer of this crate depends on one pass reaching a
/// deeper nesting level than another.
const MAX_LOWER_NEST_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(256);

impl<'r> Lowerer<'r> {
    /// Create a new lowerer with the default (Tcl-8.5+) lexer config.
    #[must_use]
    pub fn new(registry: &'r CommandRegistry) -> Self {
        Self::with_config(registry, tcl_lexer::LexerConfig::default())
    }

    /// Create a lowerer with an explicit dialect [`tcl_lexer::LexerConfig`]
    /// (see the `config` field).
    #[must_use]
    pub fn with_config(registry: &'r CommandRegistry, config: tcl_lexer::LexerConfig) -> Self {
        Self {
            module: Module::default(),
            aliases: CommandAliasMap::new(),
            class_instance_vars: HashMap::new(),
            when_counts: HashMap::new(),
            irules_execution_context: IrulesExecutionContext::TopLevel,
            irules_priority: 500,
            body_unit_count: 0,
            in_namespace_eval: false,
            registry,
            const_map_stack: Vec::new(),
            proc_depth: 0,
            namespace_imports: Vec::new(),
            namespace_exports: Vec::new(),
            dead_code_depth: 0,
            suppress_proc_register: false,
            config,
            dialect: None,
            target: CompileTarget::Analysis,
            body_cache: None,
            nest_depth: 0,
            source: String::new(),
        }
    }

    /// Set the document's analysis dialect (see [`Lowerer::dialect`]).
    ///
    /// `None` means the caller named *no* dialect — deliberately distinct
    /// from `Some(plain_tcl)`, which is an explicit plain-Tcl document. The
    /// two select different numeral sources downstream (`None` defers to the
    /// thread-ambient target; `Some` pins the profile's grammar), so a caller
    /// holding an optional ingress spelling must map the unstated case to
    /// `None` rather than defaulting to the plain profile. Matches
    /// [`crate::compilation_unit::UnitBuildOptions::dialect`].
    #[must_use]
    pub fn with_dialect(mut self, dialect: Option<&'static tcl_dialect::DialectProfile>) -> Self {
        self.dialect = dialect;
        self
    }

    /// Install a memoised per-procedure body-lowering callback (see
    /// [`body_cache`](Self::body_cache) and [`lower_to_ir_with_body_cache`]).
    #[must_use]
    pub fn with_body_cache(mut self, cache: &'r BodyCacheFn<'r>) -> Self {
        self.body_cache = Some(cache);
        self
    }

    /// Mark this as the bytecode/VM compile path, barriering constructs the
    /// backend can't compile (see [`CompileTarget::Bytecode`]). A no-op if
    /// [`Self::trace_visible`] already upgraded this to
    /// [`CompileTarget::BytecodeTraced`] (still bytecode-backend, just also
    /// trace-visible) — order-independent whichever builder call comes first.
    #[must_use]
    pub fn for_bytecode_backend(mut self) -> Self {
        if self.target == CompileTarget::Analysis {
            self.target = CompileTarget::Bytecode;
        }
        self
    }

    /// Suppress every inline/structured lowering hook so the whole lowering
    /// pass produces plain runtime dispatches (see
    /// [`CompileTarget::BytecodeTraced`]). Implies the bytecode backend.
    #[must_use]
    pub fn trace_visible(mut self) -> Self {
        self.target = CompileTarget::BytecodeTraced;
        self
    }

    /// Lower a complete source string to an IR module.
    pub fn lower(&mut self, source: &str) -> &Module {
        self.module.top_level = self.lower_script(source, "::");
        // Surface namespace import / export directives onto
        // the module for downstream consumers (codegen import
        // resolution, future warning passes).
        self.module.namespace_imports = std::mem::take(&mut self.namespace_imports);
        self.module.namespace_exports = std::mem::take(&mut self.namespace_exports);
        &self.module
    }

    /// Lower a source-text literal into a [`Script`] without
    /// installing it as the module top-level. Used by passes that
    /// need to lower a sub-script (e.g. the
    /// [`crate::inline_uplevel`] rewriter materialising a
    /// brace-literal callsite body).
    pub fn lower_into_script(&mut self, source: &str, namespace: &str) -> Script {
        self.lower_script(source, namespace)
    }

    /// Lower a source string to an IR script.
    ///
    /// Depth-guarded entry to the recursive lowering, alongside
    /// [`Self::lower_body`] — every nested body re-enters through one of
    /// these two, so bounding both here caps the whole `lower_script` ↔
    /// `lower_body` ↔ `lower_segmented` ↔ `lower_command` recursion. At the
    /// cap we stop descending and lower the remaining text as a single
    /// opaque [`Statement::Barrier`] rather than silently dropping it —
    /// downstream passes (SCCP, dead-code elimination, …) must treat
    /// unanalysed content as having unknown effects, not as empty/dead.
    fn lower_script(&mut self, source: &str, namespace: &str) -> Script {
        self.nest_depth += 1;
        if MAX_LOWER_NEST_DEPTH.exceeded(self.nest_depth) {
            self.nest_depth -= 1;
            return over_depth_script(0, source.len());
        }
        // `source` starts a span space: its statements' offsets are relative
        // to it, whether it is the document or a literal materialised inside
        // one.  Swap it in for the descent and restore the enclosing one after
        // (see [`Self::source`]).
        let enclosing_source = std::mem::replace(&mut self.source, source.to_owned());
        let result = self.lower_script_inner(source, namespace);
        self.source = enclosing_source;
        self.nest_depth -= 1;
        result
    }

    fn lower_script_inner(&mut self, source: &str, namespace: &str) -> Script {
        let commands = segment_commands_with_offset_and_config(source, 0, self.config);
        self.const_map_stack.push(HashMap::new());
        let stmts = self.lower_segmented(&commands, namespace);
        self.const_map_stack.pop();
        Script::from_statements(stmts)
    }

    /// Lower a body argument (inside braces/brackets) to an IR script.
    ///
    /// Inherits the parent scope's const-map so a child body can
    /// still relax its
    /// `eval` / `uplevel` against literals bound in the enclosing
    /// scope (`set body {literal}; catch {uplevel 1 $body}` is the
    /// canonical example).
    ///
    /// Depth-guarded the same way as [`Self::lower_script`] — see its doc
    /// comment.
    fn lower_body(&mut self, text: &str, base_offset: u32, namespace: &str) -> Script {
        let context = self.nested_irules_execution_context();
        self.lower_body_in_irules_context(text, base_offset, namespace, context)
    }

    /// Lower a body while explicitly preserving the registry-owned iRules
    /// execution context.  `when` / `proc` declarations use this to install
    /// their event/procedure frame; ordinary script-bearing commands use
    /// [`Self::lower_body`], which derives the nested context instead.
    fn lower_body_in_irules_context(
        &mut self,
        text: &str,
        base_offset: u32,
        namespace: &str,
        context: IrulesExecutionContext,
    ) -> Script {
        let previous = std::mem::replace(&mut self.irules_execution_context, context);
        self.nest_depth += 1;
        let result = if MAX_LOWER_NEST_DEPTH.exceeded(self.nest_depth) {
            self.nest_depth -= 1;
            over_depth_script(base_offset, text.len())
        } else {
            let result = self.lower_body_inner(text, base_offset, namespace);
            self.nest_depth -= 1;
            result
        };
        self.irules_execution_context = previous;
        result
    }

    /// Which iRules placement a nested script inherits.  A control-flow or
    /// callback body within an event/procedure keeps that live frame; a body
    /// reached from a top-level executable command remains invalid rather
    /// than gaining the power to declare a handler or procedure.
    const fn nested_irules_execution_context(&self) -> IrulesExecutionContext {
        match self.irules_execution_context {
            IrulesExecutionContext::TopLevel | IrulesExecutionContext::InvalidNestedBody => {
                IrulesExecutionContext::InvalidNestedBody
            }
            IrulesExecutionContext::EventBody => IrulesExecutionContext::EventBody,
            IrulesExecutionContext::ProcedureBody => IrulesExecutionContext::ProcedureBody,
        }
    }

    /// Run one synthesized script in a known iRules execution context.  A
    /// materialised proc body has no source token to route through
    /// [`Self::lower_body_in_irules_context`], but it still must not treat a
    /// nested declaration as a file-level one.
    fn lower_script_in_irules_context(
        &mut self,
        source: &str,
        namespace: &str,
        context: IrulesExecutionContext,
    ) -> Script {
        let previous = std::mem::replace(&mut self.irules_execution_context, context);
        let result = self.lower_script(source, namespace);
        self.irules_execution_context = previous;
        result
    }

    /// The part of a body word's `text` whose spans may honestly be rebased
    /// by that word's content offset, given the word occupies `tok`'s span in
    /// [`Self::source`].
    ///
    /// Every body lowering below adds one base offset to the spans the
    /// segmenter cuts from a word *value*, which is truthful only while that
    /// value is the source region verbatim.  A compound `{body}x` word breaks
    /// it: the value is the brace content welded to the tail with the closing
    /// `}` dropped, so every token past the drop slides one byte left — an
    /// off-by-one span on ASCII, an offset inside a UTF-8 sequence on
    /// anything else (issue #1325).  [`crate::segmenter::body_text_in_region`]
    /// clamps such a value to the contiguous braced part and passes an
    /// ordinary body through untouched; the analyser's `analyse_body` has
    /// applied it since #1325 and this is the lowering side of the same
    /// guard.
    ///
    /// A substituting word (`$body`, `[gen]`) is left alone: its value is
    /// run-time data that no source region can be compared against, so there
    /// is nothing here to prove and clamping it would only delete text.  Such
    /// words are the literal gates' business — they barrier rather than lower
    /// (issue #1375).
    fn guarded_body_text<'t>(&self, tok: tcl_lexer::Token, text: &'t str) -> &'t str {
        if matches!(tok.kind, TokenType::Var | TokenType::Cmd) {
            return text;
        }
        crate::segmenter::body_text_in_region(
            &self.source,
            (tok.span.start() + u32::from(tok.content_offset)) as usize,
            tok.span.end() as usize,
            text,
        )
    }

    fn lower_body_inner(&mut self, text: &str, base_offset: u32, namespace: &str) -> Script {
        let commands = segment_commands_with_offset_and_config(text, base_offset, self.config);
        let inherited = self.const_map_stack.last().cloned().unwrap_or_default();
        self.const_map_stack.push(inherited);
        let stmts = self.lower_segmented(&commands, namespace);
        self.const_map_stack.pop();
        Script::from_statements(stmts)
    }

    /// Lower a list of segmented commands to IR statements.
    fn lower_segmented(
        &mut self,
        segments: &[SegmentedCommand],
        namespace: &str,
    ) -> Vec<Statement> {
        let mut stmts = Vec::new();
        for seg in segments {
            if seg.is_partial {
                stmts.push(Statement::Barrier {
                    span: seg.span,
                    reason: "incomplete command".into(),
                    command: String::new(),
                    canonical_command: None,
                    args: vec![],
                    tokens: None,
                });
                continue;
            }
            if let Some(stmt) = self.lower_command(seg, namespace) {
                self.update_const_map(seg, &stmt);
                stmts.push(stmt);
            }
        }
        self.debug_assert_spans_in_source(&stmts);
        stmts
    }

    /// Every statement leaving [`Self::lower_segmented`] must carry a span
    /// that is a real range of the buffer it was lowered from — the contract
    /// downstream consumers rely on when they slice the source with an IR
    /// span (AGENTS.md: "trust it rather than re-deriving").
    ///
    /// This is the producer-side chokepoint: every statement the lowering
    /// emits passes through here, at the nesting level whose segments were
    /// cut from [`Self::source`], so a body rebase that overshoots its
    /// document (issue #1325's compound `{body}x` word, issue #1375's
    /// non-literal body word) fails the build's own tests instead of
    /// reaching a consumer.  Debug-only: the guards in
    /// [`Self::lower_body_from_tok`] and codegen's slice clamp are what keep
    /// a release build safe.
    fn debug_assert_spans_in_source(&self, stmts: &[Statement]) {
        debug_assert!(
            self.source.is_empty()
                || stmts
                    .iter()
                    .all(|stmt| self.source.get(stmt.span().as_range()).is_some()),
            "lowering emitted a span outside its {} byte source: {:?}",
            self.source.len(),
            stmts
                .iter()
                .map(Statement::span)
                .filter(|span| self.source.get(span.as_range()).is_none())
                .collect::<Vec<_>>(),
        );
    }

    /// Maintain the per-scope const-map after a command lowers.
    ///
    /// Populates a `name → braced-literal` entry when *seg* is a
    /// `set var {literal}` shape with a plain bareword LHS;
    /// otherwise invalidates entries that *stmt* may have written.
    /// Gated on `proc_depth > 0` — globals / namespace vars at
    /// top-level or inside `namespace eval` cannot be safely
    /// const-propagated.
    fn update_const_map(&mut self, seg: &SegmentedCommand, stmt: &Statement) {
        if self.proc_depth == 0 {
            return;
        }
        let Some(scope) = self.const_map_stack.last_mut() else {
            return;
        };

        if let Some((name, value)) = set_literal_body(seg) {
            scope.insert(name, value);
            return;
        }

        invalidate_const_map_for(stmt, scope);
    }

    /// Resolve a `$var` / `${var}` body word against the current
    /// const-map and return the bound literal, or `None` if the
    /// word is not a pure single-token variable reference, the
    /// variable has no known literal binding, or we are not inside
    /// a `proc` body (top-level / `namespace eval` scopes are out
    /// of scope for this optimisation).
    fn const_map_lookup(&self, word: &str) -> Option<String> {
        if self.proc_depth == 0 {
            return None;
        }
        let scope = self.const_map_stack.last()?;
        let inner = if let Some(rest) = word.strip_prefix("${") {
            let inner = rest.strip_suffix('}')?;
            if inner.contains('$')
                || inner.contains('[')
                || inner.contains('{')
                || inner.contains('(')
            {
                return None;
            }
            inner
        } else {
            let rest = word.strip_prefix('$')?;
            if rest.is_empty()
                || rest.contains('(')
                || rest.contains('$')
                || rest.contains('[')
                || rest.contains('{')
                || rest.starts_with(':')
            {
                return None;
            }
            rest
        };
        scope.get(inner).cloned()
    }

    /// Build a `CommandTokens` snapshot from a segmented command.
    fn cmd_tokens(seg: &SegmentedCommand) -> CommandTokens {
        CommandTokens::from_segmented(seg)
    }

    /// Extract arg token kinds for the lowering hooks.
    fn arg_kinds(seg: &SegmentedCommand) -> Vec<ArgTokenKind> {
        seg.argv
            .iter()
            .skip(1)
            .map(|t| arg_token_kind(t.kind))
            .collect()
    }

    /// Detect ``namespace import ?-force? pattern...``
    /// and ``namespace export pattern...``. Records absolute
    /// patterns only.  Skips `{*}`-expanded calls and statically-
    /// dead branches.
    fn record_namespace_directives(
        &mut self,
        cmd_name: &str,
        args: &[String],
        seg: &SegmentedCommand,
        namespace: &str,
    ) {
        if cmd_name != "namespace" || args.len() < 2 || self.dead_code_depth != 0 {
            return;
        }
        let no_expand = seg
            .expand_word
            .as_ref()
            .is_none_or(|ew| !ew.iter().any(|&e| e));
        if !no_expand {
            return;
        }
        if args[0] == "import" {
            let mut i = 1usize;
            while i < args.len() && args[i].starts_with('-') {
                i += 1;
            }
            for pat in &args[i..] {
                if pat.starts_with("::") && pat[2..].contains("::") {
                    self.namespace_imports
                        .push((namespace.to_string(), pat.clone()));
                }
            }
        } else if args[0] == "export" {
            let mut i = 1usize;
            // ``-clear`` is the only flag for ``namespace export``.
            while i < args.len() && args[i].starts_with('-') {
                i += 1;
            }
            for pat in &args[i..] {
                self.namespace_exports
                    .push((namespace.to_string(), pat.clone()));
            }
        }
    }

    /// Whether a resolved lowering hook commits to a **positional argv
    /// shape** — its lowerer reads "the body is word N", "the loop variables
    /// are word 0", "the result variable is word 1" — so a `{*}` expansion
    /// among the words makes the shape unknowable and the call must barrier
    /// instead.
    ///
    /// Registry-derived by construction: the answer is keyed on the typed
    /// [`LoweringHookId`] that [`Self::try_dispatch_structured_hook`] already
    /// dispatches on, not on a command-name list (issue #1380 — a nine-name
    /// list had drifted against seventeen structured hook IDs, so `lmap`,
    /// `dict for`, `array for`, `foreachLine`, `catch`, `try`, `eval`,
    /// `uplevel`, and `apply` all committed to their un-expanded argv).
    /// The match is exhaustive with no wildcard arm, so a new hook ID cannot
    /// be added without classifying it here.
    ///
    /// The `false` arm is the non-structured group: those hooks run in
    /// [`try_lower_hook`] before this dispatcher is reached, and each already
    /// declines an expanded call itself via `has_expansion`.
    fn hook_commits_to_argv_shape(hook: LoweringHookId) -> bool {
        match hook {
            LoweringHookId::Proc
            | LoweringHookId::When
            | LoweringHookId::NamespaceEval
            | LoweringHookId::If
            | LoweringHookId::Switch
            | LoweringHookId::For
            | LoweringHookId::While
            | LoweringHookId::Foreach
            | LoweringHookId::Lmap
            | LoweringHookId::ForeachLine
            | LoweringHookId::Catch
            | LoweringHookId::Try
            | LoweringHookId::Dict
            | LoweringHookId::Eval
            | LoweringHookId::Uplevel
            | LoweringHookId::Apply
            | LoweringHookId::ArrayFor => true,
            LoweringHookId::Expr
            | LoweringHookId::Return
            | LoweringHookId::Set
            | LoweringHookId::Incr
            | LoweringHookId::AppendOrLappend
            | LoweringHookId::Unset
            | LoweringHookId::Global
            | LoweringHookId::Variable
            | LoweringHookId::Upvar => false,
        }
    }

    /// `{*}` expansion on a structured command lowers to a barrier so
    /// downstream analyses can't reason about the expanded form.
    fn structured_expand_barrier(
        cmd_name: &str,
        args: &[String],
        seg: &SegmentedCommand,
    ) -> Statement {
        Statement::Barrier {
            span: seg.span,
            reason: format!("{cmd_name} with argument expansion"),
            command: cmd_name.into(),
            canonical_command: None,
            args: args.to_vec(),
            tokens: Some(Self::cmd_tokens(seg)),
        }
    }

    /// Try to dispatch *`cmd_name`* through the registry's typed
    /// [`LoweringHookId`] for structured commands dispatched from
    /// [`lower_command`](Self::lower_command).  Returns
    /// `Some(stmt)` when the hook ID matches a structured form;
    /// returns `None` otherwise so `lower_command` falls back to
    /// [`lower_default`](Self::lower_default).
    ///
    /// Every structured form routes through
    /// `resolve_call().lowering_hook`, and unmatched commands flow
    /// straight to `lower_default`.  Forms with shape preconditions
    /// (`proc`, `namespace eval`, `foreach`, `lmap`, `dict`,
    /// `when`, `foreachLine`) return `None` on a failed
    /// precondition so `lower_default` catches them.
    ///
    /// Dispatching through the hook ID picks up two benefits over a
    /// bare name match:
    ///
    /// 1. Canonical resolution via [`CommandRegistry::resolve_call`]
    ///    instead of bare name comparison, so any future spec
    ///    that aliases an existing form (e.g. an iRules-specific
    ///    `eval` variant) automatically dispatches correctly
    ///    when its `lowering_hook` is stamped.
    /// 2. The hook ID is the canonical key consumed by downstream
    ///    audit / LSP / compiler-explorer surfaces.
    fn try_dispatch_structured_hook(
        &mut self,
        cmd_name: &str,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Option<Statement> {
        let args = seg.args();
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        // Resolved under the registry's own availability mask (issues
        // #1462/#1463): a profile-built registry suppresses the structured
        // lowering of a command its release does not have (`lmap` at 8.4),
        // so the call flows to `lower_default` and reaches the runtime's
        // availability gate as a generic dispatch. A profile-less registry
        // keeps the dialect-blind resolution.
        let resolved = self.registry.resolve_invocation(
            cmd_name,
            &arg_refs,
            self.registry.own_availability_mask(),
        )?;
        let hook = resolved.semantics.lowering_hook?;
        // The expansion gate lives here, keyed on the same typed hook the
        // dispatch below uses, so it can never name a different set of
        // commands than the lowerers it protects (issue #1380).
        if Self::hook_commits_to_argv_shape(hook)
            && seg
                .expand_word
                .as_ref()
                .is_some_and(|ew| ew.iter().any(|&e| e))
        {
            return Some(Self::structured_expand_barrier(cmd_name, args, seg));
        }
        let inline_body_error_context = resolved.semantics.operation.inline_body_error_context();
        match hook {
            // Static-body uplevel.  Match `uplevel 1 {body}`,
            // `uplevel #0 {body}`, and the canonical no-level form
            // `uplevel {body}` (level defaults to 1) when the body
            // arg is a brace-string literal token.  Dynamic forms
            // (`uplevel 1 $body` / `uplevel $lvl {body}`) fall
            // through to the default lowering so a runtime
            // `Call` / `Barrier` carries the unresolved arguments.
            LoweringHookId::Uplevel => Some(
                self.try_lower_uplevel_static(seg, namespace)
                    .unwrap_or_else(|| self.lower_default(seg, namespace)),
            ),

            // `eval $body` / `eval {body}` with a literal /
            // const-folded body relaxes to a `Statement::Block` so
            // downstream analyses see the inlined script.  Dynamic
            // bodies (`eval $dyn` with no const-map binding,
            // `eval [cmd]`) fall through to the default barrier
            // dispatch.
            LoweringHookId::Eval => Some(
                self.try_lower_eval_static(seg, namespace, inline_body_error_context)
                    .unwrap_or_else(|| self.lower_default(seg, namespace)),
            ),

            // `apply {{params} body ?ns?} …` — walk the braced body so nested
            // definitions register (like `namespace eval`), keeping the call a
            // runtime barrier because the body runs in a separate frame.
            LoweringHookId::Apply => Some(self.lower_apply(seg)),

            // `array for {k v} arr body` (Tcl 9.0) — iterate array entries.
            // Like `apply`, the call stays a runtime barrier (C Tcl invokes
            // `::tcl::array::for` with the body as an unparsed literal), but a
            // braced literal body is walked in a fresh frame bound to the two
            // loop variables so it is analysable.
            LoweringHookId::ArrayFor => Some(self.lower_array_for(seg, namespace)),

            // Straightforward control-flow forms.  Each is a
            // single-method dispatch with no arity / shared-method /
            // subcommand complications.
            LoweringHookId::If => Some(self.lower_if(seg, namespace)),
            LoweringHookId::Switch => Some(self.lower_switch(seg, namespace)),
            LoweringHookId::For => Some(self.lower_for(seg, namespace)),
            LoweringHookId::While => Some(self.lower_while(seg, namespace)),
            LoweringHookId::Catch => Some(self.lower_catch(seg, namespace)),
            LoweringHookId::Try => Some(self.lower_try(seg, namespace)),

            // Forms with shape preconditions.  Calls with the wrong
            // arity / shape must fall through to `lower_default`
            // instead of crashing the dedicated lowerer, so each arm
            // returns `None` on precondition failure and
            // `lower_command` then falls through to `lower_default`.
            //
            // `proc name params body` — exactly three args and
            // at least three token slices (the body needs to
            // be a real token, not synthesised whitespace).
            LoweringHookId::Proc => {
                self.try_lower_proc_declaration(seg, namespace, resolved.canonical_command)
            }
            // `namespace eval ns body` — the subcommand match
            // is already handled by `resolve_call`, so the
            // hook only fires for the `eval` subcommand.  Keep
            // the arity / token-count guards (a tokenless body
            // would derail body lowering).
            LoweringHookId::NamespaceEval => {
                if args.len() >= 3 && seg.arg_tokens().len() >= 3 {
                    Some(self.lower_namespace_eval(seg, namespace))
                } else {
                    None
                }
            }
            // `foreach vars list body` / `lmap vars list body`
            // — share `lower_foreach(... is_lmap)`.  The
            // dedicated lowerer handles its own shape errors,
            // so no precondition here.
            LoweringHookId::Foreach => Some(self.lower_foreach(seg, namespace, false)),
            LoweringHookId::Lmap => Some(self.lower_foreach(seg, namespace, true)),
            // `dict <subcommand> ...` — must have at least one
            // arg so the subcommand can be picked.  Bare `dict`
            // falls through to `lower_default`.
            LoweringHookId::Dict => {
                if args.is_empty() {
                    None
                } else {
                    Some(self.lower_dict(seg, namespace))
                }
            }

            // `when EVENT ?priority N? body` — iRules event
            // handler.  The hook stamp lives on the `when` spec
            // in `tcl-registry/src/commands/irules/when.rs`,
            // which `load_dialect(DialectSet::IRULES)` brings
            // into the registry.  Production callers (the LSP
            // server) load the active dialect
            // before lowering; tests that lower iRule code call
            // `registry.load_irules()` explicitly (see
            // `irules_checks::tests::registry`).  Callers that
            // lower iRule code against a vanilla `build_default()`
            // registry now silently flow through to
            // `lower_default` — that path was always a
            // misconfiguration; the dialect needs to match the
            // source.
            LoweringHookId::When => {
                self.try_lower_when_declaration(seg, namespace, resolved.canonical_command)
            }
            // `foreachLine varName filename body` — Tcl 9.0
            // (TIP 670).  Always registered in `build_default()`
            // via `tcl::foreachline::spec`, so no dialect-load
            // dance is needed.  The `lower_foreach_line`
            // emitter handles its own shape errors and
            // dynamic-body fallback, but the `args.len() == 3`
            // guard keeps an under- or over-argued `foreachLine`
            // flowing to `lower_default` instead of triggering a
            // barrier inside the dedicated emitter.
            LoweringHookId::ForeachLine => {
                if args.len() == 3 {
                    Some(self.lower_foreach_line(seg, namespace))
                } else {
                    None
                }
            }

            // Non-structured hooks (`Expr` / `Return` / `Set` /
            // `Incr` / `AppendOrLappend` / `Unset` / `Global` /
            // `Variable` / `Upvar`) are handled by
            // [`try_lower_hook`] before [`lower_command`] reaches
            // this dispatcher.  They can still reach here when
            // their static dispatcher returned `None` for the
            // input shape (e.g. `try_lower_expr` rejects
            // multi-arg `expr`); fall through to `lower_default`.
            LoweringHookId::Expr
            | LoweringHookId::Return
            | LoweringHookId::Set
            | LoweringHookId::Incr
            | LoweringHookId::AppendOrLappend
            | LoweringHookId::Unset
            | LoweringHookId::Global
            | LoweringHookId::Variable
            | LoweringHookId::Upvar => None,
        }
    }

    fn try_lower_proc_declaration(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        canonical_command: &str,
    ) -> Option<Statement> {
        if seg.args().len() != 3 || seg.arg_tokens().len() < 3 {
            return None;
        }
        if self
            .registry
            .profile()
            .is_some_and(tcl_dialect::DialectProfile::is_irules)
        {
            if self
                .registry
                .irules_command_placement(canonical_command, self.irules_execution_context)
                != IrulesCommandPlacement::Allowed
            {
                return None;
            }
            let arg_refs: Vec<&str> = seg.args().iter().map(String::as_str).collect();
            let closed = tcl_registry::events::closed_braced_argument_words(
                &self.source,
                seg.arg_tokens(),
                seg.arg_single_token(),
            )?;
            let declaration = tcl_registry::events::IrulesDeclarationArguments::new(
                &arg_refs,
                seg.arg_tokens(),
                seg.arg_single_token(),
                &closed,
            )
            .and_then(|arguments| {
                self.registry
                    .irules_top_level_declaration_shape(canonical_command, arguments)
            });
            if !matches!(
                declaration,
                Some(tcl_registry::events::IrulesTopLevelDeclaration::Procedure { .. })
            ) {
                return None;
            }
        }
        Some(self.lower_proc(seg, namespace))
    }

    fn try_lower_when_declaration(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        canonical_command: &str,
    ) -> Option<Statement> {
        if self
            .registry
            .irules_command_placement(canonical_command, self.irules_execution_context)
            != IrulesCommandPlacement::Allowed
        {
            return None;
        }
        let arg_refs: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        let closed = tcl_registry::events::closed_braced_argument_words(
            &self.source,
            seg.arg_tokens(),
            seg.arg_single_token(),
        )?;
        let arguments = tcl_registry::events::IrulesDeclarationArguments::new(
            &arg_refs,
            seg.arg_tokens(),
            seg.arg_single_token(),
            &closed,
        )?;
        let tcl_registry::events::IrulesTopLevelDeclaration::Event {
            event,
            body_index,
            priority,
        } = self
            .registry
            .irules_top_level_declaration_shape(canonical_command, arguments)?
        else {
            return None;
        };
        Some(self.lower_when(seg, namespace, &event, body_index, priority))
    }

    /// Lower a single command.
    fn lower_command(&mut self, seg: &SegmentedCommand, namespace: &str) -> Option<Statement> {
        if seg.texts.is_empty() {
            return None;
        }

        let cmd_name = seg.name();
        let args = seg.args();

        if self.irules_execution_context == IrulesExecutionContext::TopLevel {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            if let Some(resolved) = self.registry.resolve_invocation(
                cmd_name,
                &arg_refs,
                self.registry.own_availability_mask(),
            ) && let Some(tcl_registry::events::IrulesTopLevelDeclaration::Priority { value }) =
                self.registry
                    .irules_top_level_effect(resolved.canonical_command, &arg_refs)
            {
                self.irules_priority = value;
            }
        }

        // Detect `interp alias {} name {} target ?args?` and static
        // `rename oldName newName` — both feed the same alias table, since
        // calling through a renamed name and calling through an `interp
        // alias` are indistinguishable for arg-role / body-form resolution,
        // taint sink dispatch, and canonical-command typing (`interp alias {}
        // myEval {} eval` and `rename eval myEval` both make `myEval $x` reach
        // the `eval` sink). `interp alias` pre-qualifies its name at the global
        // root, but `rename` binds NEW in the *current* namespace, so qualify
        // it against `namespace` — an unqualified `rename set myset` inside
        // `namespace eval ::ns` binds `::ns::myset`, not global `::myset` (and
        // global `myset` is then invalid). `join_namespace` leaves an
        // already-`::`-qualified NEW rooted globally, matching how
        // `resolve_alias` looks the name back up (current namespace, then
        // global) and `command_binding`'s own namespace-relative candidates.
        let args_owned: Vec<String> = args.to_vec();
        match self
            .registry
            .command_table_effect(cmd_name, args_owned.first().map(String::as_str))
        {
            Some(tcl_registry::CommandTableEffect::CreatesAliases) => {
                if let Some((qualified, target, prepended)) = detect_interp_alias(&args_owned) {
                    self.aliases.insert(qualified, (target, prepended));
                }
            }
            Some(tcl_registry::CommandTableEffect::RenamesCommands) => {
                if let Some((old, new)) = detect_rename(&args_owned) {
                    self.aliases
                        .insert(join_namespace(namespace, &new), (old, Vec::new()));
                }
            }
            // `proc` feeds the alias table nothing — its definition is
            // lowered by its own hook below.
            _ => {}
        }

        self.record_namespace_directives(cmd_name, args, seg, namespace);

        // Trace-visible compilation (`CompileTarget::BytecodeTraced`)
        // suppresses every inline/structured hook unconditionally: every
        // command in this pass becomes a plain runtime dispatch, so an
        // execution trace observes it. The alias-table/namespace-directive
        // bookkeeping above still runs (a traced body's own `rename`/
        // `interp alias` commands must still affect how later commands in it
        // resolve).
        if self.target.is_trace_visible() {
            return Some(self.lower_default(seg, namespace));
        }

        // Try registered lowering hooks first.
        let hook_cmd = LoweringCommand {
            span: seg.span,
            name: cmd_name,
            args,
            single_token_word: &seg.single_token_word,
            expand_word: seg.expand_word.as_deref(),
            tokens: Some(Self::cmd_tokens(seg)),
            arg_kinds: &Self::arg_kinds(seg),
            dialect: self.dialect,
        };
        if let Some(stmt) = try_lower_hook(
            &hook_cmd,
            &self.aliases,
            self.registry,
            self.safe_on_uninit(cmd_name, args),
        ) {
            return Some(stmt);
        }

        // Registry-driven hook-ID dispatch covers all 17
        // structured command forms — every typed
        // `LoweringHookId` (Proc, When, NamespaceEval, If,
        // Switch, For, While, Foreach, Lmap, ForeachLine, Catch,
        // Try, Dict, Eval, Uplevel, Apply, ArrayFor) flows through
        // [`try_dispatch_structured_hook`], which also raises the
        // `{*}`-expansion barrier for them.  Commands that
        // aren't in the registry, or whose hook is `None`, fall
        // through to [`lower_default`] below.
        if let Some(stmt) = self.try_dispatch_structured_hook(cmd_name, seg, namespace) {
            return Some(stmt);
        }

        Some(self.lower_default(seg, namespace))
    }

    /// Lower `proc name params body`.
    ///
    /// Sequential phases:
    ///   1. Empty-simple-name barrier (`proc ::ns:: {…} {…}`).
    ///   2. Dynamic name resolution (`proc $x …` / `proc [cmd] …`).
    ///   3. Dynamic body / params check, with const-map body
    ///      materialisation when possible.
    ///   4. IR registration + emit the runtime `proc` call.
    fn lower_proc(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args_borrow = seg.args();
        let proc_name_initial = &args_borrow[0];

        // Phase 1: empty-simple-name procs (`proc ::ns:: {args} {body}`).
        // Tcl 9 lets a trailing `::` register a command named `""` inside
        // the target namespace (`TclGetNamespaceForQualName` returns
        // `simpleName=""` rather than NULL for trailing colons on
        // cmd/var lookups — tclNamesp.c:2493).  Our static
        // `normalise_qualified_name` strips the trailing colons
        // (`"a::".split("::")` filters the empty trailing element),
        // so an AOT-lifted empty-name proc would register under the
        // namespace name itself — a different command than the
        // runtime lookup needs.  Route the whole shape through the
        // runtime `proc` builtin via `Statement::Barrier`.  Carries
        // namespace-old-1.27 / 2.1 / 2.2 and namespace-14.11.
        if proc_name_initial.ends_with("::") {
            return Statement::Barrier {
                span: seg.span,
                reason: "empty-simple-name proc".into(),
                command: "proc".into(),
                canonical_command: None,
                args: args_borrow.to_vec(),
                tokens: Some(Self::cmd_tokens(seg)),
            };
        }
        // Phase 2: dynamic proc name resolution.
        let (proc_name_owned, args_owned, name_was_substituted) =
            match self.resolve_dynamic_proc_name(seg, args_borrow) {
                Ok(triple) => triple,
                Err(barrier) => return *barrier,
            };
        let args: &[String] = &args_owned;
        let proc_name = &proc_name_owned;
        // Phase 3: dynamic body / params check, plus body
        // materialisation via the const-map.
        let (materialised_body, body_is_dynamic, body_offset) =
            match self.check_proc_body_dynamic(seg, args, name_was_substituted) {
                Ok(triple) => triple,
                Err(barrier) => return *barrier,
            };

        // Phase 4: lower the body (materialised, static, or empty
        // for dynamic-with-resolved-name), register the Procedure,
        // and emit the runtime `proc` Call.
        // A parameter list Tcl would reject (malformed, an overlong specifier,
        // an array-element or qualified name) creates no procedure: leave it to
        // the runtime `proc`, which raises the error, rather than registering a
        // Procedure whose parameters we guessed at.
        let Some(params) = parse_formal_param_names(&args[1]) else {
            return Statement::Barrier {
                span: seg.span,
                reason: "malformed proc params".into(),
                command: "proc".into(),
                canonical_command: None,
                args: seg.args().to_vec(),
                tokens: Some(Self::cmd_tokens(seg)),
            };
        };
        let qualified = qualify_proc_name(namespace, proc_name);
        // The body's own command/variable resolution namespace is the one the
        // proc is *defined in* — the qualifier prefix of its own qualified
        // name — not the lexical namespace the `proc` call was written in.
        // See [`proc_body_namespace`] for the oracle transcript.
        let body_namespace = proc_body_namespace(&qualified, namespace);
        let body_namespace: &str = &body_namespace;
        let body_text = &args[2];
        let body = self.lower_proc_body(
            materialised_body,
            body_is_dynamic,
            body_text,
            body_offset,
            body_namespace,
        );

        // A `proc` defined inside a TclOO method body is created
        // at method-call time in the global namespace, not at script
        // load — so it must not be lifted into `module.procedures`
        // (codegen would otherwise emit it unconditionally). The body
        // was still lowered above for analysis; only the global
        // registration is suppressed.
        if !self.suppress_proc_register {
            match self.module.procedures.entry(qualified.clone()) {
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(Procedure {
                        name: proc_name.clone(),
                        qualified_name: qualified,
                        params,
                        span: seg.span,
                        body,
                        params_raw: args[1].clone(),
                        body_source: Some(args[2].clone()),
                        namespace_scoped: self.in_namespace_eval,
                        base_priority: 500,
                    });
                }
                std::collections::hash_map::Entry::Occupied(_) => {
                    self.module.redefined_procedures.insert(qualified);
                }
            }
        }

        Statement::Call {
            span: seg.span,
            command: "proc".into(),
            canonical_command: None,
            args: args.to_vec(),
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(Self::cmd_tokens(seg)),
            foreach_groups: None,
        }
    }

    /// Lower one procedure body in its fresh procedure frame.
    fn lower_proc_body(
        &mut self,
        materialised_body: Option<String>,
        body_is_dynamic: bool,
        body_text: &str,
        body_offset: u32,
        body_namespace: &str,
    ) -> Script {
        // ``lower_body`` otherwise inherits the enclosing scope's tracked
        // scalars, which is sound for control flow but not a proc's fresh
        // runtime frame.
        self.proc_depth += 1;
        self.const_map_stack.push(HashMap::new());
        let body = if let Some(text) = materialised_body {
            self.lower_script_in_irules_context(
                &text,
                body_namespace,
                IrulesExecutionContext::ProcedureBody,
            )
        } else if body_is_dynamic {
            // The resolved body value is dynamic; retain the runtime call but
            // do not compile the literal `$body` spelling as Tcl source.
            Script::default()
        } else if let Some(cache) = self.body_cache.filter(|_| {
            self.proc_depth == 1
                && !self
                    .registry
                    .profile()
                    .is_some_and(tcl_dialect::DialectProfile::is_irules)
                && body_cache_eligible(body_text)
        }) {
            // Only context-free Tcl bodies can use the offset-zero body cache.
            let mut script = cache(body_text, body_namespace);
            crate::lattice_rebase::rebase_script(&mut script, i64::from(body_offset));
            script
        } else {
            self.lower_body_in_irules_context(
                body_text,
                body_offset,
                body_namespace,
                IrulesExecutionContext::ProcedureBody,
            )
        };
        self.const_map_stack.pop();
        self.proc_depth -= 1;
        body
    }

    /// Resolve a proc name that may be a `$var` / `[cmd]` substitution.
    ///
    /// Returns the resolved name, the (possibly rewritten) args
    /// vector, and a `name_was_substituted` flag.  A literal name
    /// short-circuits to `Ok` with the original args.  A dynamic
    /// name that can't be resolved via the const-map returns `Err`
    /// with a "dynamic proc name" barrier — the caller propagates
    /// it as the statement.
    ///
    /// Multi-token names (`foo_$x`) and command-substitution names
    /// (`$name[suffix]`) stay on the runtime path.
    fn resolve_dynamic_proc_name(
        &mut self,
        seg: &SegmentedCommand,
        args_borrow: &[String],
    ) -> Result<(String, Vec<String>, bool), Box<Statement>> {
        let proc_name_initial = &args_borrow[0];
        if !proc_name_initial.contains('$') && !proc_name_initial.contains('[') {
            return Ok((proc_name_initial.clone(), args_borrow.to_vec(), false));
        }

        let arg_tokens = seg.arg_tokens();
        let single_token_proc_name = seg.single_token_word.get(1).copied().unwrap_or(false);
        let resolved = if !proc_name_initial.contains('[')
            && single_token_proc_name
            && arg_tokens
                .first()
                .is_some_and(|t| t.kind == tcl_lexer::TokenType::Var)
        {
            self.const_map_lookup(proc_name_initial)
        } else {
            None
        };

        let Some(literal) = resolved else {
            return Err(Box::new(Statement::Barrier {
                span: seg.span,
                reason: "dynamic proc name".into(),
                command: "proc".into(),
                canonical_command: None,
                args: args_borrow.to_vec(),
                tokens: Some(Self::cmd_tokens(seg)),
            }));
        };

        let mut args_owned = args_borrow.to_vec();
        args_owned[0].clone_from(&literal);
        Ok((literal, args_owned, true))
    }

    /// Check whether the proc body and/or params are dynamic, and
    /// attempt to materialise the body from the const-map.
    ///
    /// Returns:
    ///   * `Ok((Some(text), _, body_offset))` — body materialised
    ///     successfully; caller should lower `text` as a fresh
    ///     script.
    ///   * `Ok((None, body_is_dynamic, body_offset))` — body is
    ///     static (lower from source) or dynamic-with-name-resolved
    ///     (lower as empty Script).
    ///   * `Err(barrier)` — params dynamic, or body dynamic without
    ///     name substitution.  Caller propagates the barrier.
    fn check_proc_body_dynamic(
        &mut self,
        seg: &SegmentedCommand,
        args: &[String],
        name_was_substituted: bool,
    ) -> Result<(Option<String>, bool, u32), Box<Statement>> {
        let arg_tokens = seg.arg_tokens();
        let params_tok = arg_tokens[1];
        let body_tok = arg_tokens[2];
        // `single_token_word[0]` covers the command itself; args[0..]
        // live at indices `[1..]`.  See `SegmentedCommand::arg_single_token`.
        let body_is_single = seg.single_token_word.get(3).copied().unwrap_or(false);
        let params_is_single = seg.single_token_word.get(2).copied().unwrap_or(false);
        let body_is_dynamic = matches!(
            body_tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        ) || !body_is_single;
        let params_is_dynamic = matches!(
            params_tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        ) || !params_is_single;
        let materialised_body: Option<String> =
            if body_is_single && body_tok.kind == tcl_lexer::TokenType::Cmd {
                // `args[2]` is the `word_piece`-reconstructed source
                // — for a Cmd token that's `[subst -nocommands {…}]`
                // (with the brackets re-added).
                // `eval_subst_nocommands_body` segments its input as
                // a top-level Tcl command stream and expects the
                // inner content, so strip the outer `[…]` before
                // handing it over.
                args[2]
                    .strip_prefix('[')
                    .and_then(|s| s.strip_suffix(']'))
                    .and_then(|inner| self.eval_subst_nocommands_body(inner))
            } else if body_is_single && body_tok.kind == tcl_lexer::TokenType::Var {
                self.const_map_lookup(&args[2])
            } else {
                None
            };
        if params_is_dynamic
            || (body_is_dynamic && materialised_body.is_none() && !name_was_substituted)
        {
            return Err(Box::new(Statement::Barrier {
                span: seg.span,
                reason: if params_is_dynamic {
                    "dynamic proc params"
                } else {
                    "dynamic proc body"
                }
                .into(),
                command: "proc".into(),
                canonical_command: None,
                args: seg.args().to_vec(),
                tokens: Some(Self::cmd_tokens(seg)),
            }));
        }
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        Ok((materialised_body, body_is_dynamic, body_offset))
    }

    /// Lower a registry-validated iRules event declaration.
    fn lower_when(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        event_name: &str,
        body_idx: usize,
        priority: Option<u16>,
    ) -> Statement {
        let args = seg.args();
        let body_tok = seg.arg_tokens()[body_idx];
        // The rebase below is truthful only for a body word that is its source
        // region verbatim — see [`Self::guarded_body_text`].
        let body_text = self.guarded_body_text(body_tok, &args[body_idx]);
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        // Fresh const-map frame for the nested proc body.
        // ``lower_body`` would otherwise inherit the enclosing
        // scope's tracked scalars — correct for control-flow
        // bodies (if / catch / loops share the frame) but unsound
        // for ``proc`` bodies, which have their own runtime frame.
        // Pushing an empty frame here means the inner ``lower_body``
        // clones an empty parent, giving the proc body a clean
        // slate.
        self.proc_depth += 1;
        self.const_map_stack.push(HashMap::new());
        let body = self.lower_body_in_irules_context(
            body_text,
            body_offset,
            namespace,
            IrulesExecutionContext::EventBody,
        );
        self.const_map_stack.pop();
        self.proc_depth -= 1;

        let base_priority = u32::from(priority.unwrap_or(self.irules_priority));

        let n = self.when_counts.get(event_name).copied().unwrap_or(0);
        *self.when_counts.entry(event_name.to_owned()).or_insert(0) += 1;
        let qualified = if n == 0 {
            format!("::when::{event_name}")
        } else {
            format!("::when::{event_name}#{n}")
        };

        self.module.procedures.insert(
            qualified.clone(),
            Procedure {
                name: event_name.to_owned(),
                qualified_name: qualified,
                params: vec![],
                span: seg.span,
                body,
                params_raw: String::new(),
                body_source: None,
                namespace_scoped: false,
                base_priority,
            },
        );

        Statement::Call {
            span: seg.span,
            command: "when".into(),
            canonical_command: None,
            args: args.to_vec(),
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(Self::cmd_tokens(seg)),
            foreach_groups: None,
        }
    }

    /// Lower `namespace eval ns body`.
    /// Try to lower `uplevel ?level? {body}` to a static-body
    /// [`Statement::UpFrame`] when:
    ///
    /// 1. The body argument is a brace-string token (`TokenType::Str`),
    ///    and
    /// 2. The level argument (if present) parses as a positive integer
    ///    or `#0` / `#N` global form.
    ///
    /// Returns `None` if the call doesn't match the static shape, in
    /// which case the caller falls back to [`Self::lower_default`]
    /// (producing a runtime [`Statement::Barrier`]).
    fn try_lower_uplevel_static(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Option<Statement> {
        use tcl_lexer::TokenType;

        // Static-body `uplevel` lowers to `Statement::UpFrame` so the analysers
        // (interproc purity, code-sinking, var-escape, the inline_uplevel pass)
        // see a structured frame-shift with a lowered body instead of an opaque
        // runtime barrier. Runtime correctness is preserved at codegen: a
        // surviving `UpFrame` re-emits the original `uplevel <level> {body}`
        // invoke from its preserved command tokens (see
        // `codegen::statements`), producing byte-identical bytecode to the old
        // barrier path and dispatching through the `uplevel` builtin's correct
        // frame semantics. The whole-callee inline_uplevel splice (which would
        // need frame-shift opcodes to run a *bare* UpFrame against the right
        // activation) is an opt-in optimiser pass, not wired into codegen.
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let (level, body_tok_idx) = match args.len() {
            // A bare `uplevel {body}` means relative level 1.
            1 => (
                UplevelLevel {
                    shift: 1,
                    absolute: false,
                },
                0,
            ),
            2 => (parse_uplevel_level(&args[0])?, 1),
            _ => return None,
        };
        let body_tok = arg_tokens.get(body_tok_idx)?;
        // Barrier gate: see `try_lower_eval_static` for the
        // rationale.  A static-body `uplevel 1 {eval $x}` would
        // relax to inline IR with a still-dynamic `eval`; reject.
        if body_tok.kind == TokenType::Str {
            let body_text = &args[body_tok_idx];
            if body_has_dynamic_barrier(body_text, self.registry) {
                return None;
            }
        }
        let body = if body_tok.kind == TokenType::Str {
            let body_text = self.guarded_body_text(*body_tok, &args[body_tok_idx]);
            let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
            self.lower_body(body_text, body_offset, namespace)
        } else if body_tok.kind == TokenType::Var {
            // `uplevel ?N? $var` with $var resolved by the
            // const-map to a brace-string literal — fold the literal
            // in and lower as a static UpFrame.
            let literal = self.const_map_lookup(&args[body_tok_idx])?;
            self.lower_script(&literal, namespace)
        } else {
            return None;
        };
        Some(Statement::UpFrame {
            span: seg.span,
            frame_shift: level.shift,
            absolute: level.absolute,
            body,
            tokens: Some(Self::cmd_tokens(seg)),
        })
    }

    /// If *`cmd_text`* is `subst -nocommands {template}` (in any
    /// flag order) AND every `$var` inside *template* is in the
    /// current const-map, return the substituted string. Otherwise
    /// `None` so the caller falls back to runtime dispatch.
    ///
    /// Used to materialise the tcltest-style `Option` factory body
    /// at compile time when the surrounding proc has all the
    /// template vars const-tracked.
    fn eval_subst_nocommands_body(&self, cmd_text: &str) -> Option<String> {
        use tcl_lexer::TokenType;
        let inner = segment_commands_with_offset_and_config(cmd_text, 0, self.config);
        if inner.len() != 1 {
            return None;
        }
        let inner_cmd = &inner[0];
        if inner_cmd.texts.is_empty() || inner_cmd.texts[0] != "subst" {
            return None;
        }
        let argv = inner_cmd.arg_tokens();
        let texts = inner_cmd.args();
        let single = &inner_cmd.single_token_word;

        let mut saw_nocommands = false;
        let mut template_text: Option<&str> = None;
        for (i, tok) in argv.iter().enumerate() {
            let text = &texts[i];
            if text == "-nocommands" {
                saw_nocommands = true;
                continue;
            }
            if text == "-nobackslashes" || text == "-novariables" {
                // Either flag changes the semantics our evaluator
                // assumes — refuse.
                return None;
            }
            if text.starts_with('-') {
                return None;
            }
            if !single.get(i + 1).copied().unwrap_or(false) {
                return None;
            }
            if tok.kind != TokenType::Str {
                return None;
            }
            if template_text.is_some() {
                // Multiple positionals — not the shape we recognise.
                return None;
            }
            template_text = Some(text.as_str());
        }
        if !saw_nocommands {
            return None;
        }
        let template = template_text?;
        if self.proc_depth == 0 {
            return None;
        }
        let scope = self.const_map_stack.last()?;
        crate::subst_nocommands::subst_nocommands(template, scope, self.config.braced_var)
    }

    /// Try to lower `eval ?body?` to a static-body
    /// [`Statement::Block`] when the body is a brace-string literal,
    /// a const-mapped `$var`, or an `eval [list lit1 lit2 ...]`
    /// command-substitution shape. Returns `None` for dynamic
    /// bodies so the caller falls through to runtime barrier
    /// dispatch.
    fn try_lower_eval_static(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        error_context: Option<tcl_registry::InlineBodyErrorContext>,
    ) -> Option<Statement> {
        use tcl_lexer::TokenType;
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        // Single-body shape only: ``eval $body`` / ``eval {body}``.
        // The list form (``eval cmd arg1 arg2``) keeps runtime
        // semantics — joining the words with spaces is observable.
        if args.len() != 1 || arg_tokens.is_empty() {
            return None;
        }
        let body_tok = arg_tokens[0];
        // Barrier gate: a braced literal body might contain a
        // nested ``eval $x`` / ``uplevel $lvl {...}`` whose own body
        // is still dynamic.  Relaxing the outer barrier in that case
        // produces IR that runs a compiled inner barrier with a
        // still-dynamic shape — we'd lose the runtime barrier without
        // gaining static knowledge.  Reject and fall back to the
        // default Statement::Barrier dispatch.
        if body_tok.kind == TokenType::Str {
            let body_text = &args[0];
            if body_has_dynamic_barrier(body_text, self.registry) {
                return None;
            }
        }
        let body = if body_tok.kind == TokenType::Str {
            let body_text = self.guarded_body_text(body_tok, &args[0]);
            let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
            self.lower_body(body_text, body_offset, namespace)
        } else if body_tok.kind == TokenType::Var {
            let literal = self.const_map_lookup(&args[0])?;
            if body_has_dynamic_barrier(&literal, self.registry) {
                return None;
            }
            self.lower_script(&literal, namespace)
        } else if body_tok.kind == TokenType::Cmd {
            // ``eval [list lit1 lit2 ...]`` — synthesise the
            // body by joining the list's literal arguments and
            // re-lowering. The bracket-substitution text retains
            // the surrounding ``[...]``; strip them via
            // ``content_offset`` if present, otherwise the helper
            // strips them itself.
            let inner_text = if body_tok.content_offset > 0 {
                let start = u32::from(body_tok.content_offset) as usize;
                &args[0][start..args[0].len() - start]
            } else {
                args[0].trim_start_matches('[').trim_end_matches(']')
            };
            let synthesised = eval_list_literal_body(inner_text)?;
            self.lower_script(&synthesised, namespace)
        } else {
            return None;
        };
        Some(Statement::Block {
            span: seg.span,
            body,
            namespace: namespace.to_string(),
            tokens: Some(Self::cmd_tokens(seg)),
            error_context,
        })
    }

    /// `apply {{params} body ?ns?} ?arg ...?` — an anonymous lambda.
    ///
    /// The body runs in a *separate* frame (its own locals, bound parameters),
    /// so — like `namespace eval` — the `apply` itself stays a runtime
    /// [`Statement::Barrier`]: codegen executes it via the runtime `apply` and
    /// the body's frame-local effects are conservatively opaque to the caller's
    /// CFG. But a braced literal body is still walked through [`Self::lower_body`]
    /// (in a fresh const-map / proc-depth frame, as `lower_proc` does) so nested
    /// `proc` definitions and other module-level effects register — where the
    /// old `lower_default` barrier discarded the body wholesale, losing them.
    ///
    /// A `$var` / `[cmd]` lambda, or a lambda whose body element is not a braced
    /// literal, stays fully opaque (nothing statically walkable).
    ///
    /// The body is walked in the *lambda's* namespace — element 2 of the lambda
    /// if present, else the global namespace `::` — not the caller's namespace,
    /// so a nested `proc` registers under the same qualified name Tcl's runtime
    /// `apply` would give it (per the `apply` manual).
    /// Register an already-lowered fresh-frame body (an `apply` lambda or a
    /// `namespace eval` block) as a synthetic *body unit* in
    /// [`Module::body_units`], so the static-analysis pipeline reaches inside
    /// it. `label` names the construct (`apply` / `namespace-eval`), `params`
    /// are the frame's bound names (empty for `namespace eval`), and `span`
    /// covers the body text (used by the complexity guard). Purely additive —
    /// the caller still emits the runtime `Statement::Barrier` that executes
    /// the body, so bytecode is unchanged.
    /// `label` is either a bare marker (`"apply"`, reducing to the global
    /// namespace once `#N`-suffixed — matching `apply`'s own default
    /// resolution namespace for the common 2-element-lambda form) or an
    /// already-namespace-qualified prefix (`"::ns::namespace-eval"`, from
    /// [`Self::lower_namespace_eval`]) — so the resulting `qualified` name's
    /// *enclosing* namespace (everything before the last `::`, the same
    /// convention every proc/method qname uses) is the namespace this body
    /// actually executes in, not always the global namespace. This is what
    /// lets [`crate::compilation_unit::build_extra_call_site_scan_contexts`]
    /// resolve a bare call inside the body against the *correct* namespace
    /// via [`crate::interprocedural::resolve_internal_call`], rather than
    /// silently defaulting every body unit to global.
    ///
    /// `is_lambda` marks the frame as a closed one whose parameter list binds
    /// locals (`apply`), recording it in [`Module::lambda_body_units`]; a
    /// `namespace eval` body is not.
    fn register_body_unit(
        &mut self,
        label: &str,
        params: Vec<String>,
        span: tcl_lexer::Span,
        body: Script,
        is_lambda: bool,
    ) {
        let n = self.body_unit_count;
        self.body_unit_count += 1;
        let prefix = label.strip_prefix("::").unwrap_or(label);
        let qualified = format!("::{prefix}#{n}");
        // `name` stays the short leaf marker (`"apply"` / `"namespace-eval"`)
        // even when `label` carries a namespace prefix — matching every
        // other `Procedure::name`'s "short name" contract.
        let short_name = prefix.rsplit("::").next().unwrap_or(prefix).to_string();
        if is_lambda {
            self.module.lambda_body_units.insert(qualified.clone());
        }
        self.module.body_units.insert(
            qualified.clone(),
            Procedure {
                name: short_name,
                qualified_name: qualified,
                params,
                span,
                body,
                params_raw: String::new(),
                body_source: None,
                namespace_scoped: false,
                base_priority: 500,
            },
        );
    }

    fn lower_apply(&mut self, seg: &SegmentedCommand) -> Statement {
        use tcl_lexer::TokenType;
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();

        // Flatten the braced literal lambda into its list elements (index 0 =
        // params, 1 = body, 2 = optional namespace). A newline splits *commands*
        // but not *list elements*, matching the analyser's `handle_apply_command`.
        let lambda_elems: Vec<(tcl_lexer::Token, String)> = match (args.first(), arg_tokens.first())
        {
            (Some(lambda_text), Some(&lambda_tok)) if lambda_tok.kind == TokenType::Str => {
                let base = lambda_tok.span.start() + u32::from(lambda_tok.content_offset);
                // The element spans below are this word's own spans rebased by
                // `base`, so the lambda text has to be the region verbatim
                // before it is split — see [`Self::guarded_body_text`].
                let lambda_text = self.guarded_body_text(lambda_tok, lambda_text);
                segment_commands_with_offset_and_config(lambda_text, base, self.config)
                    .iter()
                    .flat_map(|c| c.argv.iter().copied().zip(c.texts.iter().cloned()))
                    .collect()
            }
            _ => Vec::new(),
        };

        // The body element (index 1) must itself be a braced literal to walk.
        let body_info: Option<(String, u32)> = lambda_elems
            .get(1)
            .filter(|(tok, _)| tok.kind == TokenType::Str)
            .map(|(tok, text)| {
                (
                    self.guarded_body_text(*tok, text).to_owned(),
                    tok.span.start() + u32::from(tok.content_offset),
                )
            });

        // The lambda's bound parameters are element 0 of the lambda list, so a
        // `$param` read inside the body resolves to the param (not a caller
        // scalar). A parameter list `apply` itself would reject leaves the body
        // unwalked: with no trustworthy bound names, a body unit would resolve
        // its reads against the wrong frame.
        let params = match lambda_elems.first() {
            Some((_, param_text)) => parse_formal_param_names(param_text),
            None => Some(Vec::new()),
        };

        if let (Some((body_text, body_offset)), Some(params)) = (body_info, params) {
            // `apply` evaluates the body in the namespace named by lambda element
            // 2, or the *global* namespace when it is absent — never the caller's
            // namespace. Element 2 is interpreted relative to the **global**
            // namespace even when it does not start with `::` (`doc/apply.n`:
            // "If given, namespace is interpreted relative to the global
            // namespace even if its name does not start with ::"; `tclProc.c`
            // `TclNRApplyObjCmd` `::`-prefixes the word before the lookup), so
            // this must mirror the analyser's `handle_apply_command` exactly.
            let body_ns = match lambda_elems.get(2).map(|(_, t)| t.as_str()) {
                Some(ns) if !ns.is_empty() && !ns.starts_with('$') && !ns.starts_with('[') => {
                    join_namespace("::", ns)
                }
                _ => "::".to_string(),
            };
            // Fresh frame for the lambda body: its own runtime frame means the
            // caller's tracked scalars must not fold into it (a bare `$x` in the
            // body is an unbound local, not the caller's `x`).  Mirror
            // `lower_proc`'s const-map / proc-depth push.
            self.proc_depth += 1;
            self.const_map_stack.push(HashMap::new());
            let body = self.lower_body(&body_text, body_offset, &body_ns);
            self.const_map_stack.pop();
            self.proc_depth -= 1;

            // A body defined inside a TclOO method body is not registered
            // globally (`suppress_proc_register`), but a body unit is
            // analysis-only (never codegen-emitted), so it is always safe to
            // record for coverage.
            let span = tcl_lexer::Span::new(
                body_offset,
                body_offset + u32::try_from(body_text.len()).unwrap_or(u32::MAX),
            );
            // Prefix the body unit's qualified name with the namespace the
            // lambda actually runs in, exactly as `lower_namespace_eval` does
            // — a bare command word inside the body resolves against
            // `body_ns`, never the caller's own namespace, and
            // `interprocedural::resolve_internal_call` reads that off the
            // qname. `join_namespace` normalises the global case back to the
            // plain `apply` marker. tclsh8.6/9.0-confirmed: inside
            // `::foo::runIt`, `apply {{x} { helper $x } ::foo} b` calls
            // `::foo::helper`, not `::helper`.
            self.register_body_unit(&join_namespace(&body_ns, "apply"), params, span, body, true);
        }

        Statement::Barrier {
            span: seg.span,
            reason: "unsupported body command".into(),
            command: seg.name().into(),
            canonical_command: None,
            args: args.to_vec(),
            tokens: Some(Self::cmd_tokens(seg)),
        }
    }

    /// `array for {keyVar valueVar} arrayName body` (Tcl 9.0).
    ///
    /// The body runs in the caller's frame with the two loop variables bound
    /// per entry. C Tcl compiles this to an `invokeStk` of `::tcl::array::for`
    /// with the body pushed as an unparsed literal — it does not compile the
    /// body — so, exactly like [`Self::lower_apply`], the call itself stays a
    /// runtime [`Statement::Barrier`] (codegen unchanged, byte-identical to C
    /// Tcl). But a braced literal body is walked in a fresh frame bound to the
    /// loop variables and recorded as a body unit so static analysis reaches
    /// inside it. A `$var` / `[cmd]` body stays fully opaque.
    fn lower_array_for(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        use tcl_lexer::TokenType;
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        // `array for {k v} arr body` → args = [for, {k v}, arr, body].
        // The body (index 3) must be a single braced-literal token to walk.
        let body_is_braced = arg_tokens.get(3).is_some_and(|t| t.kind == TokenType::Str)
            && arg_single.get(3).copied() == Some(true);
        // A varList Tcl would refuse to split binds nothing statically, so it
        // falls through to the opaque runtime barrier below. Decided before the
        // body is walked, so a rejected loop leaves no const-map traces behind.
        if args.len() == 4
            && body_is_braced
            && let Some(vars) = parse_var_list_names(&args[1])
        {
            let body_tok = &arg_tokens[3];
            // The body runs in the *caller's* frame (`vm.eval_source` in place),
            // so lower it inline — inheriting the caller's const-map — into a
            // loop-variable-bound `Foreach` rather than isolating it in a
            // fresh-frame body unit. The analysis CFG inlines this into the
            // caller unit (so a body `$re` resolves to a caller literal for
            // regex-source / taint), while codegen barriers it to the byte-
            // identical `::tcl::array::for` invoke — see `lower_foreach_dispatch`.
            let body = self.lower_body_from_tok(&args[3], Some(body_tok), namespace);
            return Statement::Foreach {
                span: seg.span,
                iterators: vec![ForeachIterator {
                    vars,
                    list_arg: args[2].clone(),
                    // `array for {k v} {arr} …` — a braced array-name word
                    // is a literal name, not a substitution (issue #1260).
                    list_braced: Self::cmd_tokens(seg).arg_is_braced_literal(2),
                }],
                body,
                body_span: body_tok.span,
                is_lmap: false,
                raw_args: args.to_vec(),
                is_dict_iteration: false,
                is_array_iteration: true,
                raw_tokens: Some(Self::cmd_tokens(seg)),
            };
        }

        // A dynamic (`$body` / `[cmd]`) body stays fully opaque: the runtime
        // barrier re-emits `array for …`.
        self.lower_default(seg, namespace)
    }

    fn lower_namespace_eval(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        // The body is walked from its *written* spelling, rebased by the body
        // word's own content offset, so the word must not substitute (issue
        // #1375's gate).  A `$body` / `[list …]` word's value is run-time data
        // that appears nowhere in this document: rebasing its unsubstituted
        // spelling fabricates statement spans that overrun the source —
        // `namespace eval ::ns [list $o fw]` rebases the whole `[list $o fw]`
        // text past the opening `[`, one byte off the end.
        // [`Self::guarded_body_text`] deliberately passes such words through
        // untouched because clamping run-time data proves nothing; refusing to
        // walk them is the caller-side half of that contract.  The namespace is
        // still entered at run time — the `Barrier` below re-emits the call —
        // the body just stays opaque to static analysis, exactly as
        // [`Self::lower_apply`] and [`Self::lower_array_for`] leave a dynamic
        // body opaque.
        if !seg_word_substitutes(seg, 3) {
            self.walk_namespace_eval_body(seg, namespace);
        }

        Statement::Barrier {
            span: seg.span,
            reason: "namespace eval".into(),
            command: "namespace".into(),
            canonical_command: None,
            args: seg.args().to_vec(),
            tokens: Some(Self::cmd_tokens(seg)),
        }
    }

    /// Walk a `namespace eval` body inline and record it as a body unit.
    ///
    /// Only reached for a body word that does not substitute — see
    /// [`Self::lower_namespace_eval`] for why.
    fn walk_namespace_eval_body(&mut self, seg: &SegmentedCommand, namespace: &str) {
        let args = seg.args();
        let child_ns = join_namespace(namespace, &args[1]);
        let body_tok = seg.arg_tokens()[2];
        // `namespace eval` inlines whatever literal body word it is given,
        // including a compound `{…}x` one, so the rebase below needs the #1325
        // guard (see [`Self::guarded_body_text`]) — both for the body itself and
        // for the body unit's span derived from its length.
        let body_text = self.guarded_body_text(body_tok, &args[2]);
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let prev = self.in_namespace_eval;
        self.in_namespace_eval = true;
        let body = self.lower_body(body_text, body_offset, &child_ns);
        self.in_namespace_eval = prev;

        // A `namespace eval` body runs in the child namespace's own frame — its
        // straight-line statements are a real analysable script, so record them
        // as a body unit (no bound params: namespace vars, not locals).
        let span = tcl_lexer::Span::new(
            body_offset,
            body_offset + u32::try_from(body_text.len()).unwrap_or(u32::MAX),
        );
        // Prefix the body unit's own qualified name with `child_ns` (not just
        // the bare "namespace-eval" marker): a bare command inside this body
        // resolves against `child_ns`, never the caller's own namespace, so
        // the body unit's qname must encode that for
        // `interprocedural::resolve_internal_call` to get it right (see
        // `register_body_unit`'s doc). tclsh8.6-confirmed:
        // `namespace eval ::foo { helper }` calls `::foo::helper` even when
        // nested inside an unrelated proc, never a same-named proc in the
        // caller's own namespace.
        self.register_body_unit(
            &join_namespace(&child_ns, "namespace-eval"),
            vec![],
            span,
            body,
            false,
        );
    }

    /// Default lowering: generic [`Statement::Call`] with registry-based
    /// arg roles.
    fn lower_default(&self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let cmd_name = seg.name();
        let args = seg.args();

        // Resolve alias for arg role lookups.  When `cmd_name`
        // resolves to a different alias target, populate
        // `canonical_command` with the resolved name so downstream
        // dispatch (codegen hook lookup, side-effect classification,
        // GVN purity, var-escape) can key off the canonical target
        // instead of re-resolving from the source spelling.
        let mut role_cmd = cmd_name.to_owned();
        let mut role_args: Vec<String> = args.to_vec();
        let mut prepend_n: usize = 0;
        let mut canonical: Option<String> = None;
        if let Some((target, prepended)) = resolve_alias(cmd_name, &self.aliases, namespace) {
            // Only record canonical_command when the alias resolved
            // to a different target — ``cmd_name == target`` means
            // the source already names the canonical form.
            if target != cmd_name {
                canonical = Some(target.clone());
            }
            role_cmd = target;
            let mut new_args: Vec<String> = prepended;
            new_args.extend_from_slice(args);
            prepend_n = new_args.len() - args.len();
            role_args = new_args;
        }

        let role_args_ref: Vec<&str> = role_args.iter().map(String::as_str).collect();
        let body_indices =
            self.registry
                .arg_indices_for_role(&role_cmd, &role_args_ref, ArgRole::Body);
        let var_indices = if self.registry.frame_effect(&role_cmd).is_some_and(|effect| {
            effect.layout == tcl_registry::frame_effect::FrameArgLayout::AliasPairs
        }) {
            // Frame alias pairs are handled by the analyser's upvar grammar;
            // treating their local-name slots as ordinary VarWrite defs makes
            // an aliased upvar falsely silence W210.  Keep the prepended-level
            // vector above for the other registry role queries, but do not
            // manufacture generic Call defs for this layout.
            Vec::new()
        } else {
            self.registry
                .arg_indices_for_role(&role_cmd, &role_args_ref, ArgRole::VarWrite)
        };
        let var_read_indices =
            self.registry
                .arg_indices_for_role(&role_cmd, &role_args_ref, ArgRole::VarRead);
        // Read-modify-write commands (`lset` / `lpop` / `ledit` — like
        // `incr` / `append` / `lappend`, which are hook-lowered) read the
        // current value of their target before rewriting it, so the prior
        // definition is live. Carry that as `reads_own_defs` so dead-store /
        // unused-variable analysis does not treat a feeding `set` as dead.
        let reads_before_write = self
            .registry
            .get(&role_cmd)
            .is_some_and(|s| s.traits.contains(tcl_registry::Traits::READS_BEFORE_WRITE));

        if !body_indices.is_empty() {
            return Statement::Barrier {
                span: seg.span,
                reason: "unsupported body command".into(),
                command: cmd_name.into(),
                canonical_command: canonical.clone(),
                args: args.to_vec(),
                tokens: Some(Self::cmd_tokens(seg)),
            };
        }

        if !var_indices.is_empty() || !var_read_indices.is_empty() {
            // A brace-quoted name word (`set {$n} 1`, `unset {$n}`) is Tcl's
            // literal spelling for a name that contains `$` / `[`: it
            // substitutes nothing, so the word's content **is** the variable
            // name (issue #1078). The de-braced `args` text cannot show that;
            // the word's own token kind can.
            let braced_at = |real: usize| {
                matches!(seg.argv.get(real + 1).map(|t| t.kind), Some(TokenType::Str))
                    && seg.single_token_word.get(real + 1).copied().unwrap_or(true)
            };
            let var_defs: Vec<String> = var_indices
                .iter()
                .filter_map(|&i| {
                    let real = i.checked_sub(prepend_n)?;
                    args.get(real).map(|a| {
                        crate::naming::element_var_name_braced(a, braced_at(real)).to_owned()
                    })
                })
                .collect();
            let var_reads: Vec<String> = var_read_indices
                .iter()
                .filter_map(|&i| {
                    let real = i.checked_sub(prepend_n)?;
                    args.get(real).map(|a| {
                        crate::naming::element_var_name_braced(a, braced_at(real)).to_owned()
                    })
                })
                .collect();
            return Statement::Call {
                span: seg.span,
                command: cmd_name.into(),
                canonical_command: canonical.clone(),
                args: args.to_vec(),
                defs: var_defs,
                reads: var_reads,
                reads_own_defs: reads_before_write,
                safe_on_uninit: self.safe_on_uninit(&role_cmd, &role_args),
                tokens: Some(Self::cmd_tokens(seg)),
                foreach_groups: None,
            };
        }

        Statement::Call {
            span: seg.span,
            command: cmd_name.into(),
            canonical_command: canonical,
            args: args.to_vec(),
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(Self::cmd_tokens(seg)),
            foreach_groups: None,
        }
    }

    // TclOO method-body lowering (method purity for O126)

    /// Populate `module.methods` from the fully-assembled IR.
    ///
    /// Runs as a cache-independent post-pass (mirroring
    /// [`populate_trace_facts`]): clone the top-level script and proc
    /// bodies, then walk them — plus any `namespace eval` blocks — for
    /// `oo::class create` / `oo::define` barriers and lift each
    /// `method` / `constructor` / `destructor` body to an
    /// [`MethodDef`]. Method bodies are lowered for analysis only —
    /// codegen never reads `module.methods`, and the barrier emitted
    /// for the class command is unchanged, so bytecode is
    /// byte-identical.
    pub fn extract_oo_methods_pass(&mut self) {
        let top_level = self.module.top_level.clone();
        self.walk_for_oo_methods(&top_level.statements, "::");
        let procs: Vec<(String, Script)> = self
            .module
            .procedures
            .iter()
            .map(|(q, p)| (q.clone(), p.body.clone()))
            .collect();
        for (qname, body) in &procs {
            let proc_ns = proc_namespace(qname);
            self.walk_for_oo_methods(&body.statements, &proc_ns);
        }
        // Every definition block for every class has now been walked, so the
        // per-class instance-variable union is complete.  Merge it into each
        // method: a method extracted from an early block must still see state
        // a later `oo::define` block declared for its class, or a consumer
        // reading `MethodDef::instance_vars` (the optimiser's method-body
        // propagation gate, `elimination.rs`'s dead-store protection) would
        // treat object state as a private local.  Order-free by construction —
        // the union does not depend on which block was walked first.
        for method in self.module.methods.values_mut() {
            if let Some(class_vars) = self.class_instance_vars.get(&method.class_name) {
                method.instance_vars.extend(class_vars.iter().cloned());
            }
        }
        // Retained replacement bodies (issue #1166) are scanned by the same
        // consumers, so they need the same whole-class instance-variable
        // union.
        for method in self.module.redefined_methods.values_mut().flatten() {
            if let Some(class_vars) = self.class_instance_vars.get(&method.class_name) {
                method.instance_vars.extend(class_vars.iter().cloned());
            }
        }
    }

    /// Recursive walk for `oo::class` / `oo::define` definition
    /// barriers. Descends `namespace eval` blocks (tracking the active
    /// namespace) and extracts methods from any class/define command
    /// carrying a static braced body. The lowerer evaluates a
    /// `namespace eval` body inline and discards it (emitting a
    /// `Barrier`), so rather than descending a preserved block this
    /// walk re-segments the barrier's body token instead (see
    /// [`Self::walk_segments_for_oo`]).
    fn walk_for_oo_methods(&mut self, statements: &[Statement], namespace: &str) {
        for stmt in statements {
            match stmt {
                // Folded `eval` / `uplevel` bodies survive as Blocks.
                Statement::Block {
                    body,
                    namespace: block_ns,
                    ..
                } => {
                    let ns = if block_ns.is_empty() {
                        namespace
                    } else {
                        block_ns.as_str()
                    };
                    self.walk_for_oo_methods(&body.statements, ns);
                }
                Statement::Call {
                    command,
                    canonical_command,
                    tokens: Some(ct),
                    ..
                }
                | Statement::Barrier {
                    command,
                    canonical_command,
                    tokens: Some(ct),
                    ..
                } => {
                    if let Some(call) = self.classify_definer_call(
                        command,
                        canonical_command.as_deref(),
                        &ct.argv_texts,
                        &ct.argv_kinds,
                        &ct.single_token_word,
                    ) {
                        self.extract_definer_members(
                            call,
                            &ct.argv_texts,
                            ct.argv[call.body_idx].start() + 1,
                            namespace,
                        );
                    } else if is_namespace_eval_shape(
                        command,
                        &ct.argv_texts,
                        &ct.argv_kinds,
                        &ct.single_token_word,
                    ) {
                        // The body was lowered inline and discarded;
                        // re-segment it to find classes defined directly
                        // inside the namespace.
                        let child_ns = join_namespace(namespace, &ct.argv_texts[2]);
                        let body_off = ct.argv[3].start() + 1;
                        let segments = segment_commands_with_offset_and_config(
                            &ct.argv_texts[3],
                            body_off,
                            self.config,
                        );
                        self.walk_segments_for_oo(&segments, &child_ns);
                    }
                }
                _ => {}
            }
        }
    }

    /// Segment-level counterpart of [`Self::walk_for_oo_methods`] used
    /// for `namespace eval` bodies (which the lowerer discards). Walks
    /// re-segmented commands, recursing through nested `namespace eval`
    /// and extracting members from any registry-classified definer block.
    fn walk_segments_for_oo(&mut self, segments: &[SegmentedCommand], namespace: &str) {
        for seg in segments {
            if seg.is_partial || seg.texts.is_empty() {
                continue;
            }
            let kinds: Vec<TokenType> = seg.argv.iter().map(|t| t.kind).collect();
            let cmd = seg.texts[0].as_str();
            if let Some(call) =
                self.classify_definer_call(cmd, None, &seg.texts, &kinds, &seg.single_token_word)
            {
                let body = seg.argv[call.body_idx];
                let off = body.span.start() + u32::from(body.content_offset);
                self.extract_definer_members(call, &seg.texts, off, namespace);
            } else if is_namespace_eval_shape(cmd, &seg.texts, &kinds, &seg.single_token_word) {
                let child_ns = join_namespace(namespace, &seg.texts[2]);
                let off = seg.argv[3].span.start() + u32::from(seg.argv[3].content_offset);
                let sub = segment_commands_with_offset_and_config(&seg.texts[3], off, self.config);
                self.walk_segments_for_oo(&sub, &child_ns);
            }
        }
    }

    /// Recognise one class-body member whose registry grammar marks every
    /// argument as a class reference (`superclass A B`, `mixin M` —
    /// [`tcl_registry::definer::MemberRefKind::Class`]) and record each
    /// literal argument as a hierarchy relation of `class_qname` (issue
    /// #1164); a dynamic argument widens
    /// [`crate::ir::OoDefinitionEvidence::dynamic_class_relations`]
    /// instead. Returns `true` when the member was consumed here.
    fn record_class_relation_member(
        &mut self,
        definer_grammar: Option<&tcl_registry::definer::DefinitionBodyGrammar>,
        seg: &SegmentedCommand,
        class_qname: &str,
    ) -> bool {
        let head = seg.texts[0].as_str();
        let Some(member) = definer_grammar.and_then(|g| g.member(head)) else {
            return false;
        };
        if member.all_args_ref != Some(tcl_registry::definer::MemberRefKind::Class) {
            return false;
        }
        for word in &seg.texts[1..] {
            if word.contains('$') || word.contains('[') {
                // Dynamic related-class word: this class's ancestry is
                // unknown — hierarchy-scoped consumers must widen to
                // whole-module.
                self.module.oo_evidence.dynamic_class_relations = true;
            } else {
                self.module
                    .class_relations
                    .push((class_qname.to_owned(), word.clone()));
            }
        }
        true
    }

    /// Lift the method-frame member bodies inside one definer block to
    /// per-method [`MethodDef`] entries keyed by `{class_qname}::{name}`
    /// (nameless members use synthetic names — `<constructor>`,
    /// `<destructor>`, `<typeconstructor>`).  Member recognition and
    /// argument layout come from the definer's registry grammar
    /// ([`DefinerCall::grammar`], issue #1172), so `oo::objdefine`, snit,
    /// and itcl bodies produce method units exactly as `oo::class create` /
    /// `oo::define` always did — no member keyword is matched for layout.
    ///
    /// `texts` is the definer command's full per-word text array;
    /// `body_content_offset` is the absolute source offset of the first
    /// byte inside the body's braces.
    fn extract_definer_members(
        &mut self,
        call: DefinerCall,
        texts: &[String],
        body_content_offset: u32,
        namespace: &str,
    ) {
        let target = texts[call.name_idx].as_str();
        let body_text = texts[call.body_idx].as_str();
        let class_qname = if call.per_object {
            // An `oo::objdefine` receiver is usually a substitution
            // (`oo::objdefine $obj { … }`) — that is the common shape and
            // the point of the exercise (issue #1172 item 1), so it is NOT
            // skipped as a dynamic name.  The per-object members home under
            // a synthetic, unrepresentable class name keyed by the
            // receiver's written tail (mirroring the analyser's
            // `::@objdefine@::…` keying), so they never collide with a real
            // class or pollute its instance-variable union.
            let tail = target.trim().trim_start_matches('$');
            let tail = tail.trim_matches(|c| c == '{' || c == '}');
            if tail.is_empty() || tail.contains('[') {
                return;
            }
            format!("::@objdefine@::{tail}")
        } else {
            // Dynamic class names can't be resolved statically — and the
            // block may (re)define methods on ANY class, so whole-module OO
            // evidence is incomplete (issue #1166: the propagation barrier
            // and method purity must widen rather than trust the bodies
            // they can see).
            if target.contains('$') || target.contains('[') {
                self.module.oo_evidence.dynamic_target = true;
                return;
            }
            qualify_proc_name(namespace, target)
        };
        let segments =
            segment_commands_with_offset_and_config(body_text, body_content_offset, self.config);

        let class_ivars = declared_member_vars(call.grammar, &segments);
        // This block sees only its own declarations; a sibling `oo::define`
        // block may declare more state for the same class, and may be walked
        // *after* the methods that use it.  Accumulate the whole-class union
        // for `extract_oo_methods_pass`'s merge — see
        // [`Self::class_instance_vars`].  A per-object block accumulates
        // under its own synthetic key, so per-object `variable` declarations
        // stay per-object by construction (oracle case K, issue #1129).
        if !class_ivars.is_empty() {
            self.class_instance_vars
                .entry(class_qname.clone())
                .or_default()
                .extend(class_ivars.iter().cloned());
        }

        self.extract_members_from_segments(&call, &segments, &class_qname, &class_ivars, namespace);
    }

    /// The per-segment member walk behind [`Self::extract_definer_members`],
    /// factored out so a wrapper's block form (`self { … }` /
    /// `private { … }`) can recurse into its nested definition script with
    /// the same class context.
    fn extract_members_from_segments(
        &mut self,
        call: &DefinerCall,
        segments: &[SegmentedCommand],
        class_qname: &str,
        class_ivars: &HashSet<String>,
        namespace: &str,
    ) {
        for seg in segments {
            if seg.is_partial || seg.texts.is_empty() {
                continue;
            }
            let head = seg.texts[0].as_str();
            if self.record_class_relation_member(Some(call.grammar), seg, class_qname) {
                continue;
            }
            let Some(member) = call.grammar.member(head) else {
                continue;
            };
            // A wrapper member (`self …`, `private …`, itcl's access
            // modifiers) either prefixes an inner member (shift one place
            // right) or — for `wrapper_block_body` wrappers — carries a
            // whole nested definition script to recurse into.
            let (member, kw, base, wrapper) = match member.kind {
                tcl_registry::definer::MemberKind::Wrapper => match seg.texts.get(1) {
                    Some(inner) if call.grammar.is_member(inner) => {
                        let inner_member = call.grammar.member(inner).expect("checked is_member");
                        // No double wrapping (`self private method …` is not
                        // a real Tcl shape).
                        if inner_member.kind != tcl_registry::definer::MemberKind::Flat {
                            continue;
                        }
                        (inner_member, inner.as_str(), 2usize, Some(head))
                    }
                    Some(_)
                        if member.wrapper_block_body
                            && seg.texts.len() >= 2
                            && seg_word_is_static_braced(seg, 1) =>
                    {
                        // `self { … }` / `private { … }` — a nested
                        // definition script with the same member grammar.
                        let block_tok = seg.argv[1];
                        let off = block_tok.span.start() + u32::from(block_tok.content_offset);
                        let sub = segment_commands_with_offset_and_config(
                            &seg.texts[1],
                            off,
                            self.config,
                        );
                        // A `self { variable v }` declares per-class-object
                        // state, not instance state; keep it out of the
                        // instance union — only the members are lifted.
                        let wrapped_call = DefinerCall { ..*call };
                        let empty = HashSet::new();
                        let ivars = if head == "self" { &empty } else { class_ivars };
                        self.extract_members_from_wrapper_block(
                            &wrapped_call,
                            &sub,
                            class_qname,
                            ivars,
                            namespace,
                            head,
                        );
                        continue;
                    }
                    _ => continue,
                },
                tcl_registry::definer::MemberKind::Flat => (member, head, 1usize, None),
                // Flag-keyed bodies (`property … -get/-set …`) are accessor
                // scripts, not method frames — no unit today (documented
                // limit).
                tcl_registry::definer::MemberKind::FlagKeyed => continue,
            };
            self.extract_one_member(
                MemberExtraction {
                    call,
                    seg,
                    member,
                    kw,
                    base,
                    wrapper,
                },
                class_qname,
                class_ivars,
                namespace,
            );
        }
    }

    /// Recurse into a wrapper's block form with the wrapper name forced —
    /// `self { method m … }` records `m` as a class-object method, and
    /// `private { method m … }` as an instance method, exactly like their
    /// prefix spellings.
    fn extract_members_from_wrapper_block(
        &mut self,
        call: &DefinerCall,
        segments: &[SegmentedCommand],
        class_qname: &str,
        class_ivars: &HashSet<String>,
        namespace: &str,
        wrapper: &str,
    ) {
        for seg in segments {
            if seg.is_partial || seg.texts.is_empty() {
                continue;
            }
            let head = seg.texts[0].as_str();
            let Some(member) = call.grammar.member(head) else {
                continue;
            };
            if member.kind != tcl_registry::definer::MemberKind::Flat {
                continue;
            }
            self.extract_one_member(
                MemberExtraction {
                    call,
                    seg,
                    member,
                    kw: head,
                    base: 1,
                    wrapper: Some(wrapper),
                },
                class_qname,
                class_ivars,
                namespace,
            );
        }
    }

    /// Lift one body-bearing member call to a [`MethodDef`] unit, when its
    /// grammar layout and this consumer's kind routing say it opens a
    /// method frame.
    fn extract_one_member(
        &mut self,
        ex: MemberExtraction<'_, '_>,
        class_qname: &str,
        class_ivars: &HashSet<String>,
        namespace: &str,
    ) {
        let MemberExtraction {
            call,
            seg,
            member,
            kw,
            base,
            wrapper,
        } = ex;
        let args = &seg.texts[base..];
        let Some(kind) = member_method_kind(kw, wrapper == Some("self")) else {
            return;
        };
        // Argument layout comes from the grammar: which relative index (0-
        // based after the keyword) is the body / name / parameter list.
        let Some(body_rel) = member.indices_for_call(args, ArgRole::Body).next() else {
            return;
        };
        // A member that also declares a variable (itcl `variable NAME ?init?
        // ?configbody?`, snit 1.x `onconfigure`) is a declaration whose
        // trailing script is not an ordinary method frame — skipped
        // (documented limit; `member_method_kind` already excludes them by
        // keyword, this keeps the exclusion structural too).
        if member
            .indices_for_call(args, ArgRole::VarWrite)
            .next()
            .is_some()
        {
            return;
        }
        let b_idx = base + body_rel;
        let name_owned: String;
        let name: &str = if let Some(rel) = member.indices_for_call(args, ArgRole::Name).next() {
            let Some(n) = seg.texts.get(base + rel) else {
                return;
            };
            n.as_str()
        } else {
            // Nameless members: the synthetic id is the keyword, matching
            // the established `<constructor>` / `<destructor>` scheme.
            name_owned = format!("<{kw}>");
            &name_owned
        };
        // Dynamic method names / non-static bodies are left un-lowered —
        // and recorded as an unanalysable member of the class (issue
        // #1166): any method of the class may have been (re)defined with a
        // body no scan can read, so per-class analysis must abstain for
        // the whole class.
        if name.contains('$') || name.contains('[') || !seg_word_is_static_braced(seg, b_idx) {
            self.module
                .oo_unanalysed_classes
                .insert(class_qname.to_string());
            return;
        }
        // A parameter list Tcl would reject defines no method, and the class as
        // a whole becomes unanalysable — the same abstention the dynamic name /
        // body check above makes.
        let Some(params) = member_param_names(member, args, seg, base) else {
            self.module
                .oo_unanalysed_classes
                .insert(class_qname.to_string());
            return;
        };
        // Lower the method body in its own frame (fresh const-map +
        // proc-depth) so the enclosing scope's tracked scalars
        // don't leak into the body's barrier-relaxation gate —
        // exactly as the `proc` case does — and suppress
        // nested-proc registration while doing so.
        let body_tok = seg.argv[b_idx];
        let body_off = body_tok.span.start() + u32::from(body_tok.content_offset);
        self.proc_depth += 1;
        self.const_map_stack.push(HashMap::new());
        let prev_suppress = self.suppress_proc_register;
        self.suppress_proc_register = true;
        let body_script = self.lower_body(seg.texts[b_idx].as_str(), body_off, namespace);
        self.suppress_proc_register = prev_suppress;
        self.const_map_stack.pop();
        self.proc_depth -= 1;

        // Instance vars in scope: class-level decls (plus the grammar's
        // implicit member-body variables — snit's `self`/`selfns`/`type`/
        // `options`, a widget's `win`/`hull`, itcl's `this`) plus this
        // method's own top-level `variable` declarations. A write to any of
        // these mutates object state (impure for O126).
        let mut method_ivars = class_ivars.clone();
        method_ivars.extend(call.grammar.implicit_vars.iter().map(|s| (*s).to_string()));
        for st in &body_script.statements {
            if let Statement::Call {
                command,
                canonical_command,
                args,
                ..
            } = st
                && canonical_matches(command, canonical_command.as_deref(), "variable")
            {
                for nm in args {
                    if is_instance_var_name(nm) {
                        method_ivars.insert(normalise_var_name(nm).to_string());
                    }
                }
            }
        }

        let method_qname = format!("{class_qname}::{name}");
        let def = MethodDef {
            class_name: class_qname.to_string(),
            method_name: name.to_string(),
            params,
            body: body_script,
            kind: MethodKind::from_str_lossy(kind),
            span: Some(seg.span),
            instance_vars: method_ivars,
        };
        // First definition wins for the stored body (matches proc
        // registration), but a redefinition (a later `oo::define`
        // or a duplicate in-body `method`) replaces the body at
        // runtime — we can't statically know which body a given
        // dispatch runs, so RETAIN the replacement in definition
        // order (issue #1166): analysis consumers scan every retained
        // body (the union over-approximates whichever is live) rather
        // than abstaining on the mere fact of redefinition.
        if self.module.methods.contains_key(&method_qname) {
            self.module
                .redefined_methods
                .entry(method_qname)
                .or_default()
                .push(def);
            return;
        }
        self.module.methods.insert(method_qname, def);
    }
}

/// Formal-parameter names of a body-bearing member, read from the word its
/// grammar marks as the parameter list (`base + rel` in `seg.texts`).
///
/// An absent or empty word is the nullary member `method m {} {…}`, so it
/// yields no names; `None` is a list Tcl itself would reject.
fn member_param_names(
    member: &tcl_registry::definer::MemberSpec,
    args: &[String],
    seg: &SegmentedCommand,
    base: usize,
) -> Option<Vec<String>> {
    let param_text = member
        .indices_for_call(args, ArgRole::ParamList)
        .next()
        .and_then(|rel| seg.texts.get(base + rel))
        .filter(|text| !text.is_empty());
    match param_text {
        Some(text) => parse_formal_param_names(text),
        None => Some(Vec::new()),
    }
}

/// One body-bearing member call ready to lift — bundles the walk context so
/// [`Lowerer::extract_one_member`] stays under the argument limit.
#[derive(Clone, Copy)]
struct MemberExtraction<'a, 'b> {
    call: &'a DefinerCall,
    seg: &'a SegmentedCommand,
    member: &'b tcl_registry::definer::MemberSpec,
    /// The effective member keyword (the inner one for a wrapper prefix).
    kw: &'a str,
    /// Index of the member's first argument word in `seg.texts` (1, or 2
    /// past a wrapper prefix).
    base: usize,
    /// The wrapper the member was written under, when any (`self`,
    /// `private`, itcl's access modifiers).
    wrapper: Option<&'a str>,
}

/// Which [`MethodDef`] kind a member keyword's body opens, or `None` for
/// members whose trailing script is **not** a method frame (`initialise` /
/// `initialize` evaluate a *definition script* in the class object's
/// namespace; `property` accessors are flag-keyed scripts; declarations
/// carry no frame at all).
///
/// Routing a member keyword to its `MethodDef` kind is the analyser-local
/// semantics AGENTS.md's definition-body contract leaves with the consumer
/// (an object `destructor` and a class-level `initialise` are structurally
/// identical single-body members — the difference is frame modelling, not
/// command structure).  Recognition and argument layout still come from the
/// registry grammar; this routes only.
fn member_method_kind(kw: &str, wrapped_in_self: bool) -> Option<&'static str> {
    Some(match kw {
        "method" if wrapped_in_self => "classmethod",
        // snit's `typemethod` / `typeconstructor` dispatch on the type
        // command with no instance in frame — the class-method shape.
        "classmethod" | "typemethod" | "typeconstructor" => "classmethod",
        // A snit / itcl class-scoped `proc` opens a fresh frame like a
        // method (with no instance state auto-bound; the over-approximated
        // instance-var set only widens abstention, never a false claim).
        "method" | "proc" => "method",
        "constructor" => "constructor",
        "destructor" => "destructor",
        _ => return None,
    })
}

/// The instance variables one definition body declares at class level, per
/// the definer's grammar: every `all_args_var` member's names (the `TclOO`
/// `variable` slot — skipping a leading slot-operation word, which names no
/// variable) plus every `VarWrite`-role name (snit's `variable` /
/// `typevariable` / `component` / `typecomponent`, itcl's `variable` /
/// `common`), including itcl's access-modifier-wrapped spellings.
///
/// The result is the **union of every name ever declared** across the
/// block: slot removal operations (`variable -remove a`, `-set`, `-clear`)
/// are deliberately not folded here — the union is the conservative
/// direction for every consumer of [`crate::ir::MethodDef::instance_vars`]
/// (existence-fold abstention, W-family known-bound, O126 impurity), where
/// a stale extra name only widens abstention while a missed name produces a
/// false claim.  A *sibling* definition block may declare more, which is
/// why the caller accumulates a per-class union rather than using this
/// result directly.
fn declared_member_vars(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    segments: &[crate::segmenter::SegmentedCommand],
) -> HashSet<String> {
    let mut out = HashSet::new();
    for seg in segments {
        if seg.is_partial || seg.texts.is_empty() {
            continue;
        }
        let head = seg.texts[0].as_str();
        let Some(member) = grammar.member(head) else {
            continue;
        };
        // Unwrap an access-modifier prefix (itcl `public variable x`,
        // TclOO 9's `private variable x`) one level.
        let (member, args): (&tcl_registry::definer::MemberSpec, &[String]) =
            if member.kind == tcl_registry::definer::MemberKind::Wrapper {
                match seg.texts.get(1).and_then(|inner| grammar.member(inner)) {
                    Some(inner_member)
                        if inner_member.kind == tcl_registry::definer::MemberKind::Flat
                            && seg.texts.len() >= 3 =>
                    {
                        (inner_member, &seg.texts[2..])
                    }
                    _ => continue,
                }
            } else {
                (member, &seg.texts[1..])
            };
        if member.all_args_var {
            // The `TclOO` `variable` slot: a leading operation word names
            // no variable (issue #1169).
            let values = match member.slot {
                Some(slot) => match slot.split_call(args) {
                    Some((_, values)) => values,
                    None => continue,
                },
                None => args,
            };
            for nm in values {
                if is_instance_var_name(nm) {
                    out.insert(normalise_var_name(nm).to_string());
                }
            }
        } else {
            for rel in member.indices_for_call(args, ArgRole::VarWrite) {
                if let Some(nm) = args.get(rel)
                    && is_instance_var_name(nm)
                {
                    out.insert(normalise_var_name(nm).to_string());
                }
            }
        }
    }
    out
}

// Public API

/// Lower Tcl source to an IR module.
///
/// This is the main entry point for the lowering phase.  Lexes with the
/// default (Tcl-8.5+) config and parses expressions in the plain-Tcl grammar;
/// use [`lower_to_ir_with_dialect`] to honour a document's dialect.
#[must_use]
pub fn lower_to_ir(source: &str, registry: &CommandRegistry) -> Module {
    lower_to_ir_with_config(source, registry, tcl_lexer::LexerConfig::default())
}

/// Lower a single `proc` body in isolation at `proc_depth == 1` (a fresh
/// [`Lowerer`] with an empty const-map frame), returning the **offset-0** body
/// [`Script`].  Replicates the body-lowering setup `lower_proc` performs for a
/// static literal body (`proc_depth += 1`; push an empty const-map;
/// `lower_body(text, 0, namespace)`).
///
/// For a body free of cross-item context (no nested `proc` / `namespace
/// import`/`export` / command alias / const-map materialisation), this is
/// byte-identical to the body the whole-file lowering produces for that
/// procedure, normalised to offset 0 — the seam the SRV-INCREMENTAL per-procedure
/// lowering memo (Task 3) keys on the offset-0 body text and feeds back through
/// [`Lowerer::with_body_cache`].
#[must_use]
pub fn lower_proc_body_isolated(
    body_text: &str,
    namespace: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Script {
    let mut lowerer = Lowerer::with_config(registry, config).with_dialect(dialect);
    lowerer.proc_depth += 1;
    lowerer.irules_execution_context = IrulesExecutionContext::ProcedureBody;
    lowerer.const_map_stack.push(HashMap::new());
    let body = lowerer.lower_body(body_text, 0, namespace);
    lowerer.const_map_stack.pop();
    lowerer.proc_depth -= 1;
    body
}

/// Whether a single top-level `proc` body can be lowered in isolation (through
/// the SRV-INCREMENTAL Task 3 body-cache memo) byte-identically to the in-place
/// [`Lowerer::lower_body`].
///
/// The isolated lowering runs against a fresh [`Lowerer`] with an empty
/// const-map frame, so it drops every *cross-item* side effect `lower_body`
/// performs while lowering a body — registering a nested [`Procedure`], tracking
/// `namespace import`/`export`, recording command aliases, and `TclOO` / `when`
/// definitions. A body qualifies exactly when it carries none of those
/// constructs, so its lowering is a pure function of `(body_text, namespace,
/// dialect, config)`.
///
/// This is a **per-body** gate: a context-carrying sibling `proc` (or top-level
/// `namespace eval` / OO / `when` code) no longer disqualifies the *other* bodies
/// in the same file. The scan is deliberately conservative — a false negative
/// only forgoes reuse (falling back to the identical in-place lowering); a false
/// positive would corrupt the IR — and is backstopped by the corpus differential
/// gates.
///
/// **Precondition:** the caller must also check [`source_may_alias_commands`] at
/// the *file* level. A command alias declared outside any body (`interp alias`)
/// populates the lowerer's alias table that `resolve_alias` consults while
/// lowering every body; this per-body scan cannot see it, so a file that
/// establishes aliases must not install the cache at all.
#[must_use]
pub fn body_cache_eligible(body: &str) -> bool {
    // Command keywords whose body-level use carries a cross-item effect the
    // isolated lowering drops. `proc` covers a nested procedure definition.
    // Matched as a word followed by Tcl inter-word whitespace (space, tab, CR,
    // newline, or a `\`-continuation), so a tab-separated `interp\talias` trips
    // it too.
    const WORD_DISQUALIFIERS: &[&str] = &[
        "namespace",
        "interp",
        "rename",
        "method",
        "when",
        "apply",
        "alias",
        "proc",
    ];
    // Substring markers (the token itself is the marker — no trailing arg).
    const SUBSTR_DISQUALIFIERS: &[&str] = &["oo::", "::oo", "itcl"];
    !SUBSTR_DISQUALIFIERS.iter().any(|kw| body.contains(kw))
        && !WORD_DISQUALIFIERS
            .iter()
            .any(|kw| contains_word_followed_by_ws(body, kw))
}

/// Whether `source` may establish a command alias (`interp alias {} name {}
/// target`, or a static `rename old new`) that a cached proc body could
/// reference.
///
/// `interp alias` and `rename` both populate the lowerer's alias table, and
/// `resolve_alias` consults it while lowering *every* subsequent body — but
/// the isolated body-cache lowering starts with an empty table, so an alias
/// declared at the top level (outside any body the per-body
/// [`body_cache_eligible`] scan inspects) would silently resolve differently
/// there. The whole file must therefore forgo the body cache when this
/// returns `true`. `interp` and `rename` are the only commands that feed the
/// alias table (see `detect_interp_alias` / `detect_rename`), so scanning for
/// either as a word is both sufficient and conservative.
#[must_use]
pub fn source_may_alias_commands(source: &str) -> bool {
    contains_word_followed_by_ws(source, "interp") || contains_word_followed_by_ws(source, "rename")
}

/// True when `word` occurs in `source` followed by Tcl inter-word whitespace —
/// i.e. used as a command with an argument. Ignores the character *before* the
/// word (a preceding non-boundary only over-disqualifies, which is safe).
fn contains_word_followed_by_ws(source: &str, word: &str) -> bool {
    source.match_indices(word).any(|(i, _)| {
        source
            .as_bytes()
            .get(i + word.len())
            .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n' | b'\\'))
    })
}

#[cfg(test)]
mod body_cache_eligible_tests {
    use super::body_cache_eligible;

    #[test]
    fn plain_bodies_are_eligible() {
        assert!(body_cache_eligible("set x 1"));
        assert!(body_cache_eligible("puts \"hi $name\"\nreturn $x"));
        assert!(body_cache_eligible("if {$x} { foo } else { bar }"));
        assert!(body_cache_eligible(""));
    }

    #[test]
    fn cross_item_constructs_disqualify() {
        for body in [
            "namespace eval x { }",
            "namespace import ::ns::*",
            "interp alias {} x {} y",
            "rename set myset",
            "apply {{} { }}",
            "when HTTP_REQUEST { }",
            "oo::class create C { }",
            "itcl::class C { }",
            "proc inner {} { }",
        ] {
            assert!(!body_cache_eligible(body), "should disqualify: {body}");
        }
    }

    #[test]
    fn tab_separated_command_still_disqualifies() {
        assert!(!body_cache_eligible("interp\talias {} x {} y"));
        assert!(!body_cache_eligible("namespace\timport ::ns::*"));
    }

    #[test]
    fn substring_only_use_is_safe() {
        assert!(body_cache_eligible("set myproclist {a b c}"));
        assert!(body_cache_eligible("set renamed 1"));
    }

    #[test]
    fn source_may_alias_commands_flags_interp() {
        use super::source_may_alias_commands;
        // `interp alias` establishes an alias a cached body cannot see.
        assert!(source_may_alias_commands(
            "interp alias {} = {} expr\nproc f {} { = 1 }\n"
        ));
        assert!(source_may_alias_commands("interp\talias {} = {} expr"));
        // No `interp` -> safe to cache.
        assert!(!source_may_alias_commands(
            "proc f {x} { set y $x }\nproc g {} { return 1 }\n"
        ));
        // `interp` as a substring of a bareword does not trip it.
        assert!(!source_may_alias_commands("set interpreter 1"));
    }

    // Proves *why* `source_may_alias_commands` must guard the file (Codex #739
    // P2): with a top-level `interp alias {} = {} expr` in scope, lowering `f`'s
    // body through the isolated body cache (empty alias table) resolves `=` as an
    // unknown command, whereas the in-place whole-file lowering resolves it to
    // `expr` — so the module IRs differ. The db therefore must not install the
    // cache for such a file.
    #[test]
    fn alias_in_scope_makes_body_cache_diverge() {
        use tcl_registry::CommandRegistry;

        use super::{
            lower_proc_body_isolated, lower_to_ir_with_body_cache, lower_to_ir_with_config,
        };

        let reg = CommandRegistry::build_default();
        let cfg = tcl_lexer::LexerConfig::default();
        let src = "interp alias {} = {} expr\nproc f {x} { return [= {$x + 1}] }\n";
        let cache = |body: &str, ns: &str| lower_proc_body_isolated(body, ns, &reg, cfg, None);
        let cached = lower_to_ir_with_body_cache(src, &reg, cfg, None, &cache);
        let fresh = lower_to_ir_with_config(src, &reg, cfg);
        assert_ne!(
            format!("{cached:?}"),
            format!("{fresh:?}"),
            "an alias-in-scope body cache is expected to diverge from the in-place lowering"
        );
    }

    #[test]
    fn source_may_alias_commands_flags_rename() {
        use super::source_may_alias_commands;
        // A static `rename` establishes an alias a cached body cannot see —
        // same hazard as `interp alias`, see `detect_rename`.
        assert!(source_may_alias_commands(
            "rename puts myputs\nproc f {x} { myputs $x }\n"
        ));
        assert!(source_may_alias_commands("rename\tputs myputs"));
        // `rename` as a substring of a bareword does not trip it.
        assert!(!source_may_alias_commands("set renamed 1"));
    }

    // Same hazard as `alias_in_scope_makes_body_cache_diverge`, for a static
    // `rename`: `rename puts myputs` at the top level must make `f`'s body
    // resolve `myputs` to `puts`, which the isolated (empty-alias-table) body
    // cache cannot see.
    #[test]
    fn rename_in_scope_makes_body_cache_diverge() {
        use tcl_registry::CommandRegistry;

        use super::{
            lower_proc_body_isolated, lower_to_ir_with_body_cache, lower_to_ir_with_config,
        };

        let reg = CommandRegistry::build_default();
        let cfg = tcl_lexer::LexerConfig::default();
        let src = "rename puts myputs\nproc f {x} { myputs $x }\n";
        let cache = |body: &str, ns: &str| lower_proc_body_isolated(body, ns, &reg, cfg, None);
        let cached = lower_to_ir_with_body_cache(src, &reg, cfg, None, &cache);
        let fresh = lower_to_ir_with_config(src, &reg, cfg);
        assert_ne!(
            format!("{cached:?}"),
            format!("{fresh:?}"),
            "a rename-in-scope body cache is expected to diverge from the in-place lowering"
        );
    }
}

/// Like [`lower_to_ir_with_dialect`] but with a memoised per-procedure body-lowering
/// callback (SRV-INCREMENTAL Task 3): a top-level `proc`'s static body is lowered
/// through `body_cache` `(offset-0 body text, namespace) -> offset-0 Script` and
/// rebased, so an unchanged proc's body IR is reused across edits.  The caller must
/// only install a cache for **context-free** files (see [`Lowerer::body_cache`]);
/// byte-identity is guarded by the corpus differential gates.
///
/// The cached bodies must be produced by [`lower_proc_body_isolated`] under the
/// **same** `dialect`, since the dialect selects the expression grammar the
/// body's conditions are parsed with.
#[must_use]
pub fn lower_to_ir_with_body_cache(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    body_cache: &dyn Fn(&str, &str) -> Script,
) -> Module {
    lower_with(
        Lowerer::with_config(registry, config)
            .with_dialect(dialect)
            .with_body_cache(body_cache),
        source,
    )
}

/// Like [`lower_to_ir`] but with an explicit dialect
/// [`tcl_lexer::LexerConfig`], threaded into every body re-segmentation
/// so `{*}` expansion (off for Tcl 8.4 / iRules) and the iRules `}{`
/// ghost SEP are honoured.
///
/// Expressions are still parsed in the plain-Tcl grammar — a caller that
/// knows the dialect *name* should use [`lower_to_ir_with_dialect`], which
/// also gives `if` / `while` / `for` / `expr` the dialect's operator set.
#[must_use]
pub fn lower_to_ir_with_config(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Module {
    lower_with(Lowerer::with_config(registry, config), source)
}

/// Like [`lower_to_ir_with_config`] but also naming the document's analysis
/// `dialect` (`""` for plain Tcl), so every `if` / `while` / `for` condition
/// and every inlined `expr` is parsed with that dialect's operator set.
///
/// This is what makes an iRules word-operator condition — `if {$x contains
/// "cd"}` — reach the IR as a real
/// [`BinOp::Contains`](crate::expr_ast::BinOp::Contains) comparison instead of
/// an opaque [`ExprNode::Raw`](crate::expr_ast::ExprNode::Raw), which is the
/// only shape the constant folder (and the I230 constant-condition
/// diagnostic behind it) can evaluate.
#[must_use]
pub fn lower_to_ir_with_dialect(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Module {
    lower_with(
        Lowerer::with_config(registry, config).with_dialect(dialect),
        source,
    )
}

/// Like [`lower_to_ir`] but for the bytecode/VM compile path: constructs the
/// backend can't compile correctly (`try`, and a `foreach`/`lmap` directly
/// nesting another) are lowered to runtime-command barriers (see
/// [`Lowerer::for_bytecode`]). Analysis callers keep the structured IR via
/// [`lower_to_ir`].
#[must_use]
pub fn lower_to_ir_for_bytecode(source: &str, registry: &CommandRegistry) -> Module {
    lower_to_ir_for_bytecode_with_dialect(source, registry, tcl_lexer::LexerConfig::default(), None)
}

/// Like [`lower_to_ir_for_bytecode`] but also naming the document's `dialect`
/// (`""` for plain Tcl) — the bytecode-path counterpart of
/// [`lower_to_ir_with_dialect`], so a dialect expression such as an iRules
/// word-operator condition compiles to its dedicated opcode rather than the
/// generic runtime-`expr` fallback.
#[must_use]
pub fn lower_to_ir_for_bytecode_with_dialect(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Module {
    lower_with(
        Lowerer::with_config(registry, config)
            .with_dialect(dialect)
            .for_bytecode_backend(),
        source,
    )
}

/// Like [`lower_to_ir_for_bytecode`] but with every inline/structured
/// lowering hook suppressed (see [`Lowerer::trace_visible`]) — every command
/// in `source` compiles to a plain runtime dispatch, so an execution trace
/// observes it. For recompiling a proc/script body once a step-capable
/// execution trace targets it (issue #946); see
/// [`tcl_runtime_api::CompileService::compile_traced`].
#[must_use]
pub fn lower_to_ir_traced(source: &str, registry: &CommandRegistry) -> Module {
    lower_to_ir_traced_with_config(source, registry, tcl_lexer::LexerConfig::default())
}

/// Like [`lower_to_ir_traced`] but with an explicit dialect
/// [`tcl_lexer::LexerConfig`], so a version-pinned host's traced recompiles
/// parse under the same grammar as its ordinary compiles (issue #1462).
#[must_use]
pub fn lower_to_ir_traced_with_config(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Module {
    lower_to_ir_traced_with_dialect(source, registry, config, None)
}

/// Like [`lower_to_ir_traced_with_config`] but also records the exact dialect
/// selected by the host.  Runtime compile services for a named profile must
/// use this entry point so the resulting bytecode artifact retains the
/// profile identity that its lexer, registry, and expression grammar used.
#[must_use]
pub fn lower_to_ir_traced_with_dialect(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Module {
    lower_with(
        Lowerer::with_config(registry, config)
            .with_dialect(dialect)
            .for_bytecode_backend()
            .trace_visible(),
        source,
    )
}

/// Lexer warning messages that correspond to a hard `Tcl_ParseCommand`
/// failure in C Tcl — i.e. a malformed word that C Tcl reports as a parse
/// error rather than tokenising leniently.
///
/// The lexer detects these but only *raises* them under `strict_quoting`
/// (off by default); in the VM compile path it records them as warnings and
/// recovers. C Tcl, however, aborts compilation of the script and defers the
/// error to a bytecode instruction that raises it at runtime — so a parse
/// error inside a `catch {…}` body (re-compiled at runtime) is catchable. See
/// [`first_fatal_parse_error`].
const FATAL_PARSE_MESSAGES: &[&str] = &[
    "extra characters after close-quote",
    "extra characters after close-brace",
    "missing close-brace",
    "missing close-brace for variable name",
    "missing \"",
    "missing )",
    "missing close-bracket",
];

/// Return the first hard parse-error message in `source`, in source order, or
/// `None` if `source` parses cleanly.
///
/// A VM compile front-end (`CompileService::compile`) calls this so that a
/// malformed script becomes a catchable runtime error carrying the **exact**
/// C Tcl message (e.g. `extra characters after close-quote`), matching C Tcl's
/// deferral of a compile-time parse error to a runtime instruction. The lexer
/// otherwise recovers from these for the benefit of editor diagnostics, so the
/// IR / bytecode never sees them.
///
/// Lexes with the default (Tcl-8.5+) dialect config — the same one
/// [`lower_to_ir_for_bytecode`] uses. Only the messages in
/// [`FATAL_PARSE_MESSAGES`] count; benign warnings are ignored.
#[must_use]
pub fn first_fatal_parse_error(source: &str) -> Option<String> {
    first_fatal_parse_error_with_config(source, tcl_lexer::LexerConfig::default())
}

/// Like [`first_fatal_parse_error`] but lexing under an explicit dialect
/// [`tcl_lexer::LexerConfig`], so a version-pinned host rejects exactly what
/// its emulated release rejects — under the Tcl 8.4 grammar `{*}{a b}` is a
/// hard `extra characters after close-brace`, which the default (8.5+) config
/// lexes as an ordinary expansion (issue #1462).
#[must_use]
pub fn first_fatal_parse_error_with_config(
    source: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let lexer = tcl_lexer::Lexer::with_source_map(tcl_lexer::SourceMap::new(source), config);
    let (_tokens, warnings) = lexer.tokenise_all_with_warnings().ok()?;
    warnings
        .into_iter()
        .filter(|w| FATAL_PARSE_MESSAGES.contains(&w.message.as_str()))
        .min_by_key(|w| w.offset)
        .map(|w| w.message)
}

/// Drive a configured [`Lowerer`] to a finished [`Module`] (the shared tail of
/// the `lower_to_ir*` entry points).
fn lower_with(mut lowerer: Lowerer<'_>, source: &str) -> Module {
    lowerer.lower(source);
    // Extract TclOO method bodies from the fully-assembled
    // module (cache-independent — see `extract_oo_methods_pass`).
    lowerer.extract_oo_methods_pass();
    let registry = lowerer.registry;
    let dialect = lowerer.dialect.map(|profile| profile.name.to_owned());
    let mut module = lowerer.module;
    module.source = source.to_string();
    module.dialect = dialect;
    populate_trace_facts(&mut module, registry);
    module
}

/// Post-lower scan for `trace add ...` calls that populates
/// `Module::traced_commands` / `has_dynamic_trace` (execution traces)
/// and `Module::traced_variables` / `has_dynamic_variable_trace`
/// (variable traces).
///
/// Runs over the top-level script + every procedure body + every `TclOO`
/// method body (`module.methods`, already populated by
/// `extract_oo_methods_pass` before this runs).  Literal command names
/// land in `traced_commands` (`::`-stripped to match the canonical key);
/// non-literal targets (`$cmd`, `[expr ...]`, command substitutions) flip
/// `has_dynamic_trace` so GVN treats every call as potentially traced.
/// Literal variable names targeted by any `Traits::
/// ESTABLISHES_VARIABLE_TRACE` subcommand — the registry's
/// `ArgRole::VarWrite` resolution for `trace add|remove|variable|vdelete`,
/// covering both the modern and deprecated legacy spellings — land in
/// `traced_variables`; non-literal targets flip
/// `has_dynamic_variable_trace`.
fn populate_trace_facts(module: &mut Module, registry: &CommandRegistry) {
    let top_level = module.top_level.clone();
    walk_for_trace(&top_level, module, registry, 0);
    // Every statically-known frame, not just named procedures: a `trace`
    // call inside a `namespace eval` / `apply` body (`Module::body_units`)
    // or a `TclOO` method (`Module::methods`) is just as live as one inside
    // a `proc` — omitting either class here would silently under-populate
    // `traced_variables` the moment an optimiser pass starts reaching into
    // those bodies (`CompilationUnit::all_body_function_units`).
    let bodies: Vec<Script> = module
        .procedures
        .values()
        .map(|p| p.body.clone())
        .chain(module.body_units.values().map(|p| p.body.clone()))
        .chain(module.methods.values().map(|m| m.body.clone()))
        .collect();
    for body in &bodies {
        walk_for_trace(body, module, registry, 0);
    }
    let method_bodies: Vec<Script> = module.methods.values().map(|m| m.body.clone()).collect();
    for body in &method_bodies {
        walk_for_trace(body, module, registry, 0);
    }
}

/// Resolve a `trace add|remove` type word (`variable`/`command`/
/// `execution`) against C Tcl 9.0's `Tcl_GetIndexFromObj` abbreviation
/// rule: a unique, non-empty prefix is accepted (`trace add e foo enter
/// h` installs the same execution trace as the full spelling, checked
/// against tclsh 8.6.14). Mirrors
/// `tcl_registry::commands::tcl::trace`'s private resolver of the same
/// name — duplicated rather than exposed across the crate boundary for
/// one three-word list.
fn resolve_trace_type_word(word: &str) -> Option<&'static str> {
    const TYPES: &[&str] = &["variable", "command", "execution"];
    if word.is_empty() {
        return None;
    }
    let mut hits = TYPES.iter().copied().filter(|t| t.starts_with(word));
    let first = hits.next()?;
    if hits.next().is_some() {
        return None; // ambiguous prefix
    }
    Some(first)
}

/// `depth` is the nesting level of `script` — see [`MAX_LOWER_NEST_DEPTH`]
/// (this post-lower scan reuses the same cap `lower_script`/`lower_body`
/// already build every `Script` under, for consistency; every input this
/// walk sees is already transitively bounded to that depth by construction,
/// so this is defence-in-depth rather than a currently-reachable path).
fn walk_for_trace(script: &Script, module: &mut Module, registry: &CommandRegistry, depth: u32) {
    use crate::ir::Statement;
    if MAX_LOWER_NEST_DEPTH.exceeded(depth) {
        return;
    }
    for stmt in &script.statements {
        match stmt {
            // Canonical (alias-resolved) name, so `interp alias exampleTrace trace`
            // still lands in `traced_commands`/`traced_variables` instead of being
            // silently missed because the call site spells it `exampleTrace ...`.
            Statement::Call { args, .. } | Statement::Barrier { args, .. }
                if stmt.canonical_command_or_source().trim_start_matches("::") == "trace" =>
            {
                let command = stmt.canonical_command_or_source();
                let is_add = args.first().is_some_and(|w| {
                    registry
                        .get(command)
                        .and_then(|spec| spec.resolve_subcommand(w))
                        .is_some_and(|sub| sub.name == "add")
                });
                if args.len() >= 4
                    && is_add
                    && resolve_trace_type_word(&args[1]) == Some("execution")
                {
                    let target = &args[2];
                    if is_literal_trace_target(target) {
                        let canonical = target.trim_start_matches("::").to_string();
                        if !canonical.is_empty() {
                            module.traced_commands.insert(canonical);
                        }
                    } else {
                        module.has_dynamic_trace = true;
                    }
                }
                populate_variable_trace_facts(command, args, module, registry);
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    walk_for_trace(&c.body, module, registry, depth + 1);
                }
                if let Some(e) = else_body {
                    walk_for_trace(e, module, registry, depth + 1);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                walk_for_trace(init, module, registry, depth + 1);
                walk_for_trace(next, module, registry, depth + 1);
                walk_for_trace(body, module, registry, depth + 1);
            }
            Statement::While { body, .. }
            | Statement::Foreach { body, .. }
            | Statement::Catch { body, .. }
            | Statement::Block { body, .. }
            | Statement::UpFrame { body, .. } => walk_for_trace(body, module, registry, depth + 1),
            Statement::Switch {
                arms, default_body, ..
            } => {
                for arm in arms {
                    if let Some(b) = &arm.body {
                        walk_for_trace(b, module, registry, depth + 1);
                    }
                }
                if let Some(b) = default_body {
                    walk_for_trace(b, module, registry, depth + 1);
                }
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                walk_for_trace(body, module, registry, depth + 1);
                for h in handlers {
                    walk_for_trace(&h.body, module, registry, depth + 1);
                }
                if let Some(f) = finally_body {
                    walk_for_trace(f, module, registry, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// Word indices of every variable-trace target that `command`/`args`
/// installs or removes an active trace on — any subcommand carrying
/// `Traits::ESTABLISHES_VARIABLE_TRACE` (`trace add`/`remove`/
/// `variable`/`vdelete` — not the read-only `info`/`vinfo` forms),
/// located via the registry's `ArgRole::VarWrite` resolution. Entirely
/// data-driven off the registry — no hardcoded knowledge of `trace`'s
/// subcommand grammar (`add` vs the legacy `variable` spelling, which
/// argument position holds the name, …). Shared by
/// [`populate_variable_trace_facts`] (the whole-module fact) and
/// `var_observability::stmt_gen` (the flow-sensitive `TRACED` mark), so
/// both derive the same trace-target positions from one query.
pub(crate) fn variable_trace_write_indices(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
) -> Vec<usize> {
    let Some(spec) = registry.get(command) else {
        return Vec::new();
    };
    let Some(sub) = args.first().and_then(|s| spec.resolve_subcommand(s)) else {
        return Vec::new();
    };
    if !sub
        .traits
        .contains(tcl_registry::prelude::Traits::ESTABLISHES_VARIABLE_TRACE)
    {
        return Vec::new();
    }
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    registry.arg_indices_for_role(command, &arg_strs, ArgRole::VarWrite)
}

/// Registry-driven half of [`walk_for_trace`]: record every literal
/// variable name a `trace` call targets via a
/// `Traits::ESTABLISHES_VARIABLE_TRACE` subcommand (`add`/`remove`/
/// `variable`/`vdelete` — not the read-only `info`/`vinfo` forms).
/// Dispatch is entirely data-driven off the registry's
/// `ArgRole::VarWrite` resolution — this function has no hardcoded
/// knowledge of `trace`'s subcommand grammar (`add` vs the legacy
/// `variable` spelling, which argument position holds the name, …).
fn populate_variable_trace_facts(
    command: &str,
    args: &[String],
    module: &mut Module,
    registry: &CommandRegistry,
) {
    for idx in variable_trace_write_indices(registry, command, args) {
        let Some(target) = args.get(idx) else {
            continue;
        };
        if is_literal_trace_target(target) {
            // `::`-stripped to match the canonical key — mirrors
            // `traced_commands`'s treatment of an execution-trace
            // target, so a top-level `set x 5` (chain key `x`) matches
            // a `trace add variable ::x ...` target the same way an
            // unqualified `trace add variable x ...` would.
            let canonical = crate::naming::normalise_var_name(&format!("${target}"))
                .trim_start_matches("::")
                .to_string();
            if !canonical.is_empty() {
                module.traced_variables.insert(canonical);
            }
        } else {
            module.has_dynamic_variable_trace = true;
        }
    }
}

/// True when a `trace` target word names a static variable literally —
/// no substitution, quoting, or whitespace.  Shared with the optimiser's
/// scope-alias / traced-global scans so every consumer applies one
/// literalness rule.
pub(crate) fn is_literal_trace_target(s: &str) -> bool {
    !s.is_empty()
        && !s.contains('$')
        && !s.contains('[')
        && !s.contains('{')
        && !s.contains('"')
        && !s.contains(' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn irules_event_priorities_follow_file_state_and_keep_repeated_handlers() {
        let profile = tcl_dialect::DialectProfile::irules();
        let registry = tcl_registry::registry_for_profile(profile);
        let module = lower_to_ir_with_config(
            "priority 700\n\
             when HTTP_REQUEST {}\n\
             priority 400\n\
             when HTTP_REQUEST {}\n\
             when HTTP_REQUEST priority 200 {}\n\
             when HTTP_REQUEST {}",
            registry,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        );

        let priorities: Vec<_> = [
            "::when::HTTP_REQUEST",
            "::when::HTTP_REQUEST#1",
            "::when::HTTP_REQUEST#2",
            "::when::HTTP_REQUEST#3",
        ]
        .into_iter()
        .map(|name| module.procedures[name].base_priority)
        .collect();
        assert_eq!(priorities, [700, 400, 200, 400]);
    }

    #[test]
    fn irules_declaration_body_shape_gates_lowered_regions() {
        let profile = tcl_dialect::DialectProfile::irules();
        let registry = tcl_registry::registry_for_profile(profile);
        let module = lower_to_ir_with_config(
            "when HTTP_REQUEST priority 1001 {}\n\
             when HTTP_REQUEST bare_body\n\
             when CLIENT_DATA \"quoted body\"\n\
             proc bare_proc {} bare_body\n\
             proc quoted_proc {} \"return\"\n\
             proc valid_proc {} { return }\n\
             when NOT_A_REAL_EVENT {}\n\
             when HTTP_REQUEST priority 200 {}\n\
             when HTTP_REQUEST {}",
            registry,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        );

        assert!(
            module.procedures.contains_key("::when::NOT_A_REAL_EVENT"),
            "a syntactically valid unknown event is lowered so the XC translator can report it"
        );
        for name in ["::when::CLIENT_DATA", "::bare_proc", "::quoted_proc"] {
            assert!(
                !module.procedures.contains_key(name),
                "a non-braced declaration body must not open {name}"
            );
        }
        assert!(module.procedures.contains_key("::valid_proc"));
        let request = [
            module.procedures["::when::HTTP_REQUEST"].base_priority,
            module.procedures["::when::HTTP_REQUEST#1"].base_priority,
        ];
        assert_eq!(request, [200, 500], "valid repeated handlers keep priority");
    }

    #[test]
    fn irules_declarations_only_lower_at_the_file_surface() {
        let profile = tcl_dialect::DialectProfile::irules();
        let registry = tcl_registry::registry_for_profile(profile);
        let module = lower_to_ir_with_config(
            "if {1} {\n\
                 when CLIENT_DATA { pool top_level_nested_when }\n\
                 proc top_level_nested_proc {} { pool top_level_nested_proc }\n\
             }\n\
             when HTTP_REQUEST {\n\
                 if {1} {\n\
                     when SERVER_DATA { pool event_nested_when }\n\
                     proc event_nested_proc {} { pool event_nested_proc }\n\
                 }\n\
                 pool live_event\n\
             }\n\
             proc helper {} {\n\
                 if {1} {\n\
                     when HTTP_RESPONSE { pool proc_nested_when }\n\
                     proc proc_nested_proc {} { pool proc_nested_proc }\n\
                 }\n\
                 pool live_proc\n\
             }\n",
            registry,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        );

        for absent in [
            "::when::CLIENT_DATA",
            "::when::SERVER_DATA",
            "::when::HTTP_RESPONSE",
            "::top_level_nested_proc",
            "::event_nested_proc",
            "::proc_nested_proc",
        ] {
            assert!(
                !module.procedures.contains_key(absent),
                "nested declaration must remain inert: {absent}"
            );
        }
        assert!(module.procedures.contains_key("::when::HTTP_REQUEST"));
        assert!(module.procedures.contains_key("::helper"));
    }

    #[test]
    fn static_bool_accepts_unique_prefix_booleans() {
        // A condition folds boolean words by unique prefix, like Tcl
        // (`if {tr}` runs, `if {of}` does not — verified against tclsh).
        assert_eq!(static_bool("tr"), Some(true));
        assert_eq!(static_bool(" ye "), Some(true));
        assert_eq!(static_bool("on"), Some(true));
        assert_eq!(static_bool("of"), Some(false));
        assert_eq!(static_bool("n"), Some(false));
        assert_eq!(static_bool("0"), Some(false));
        assert_eq!(static_bool("1"), Some(true));
        // Ambiguous `o` (on/off) is not a static boolean.
        assert_eq!(static_bool("o"), None);
        assert_eq!(static_bool("$x"), None);
    }

    #[test]
    fn lowering_is_dialect_aware_via_expand_syntax() {
        // Lowering re-segments each
        // body under the document dialect.  For `if {*}$cond { puts hi }`,
        // on 8.5+ the `{*}` expands the condition, so the structured `if`
        // trips the expand-barrier (an opaque `Statement::Barrier` — the
        // structure can't be reasoned about); on 8.4 `{*}` is a literal
        // word, so the `if` lowers normally (no barrier).
        let reg = reg();
        let src = "if {*}$cond { puts hi }";
        let m90 = lower_to_ir_with_config(src, &reg, tcl_lexer::LexerConfig::default());
        let m84 = lower_to_ir_with_config(src, &reg, tcl_lexer::LexerConfig::for_dialect("tcl8.4"));
        let is_barrier = |m: &Module| {
            matches!(
                m.top_level.statements.first(),
                Some(Statement::Barrier { .. })
            )
        };
        assert!(
            is_barrier(&m90),
            "9.0 expands `{{*}}` → structured `if` becomes a barrier: {:?}",
            m90.top_level.statements,
        );
        assert!(
            !is_barrier(&m84),
            "8.4 treats `{{*}}` as literal → `if` lowers without a barrier: {:?}",
            m84.top_level.statements,
        );
    }

    /// Issue #1380: the `{*}` barrier used to be a nine-name list
    /// (`proc`/`when`/`namespace`/`if`/`switch`/`for`/`while`/`foreach`/
    /// `foreach_in_collection`) while `try_dispatch_structured_hook` covers
    /// seventeen typed hook IDs, so every structured form missing from the
    /// list committed to its *un-expanded* argv — `lmap i {*}$spec {…}`
    /// fabricated an `IRForeach` with one iterator over the literal word
    /// `${spec}`, which is what made W210 claim a loop variable was read
    /// before it was set.
    #[test]
    fn every_argv_shaped_hook_barriers_on_expansion() {
        let reg = reg();
        for src in [
            // Forms the old name list already covered.
            "foreach i {*}$spec {puts $i}",
            "if {*}$cond {puts hi}",
            "switch {*}$spec {a {puts hi}}",
            "while {*}$spec {puts hi}",
            "for {*}$spec {puts hi}",
            "proc {*}$spec {puts hi}",
            "namespace eval {*}$spec {puts hi}",
            // Forms it missed.
            "lmap i {*}$spec {puts $i}",
            "dict for {k v} {*}$rest {puts $k}",
            "array for {k v} {*}$rest {puts $k}",
            "foreachLine ln {*}$rest {puts $ln}",
            "catch {puts hi} {*}$rest",
            "try {puts hi} {*}$rest",
            "eval {*}$rest",
            "uplevel {*}$rest",
            "apply {*}$rest",
        ] {
            let m = lower_to_ir(src, &reg);
            assert!(
                matches!(
                    m.top_level.statements.first(),
                    Some(Statement::Barrier { .. })
                ),
                "`{src}` must barrier on `{{*}}` expansion: {:?}",
                m.top_level.statements,
            );
        }
    }

    /// The barrier is an *expansion* gate, not a blanket one: the same
    /// structured forms without `{*}` keep their specialised lowering, and a
    /// non-structured hook (`set`) never barriers on expansion at all — its
    /// own `has_expansion` check declines and the call reaches
    /// `lower_default`.
    #[test]
    fn expansion_barrier_leaves_unexpanded_and_non_structured_forms_alone() {
        let reg = reg();
        for src in [
            "foreach i $spec {puts $i}",
            "lmap i $spec {puts $i}",
            "catch {puts hi} err",
            "set {*}$rest",
            "puts {*}$rest",
        ] {
            let m = lower_to_ir(src, &reg);
            assert!(
                !matches!(
                    m.top_level.statements.first(),
                    Some(Statement::Barrier { .. })
                ),
                "`{src}` must not raise the expansion barrier: {:?}",
                m.top_level.statements,
            );
        }
    }

    /// The barrier reason names the command, which the compiler-explorer
    /// `--show ir` view and the audit notes in issue #1380 both quote.
    #[test]
    fn expansion_barrier_reason_names_the_command() {
        let m = lower_to_ir("lmap i {*}$spec {puts $i}", &reg());
        let Some(Statement::Barrier { reason, .. }) = m.top_level.statements.first() else {
            panic!("expected a barrier: {:?}", m.top_level.statements);
        };
        assert_eq!(reason, "lmap with argument expansion");
    }

    #[test]
    fn empty_source() {
        let m = lower_to_ir("", &reg());
        assert!(m.top_level.statements.is_empty());
    }

    #[test]
    fn simple_set() {
        let m = lower_to_ir("set x 1", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::AssignConst { name, value, .. } if name == "x" && value == "1"
        ));
    }

    #[test]
    fn set_with_variable_value() {
        let m = lower_to_ir("set y $x", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::AssignValue { name, .. } if name == "y"
        ));
    }

    #[test]
    fn incr_command() {
        let m = lower_to_ir("incr i", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Incr { name, .. } if name == "i"
        ));
    }

    // The `extract_oo_methods_pass` post-pass
    // lifts TclOO method bodies into `module.methods`.

    #[test]
    fn extracts_oo_class_methods() {
        let src = "oo::class create Counter {\n\
                   \x20   variable n\n\
                   \x20   constructor {} { set n 0 }\n\
                   \x20   method bump {} { incr n }\n\
                   \x20   method get {} { return $n }\n\
                   }\n";
        let m = lower_to_ir(src, &reg());
        let mut keys: Vec<&String> = m.methods.keys().collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                &"::Counter::<constructor>".to_string(),
                &"::Counter::bump".to_string(),
                &"::Counter::get".to_string(),
            ],
            "methods: {keys:?}",
        );
        let get = &m.methods["::Counter::get"];
        assert_eq!(get.method_name, "get");
        assert_eq!(get.kind, MethodKind::Method);
        assert_eq!(get.class_name, "::Counter");
        // `variable n` in the class body is an in-scope instance var
        // for every method (auto-linked).
        assert!(
            get.instance_vars.contains("n"),
            "get ivars: {:?}",
            get.instance_vars
        );
        // The method body is lowered for analysis (the `incr n` /
        // `return $n` statements are present, not an empty barrier).
        let bump = &m.methods["::Counter::bump"];
        assert!(
            !bump.body.statements.is_empty(),
            "bump body should be lowered"
        );
        let ctor = &m.methods["::Counter::<constructor>"];
        assert_eq!(ctor.kind, MethodKind::Constructor);
        assert!(m.redefined_methods.is_empty());
    }

    #[test]
    fn lowers_oo_configurable_class_body() {
        // `oo::configurable create` (and `oo::abstract` / `oo::singleton`)
        // share the `METACLASS create NAME { body }` shape, so their bodies
        // must be lowered and their methods lifted like `oo::class`'s — not left
        // as an unanalysed barrier (issue #797: a `Device` defined with
        // `oo::configurable` otherwise had every method body skipped).
        let src = "oo::configurable create Pin {\n\
                   \x20   property node\n\
                   \x20   method describe {} { return [my configure -node] }\n\
                   }\n";
        let m = lower_to_ir(src, &reg());
        assert!(
            m.methods.contains_key("::Pin::describe"),
            "oo::configurable method must be lifted; methods: {:?}",
            m.methods.keys().collect::<Vec<_>>()
        );
        assert!(
            !m.methods["::Pin::describe"].body.statements.is_empty(),
            "the method body should be lowered, not barriered"
        );
    }

    #[test]
    fn extracts_oo_define_methods_and_namespaced_class() {
        // `oo::define` adds methods to an existing class, and a class
        // created inside `namespace eval` qualifies under that ns.
        let src = "namespace eval ::app {\n\
                   \x20   oo::class create Widget {\n\
                   \x20       method draw {} { return ok }\n\
                   \x20   }\n\
                   }\n\
                   oo::define ::app::Widget {\n\
                   \x20   method hide {} { return done }\n\
                   }\n";
        let m = lower_to_ir(src, &reg());
        assert!(
            m.methods.contains_key("::app::Widget::draw"),
            "methods: {:?}",
            m.methods.keys().collect::<Vec<_>>()
        );
        assert!(
            m.methods.contains_key("::app::Widget::hide"),
            "methods: {:?}",
            m.methods.keys().collect::<Vec<_>>()
        );
    }

    // Issue #1172: extraction is driven by the registry definer grammars,
    // so oo::objdefine, snit, and itcl bodies produce method units too.

    // TP (item 1): an `oo::objdefine $obj { … }` body produces per-object
    // method units — previously no `MethodDef` existed at all, leaving the
    // body invisible to every analysis.
    #[test]
    fn objdefine_bodies_produce_per_object_method_units() {
        let src = "oo::class create C {}\n\
                   set k [C new]\n\
                   oo::objdefine $k {\n\
                   \x20   variable z\n\
                   \x20   method probe {} { return $z }\n\
                   }\n";
        let m = lower_to_ir(src, &reg());
        let unit = m
            .methods
            .get("::@objdefine@::k::probe")
            .unwrap_or_else(|| panic!("methods: {:?}", m.methods.keys().collect::<Vec<_>>()));
        assert_eq!(unit.method_name, "probe");
        assert!(!unit.body.statements.is_empty(), "body must be lowered");
        // The block's `variable z` is per-object state in scope for the
        // per-object method.
        assert!(
            unit.instance_vars.contains("z"),
            "ivars: {:?}",
            unit.instance_vars
        );
    }

    // TN (oracle case K, issue #1129): objdefine `variable` declarations are
    // per-object — they must NOT pollute the real class's cross-block
    // instance-variable union.
    #[test]
    fn objdefine_variables_stay_out_of_the_class_union() {
        let src = "oo::class create C {\n\
                   \x20   method m {} { info exists z }\n\
                   }\n\
                   set k [C new]\n\
                   oo::objdefine $k { variable z }\n";
        let m = lower_to_ir(src, &reg());
        let class_m = &m.methods["::C::m"];
        assert!(
            !class_m.instance_vars.contains("z"),
            "per-object `z` leaked into the class union: {:?}",
            class_m.instance_vars
        );
    }

    // TP (item 2): snit method / typemethod / constructor bodies become
    // method units, with the grammar's implicit member-body variables
    // (self / selfns / type / options) and declared type variables in scope.
    #[test]
    fn snit_type_bodies_produce_method_units() {
        let src = "snit::type Dog {\n\
                   \x20   variable name\n\
                   \x20   constructor {args} { set name fido }\n\
                   \x20   method bark {} { return \"$name barks\" }\n\
                   \x20   typemethod count {} { return 0 }\n\
                   }\n";
        let m = lower_to_ir(src, &reg());
        let bark = m
            .methods
            .get("::Dog::bark")
            .unwrap_or_else(|| panic!("methods: {:?}", m.methods.keys().collect::<Vec<_>>()));
        assert_eq!(bark.kind, MethodKind::Method);
        assert!(!bark.body.statements.is_empty());
        for implicit in ["self", "selfns", "type", "options", "name"] {
            assert!(
                bark.instance_vars.contains(implicit),
                "missing {implicit}: {:?}",
                bark.instance_vars
            );
        }
        assert_eq!(
            m.methods["::Dog::count"].kind,
            MethodKind::ClassMethod,
            "typemethod dispatches on the type command"
        );
        assert!(m.methods.contains_key("::Dog::<constructor>"));
    }

    // TP: the widget grammar's extra implicit vars (win / hull) reach
    // widget method units — registry data, not a name-suffix check.
    #[test]
    fn snit_widget_bodies_see_win_and_hull() {
        let src = "snit::widget MyBar {\n\
                   \x20   method redraw {} { return $win }\n\
                   }\n";
        let m = lower_to_ir(src, &reg());
        let redraw = m
            .methods
            .get("::MyBar::redraw")
            .unwrap_or_else(|| panic!("methods: {:?}", m.methods.keys().collect::<Vec<_>>()));
        assert!(redraw.instance_vars.contains("win"));
        assert!(redraw.instance_vars.contains("hull"));
    }

    // TP (item 2): itcl method bodies — including the access-modifier
    // wrapped spellings — become method units with `this`, instance
    // variables, and commons in scope.
    #[test]
    fn itcl_class_bodies_produce_method_units() {
        let src = "itcl::class Toaster {\n\
                   \x20   variable crumbs 0\n\
                   \x20   common heat 3\n\
                   \x20   public method toast {n} { incr crumbs $n }\n\
                   \x20   method clean {} { set crumbs 0 }\n\
                   \x20   constructor {} { set crumbs 0 }\n\
                   }\n";
        let m = lower_to_ir(src, &reg());
        let toast = m
            .methods
            .get("::Toaster::toast")
            .unwrap_or_else(|| panic!("methods: {:?}", m.methods.keys().collect::<Vec<_>>()));
        assert_eq!(toast.params, vec!["n".to_string()]);
        for name in ["this", "crumbs", "heat"] {
            assert!(
                toast.instance_vars.contains(name),
                "missing {name}: {:?}",
                toast.instance_vars
            );
        }
        assert!(m.methods.contains_key("::Toaster::clean"));
        assert!(m.methods.contains_key("::Toaster::<constructor>"));
    }

    // TP: TclOO wrapper members — `self { method … }` lifts a
    // classmethod-kind unit, `private { method … }` an instance one; the
    // prefix spellings match.
    #[test]
    fn tcloo_wrapper_members_produce_units() {
        let src = "oo::class create W {\n\
                   \x20   self { method make {} { return [my new] } }\n\
                   \x20   private { method hidden {} { return 1 } }\n\
                   \x20   self method direct {} { return 2 }\n\
                   }\n";
        let m = lower_to_ir(src, &reg());
        assert_eq!(
            m.methods.get("::W::make").map_or_else(
                || panic!("methods: {:?}", m.methods.keys().collect::<Vec<_>>()),
                |d| d.kind
            ),
            MethodKind::ClassMethod
        );
        assert_eq!(m.methods["::W::direct"].kind, MethodKind::ClassMethod);
        assert_eq!(m.methods["::W::hidden"].kind, MethodKind::Method);
    }

    // TN: an `initialise` body is a *definition script* evaluated in the
    // class object's namespace, not a method frame — no unit.
    #[test]
    fn tcloo_initialise_body_is_not_a_method_unit() {
        let src = "oo::class create I {\n\
                   \x20   initialise { variable cache {} }\n\
                   \x20   method read {} { return 1 }\n\
                   }\n";
        let m = lower_to_ir(src, &reg());
        assert!(
            !m.methods.keys().any(|k| k.contains("initialise")),
            "methods: {:?}",
            m.methods.keys().collect::<Vec<_>>()
        );
        assert!(m.methods.contains_key("::I::read"));
    }

    // TN: a dynamic objdefine receiver that is not a simple variable
    // reference (a command substitution) stays un-extracted.
    #[test]
    fn objdefine_command_substitution_receiver_abstains() {
        let src = "oo::class create C {}\n\
                   oo::objdefine [C new] { method m {} { return 1 } }\n";
        let m = lower_to_ir(src, &reg());
        assert!(
            !m.methods.keys().any(|k| k.starts_with("::@objdefine@")),
            "methods: {:?}",
            m.methods.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn redefined_oo_method_retains_the_replacement_body() {
        // A method redefined by a later `oo::define` keeps the first
        // body in `methods`, and RETAINS the replacement in
        // `redefined_methods` (issue #1166) so analysis consumers can
        // scan every body a dispatch may run instead of abstaining.
        let src = "oo::class create C {\n\
                   \x20   method m {} { return 1 }\n\
                   }\n\
                   oo::define C {\n\
                   \x20   method m {} { return 2 }\n\
                   }\n";
        let m = lower_to_ir(src, &reg());
        assert!(m.methods.contains_key("::C::m"));
        let retained = m
            .redefined_methods
            .get("::C::m")
            .unwrap_or_else(|| panic!("redefined: {:?}", m.redefined_methods));
        assert_eq!(retained.len(), 1, "one replacement body retained");
        assert_eq!(retained[0].class_name, "::C");
        assert!(
            !retained[0].body.statements.is_empty(),
            "the replacement body is lowered, not discarded"
        );
        assert!(m.oo_unanalysed_classes.is_empty());
        assert!(!m.oo_evidence.dynamic_target);
    }

    #[test]
    fn unreadable_oo_members_flag_the_class_or_module() {
        // A dynamic member NAME may redefine any method of the class —
        // the class is flagged unanalysable.
        let m = lower_to_ir(
            "oo::class create C {\n    method m {} { return 1 }\n    method $n {} { return 2 }\n}\n",
            &reg(),
        );
        assert!(m.oo_unanalysed_classes.contains("::C"), "{m:?}");
        // A dynamic member BODY is a method whose code no scan can read.
        let m = lower_to_ir("oo::class create C {\n    method m {} $body\n}\n", &reg());
        assert!(m.oo_unanalysed_classes.contains("::C"));
        // A dynamic CLASS word may touch any class — module-wide flag.
        let m = lower_to_ir("oo::define $cls { method m {} { return 1 } }\n", &reg());
        assert!(m.oo_evidence.dynamic_target);
        // A fully-static module sets neither.
        let m = lower_to_ir("oo::class create C { method m {} { return 1 } }\n", &reg());
        assert!(m.oo_unanalysed_classes.is_empty());
        assert!(!m.oo_evidence.dynamic_target);
    }

    #[test]
    fn oo_method_nested_proc_not_registered_globally() {
        // A `proc` defined inside a method body is created at
        // method-call time, not script load — it must not leak into
        // `module.procedures` (codegen safety).
        let src = "oo::class create C {\n\
                   \x20   method m {} { proc helper {} { return 1 }; return 2 }\n\
                   }\n\
                   proc toplevel {} { return 3 }\n";
        let m = lower_to_ir(src, &reg());
        assert!(
            m.procedures.contains_key("::toplevel"),
            "procs: {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
        assert!(
            !m.procedures.contains_key("::helper"),
            "nested method proc must not be registered: {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
        // The method body still lowered the `proc helper ...` call.
        assert!(m.methods.contains_key("::C::m"));
    }

    #[test]
    fn apply_body_proc_registers_in_lambda_namespace_not_callers() {
        // `apply`'s body runs in the namespace named by the lambda — or the
        // *global* namespace when none is given — NOT the caller's namespace.
        // So a nested `proc` registers as `::helper`, never `::foo::helper`
        // (Tcl `apply` manual).
        let src = "namespace eval foo {\n\
                   \x20   apply {{} { proc helper {} { return 1 } }}\n\
                   }\n";
        let m = lower_to_ir(src, &reg());
        assert!(
            m.procedures.contains_key("::helper"),
            "apply body proc must register in the global namespace: {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
        assert!(
            !m.procedures.contains_key("::foo::helper"),
            "apply body proc must NOT inherit the caller namespace: {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn apply_body_proc_uses_explicit_lambda_namespace() {
        // A lambda whose element 2 pins a namespace runs its body there.
        let src = "apply {{} { proc helper {} { return 1 } } ::bar}\n";
        let m = lower_to_ir(src, &reg());
        assert!(
            m.procedures.contains_key("::bar::helper"),
            "explicit lambda namespace must be honoured: {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
    }

    /// TP — a relative lambda namespace element homes against the GLOBAL
    /// namespace even from inside a qualified-name proc (`doc/apply.n`:
    /// "interpreted relative to the global namespace even if its name does
    /// not start with ::"; tclsh 9.0.4 probe `a2b_apply.tcl`). Mirrors the
    /// analyser's `handle_apply_command` so IR and analysis agree.
    #[test]
    fn apply_relative_lambda_namespace_homes_globally_not_to_the_caller() {
        let src = "proc ::caller::p {} { apply {{} { proc helper {} { return 1 } } sub} }\n";
        let m = lower_to_ir(src, &reg());
        assert!(
            m.procedures.contains_key("::sub::helper"),
            "relative lambda ns must resolve against ::: {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
        assert!(
            !m.procedures.contains_key("::caller::sub::helper"),
            "relative lambda ns must NOT pin against the caller: {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn no_oo_methods_for_plain_script() {
        let m = lower_to_ir("proc greet {} { puts hi }", &reg());
        assert!(m.methods.is_empty());
        assert!(m.redefined_methods.is_empty());
    }

    #[test]
    fn proc_definition() {
        let m = lower_to_ir("proc greet {name} {puts $name}", &reg());
        // proc emits a Statement::Call + registers a procedure.
        assert!(m.procedures.contains_key("::greet"));
        let p = &m.procedures["::greet"];
        assert_eq!(p.params, vec!["name"]);
    }

    #[test]
    fn if_statement() {
        let m = lower_to_ir("if {1} {set x 1}", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(&m.top_level.statements[0], Statement::If { .. }));
    }

    #[test]
    fn for_loop() {
        let m = lower_to_ir("for {set i 0} {$i < 10} {incr i} {puts $i}", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(&m.top_level.statements[0], Statement::For { .. }));
    }

    #[test]
    fn while_loop() {
        let m = lower_to_ir("while {1} {puts loop}", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::While { .. }
        ));
    }

    #[test]
    fn foreach_loop() {
        let m = lower_to_ir("foreach x {a b c} {puts $x}", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Foreach { .. }
        ));
    }

    #[test]
    fn catch_statement() {
        let m = lower_to_ir("catch {error oops} result", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Catch { .. }
        ));
    }

    #[test]
    fn generic_command() {
        let m = lower_to_ir("puts hello", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Call { command, .. } if command == "puts"
        ));
    }

    #[test]
    fn multiple_commands() {
        let m = lower_to_ir("set x 1\nset y 2\nputs $x", &reg());
        assert_eq!(m.top_level.statements.len(), 3);
    }

    #[test]
    fn return_statement() {
        let m = lower_to_ir("return 42", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Return { value: Some(v), .. } if v == "42"
        ));
    }

    #[test]
    fn lower_to_ir_public_api() {
        let r = reg();
        let m = lower_to_ir("set x 1\nproc foo {} {return 1}", &r);
        assert_eq!(m.top_level.statements.len(), 2);
        assert!(m.procedures.contains_key("::foo"));
    }

    // ---- issue #1077: a proc body homes nested definitions to the proc's
    // *defining* namespace, not the lexical namespace of the `proc` call. ----

    #[test]
    fn proc_body_namespace_peels_the_names_own_qualifier() {
        // Unit-level: the helper is the inverse of `qualify_proc_name`.
        assert_eq!(proc_body_namespace("::a::outer", "::"), "::a");
        assert_eq!(proc_body_namespace("::b::c::o2", "::"), "::b::c");
        assert_eq!(proc_body_namespace("::plain", "::ignored"), "::");
        // A bare (unrooted) key can't come out of `qualify_proc_name`, but the
        // fallback must not manufacture an empty namespace if it ever did.
        assert_eq!(proc_body_namespace("plain", "::lex"), "::lex");
    }

    #[test]
    fn nested_proc_homes_to_the_outer_procs_defining_namespace() {
        // TP. Oracle (tclsh 9.0.4 and 8.6.16, identical):
        //   namespace eval ::a {}
        //   proc a::outer {} { proc helper {x} {return $x} }
        //   a::outer ; info commands ::a::helper   -> ::a::helper
        //              info commands ::helper      -> {}
        let m = lower_to_ir(
            "namespace eval ::a {}\nproc a::outer {} { proc helper {x} { return $x } }\n",
            &reg(),
        );
        assert!(
            m.procedures.contains_key("::a::helper"),
            "nested proc must home to ::a, got {:?}",
            m.procedures.keys().collect::<Vec<_>>(),
        );
        assert!(
            !m.procedures.contains_key("::helper"),
            "nested proc must NOT home lexically to ::",
        );
    }

    #[test]
    fn nested_proc_homing_composes_through_three_levels() {
        // TP. Oracle: `proc g::o7 {} { proc o7b {} { proc h7 {u} … } }` →
        // running `g::o7` then `::g::o7b` leaves `::g::h7` (never `::h7`).
        let m = lower_to_ir(
            "namespace eval ::g {}\nproc g::o7 {} { proc o7b {} { proc h7 {u} { return $u } } }\n",
            &reg(),
        );
        assert!(m.procedures.contains_key("::g::o7b"));
        assert!(
            m.procedures.contains_key("::g::h7"),
            "third level must stay in ::g, got {:?}",
            m.procedures.keys().collect::<Vec<_>>(),
        );
        assert!(!m.procedures.contains_key("::h7"));
    }

    #[test]
    fn nested_proc_name_qualifier_beats_the_lexical_namespace_eval() {
        // TP. Oracle: inside `namespace eval ::d`, `proc ::e::o3` still homes
        // its body to `::e` — `::e::h3` exists, `::d::h3` and `::h3` do not.
        let m = lower_to_ir(
            "namespace eval ::e {}\nnamespace eval ::d {\n    proc ::e::o3 {} { proc h3 {z} { return $z } }\n}\n",
            &reg(),
        );
        assert!(
            m.procedures.contains_key("::e::h3"),
            "got {:?}",
            m.procedures.keys().collect::<Vec<_>>(),
        );
        assert!(!m.procedures.contains_key("::d::h3"));
        assert!(!m.procedures.contains_key("::h3"));
    }

    #[test]
    fn unqualified_outer_proc_leaves_nested_homing_unchanged() {
        // TN. The common shape: lexical and defining namespace agree, so the
        // fix must be a no-op. Both at top level…
        let m = lower_to_ir("proc outer {} { proc inner {} { return 1 } }\n", &reg());
        assert!(m.procedures.contains_key("::inner"));
        // …and inside a `namespace eval`, where both are `::ns`.
        let m = lower_to_ir(
            "namespace eval ::ns {\n    proc outer {} { proc inner {} { return 1 } }\n}\n",
            &reg(),
        );
        assert!(
            m.procedures.contains_key("::ns::inner"),
            "got {:?}",
            m.procedures.keys().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn absolutely_named_nested_proc_is_unaffected() {
        // TN. Oracle: `proc a::outer4 {} { proc ::abs {q} … }` creates `::abs`
        // — an absolute inner name ignores the enclosing frame entirely.
        let m = lower_to_ir(
            "namespace eval ::a {}\nproc a::outer4 {} { proc ::abs {q} { return $q } }\n",
            &reg(),
        );
        assert!(m.procedures.contains_key("::abs"));
        assert!(!m.procedures.contains_key("::a::abs"));
    }

    #[test]
    fn formal_param_names_take_the_specifier_name_only() {
        assert_eq!(
            parse_formal_param_names("a b c").unwrap(),
            vec!["a", "b", "c"],
        );
        assert_eq!(
            parse_formal_param_names("{x default} y").unwrap(),
            vec!["x", "y"]
        );
        assert!(parse_formal_param_names("").unwrap().is_empty());
        // A wrapped parameter list collapses its continuation before splitting.
        assert_eq!(
            parse_formal_param_names("a b\\\n    c").unwrap(),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn formal_param_names_reject_what_tcl_rejects() {
        // Tcl refuses to create these procedures, so lowering declines too.
        assert_eq!(parse_formal_param_names("{a b c}"), None);
        assert_eq!(parse_formal_param_names("{a::b}"), None);
        assert_eq!(parse_formal_param_names("{a(1)}"), None);
        assert_eq!(parse_formal_param_names("{a"), None);
    }

    #[test]
    fn var_list_names_keep_grouped_elements_whole() {
        assert_eq!(parse_var_list_names("a b").unwrap(), vec!["a", "b"]);
        // tclsh 9.0: both spellings bind the one variable named `a b`.
        assert_eq!(parse_var_list_names("{a b}").unwrap(), vec!["a b"]);
        assert_eq!(parse_var_list_names("a\\ b").unwrap(), vec!["a b"]);
        assert!(parse_var_list_names("").unwrap().is_empty());
        assert_eq!(
            parse_var_list_names("a b\\\n    c").unwrap(),
            vec!["a", "b", "c"]
        );
        // Unbalanced — no binding can be named, so the caller barriers.
        assert_eq!(parse_var_list_names("{a"), None);
    }

    // issue #1431: a formal-parameter list has two list levels, so a grouped or
    // escaped specifier is one parameter, not one name per whitespace run. The
    // hand-rolled splitter gave `proc p {a\ b} {}` an arity of two.

    /// Parameter names recorded for `::p` when `src` is lowered.
    fn proc_params(src: &str) -> Vec<String> {
        let m = lower_to_ir(src, &reg());
        m.procedures
            .get("::p")
            .unwrap_or_else(|| panic!("::p registered for {src:?}"))
            .params
            .clone()
    }

    #[test]
    fn escaped_space_proc_param_is_one_parameter() {
        // tclsh 9.0: `proc p {a\ b} {}` → `info args p` is `a`, with
        // `info default p a` yielding `b`. Only the name reaches the IR, so the
        // arity is one — the splitter used to report two (`a\` and `b`).
        assert_eq!(proc_params("proc p {a\\ b} {}"), vec!["a"]);
        // The grouped spelling decodes identically.
        assert_eq!(proc_params("proc p {{a b}} {}"), vec!["a"]);
    }

    #[test]
    fn proc_params_keep_defaults_and_args_as_names() {
        // FP-guard: `{x {y 2} args}` is still the three parameters `x`, `y`,
        // and the variadic `args`.
        assert_eq!(
            proc_params("proc p {x {y 2} args} {}"),
            vec!["x", "y", "args"]
        );
        assert!(proc_params("proc p {} {}").is_empty());
    }

    #[test]
    fn proc_params_tcl_rejects_fall_through_to_barrier() {
        // tclsh: `proc p {{a b c}} {}` → "too many fields in argument
        // specifier". No procedure is created, so lowering registers none and
        // leaves the runtime `proc` to report it.
        for src in [
            "proc p {{a b c}} {}",
            "proc p {a::b} {}",
            "proc p {a(1)} {}",
        ] {
            let m = lower_to_ir(src, &reg());
            assert!(
                !m.procedures.contains_key("::p"),
                "no ::p for {src:?}: {:?}",
                m.procedures.keys().collect::<Vec<_>>(),
            );
            assert!(
                matches!(&m.top_level.statements[0], Statement::Barrier { reason, .. } if reason == "malformed proc params"),
                "expected a malformed-params barrier for {src:?}, got {:?}",
                m.top_level.statements[0],
            );
        }
    }

    #[test]
    fn oo_method_params_use_the_formal_parameter_grammar() {
        let m = lower_to_ir(
            "oo::class create C {\n    method m {a\\ b {c 1}} { return 1 }\n}\n",
            &reg(),
        );
        let method = m.methods.get("::C::m").unwrap_or_else(|| {
            panic!(
                "::C::m lowered, got {:?}",
                m.methods.keys().collect::<Vec<_>>()
            )
        });
        assert_eq!(method.params, vec!["a", "c"]);
        // A parameter list Tcl rejects makes the whole class unanalysable.
        let m = lower_to_ir(
            "oo::class create C {\n    method m {{a b c}} { return 1 }\n}\n",
            &reg(),
        );
        assert!(m.oo_unanalysed_classes.contains("::C"), "{m:?}");
        assert!(!m.methods.contains_key("::C::m"));
    }

    #[test]
    fn qualify_proc_name_global() {
        assert_eq!(qualify_proc_name("::", "foo"), "::foo");
    }

    #[test]
    fn qualify_proc_name_nested() {
        assert_eq!(qualify_proc_name("::ns", "bar"), "::ns::bar");
    }

    #[test]
    fn qualify_proc_name_already_qualified() {
        assert_eq!(qualify_proc_name("::ns", "::abs"), "::abs");
    }

    #[test]
    fn qualify_proc_name_preserves_colon_named_namespace_keys() {
        assert_eq!(qualify_proc_name(":::", ":"), "::::::");
        assert_eq!(qualify_proc_name("::a", "a:::b"), "::a::a::b");
        assert_eq!(proc_namespace("::::::"), ":::");
    }

    #[test]
    fn parse_uplevel_level_decimal() {
        let relative = |shift| {
            Some(UplevelLevel {
                shift,
                absolute: false,
            })
        };
        assert_eq!(parse_uplevel_level("1"), relative(1));
        assert_eq!(parse_uplevel_level("3"), relative(3));
        assert_eq!(parse_uplevel_level("0"), relative(0));
    }

    #[test]
    fn parse_uplevel_level_hash_form() {
        let absolute = |shift| {
            Some(UplevelLevel {
                shift,
                absolute: true,
            })
        };
        assert_eq!(parse_uplevel_level("#0"), absolute(0));
        assert_eq!(parse_uplevel_level("#3"), absolute(3));
    }

    #[test]
    fn parse_uplevel_level_zero_keeps_absolute_and_relative_apart() {
        // `uplevel #0` runs the body in the global frame; `uplevel 0` runs it
        // in the current frame. Both carry the magnitude 0, so only the
        // `absolute` flag tells them apart — conflating them miscompiled the
        // command-word namespace of every `uplevel 0` body.
        let hash_zero = parse_uplevel_level("#0").expect("#0 parses");
        let bare_zero = parse_uplevel_level("0").expect("0 parses");
        assert_eq!(hash_zero.shift, bare_zero.shift);
        assert!(hash_zero.absolute);
        assert!(!bare_zero.absolute);
        assert_ne!(hash_zero, bare_zero);
    }

    #[test]
    fn parse_uplevel_level_dynamic_returns_none() {
        assert_eq!(parse_uplevel_level("$lvl"), None);
        assert_eq!(parse_uplevel_level("[expr {1+1}]"), None);
        assert_eq!(parse_uplevel_level("foo"), None);
    }

    #[test]
    fn uplevel_static_body_no_level() {
        let m = lower_to_ir("uplevel {set x 1}", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        match &m.top_level.statements[0] {
            Statement::UpFrame {
                frame_shift, body, ..
            } => {
                assert_eq!(*frame_shift, 1);
                assert_eq!(body.statements.len(), 1);
            }
            other => panic!("expected UpFrame, got {other:?}"),
        }
    }

    #[test]
    fn uplevel_static_body_with_level_one() {
        let m = lower_to_ir("uplevel 1 {set x 1}", &reg());
        match &m.top_level.statements[0] {
            Statement::UpFrame {
                frame_shift,
                absolute,
                ..
            } => {
                assert_eq!(*frame_shift, 1);
                assert!(!*absolute, "`uplevel 1` is the relative form");
            }
            other => panic!("expected UpFrame, got {other:?}"),
        }
    }

    #[test]
    fn uplevel_static_body_with_hash_zero() {
        let m = lower_to_ir("uplevel #0 {set x 1}", &reg());
        match &m.top_level.statements[0] {
            Statement::UpFrame {
                frame_shift,
                absolute,
                ..
            } => {
                assert_eq!(*frame_shift, 0);
                assert!(*absolute, "`uplevel #0` is the absolute global form");
            }
            other => panic!("expected UpFrame for #0, got {other:?}"),
        }
    }

    #[test]
    fn uplevel_static_body_with_bare_zero_is_relative() {
        // `uplevel 0 {body}` evaluates the body in the *current* frame, not
        // the global one. It shares the magnitude 0 with `#0` and is told
        // apart only by `absolute`.
        let m = lower_to_ir("uplevel 0 {set x 1}", &reg());
        match &m.top_level.statements[0] {
            Statement::UpFrame {
                frame_shift,
                absolute,
                ..
            } => {
                assert_eq!(*frame_shift, 0);
                assert!(!*absolute, "`uplevel 0` is the relative current-frame form");
            }
            other => panic!("expected UpFrame for bare 0, got {other:?}"),
        }
    }

    #[test]
    fn uplevel_dynamic_body_falls_back_to_default() {
        // ``uplevel 1 $body`` body is a $var, not a brace literal —
        // can't be statically resolved without const-propagation.
        // Falls back to ``lower_default`` (Statement::Call).
        let m = lower_to_ir("uplevel 1 $body", &reg());
        assert!(matches!(
            m.top_level.statements[0],
            Statement::Call { .. } | Statement::Barrier { .. }
        ));
    }

    #[test]
    fn uplevel_dynamic_level_falls_back_to_default() {
        // ``uplevel $lvl {body}`` — level is dynamic, can't pick a
        // ``frame_shift``. Falls back to default lowering.
        let m = lower_to_ir("uplevel $lvl {set x 1}", &reg());
        assert!(matches!(
            m.top_level.statements[0],
            Statement::Call { .. } | Statement::Barrier { .. }
        ));
    }

    #[test]
    fn const_prop_uplevel_resolves_set_var_body() {
        // ``set body {set x 1}; uplevel 1 $body`` inside a
        // proc — the const-map records ``body`` and the uplevel
        // folds in the literal as an UpFrame.
        let m = lower_to_ir("proc f {} { set body {set x 1}\n uplevel 1 $body }", &reg());
        let proc = m.procedures.get("::f").expect("proc registered");
        // proc body: [AssignConst(body), UpFrame { body: [...] }]
        let last = proc.body.statements.last().expect("body has statements");
        match last {
            Statement::UpFrame { body, .. } => {
                assert!(!body.statements.is_empty(), "expected lowered body");
            }
            other => panic!("expected UpFrame, got {other:?}"),
        }
    }

    #[test]
    fn const_prop_eval_resolves_set_var_body() {
        // ``eval $body`` with const-mapped body folds to a
        // Statement::Block.
        let m = lower_to_ir("proc f {} { set body {set x 1}\n eval $body }", &reg());
        let proc = m.procedures.get("::f").expect("proc registered");
        let last = proc.body.statements.last().expect("body has statements");
        match last {
            Statement::Block { body, .. } => {
                assert!(!body.statements.is_empty(), "expected lowered block");
            }
            other => panic!("expected Block, got {other:?}"),
        }
    }

    #[test]
    fn const_prop_eval_brace_body_emits_block() {
        // ``eval {body}`` in a proc context lowers to Block.
        let m = lower_to_ir("proc f {} { eval {set x 1} }", &reg());
        let proc = m.procedures.get("::f").expect("proc registered");
        assert!(matches!(proc.body.statements[0], Statement::Block { .. }));
    }

    #[test]
    fn const_prop_disabled_at_top_level() {
        // The const-map is gated on ``proc_depth > 0``. At
        // top-level, ``set body {set x 1}; uplevel 1 $body``
        // does NOT relax — the uplevel falls back to default
        // dispatch (Call / Barrier).
        let m = lower_to_ir("set body {set x 1}\nuplevel 1 $body", &reg());
        // statements[0] = AssignConst(body), [1] = uplevel call
        assert!(matches!(
            m.top_level.statements[1],
            Statement::Call { .. } | Statement::Barrier { .. }
        ));
    }

    #[test]
    fn const_prop_invalidated_on_reassignment() {
        // ``set body {a}; set body $other; uplevel 1 $body`` — the
        // second ``set`` invalidates the binding (RHS is a $var,
        // not a brace literal), so the uplevel can't fold.
        let m = lower_to_ir(
            "proc f {} { set body {a}\n set body $other\n uplevel 1 $body }",
            &reg(),
        );
        let proc = m.procedures.get("::f").expect("proc registered");
        let last = proc.body.statements.last().expect("body");
        assert!(
            !matches!(last, Statement::UpFrame { .. }),
            "expected fallback after re-assignment, got {last:?}",
        );
    }

    #[test]
    fn const_prop_eval_list_literal() {
        // ``eval [list set x 42]`` recognised as static body.
        let m = lower_to_ir("proc f {} { eval [list set x 42] }", &reg());
        let proc = m.procedures.get("::f").expect("proc registered");
        assert!(matches!(proc.body.statements[0], Statement::Block { .. }));
    }

    #[test]
    fn const_prop_eval_list_with_dynamic_arg_rejected() {
        // ``eval [list set x $v]`` — dynamic ``\$v`` rejects the
        // list-literal shape. Falls back to runtime barrier.
        let m = lower_to_ir("proc f {} { eval [list set x $v] }", &reg());
        let proc = m.procedures.get("::f").expect("proc registered");
        assert!(!matches!(proc.body.statements[0], Statement::Block { .. }));
    }

    #[test]
    fn const_prop_eval_non_list_command_rejected() {
        // ``eval [foo arg]`` — inner command isn't ``list``;
        // can't synthesise a body. Falls back to runtime barrier.
        let m = lower_to_ir("proc f {} { eval [foo arg] }", &reg());
        let proc = m.procedures.get("::f").expect("proc registered");
        assert!(!matches!(proc.body.statements[0], Statement::Block { .. }));
    }

    #[test]
    fn const_prop_does_not_leak_into_nested_proc() {
        // A ``set body {literal}`` in the outer proc must
        // NOT appear to a nested ``proc inner``'s
        // barrier-relaxation gate as a tracked literal.
        let m = lower_to_ir(
            "proc outer {} {\n  set body {set x 1}\n  proc inner {} { uplevel 1 $body }\n}",
            &reg(),
        );
        let inner = m.procedures.get("::inner").expect("inner registered");
        // The inner uplevel must remain a Call/Barrier — NOT an
        // UpFrame. If the const-map leaked, the inner body would
        // be folded as UpFrame { body: [set x 1], .. }.
        let last = inner.body.statements.last().expect("body");
        assert!(
            !matches!(last, Statement::UpFrame { .. }),
            "outer scope's const-map must not leak into nested proc, got {last:?}",
        );
    }

    #[test]
    fn ns_import_recorded_with_context_namespace() {
        // ``namespace import ::tcltest::*`` at top-level
        // records (``::``, ``::tcltest::*``).
        let m = lower_to_ir("namespace import ::tcltest::*", &reg());
        assert_eq!(
            m.namespace_imports,
            vec![("::".to_string(), "::tcltest::*".to_string())]
        );
    }

    #[test]
    fn ns_import_skips_relative_pattern() {
        // Relative patterns (``foo::*`` without leading ``::``)
        // require runtime namespace-path walking; we skip them.
        let m = lower_to_ir("namespace import foo::*", &reg());
        assert!(m.namespace_imports.is_empty());
    }

    #[test]
    fn ns_import_handles_force_flag() {
        // ``-force`` is the documented option; the next word is
        // the pattern.
        let m = lower_to_ir("namespace import -force ::tcltest::*", &reg());
        assert_eq!(
            m.namespace_imports,
            vec![("::".to_string(), "::tcltest::*".to_string())]
        );
    }

    #[test]
    fn proc_dynamic_name_resolved_via_const_map() {
        // ``set name {Verbose}; proc \$name {} { ... }``
        // inside a proc — ``name`` is in the const-map, so the
        // inner proc registers as ``::Verbose``.
        let m = lower_to_ir(
            "proc factory {} { set name {Verbose}\n proc $name {} { puts hi } }",
            &reg(),
        );
        assert!(
            m.procedures.contains_key("::Verbose"),
            "expected ::Verbose in {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn proc_dynamic_name_no_binding_stays_barrier() {
        // ``proc \$name`` with no const-map entry — bail to barrier.
        let m = lower_to_ir("proc factory {} { proc $name {} { puts hi } }", &reg());
        let factory = m.procedures.get("::factory").expect("factory registered");
        let last = factory.body.statements.last().expect("body");
        assert!(
            matches!(last, Statement::Barrier { .. }),
            "expected Barrier, got {last:?}"
        );
    }

    #[test]
    fn proc_with_trailing_colons_routes_through_barrier() {
        // Tcl 9 ``proc ::ns:: {args} {body}`` registers a command
        // named ``""`` inside ``::ns``.  Static normalisation would
        // strip the trailing ``::`` and register the proc under the
        // namespace name itself — wrong cmd.  Route through
        // Statement::Barrier so the runtime ``proc`` builtin handles
        // the registration.
        let m = lower_to_ir("proc ::ns:: {a b} { puts $a$b }", &reg());
        // No procedure entry should have been AOT-registered under
        // ``::ns`` (which is what the bug would produce) or any
        // empty-key variant.
        assert!(
            !m.procedures.contains_key("::ns")
                && !m.procedures.contains_key("::ns::")
                && !m.procedures.contains_key(""),
            "trailing-:: proc should not be AOT-registered; got {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
        // The top-level statement should be a Barrier with the
        // empty-simple-name reason.
        let top = m
            .top_level
            .statements
            .last()
            .expect("at least one top-level statement");
        match top {
            Statement::Barrier {
                reason, command, ..
            } => {
                assert_eq!(command, "proc");
                assert!(
                    reason.contains("empty-simple-name"),
                    "unexpected barrier reason: {reason}"
                );
            }
            other => panic!("expected Statement::Barrier, got {other:?}"),
        }
    }

    #[test]
    fn proc_with_command_substitution_in_name_stays_barrier() {
        // ``proc \$name[suffix]`` — multi-token name with a command
        // substitution. The const-map gate only covers single-VAR
        // tokens; this stays on the runtime path.
        let m = lower_to_ir(
            "proc factory {} { set name {x}\n proc $name[suffix] {} { puts hi } }",
            &reg(),
        );
        // ``::factory`` registered; the inner proc must NOT be
        // registered under any literal name (``::x...``).
        assert!(!m.procedures.keys().any(|k| k.contains("suffix")));
    }

    #[test]
    fn proc_subst_nocommands_body_materialised() {
        // ``proc \$name {x} [subst -nocommands {return \$default}]``
        // with both ``name`` and ``default`` const-tracked materialises
        // the body to ``return 0`` and lowers it as a real script.
        let m = lower_to_ir(
            "proc factory {} { set name {Verbose}\n set default {0}\n proc $name {x} [subst -nocommands {return $default}] }",
            &reg(),
        );
        let inner = m.procedures.get("::Verbose").expect("::Verbose registered");
        // Body should contain a Return statement (or at least be
        // non-empty — the materialised body lowers to a real
        // statement, not a Barrier).
        assert!(
            !inner.body.statements.is_empty(),
            "expected lowered body, got empty"
        );
    }

    #[test]
    fn foreach_line_lowers_to_foreach_statement() {
        // `foreachLine var filename body` lowers
        // to a single-iterator Statement::Foreach so variables assigned
        // inside the body propagate to the enclosing scope.  The
        // generic stdlib-proc path would route through Statement::Barrier and
        // lose that lattice information.
        let m = lower_to_ir(
            "foreachLine line /etc/hosts { set count [expr {$count + 1}] }",
            &reg(),
        );
        let stmts = &m.top_level.statements;
        let last = stmts.last().expect("at least one statement");
        match last {
            Statement::Foreach {
                iterators, is_lmap, ..
            } => {
                assert!(!is_lmap);
                assert_eq!(iterators.len(), 1);
                assert_eq!(iterators[0].vars, vec!["line".to_owned()]);
            }
            other => panic!("expected Statement::Foreach, got {other:?}"),
        }
    }

    #[test]
    fn foreach_line_var_body_routes_through_barrier() {
        // `foreachLine line file $body` — the body is a Var token,
        // which is single-token but NOT a braced literal.  Compiling
        // the literal `$body` text as a static loop body would
        // produce incorrect IR / data-flow; the lowering must fall
        // through to the runtime command via Barrier.
        let m = lower_to_ir("foreachLine line /etc/hosts $body", &reg());
        let last = m.top_level.statements.last().expect("top-level statement");
        match last {
            Statement::Barrier {
                reason, command, ..
            } => {
                assert_eq!(command, "foreachLine");
                assert!(
                    reason.contains("dynamic body"),
                    "unexpected reason: {reason}"
                );
            }
            other => panic!("expected Barrier for Var body, got {other:?}"),
        }
    }

    #[test]
    fn foreach_line_cmd_body_routes_through_barrier() {
        // Same guard for a `[cmd]` body — single-token but a Cmd
        // substitution, not a braced literal.
        let m = lower_to_ir("foreachLine line /etc/hosts [build-body]", &reg());
        let last = m.top_level.statements.last().expect("top-level statement");
        assert!(
            matches!(last, Statement::Barrier { .. }),
            "expected Barrier for Cmd body, got {last:?}"
        );
    }

    #[test]
    fn proc_var_body_const_map_materialised() {
        // A bare `$body` body word whose value
        // is in the const-map materialises into the Procedure body
        // (matching the `proc $name {x} $body` shape).
        let m = lower_to_ir(
            "proc factory {} { set name {Greeter}\n set body {return hi}\n proc $name {} $body }",
            &reg(),
        );
        let inner = m.procedures.get("::Greeter").expect("::Greeter registered");
        // The body was const-resolved to "return hi"; lowering it
        // produces a Return statement.
        assert!(
            !inner.body.statements.is_empty(),
            "expected lowered body, got empty"
        );
    }

    #[test]
    fn proc_var_body_unresolved_keeps_empty_body() {
        // `$body` has no const-map binding but `$name` does — the
        // proc name resolves, but the body stays empty (the runtime
        // proc call below carries the dynamic body text).
        let m = lower_to_ir(
            "proc factory {} { set name {Greeter}\n proc $name {} $body }",
            &reg(),
        );
        let inner = m.procedures.get("::Greeter").expect("::Greeter registered");
        assert!(
            inner.body.statements.is_empty(),
            "expected empty body for unresolved dynamic body, got {:?}",
            inner.body.statements
        );
    }

    #[test]
    fn proc_dynamic_params_routes_through_barrier() {
        // A dynamic params word — there's no static arg-list to
        // build a real Procedure from, so the whole proc shape
        // bails to a Barrier.  Even when the name resolves via the
        // const-map.
        let m = lower_to_ir(
            "proc factory {} { set name {x}\n proc $name $params { puts hi } }",
            &reg(),
        );
        // No inner procedure should be registered — params couldn't
        // be parsed statically.
        assert!(
            !m.procedures.contains_key("::x"),
            "dynamic-params shape must not AOT-register: {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn proc_multi_token_body_word_is_dynamic() {
        // `proc foo {} "pre${body}post"` — the body is a quoted
        // multi-token word.  Even though the first token is Str,
        // single_token_word is false so the whole word is dynamic
        // and we route through Barrier (the multi-token guard).
        let m = lower_to_ir(
            r#"proc factory {} { set body {return hi}; proc inner {} "pre${body}post" }"#,
            &reg(),
        );
        // `::inner` would only be registered if the body was
        // materialised — which can't happen for multi-token words.
        // We allow either no entry or an empty-body entry; the key
        // assertion is that the literal source text isn't compiled
        // as a script.
        if let Some(inner) = m.procedures.get("::inner") {
            assert!(
                inner.body.statements.is_empty(),
                "multi-token body must not be statically lowered, got {:?}",
                inner.body.statements
            );
        }
    }

    #[test]
    fn proc_subst_nocommands_missing_var_skips_materialisation() {
        // ``\$default`` is not in the const-map — the materialiser
        // refuses, leaving the body to fall back to runtime
        // dispatch.
        let m = lower_to_ir(
            "proc factory {} { set name {Verbose}\n proc $name {x} [subst -nocommands {return $default}] }",
            &reg(),
        );
        // Verbose not registered (because \$default missing means
        // we keep the dynamic body which routes via Barrier — but
        // the proc name itself was substituted). Actually
        // the proc IS registered with whatever the body lowering
        // produces; the assertion is that it's NOT the materialised
        // form.
        let inner = m.procedures.get("::Verbose");
        if let Some(p) = inner {
            // The body should not contain a Return whose value is
            // the literal "0" — that would be the materialised
            // form. It can be empty / contain a Barrier.
            // Conservative check: just verify the helper refused.
            // (The body might still lower the original CMD-token
            // text as a runtime call.)
            assert!(
                p.body.statements.is_empty()
                    || matches!(
                        p.body.statements[0],
                        Statement::Call { .. } | Statement::Barrier { .. }
                    )
            );
        }
    }

    #[test]
    fn proc_subst_nocommands_nobackslashes_refused() {
        // ``-nobackslashes`` flag — semantics differ from our
        // evaluator's default. Refuse and fall through.
        let m = lower_to_ir(
            "proc factory {} { set name {Verbose}\n set default {0}\n proc $name {x} [subst -nobackslashes -nocommands {return $default}] }",
            &reg(),
        );
        let inner = m.procedures.get("::Verbose");
        if let Some(p) = inner {
            // Body did not materialise — should be empty or have
            // a fallback shape.
            assert!(
                p.body.statements.is_empty()
                    || matches!(
                        p.body.statements[0],
                        Statement::Call { .. } | Statement::Barrier { .. }
                    )
            );
        }
    }

    #[test]
    fn proc_dynamic_name_picks_latest_set() {
        // Multiple ``set`` calls — the most recent literal wins.
        let m = lower_to_ir(
            "proc factory {} { set name {First}\n set name {Second}\n proc $name {} { puts hi } }",
            &reg(),
        );
        assert!(m.procedures.contains_key("::Second"));
        assert!(!m.procedures.contains_key("::First"));
    }

    #[test]
    fn ns_export_recorded() {
        let m = lower_to_ir("namespace eval ::tcltest { namespace export test }", &reg());
        // The export was inside a ``namespace eval`` body so the
        // context namespace is ``::tcltest``.
        assert!(
            m.namespace_exports
                .iter()
                .any(|(ns, pat)| ns == "::tcltest" && pat == "test")
        );
    }

    #[test]
    fn ns_import_in_dead_branch_suppressed() {
        // ``if {0} { namespace import ::evil::* }`` — the
        // import is inside a syntactically-dead branch so it must
        // NOT be recorded.
        let m = lower_to_ir("if {0} { namespace import ::evil::* }", &reg());
        assert!(
            m.namespace_imports.is_empty(),
            "imports inside dead if{{0}} branch must not be collected, got {:?}",
            m.namespace_imports,
        );
    }

    #[test]
    fn ns_import_in_static_true_else_suppressed() {
        // ``if {1} { ... } else { namespace import ::evil::* }``
        // — the else branch is dead.
        let m = lower_to_ir(
            "if {1} { namespace import ::good::* } else { namespace import ::evil::* }",
            &reg(),
        );
        // Only ``::good::*`` recorded.
        assert_eq!(m.namespace_imports.len(), 1);
        assert_eq!(m.namespace_imports[0].1, "::good::*");
    }

    #[test]
    fn const_prop_inherited_into_catch_body() {
        // Child scope (catch body) inherits parent's const-map.
        let m = lower_to_ir(
            "proc f {} { set body {set x 1}\n catch { uplevel 1 $body } }",
            &reg(),
        );
        let proc = m.procedures.get("::f").expect("proc registered");
        // Look for a Catch wrapping an UpFrame.
        let catch_stmt = proc
            .body
            .statements
            .iter()
            .find(|s| matches!(s, Statement::Catch { .. }))
            .expect("expected Catch");
        if let Statement::Catch { body, .. } = catch_stmt {
            assert!(
                body.statements
                    .iter()
                    .any(|s| matches!(s, Statement::UpFrame { .. })),
                "expected UpFrame inside catch body, got {body:?}",
            );
        }
    }

    // trace add execution module-fact population

    #[test]
    fn trace_add_execution_literal_recorded() {
        let m = lower_to_ir("trace add execution foo enter handler", &reg());
        assert!(m.traced_commands.contains("foo"));
        assert!(!m.has_dynamic_trace);
    }

    #[test]
    fn trace_add_execution_dynamic_widens() {
        let m = lower_to_ir("trace add execution $cmd enter handler", &reg());
        assert!(m.has_dynamic_trace);
        assert!(m.traced_commands.is_empty());
    }

    #[test]
    fn trace_add_execution_qualified_canonicalised() {
        let m = lower_to_ir("trace add execution ::ns::foo enter h", &reg());
        // Stripped of leading ``::`` so the GVN gate's
        // `command.trim_start_matches("::")` lookup hits.
        assert!(m.traced_commands.contains("ns::foo"));
    }

    #[test]
    fn trace_add_variable_does_not_record_execution_trace() {
        // `trace add variable` is a separate channel — should not
        // populate `traced_commands` (those are command traces only).
        let m = lower_to_ir("trace add variable x write h", &reg());
        assert!(m.traced_commands.is_empty());
        assert!(!m.has_dynamic_trace);
    }

    /// Regression coverage for issue #996: `walk_for_trace`'s post-lower
    /// scan recurses once per nested `if`/`for`/`while`/`foreach`/`catch`/
    /// `try`/`switch`/`Block`/`UpFrame` body, with no depth cap of its own
    /// before this fix. Transitively bounded to `MAX_LOWER_NEST_DEPTH`
    /// (256) by `lower_script`/`lower_body` (this same `lower_to_ir` call
    /// builds the `Script` `walk_for_trace` then scans), so this is
    /// defence-in-depth / consistency with every other full-tree walker in
    /// this crate, not a currently-reproducible crash. 1000 levels of
    /// source nesting is comfortably past the new cap; the assertion is
    /// that lowering (which runs `populate_trace_facts` ->
    /// `walk_for_trace`) returns at all, not what it returns. Spawns its
    /// own big-stack thread since the lexer/CST/segmenter stages upstream
    /// of `lower_script`'s own cap still walk the full un-truncated source
    /// nesting before that cap trims it — same rationale as
    /// `codegen::structured::tests::deeply_nested_if_survives_structured_walk`.
    #[test]
    fn deeply_nested_if_survives_walk_for_trace() {
        const DEPTH: usize = 1000;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut src = String::new();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("trace add execution foo enter handler\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let _ = lower_to_ir(&src, &reg());
            })
            .unwrap()
            .join()
            .unwrap();
    }

    // trace add/remove/variable/vdelete module-fact population

    #[test]
    fn trace_add_variable_literal_recorded() {
        let m = lower_to_ir("trace add variable x write h", &reg());
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
        assert!(!m.has_dynamic_variable_trace);
    }

    #[test]
    fn trace_remove_variable_literal_recorded() {
        // A `remove` only proves a trace *might* have existed — still
        // conservative to record, mirroring `trace add`.
        let m = lower_to_ir("trace remove variable x write h", &reg());
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_add_variable_qualified_canonicalised() {
        let m = lower_to_ir("trace add variable ::x write h", &reg());
        // `::`-stripped so a top-level `set x 5` (chain key `x`) matches.
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_add_variable_dynamic_widens() {
        let m = lower_to_ir("trace add variable $name write h", &reg());
        assert!(m.has_dynamic_variable_trace);
        assert!(m.traced_variables.is_empty());
    }

    #[test]
    fn trace_add_variable_inside_proc_recorded() {
        let m = lower_to_ir(
            "proc setup {} { trace add variable ::x read onread }",
            &reg(),
        );
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_add_variable_inside_method_recorded() {
        // TclOO method bodies are extracted into `module.methods` (a
        // separate map from `module.procedures`) before this scan runs —
        // must not be missed just because it isn't a plain proc.
        let m = lower_to_ir(
            "oo::class create C {\n method m {} { trace add variable v write cb }\n}",
            &reg(),
        );
        assert!(m.traced_variables.contains("v"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_legacy_variable_form_recorded() {
        // The deprecated `trace variable name ops command` spelling must
        // populate the same fact as `trace add variable` — no
        // hardcoded-per-form gap.
        let m = lower_to_ir("trace variable x r onread", &reg());
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_legacy_vdelete_form_recorded() {
        let m = lower_to_ir("trace vdelete x r onread", &reg());
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_info_variable_does_not_record() {
        // `trace info`/`vinfo` only query trace state — no
        // `ESTABLISHES_VARIABLE_TRACE` trait, so they must not widen the
        // module fact.
        let m = lower_to_ir("trace info variable x", &reg());
        assert!(m.traced_variables.is_empty());
        assert!(!m.has_dynamic_variable_trace);
    }

    #[test]
    fn trace_add_variable_through_interp_alias_recorded() {
        // `interp alias {} tracer {} trace` means `tracer add variable ...`
        // is really a `trace add variable ...` call — the whole-module scan
        // must key off the canonical (alias-resolved) command name, not the
        // source-surface `tracer` spelling, or this trace target is missed.
        let m = lower_to_ir(
            "interp alias {} tracer {} trace\ntracer add variable x write h",
            &reg(),
        );
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_add_execution_through_interp_alias_recorded() {
        // Same alias-resolution requirement for the execution-trace channel:
        // `walk_for_trace` gates entry into both `traced_commands` and
        // `populate_variable_trace_facts` on one shared canonical-name check.
        let m = lower_to_ir(
            "interp alias {} tracer {} trace\ntracer add execution foo enter h",
            &reg(),
        );
        assert!(m.traced_commands.contains("foo"), "{:?}", m.traced_commands);
        assert!(!m.has_dynamic_trace);
    }

    #[test]
    fn trace_add_execution_does_not_record_variable_trace() {
        // The command-execution channel is separate — should not
        // populate `traced_variables`.
        let m = lower_to_ir("trace add execution foo enter h", &reg());
        assert!(m.traced_variables.is_empty());
        assert!(!m.has_dynamic_variable_trace);
    }

    #[test]
    fn trace_add_execution_inside_proc_recorded() {
        let m = lower_to_ir("proc init {} { trace add execution foo enter h }", &reg());
        assert!(
            m.traced_commands.contains("foo"),
            "traced_commands={:?}",
            m.traced_commands,
        );
    }

    #[test]
    fn trace_add_variable_abbreviated_type_word_recorded() {
        // `trace add v x write h` / `trace add var x write h` install the
        // same variable trace as the full `variable` spelling — C Tcl
        // accepts any unique prefix (checked against tclsh 8.6.14).
        let m = lower_to_ir("trace add v x write h", &reg());
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
        let m = lower_to_ir("trace add var x write h", &reg());
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_remove_variable_abbreviated_type_word_recorded() {
        let m = lower_to_ir("trace remove var x write h", &reg());
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_add_execution_abbreviated_type_word_not_variable() {
        // `e`/`exec` abbreviate `execution`, not `variable` — must not
        // widen `traced_variables`.
        let m = lower_to_ir("trace add e foo enter h", &reg());
        assert!(m.traced_variables.is_empty());
        assert!(m.traced_commands.contains("foo"), "{:?}", m.traced_commands);
    }

    #[test]
    fn trace_add_variable_inside_namespace_eval_body_recorded() {
        // A trace installed inside a `namespace eval` body (a synthetic
        // `Module::body_units` entry, not a named proc) must populate the
        // same whole-module fact as one inside a `proc`.
        let m = lower_to_ir(
            "namespace eval ::n { trace add variable ::x write h }",
            &reg(),
        );
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_add_variable_inside_apply_body_recorded() {
        let m = lower_to_ir("apply {{} { trace add variable ::x write h }}", &reg());
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    #[test]
    fn trace_add_variable_inside_method_body_recorded() {
        let m = lower_to_ir(
            "oo::class create C { method m {} { trace add variable ::x write h } }",
            &reg(),
        );
        assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    }

    // barrier-gate

    #[test]
    fn body_has_dynamic_barrier_clean() {
        // No barriers at all.
        assert!(!body_has_dynamic_barrier("set x 1", &reg()));
        // Barrier with a fully literal body.
        assert!(!body_has_dynamic_barrier("eval { set x 1 }", &reg()));
        // Nested literal.
        assert!(!body_has_dynamic_barrier(
            "if { 1 } { eval { set x 1 } }",
            &reg()
        ));
    }

    #[test]
    fn body_has_dynamic_barrier_dynamic_eval_body() {
        // ``eval $x`` inside the outer body — dynamic.
        assert!(body_has_dynamic_barrier("eval $x", &reg()));
        // Same shape nested.
        assert!(body_has_dynamic_barrier("if { 1 } { eval $x }", &reg()));
    }

    #[test]
    fn body_has_dynamic_barrier_dynamic_uplevel_body() {
        assert!(body_has_dynamic_barrier("uplevel 1 $body", &reg()));
        assert!(body_has_dynamic_barrier("uplevel #0 $b", &reg()));
    }

    #[test]
    fn body_has_dynamic_barrier_uplevel_with_literal_body_clean() {
        // ``uplevel $lvl {body}`` with a literal body is OK — the
        // gate only poisons when the BODY is substitution-bearing.
        assert!(!body_has_dynamic_barrier("uplevel $lvl {set x 1}", &reg()));
    }

    #[test]
    fn body_has_dynamic_barrier_qualified_eval_uplevel() {
        // ``::eval`` and ``::uplevel`` are also caught.
        assert!(body_has_dynamic_barrier("::eval $x", &reg()));
        assert!(body_has_dynamic_barrier("::uplevel 1 $body", &reg()));
    }

    /// Issue #1055 — the gate resolves `uplevel`'s script word through the
    /// registry's `ArgRole::Body` resolver, not a local level-word sniff, so
    /// every documented `uplevel ?level? arg ?arg ...?` shape agrees with
    /// `uplevel_body_arg_role_skips_optional_level` in `tcl-registry`.
    #[test]
    fn body_has_dynamic_barrier_uplevel_level_shapes_follow_the_registry() {
        let r = reg();
        // TN — a literal script in each level spelling relaxes.
        for clean in [
            "uplevel {set x 1}",
            "uplevel 1 {set x 1}",
            "uplevel #0 {set x 1}",
            "uplevel $lvl {set x 1}",
            "uplevel [expr {$n - 1}] {set x 1}",
        ] {
            assert!(
                !body_has_dynamic_barrier(clean, &r),
                "{clean} must not poison"
            );
        }
        // TP — a substituted script word in each level spelling poisons.
        for dirty in [
            "uplevel $body",
            "uplevel 1 $body",
            "uplevel #0 $body",
            "uplevel $lvl $body",
            "uplevel 1 [gen]",
        ] {
            assert!(body_has_dynamic_barrier(dirty, &r), "{dirty} must poison");
        }
        // FN guard — the words after the script word concatenate into it
        // (`SCRIPT_CONCATENATES_ARGS`), so a dynamic tail is still dynamic
        // even though the marked body word is a braced literal.
        assert!(body_has_dynamic_barrier("uplevel 1 {set x} $tail", &reg()));
        assert!(body_has_dynamic_barrier("eval {set x} $tail", &reg()));
        // …and an all-literal multi-word tail stays clean.
        assert!(!body_has_dynamic_barrier("uplevel 1 {set x} {1}", &reg()));
        // A bodyless `uplevel 1` is a wrong-#args error the registry exposes
        // no body word for — poison rather than guess.
        assert!(body_has_dynamic_barrier("uplevel 1", &reg()));
        // The deleted local sniff accepted `-N` as a level (it stripped a
        // leading `-` before the digit test); the registry does not, and the
        // registry is right.  Oracle, tclsh8.6.14: with `proc -1 {args} {…}`
        // defined, `uplevel -1 {set x 1}` *calls* `-1` with `set x 1` — the
        // word is the script's first word, not a level.  tclsh9.0.4 instead
        // errors `bad level "-1"`.  Under either reading the word is an
        // unbraced script word, so the gate poisons.
        assert!(body_has_dynamic_barrier("uplevel -1 {set x 1}", &reg()));
    }

    /// Issue #1055, the generalisation dividend: the eval-family *subcommand*
    /// members carry `EVALUATES_CODE` / `SCRIPT_CONCATENATES_ARGS` on the
    /// subcommand, not on the `namespace` / `interp` spec.  Composing them
    /// (`invocation_traits`) plus registry-resolved body indices makes them
    /// flow through the same path a bare `eval` does, which a parent-only
    /// trait test missed entirely.
    #[test]
    fn body_has_dynamic_barrier_covers_compound_eval_family() {
        let r = reg();
        // TP — the script is substituted.
        assert!(body_has_dynamic_barrier("namespace eval ns $body", &r));
        assert!(body_has_dynamic_barrier("namespace inscope ns $body", &r));
        assert!(body_has_dynamic_barrier("interp eval slave $body", &r));
        // TN — a braced literal script relaxes, and its own contents are
        // still recursed.
        assert!(!body_has_dynamic_barrier("namespace eval ns {set x 1}", &r));
        assert!(body_has_dynamic_barrier("namespace eval ns {eval $x}", &r));
        // TN — a `namespace` subcommand that evaluates nothing is not a
        // barrier, however dynamic its arguments are.
        assert!(!body_has_dynamic_barrier("namespace delete $ns", &r));
        assert!(!body_has_dynamic_barrier("namespace current", &r));
    }

    /// TN for the barrier test itself: an ordinary command with a
    /// substituted argument is not a script evaluator, so it never poisons —
    /// the gate keys on the registry trait, not on "has a `$var` argument".
    #[test]
    fn body_has_dynamic_barrier_non_body_command_is_clean() {
        let r = reg();
        assert!(!body_has_dynamic_barrier("set x $y", &r));
        assert!(!body_has_dynamic_barrier("puts [format %s $x]", &r));
        assert!(!body_has_dynamic_barrier("lappend acc $item", &r));
        // A user command the registry does not know at all is not a barrier.
        assert!(!body_has_dynamic_barrier("mycmd $script", &r));
    }

    #[test]
    fn try_lower_eval_static_rejects_nested_dynamic_barrier() {
        // The relaxer would normally promote ``eval { ... }`` to a
        // ``Statement::Block``; with the barrier gate, the nested
        // ``eval $x`` poisons the relaxation and we fall back to
        // ``Statement::Barrier``.
        let m = lower_to_ir(r"eval { eval $x }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Barrier { .. }),
            "expected Barrier (gate triggered), got {stmt:?}",
        );
    }

    #[test]
    fn try_lower_eval_static_clean_body_relaxes() {
        // No nested barrier — relaxes to Block.
        let m = lower_to_ir("eval { set x 1 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        let Statement::Block { error_context, .. } = stmt else {
            panic!("expected Block (relaxed), got {stmt:?}");
        };
        assert_eq!(
            *error_context,
            Some(tcl_registry::InlineBodyErrorContext::SameFrameScriptEvaluation),
        );
    }

    // The next three tests confirm `eval` and `uplevel` dispatch
    // through the registry-driven hook ID to the expected output.

    #[test]
    fn registry_dispatch_eval_static_body_lowers_to_block() {
        // `eval {body}` with a literal body should still relax
        // to `Statement::Block` when dispatched through the
        // registry-driven path.
        let m = lower_to_ir("eval { set y 2 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Block { .. }),
            "expected Block via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_uplevel_static_body_lowers_to_upframe() {
        // `uplevel 1 {body}` with a literal body should lower
        // to a UpFrame via the registry-driven path.
        let m = lower_to_ir("uplevel 1 { set z 3 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::UpFrame { .. }),
            "expected UpFrame via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_unknown_command_falls_through_to_default() {
        // `lower_command` has no per-
        // name fallback — commands that aren't in the registry
        // (or whose `lowering_hook` is `None`) route directly
        // to [`Lowerer::lower_default`], which emits a generic
        // `Statement::Call`.  Pin that contract with a clearly
        // unknown command name.
        let m = lower_to_ir("totallyMadeUpCommand arg1 arg2", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        match stmt {
            Statement::Call { command, .. } => assert_eq!(
                command, "totallyMadeUpCommand",
                "expected generic Call via lower_default, got command={command}",
            ),
            other => panic!("expected Call via lower_default, got {other:?}"),
        }
    }

    // These tests pin the registry-driven hook-ID dispatch of `if`,
    // `switch`, `for`, `while`, `catch`, `try` to the expected shape.

    #[test]
    fn registry_dispatch_if_lowers_to_if() {
        let m = lower_to_ir("if {1} { set q 4 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::If { .. }),
            "expected If via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_switch_lowers_to_switch() {
        let m = lower_to_ir("switch a { a { set q 1 } b { set q 2 } }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Switch { .. }),
            "expected Switch via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_for_lowers_to_for() {
        let m = lower_to_ir("for {set i 0} {$i < 3} {incr i} { set q $i }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::For { .. }),
            "expected For via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_while_lowers_to_while() {
        let m = lower_to_ir("while {0} { set q 1 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::While { .. }),
            "expected While via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_catch_lowers_to_catch() {
        let m = lower_to_ir("catch { set q 1 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Catch { .. }),
            "expected Catch via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_try_lowers_to_try() {
        let m = lower_to_ir("try { set q 1 } on error e { set q 2 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Try { .. }),
            "expected Try via registry hook, got {stmt:?}",
        );
    }

    // `proc`, `namespace eval`, `foreach`, `lmap`, `dict` dispatch
    // through registry-driven hook-ID, with shape preconditions
    // enforced inside each match arm.

    #[test]
    fn registry_dispatch_proc_lowers_to_proc_call_and_registers_procedure() {
        let m = lower_to_ir("proc greet {name} { puts hi }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        match stmt {
            Statement::Call { command, .. } => assert_eq!(
                command, "proc",
                "expected `proc` call via registry hook, got command={command}",
            ),
            other => panic!("expected Call via registry hook, got {other:?}"),
        }
        assert!(
            m.procedures.contains_key("::greet"),
            "expected ::greet to be registered in module.procedures, got keys={:?}",
            m.procedures.keys().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn registry_dispatch_proc_wrong_arity_falls_through_to_default() {
        // `proc` with two args — fails the `args.len() == 3`
        // precondition.  The registry-driven path returns None,
        // falls through to the residual string match's
        // `_ => lower_default(...)` arm, which produces a
        // runtime `Call` rather than registering a procedure.
        let m = lower_to_ir("proc greet onlytwo", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Call { .. }),
            "expected Call via lower_default fallback, got {stmt:?}",
        );
        assert!(
            !m.procedures.contains_key("::greet"),
            "expected ::greet NOT to be registered when arity is wrong",
        );
    }

    #[test]
    fn registry_dispatch_namespace_eval_lowers_to_barrier() {
        // `lower_namespace_eval` emits a `Statement::Barrier`
        // tagged with `reason: "namespace eval"`.
        let m = lower_to_ir("namespace eval ::myns { set q 1 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        match stmt {
            Statement::Barrier { reason, .. } => assert_eq!(
                reason, "namespace eval",
                "expected `namespace eval` barrier via registry hook, got reason={reason}",
            ),
            other => panic!("expected Barrier via registry hook, got {other:?}"),
        }
    }

    #[test]
    fn registry_dispatch_namespace_non_eval_subcommand_falls_through() {
        // `namespace import` has no `lowering_hook` on its
        // subcommand spec, so `resolve_call` returns hook=None.
        // The registry-driven dispatcher returns None and the
        // residual string match's `_ => lower_default(...)`
        // arm produces a generic `Call`.
        let m = lower_to_ir("namespace import ::foo::*", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Call { .. }),
            "expected Call via lower_default fallback, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_foreach_lowers_to_foreach() {
        let m = lower_to_ir("foreach v {1 2 3} { set q $v }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Foreach { .. }),
            "expected Foreach via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_lmap_lowers_to_foreach() {
        // `lmap` shares `lower_foreach(... is_lmap=true)` with
        // `foreach`; both produce `Statement::Foreach`.
        let m = lower_to_ir("lmap v {1 2 3} { expr {$v + 1} }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Foreach { .. }),
            "expected Foreach (is_lmap=true) via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_dict_lowers_via_lower_dict() {
        // `dict set d k v` exercises the `lower_dict`
        // subcommand-dispatch path.  Pin the registry-driven
        // route by checking the canonical command name on the
        // emitted Call.
        let m = lower_to_ir("dict set d k v", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        match stmt {
            Statement::Call { command, .. } => assert_eq!(
                command, "dict",
                "expected `dict` call via registry hook, got command={command}",
            ),
            other => panic!("expected Call via registry hook, got {other:?}"),
        }
    }

    #[test]
    fn registry_dispatch_dict_empty_args_falls_through_to_default() {
        // Bare `dict` — fails the `!args.is_empty()`
        // precondition, falls through to `lower_default`.
        let m = lower_to_ir("dict", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Call { .. }),
            "expected Call via lower_default fallback, got {stmt:?}",
        );
    }

    // `when` and `foreachLine` dispatch through registry-driven
    // hook-ID.  `when` is dialect-gated (iRules), so its test
    // registry loads the dialect explicitly.  `foreachLine` is
    // a Tcl 9.0+ command always registered in `build_default()`.

    #[test]
    fn registry_dispatch_when_lowers_via_lower_when_with_irules_loaded() {
        // The iRules `when` spec carries `LoweringHookId::When`
        // and only enters the registry after
        // `load_dialect(IRULES)`.  Production callers (the LSP
        // server) always pair `build_default()`
        // with the active dialect; this test mirrors that.
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        let m = lower_to_ir("when HTTP_REQUEST { set q 1 }", &registry);
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        match stmt {
            Statement::Call { command, .. } => assert_eq!(
                command, "when",
                "expected `when` call via registry hook, got command={command}",
            ),
            other => panic!("expected Call via registry hook, got {other:?}"),
        }
        assert!(
            m.procedures.contains_key("::when::HTTP_REQUEST"),
            "expected ::when::HTTP_REQUEST procedure to be registered, got keys={:?}",
            m.procedures.keys().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn registry_dispatch_when_without_irules_loaded_falls_through() {
        // Without `load_irules()`, the registry has no `when`
        // spec, so `resolve_call` returns `None` and
        // `try_dispatch_structured_hook` falls through to
        // `lower_default`.  This is a misconfiguration on the
        // caller's side (the source uses iRules but the
        // registry doesn't know about it); the lowerer emits a
        // generic `Call` rather than silently treating it as
        // an event handler.  Pinning this behaviour catches
        // future drift if `build_default()` ever folds in
        // iRules.
        let m = lower_to_ir("when HTTP_REQUEST { set q 1 }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        match stmt {
            Statement::Call { command, .. } => assert_eq!(
                command, "when",
                "expected generic Call via lower_default, got command={command}",
            ),
            other => panic!("expected Call via lower_default, got {other:?}"),
        }
        assert!(
            !m.procedures.keys().any(|k| k.starts_with("::when::")),
            "expected NO ::when::* procedure registered without iRules loaded",
        );
    }

    #[test]
    fn registry_dispatch_foreach_line_lowers_via_lower_foreach_line() {
        // `foreachLine` is a Tcl 9.0+ command (TIP 670) and
        // `build_default()` registers it via the
        // `tcl::foreachline` spec.  No dialect-load dance is
        // needed.  The body is a brace-string literal here, so
        // the dedicated lowerer relaxes to a typed `Foreach`
        // (not a barrier).
        let m = lower_to_ir("foreachLine line readme.txt { set q $line }", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        assert!(
            matches!(stmt, Statement::Foreach { .. }),
            "expected Foreach via registry hook, got {stmt:?}",
        );
    }

    #[test]
    fn registry_dispatch_foreach_line_wrong_arity_falls_through_to_default() {
        // `foreachLine` with two args — fails the
        // `args.len() == 3` precondition, falls through to
        // `lower_default`.
        let m = lower_to_ir("foreachLine onlytwo args", &reg());
        let stmt = m.top_level.statements.first().expect("at least one stmt");
        match stmt {
            Statement::Call { command, .. } => assert_eq!(
                command, "foreachLine",
                "expected generic Call via lower_default, got command={command}",
            ),
            other => panic!("expected Call via lower_default, got {other:?}"),
        }
    }

    /// `if {1} { if {1} { ... } }`, `depth` levels deep.
    fn nested_if_source(depth: u32) -> String {
        let mut source = String::new();
        for _ in 0..depth {
            source.push_str("if {1} {\n");
        }
        for _ in 0..depth {
            source.push_str("}\n");
        }
        source
    }

    /// Depth-first search for a [`Statement::Barrier`] whose `reason`
    /// contains `needle`, anywhere in `script`'s tree (into `If` clauses and
    /// `else` bodies — the only nesting `nested_if_source` produces).
    fn contains_barrier_reason(script: &Script, needle: &str) -> bool {
        script.statements.iter().any(|stmt| match stmt {
            Statement::Barrier { reason, .. } => reason.contains(needle),
            Statement::If {
                clauses, else_body, ..
            } => {
                clauses
                    .iter()
                    .any(|c| contains_barrier_reason(&c.body, needle))
                    || else_body
                        .as_ref()
                        .is_some_and(|e| contains_barrier_reason(e, needle))
            }
            _ => false,
        })
    }

    /// Issue #996: `lower_script` / `lower_body` recurse one Rust frame
    /// group per `if` nesting level with no depth cap prior to this fix —
    /// unlike `analyse_body` (`tcl_compiler::analyser::commands::
    /// MAX_BODY_DEPTH`) and `cfg_builder::lower_script`
    /// (`MAX_LOWER_DEPTH`), both already guarded. A document whose IR
    /// lowering runs unguarded defeats the CFG builder's own guard
    /// downstream — the crash happens one stage earlier. Lowering 2000
    /// levels must neither hang nor overflow the stack, and must record the
    /// cap trip as a `Statement::Barrier` (unknown effects) rather than
    /// silently truncating to an empty, falsely-dead-looking script.
    #[test]
    fn deeply_nested_if_past_max_lower_depth_barriers_not_crashes() {
        let source = nested_if_source(2000);
        // `cargo test` runs each test on a small default-stack thread (the
        // same undersized budget issue #996 was actually caused by) — see
        // the identical helper's doc comment in `analyser::commands::tests`.
        let m = std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(move || lower_to_ir(&source, &reg()))
            .expect("spawn big-stack test thread")
            .join()
            .expect("lower_to_ir on big-stack thread panicked");
        assert!(
            contains_barrier_reason(&m.top_level, "nesting depth exceeds analysis limit"),
            "expected a depth-cap Barrier somewhere in the lowered tree"
        );
    }

    /// Depth of the deepest chain of nested `If` statements in `script`.
    fn if_chain_depth(script: &Script) -> u32 {
        script
            .statements
            .iter()
            .map(|stmt| match stmt {
                Statement::If { clauses, .. } => {
                    1 + clauses.first().map_or(0, |c| if_chain_depth(&c.body))
                }
                _ => 0,
            })
            .max()
            .unwrap_or(0)
    }

    #[test]
    fn shallow_nested_if_lowers_with_no_barrier() {
        // False-positive guard: ordinary, hand-written nesting must lower
        // as real `If` statements all the way down, never barriered.
        let source = nested_if_source(10);
        let m = lower_to_ir(&source, &reg());
        assert!(
            !contains_barrier_reason(&m.top_level, "nesting depth exceeds analysis limit"),
            "shallow nesting must not trip the depth cap"
        );
        // 10 real nested `If`s, not a single opaque barrier standing in for
        // all of them.
        assert_eq!(if_chain_depth(&m.top_level), 10);
    }
}
