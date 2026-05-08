//! Per-proc `upvar` declaration summary (SYNC-JUN-CFG-upvar-info).
//!
//! Mirrors Python's `_UpvarInfo` dataclass and `_collect_upvar_targets`
//! pass in `core/compiler/cfg.py` (commit `d5ac467c`).  The proc-summary
//! integration point pre-populates caller-side `defs` from
//! [`UpvarInfo::literal_targets`] / [`UpvarInfo::param_targets`] /
//! [`UpvarInfo::args_tail_upvar`] so callers see the local-side names
//! the callee is going to write at runtime.
//!
//! ## Three shapes
//!
//! `upvar` declarations come in three classifiable shapes:
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
//! ## Status
//!
//! This is the **port** half of the chunk — the data structures, the
//! `_collect_upvar_targets` pass, and unit tests covering each shape.
//! The caller-side `defs` pre-population at the proc-summary
//! integration point is the **wiring** half, deferred to the
//! `SYNC-JUN-CFG-uplevel-literal-set` follow-up which Python tracks at
//! `cfg.py:441` / `:589` / `:602` / `:621` / `:633` / `:1253`.  That
//! follow-up is mechanical once this module exists — it's the same
//! shape consumers already use for `param_targets` and the dispatch
//! plumbing is structural.
//!
//! Until the wiring lands, [`UpvarInfo`] is a self-contained data
//! point that downstream consumers (interprocedural analysis,
//! optimiser passes, the LSP's hover provider) can opt into without
//! the `cfg_builder` needing further changes.

use std::collections::BTreeMap;

use crate::ir::{Script, Statement};

/// Per-proc summary of every `upvar` declaration in the body.
///
/// Mirrors Python's `_UpvarInfo` dataclass.  `literal_targets` /
/// `param_targets` / `args_tail_upvar` are mutually exclusive — each
/// `(source, local)` pair lands in exactly one bucket.  Maps key on
/// the *local* name (the alias target) so caller-side resolution can
/// look up "which caller-frame name does my local `x` alias?" in a
/// single hash hit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpvarInfo {
    /// `local_name -> caller_literal_name` for declarations whose
    /// source word is a plain string (no `$` / `[`).  Mirrors
    /// `_UpvarInfo.literal_targets`.
    pub literal_targets: BTreeMap<String, String>,
    /// `local_name -> param_name` for declarations whose source word
    /// is `$<param>` and `<param>` is a proc parameter.  The
    /// caller-side resolver substitutes the actual argument passed
    /// for `<param>` to produce the caller-frame name.  Mirrors
    /// `_UpvarInfo.param_targets`.
    pub param_targets: BTreeMap<String, String>,
    /// `local_name`s whose source is `$args` (the trailing-vararg
    /// param).  Each call-site arg in the tail position is aliased
    /// to one of these locals; the per-local pairing is positional,
    /// the order here matches the order of declarations in the
    /// proc body.  Mirrors `_UpvarInfo.args_tail_upvar`.
    pub args_tail_upvar: Vec<String>,
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
/// special characters.  Mirrors the Python check that gates the
/// literal-vs-dynamic split in `_collect_upvar_targets`.
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

/// `upvar ?level? src1 dst1 ?src2 dst2 ...?` — return the
/// `(src, dst)` pairs.  Skips the optional level word (a literal
/// integer with optional `#` prefix).
fn upvar_pairs(args: &[String]) -> Vec<(&str, &str)> {
    if args.is_empty() {
        return Vec::new();
    }
    let head = &args[0];
    let no_dash = head.trim_start_matches('-');
    let is_level_literal = (!no_dash.is_empty() && no_dash.chars().all(|c| c.is_ascii_digit()))
        || (head.starts_with('#') && head[1..].chars().all(|c| c.is_ascii_digit()));
    let start = usize::from(is_level_literal);
    let mut pairs = Vec::new();
    let mut i = start;
    while i + 1 < args.len() {
        pairs.push((args[i].as_str(), args[i + 1].as_str()));
        i += 2;
    }
    pairs
}

/// Collect upvar targets from a proc body.
///
/// `params` is the proc's parameter list (used to gate
/// `param_targets`).  `body` is the lowered IR body.  Walks every
/// `Statement::Call` and `Statement::Barrier` for `upvar`
/// invocations, classifies each pair into one of the three buckets,
/// and accumulates the result in [`UpvarInfo`].
///
/// Mirrors Python's `_collect_upvar_targets(proc)` in `cfg.py`.
#[must_use]
pub fn collect_upvar_targets(body: &Script, params: &[String]) -> UpvarInfo {
    let mut info = UpvarInfo::default();
    walk_script(&body.statements, params, &mut info);
    info
}

fn walk_script(stmts: &[Statement], params: &[String], info: &mut UpvarInfo) {
    for stmt in stmts {
        walk_stmt(stmt, params, info);
    }
}

fn walk_stmt(stmt: &Statement, params: &[String], info: &mut UpvarInfo) {
    match stmt {
        Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. } => {
            if command == "upvar" {
                record_upvar_call(args, params, info);
            }
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                walk_script(&c.body.statements, params, info);
            }
            if let Some(e) = else_body {
                walk_script(&e.statements, params, info);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_script(&init.statements, params, info);
            walk_script(&next.statements, params, info);
            walk_script(&body.statements, params, info);
        }
        Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Block { body, .. }
        | Statement::UpFrame { body, .. } => walk_script(&body.statements, params, info),
        Statement::Switch {
            arms,
            default_body,
            ..
        } => {
            for arm in arms {
                if let Some(b) = &arm.body {
                    walk_script(&b.statements, params, info);
                }
            }
            if let Some(b) = default_body {
                walk_script(&b.statements, params, info);
            }
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(&body.statements, params, info);
            for h in handlers {
                walk_script(&h.body.statements, params, info);
            }
            if let Some(f) = finally_body {
                walk_script(&f.statements, params, info);
            }
        }
        _ => {}
    }
}

fn record_upvar_call(args: &[String], params: &[String], info: &mut UpvarInfo) {
    for (src, dst) in upvar_pairs(args) {
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
            }
            // else: $foo where foo isn't a param — caller can't
            // resolve, skip.
        }
        // else: dynamic source we can't classify.
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
        assert_eq!(
            info.literal_targets.get("x"),
            Some(&"caller_x".to_string()),
        );
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
        // $foo is not a param (only `bar` is); not classifiable.
        assert!(info.param_targets.is_empty());
        assert!(info.literal_targets.is_empty());
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
        assert_eq!(
            info.literal_targets.get("x"),
            Some(&"caller_x".to_string()),
        );
    }

    #[test]
    fn level_hash_zero_still_classified() {
        let body = lower("upvar #0 g local_g");
        let info = collect_upvar_targets(&body, &[]);
        assert_eq!(
            info.literal_targets.get("local_g"),
            Some(&"g".to_string()),
        );
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
        assert_eq!(
            info.literal_targets.get("x"),
            Some(&"caller_x".to_string()),
        );
    }

    #[test]
    fn upvar_inside_while_body_collected() {
        let body = lower("while {1} { upvar 1 caller_y y; break }");
        let info = collect_upvar_targets(&body, &[]);
        assert_eq!(
            info.literal_targets.get("y"),
            Some(&"caller_y".to_string()),
        );
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
}
