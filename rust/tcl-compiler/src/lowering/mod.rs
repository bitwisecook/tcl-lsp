//! Lower Tcl source to structured analysis IR.
//!
//! Translates a flat token stream (via the segmenter) into the tree of
//! IR nodes defined in [`ir`]. Each Tcl command is pattern-matched by
//! name and converted to a typed IR statement.
//!
//! Ports `core/compiler/lowering.py`.

use std::collections::HashMap;

use tcl_lexer::TokenType;
use tcl_registry::{ArgRole, CommandRegistry};

use crate::alias::{detect_interp_alias, resolve_alias, CommandAliasMap};
use crate::ir::{CommandTokens, Module, Procedure, Script, Statement};
use crate::lowering_hooks::{try_lower_hook, ArgTokenKind, LoweringCommand};
use crate::naming::{normalise_qualified_name, normalise_var_name};
use crate::segmenter::{segment_commands, segment_commands_with_offset, SegmentedCommand};

mod structured;

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
    if child.starts_with("::") {
        return normalise_qualified_name(child);
    }
    if parent == "::" {
        return normalise_qualified_name(&format!("::{child}"));
    }
    normalise_qualified_name(&format!("{parent}::{child}"))
}

/// Qualify a procedure name relative to a namespace.
fn qualify_proc_name(namespace: &str, proc_name: &str) -> String {
    if proc_name.starts_with("::") {
        return normalise_qualified_name(proc_name);
    }
    if namespace == "::" {
        return normalise_qualified_name(&format!("::{proc_name}"));
    }
    normalise_qualified_name(&format!("{namespace}::{proc_name}"))
}

/// Parse a Tcl parameter list into parameter names.
fn parse_param_names(param_str: &str) -> Vec<String> {
    let mut params = Vec::new();
    let text = param_str.trim();
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Skip whitespace.
        while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        if bytes[i] == b'{' {
            // Braced parameter — find matching close brace.
            let mut level = 1i32;
            i += 1;
            let start = i;
            while i < bytes.len() && level > 0 {
                match bytes[i] {
                    b'{' => level += 1,
                    b'}' => level -= 1,
                    _ => {}
                }
                i += 1;
            }
            let inner = &text[start..i.saturating_sub(1)].trim();
            if !inner.is_empty() {
                if let Some(name) = inner.split_whitespace().next() {
                    params.push(name.to_owned());
                }
            }
        } else {
            // Bare word parameter.
            let start = i;
            while i < bytes.len() && !matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
                i += 1;
            }
            let word = &text[start..i];
            if !word.is_empty() {
                params.push(word.to_owned());
            }
        }
    }
    params
}

/// Parse the level argument of an `uplevel` call into a frame
/// shift. Accepts the canonical positive-integer form (`uplevel 1
/// body`, `uplevel 3 body`) and the global form (`#0` / `#N`),
/// returning `None` when the argument is dynamic (`$lvl`, `[expr
/// {...}]`) or otherwise unparseable.
///
/// The returned shift is normalised so callers can decide whether to
/// route the call through [`Statement::UpFrame`] (positive shifts)
/// or fall back to a barrier.
fn parse_uplevel_level(text: &str) -> Option<i32> {
    if let Some(rest) = text.strip_prefix('#') {
        return rest.parse::<i32>().ok().map(|n| -n);
    }
    text.parse::<i32>().ok()
}

/// The lowering engine — accumulates procedures and IR statements.
pub struct Lowerer<'r> {
    /// Output module being built.
    pub module: Module,
    /// Command alias table built during lowering.
    aliases: CommandAliasMap,
    /// Event handler occurrence counts (for `when` numbering).
    when_counts: HashMap<String, u32>,
    /// Whether we're inside a `namespace eval` body.
    in_namespace_eval: bool,
    /// Command registry for arg-role queries.
    registry: &'r CommandRegistry,
}

impl<'r> Lowerer<'r> {
    /// Create a new lowerer.
    #[must_use]
    pub fn new(registry: &'r CommandRegistry) -> Self {
        Self {
            module: Module::default(),
            aliases: CommandAliasMap::new(),
            when_counts: HashMap::new(),
            in_namespace_eval: false,
            registry,
        }
    }

    /// Lower a complete source string to an IR module.
    pub fn lower(&mut self, source: &str) -> &Module {
        self.module.top_level = self.lower_script(source, "::");
        &self.module
    }

    /// Lower a source string to an IR script.
    fn lower_script(&mut self, source: &str, namespace: &str) -> Script {
        let commands = segment_commands(source);
        Script::from_statements(self.lower_segmented(&commands, namespace))
    }

    /// Lower a body argument (inside braces/brackets) to an IR script.
    fn lower_body(&mut self, text: &str, base_offset: u32, namespace: &str) -> Script {
        let commands = segment_commands_with_offset(text, base_offset);
        Script::from_statements(self.lower_segmented(&commands, namespace))
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
                    args: vec![],
                    tokens: None,
                });
                continue;
            }
            if let Some(stmt) = self.lower_command(seg, namespace) {
                stmts.push(stmt);
            }
        }
        stmts
    }

    /// Build a `CommandTokens` snapshot from a segmented command.
    fn cmd_tokens(seg: &SegmentedCommand) -> CommandTokens {
        CommandTokens {
            argv: seg.argv.iter().map(|t| t.span).collect(),
            argv_texts: seg.texts.clone(),
            single_token_word: seg.single_token_word.clone(),
            all_tokens: seg.all_tokens.iter().map(|t| t.span).collect(),
            expand_word: seg.expand_word.clone(),
        }
    }

    /// Extract arg token kinds for the lowering hooks.
    fn arg_kinds(seg: &SegmentedCommand) -> Vec<ArgTokenKind> {
        seg.argv
            .iter()
            .skip(1)
            .map(|t| arg_token_kind(t.kind))
            .collect()
    }

    /// Lower a single command.
    fn lower_command(&mut self, seg: &SegmentedCommand, namespace: &str) -> Option<Statement> {
        if seg.texts.is_empty() {
            return None;
        }

        let cmd_name = seg.name();
        let args = seg.args();

        // Detect `interp alias {} name {} target ?args?`.
        let args_owned: Vec<String> = args.to_vec();
        if let Some((qualified, target, prepended)) = detect_interp_alias(cmd_name, &args_owned) {
            self.aliases.insert(qualified, (target, prepended));
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
        };
        if let Some(stmt) = try_lower_hook(&hook_cmd, &self.aliases) {
            return Some(stmt);
        }

        // {*} expansion on structured commands → barrier.
        let structured = matches!(
            cmd_name,
            "proc"
                | "when"
                | "namespace"
                | "if"
                | "switch"
                | "for"
                | "while"
                | "foreach"
                | "foreach_in_collection"
        );
        if structured
            && seg
                .expand_word
                .as_ref()
                .is_some_and(|ew| ew.iter().any(|&e| e))
        {
            return Some(Statement::Barrier {
                span: seg.span,
                reason: format!("{cmd_name} with argument expansion"),
                command: cmd_name.into(),
                args: args.to_vec(),
                tokens: Some(Self::cmd_tokens(seg)),
            });
        }

        // Command-specific dispatch.
        match cmd_name {
            "proc" if args.len() == 3 && seg.arg_tokens().len() >= 3 => {
                Some(self.lower_proc(seg, namespace))
            }

            "when" if args.len() >= 2 && seg.arg_tokens().len() >= 2 => {
                Some(self.lower_when(seg, namespace))
            }

            "namespace" if args.len() >= 3 && args[0] == "eval" && seg.arg_tokens().len() >= 3 => {
                Some(self.lower_namespace_eval(seg, namespace))
            }

            // C34a: static-body uplevel. Match `uplevel 1 {body}`,
            // `uplevel #0 {body}`, and the canonical no-level form
            // `uplevel {body}` (level defaults to 1) when the body
            // arg is a brace-string literal token. Dynamic forms
            // (``uplevel 1 $body`` / ``uplevel $lvl {body}``) fall
            // through to the default lowering so a runtime ``Call`` /
            // ``Barrier`` carries the unresolved arguments.
            "uplevel" => Some(
                self.try_lower_uplevel_static(seg, namespace)
                    .unwrap_or_else(|| self.lower_default(seg, namespace)),
            ),

            "if" => Some(self.lower_if(seg, namespace)),
            "switch" => Some(self.lower_switch(seg, namespace)),
            "for" => Some(self.lower_for(seg, namespace)),
            "while" => Some(self.lower_while(seg, namespace)),
            "foreach" => Some(self.lower_foreach(seg, namespace, false)),
            "lmap" => Some(self.lower_foreach(seg, namespace, true)),
            "catch" => Some(self.lower_catch(seg, namespace)),
            "try" => Some(self.lower_try(seg, namespace)),
            "dict" if !args.is_empty() => Some(self.lower_dict(seg, namespace)),

            _ => Some(self.lower_default(seg, namespace)),
        }
    }

    /// Lower `proc name params body`.
    fn lower_proc(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let proc_name = &args[0];

        // Dynamic proc names can only be resolved at runtime.
        if proc_name.contains('$') || proc_name.contains('[') {
            return Statement::Barrier {
                span: seg.span,
                reason: "dynamic proc name".into(),
                command: "proc".into(),
                args: args.to_vec(),
                tokens: Some(Self::cmd_tokens(seg)),
            };
        }

        let params = parse_param_names(&args[1]);
        let qualified = qualify_proc_name(namespace, proc_name);
        let body_tok = seg.arg_tokens()[2];
        let body_text = &args[2];
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let body = self.lower_body(body_text, body_offset, namespace);

        if let std::collections::hash_map::Entry::Vacant(e) =
            self.module.procedures.entry(qualified.clone())
        {
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
        } else {
            self.module.redefined_procedures.insert(qualified);
        }

        Statement::Call {
            span: seg.span,
            command: "proc".into(),
            args: args.to_vec(),
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(Self::cmd_tokens(seg)),
        }
    }

    /// Lower `when EVENT ?priority N? body`.
    fn lower_when(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let event_name = &args[0];
        let body_idx = args.len() - 1;
        let body_tok = seg.arg_tokens()[body_idx];
        let body_text = &args[body_idx];
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let body = self.lower_body(body_text, body_offset, namespace);

        let mut base_priority: u32 = 500;
        if args.len() >= 4 && args[1] == "priority" {
            if let Ok(p) = args[2].parse::<u32>() {
                base_priority = p;
            }
        }

        let n = self
            .when_counts
            .get(event_name.as_str())
            .copied()
            .unwrap_or(0);
        *self.when_counts.entry(event_name.clone()).or_insert(0) += 1;
        let qualified = if n == 0 {
            format!("::when::{event_name}")
        } else {
            format!("::when::{event_name}#{n}")
        };

        self.module.procedures.insert(
            qualified.clone(),
            Procedure {
                name: event_name.clone(),
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
            args: args.to_vec(),
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(Self::cmd_tokens(seg)),
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
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let (frame_shift, body_tok_idx) = match args.len() {
            1 => (1_i32, 0),
            2 => (parse_uplevel_level(&args[0])?, 1),
            _ => return None,
        };
        let body_tok = arg_tokens.get(body_tok_idx)?;
        if body_tok.kind != TokenType::Str {
            return None;
        }
        let body_text = &args[body_tok_idx];
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let body = self.lower_body(body_text, body_offset, namespace);
        Some(Statement::UpFrame {
            span: seg.span,
            frame_shift,
            body,
            tokens: Some(Self::cmd_tokens(seg)),
        })
    }

    fn lower_namespace_eval(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let child_ns = join_namespace(namespace, &args[1]);
        let prev = self.in_namespace_eval;
        self.in_namespace_eval = true;
        let body_tok = seg.arg_tokens()[2];
        let body_text = &args[2];
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let _body = self.lower_body(body_text, body_offset, &child_ns);
        self.in_namespace_eval = prev;

        Statement::Barrier {
            span: seg.span,
            reason: "namespace eval".into(),
            command: "namespace".into(),
            args: args.to_vec(),
            tokens: Some(Self::cmd_tokens(seg)),
        }
    }

    /// Default lowering: generic `IRCall` with registry-based arg roles.
    fn lower_default(&self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let cmd_name = seg.name();
        let args = seg.args();

        // Resolve alias for arg role lookups.
        let mut role_cmd = cmd_name.to_owned();
        let mut role_args: Vec<String> = args.to_vec();
        let mut prepend_n: usize = 0;
        if let Some((target, prepended)) = resolve_alias(cmd_name, &self.aliases, namespace) {
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
        let var_indices =
            self.registry
                .arg_indices_for_role(&role_cmd, &role_args_ref, ArgRole::VarWrite);
        let var_read_indices =
            self.registry
                .arg_indices_for_role(&role_cmd, &role_args_ref, ArgRole::VarRead);

        if !body_indices.is_empty() {
            return Statement::Barrier {
                span: seg.span,
                reason: "unsupported body command".into(),
                command: cmd_name.into(),
                args: args.to_vec(),
                tokens: Some(Self::cmd_tokens(seg)),
            };
        }

        if !var_indices.is_empty() || !var_read_indices.is_empty() {
            let var_defs: Vec<String> = var_indices
                .iter()
                .filter_map(|&i| {
                    let real = i.checked_sub(prepend_n)?;
                    args.get(real).map(|a| normalise_var_name(a).to_owned())
                })
                .collect();
            let var_reads: Vec<String> = var_read_indices
                .iter()
                .filter_map(|&i| {
                    let real = i.checked_sub(prepend_n)?;
                    args.get(real).map(|a| normalise_var_name(a).to_owned())
                })
                .collect();
            return Statement::Call {
                span: seg.span,
                command: cmd_name.into(),
                args: args.to_vec(),
                defs: var_defs,
                reads: var_reads,
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: Some(Self::cmd_tokens(seg)),
            };
        }

        Statement::Call {
            span: seg.span,
            command: cmd_name.into(),
            args: args.to_vec(),
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(Self::cmd_tokens(seg)),
        }
    }
}

// Public API

/// Lower Tcl source to an IR module.
///
/// This is the main entry point for the lowering phase.
#[must_use]
pub fn lower_to_ir(source: &str, registry: &CommandRegistry) -> Module {
    let mut lowerer = Lowerer::new(registry);
    lowerer.lower(source);
    lowerer.module
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default()
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

    #[test]
    fn proc_definition() {
        let m = lower_to_ir("proc greet {name} {puts $name}", &reg());
        // proc emits an IRCall + registers a procedure.
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

    #[test]
    fn parse_param_names_basic() {
        assert_eq!(parse_param_names("a b c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_param_names_braced() {
        assert_eq!(parse_param_names("{x default} y"), vec!["x", "y"]);
    }

    #[test]
    fn parse_param_names_empty() {
        assert!(parse_param_names("").is_empty());
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
    fn parse_uplevel_level_decimal() {
        assert_eq!(parse_uplevel_level("1"), Some(1));
        assert_eq!(parse_uplevel_level("3"), Some(3));
        assert_eq!(parse_uplevel_level("0"), Some(0));
    }

    #[test]
    fn parse_uplevel_level_hash_form() {
        assert_eq!(parse_uplevel_level("#0"), Some(0));
        assert_eq!(parse_uplevel_level("#3"), Some(-3));
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
            Statement::UpFrame { frame_shift, .. } => assert_eq!(*frame_shift, 1),
            other => panic!("expected UpFrame, got {other:?}"),
        }
    }

    #[test]
    fn uplevel_static_body_with_hash_zero() {
        let m = lower_to_ir("uplevel #0 {set x 1}", &reg());
        match &m.top_level.statements[0] {
            Statement::UpFrame { frame_shift, .. } => assert_eq!(*frame_shift, 0),
            other => panic!("expected UpFrame for #0, got {other:?}"),
        }
    }

    #[test]
    fn uplevel_dynamic_body_falls_back_to_default() {
        // ``uplevel 1 $body`` body is a $var, not a brace literal —
        // can't be statically resolved without C35's const-propagate.
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
}
