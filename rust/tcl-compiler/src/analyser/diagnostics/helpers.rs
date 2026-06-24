//! Shared helpers for the analyser diagnostics families.
//!
//! Free functions and small types used by more than one diagnostic family
//! (or by both the per-function dispatcher in the module root and a family):
//! source-slice extraction, dotted-quad scanning shared between the
//! subnet-mask and invalid-IP checks, the substitution / braced-word
//! predicates shared by the usage and security checks, the
//! defined-variable / existence-guard / globals-written collectors consumed
//! by the read-before-set machinery, and the [`UndefSuppression`] context
//! plus its phi-undef index that the dataflow read-before-set emitters
//! consult.

use std::collections::HashSet;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::cfg::BlockId;

/// Find a case-insensitive match for `variable` in `defined_vars`.
///
/// The source text covered by `span`, or `None` when the span is out
/// of bounds / not on char boundaries.
pub(super) fn source_slice(source: &str, span: tcl_lexer::Span) -> Option<String> {
    let start = span.start() as usize;
    let end = span.end() as usize;
    if start <= end && end <= source.len() {
        source.get(start..end).map(str::to_owned)
    } else {
        None
    }
}

/// A `\w` byte: ASCII alphanumeric or underscore (word boundary basis).
pub(super) fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// One dotted-quad match found in a value: the four octet substrings and
/// the byte offset where it begins (for context checks like a preceding
/// `/`).
pub(super) struct DottedQuad<'a> {
    pub(super) octets: [&'a str; 4],
    pub(super) start: usize,
    /// Byte offset just past the final octet (the regex `m.end()`).
    pub(super) end: usize,
}

/// Find every `\b\d{1,N}.\d{1,N}.\d{1,N}.\d{1,N}\b` dotted quad in
/// `text` (non-overlapping, left-to-right), replacing the regex scan.
/// `max_digits` caps each octet's digit count (`3` for the subnet-mask
/// check, `4` for the invalid-IP one).  Each octet starts at a word
/// boundary, so a longer digit run (a 4th/5th digit) simply fails to
/// align with the following `.` and is skipped — matching the regex.
pub(super) fn find_dotted_quads(text: &str, max_digits: usize) -> Vec<DottedQuad<'_>> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let boundary_before = i == 0 || !is_word_byte(bytes[i - 1]);
        if boundary_before && let Some((octets, end)) = match_dotted_quad(text, i, max_digits) {
            out.push(DottedQuad {
                octets,
                start: i,
                end,
            });
            i = end;
            continue;
        }
        i += 1;
    }
    out
}

/// Match a dotted quad starting at byte `start` (a word boundary), each
/// octet `1..=max_digits` digits separated by `.`, requiring a trailing
/// word boundary.  Returns the octet substrings and the end offset.
fn match_dotted_quad(text: &str, start: usize, max_digits: usize) -> Option<([&str; 4], usize)> {
    let bytes = text.as_bytes();
    let mut pos = start;
    let mut octets: [&str; 4] = [""; 4];
    for (k, slot) in octets.iter_mut().enumerate() {
        let run_start = pos;
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            pos += 1;
        }
        let len = pos - run_start;
        if len == 0 || len > max_digits {
            return None;
        }
        *slot = &text[run_start..pos];
        if k < 3 {
            if bytes.get(pos) != Some(&b'.') {
                return None;
            }
            pos += 1; // consume the dot
        }
    }
    // Trailing `\b`: end of string or a non-word byte.
    if pos < bytes.len() && is_word_byte(bytes[pos]) {
        return None;
    }
    Some((octets, pos))
}

/// True when `tok` is a brace-quoted word (`{…}`, a `Str` token).
pub(super) fn is_braced_word(tok: &tcl_lexer::Token) -> bool {
    tok.kind == tcl_lexer::TokenType::Str
}

/// True when `text` carries a substitution (`$` / `[`) or `tok` is a
/// `Var` / `Cmd` token.
pub(super) fn has_substitution(text: &str, tok: &tcl_lexer::Token) -> bool {
    text.contains('$')
        || text.contains('[')
        || matches!(
            tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        )
}

/// An identifier-continuation byte: ASCII alphanumeric, `_`, or `:` (the
/// namespace-separator byte).
pub(super) fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b':'
}

/// Collect `(var, guard_block)` pairs for every
/// `[info exists X]` / `[array exists X]` branch condition in `fu`.
/// A read of `var` in any block dominated by `guard_block` is guarded
/// (X provably exists).  A positive query guards the true target; a
/// `![info exists X]` query guards the false target.
pub(super) fn collect_existence_guards(
    fu: &crate::compilation_unit::FunctionUnit,
) -> Vec<(String, BlockId)> {
    use crate::cfg::Terminator;
    let mut guards = Vec::new();
    for block in fu.cfg.blocks.values() {
        if let Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            ..
        }) = &block.terminator
            && let Some((var, negated)) = crate::expr_ast::existence_query_var(condition)
        {
            let target = if negated { *false_target } else { *true_target };
            guards.push((var, target));
        }
    }
    guards
}

/// True when `block` is dominated by `dom` (walking the SSA immediate
/// dominator chain; a block dominates itself).
pub(super) fn block_dominated_by(
    ssa: &crate::ssa::SsaFunction,
    block: BlockId,
    dom: BlockId,
) -> bool {
    let mut cur = block;
    loop {
        if cur == dom {
            return true;
        }
        match ssa.idom.get(&cur) {
            Some(Some(parent)) => cur = *parent,
            _ => return false,
        }
    }
}

/// Names whose whole binding is removed by an `unset` call.  Conservative:
/// only a **literal** bare name kills
/// (a dynamic `unset $name` targets the variable *named by* `$name`, not
/// `name` itself — yet the IR records `name` in the call's defs, so a
/// `$`-stripping harvest would wrongly mark it killed).  Per-element
/// `unset x(k)` drops one array element, not the binding, so it is
/// skipped too.
fn whole_unset_names(args: &[String]) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    let mut i = 0;
    while i < args.len() && args[i].starts_with('-') {
        let is_dashdash = args[i] == "--";
        i += 1;
        if is_dashdash {
            break;
        }
    }
    for raw in &args[i..] {
        // Literal bare names only — skip dynamic (`$`/`${…}`/`[…]`) and
        // element-subscripted (`x(k)`) targets.
        if raw.contains('$') || raw.contains('[') || raw.contains('(') {
            continue;
        }
        let base = crate::naming::normalise_var_name(raw);
        if !base.is_empty() {
            out.insert(base.to_string());
        }
    }
    out
}

/// Phi-from-undef trace.  A use's SSA version > 0 normally proves a prior
/// definition reached it, but a phi result whose reachable incomings
/// include an undefined (version-0) or `unset`-killed origin only reaches
/// on a subset of paths — the others read an unset variable.  Returns
/// true when `(name, version)` can be undefined on some reachable path.
///
/// Version 0 is
/// the undef origin; an `unset`-killed version is undef; a non-phi
/// (concrete) definition is never undef; a phi is undef if any of its
/// reachable, non-existence-guarded incomings is undef.  Cycles
/// (loop-header phis) conservatively resolve to *not* undef on the cycle.
#[allow(clippy::too_many_arguments)]
pub(super) fn phi_can_undef(
    name: &str,
    version: crate::ssa::Version,
    phi_def: &PhiDefMap,
    phi_block: &PhiBlockMap,
    killed: &FxHashSet<(String, crate::ssa::Version)>,
    considered: &HashSet<BlockId>,
    executable_edges: &HashSet<(BlockId, BlockId)>,
    exists_guards: &[(String, BlockId)],
    ssa: &crate::ssa::SsaFunction,
    seen: &mut FxHashSet<(String, crate::ssa::Version)>,
) -> bool {
    if version == 0 {
        return true;
    }
    let key = (name.to_string(), version);
    if killed.contains(&key) {
        return true;
    }
    if seen.contains(&key) {
        // Cycle (loop-header phi): the DFS seed already accounted for the
        // entry path's contribution; treat the back-edge as not-undef to
        // avoid every loop-header phi self-triggering.
        return false;
    }
    let Some(phi) = phi_def.get(&key) else {
        // Concrete (non-phi) definition reached this version — safe.
        return false;
    };
    // The block this phi lives in — the destination of each incoming edge.
    let this_block = phi_block.get(&key).copied();
    seen.insert(key.clone());
    let mut result = false;
    for (&pred, &incoming_ver) in &phi.incoming {
        if !considered.contains(&pred) {
            continue;
        }
        // A phi has one operand per predecessor *edge*; an operand arriving on
        // a non-executable edge (SCCP proved the edge dead — e.g. the
        // `cond → exit` edge of `while 1`, which a `break` makes the loop's
        // only real exit) can never actually be read, so its version-0 origin
        // must not count as a possible undef.  This filter is only applied
        // when SCCP edge info is available (a non-empty set).
        if let Some(blk) = this_block
            && !executable_edges.is_empty()
            && !executable_edges.contains(&(pred, blk))
        {
            continue;
        }
        // A dominating existence guard proves the variable is defined at
        // the predecessor; that incoming cannot be undef regardless of
        // its SSA version.
        if exists_guards
            .iter()
            .any(|(gv, gblk)| gv == name && block_dominated_by(ssa, pred, *gblk))
        {
            continue;
        }
        if phi_can_undef(
            name,
            incoming_ver,
            phi_def,
            phi_block,
            killed,
            considered,
            executable_edges,
            exists_guards,
            ssa,
            seen,
        ) {
            result = true;
            break;
        }
    }
    seen.remove(&key);
    result
}

/// `(name, version) → Phi` index used by [`phi_can_undef`].
pub(super) type PhiDefMap = FxHashMap<(String, crate::ssa::Version), crate::ssa::Phi>;

/// `(name, version) → defining block` index, so [`phi_can_undef`] can test
/// each incoming `(pred, phi_block)` edge against the SCCP-executable edge set.
pub(super) type PhiBlockMap = FxHashMap<(String, crate::ssa::Version), BlockId>;

/// Build the `(name, version) → Phi` index, the `(name, version) → block`
/// index, and the set of `unset`-killed versions for [`phi_can_undef`],
/// restricted to `considered` (executable) blocks.
pub(super) fn build_phi_undef_index(
    ssa: &crate::ssa::SsaFunction,
    considered: &HashSet<BlockId>,
) -> (
    PhiDefMap,
    PhiBlockMap,
    FxHashSet<(String, crate::ssa::Version)>,
) {
    use crate::ir::Statement;
    let mut phi_def: PhiDefMap = FxHashMap::default();
    let mut phi_block: PhiBlockMap = FxHashMap::default();
    let mut killed: FxHashSet<(String, crate::ssa::Version)> = FxHashSet::default();
    for &bn in considered {
        let Some(sblock) = ssa.blocks.get(&bn) else {
            continue;
        };
        for phi in &sblock.phis {
            phi_def.insert((phi.name.clone(), phi.version), phi.clone());
            phi_block.insert((phi.name.clone(), phi.version), bn);
        }
        for s in &sblock.statements {
            let Statement::Call {
                command,
                canonical_command,
                args,
                ..
            } = &s.statement
            else {
                continue;
            };
            let is_unset = canonical_command.as_deref() == Some("::unset") || command == "unset";
            if !is_unset {
                continue;
            }
            let whole = whole_unset_names(args);
            for (def_name, def_ver) in &s.defs {
                if whole.contains(def_name) {
                    killed.insert((def_name.clone(), *def_ver));
                }
            }
        }
    }
    (phi_def, phi_block, killed)
}

/// Name-level suppression context for the `return`-value phi-from-undef W210
/// pass, harvested from `dict with` / `dict update` and qualified `variable`
/// declarations.
#[derive(Default)]
pub(super) struct UndefSuppression {
    /// A `dict with` / `dict update` is present (enables the key-aware gate).
    has_dict_with: bool,
    /// At least one dict-with target's value shape is statically unknown.
    dict_with_any_unknown: bool,
    /// Keys provably unpacked by some known-literal dict-with target.
    dict_with_known_keys: HashSet<String>,
    /// The dict-with target variable names themselves.
    dict_vars: HashSet<String>,
    /// Names with a concrete (version > 0) statement/phi definition.
    explicitly_defined: HashSet<String>,
    /// Local-alias tails declared by a qualified `variable ns::tail`.
    alias_tails: FxHashSet<String>,
    /// Names written by a command substitution buried inside an `expr`
    /// argument (`set e [expr {[catch {…} tmp] || $tmp}]` writes `tmp` during
    /// expr evaluation).  The `[…]` is opaque to SSA def tracking, so a later
    /// `$tmp` read in the same expression looks read-before-set.  Name-level,
    /// suppress-only.
    cmd_sub_writes: FxHashSet<String>,
    /// Locals aliased to a *dynamic* upvar target (`upvar 1 $name local`).
    /// The alias may resolve to a non-existent caller variable, so the local
    /// is possibly-unset: read-before-set must *override* the scope-alias
    /// suppression and fire on an unconditional read (an `[info exists]` guard
    /// still suppresses it per-use).
    pub(super) dynamic_upvar_locals: HashSet<String>,
    /// `(name, version)` pairs killed by an `unset` — undef at their reads,
    /// so a direct read of one is read-before-set just like a version-0
    /// origin.
    pub(super) killed: FxHashSet<(String, crate::ssa::Version)>,
    /// Phi versions that can be undefined on some executable path
    /// (a one-branch `set y 1` merge, or a try-handler merge). A statement
    /// read of one is read-before-set; the def-use pass can't express this
    /// because the read targets the *phi* version, not a version-0 origin.
    pub(super) can_undef: FxHashSet<(String, crate::ssa::Version)>,
}

impl UndefSuppression {
    /// True when a read of `name` is suppressed by an alias declaration or a
    /// `dict with` / `dict update` unpack.  Blanket variant: an unknown-shape
    /// dict suppresses every non-concrete name (the conservative
    /// "might-have-the-key" stance, used where no truth source can confirm
    /// the dict is empty — e.g. a `return` after a `dict with` on a param).
    pub(super) fn suppresses(&self, name: &str) -> bool {
        self.suppresses_strict(name)
            || (self.has_dict_with
                && self.dict_with_any_unknown
                && !self.explicitly_defined.contains(name))
    }

    /// Like [`Self::suppresses`] but **without** the unknown-shape blanket —
    /// only alias tails, dict vars, and *provably-unpacked* keys suppress.
    /// Used on statement reads inside a `dict with` body, where an
    /// unknown-shape dict (e.g. an interprocedurally-empty literal Rust's
    /// SCCP cannot yet resolve) must still fire so a genuine missing-key read
    /// is not hidden.
    pub(super) fn suppresses_strict(&self, name: &str) -> bool {
        if self.alias_tails.contains(name)
            || self.dict_vars.contains(name)
            || self.cmd_sub_writes.contains(name)
        {
            return true;
        }
        self.has_dict_with
            && !self.explicitly_defined.contains(name)
            && self.dict_with_known_keys.contains(name)
    }
}

/// Build the [`UndefSuppression`] context over `considered` blocks.
/// Names written by a command substitution buried inside an `expr` argument.
/// `set e [expr {[catch {…} tmp] || $tmp}]` writes `tmp` during expr
/// evaluation; the `set x [expr {E}]` form lowers to `AssignExpr`, so the
/// condition-out-var extractor over its expr recovers those writes.
/// Name-level, suppress-only.
fn collect_expr_cmd_sub_writes(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
) -> FxHashSet<String> {
    use crate::ir::Statement;
    let mut out = FxHashSet::default();
    for &bn in considered {
        let Some(block) = fu.cfg.blocks.get(&bn) else {
            continue;
        };
        for stmt in &block.statements {
            if let Statement::AssignExpr { expr, .. } = stmt {
                out.extend(crate::ir_helpers::condition_command_out_vars(expr));
            }
        }
    }
    out
}

/// `dict with` / `dict update` key-aware suppression: record the dict-var
/// names and, when the dict value is a same-block literal (or an
/// interprocedurally-propagated SCCP const), its keys.  A value that resolves
/// to neither marks the dict shape unknown.
fn harvest_dict_with_suppression(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    s: &mut UndefSuppression,
) {
    use crate::ir::Statement;
    for &bn in considered {
        let Some(block) = fu.cfg.blocks.get(&bn) else {
            continue;
        };
        for (idx, stmt) in block.statements.iter().enumerate() {
            let (Statement::Barrier { command, args, .. } | Statement::Call { command, args, .. }) =
                stmt
            else {
                continue;
            };
            let is_dict = command == "dict" || stmt.canonical_command_or_source() == "::dict";
            if !is_dict {
                continue;
            }
            if args.first().map(String::as_str) != Some("with")
                && args.first().map(String::as_str) != Some("update")
            {
                continue;
            }
            s.has_dict_with = true;
            let Some(dict_var) = args.get(1) else {
                s.dict_with_any_unknown = true;
                continue;
            };
            let dvar = crate::naming::normalise_var_name(dict_var).to_string();
            if dvar.is_empty() {
                s.dict_with_any_unknown = true;
                continue;
            }
            s.dict_vars.insert(dvar.clone());
            // Resolve the dict's value to harvest its keys.  Prefer the SCCP
            // CONST of the SPECIFIC version read by this dict-with (so
            // interprocedurally-propagated literals — a caller passing `{}` —
            // are honoured), falling back to a same-block literal `set`.  A
            // known value (even empty) harvests its keys; only a value that
            // resolves to neither marks the dict shape unknown.
            let mut literal: Option<String> = None;
            if let Some(sb) = fu.ssa.blocks.get(&bn)
                && let Some(ver) = sb
                    .statements
                    .get(idx)
                    .and_then(|s| s.uses.get(&dvar).copied())
                && let Some(crate::analyses::LatticeValue::Const(
                    crate::analyses::ConstValue::String(v),
                )) = fu.sccp.values.get(&(dvar.clone(), ver))
            {
                literal = Some(v.clone());
            }
            if literal.is_none() {
                for prev in (0..idx).rev() {
                    match &block.statements[prev] {
                        Statement::AssignConst { name, value, .. }
                            if crate::naming::normalise_var_name(name) == dvar =>
                        {
                            literal = Some(value.clone());
                            break;
                        }
                        // A barrier between us and the literal invalidates it.
                        Statement::Barrier { .. } => break,
                        _ => {}
                    }
                }
            }
            match literal {
                Some(v) => {
                    for (i, key) in crate::tcl_expr_eval::split_tcl_list(&v)
                        .into_iter()
                        .enumerate()
                    {
                        if i % 2 == 0 {
                            s.dict_with_known_keys.insert(key);
                        }
                    }
                }
                None => s.dict_with_any_unknown = true,
            }
        }
    }
}

pub(super) fn build_undef_suppression(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
) -> UndefSuppression {
    let (phi_def, phi_block, killed) = build_phi_undef_index(&fu.ssa, considered);
    // Phi versions that can reach an undef origin on some executable path —
    // a statement read of one is read-before-set. The per-use existence
    // guard + suppression set still apply in the emitter loop.
    let exists_guards = collect_existence_guards(fu);
    let mut can_undef: FxHashSet<(String, crate::ssa::Version)> = FxHashSet::default();
    for key in phi_def.keys() {
        let mut seen = FxHashSet::default();
        if phi_can_undef(
            &key.0,
            key.1,
            &phi_def,
            &phi_block,
            &killed,
            considered,
            &fu.sccp.executable_edges,
            &exists_guards,
            &fu.ssa,
            &mut seen,
        ) {
            can_undef.insert(key.clone());
        }
    }
    let mut s = UndefSuppression {
        cmd_sub_writes: collect_expr_cmd_sub_writes(fu, considered),
        dynamic_upvar_locals: crate::optimiser::elimination::scan_dynamic_upvar_locals(&fu.cfg),
        killed,
        can_undef,
        ..Default::default()
    };
    harvest_dict_with_suppression(fu, considered, &mut s);

    // Names with a concrete (version > 0) statement or phi definition — a
    // dict-with scope never suppresses these (they are genuinely set).
    if s.has_dict_with {
        for &bn in considered {
            let Some(sb) = fu.ssa.blocks.get(&bn) else {
                continue;
            };
            for st in &sb.statements {
                for (n, v) in &st.defs {
                    if *v > 0 {
                        s.explicitly_defined.insert(n.clone());
                    }
                }
            }
            for phi in &sb.phis {
                if phi.version > 0 {
                    s.explicitly_defined.insert(phi.name.clone());
                }
            }
        }
    }

    s.alias_tails = collect_qualified_variable_alias_tails(fu, considered);
    s
}

/// Local-alias tail names declared by a *qualified* `variable`
/// (`variable ns::tail` / `variable ${name}::tail`): the bare tail read
/// resolves to the namespace var, not an unset local.
fn collect_qualified_variable_alias_tails(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
) -> FxHashSet<String> {
    use crate::ir::Statement;
    let mut tails = FxHashSet::default();
    for &bn in considered {
        let Some(block) = fu.cfg.blocks.get(&bn) else {
            continue;
        };
        for stmt in &block.statements {
            let (Statement::Barrier { command, args, .. } | Statement::Call { command, args, .. }) =
                stmt
            else {
                continue;
            };
            if command != "variable" && stmt.canonical_command_or_source() != "::variable" {
                continue;
            }
            // `variable` alternates (name, value?) pairs — names at even args.
            let mut i = 0;
            while i < args.len() {
                let text = &args[i];
                if text.contains("::") {
                    let tail = text.rsplit("::").next().unwrap_or(text);
                    let (base, _) = crate::naming::split_array_name(tail);
                    if !base.is_empty()
                        && !base.contains('$')
                        && !base.contains('[')
                        && !base.contains('{')
                    {
                        tails.insert(crate::naming::normalise_var_name(base).to_string());
                    }
                }
                i += 2;
            }
        }
    }
    tails
}

/// Collect every variable name defined anywhere in `cfg`.
///
/// Walks every block and pulls
/// the `defs` field off each [`crate::ir::Statement`] that has
/// one (assignments, ``incr``, ``Call`` statements with explicit
/// defs).  Used for the "did you mean…?" case-mismatch
/// suggestion in W210 / W211 / W220 messages.
pub(super) fn collect_defined_vars(cfg: &crate::cfg::Function) -> HashSet<String> {
    use crate::ir::Statement;
    let mut names: HashSet<String> = HashSet::new();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::AssignConst { name, .. }
                | Statement::AssignExpr { name, .. }
                | Statement::AssignValue { name, .. }
                | Statement::Incr { name, .. } => {
                    let normalised = crate::naming::normalise_var_name(name);
                    if !normalised.is_empty() {
                        names.insert(normalised.to_string());
                    }
                }
                Statement::Call { defs, .. } => {
                    for def in defs {
                        names.insert(def.clone());
                    }
                }
                _ => {}
            }
        }
    }
    names
}

/// Compute the set of global variable names that any procedure
/// in `cu` writes.
///
/// A global write happens when a proc either:
///
/// 1. assigns to a fully-qualified name (``::var``), or
/// 2. declares ``global var`` and then assigns to ``var`` in the
///    same proc body.
///
/// The result is the union of (1) and the intersection of
/// global aliases × locally-written names (case (2)).  Used at
/// top-level to suppress W210 for globals a helper proc may
/// populate before the top-level read.
///
/// There is no ``CommandRegistry::is_destroys_variable`` yet, so
/// commands like ``unset`` aren't filtered out of the "writes" set.
/// That makes the suppression slightly more permissive (more
/// vars marked "written-by-procs" → more W210 suppressions).
/// Safe-on-correctness — the alternative is false positives
/// on real RBS sites.  When the registry gains
/// ``destroys_variable``, add the filter here.
pub(super) fn globals_written_by_procs(
    cu: &crate::compilation_unit::CompilationUnit,
) -> HashSet<String> {
    use crate::ir::Statement;
    let mut result: HashSet<String> = HashSet::new();
    for fu in cu.procedures.values() {
        let mut global_aliases: FxHashSet<String> = FxHashSet::default();
        let mut written: FxHashSet<String> = FxHashSet::default();
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                let names: Vec<&String> = match stmt {
                    Statement::Call { command, defs, .. } => {
                        if command == "global" {
                            for d in defs {
                                global_aliases.insert(d.clone());
                            }
                            continue;
                        }
                        if matches!(command.as_str(), "variable" | "upvar") {
                            continue;
                        }
                        defs.iter().collect()
                    }
                    Statement::AssignConst { name, .. }
                    | Statement::AssignExpr { name, .. }
                    | Statement::AssignValue { name, .. }
                    | Statement::Incr { name, .. } => vec![name],
                    _ => continue,
                };
                for name in names {
                    if let Some(bare) = name.strip_prefix("::") {
                        let bare = bare.trim_start_matches(':');
                        if !bare.is_empty() {
                            result.insert(bare.to_string());
                        }
                    } else {
                        written.insert(name.clone());
                    }
                }
            }
        }
        for n in global_aliases.intersection(&written) {
            result.insert(n.clone());
        }
    }
    result
}
