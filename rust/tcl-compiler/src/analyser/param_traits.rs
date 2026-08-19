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

//! Proc argument trait inference.
//!
//! Walks a proc body to determine how each parameter is used:
//!
//! - `Eval` — passed to ``eval`` / ``uplevel`` / ``subst``
//! - `Body` — used as a loop / control body
//! - `VarWrite` — names a variable the proc writes (upvar +
//!   ``set`` / ``incr`` / ``append`` / ``lappend``, or a
//!   registry-marked variable-write site)
//! - `VarRead` — names a variable the proc reads via ``upvar``
//! - `Expr` — evaluated as an expression
//! - `LoopList` — used as the list arg in ``foreach`` / ``lmap``
//! - `DynamicNameLocal` — the param's *value* names a
//!   **callee-local** variable (``set $p 1`` / ``scan … $p`` /
//!   ``lassign … $p`` / ``regsub … $p``, or a registry
//!   ``VarWrite`` / ``VarRead`` role on a bare ``$param``).  Emitted
//!   with `VarRead`; refines it so caller-side dead-store / unused
//!   suppression does **not** treat a literal-name call arg as
//!   consumed (the callee uses the string to name its *own* local,
//!   not a caller-frame alias).
//!
//! Two passes are exposed:
//!
//! * [`infer_param_traits`] — shallow command scan.  Fast enough
//!   for synchronous use during analysis; detects direct patterns
//!   like ``eval $param``, ``upvar 1 $param local``, ``foreach x
//!   $list body``.  It **does** descend into ``[...]`` command
//!   substitutions (`return [set $v]` is a first-order use), and
//!   recognises variable names built from parameters — a bare
//!   ``$p``, a compound array element (``set ${v}($k)`` marks both
//!   ``v`` and ``k``), and ``namespace upvar ns $token arr``.  It
//!   does **not** recurse into braced *body* arguments (that is the
//!   deep pass's job).
//! * [`infer_param_traits_deep`] — recursive descent into
//!   braced body args, catching traits hidden one or more
//!   levels deep (`foreach item $items { uplevel 1 $body }`
//!   surfaces the `$body` Eval trait via the recursion).  More
//!   expensive than the shallow pass; intended for asynchronous
//!   analysis (call-graph / symbol-graph / dataflow-graph /
//!   semantic-graph builders).  Bounded by [`MAX_DEPTH`]
//!   (8 levels) to prevent runaway recursion on pathological
//!   input.
//!
//! [`merge_traits`] unions the two passes' results when
//! callers want both.

use std::collections::{HashMap, HashSet};

use tcl_registry::CommandRegistry;
use tcl_registry::arg_role::ArgRole;
use tcl_registry::stub_overlay::StubOverlay;

use super::types::ProcArgTrait;
use crate::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::{LexerConfig, TokenType};

/// Top-level shallow trait inference.  Returns a map from
/// parameter name to a set of inferred traits.  Empty entries
/// (parameters with no detected trait) are dropped from the
/// returned map.
///
/// `env` carries the dialect-aware registry, the document's
/// `# tcl-lsp: stub` overlay, the dialect lexer configuration, and the
/// document's proven command-identity facts — see [`TraitScanEnv`].
#[must_use]
pub fn infer_param_traits(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    if params.is_empty() || body_source.trim().is_empty() {
        return HashMap::new();
    }
    let param_set: HashSet<&str> = params.iter().copied().collect();
    let mut traits: HashMap<&str, HashSet<ProcArgTrait>> =
        params.iter().map(|p| (*p, HashSet::new())).collect();
    let mut aliases = Aliases::default();

    let ctx = ScanCtx {
        param_set: &param_set,
        registry: env.registry,
        stub_overlay: env.stub_overlay,
        config: env.config,
        identities: env.identities,
    };
    scan_commands(body_source, &ctx, &mut traits, &mut aliases);

    finalise_traits(traits)
}

/// Maximum recursion depth for [`infer_param_traits_deep`]
/// (8 levels).  Pathological input (deeply-nested braced bodies)
/// stops descending past this bound rather than blowing the
/// stack.
pub const MAX_DEPTH: u8 = 8;

/// Recursive deep trait inference.  Same return shape as
/// [`infer_param_traits`] but additionally descends into braced
/// body arguments to surface traits hidden one or more levels
/// in.  More expensive than the shallow pass — intended for
/// asynchronous use behind the call-graph / symbol-graph /
/// dataflow-graph / semantic-graph builders.
///
/// Recursion is bounded by [`MAX_DEPTH`] and only enters braced
/// body args — `$var` or `[cmd]` references at the head of a
/// body arg are treated as opaque (their `Eval` trait is already
/// captured at the top level by the same call-site's role
/// scan).
#[must_use]
pub fn infer_param_traits_deep(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    infer_param_traits_deep_at_depth(params, body_source, env, 0)
}

/// [`infer_param_traits_deep`] with an explicit starting depth.
///
/// The public entry points always start at `0`; `scan_deep`'s own `apply`
/// (`ArgRole::LambdaLiteral`) handling calls this with `depth + 1` instead
/// of re-entering at `0`. A lambda body is inferred in its own frame (see
/// the doc comment at that call site for why), but it is still lexically
/// nested inside the enclosing scan — starting it back at depth `0` would
/// let alternating `if {…} { apply {x {…}} … }` nesting reset the logical
/// counter on every `apply`, defeating [`MAX_DEPTH`] while the *native*
/// call stack keeps growing one `scan_deep` ↔
/// `infer_param_traits_deep_at_depth` frame group per level regardless —
/// the same "guard exists but doesn't cover every recursive edge" bug
/// class as issue #996 / #997.
fn infer_param_traits_deep_at_depth(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
    depth: u8,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    if params.is_empty() || body_source.trim().is_empty() || depth > MAX_DEPTH {
        return HashMap::new();
    }
    let param_set: HashSet<&str> = params.iter().copied().collect();
    let mut traits: HashMap<&str, HashSet<ProcArgTrait>> =
        params.iter().map(|p| (*p, HashSet::new())).collect();
    let mut aliases = Aliases::default();

    let ctx = ScanCtx {
        param_set: &param_set,
        registry: env.registry,
        stub_overlay: env.stub_overlay,
        config: env.config,
        identities: env.identities,
    };
    scan_deep(body_source, &ctx, &mut traits, &mut aliases, depth);

    finalise_traits(traits)
}

/// Union shallow + deep trait results per parameter.  Useful
/// when callers want to run the shallow pass synchronously for
/// an initial result and then upgrade with the deep pass once it
/// completes.
///
/// Generic over the hasher so it composes with whatever
/// [`infer_param_traits`] / [`infer_param_traits_deep`] return.
#[must_use]
pub fn merge_traits<S, H>(
    shallow: HashMap<String, HashSet<ProcArgTrait, H>, S>,
    deep: HashMap<String, HashSet<ProcArgTrait, H>, S>,
) -> HashMap<String, HashSet<ProcArgTrait, H>, S>
where
    S: std::hash::BuildHasher,
    H: std::hash::BuildHasher + Default,
{
    let mut merged = shallow;
    for (param, deep_traits) in deep {
        merged.entry(param).or_default().extend(deep_traits);
    }
    merged
}

/// Drop parameters with no detected trait and convert the
/// borrowed keys back to owned `String`s.  Shared between
/// `infer_param_traits` and `infer_param_traits_deep` so both
/// pass shapes return the same kind of map.
fn finalise_traits(
    traits: HashMap<&str, HashSet<ProcArgTrait>>,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    traits
        .into_iter()
        .filter(|(_, set)| !set.is_empty())
        .map(|(k, v)| (k.to_string(), v))
        .collect()
}

/// Read-only inference state threaded unchanged through the
/// shallow and deep scan family (`scan_commands` / `scan_command`
/// / `scan_deep`).  Bundling these into one borrowing context
/// keeps each scan helper at or under the 7-argument limit.
///
/// `'p` is the params' lifetime (the param-name slices borrowed
/// from the caller's `params` argument); `'r` borrows the
/// registry / overlay / param-set for the duration of a scan.
struct ScanCtx<'p, 'r> {
    param_set: &'r HashSet<&'p str>,
    registry: &'r CommandRegistry,
    stub_overlay: Option<&'r StubOverlay>,
    config: LexerConfig,
    /// The **document's** proven command-identity facts (issue #1275), so a
    /// role, frame-effect, or structural handler is chosen by the command a
    /// head *is* rather than the one it is spelled as.
    ///
    /// Read *unpositioned*: a proc body is segmented from its own text at
    /// offset 0, so no document-absolute offset exists here.
    identities: &'r crate::head_identity::HeadIdentityMap,
}

impl ScanCtx<'_, '_> {
    /// One command head in both its forms.
    fn head<'h>(&'h self, written: &'h str) -> crate::head_identity::HeadWords<'h> {
        self.identities.head_words_unpositioned(written)
    }
}

/// Everything a param-trait scan needs about the document it is scanning
/// inside: the dialect-aware registry, the per-document `# tcl-lsp: stub`
/// overlay, the dialect lexer configuration the body re-segments under, and
/// the document's proven command-identity facts.
///
/// Bundled so the four public entry points share one parameter rather than
/// four, and so adding a document-level fact does not re-open every signature.
#[derive(Clone, Copy)]
pub struct TraitScanEnv<'a> {
    /// The caller's already-built, dialect-aware registry (typically
    /// `Analyser::registry`).  Building a fresh `CommandRegistry::build_default()`
    /// per proc would both be expensive and miss the dialect-specific
    /// `arg_role_resolver` / `arg_roles` the caller's registry has loaded.
    pub registry: &'a CommandRegistry,
    /// The document's `# tcl-lsp: stub` overlay, when it declares any: a stub
    /// like `# tcl-lsp: stub my_eval {script:body}` makes a `my_eval $param`
    /// invocation mark the parameter `ProcArgTrait::Body`.
    pub stub_overlay: Option<&'a StubOverlay>,
    /// The dialect lexer configuration the body re-segments under.  `{*}`
    /// expansion (off for Tcl 8.4 / iRules) and the iRules `}{` ghost SEP
    /// change how a body splits into commands and words, which changes which
    /// arguments resolve to a clean `$param`.
    pub config: LexerConfig,
    /// The document's proven command-identity facts — see
    /// [`crate::head_identity`].  Pass
    /// [`HeadIdentityMap::none()`](crate::head_identity::HeadIdentityMap::none)
    /// when there is no document to scan.
    pub identities: &'a crate::head_identity::HeadIdentityMap,
}

/// Mutable alias / value-copy state accumulated while scanning a proc body.
/// Bundling the two maps keeps the scan helpers at or under the argument limit.
#[derive(Default)]
struct Aliases<'p> {
    /// `myVar` (an `upvar` / `namespace upvar` alias name) → the param it
    /// aliases, so a later `set myVar …` writes the caller's variable
    /// (`VarWrite`).
    upvar: HashMap<String, &'p str>,
    /// A local name → the param whose *value* it currently holds (`set n $p`),
    /// so a later `$n` in a name / command position resolves to that param.
    /// Invalidated when the local is written to anything else.
    value_copies: HashMap<String, &'p str>,
}

/// `true` when `name` is a plain scalar local identifier — the only shape a
/// tracked value-copy target can take (`set n $p` records `n`, but `set
/// arr(x)` / `set $n` do not).
fn is_plain_local_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
}

/// Single-level command scan shared by both passes.  Extracts
/// segmented commands from `source` and dispatches each through
/// [`scan_command`].
///
/// `source` has a lifetime independent of `'p` so callers can
/// re-enter with body-arg slices that don't outlive the
/// enclosing source.
fn scan_commands<'p>(
    source: &str,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
) {
    scan_commands_bounded(source, ctx, traits, aliases, 0);
}

/// [`scan_commands`] plus descent into `[...]` command substitutions, bounded
/// by [`MAX_DEPTH`].  A param used inside a substitution (`return [set $v]`,
/// `set x [foo $v]`, `set ${v}($k)` wrapped in `[...]`) is a first-order use of
/// that param, not a "deep" one, so both the shallow and deep passes see it —
/// the substitution is where much ordinary Tcl computation happens.  Nested
/// substitutions are reached by the recursion.
fn scan_commands_bounded<'p>(
    source: &str,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
    depth: u8,
) {
    let segments = segment_commands_with_offset_and_config(source, 0, ctx.config);
    for seg in &segments {
        if seg.texts.is_empty() {
            continue;
        }
        let cmd_args: Vec<String> = seg.texts[1..].to_vec();
        // Per-arg "braced literal" flags: a single `Str` word (`{…}`) suppresses
        // substitution, so a `$k` inside it is literal text, not a param ref.
        let braced: Vec<bool> = (1..seg.texts.len())
            .map(|i| {
                seg.argv.get(i).is_some_and(|t| t.kind == TokenType::Str)
                    && seg.single_token_word.get(i).copied().unwrap_or(false)
            })
            .collect();
        // The head names a command by *substitution* only when it lexes as a
        // `Var` word (`$cmd` / `${cmd}`); a braced `{$cmd}` or bareword head is
        // a literal command name, not a param reference.
        let head_is_var = seg.argv.first().is_some_and(|t| t.kind == TokenType::Var);
        scan_command(
            ctx.head(&seg.texts[0]),
            &cmd_args,
            &braced,
            head_is_var,
            ctx,
            traits,
            aliases,
        );
        if depth >= MAX_DEPTH {
            continue;
        }
        // A whole-word `Cmd` token's `texts` entry is the bracketed
        // substitution; strip the outer `[` / `]` and recurse into its script.
        for (word, tok) in seg.texts.iter().zip(seg.argv.iter()) {
            if tok.kind == TokenType::Cmd
                && let Some(inner) = word.strip_prefix('[').and_then(|w| w.strip_suffix(']'))
                && !inner.trim().is_empty()
            {
                scan_commands_bounded(inner, ctx, traits, aliases, depth + 1);
            }
        }
    }
}

/// Recursive scan with depth tracking.  Walks every command at
/// the current level, then descends into the braced body
/// arguments each command declares via the registry's
/// ``ArgRole::Body`` role assignments.
///
/// `$var` / `[cmd]` body args are skipped — they aren't
/// braced bodies, and any `Eval` trait they carry is recorded
/// at the top level by the same call-site's role scan.
//
fn scan_deep<'p>(
    source: &str,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
    depth: u8,
) {
    if depth > MAX_DEPTH {
        return;
    }

    scan_commands(source, ctx, traits, aliases);

    // The recursion only walks braced bodies, so we re-segment
    // here rather than threading the segmented commands through
    // `scan_commands`.  The segmented slices have a lifetime
    // tied to this stack frame; each recursion needs its own.
    let segments = segment_commands_with_offset_and_config(source, 0, ctx.config);
    for seg in segments {
        if seg.texts.is_empty() {
            continue;
        }
        let cmd_name = ctx.head(&seg.texts[0]).resolved;
        let cmd_args: Vec<&str> = seg.texts[1..].iter().map(String::as_str).collect();
        // Look up body args from both the registry (for built-in
        // commands) and the stub overlay (for user-declared
        // `# tcl-lsp: stub` commands).  Union so a stub-defined
        // body arg recurses just like a registry-defined one.
        let mut body_indices: HashSet<usize> = ctx
            .registry
            .arg_indices_for_role(cmd_name, &cmd_args, ArgRole::Body)
            .into_iter()
            .collect();
        if let Some(overlay) = ctx.stub_overlay {
            body_indices.extend(overlay.arg_indices_for_role(cmd_name, &cmd_args, ArgRole::Body));
        }
        for idx in body_indices {
            let Some(body_text) = cmd_args.get(idx) else {
                continue;
            };
            if body_text.trim().is_empty() {
                continue;
            }
            // Skip non-braced bodies — `$var` / `[cmd]` heads
            // are already handled at the top-level role scan
            // (their `Eval` trait is recorded by
            // `apply_arg_role_traits` /
            // `apply_eval_traits`).  Peek at the first two bytes
            // for cheap detection.
            let head = body_text.as_bytes();
            if head.first().is_some_and(|&b| b == b'$' || b == b'[') {
                continue;
            }
            if head.len() >= 2 && (head[1] == b'$' || head[1] == b'[') {
                continue;
            }
            scan_deep(body_text, ctx, traits, aliases, depth + 1);
        }

        // `apply {argList body ?ns?} …` — an apply body runs in a *fresh*
        // call frame with its own parameters; recursing into it with the
        // enclosing proc's own `ctx`/`traits` (as a plain `Body` arg would)
        // conflates the two frames. A lambda-local variable that happens to
        // share a name with an enclosing param (`proc f {body} { apply {x
        // {eval $body}} 1 }`) would wrongly mark `f`'s `body` param as
        // evaluated, while the real forwarding case (`apply {x {eval $x}}
        // $body`) would be missed entirely (codex review of #954's
        // follow-up). Instead: infer the lambda's own traits in complete
        // isolation — as if it were its own tiny proc — then propagate a
        // lambda param's trait back onto an enclosing param only when the
        // corresponding actual argument is a bare, unadorned reference to
        // that enclosing param, i.e. only when the value genuinely flows
        // from the caller's frame into the lambda's.
        for idx in ctx
            .registry
            .arg_indices_for_role(cmd_name, &cmd_args, ArgRole::LambdaLiteral)
        {
            let Some(&tok) = seg.argv.get(idx + 1) else {
                continue;
            };
            if tok.kind != tcl_lexer::TokenType::Str {
                continue;
            }
            let Some(elems) = crate::lambda_literal::split_lambda_literal(source, tok) else {
                continue;
            };
            let Some(body_span) = elems.body else {
                continue;
            };
            let Some(body_text) = source.get(body_span.start() as usize..body_span.end() as usize)
            else {
                continue;
            };
            if body_text.trim().is_empty() {
                continue;
            }
            let Some(params_text) =
                source.get(elems.params.start() as usize..elems.params.end() as usize)
            else {
                continue;
            };
            let lambda_param_defs = crate::signature_scan::params::parse_param_list(params_text);
            let lambda_param_names: Vec<&str> =
                lambda_param_defs.iter().map(|p| p.name.as_str()).collect();
            if lambda_param_names.is_empty() {
                continue;
            }
            let lambda_traits = infer_param_traits_deep_at_depth(
                &lambda_param_names,
                body_text,
                TraitScanEnv {
                    registry: ctx.registry,
                    stub_overlay: ctx.stub_overlay,
                    config: ctx.config,
                    identities: ctx.identities,
                },
                depth + 1,
            );
            if lambda_traits.is_empty() {
                continue;
            }
            // Positional actual arguments following the lambda literal bind
            // to `argList`'s names in order (`apply {argList body} a1 a2 …`).
            for (param_name, actual) in lambda_param_names.iter().copied().zip(&cmd_args[idx + 1..])
            {
                let Some(outer_name) = extract_var_name(actual) else {
                    continue;
                };
                let Some(outer_param) = ctx.param_set.get(outer_name).copied() else {
                    continue;
                };
                if let Some(lambda_param_traits) = lambda_traits.get(param_name) {
                    traits
                        .entry(outer_param)
                        .or_default()
                        .extend(lambda_param_traits.iter().copied());
                }
            }
        }
    }
}

/// Extract a bare variable name from ``$var`` or ``${var}``.
/// Returns `None` when the text isn't a simple variable
/// reference.
fn extract_var_name(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'$' {
        return None;
    }
    let (name_start, name_end) = if bytes[1] == b'{' {
        // ``${name}`` — find the closing ``}``.
        let close = text[2..].find('}')?;
        (2, 2 + close)
    } else {
        (1, bytes.len())
    };
    let name = &text[name_start..name_end];
    if name.is_empty() {
        return None;
    }
    // Verify identifier-like content (alphanumerics, underscore,
    // colons for namespace-qualified names).
    let mut iter = name.chars();
    let first = iter.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return None;
    }
    if !iter.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':') {
        return None;
    }
    // Reject anything past the closing ``}`` for the braced form.
    if bytes[1] == b'{' && name_end + 1 < bytes.len() {
        return None;
    }
    Some(name)
}

/// Resolve a command's per-arg roles via the registry, unioned
/// with any matching `# tcl-lsp: stub` overlay entry.  Picks the
/// `arg_role_resolver` callback first, then static
/// `arg_roles`, then sub-command-level roles.  When
/// `stub_overlay` is `Some`, user-declared stub commands
/// contribute their declared roles on top of the registry's;
/// a stub-defined role for a given arg index overrides the
/// registry's (the overlay is later-write-wins).
fn resolve_arg_roles(
    command: &str,
    args: &[String],
    registry: &CommandRegistry,
    stub_overlay: Option<&StubOverlay>,
) -> HashMap<u8, ArgRole> {
    let mut roles: HashMap<u8, ArgRole> = HashMap::new();
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    // `upvar`'s VarWrite slots are binding destinations, not ordinary
    // callee-local variable-name writes.  Treating a `$param` in one of those
    // slots as a parameter-flow write upgrades its ProcArgTrait spuriously
    // (the binding is established in another frame).  The frame-effect
    // descriptor remains the owner of the pair layout; param-trait inference
    // simply abstains from this binding role.
    let alias_binding = registry.frame_effect(command).is_some_and(|effect| {
        effect.layout == tcl_registry::frame_effect::FrameArgLayout::AliasPairs
    });
    for role in [
        ArgRole::Body,
        ArgRole::Expr,
        ArgRole::VarWrite,
        ArgRole::VarRead,
        ArgRole::CommandPrefix,
    ] {
        if alias_binding && role == ArgRole::VarWrite {
            continue;
        }
        for idx in registry.arg_indices_for_role(command, &arg_strs, role) {
            if let Ok(idx_u8) = u8::try_from(idx) {
                roles.insert(idx_u8, role);
            }
        }
        if let Some(overlay) = stub_overlay {
            for idx in overlay.arg_indices_for_role(command, &arg_strs, role) {
                if let Ok(idx_u8) = u8::try_from(idx) {
                    roles.insert(idx_u8, role);
                }
            }
        }
    }
    roles
}

fn scan_command<'p>(
    head: crate::head_identity::HeadWords<'_>,
    cmd_args: &[String],
    braced: &[bool],
    head_is_var: bool,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
) {
    let param_set = ctx.param_set;
    // The *written* spelling is what a `$param` head test reads (a `$cmd` head
    // is a substitution, not a command binding); every registry query and
    // structural handler below reads the resolved one (issue #1275).
    let cmd_name = head.written;
    let resolved = head.resolved;
    // A `$param` command *head* (`$cmd arg1 arg2`) means the param's value names
    // a command.  Only a genuine `$`-substitution head counts — a braced
    // `{$cmd}` is a literal command name.  Resolves value-copies, so
    // `set c $cmd; $c …` also marks `cmd`.
    if head_is_var
        && let Some(vn) = extract_var_name(cmd_name)
        && let Some(p) = lookup_param(vn, param_set, aliases)
        && let Some(set) = traits.get_mut(p)
    {
        set.insert(ProcArgTrait::Command);
    }
    apply_arg_role_traits(resolved, cmd_args, braced, ctx, traits, aliases);
    apply_eval_traits(resolved, cmd_args, param_set, traits);

    // Per-command structural handlers.  Matched on the *resolved* head, so a
    // proven alias or rename of `upvar` / `foreach` / `after` is handled like
    // the command it is, and a spelling whose binding was provably taken over
    // matches nothing.
    match resolved {
        "upvar" => {
            handle_upvar(cmd_args, ctx, traits, aliases);
        }
        "namespace" if cmd_args.first().map(String::as_str) == Some("upvar") => {
            handle_namespace_upvar(cmd_args, param_set, traits, aliases);
        }
        "foreach" | "lmap" => handle_foreach(cmd_args, param_set, traits),
        "while" => handle_while(cmd_args, param_set, traits),
        "for" => handle_for(cmd_args, param_set, traits),
        "after" => handle_after(cmd_args, param_set, traits),
        "scan" => handle_variadic_var_write(cmd_args, param_set, traits, 2),
        "lassign" => handle_variadic_var_write(cmd_args, param_set, traits, 1),
        _ => {}
    }

    // (Variable-writing commands where a param is used directly as
    // the var name — `set`/`incr`/`append`/`lappend`/`global`/
    // `variable` etc. — are already covered by
    // `apply_arg_role_traits` above, which marks `ProcArgTrait::VarWrite`
    // for any arg whose registry `ArgRole` is `VarWrite`.  The old
    // hardcoded `var_write_index` name list was a redundant duplicate
    // of that registry query and has been removed.  `regexp` capture
    // vars and `regsub`'s output var are covered the same way: their
    // specs' `arg_role_resolver` performs the spec-declared switch skip
    // (`-start` consumes a value, `--` terminates) and resolves the
    // trailing vars as `VarWrite`, so the old hardcoded
    // `REGEXP_SWITCHES` skip — which missed `-about` and the Tcl 9
    // `regsub -command` — has been removed too.)

    // Track writes through upvar aliases — ``set local …`` where
    // ``local`` was registered as an alias for some param.
    if matches!(resolved, "set" | "incr" | "append" | "lappend")
        && !cmd_args.is_empty()
        && let Some(target) = aliases.upvar.get(cmd_args[0].as_str())
        && let Some(set) = traits.get_mut(target)
    {
        set.insert(ProcArgTrait::VarWrite);
    }

    // foreach / lmap loop variables write through aliases.
    if matches!(resolved, "foreach" | "lmap") && cmd_args.len() >= 3 {
        let remaining = &cmd_args[..cmd_args.len() - 1];
        let mut i = 0;
        while i < remaining.len() {
            if let Some(target) = aliases.upvar.get(remaining[i].as_str())
                && let Some(set) = traits.get_mut(target)
            {
                set.insert(ProcArgTrait::VarWrite);
            }
            i += 2;
        }
    }

    // Value-copy tracking: `set n $p` makes local `n` carry param `p`'s value,
    // so a later `$n` in a name / command position resolves to `p`.  Any other
    // write to a tracked local invalidates the copy.  Recorded *after* the role
    // scan so this command's effect applies only to later commands.
    match resolved {
        "set" if cmd_args.len() == 2 => {
            let target = cmd_args[0].as_str();
            if extract_var_name(&cmd_args[0]).is_none()
                && is_plain_local_name(target)
                && !param_set.contains(target)
            {
                match extract_var_name(&cmd_args[1])
                    .and_then(|vn| lookup_param(vn, param_set, aliases))
                {
                    Some(p) => {
                        aliases.value_copies.insert(target.to_owned(), p);
                    }
                    None => {
                        aliases.value_copies.remove(target);
                    }
                }
            }
        }
        "incr" | "append" | "lappend"
            if cmd_args.first().is_some_and(|t| is_plain_local_name(t)) =>
        {
            aliases.value_copies.remove(cmd_args[0].as_str());
        }
        _ => {}
    }
}

/// Per-arg role-driven trait recording — apply
/// ``ArgRole::Body`` / ``Expr`` / ``VarWrite`` / ``VarRead`` to
/// the matching parameter trait set when an arg is a simple
/// ``$param`` reference (or aliases an upvar'd one).
///
/// `braced[i]` is `true` when argument `i` was a braced (`{…}`) literal word,
/// in which case **no substitution occurs**: `set {arr($k)} 1` names the literal
/// variable `arr($k)`, so the `$k` inside must not be read as a substitution
/// (issue #814 review).  The braced guard applies only to the variable-name
/// roles; a braced `Body` / `Expr` word still substitutes internally when Tcl
/// evaluates it, and the deep pass recurses into it.
fn apply_arg_role_traits<'p>(
    cmd_name: &str,
    cmd_args: &[String],
    braced: &[bool],
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &Aliases<'p>,
) {
    let param_set = ctx.param_set;
    let arg_roles = resolve_arg_roles(cmd_name, cmd_args, ctx.registry, ctx.stub_overlay);
    for (idx, arg) in cmd_args.iter().enumerate() {
        let Ok(idx_u8) = u8::try_from(idx) else {
            continue;
        };
        match arg_roles.get(&idx_u8) {
            // Body / Expr apply only to a bare `$param` (or upvar-aliased) word:
            // the whole argument *is* the param's value, evaluated as a script /
            // expression.
            Some(ArgRole::Body) => {
                if let Some(p) = resolve_param(arg, param_set, aliases)
                    && let Some(set) = traits.get_mut(p)
                {
                    set.insert(ProcArgTrait::Body);
                }
            }
            Some(ArgRole::Expr) => {
                if let Some(p) = resolve_param(arg, param_set, aliases)
                    && let Some(set) = traits.get_mut(p)
                {
                    set.insert(ProcArgTrait::Expr);
                }
            }
            // A registry / stub `VarWrite` / `VarRead` role names a variable.
            // Every param whose value flows into that name — the whole `$p`, or
            // a `$p` component of a compound dynamic name (`${p}($k)`,
            // `$p($k)`) — names a CALLEE-LOCAL variable (`set $p 1`,
            // `set ${v}($k)`, `incr $p`), NOT a caller-frame alias.  Record the
            // callee-local refinement (never `VarWrite`; only an `upvar`-aliased
            // write-back is a genuine caller write).  A braced word is a literal
            // name — no substitution — so no param flows into it.
            Some(ArgRole::VarWrite | ArgRole::VarRead)
                if !braced.get(idx).copied().unwrap_or(false) =>
            {
                for var_name in var_substitutions(arg) {
                    if let Some(p) = lookup_param(var_name, param_set, aliases)
                        && let Some(set) = traits.get_mut(p)
                    {
                        mark_dynamic_name_local(set);
                    }
                }
            }
            // A registry / stub `CommandPrefix` role names a command — a callback
            // command prefix (`tcltest::customMatch`, `selection handle`, a stub
            // `:command_prefix` arg).  A `$param` (or component of a dynamic
            // name) flowing there means the param's value is a command name.
            // Braced words are literal — no substitution.
            Some(ArgRole::CommandPrefix) if !braced.get(idx).copied().unwrap_or(false) => {
                for var_name in var_substitutions(arg) {
                    if let Some(p) = lookup_param(var_name, param_set, aliases)
                        && let Some(set) = traits.get_mut(p)
                    {
                        set.insert(ProcArgTrait::Command);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Resolve a bare `$param` / `${param}` argument to the param it references,
/// following an `upvar` alias.  `None` when the argument is not a simple
/// variable reference or names no known param.
fn resolve_param<'p>(
    arg: &str,
    param_set: &HashSet<&'p str>,
    aliases: &Aliases<'p>,
) -> Option<&'p str> {
    lookup_param(extract_var_name(arg)?, param_set, aliases)
}

/// Map a bare variable name to the param it references — directly, via an
/// `upvar` alias, or via a local that carries a param's value (`set n $p`).
/// `None` when it names no known param.
fn lookup_param<'p>(
    var_name: &str,
    param_set: &HashSet<&'p str>,
    aliases: &Aliases<'p>,
) -> Option<&'p str> {
    param_set
        .get(var_name)
        .copied()
        .or_else(|| aliases.upvar.get(var_name).copied())
        .or_else(|| aliases.value_copies.get(var_name).copied())
}

/// Yield each simple variable name substituted in `text` — `$name`, `${name}`,
/// or the array *name* of an element reference — in source order.  Used to find
/// the param(s) whose value flows into a (possibly compound) dynamic variable
/// name such as `${v}($k)` (both `v` and `k`), `arr($k)` (just `k`), or the
/// segmenter's normalised `${gas(idx)}` form (just `gas`).  Best-effort and
/// highlighting-grade: a leading `\$` is skipped, and a name must start with a
/// letter or `_` (so `$1` backrefs and `$` alone are ignored).
fn var_substitutions(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'$' || (i > 0 && bytes[i - 1] == b'\\') {
            i += 1;
            continue;
        }
        if bytes.get(i + 1) == Some(&b'{') {
            // `${…}` — the braced content is the variable name, but the
            // segmenter normalises an element ref to `${gas(idx)}`, so take only
            // the leading identifier as the name.  Advance past `${` (not past
            // the closer) so any `$k` *inside* the braces (`${arr($k)}`) or
            // after them (`${v}($k)`) is still scanned.
            if let Some(rel) = text[i + 2..].find('}') {
                push_leading_ident(&text[i + 2..i + 2 + rel], &mut out);
            }
            i += 2;
        } else {
            let mut j = i + 1;
            while j < bytes.len()
                && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b':')
            {
                j += 1;
            }
            push_leading_ident(&text[i + 1..j], &mut out);
            i = j.max(i + 1);
        }
    }
    out
}

/// Push the leading identifier of `content` (up to the first non-`[A-Za-z0-9_:]`
/// character) onto `out`, when it starts with a letter or `_`.  Used to peel the
/// array *name* off an element reference (`gas(idx)` → `gas`).
fn push_leading_ident<'a>(content: &'a str, out: &mut Vec<&'a str>) {
    let end = content
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':'))
        .unwrap_or(content.len());
    let name = &content[..end];
    if name
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
    {
        out.push(name);
    }
}

/// Record the callee-local dynamic-name use of a param — its **value**
/// is used as a variable *name* in the proc's own scope (``set $p 1``,
/// ``scan $s %d $p``, ``lassign $l $p``, ``regsub … $p``).  Emits
/// [`ProcArgTrait::DynamicNameLocal`] (the refinement that stops
/// caller-side dead-store / unused-variable suppression from treating
/// the call site as consuming the caller's variable) plus
/// [`ProcArgTrait::VarRead`] (the param's string *is* read).  Never
/// `VarWrite` — only an ``upvar``-aliased write-back is a genuine
/// caller-frame write.
fn mark_dynamic_name_local(set: &mut HashSet<ProcArgTrait>) {
    set.insert(ProcArgTrait::DynamicNameLocal);
    set.insert(ProcArgTrait::VarRead);
}

/// Code-evaluating commands — ``eval`` / ``subst`` mark every
/// ``$param`` arg as ``Eval``; ``uplevel ?level? script`` marks
/// only the last arg.
fn apply_eval_traits<'a>(
    cmd_name: &str,
    cmd_args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    let mark_as_eval = |vn: &str, traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>| {
        if let Some(p) = param_set.get(vn)
            && let Some(set) = traits.get_mut(p)
        {
            set.insert(ProcArgTrait::Eval);
        }
    };
    match cmd_name {
        "eval" | "subst" => {
            for arg in cmd_args {
                if let Some(vn) = extract_var_name(arg) {
                    mark_as_eval(vn, traits);
                }
            }
        }
        "uplevel" => {
            // ``uplevel ?level? script`` — last arg is the script.
            if let Some(last) = cmd_args.last()
                && let Some(vn) = extract_var_name(last)
            {
                mark_as_eval(vn, traits);
            }
        }
        _ => {}
    }
}

/// Split an `upvar` argument list into the frame its level word selects and
/// the `otherVar myVar …` pairs that follow.
///
/// C Tcl decides whether the level word is present from the **argument count
/// parity** (`Tcl_UpvarObjCmd` tests `objc`), not from the word's text.
/// Sniffing the text instead dropped the commonest by-reference idiom of all:
/// `upvar $lvl a b` has three words, so `$lvl` *is* the level and `(a, b)` is
/// the pair, but a digits-or-`#` test sees no level and pairs `($lvl, a)` —
/// losing the `a`/`b` binding entirely.  tclsh 9.0.4 / 8.6.14 agree; the rule
/// itself lives in the registry as
/// [`tcl_registry::frame_effect::FrameLevelWord::ArityParity`], queried here
/// through the spec rather than re-derived, so this stays the one description
/// of `upvar`'s shape (issue #1069).
///
/// A level word whose value is not a frame at all (C Tcl's `bad level "…"`)
/// answers [`FrameLevel::Dynamic`] — unplaceable, which is the abstaining
/// direction every consumer here wants.
fn upvar_level_and_pairs<'a>(
    args: &'a [String],
    registry: &CommandRegistry,
) -> (tcl_registry::frame_effect::FrameLevel, &'a [String]) {
    use tcl_registry::frame_effect::FrameLevel;
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let taken = registry.frame_effect("upvar").map_or_else(
        || usize::from(args.len() % 2 == 1),
        |s| s.level_word_len(&refs),
    );
    let level = if taken == 0 {
        FrameLevel::DEFAULT
    } else {
        FrameLevel::parse_in(&args[0], registry).unwrap_or(FrameLevel::Dynamic)
    };
    (level, args.get(taken..).unwrap_or(&[]))
}

fn handle_upvar<'p>(
    args: &[String],
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
) {
    let (_, pairs) = upvar_level_and_pairs(args, ctx.registry);
    record_upvar_pairs(pairs, ctx.param_set, traits, aliases);
}

/// The parameters whose **value names a variable in the immediate caller's
/// frame** — `upvar 1 $param local` and its default-level spelling `upvar
/// $param local`, and nothing else.
///
/// This is the fact call-site navigation needs, and it is strictly narrower
/// than [`ProcArgTrait::VarWrite`] / [`ProcArgTrait::VarRead`], which record
/// only *that* a parameter's value is used as a variable name through an
/// `upvar`, never *which frame* the alias lands in.  Every other level names a
/// different frame, pinned identical on tclsh 9.0.4 and 8.6.14:
///
/// | callee body | `p x` from a proc writes … |
/// |---|---|
/// | `upvar 1 $n a; set a 1` | the caller's `x` |
/// | `upvar 0 $n a; set a 1` | the **callee's own** `x` — caller's `x` never exists |
/// | `upvar #0 $n a; set a 1` | the **global** `::x` |
/// | `upvar 2 $n a; set a 1` | the caller's **caller** — the immediate caller is skipped |
///
/// `namespace upvar ns $token local` is deliberately absent too: it aliases a
/// *namespace* variable, so a call site passing `token` creates nothing in the
/// calling frame either.
///
/// Only the top-level command scan is walked, matching the shallow trait pass
/// exactly — a consumer that requires both facts is then never widened by a
/// disagreement between them, only narrowed.
#[must_use]
pub fn caller_frame_upvar_params(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
) -> HashSet<String> {
    let registry = env.registry;
    let mut out = HashSet::new();
    if params.is_empty() || body_source.trim().is_empty() {
        return out;
    }
    let param_set: HashSet<&str> = params.iter().copied().collect();
    for seg in &segment_commands_with_offset_and_config(body_source, 0, env.config) {
        let Some(head) = seg.texts.first() else {
            continue;
        };
        let resolved = env.identities.resolve_unpositioned(head).spec_name();
        if registry.frame_effect(resolved).is_none_or(|spec| {
            spec.layout != tcl_registry::frame_effect::FrameArgLayout::AliasPairs
        }) {
            continue;
        }
        let (level, pairs) = upvar_level_and_pairs(&seg.texts[1..], registry);
        if !level.is_caller_frame() {
            continue;
        }
        let mut i = 0;
        while i + 1 < pairs.len() {
            if let Some(vn) = extract_var_name(&pairs[i])
                && let Some(p) = param_set.get(vn)
            {
                out.insert((*p).to_string());
            }
            i += 2;
        }
    }
    out
}

/// The **literal caller-frame targets** a proc body binds through `upvar` —
/// caller-frame names the callee spells in its *own* text, with no call-site
/// argument word to key on (`upvar 1 name name`, issue #923 audit idx 22 /
/// issue #1139).  Returns `caller_name → written-through-alias`.
///
/// Strictly the complement of [`caller_frame_upvar_params`]: that fact keys
/// on a `$param` **source** (the name arrives from the call site); this one
/// keys on a *literal* source (the name is fixed by the callee).  tclsh
/// 9.0.4 / 8.6.14, identical: with `proc np {} {upvar name name; set name
/// W1}`, a caller running `np; puts $name` prints `W1` — `name` exists in
/// the caller's frame although the caller never assigns it and no call-site
/// word spells it.
///
/// Exclusions, each the abstaining direction:
///
/// * a non-caller frame level (`upvar 0` / `#0` / `2` / `$lvl`) — a
///   different frame entirely, see [`caller_frame_upvar_params`]'s table;
/// * a `::`-qualified source (`upvar 1 ::ns::x local`) — a fixed global/
///   namespace cell, level-independent, already linked by the analyser's
///   `handle_upvar_command` `otherVar` link (issue #923 idx 98);
/// * an array element or any substituted/computed source — not a plain
///   caller-frame scalar name this scan can claim;
/// * a dynamic **local** side — with no alias name, the write-through scan
///   below has nothing to match, so the pair contributes nothing here (the
///   compiler-side summary still widens the call site, issue #1165).
///
/// The write-through upgrade mirrors [`record_upvar_pairs`]'s rule for
/// params: a later registry-declared `VarWrite` on the alias local (`set
/// name …`, `lassign … name`) marks the caller name as *created* by a call;
/// an alias only ever read leaves it `false` (a referencing call site, not a
/// creating one).  Only the top-level command scan is walked, matching the
/// shallow trait pass exactly.
#[must_use]
pub fn caller_frame_literal_targets(
    body_source: &str,
    env: TraitScanEnv<'_>,
) -> HashMap<String, bool> {
    use tcl_registry::frame_effect::FrameArgLayout;

    let registry = env.registry;
    let mut targets: HashMap<String, bool> = HashMap::new();
    if body_source.trim().is_empty() {
        return targets;
    }
    // local alias name → the caller-frame name it stands for.
    let mut alias_to_target: HashMap<String, String> = HashMap::new();
    let segs = segment_commands_with_offset_and_config(body_source, 0, env.config);
    for seg in &segs {
        let Some(written) = seg.texts.first() else {
            continue;
        };
        let head = env.identities.resolve_unpositioned(written).spec_name();
        if registry
            .frame_effect(head)
            .is_none_or(|spec| spec.layout != FrameArgLayout::AliasPairs)
        {
            continue;
        }
        let (level, pairs) = upvar_level_and_pairs(&seg.texts[1..], registry);
        if !level.is_caller_frame() {
            continue;
        }
        let mut i = 0;
        while i + 1 < pairs.len() {
            let (src, dst) = (&pairs[i], &pairs[i + 1]);
            i += 2;
            if is_literal_caller_frame_name(src) && is_plain_local_name(dst) {
                targets.entry(src.clone()).or_insert(false);
                alias_to_target.insert(dst.clone(), src.clone());
            }
        }
    }
    if alias_to_target.is_empty() {
        return targets;
    }
    for seg in &segs {
        let Some(written) = seg.texts.first() else {
            continue;
        };
        let head = env.identities.resolve_unpositioned(written).spec_name();
        if registry.frame_effect(head).is_some_and(|effect| {
            effect.layout == tcl_registry::frame_effect::FrameArgLayout::AliasPairs
        }) {
            continue;
        }
        let args: Vec<&str> = seg.texts.iter().skip(1).map(String::as_str).collect();
        for idx in registry.arg_indices_for_role(head, &args, ArgRole::VarWrite) {
            if let Some(word) = args.get(idx)
                && let Some(caller_name) = alias_to_target.get(*word)
            {
                targets.insert(caller_name.clone(), true);
            }
        }
    }
    targets
}

/// A plain caller-frame scalar name: no substitution, no bracket, no
/// namespace qualifier, no array element.  Anything else is either a
/// different cell (`::`-qualified) or unknowable, and abstains.
fn is_literal_caller_frame_name(word: &str) -> bool {
    !word.is_empty()
        && !word.contains("::")
        && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `namespace upvar namespace ?otherVar myVar ...?` — the pairs alias namespace
/// variables named by `otherVar` (`$token`) into locals.  Structurally
/// identical to [`handle_upvar`] once the `upvar` sub-command word and the
/// `namespace` word are skipped.  `args` is the whole sub-command arg list:
/// `["upvar", namespace, otherVar, myVar, …]`, so the pairs start at index 2.
fn handle_namespace_upvar<'p>(
    args: &[String],
    param_set: &HashSet<&'p str>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
) {
    if args.len() > 2 {
        record_upvar_pairs(&args[2..], param_set, traits, aliases);
    }
}

/// Record the `otherVar myVar` pairs shared by `upvar` and `namespace upvar`:
/// each `otherVar` that is a `$param` reads the aliased caller variable
/// (`VarRead`) and registers `myVar` as an alias for that param, so a later
/// write *through the alias* (`set myVar …`) upgrades it to a genuine
/// caller-frame `VarWrite`.  A `$param` in the `myVar` slot is different — its
/// value names a *callee-local* alias variable, not a caller variable, so it is
/// a `DynamicNameLocal` use (never a caller-frame `VarWrite`).
fn record_upvar_pairs<'p>(
    pairs: &[String],
    param_set: &HashSet<&'p str>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
) {
    let mut i = 0;
    while i + 1 < pairs.len() {
        let other_var = &pairs[i];
        let my_var = &pairs[i + 1];
        i += 2;

        if let Some(other_vn) = extract_var_name(other_var)
            && let Some(p) = param_set.get(other_vn).copied()
        {
            if let Some(set) = traits.get_mut(p) {
                set.insert(ProcArgTrait::VarRead);
            }
            aliases.upvar.insert(my_var.clone(), p);
        }
        if let Some(my_vn) = extract_var_name(my_var)
            && let Some(p) = param_set.get(my_vn).copied()
            && let Some(set) = traits.get_mut(p)
        {
            mark_dynamic_name_local(set);
        }
    }
}

fn handle_foreach<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    if args.len() < 3 {
        return;
    }
    if let Some(body_vn) = extract_var_name(args.last().unwrap())
        && let Some(p) = param_set.get(body_vn).copied()
        && let Some(set) = traits.get_mut(p)
    {
        set.insert(ProcArgTrait::Body);
    }
    let remaining = &args[..args.len() - 1];
    let mut i = 0;
    while i + 1 < remaining.len() {
        if let Some(list_vn) = extract_var_name(&remaining[i + 1])
            && let Some(p) = param_set.get(list_vn).copied()
            && let Some(set) = traits.get_mut(p)
        {
            set.insert(ProcArgTrait::LoopList);
        }
        i += 2;
    }
}

fn handle_while<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    if args.len() < 2 {
        return;
    }
    if let Some(vn) = extract_var_name(&args[0])
        && let Some(p) = param_set.get(vn).copied()
        && let Some(set) = traits.get_mut(p)
    {
        set.insert(ProcArgTrait::Expr);
    }
    if let Some(vn) = extract_var_name(&args[1])
        && let Some(p) = param_set.get(vn).copied()
        && let Some(set) = traits.get_mut(p)
    {
        set.insert(ProcArgTrait::Body);
    }
}

fn handle_for<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    if args.len() < 4 {
        return;
    }
    let pairs = [
        (&args[0], ProcArgTrait::Body),
        (&args[1], ProcArgTrait::Expr),
        (&args[2], ProcArgTrait::Body),
        (&args[3], ProcArgTrait::Body),
    ];
    for (arg, trait_) in pairs {
        if let Some(vn) = extract_var_name(arg)
            && let Some(p) = param_set.get(vn).copied()
            && let Some(set) = traits.get_mut(p)
        {
            set.insert(trait_);
        }
    }
}

fn handle_after<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    if args.len() < 2 {
        return;
    }
    if matches!(args[0].as_str(), "cancel" | "info") {
        return;
    }
    let mut start = 1usize;
    if start < args.len() && args[start] == "-periodic" {
        start += 1;
    }
    for arg in &args[start..] {
        if let Some(vn) = extract_var_name(arg)
            && let Some(p) = param_set.get(vn).copied()
            && let Some(set) = traits.get_mut(p)
        {
            set.insert(ProcArgTrait::Eval);
        }
    }
}

/// Mark every ``$param`` from `start` onward as a callee-local
/// dynamic name.  Used for commands whose trailing args name
/// CALLEE-LOCAL output variables — ``scan`` (start 2) and ``lassign``
/// (start 1); ``regexp`` / ``regsub`` output vars take the
/// registry-role path in `apply_arg_role_traits` instead.  These
/// writes land in the
/// callee's own frame; they do **not** consume / alias the caller's
/// variable unless an explicit ``upvar`` set one up (handled
/// separately via the upvar-alias path, which emits a genuine
/// `VarWrite`).  Emitting [`ProcArgTrait::DynamicNameLocal`] (+
/// `VarRead`) rather than `VarWrite` keeps caller-side dead-store /
/// unused-variable suppression from silencing the caller's literal
/// arg (PR #498 / #499 finding 6).
fn handle_variadic_var_write<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
    start: usize,
) {
    for arg in &args[start.min(args.len())..] {
        if let Some(vn) = extract_var_name(arg)
            && let Some(p) = param_set.get(vn).copied()
            && let Some(set) = traits.get_mut(p)
        {
            mark_dynamic_name_local(set);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_trait(
        traits: &HashMap<String, HashSet<ProcArgTrait>>,
        param: &str,
        expected: ProcArgTrait,
    ) {
        let set = traits
            .get(param)
            .unwrap_or_else(|| panic!("no traits for {param}"));
        assert!(
            set.contains(&expected),
            "{param}: expected {expected:?}, got {set:?}",
        );
    }

    /// Test helper — builds the registry once, since the public
    /// API now requires the caller to thread one through.  No
    /// stub overlay; tests that need one construct it inline.
    fn infer(params: &[&str], body: &str) -> HashMap<String, HashSet<ProcArgTrait>> {
        let registry = CommandRegistry::build_default();
        infer_param_traits(params, body, env_for(&registry, LexerConfig::default()))
    }

    /// A scan environment over `registry` with no stub overlay and no document
    /// binding facts — what a bare-body unit test scans under.
    fn env_for(registry: &CommandRegistry, config: LexerConfig) -> TraitScanEnv<'_> {
        TraitScanEnv {
            registry,
            stub_overlay: None,
            config,
            identities: crate::head_identity::HeadIdentityMap::none(),
        }
    }

    /// [`env_for`] with a stub overlay attached.
    fn env_with_overlay<'a>(
        registry: &'a CommandRegistry,
        overlay: &'a StubOverlay,
    ) -> TraitScanEnv<'a> {
        TraitScanEnv {
            stub_overlay: Some(overlay),
            ..env_for(registry, LexerConfig::default())
        }
    }

    #[test]
    fn extract_var_name_simple() {
        assert_eq!(extract_var_name("$foo"), Some("foo"));
        assert_eq!(extract_var_name("${foo}"), Some("foo"));
        assert_eq!(extract_var_name("foo"), None);
        assert_eq!(extract_var_name("$"), None);
        assert_eq!(extract_var_name("$1abc"), None);
    }

    #[test]
    fn eval_param_records_eval_trait() {
        let traits = infer(&["body"], "eval $body");
        assert_trait(&traits, "body", ProcArgTrait::Eval);
    }

    /// Issue #954's param-trait sibling gap: the *deep* pass
    /// (`infer_param_traits_deep`, which alone recurses into braced body
    /// arguments) must reach real commands *inside* an `apply` lambda body,
    /// not misread the whole `{argList} {body}` blob as one script (which
    /// would treat the parameter word as a command name and never find the
    /// `eval` at all) — mirrors `overlay_deep_recurses_through_stub_body_args`,
    /// swapping the registry-known `apply`/`LambdaLiteral` shape in for a
    /// stub-declared `Body` shape. The lambda's own param is `x`, bound to
    /// the literal `1` — an enclosing `body` param is neither the lambda's
    /// param nor forwarded into the call, so it must record nothing (codex
    /// review of #954's follow-up: a lambda body runs in a fresh frame, and
    /// a same-named enclosing param is not implicitly in scope there).
    #[test]
    fn eval_inside_apply_lambda_body_does_not_leak_to_unrelated_enclosing_param() {
        let registry = CommandRegistry::build_default();
        let traits = infer_param_traits_deep(
            &["body"],
            "apply {x {eval $body}} 1",
            env_for(&registry, LexerConfig::default()),
        );
        assert!(
            !traits
                .get("body")
                .is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
            "an apply lambda's fresh frame must not leak a same-named \
             enclosing param's trait when that param is never forwarded \
             into the call; got {traits:?}"
        );
    }

    /// The real forwarding case: the lambda's own param `x` is `eval`'d
    /// inside its body, and the enclosing `body` param is passed as the
    /// actual argument that binds to `x` — so `body`'s value genuinely does
    /// flow into an `eval`, and the trait must propagate back to it.
    #[test]
    fn eval_param_forwarded_into_apply_lambda_records_eval_trait() {
        let registry = CommandRegistry::build_default();
        let traits = infer_param_traits_deep(
            &["body"],
            "apply {x {eval $x}} $body",
            env_for(&registry, LexerConfig::default()),
        );
        assert_trait(&traits, "body", ProcArgTrait::Eval);
    }

    #[test]
    fn trait_inference_is_dialect_aware_via_expand_syntax() {
        // `extract_commands` / `scan_deep` re-segment under the document
        // dialect, so the body's `{*}` is the expansion operator on 8.5+
        // but a literal brace word on 8.4.  `eval {*}$p` is therefore
        // `eval $p` (→ `p` is Eval) on 9.0 but `eval` of the single
        // composite word `{*}$p` (no clean `$p`, no trait) on 8.4.
        let registry = CommandRegistry::build_default();
        let on = infer_param_traits(
            &["p"],
            "eval {*}$p",
            env_for(&registry, LexerConfig::default()),
        );
        assert!(
            on.get("p").is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
            "9.0 expands `{{*}}` → eval $p → Eval: {on:?}",
        );
        let off = infer_param_traits(
            &["p"],
            "eval {*}$p",
            env_for(&registry, LexerConfig::for_dialect("tcl8.4")),
        );
        assert!(
            off.get("p")
                .is_none_or(|s| !s.contains(&ProcArgTrait::Eval)),
            "8.4 treats `{{*}}` as literal → no clean $p → no Eval: {off:?}",
        );
    }

    #[test]
    fn uplevel_records_eval_on_last_arg_only() {
        let traits = infer(&["lvl", "script"], "uplevel $lvl $script");
        assert_trait(&traits, "script", ProcArgTrait::Eval);
        assert!(
            !traits
                .get("lvl")
                .is_some_and(|s| s.contains(&ProcArgTrait::Eval))
        );
    }

    #[test]
    fn foreach_records_loop_list_and_body() {
        let traits = infer(&["items", "body"], "foreach x $items $body");
        assert_trait(&traits, "items", ProcArgTrait::LoopList);
        assert_trait(&traits, "body", ProcArgTrait::Body);
    }

    #[test]
    fn while_records_expr_and_body() {
        let traits = infer(&["cond", "body"], "while $cond $body");
        assert_trait(&traits, "cond", ProcArgTrait::Expr);
        assert_trait(&traits, "body", ProcArgTrait::Body);
    }

    #[test]
    fn for_records_init_cond_next_body() {
        let traits = infer(&["i", "c", "n", "b"], "for $i $c $n $b");
        assert_trait(&traits, "i", ProcArgTrait::Body);
        assert_trait(&traits, "c", ProcArgTrait::Expr);
        assert_trait(&traits, "n", ProcArgTrait::Body);
        assert_trait(&traits, "b", ProcArgTrait::Body);
    }

    #[test]
    fn upvar_records_var_read_and_aliases_writes() {
        let traits = infer(&["var"], "upvar 1 $var local\nset local 1");
        assert_trait(&traits, "var", ProcArgTrait::VarRead);
        // Write through the alias upgrades to VarWrite.
        assert_trait(&traits, "var", ProcArgTrait::VarWrite);
    }

    /// The frame level the trait map cannot record.  Pinned against tclsh
    /// 9.0.4 and 8.6.14 (byte-identical): with `proc q {n} {upvar L $n a; set
    /// a 1}` called as `q y` from a proc, the caller's `y` exists afterwards
    /// only for `L` = `1` (or an omitted level).  `0` aliases the callee's
    /// own frame, `#0` the global one, `2` the caller's caller.
    #[test]
    fn caller_frame_upvar_params_accepts_only_the_caller_frame_level() {
        let registry = CommandRegistry::build_default();
        let params = ["n"];
        for (body, expected) in [
            ("upvar 1 $n a; set a 1", true),
            ("upvar $n a; set a 1", true),
            ("upvar +1 $n a", true),
            ("upvar 0x1 $n a", true),
            ("upvar 0 $n a; set a 1", false),
            ("upvar #0 $n a; set a 1", false),
            ("upvar 2 $n a; set a 1", false),
            ("upvar -1 $n a", false),
            ("upvar $lvl $n a", false),
            ("upvar bogus $n a", false),
            // `namespace upvar` aliases a namespace variable, never the
            // calling frame.
            ("namespace upvar ::cfg $n a", false),
            // The `myVar` slot names a callee-local alias, not a caller
            // variable.
            ("upvar 1 caller $n", false),
        ] {
            let got = caller_frame_upvar_params(
                &params,
                body,
                env_for(&registry, LexerConfig::default()),
            );
            assert_eq!(
                got.contains("n"),
                expected,
                "{body:?} -> {got:?} (expected caller-frame param: {expected})"
            );
        }
    }

    /// Issue #1139: the literal-target fact — `upvar 1 name name` binds the
    /// caller's `name`, written through the alias when the body assigns it.
    /// tclsh 9.0.4: `proc np {} {upvar name name; set name W1}` then
    /// `np; puts $name` prints `W1` in the caller.
    #[test]
    fn caller_frame_literal_targets_records_written_and_read_only() {
        let registry = CommandRegistry::build_default();
        let got = caller_frame_literal_targets(
            "upvar name name\nset name W1",
            env_for(&registry, LexerConfig::default()),
        );
        assert_eq!(got.get("name"), Some(&true), "written through the alias");
        // A body that only reads through the alias creates nothing.
        let got = caller_frame_literal_targets(
            "upvar name name\nreturn [string length $name]",
            env_for(&registry, LexerConfig::default()),
        );
        assert_eq!(got.get("name"), Some(&false), "read-only alias");
        // Distinct local alias spelling still records the CALLER name.
        let got = caller_frame_literal_targets(
            "upvar 1 other mine\nset mine 1",
            env_for(&registry, LexerConfig::default()),
        );
        assert_eq!(got.get("other"), Some(&true));
        assert!(!got.contains_key("mine"));
    }

    /// The exclusions: non-caller levels, `::`-qualified cells, array
    /// elements, substituted sources, dynamic locals, `namespace upvar`.
    #[test]
    fn caller_frame_literal_targets_excludes_other_frames_and_cells() {
        let registry = CommandRegistry::build_default();
        for body in [
            "upvar 0 name name; set name 1",
            "upvar #0 name name; set name 1",
            "upvar 2 name name; set name 1",
            "upvar $lvl name name; set name 1",
            "upvar 1 ::tk::FocusGrab($i) data; set data 1",
            "upvar 1 ::ns::cell local; set local 1",
            "upvar 1 arr(k) local; set local 1",
            "upvar 1 $src local; set local 1",
            "upvar 1 name $dst",
            "namespace upvar ::cfg name local; set local 1",
        ] {
            let got =
                caller_frame_literal_targets(body, env_for(&registry, LexerConfig::default()));
            assert!(got.is_empty(), "{body:?} must record nothing, got {got:?}");
        }
    }

    /// The fact is strictly narrower than the trait: `upvar 0 $n a; set a 1`
    /// still records `VarWrite` (the parameter's value *is* used as a
    /// variable name through an `upvar`) while contributing no caller-frame
    /// parameter — which is exactly why a call-site consumer must intersect
    /// the two.
    #[test]
    fn caller_frame_params_is_narrower_than_the_var_traits() {
        let registry = CommandRegistry::build_default();
        let body = "upvar 0 $n a; set a 1";
        let traits = infer(&["n"], body);
        assert_trait(&traits, "n", ProcArgTrait::VarWrite);
        assert!(
            caller_frame_upvar_params(&["n"], body, env_for(&registry, LexerConfig::default()))
                .is_empty(),
            "the level fact must reject what the trait alone accepts"
        );
    }

    /// `upvar $lvl a b` has three words, so parity puts `$lvl` in the level
    /// slot and `(a, b)` in the pair slot — the level is unplaceable, so no
    /// caller-frame parameter is claimed, but the *pair* handling (and hence
    /// the trait map) is unchanged by the refactor.
    #[test]
    fn dynamic_level_word_keeps_parity_pairing_but_claims_no_frame() {
        let registry = CommandRegistry::build_default();
        let traits = infer(&["lvl", "v"], "upvar $lvl $v local\nset local 1");
        assert_trait(&traits, "v", ProcArgTrait::VarWrite);
        assert!(
            caller_frame_upvar_params(
                &["lvl", "v"],
                "upvar $lvl $v local\nset local 1",
                env_for(&registry, LexerConfig::default())
            )
            .is_empty(),
            "a computed level names no statically-known frame"
        );
    }

    #[test]
    fn upvar_my_var_slot_param_is_callee_local_not_var_write() {
        // `upvar 1 caller $local` — the param in the `myVar` slot names the
        // *callee-local* alias, not a caller variable, so it is a
        // `DynamicNameLocal` use, never a caller-frame `VarWrite` (PR review).
        let traits = infer(&["local"], "upvar 1 caller $local");
        assert_trait(&traits, "local", ProcArgTrait::DynamicNameLocal);
        assert!(
            !traits
                .get("local")
                .unwrap()
                .contains(&ProcArgTrait::VarWrite),
            "myVar-slot param must not be a caller VarWrite; got {:?}",
            traits.get("local"),
        );
    }

    #[test]
    fn lassign_records_dynamic_name_local() {
        // `lassign {1 2} $a $b` names CALLEE-LOCAL output vars via the
        // params' values — DynamicNameLocal (+ VarRead), NOT VarWrite
        // (which would imply a caller-frame alias).
        let traits = infer(&["a", "b"], "lassign {1 2} $a $b");
        for p in ["a", "b"] {
            assert_trait(&traits, p, ProcArgTrait::DynamicNameLocal);
            assert_trait(&traits, p, ProcArgTrait::VarRead);
            assert!(
                !traits.get(p).unwrap().contains(&ProcArgTrait::VarWrite),
                "{p}: callee-local lassign target must not be VarWrite, got {:?}",
                traits.get(p),
            );
        }
    }

    #[test]
    fn command_substitution_is_scanned() {
        // A param used inside a `[...]` substitution is a first-order use — the
        // shallow scan must descend into it.  `return [set $v 1]` names a
        // callee-local variable via `$v` → DynamicNameLocal (+ VarRead).
        let traits = infer(&["v"], "return [set $v 1]");
        assert_trait(&traits, "v", ProcArgTrait::DynamicNameLocal);
        assert_trait(&traits, "v", ProcArgTrait::VarRead);
        // The read form (single-arg `set`) is scanned identically.
        let traits = infer(&["v"], "return [set $v]");
        assert_trait(&traits, "v", ProcArgTrait::DynamicNameLocal);
    }

    #[test]
    fn command_substitution_plain_value_is_not_a_var_name() {
        // `[string length $x]` reads `$x` as a value, not a variable name, so
        // no role is recorded — the descent must not over-mark ordinary reads.
        let traits = infer(&["x"], "return [string length $x]");
        assert!(
            traits.get("x").is_none_or(HashSet::is_empty),
            "plain value read must record no trait, got {:?}",
            traits.get("x"),
        );
    }

    #[test]
    fn braced_var_name_has_no_substitution() {
        // `set {arr($k)} 1` — the braces make `arr($k)` a *literal* variable
        // name, so `$k` is not a substitution and must not mark `k` (issue #814
        // review: the segmenter drops the braces, so the guard is by token kind).
        assert!(!infer(&["k"], "set {arr($k)} 1").contains_key("k"));
        assert!(!infer(&["p"], "set {$p} 1").contains_key("p"));
        // The unbraced form *does* substitute, so it still marks the component.
        assert_trait(
            &infer(&["k"], "set arr($k) 1"),
            "k",
            ProcArgTrait::DynamicNameLocal,
        );
    }

    #[test]
    fn compound_dynamic_array_name_marks_components() {
        // `set ${v}($k) 1` — both the array-name param `v` and the key param
        // `k` flow into a dynamic variable name, so both are DynamicNameLocal
        // (+ VarRead).  This is the shape behind `return [set ${v}($k)]`.
        for body in ["set ${v}($k) 1", "set $v($k) 1", "return [set ${v}($k)]"] {
            let traits = infer(&["v", "k"], body);
            for p in ["v", "k"] {
                assert_trait(&traits, p, ProcArgTrait::DynamicNameLocal);
                assert_trait(&traits, p, ProcArgTrait::VarRead);
            }
        }
        // A literal array name with a `$k` key marks only the key.
        let traits = infer(&["k"], "set arr($k) 1");
        assert_trait(&traits, "k", ProcArgTrait::DynamicNameLocal);
    }

    #[test]
    fn var_substitutions_finds_compound_components() {
        assert_eq!(var_substitutions("$v"), vec!["v"]);
        assert_eq!(var_substitutions("${v}($k)"), vec!["v", "k"]);
        assert_eq!(var_substitutions("$v($k)"), vec!["v", "k"]);
        assert_eq!(var_substitutions("arr($k)"), vec!["k"]);
        assert_eq!(var_substitutions("literal"), Vec::<&str>::new());
        assert_eq!(var_substitutions("\\$escaped"), Vec::<&str>::new());
        assert_eq!(var_substitutions("$1backref"), Vec::<&str>::new());
    }

    #[test]
    fn value_copy_carries_param_into_name_and_command_positions() {
        // `set n $p` copies the param's value into local `n`; a later `$n` in a
        // name / command position then resolves back to the param.
        assert_trait(
            &infer(&["v"], "set n $v\nset $n 1"),
            "v",
            ProcArgTrait::DynamicNameLocal,
        );
        assert_trait(
            &infer(&["cmd"], "set c $cmd\n$c arg"),
            "cmd",
            ProcArgTrait::Command,
        );
        // In a command substitution too.
        assert_trait(
            &infer(&["v"], "set n $v\nreturn [set $n]"),
            "v",
            ProcArgTrait::DynamicNameLocal,
        );
        // Transitive copies chain.
        assert_trait(
            &infer(&["cmd"], "set c $cmd\nset d $c\n$d arg"),
            "cmd",
            ProcArgTrait::Command,
        );
    }

    #[test]
    fn value_copy_is_invalidated_and_not_over_eager() {
        // Reassigning the local to a non-param drops the copy, so a later `$n`
        // no longer resolves to the original param.
        assert!(!infer(&["v"], "set n $v\nset n other\nset $n 1").contains_key("v"));
        // Merely passing the value through (never used as a name) is not a role.
        assert!(!infer(&["v"], "set n $v\nreturn $n").contains_key("v"));
    }

    #[test]
    fn param_used_as_command_head_is_command() {
        // `$cmd arg1 arg2` — the param's value is the command word, so it names
        // a command.  Works at the top level and inside a `[...]` substitution.
        assert_trait(&infer(&["cmd"], "$cmd a b"), "cmd", ProcArgTrait::Command);
        assert_trait(
            &infer(&["cmd"], "return [$cmd x]"),
            "cmd",
            ProcArgTrait::Command,
        );
        // A braced `{$cmd}` head is a *literal* command name — no substitution.
        assert!(!infer(&["cmd"], "{$cmd} arg").contains_key("cmd"));
        // A param merely read as a value is not a command.
        assert!(!infer(&["x"], "puts $x").contains_key("x"));
    }

    #[test]
    fn stub_command_prefix_role_is_command() {
        // A stub-declared `command_prefix` argument names a command, so a
        // `$param` there is inferred `Command`.
        let overlay = make_overlay(vec![stub_sig(
            "on_event",
            &[
                ("event", ArgRole::Value),
                ("handler", ArgRole::CommandPrefix),
            ],
        )]);
        let registry = CommandRegistry::build_default();
        let traits = infer_param_traits(
            &["h"],
            "on_event click $h",
            TraitScanEnv {
                registry: &registry,
                stub_overlay: Some(&overlay),
                config: LexerConfig::default(),
                identities: crate::head_identity::HeadIdentityMap::none(),
            },
        );
        assert_trait(&traits, "h", ProcArgTrait::Command);
    }

    #[test]
    fn namespace_upvar_dynamic_name_reads_other_var() {
        // `namespace upvar ns $token arr` — the token/state-array idiom, with a
        // dynamic `$token` naming the namespace variable aliased into `arr`.
        // The pairs start after the `namespace` word (index 2 of the
        // sub-command args), so `$token` is the *other* var → VarRead, and a
        // later write through the `arr` alias upgrades it to VarWrite.
        let traits = infer(&["token"], "namespace upvar ns $token arr");
        assert_trait(&traits, "token", ProcArgTrait::VarRead);
        let traits = infer(&["token"], "namespace upvar ns $token arr\nset arr 1");
        assert_trait(&traits, "token", ProcArgTrait::VarWrite);
    }

    #[test]
    fn real_world_by_reference_patterns_are_inferred() {
        // The dominant variable-name-by-reference shapes mined from the tcllib /
        // Tcl-library corpus (`http`, `smtp`, `ftp`, … token idiom).  Each names
        // a variable via a parameter, so each must record a read/write trait.
        let has_var = |body: &str, p: &str| {
            let t = infer(&[p], body);
            t.get(p).is_some_and(|s| {
                s.contains(&ProcArgTrait::VarRead) || s.contains(&ProcArgTrait::VarWrite)
            })
        };
        for (body, p) in [
            ("upvar 1 $varname data", "varname"),
            ("upvar 0 $token state", "token"),
            ("upvar #0 $token state", "token"),
            ("upvar $v local", "v"),
            ("variable $name", "name"),
            ("array set $a {x 1}", "a"),
            ("array get $a", "a"),
            ("array names $a", "a"),
            ("unset $v", "v"),
            ("dict set $d k 1", "d"),
            ("incr $counter", "counter"),
            ("append $resultVar x", "resultVar"),
            ("lappend $pages_var p", "pages_var"),
            ("set ${tok}(state) connected", "tok"),
            ("set $gas(idx) 1", "gas"),
            ("namespace upvar ns $v local", "v"),
            ("foreach k [array names $a] { puts $k }", "a"),
            ("return [set ${token}($field)]", "token"),
        ] {
            assert!(
                has_var(body, p),
                "expected a by-reference var trait for `{p}` in `{body}`; got {:?}",
                infer(&[p], body).get(p),
            );
        }
    }

    #[test]
    fn after_records_eval_skipping_cancel_info() {
        let traits = infer(&["body"], "after 100 $body");
        assert_trait(&traits, "body", ProcArgTrait::Eval);
        let traits = infer(&["x"], "after cancel $x");
        // ``after cancel`` doesn't take a script, so $x is not eval.
        assert!(
            traits
                .get("x")
                .is_none_or(|s| !s.contains(&ProcArgTrait::Eval))
        );
    }

    #[test]
    fn regsub_records_dynamic_name_local() {
        // `regsub`'s output var is CALLEE-LOCAL — DynamicNameLocal
        // (+ VarRead), not VarWrite.
        let traits = infer(&["out"], "regsub -all foo $line bar $out");
        assert_trait(&traits, "out", ProcArgTrait::DynamicNameLocal);
        assert_trait(&traits, "out", ProcArgTrait::VarRead);
        assert!(
            !traits.get("out").unwrap().contains(&ProcArgTrait::VarWrite),
            "callee-local regsub target must not be VarWrite, got {:?}",
            traits.get("out"),
        );
    }

    #[test]
    fn regexp_capture_vars_after_switches_record_dynamic_name_local() {
        // `regexp -nocase -- $pat $s $m` — the registry `regexp` spec's
        // arg-role resolver performs the switch skip (`--` terminator
        // included), so the capture var `$m` names a callee-local output
        // variable.  The pattern / subject args are plain value reads and
        // must record no trait.
        let traits = infer(&["pat", "s", "m"], "regexp -nocase -- $pat $s $m");
        assert_trait(&traits, "m", ProcArgTrait::DynamicNameLocal);
        assert_trait(&traits, "m", ProcArgTrait::VarRead);
        assert!(!traits.contains_key("pat"), "{traits:?}");
        assert!(!traits.contains_key("s"), "{traits:?}");
    }

    #[test]
    fn regexp_start_switch_value_is_skipped() {
        // `-start` consumes its value word (spec-declared), so the capture
        // var still resolves after the pattern + string positionals.
        let traits = infer(&["m"], "regexp -start 2 {x+} $s $m");
        assert_trait(&traits, "m", ProcArgTrait::DynamicNameLocal);
    }

    #[test]
    fn regsub_command_switch_is_skipped_to_output_var() {
        // Tcl 9 `regsub -command exp string cmdPrefix varName` — `-command`
        // is a spec-declared flag the old hardcoded switch list missed, so
        // the output var must still resolve at exp + 3.
        let traits = infer(&["out"], "regsub -command {x+} $s myCb $out");
        assert_trait(&traits, "out", ProcArgTrait::DynamicNameLocal);
        assert_trait(&traits, "out", ProcArgTrait::VarRead);
    }

    #[test]
    fn set_dynamic_name_records_dynamic_name_local_not_var_write() {
        // `set $p 1` — the param's VALUE names a callee-local variable
        // (registry `VarWrite` role on the `$p` substitution).  The
        // refined trait is DynamicNameLocal (+ VarRead), NOT VarWrite:
        // a caller passing a literal name (`f x`) does not have its `x`
        // consumed by this callee.  Pins the PR #498 finding-10 fix.
        let traits = infer(&["p"], "set $p 1");
        assert_trait(&traits, "p", ProcArgTrait::DynamicNameLocal);
        assert_trait(&traits, "p", ProcArgTrait::VarRead);
        assert!(
            !traits.get("p").unwrap().contains(&ProcArgTrait::VarWrite),
            "callee-local `set $p` must not be VarWrite, got {:?}",
            traits.get("p"),
        );
    }

    #[test]
    fn genuine_upvar_write_stays_var_write_not_dynamic_name_local() {
        // A real caller-frame write-back through an `upvar` alias is a
        // genuine `VarWrite` — the DynamicNameLocal refinement must NOT
        // leak onto it (otherwise caller-side suppression of a genuine
        // call-by-name write would regress).
        let traits = infer(&["var"], "upvar 1 $var local\nset local 1");
        assert_trait(&traits, "var", ProcArgTrait::VarWrite);
        assert!(
            !traits
                .get("var")
                .unwrap()
                .contains(&ProcArgTrait::DynamicNameLocal),
            "upvar write-back must not be DynamicNameLocal, got {:?}",
            traits.get("var"),
        );
    }

    #[test]
    fn empty_body_returns_empty_map() {
        let traits = infer(&["a"], "");
        assert!(traits.is_empty());
    }

    /// Deep-pass helper that mirrors [`infer`] for ergonomics.
    fn infer_deep(params: &[&str], body: &str) -> HashMap<String, HashSet<ProcArgTrait>> {
        let registry = CommandRegistry::build_default();
        infer_param_traits_deep(params, body, env_for(&registry, LexerConfig::default()))
    }

    #[test]
    fn deep_pass_surfaces_eval_trait_inside_braced_body() {
        // `$body` is used inside a nested `foreach` body — the
        // shallow pass walks only top-level commands, so it
        // misses the trait.  The deep pass descends into the
        // braced `foreach` body and surfaces `Eval`.
        let body = "foreach item $items {\n  uplevel 1 $body\n}";
        let shallow = infer(&["items", "body"], body);
        let deep = infer_deep(&["items", "body"], body);
        // Shallow catches `items` (LoopList) but misses `body`.
        assert_trait(&shallow, "items", ProcArgTrait::LoopList);
        assert!(
            !shallow
                .get("body")
                .is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
            "shallow pass should not surface nested Eval, got {shallow:?}",
        );
        // Deep catches both.
        assert_trait(&deep, "items", ProcArgTrait::LoopList);
        assert_trait(&deep, "body", ProcArgTrait::Eval);
    }

    #[test]
    fn deep_pass_descends_through_multiple_levels() {
        // `$inner` is buried two levels deep: `if` → `while` →
        // `eval $inner`.  Shallow misses it; deep finds it.
        let body = "if {1} {\n  while {1} {\n    eval $inner\n  }\n}";
        let deep = infer_deep(&["inner"], body);
        assert_trait(&deep, "inner", ProcArgTrait::Eval);
    }

    #[test]
    fn deep_pass_respects_max_depth() {
        // Build a body nested past `MAX_DEPTH` (8 levels of
        // `if {1} { ... }`) with `eval $deep_var` at the
        // innermost level.  The recursion should stop before
        // reaching the innermost level and the trait should not
        // be surfaced.  Using `MAX_DEPTH + 2` (10) levels of
        // nesting puts the eval below the recursion bound.
        let depth_to_nest = usize::from(MAX_DEPTH) + 2;
        let mut body = String::from("eval $deep_var");
        for _ in 0..depth_to_nest {
            body = format!("if {{1}} {{ {body} }}");
        }
        let deep = infer_deep(&["deep_var"], &body);
        assert!(
            !deep
                .get("deep_var")
                .is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
            "MAX_DEPTH bound should keep deeply-nested eval from being surfaced, got {deep:?}",
        );
    }

    /// Same-bug-class regression as issue #996's own fix, in the sibling
    /// walker the issue explicitly calls out (`param_traits.rs`): before
    /// this fix, `scan_deep`'s `apply` (`ArgRole::LambdaLiteral`) handling
    /// re-entered `infer_param_traits_deep_with_config` — the *public*,
    /// depth-0 entry point — instead of threading its own `depth + 1`
    /// through. Alternating `if {1} { apply {x {…}} … }` nesting therefore
    /// reset the *logical* [`MAX_DEPTH`] counter back to 0 on every `apply`
    /// boundary while the *native* Rust call stack (`scan_deep` ↔
    /// `infer_param_traits_deep_at_depth`) kept growing one frame group per
    /// level regardless of the reset — unboundedly, for however deep the
    /// input alternates. `MAX_DEPTH` (8) never actually bit.
    ///
    /// 2000 alternating pairs (4000 real nesting levels) is far beyond
    /// anything the old bypass would have tolerated on a small-stack
    /// thread; this must terminate cleanly, not hang or overflow the
    /// stack — the same rationale as the big-stack helpers in
    /// `analyser::commands::tests` / `lowering::tests` (`cargo test`'s
    /// per-test thread has the same undersized default stack that made
    /// issue #996 reproduce in production).
    #[test]
    fn deep_pass_bounds_alternating_if_apply_nesting() {
        const PAIRS: usize = 2000;
        let mut body = String::from("puts leaf");
        for _ in 0..PAIRS {
            body = format!("if {{1}} {{ apply {{x {{ {body} }}}} 1 }}");
        }
        // With `depth` correctly threaded, an `if`/`apply` pair costs 2
        // logical levels, so real recursion stops at MAX_DEPTH (8) — a
        // handful of native frames regardless of PAIRS. This deliberately
        // runs on the *default* test-thread stack (no big-stack wrapper,
        // unlike the sibling tests in this file and in
        // `commands::tests`/`lowering::tests`): if the reset bug ever comes
        // back, real recursion runs all the way down through every one of
        // PAIRS × 2 levels again — verified locally, reverting just the
        // `depth + 1` fix below to `0` reliably overflows the stack and
        // aborts the test process outright on this same (small,
        // default-sized) thread, before the wall-clock assertion even gets
        // a chance to run. The elapsed-time check below is a second,
        // softer signal for the rare platform/build where the crash
        // threshold happens to sit a little higher.
        let registry = CommandRegistry::build_default();
        let start = std::time::Instant::now();
        let _ = infer_param_traits_deep(&["p"], &body, env_for(&registry, LexerConfig::default()));
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "took {elapsed:?} — alternating if/apply nesting is no longer bounded by \
             MAX_DEPTH (reset-to-0 bypass regression?)"
        );
    }

    #[test]
    fn deep_pass_skips_dynamic_body_args() {
        // `if {1} $body` — the body is a `$var` reference,
        // not a braced literal.  The deep pass shouldn't try
        // to descend into it (we have no body text to scan);
        // the shallow pass already surfaces the `Body` trait
        // via the registry's role scan.  This pins that
        // contract: dynamic body args don't double-count.
        let body = "if {1} $body";
        let deep = infer_deep(&["body"], body);
        assert_trait(&deep, "body", ProcArgTrait::Body);
    }

    #[test]
    fn merge_traits_unions_shallow_and_deep() {
        let mut shallow: HashMap<String, HashSet<ProcArgTrait>> = HashMap::new();
        shallow
            .entry("p1".into())
            .or_default()
            .insert(ProcArgTrait::VarRead);
        let mut deep: HashMap<String, HashSet<ProcArgTrait>> = HashMap::new();
        deep.entry("p1".into())
            .or_default()
            .insert(ProcArgTrait::Eval);
        deep.entry("p2".into())
            .or_default()
            .insert(ProcArgTrait::Body);

        let merged = merge_traits(shallow, deep);
        // p1 gains Eval from deep without losing VarRead from shallow.
        assert!(merged.get("p1").unwrap().contains(&ProcArgTrait::VarRead));
        assert!(merged.get("p1").unwrap().contains(&ProcArgTrait::Eval));
        // p2 (deep-only) lands in the merged map.
        assert!(merged.get("p2").unwrap().contains(&ProcArgTrait::Body));
    }

    #[test]
    fn merge_traits_with_empty_deep_returns_shallow_unchanged() {
        let mut shallow: HashMap<String, HashSet<ProcArgTrait>> = HashMap::new();
        shallow
            .entry("p1".into())
            .or_default()
            .insert(ProcArgTrait::VarWrite);
        let merged = merge_traits(shallow.clone(), HashMap::new());
        assert_eq!(merged.get("p1"), shallow.get("p1"));
    }

    #[test]
    fn deep_pass_matches_shallow_for_top_level_only_bodies() {
        // When there are no nested bodies, the deep pass
        // should return exactly what the shallow pass does.
        let body = "set $x 1\nupvar 1 $var local";
        let shallow = infer(&["x", "var"], body);
        let deep = infer_deep(&["x", "var"], body);
        assert_eq!(shallow, deep);
    }

    #[test]
    fn deep_pass_empty_params_returns_empty_map() {
        let deep = infer_deep(&[], "foreach item $items { uplevel 1 $body }");
        assert!(deep.is_empty());
    }

    // stub-overlay integration
    //
    // These tests pin the contract that a non-empty
    // [`StubOverlay`] threaded through `infer_param_traits` /
    // `infer_param_traits_deep` lets user-declared
    // `# tcl-lsp: stub` commands participate in role-driven
    // trait inference alongside the built-in registry.

    use tcl_registry::stub_overlay::{StubArg, StubSig};

    fn make_overlay(sigs: Vec<StubSig>) -> StubOverlay {
        let mut o = StubOverlay::new();
        for s in sigs {
            o.insert(s);
        }
        o
    }

    fn stub_sig(name: &str, args: &[(&str, ArgRole)]) -> StubSig {
        use tcl_registry::stub_overlay::StubSigFlags;
        StubSig {
            name: name.to_string(),
            args: args
                .iter()
                .map(|(n, r)| StubArg {
                    name: (*n).to_string(),
                    role: *r,
                    optional: false,
                })
                .collect(),
            flags: StubSigFlags::empty(),
        }
    }

    #[test]
    fn overlay_shallow_surfaces_stub_declared_body_role() {
        // `my_eval` isn't in the built-in registry, but the
        // overlay declares its arg-0 as `body`.  An invocation
        // `my_eval $script` should therefore surface `Body` on
        // the `script` param.
        let overlay = make_overlay(vec![stub_sig("my_eval", &[("script", ArgRole::Body)])]);
        let registry = CommandRegistry::build_default();
        let traits = infer_param_traits(
            &["script"],
            "my_eval $script",
            TraitScanEnv {
                registry: &registry,
                stub_overlay: Some(&overlay),
                config: LexerConfig::default(),
                identities: crate::head_identity::HeadIdentityMap::none(),
            },
        );
        assert_trait(&traits, "script", ProcArgTrait::Body);
    }

    #[test]
    fn overlay_shallow_surfaces_stub_declared_var_write_role() {
        // Stub-declared `VarWrite` on a bare `$param` substitution
        // surfaces the callee-local DynamicNameLocal refinement.
        let overlay = make_overlay(vec![stub_sig(
            "with_var",
            &[("varName", ArgRole::VarWrite), ("value", ArgRole::Value)],
        )]);
        let registry = CommandRegistry::build_default();
        let traits = infer_param_traits(
            &["v"],
            "with_var $v 42",
            TraitScanEnv {
                registry: &registry,
                stub_overlay: Some(&overlay),
                config: LexerConfig::default(),
                identities: crate::head_identity::HeadIdentityMap::none(),
            },
        );
        // A stub `VarWrite` role on a bare `$v` substitution is the same
        // callee-local dynamic-name shape as `set $v …` — refined to
        // DynamicNameLocal (+ VarRead), not a caller-frame VarWrite.
        assert_trait(&traits, "v", ProcArgTrait::DynamicNameLocal);
        assert_trait(&traits, "v", ProcArgTrait::VarRead);
    }

    #[test]
    fn overlay_deep_recurses_through_stub_body_args() {
        // The overlay declares `my_loop`'s arg-1 as a body.
        // Without the overlay the deep pass can't see that
        // `my_loop { uplevel 1 $body }` carries an Eval inside.
        // With the overlay it should descend into the brace
        // and surface the Eval.
        let overlay = make_overlay(vec![stub_sig(
            "my_loop",
            &[("count", ArgRole::Value), ("body", ArgRole::Body)],
        )]);
        let registry = CommandRegistry::build_default();
        let body = "my_loop 5 { uplevel 1 $script }";
        // Sanity: without the overlay, the deep pass misses
        // the nested Eval because `my_loop` isn't recognised.
        let no_overlay = infer_param_traits_deep(
            &["script"],
            body,
            env_for(&registry, LexerConfig::default()),
        );
        assert!(
            !no_overlay
                .get("script")
                .is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
            "without overlay, my_loop body shouldn't be recognised, got {no_overlay:?}",
        );
        // With the overlay, the recursion fires.
        let with_overlay = infer_param_traits_deep(
            &["script"],
            body,
            TraitScanEnv {
                registry: &registry,
                stub_overlay: Some(&overlay),
                config: LexerConfig::default(),
                identities: crate::head_identity::HeadIdentityMap::none(),
            },
        );
        assert_trait(&with_overlay, "script", ProcArgTrait::Eval);
    }

    #[test]
    fn overlay_does_not_disturb_registry_resolution() {
        // An overlay covering `my_thing` mustn't shadow any
        // built-in command.  A built-in `foreach` invocation
        // still records its `LoopList` / `Body` traits via the
        // registry path even when an unrelated stub overlay is
        // active.
        let overlay = make_overlay(vec![stub_sig("my_thing", &[("a", ArgRole::Body)])]);
        let registry = CommandRegistry::build_default();
        let traits = infer_param_traits(
            &["items", "body"],
            "foreach x $items $body",
            env_with_overlay(&registry, &overlay),
        );
        assert_trait(&traits, "items", ProcArgTrait::LoopList);
        assert_trait(&traits, "body", ProcArgTrait::Body);
    }

    #[test]
    fn overlay_none_matches_overlay_empty() {
        // An empty overlay should produce the same result as
        // `None` — the overlay is a no-op when it has no
        // entries.
        let registry = CommandRegistry::build_default();
        let body = "foreach x {1 2 3} $body";
        let none = infer_param_traits(&["body"], body, env_for(&registry, LexerConfig::default()));
        let overlay = StubOverlay::new();
        let empty = infer_param_traits(&["body"], body, env_with_overlay(&registry, &overlay));
        assert_eq!(none, empty);
    }

    /// Issue #1275 — trait inference must resolve a command head's *effective
    /// identity*, not its written spelling.
    ///
    /// tclsh oracle (8.6.16 and 9.0.4, byte-identical): `interp alias {} run
    /// {} eval` makes `run $body` evaluate `$body`; `rename eval run` moves it
    /// and leaves `eval` gone; a top-level `proc eval …` takes the name over,
    /// so `eval $body` no longer evaluates anything.
    ///
    /// The environment carries the *document's* facts while the scan reads a
    /// *proc body*, which is exactly the shape the analyser threads.
    fn traits_under(prelude: &str, body: &str) -> HashMap<String, HashSet<ProcArgTrait>> {
        let registry = CommandRegistry::build_default();
        let identities = crate::head_identity::command_head_identities(
            prelude,
            tcl_dialect::DialectProfile::by_name("tcl8.6"),
            &registry,
        );
        infer_param_traits(
            &["body"],
            body,
            TraitScanEnv {
                registry: &registry,
                stub_overlay: None,
                config: LexerConfig::default(),
                identities: &identities,
            },
        )
    }

    fn is_eval(traits: &HashMap<String, HashSet<ProcArgTrait>>) -> bool {
        traits
            .get("body")
            .is_some_and(|s| s.contains(&ProcArgTrait::Eval))
    }

    #[test]
    fn param_traits_follow_an_aliased_head() {
        assert!(is_eval(&traits_under(
            "interp alias {} run {} eval\n",
            "run $body"
        )));
        // The `::`-qualified spelling of the alias classifies alike.
        assert!(is_eval(&traits_under(
            "interp alias {} run {} eval\n",
            "::run $body"
        )));
        // Guard: an unbound `run` evaluates nothing.
        assert!(!is_eval(&traits_under("set y 1\n", "run $body")));
    }

    #[test]
    fn param_traits_follow_a_renamed_head() {
        assert!(is_eval(&traits_under("rename eval run\n", "run $body")));
        assert!(
            !is_eval(&traits_under("rename eval run\n", "eval $body")),
            "a renamed-away `eval` must not keep the built-in's grammar"
        );
    }

    #[test]
    fn param_traits_abstain_for_a_builtin_shadowed_by_a_user_proc() {
        assert!(
            !is_eval(&traits_under("proc eval {s} { return $s }\n", "eval $body")),
            "a user `proc eval` takes the name over; its argument is not a script"
        );
        // Guard: the unshadowed built-in still records the trait.
        assert!(is_eval(&traits_under("set y 1\n", "eval $body")));
    }

    #[test]
    fn param_traits_abstain_for_a_dynamic_binding() {
        assert!(
            !is_eval(&traits_under("rename $old run\n", "run $body")),
            "a dynamic rename must not make `run` an evaluator"
        );
        assert!(
            is_eval(&traits_under("rename $old run\n", "eval $body")),
            "a dynamic rename must not take `eval`'s grammar away either"
        );
    }

    /// The caller-frame scans read the resolved head too: `upvar`'s
    /// `FrameArgLayout::AliasPairs` grammar belongs to the command a head is.
    #[test]
    fn caller_frame_scans_follow_the_resolved_head() {
        let registry = CommandRegistry::build_default();
        let env_of = |prelude: &str| {
            crate::head_identity::command_head_identities(
                prelude,
                tcl_dialect::DialectProfile::by_name("tcl8.6"),
                &registry,
            )
        };

        let aliased = env_of("interp alias {} peek {} upvar\n");
        let env = TraitScanEnv {
            registry: &registry,
            stub_overlay: None,
            config: LexerConfig::default(),
            identities: &aliased,
        };
        assert!(
            caller_frame_upvar_params(&["v"], "peek 1 $v alias", env).contains("v"),
            "an alias of `upvar` must bind the caller's frame like `upvar`"
        );
        assert!(
            caller_frame_literal_targets("peek 1 name name\nset name W1", env).contains_key("name"),
            "an alias of `upvar` must record its literal caller-frame target"
        );

        let shadowed = env_of("proc upvar {a b c} { return 1 }\n");
        let env = TraitScanEnv {
            identities: &shadowed,
            ..env
        };
        assert!(
            caller_frame_upvar_params(&["v"], "upvar 1 $v alias", env).is_empty(),
            "a shadowed `upvar` binds no caller frame"
        );
        assert!(
            caller_frame_literal_targets("upvar 1 name name\nset name W1", env).is_empty(),
            "a shadowed `upvar` records no literal caller-frame target"
        );

        // Guard: with nothing bound, both scans fire on the built-in.
        let none = crate::head_identity::HeadIdentityMap::none();
        let env = TraitScanEnv {
            identities: none,
            ..env
        };
        assert!(caller_frame_upvar_params(&["v"], "upvar 1 $v alias", env).contains("v"));
        assert!(
            caller_frame_literal_targets("upvar 1 name name\nset name W1", env)
                .contains_key("name")
        );
    }
}
