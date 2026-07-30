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

//! Per-proc **frame-effect summary** — which names a procedure injects into
//! the frame of whoever calls it.
//!
//! Tcl procedures routinely write their caller's variables: `upvar` aliases
//! a caller-frame name into a local, and `uplevel` runs a whole script up
//! there.  Neither shows up in the caller's own text, so single-function
//! dataflow sees a read of a variable nothing ever assigned.  This module
//! computes, once per procedure, *what a call to it does to the caller's
//! frame*; [`super::CfgBuilder::apply_upvar_invalidation`] then consults it
//! at every call site — a hash lookup plus work proportional to the bound
//! arguments, with no fixpoint.
//!
//! The type is still named [`UpvarInfo`] because the salsa interning layer
//! (`tcl-lsp-db`) stores it by that path; it covers `uplevel` as well.
//!
//! ## What the summary records
//!
//! **Named caller-frame bindings** — `upvar`, resolvable to a name:
//!
//! * **`literal_targets`** — `upvar 1 caller_name local_name`: the source
//!   name is a literal (no `$` substitution), so we know exactly which
//!   caller-frame variable the local aliases.
//! * **`param_targets`** — `upvar 1 $param local_name`: the source is a
//!   `$<param>` reference where the proc has a parameter named
//!   `<param>`.  Resolved at call-site by substituting the actual
//!   argument value passed for that parameter.
//! * **`args_tail_upvar`** — `upvar 1 args local_name`: the source is the
//!   proc's `args` parameter (the trailing-vararg slot).  Each
//!   call-site arg in the tail position is substituted into the alias.
//!
//! **Named caller-frame writes without an alias** — `uplevel`, whose script
//! runs in the caller's frame rather than binding a local:
//!
//! * **`uplevel_literal_writes`** — `uplevel 1 {set n …}` (a brace-literal
//!   body the lowering already turned into [`Statement::UpFrame`]) and
//!   `uplevel 1 [list set n …]`.
//! * **`uplevel_param_writes`** — `uplevel 1 [list set $param …]`: the
//!   written name is the value of `<param>`, resolved at the call site
//!   exactly like `param_targets`.
//! * **`uplevel_forwarded_calls`** — `uplevel 1 [list worker $p]`: the
//!   constructed script *calls* another proc, so that proc's own `upvar 1`
//!   reaches one frame further out than a plain call would — into **this**
//!   proc's caller ([issue #1019][]).  Resolved one hop, in
//!   [`super::detect_upvar_procs`], where the module-wide map exists.
//!
//! **Opaque caller-frame effects** — the effect is real but the names are
//! not knowable:
//!
//! * **`has_unresolvable_caller_target`** — `upvar 1 $computed x`.
//! * **`caller_frame_opaque_writes` / `caller_frame_opaque_reads`** —
//!   `uplevel 1 $body`: arbitrary code in the caller's frame.
//!
//! ## Frame targeting is not decoration
//!
//! Only a **`Relative(1)`** level (or an omitted one) touches the direct
//! caller.  Everything else is a different frame and must not be reported
//! as a caller-frame binding — pinned on tclsh 9.0.4 and 8.6.14, identical:
//!
//! | written in the callee | frame written | in this summary |
//! |---|---|---|
//! | `upvar 1 x y` / `upvar x y` | the caller | named binding |
//! | `upvar #0 g l` | the global frame | nothing — [`super::global_write_info`] owns it |
//! | `upvar 0 x y` | the callee's **own** frame | nothing |
//! | `upvar 2 far f` | the caller's caller | widen (see below) |
//! | `upvar $lvl a b` | unknown | widen |
//! | `uplevel 1 $body` | the caller | opaque write **and** read |
//! | `eval $body` | the callee's own frame | nothing — [`crate::dynamic_names`] owns it |
//!
//! A level the summary cannot place at the direct caller is widened to an
//! opaque caller-frame write rather than dropped.  Dropping it would leave
//! a real `upvar 2` write invisible to the grandparent frame that receives
//! it, producing a false `W210`; widening only ever silences one. That is
//! the [documented abstention direction][soundness] for every consumer of
//! this summary.
//!
//! [issue #1019]: https://github.com/bitwisecook/tcl-lsp/issues/1019
//! [soundness]: crate::dynamic_names

use std::collections::{BTreeMap, BTreeSet};

use tcl_registry::frame_effect::{FrameArgLayout, FrameEffectSpec, FrameLevel};
use tcl_registry::{ArgRole, CommandRegistry};

use crate::ir::{Script, Statement};

/// Per-proc summary of every `upvar` declaration in the body.
///
/// `literal_targets` /
/// `param_targets` / `args_tail_upvar` are mutually exclusive — each
/// `(source, local)` pair lands in exactly one bucket.  Maps key on
/// the *local* name (the alias target) so caller-side resolution can
/// look up "which caller-frame name does my local `x` alias?" in a
/// single hash hit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct UpvarInfo {
    /// `local_name -> caller_literal_name` for declarations whose
    /// source word is a plain string (no `$` / `[`).
    pub literal_targets: BTreeMap<String, String>,
    /// `local_name -> param_name` for declarations whose source word
    /// is `$<param>` and `<param>` is a proc parameter.  The
    /// caller-side resolver substitutes the actual argument passed
    /// for `<param>` to produce the caller-frame name.
    pub param_targets: BTreeMap<String, String>,
    /// `local_name`s whose source is `$args` (the trailing-vararg
    /// param).  Each call-site arg in the tail position is aliased
    /// to one of these locals; the per-local pairing is positional,
    /// the order here matches the order of declarations in the
    /// proc body.
    pub args_tail_upvar: Vec<String>,
    /// The proc has an `upvar` whose CALLER-side name cannot be resolved
    /// statically (`upvar 1 $computed x`, or `$foo` where `foo` is not a
    /// parameter): the callee can write *any* caller variable, so
    /// [`Self::caller_side_defs`] under-approximates and the call site must
    /// widen to an opaque caller-frame clobber instead of trusting it.
    pub has_unresolvable_caller_target: bool,
    /// Literal caller-frame names the proc writes through an `uplevel`
    /// script that runs in the caller's frame — `uplevel 1 {set n 1}` and
    /// `uplevel 1 [list set n 1]`.  Unlike `upvar`, no local alias is
    /// created, so these have no key to hang off.
    pub uplevel_literal_writes: BTreeSet<String>,
    /// Parameter names whose **value** names a caller-frame variable the
    /// proc writes through an `uplevel` script — `uplevel 1 [list set
    /// $varName …]`.  Resolved at the call site against the actual argument
    /// passed for that parameter, exactly like [`Self::param_targets`].
    pub uplevel_param_writes: BTreeSet<String>,
    /// `uplevel <caller frame> [list CMD ARG…]` sites whose `CMD` is not a
    /// registry command — a candidate user proc whose own caller-frame
    /// effects land in *this* proc's caller.  Stored unresolved because a
    /// per-proc walk cannot see the module; [`super::detect_upvar_procs`]
    /// resolves each entry one hop against the completed map.
    ///
    /// Each entry is `(command, constructed argument words)`.
    pub uplevel_forwarded_calls: Vec<(String, Vec<String>)>,
    /// The proc runs a script it cannot read in its caller's frame
    /// (`uplevel 1 $body`), or aliases a caller-frame name it cannot place
    /// (`upvar 2 …`, `upvar $lvl …`).  **Any** caller-frame name may be
    /// created or overwritten, so a call site must widen instead of
    /// enumerating.
    pub caller_frame_opaque_writes: bool,
    /// The same script may **read** any caller-frame name, so no store in
    /// the caller can be proved dead across the call.
    pub caller_frame_opaque_reads: bool,
}

impl UpvarInfo {
    /// True if every bucket is empty — no `upvar` declarations in
    /// the proc body.  Callers with no upvar info can skip the
    /// per-call resolution path.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.literal_targets.is_empty()
            && self.param_targets.is_empty()
            && self.args_tail_upvar.is_empty()
            && self.uplevel_literal_writes.is_empty()
            && self.uplevel_param_writes.is_empty()
            && self.uplevel_forwarded_calls.is_empty()
            // A flag-only summary still matters: an unresolvable caller
            // target means call sites must widen, so the info must not be
            // dropped by `detect_upvar_procs`'s emptiness filter.
            && !self.has_unresolvable_caller_target
            && !self.caller_frame_opaque_writes
            && !self.caller_frame_opaque_reads
    }

    /// The caller-frame barrier a call to this proc imposes on the calling
    /// function: `writes` when any name in the caller's frame may be
    /// created or overwritten under a name this analysis cannot enumerate,
    /// `reads` when any store in the caller may be observed.
    ///
    /// `destroys` rides with `reads`: only an arbitrary script
    /// (`uplevel 1 $body`) can `unset` a caller-frame name, and that is
    /// exactly the case that sets `reads` too.  An unresolvable `upvar`
    /// target writes through an alias, which cannot destroy the binding.
    #[must_use]
    pub fn caller_frame_barrier(&self) -> crate::dynamic_names::DynamicNameBarrier {
        crate::dynamic_names::DynamicNameBarrier {
            writes: self.caller_frame_opaque_writes || self.has_unresolvable_caller_target,
            destroys: self.caller_frame_opaque_reads,
            reads: self.caller_frame_opaque_reads,
        }
    }

    /// Resolve the caller-frame name aliased by *local* in this
    /// proc.  Returns the literal name for `literal_targets`, the
    /// substituted argument value for `param_targets` (looked up in
    /// the supplied `args` map), and `None` when *local* isn't an
    /// upvar target or when the param substitution fails.
    ///
    /// `args` maps proc parameter names to the literal value passed
    /// at the call site (or `None` when the value is dynamic).
    /// `args_tail_upvar` resolution is positional and lives outside
    /// this helper — caller-side logic walks the per-local list and
    /// pairs each entry with the corresponding tail-position
    /// argument.
    #[must_use]
    pub fn resolve_literal_or_param(
        &self,
        local: &str,
        args: &BTreeMap<String, Option<String>>,
    ) -> Option<String> {
        if let Some(lit) = self.literal_targets.get(local) {
            return Some(lit.clone());
        }
        if let Some(param) = self.param_targets.get(local) {
            return args.get(param).and_then(Clone::clone);
        }
        None
    }

    /// Resolve every upvar declaration in this proc against a call
    /// site, returning the unique set of caller-frame variable names
    /// the proc may modify via its upvars.
    ///
    /// `call_args` is the positional argument list at the call site;
    /// `params` is the callee proc's parameter list (used to locate
    /// param-based upvar sources by positional index).  Resolution:
    ///
    /// * Every value in [`literal_targets`](Self::literal_targets) is
    ///   added directly — those are caller-frame literal names.
    /// * For each value in [`param_targets`](Self::param_targets), we
    ///   find the param's positional index in `params`, look up the
    ///   actual argument value at that index in `call_args`, and
    ///   normalise it (stripping `$` / `${…}` / `(…)` suffixes).
    /// * For each entry in [`args_tail_upvar`](Self::args_tail_upvar),
    ///   we pair it positionally with a tail argument at the call
    ///   site (tail start = `params.len() - 1`, since `args` consumes
    ///   the trailing slot in the param list).
    #[must_use]
    pub fn caller_side_defs(&self, call_args: &[String], params: &[String]) -> Vec<String> {
        let mut defs: Vec<String> = Vec::new();
        let mut push = |name: String| {
            if !name.is_empty() && !defs.contains(&name) {
                defs.push(name);
            }
        };
        for caller_lit in self
            .literal_targets
            .values()
            .chain(self.uplevel_literal_writes.iter())
        {
            push(caller_lit.clone());
        }
        for param_name in self
            .param_targets
            .values()
            .chain(self.uplevel_param_writes.iter())
        {
            if let Some(idx) = params.iter().position(|p| p == param_name)
                && let Some(arg) = call_args.get(idx)
            {
                push(crate::naming::normalise_var_name(arg).to_owned());
            }
        }
        if !self.args_tail_upvar.is_empty() {
            // `args` occupies the trailing param slot, so tail args
            // at the call site start at `params.len() - 1`.
            let tail_start = params.len().saturating_sub(1);
            for (i, _local) in self.args_tail_upvar.iter().enumerate() {
                if let Some(arg) = call_args.get(tail_start + i) {
                    push(crate::naming::normalise_var_name(arg).to_owned());
                }
            }
        }
        defs
    }
}

/// True if *src* looks like a `$<param>` or `${<param>}` reference,
/// returning the inner param name.  The IR lowerer normalises bare
/// `$name` to `${name}` for some shapes, so we accept both.
fn is_dollar_param_ref(raw_src: &str) -> Option<&str> {
    let stripped = raw_src.strip_prefix('$')?;
    let inner = if let Some(rest) = stripped.strip_prefix('{') {
        // ${name}
        let end = rest.strip_suffix('}')?;
        if end.is_empty() {
            return None;
        }
        end
    } else {
        stripped
    };
    if inner.is_empty() || !is_var_name(inner) {
        return None;
    }
    Some(inner)
}

/// True if *name* is a plain literal name with no substitutions or
/// special characters.
fn is_literal_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('$')
        && !name.contains('[')
        && !name.contains('{')
        && !name.contains('"')
        && !name.contains(' ')
}

fn is_var_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
}

/// Resolve a frame-crossing command's level word and its remaining
/// arguments through the registry's [`FrameEffectSpec`].
fn resolve_frame_args(spec: FrameEffectSpec, args: &[String]) -> (FrameLevel, &[String]) {
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let taken = spec.level_word_len(&refs);
    let level = if taken == 0 {
        FrameLevel::DEFAULT
    } else {
        FrameLevel::parse(&args[0]).unwrap_or(FrameLevel::Dynamic)
    };
    (level, &args[taken..])
}

/// Collect the per-proc frame-effect summary from a proc body.
///
/// `params` is the proc's parameter list (used to gate
/// `param_targets`).  `body` is the lowered IR body.  Walks every
/// statement for a registry-declared frame-crossing command
/// ([`FrameEffectSpec`]) and accumulates the result in [`UpvarInfo`].
#[must_use]
pub fn collect_upvar_targets(body: &Script, params: &[String]) -> UpvarInfo {
    // The frame grammar of `upvar` / `uplevel` is core Tcl and identical in
    // every dialect, so the cached default registry is the right source —
    // the same reasoning (and the same issue #1035 allocation concern)
    // `global_write_info::detect_global_write_procs` records for its own
    // scan, which runs on the same per-keystroke path.
    let registry = tcl_registry::cache::default_registry();
    let mut info = UpvarInfo::default();
    walk_script(&body.statements, params, registry, &mut info);
    info
}

fn walk_script(
    stmts: &[Statement],
    params: &[String],
    registry: &CommandRegistry,
    info: &mut UpvarInfo,
) {
    for stmt in stmts {
        walk_stmt(stmt, params, registry, info);
    }
}

fn walk_stmt(
    stmt: &Statement,
    params: &[String],
    registry: &CommandRegistry,
    info: &mut UpvarInfo,
) {
    match stmt {
        Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. } => {
            let Some(spec) = registry.frame_effect(command) else {
                return;
            };
            match spec.layout {
                FrameArgLayout::AliasPairs => record_upvar_call(spec, args, params, info),
                FrameArgLayout::ScriptInSelectedFrame => {
                    record_uplevel_call(spec, args, params, registry, info);
                }
                // A script that runs in the *callee's own* frame, and an
                // opaque injection into whoever called the command, are both
                // effects on the frame the command is written in — not on
                // this proc's caller. `crate::dynamic_names` owns them.
                FrameArgLayout::ScriptInCurrentFrame | FrameArgLayout::OpaqueCallerVars => {}
            }
        }
        // `uplevel <level> {literal body}` — the lowering already proved the
        // body readable and inlined it here. Its writes land in the frame
        // `frame_shift`/`absolute` select, so only the caller-frame form
        // contributes; every other level is a different frame.
        Statement::UpFrame {
            frame_shift,
            absolute,
            body,
            ..
        } => {
            if !*absolute && *frame_shift == 1 {
                record_upframe_body(&body.statements, info);
            }
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                walk_script(&c.body.statements, params, registry, info);
            }
            if let Some(e) = else_body {
                walk_script(&e.statements, params, registry, info);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_script(&init.statements, params, registry, info);
            walk_script(&next.statements, params, registry, info);
            walk_script(&body.statements, params, registry, info);
        }
        Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Block { body, .. } => walk_script(&body.statements, params, registry, info),
        Statement::Switch {
            arms, default_body, ..
        } => {
            for arm in arms {
                if let Some(b) = &arm.body {
                    walk_script(&b.statements, params, registry, info);
                }
            }
            if let Some(b) = default_body {
                walk_script(&b.statements, params, registry, info);
            }
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(&body.statements, params, registry, info);
            for h in handlers {
                walk_script(&h.body.statements, params, registry, info);
            }
            if let Some(f) = finally_body {
                walk_script(&f.statements, params, registry, info);
            }
        }
        _ => {}
    }
}

/// `upvar ?level? otherVar myVar …` — classify each pair against the frame
/// the level word selects.
fn record_upvar_call(
    spec: FrameEffectSpec,
    args: &[String],
    params: &[String],
    info: &mut UpvarInfo,
) {
    let (level, rest) = resolve_frame_args(spec, args);
    if !level.is_caller_frame() {
        // A level this summary cannot place at the direct caller. `#0` and
        // `0` genuinely miss the caller's frame (the global frame is
        // `global_write_info`'s business, the callee's own frame nobody
        // else's), so they contribute nothing. Everything else — `upvar 2`,
        // `upvar #3`, `upvar $lvl` — writes *some* frame further out, and
        // dropping it would leave that write invisible where it lands, so
        // widen. See the module doc's frame table.
        if !level.is_global_frame() && !level.is_current_frame() {
            info.caller_frame_opaque_writes = true;
        }
        return;
    }
    let mut i = 0;
    while i + 1 < rest.len() {
        let (src, dst) = (rest[i].as_str(), rest[i + 1].as_str());
        i += 2;
        if !is_literal_name(dst) {
            // Dynamic destination — can't classify.  Caller-side
            // analysis falls back to the dynamic-barrier path.
            continue;
        }
        if is_literal_name(src) {
            // `upvar 1 caller_x x` — literal source.
            info.literal_targets
                .insert(dst.to_string(), src.to_string());
        } else if let Some(param) = is_dollar_param_ref(src) {
            // `upvar 1 $param x` — substitution-driven, gated on
            // the param existing on the proc.
            if param == "args" {
                info.args_tail_upvar.push(dst.to_string());
            } else if params.iter().any(|p| p == param) {
                info.param_targets
                    .insert(dst.to_string(), param.to_string());
            } else {
                // `$foo` where `foo` isn't a param — the caller-side name
                // depends on runtime state; the callee can clobber any
                // caller variable, so the summary must widen (a silent
                // skip would let SCCP propagate stale caller values past
                // the call).
                info.has_unresolvable_caller_target = true;
            }
        } else {
            // Fully dynamic source (`upvar 1 [pick] x`, `upvar 1 a($i) x`,
            // …) — same widening as above.
            info.has_unresolvable_caller_target = true;
        }
    }
}

/// `uplevel ?level? arg …` — the concatenated `arg`s run as a script in the
/// frame the level selects.
///
/// Three body shapes are distinguishable, and only the third is opaque:
///
/// * a **brace literal** never reaches here — the lowering turned it into
///   [`Statement::UpFrame`], handled by [`record_upframe_body`];
/// * a **constructed list** (`[list set $v 1]`) names its command
///   statically, so [`record_constructed_body`] can read it;
/// * anything else (`$body`, `[build]`, `"$a $b"`) is arbitrary code —
///   widen both directions.
fn record_uplevel_call(
    spec: FrameEffectSpec,
    args: &[String],
    params: &[String],
    registry: &CommandRegistry,
    info: &mut UpvarInfo,
) {
    let (level, rest) = resolve_frame_args(spec, args);
    if !level.is_caller_frame() {
        // Same reasoning as `record_upvar_call`: `uplevel 0` stays in the
        // callee's own frame and `uplevel #0` runs in the global one, so
        // neither touches the caller; any other level does, somewhere this
        // summary cannot name.
        if !level.is_global_frame() && !level.is_current_frame() {
            info.caller_frame_opaque_writes = true;
            info.caller_frame_opaque_reads = true;
        }
        return;
    }
    // `uplevel` concat-joins every remaining word into one script (its
    // `SCRIPT_CONCATENATES_ARGS` trait). A single constructed-list word is
    // the readable case; a multi-word join is not worth reassembling.
    if let [single] = rest
        && record_constructed_body(single, params, registry, info)
    {
        return;
    }
    info.caller_frame_opaque_writes = true;
    info.caller_frame_opaque_reads = true;
}

/// The statements of an inlined `uplevel 1 {literal}` body: every literal
/// name it assigns is a caller-frame write.
///
/// A body statement whose target is *not* a plain literal name means the
/// body writes somewhere this summary cannot name, so widen — the same
/// direction the constructed-list path takes.
fn record_upframe_body(stmts: &[Statement], info: &mut UpvarInfo) {
    for stmt in stmts {
        for name in crate::ssa::defs_of(stmt) {
            if is_literal_name(&name) {
                info.uplevel_literal_writes.insert(name);
            } else {
                info.caller_frame_opaque_writes = true;
            }
        }
    }
}

/// Read a `[list CMD ARG…]` body word, recording the caller-frame names the
/// constructed command writes.  Returns `false` when the word is not a
/// readable constructed list, so the caller can widen.
///
/// `[list …]` is recognised through the registry's
/// [`ReturnElements::ListOfArgs`](tcl_registry::ReturnElements) — the same
/// fact the concat rules of #1068 read — not by matching the name `list`.
fn record_constructed_body(
    word: &str,
    params: &[String],
    registry: &CommandRegistry,
    info: &mut UpvarInfo,
) -> bool {
    let Some(inner) = word.strip_prefix('[').and_then(|w| w.strip_suffix(']')) else {
        return false;
    };
    let words = super::words_from_text(inner);
    let Some((builder, constructed)) = words.split_first() else {
        return false;
    };
    if !registry.get(builder).is_some_and(|spec| {
        matches!(
            spec.return_elements,
            Some(tcl_registry::ReturnElements::ListOfArgs { from: 0 })
        )
    }) {
        return false;
    }
    let Some((command, cargs)) = constructed.split_first() else {
        return false;
    };
    if !is_literal_name(command) {
        return false;
    }
    let arg_refs: Vec<&str> = cargs.iter().map(String::as_str).collect();
    if registry.get(command).is_some() {
        // A registry command: its own `VarWrite` roles say which words name
        // the variables it creates, and they name them in whatever frame it
        // runs in — here, the caller's.
        let indices = registry.arg_indices_for_role(command, &arg_refs, ArgRole::VarWrite);
        if indices.is_empty() {
            // A known command that writes no variable (`uplevel 1 [list
            // puts hi]`) has no caller-frame effect at all.
            return true;
        }
        for idx in indices {
            let Some(target) = arg_refs.get(idx) else {
                continue;
            };
            if is_literal_name(target) {
                info.uplevel_literal_writes.insert((*target).to_string());
            } else if let Some(param) = is_dollar_param_ref(target)
                && params.iter().any(|p| p == param)
            {
                info.uplevel_param_writes.insert(param.to_string());
            } else {
                info.caller_frame_opaque_writes = true;
            }
        }
        return true;
    }
    // Not a registry command — a candidate user proc. Its own `upvar 1`
    // reaches *this* proc's caller, one frame further out than a plain call
    // would (issue #1019), but only the module-wide pass can resolve it.
    info.uplevel_forwarded_calls
        .push((command.clone(), constructed[1..].to_vec()));
    true
}

/// Compose a callee's caller-frame effects into `info`, for a
/// `uplevel <caller frame> [list callee ARG…]` forward.
///
/// The callee's `upvar 1` binds a name in *its* caller's frame — which,
/// because the constructed script runs in `info`'s own caller's frame, is
/// that same frame.  So each of the callee's caller-frame names becomes one
/// of `info`'s, translated through the constructed argument list:
/// a literal argument contributes a literal name, a `$param` argument
/// contributes a param write, anything else widens.
///
/// tclsh 9.0.4 / 8.6.14, identical: with
/// `proc worker {nvar} {upvar 1 $nvar v; set v WORKED}` and
/// `proc wrapper {nvar} {uplevel 1 [list worker $nvar]}`, calling
/// `wrapper target` leaves `target` set in *wrapper's* caller — while the
/// plain-call spelling `proc wrapper2 {nvar} {worker $nvar}` raises
/// `can't read "target": no such variable`.
pub(super) fn compose_forwarded(
    callee: &UpvarInfo,
    callee_params: &[String],
    constructed: &[String],
    params: &[String],
    info: &mut UpvarInfo,
) {
    if callee.has_unresolvable_caller_target
        || callee.caller_frame_opaque_writes
        || !callee.args_tail_upvar.is_empty()
    {
        info.caller_frame_opaque_writes = true;
    }
    if callee.caller_frame_opaque_reads {
        info.caller_frame_opaque_reads = true;
    }
    for name in callee
        .literal_targets
        .values()
        .chain(callee.uplevel_literal_writes.iter())
    {
        info.uplevel_literal_writes.insert(name.clone());
    }
    for param_name in callee
        .param_targets
        .values()
        .chain(callee.uplevel_param_writes.iter())
    {
        let Some(idx) = callee_params.iter().position(|p| p == param_name) else {
            continue;
        };
        let Some(arg) = constructed.get(idx) else {
            continue;
        };
        if is_literal_name(arg) {
            info.uplevel_literal_writes.insert(arg.clone());
        } else if let Some(param) = is_dollar_param_ref(arg)
            && params.iter().any(|p| p == param)
        {
            info.uplevel_param_writes.insert(param.to_string());
        } else {
            info.caller_frame_opaque_writes = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Script;
    use crate::lowering::lower_to_ir;
    use tcl_registry::CommandRegistry;

    fn lower(src: &str) -> Script {
        let m = lower_to_ir(src, &CommandRegistry::build_default());
        m.top_level
    }

    #[test]
    fn empty_body_has_no_upvars() {
        let body = lower("");
        let info = collect_upvar_targets(&body, &[]);
        assert!(info.is_empty());
    }

    #[test]
    fn literal_target_recorded() {
        let body = lower("upvar 1 caller_x x");
        let info = collect_upvar_targets(&body, &[]);
        assert_eq!(info.literal_targets.get("x"), Some(&"caller_x".to_string()),);
        assert!(info.param_targets.is_empty());
        assert!(info.args_tail_upvar.is_empty());
    }

    #[test]
    fn multiple_literal_targets_recorded() {
        let body = lower("upvar 1 a la b lb c lc");
        let info = collect_upvar_targets(&body, &[]);
        assert_eq!(info.literal_targets.get("la"), Some(&"a".to_string()));
        assert_eq!(info.literal_targets.get("lb"), Some(&"b".to_string()));
        assert_eq!(info.literal_targets.get("lc"), Some(&"c".to_string()));
    }

    #[test]
    fn param_target_recorded_when_param_exists() {
        let body = lower("upvar 1 $name x");
        let info = collect_upvar_targets(&body, &["name".to_string()]);
        assert_eq!(info.param_targets.get("x"), Some(&"name".to_string()));
        assert!(info.literal_targets.is_empty());
    }

    #[test]
    fn dollar_var_skipped_when_not_a_param() {
        let body = lower("upvar 1 $foo x");
        let info = collect_upvar_targets(&body, &["bar".to_string()]);
        // $foo is not a param (only `bar` is); not classifiable per-name —
        // and the summary must say so, not silently under-approximate: the
        // callee can write ANY caller variable through the alias.
        assert!(info.param_targets.is_empty());
        assert!(info.literal_targets.is_empty());
        assert!(info.has_unresolvable_caller_target);
    }

    #[test]
    fn fully_dynamic_source_widens_summary() {
        // `upvar 1 [pick] x` / an array-element source — the caller-side
        // name is runtime state; the flag must be set so the call site
        // widens to an opaque caller-frame clobber.
        for src in ["upvar 1 [pick] x", "upvar 1 a($i) x"] {
            let body = lower(src);
            let info = collect_upvar_targets(&body, &[]);
            assert!(
                info.has_unresolvable_caller_target,
                "{src} must widen the summary"
            );
        }
        // Resolvable shapes must NOT set the flag.
        let body = lower("upvar 1 caller_x x\nupvar 1 $p y");
        let info = collect_upvar_targets(&body, &["p".to_string()]);
        assert!(!info.has_unresolvable_caller_target);
    }

    #[test]
    fn args_tail_upvar_recorded() {
        let body = lower("upvar 1 $args y");
        let info = collect_upvar_targets(&body, &["args".to_string()]);
        // `args` triggers the trailing-vararg path regardless of
        // its position in the param list.
        assert_eq!(info.args_tail_upvar, vec!["y".to_string()]);
        assert!(info.param_targets.is_empty());
    }

    #[test]
    fn level_omitted_defaults_to_one() {
        // No leading level word: `upvar src dst`.
        let body = lower("upvar caller_x x");
        let info = collect_upvar_targets(&body, &[]);
        assert_eq!(info.literal_targets.get("x"), Some(&"caller_x".to_string()),);
    }

    #[test]
    fn level_hash_zero_is_not_a_caller_frame_binding() {
        // TN — `upvar #0 g local_g` binds the **global** `g`, not the
        // caller's, however deep the call stack is (tclsh 9.0.4 / 8.6.14:
        // `proc deep {} {upvar #0 gvar g; set g DEEP}` called two frames
        // down sets `::gvar`).  Reporting it as a caller-frame def would
        // mark an unrelated *local* named `g` defined in every caller.
        // `global_write_info` owns the real, global effect.
        let body = lower("upvar #0 g local_g");
        let info = collect_upvar_targets(&body, &[]);
        assert!(info.literal_targets.is_empty(), "got {info:?}");
        assert!(!info.caller_frame_opaque_writes, "got {info:?}");
    }

    #[test]
    fn level_zero_is_the_callees_own_frame_and_contributes_nothing() {
        // TN — `upvar 0 x y` aliases the *callee's own* local `x` (tclsh
        // 9.0.4 / 8.6.14: `upvar 0 target alias` inside a proc whose caller
        // has `target` reads `can't read "alias"`), so no caller effect.
        let body = lower("upvar 0 x y");
        let info = collect_upvar_targets(&body, &[]);
        assert!(info.is_empty(), "got {info:?}");
    }

    #[test]
    fn level_beyond_the_caller_widens_rather_than_binding() {
        // `upvar 2 far f` writes the caller's *caller*. Naming `far` as a
        // caller-frame def would be wrong; dropping it would hide the write
        // from the frame that does receive it. Widen — the abstain-toward-
        // silence direction (tclsh 9.0.4 / 8.6.14: `proc gp {} {upvar 2 far
        // f; set f FAR}` through one intermediate frame sets the
        // grandparent's `far`).
        for src in ["upvar 2 far f", "upvar #3 far f", "upvar $lvl a b"] {
            let body = lower(src);
            let info = collect_upvar_targets(&body, &["lvl".to_string()]);
            assert!(
                info.caller_frame_opaque_writes,
                "{src} must widen; got {info:?}"
            );
            assert!(info.literal_targets.is_empty(), "{src}: got {info:?}");
        }
    }

    #[test]
    fn level_word_presence_follows_argument_count_parity() {
        // `upvar $lvl a b` — three words, so `$lvl` is the level and
        // `(a, b)` is the pair. A text-sniffing level test saw no level here
        // and paired `($lvl, a)`, losing the binding (tclsh 9.0.4 / 8.6.14:
        // `proc t4 {lvl} {upvar $lvl a b; set b 4}` really does set the
        // caller's `a`).  The level is dynamic, so the summary widens.
        let body = lower("upvar $lvl a b");
        let info = collect_upvar_targets(&body, &["lvl".to_string()]);
        assert!(info.caller_frame_opaque_writes, "got {info:?}");

        // `upvar 1 b` — two words, so there is NO level word and `1` is the
        // caller-side *variable name* (tclsh 9.0.4 / 8.6.14: with `set 1
        // ONE` in the caller, the callee reads `ONE` through the alias).
        let body = lower("upvar 1 b");
        let info = collect_upvar_targets(&body, &[]);
        assert_eq!(info.literal_targets.get("b"), Some(&"1".to_string()));

        // `upvar 1 a b c` — four words, so no level: two pairs, `(1, a)` and
        // `(b, c)` (tclsh 9.0.4 / 8.6.14 accept it without error).
        let body = lower("upvar 1 a b c");
        let info = collect_upvar_targets(&body, &[]);
        assert_eq!(info.literal_targets.get("a"), Some(&"1".to_string()));
        assert_eq!(info.literal_targets.get("c"), Some(&"b".to_string()));
    }

    #[test]
    fn uplevel_literal_body_records_caller_frame_writes() {
        // `uplevel 1 {set litVar hello}` — the lowering inlines the literal
        // body as `Statement::UpFrame`; its writes land in the caller.
        let body = lower("uplevel 1 {set litVar hello}");
        let info = collect_upvar_targets(&body, &[]);
        assert!(
            info.uplevel_literal_writes.contains("litVar"),
            "got {info:?}"
        );
        assert!(!info.caller_frame_opaque_writes, "got {info:?}");
    }

    #[test]
    fn uplevel_constructed_list_resolves_the_written_name() {
        // `uplevel 1 [list set $varName …]` — the constructed script names
        // its command statically, and `set`'s own `VarWrite` role says which
        // word is the target (tclsh 9.0.4 / 8.6.14: the caller's variable is
        // genuinely created).
        let body = lower("uplevel 1 [list set $varName 1]");
        let info = collect_upvar_targets(&body, &["varName".to_string()]);
        assert!(
            info.uplevel_param_writes.contains("varName"),
            "got {info:?}"
        );
        assert!(!info.caller_frame_opaque_writes, "got {info:?}");

        // A literal target needs no call-site resolution.
        let body = lower("uplevel 1 [list set fixed 1]");
        let info = collect_upvar_targets(&body, &[]);
        assert!(
            info.uplevel_literal_writes.contains("fixed"),
            "got {info:?}"
        );

        // A target that is neither literal nor a parameter (a `foreach`
        // variable, say) is unknowable — widen.
        let body = lower("foreach v $vs { uplevel 1 [list set $v 1] }");
        let info = collect_upvar_targets(&body, &["vs".to_string()]);
        assert!(info.caller_frame_opaque_writes, "got {info:?}");
    }

    #[test]
    fn uplevel_dynamic_body_is_an_opaque_caller_frame_barrier() {
        let body = lower("uplevel 1 $script");
        let info = collect_upvar_targets(&body, &["script".to_string()]);
        assert!(info.caller_frame_opaque_writes, "got {info:?}");
        assert!(info.caller_frame_opaque_reads, "got {info:?}");
        let barrier = info.caller_frame_barrier();
        assert!(barrier.writes && barrier.reads && barrier.destroys);
    }

    #[test]
    fn uplevel_at_the_current_or_global_frame_is_not_a_caller_effect() {
        // TN — `uplevel 0 $s` re-enters the callee's own frame and
        // `uplevel #0 $s` the global one; neither touches the caller.
        // `crate::dynamic_names` raises the *callee's* own barrier for both.
        for src in ["uplevel 0 $s", "uplevel #0 $s"] {
            let body = lower(src);
            let info = collect_upvar_targets(&body, &["s".to_string()]);
            assert!(info.is_empty(), "{src}: got {info:?}");
        }
    }

    #[test]
    fn uplevel_of_a_side_effect_free_command_has_no_caller_effect() {
        // TN — `uplevel 1 [list puts hi]` writes no variable anywhere, so
        // the summary must stay clear rather than widening on the mere
        // presence of an `uplevel`.
        let body = lower("uplevel 1 [list puts hi]");
        let info = collect_upvar_targets(&body, &[]);
        assert!(info.is_empty(), "got {info:?}");
    }

    #[test]
    fn dynamic_destination_skipped() {
        let body = lower("upvar 1 caller_x $local");
        let info = collect_upvar_targets(&body, &[]);
        // Destination `$local` is dynamic — can't classify.
        assert!(info.is_empty());
    }

    #[test]
    fn upvar_inside_if_body_collected() {
        let body = lower("if {1} { upvar 1 caller_x x }");
        let info = collect_upvar_targets(&body, &[]);
        assert_eq!(info.literal_targets.get("x"), Some(&"caller_x".to_string()),);
    }

    #[test]
    fn upvar_inside_while_body_collected() {
        let body = lower("while {1} { upvar 1 caller_y y; break }");
        let info = collect_upvar_targets(&body, &[]);
        assert_eq!(info.literal_targets.get("y"), Some(&"caller_y".to_string()),);
    }

    #[test]
    fn resolve_literal_or_param_literal() {
        let mut info = UpvarInfo::default();
        info.literal_targets
            .insert("x".to_string(), "caller_x".to_string());
        let args = BTreeMap::new();
        assert_eq!(
            info.resolve_literal_or_param("x", &args),
            Some("caller_x".to_string()),
        );
    }

    #[test]
    fn resolve_literal_or_param_via_param() {
        let mut info = UpvarInfo::default();
        info.param_targets
            .insert("x".to_string(), "name".to_string());
        let mut args = BTreeMap::new();
        args.insert("name".to_string(), Some("caller_real".to_string()));
        assert_eq!(
            info.resolve_literal_or_param("x", &args),
            Some("caller_real".to_string()),
        );
    }

    #[test]
    fn resolve_literal_or_param_dynamic_param_returns_none() {
        let mut info = UpvarInfo::default();
        info.param_targets
            .insert("x".to_string(), "name".to_string());
        let mut args = BTreeMap::new();
        args.insert("name".to_string(), None); // dynamic call-site arg
        assert!(info.resolve_literal_or_param("x", &args).is_none());
    }

    #[test]
    fn resolve_literal_or_param_unknown_local_returns_none() {
        let info = UpvarInfo::default();
        let args = BTreeMap::new();
        assert!(info.resolve_literal_or_param("missing", &args).is_none());
    }

    #[test]
    fn is_empty_helper() {
        let mut info = UpvarInfo::default();
        assert!(info.is_empty());
        info.literal_targets.insert("x".into(), "y".into());
        assert!(!info.is_empty());
    }

    #[test]
    fn caller_side_defs_literal_only() {
        let mut info = UpvarInfo::default();
        info.literal_targets.insert("x".into(), "caller_x".into());
        info.literal_targets.insert("y".into(), "caller_y".into());
        let defs = info.caller_side_defs(&[], &[]);
        let mut sorted = defs.clone();
        sorted.sort();
        assert_eq!(sorted, vec!["caller_x".to_string(), "caller_y".to_string()]);
    }

    #[test]
    fn caller_side_defs_param_resolution() {
        let mut info = UpvarInfo::default();
        info.param_targets.insert("x".into(), "name".into());
        // proc foo {name body} { upvar 1 $name x ... }
        // Call: foo my_var { ... }
        let params = vec!["name".to_string(), "body".to_string()];
        let call_args = vec!["my_var".to_string(), "{set x 1}".to_string()];
        let defs = info.caller_side_defs(&call_args, &params);
        assert_eq!(defs, vec!["my_var".to_string()]);
    }

    #[test]
    fn caller_side_defs_param_dollar_normalisation() {
        let mut info = UpvarInfo::default();
        info.param_targets.insert("x".into(), "name".into());
        let params = vec!["name".to_string()];
        // call site passes `$user_var` — should normalise to `user_var`.
        let call_args = vec!["$user_var".to_string()];
        let defs = info.caller_side_defs(&call_args, &params);
        assert_eq!(defs, vec!["user_var".to_string()]);
    }

    #[test]
    fn caller_side_defs_literal_plus_param() {
        let mut info = UpvarInfo::default();
        info.literal_targets.insert("a".into(), "caller_a".into());
        info.param_targets.insert("b".into(), "n".into());
        let params = vec!["n".to_string()];
        let call_args = vec!["caller_b".to_string()];
        let defs = info.caller_side_defs(&call_args, &params);
        let mut sorted = defs.clone();
        sorted.sort();
        assert_eq!(sorted, vec!["caller_a".to_string(), "caller_b".to_string()]);
    }

    #[test]
    fn caller_side_defs_dedup() {
        let mut info = UpvarInfo::default();
        info.literal_targets.insert("a".into(), "same".into());
        info.literal_targets.insert("b".into(), "same".into());
        let defs = info.caller_side_defs(&[], &[]);
        assert_eq!(defs, vec!["same".to_string()]);
    }

    #[test]
    fn caller_side_defs_args_tail_upvar_positional() {
        // `args_tail_upvar` records the locals (in declaration order)
        // for every `upvar 1 $args local` form in the proc body.  The
        // caller-side resolver pairs each entry positionally with a
        // call-site tail argument starting at `params.len() - 1`.
        let mut info = UpvarInfo::default();
        info.args_tail_upvar.push("x".into()); // first tail arg → x
        info.args_tail_upvar.push("y".into()); // second tail arg → y
        let params = vec!["a".to_string(), "args".to_string()];
        // Call: foo first_pos tail_one tail_two tail_three
        // tail_start = params.len() - 1 = 1.
        let call_args = vec![
            "first_pos".to_string(),
            "tail_one".to_string(),
            "tail_two".to_string(),
            "tail_three".to_string(),
        ];
        let defs = info.caller_side_defs(&call_args, &params);
        // Two declarations pick the first two tail args.
        assert_eq!(defs, vec!["tail_one".to_string(), "tail_two".to_string()]);
    }

    #[test]
    fn caller_side_defs_missing_param_skipped() {
        let mut info = UpvarInfo::default();
        info.param_targets
            .insert("x".into(), "missing_param".into());
        let params = vec!["other".to_string()];
        let call_args = vec!["v".to_string()];
        let defs = info.caller_side_defs(&call_args, &params);
        // `missing_param` isn't in params — skip silently.
        assert!(defs.is_empty());
    }

    #[test]
    fn caller_side_defs_dynamic_arg_normalises_to_empty_skipped() {
        let mut info = UpvarInfo::default();
        info.param_targets.insert("x".into(), "n".into());
        let params = vec!["n".to_string()];
        // `$` alone normalises to empty — should be skipped.
        let call_args = vec!["$".to_string()];
        let defs = info.caller_side_defs(&call_args, &params);
        assert!(defs.is_empty());
    }
}
