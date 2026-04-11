//! Structured IR lowering for control-flow commands.
//!
//! Each method converts a segmented command into its corresponding
//! structured IR statement (`If`, `For`, `While`, `Foreach`, `Catch`,
//! `Try`, `Switch`, `dict` subcommands).

use tcl_lexer::{Lexer, SourceMap, Span, TokenType};

use crate::expr_parser::parse_expr;
use crate::ir::{ForeachIterator, IfClause, Script, Statement, SwitchArm, SwitchMode, TryHandler};
use crate::naming::normalise_var_name;
use crate::segmenter::SegmentedCommand;

use crate::segmenter::word_piece;

use super::{parse_param_names, Lowerer};

/// Parse a braced switch body into a flat list of word elements.
///
/// Tokenises the body text and collects words separated by whitespace/EOL,
/// merging multi-token words. Returns the element texts in order.
fn switch_body_elements(body_text: &str) -> Vec<String> {
    let sm = SourceMap::new(body_text);
    let lexer = Lexer::new(body_text);
    let Ok(tokens) = lexer.tokenise_all() else {
        return Vec::new();
    };

    let mut elements = Vec::new();
    let mut prev_is_sep = true;

    for &tok in &tokens {
        match tok.kind {
            TokenType::Sep | TokenType::Eol | TokenType::Comment => {
                prev_is_sep = true;
                continue;
            }
            TokenType::Eof => continue,
            _ => {}
        }

        let piece = word_piece(&sm, tok);
        if prev_is_sep {
            elements.push(piece);
        } else if let Some(last) = elements.last_mut() {
            last.push_str(&piece);
        } else {
            elements.push(piece);
        }
        prev_is_sep = false;
    }

    elements
}

/// Parse switch options, returning `(first_non_option_index, mode, nocase)`.
fn parse_switch_options(args: &[String]) -> (usize, SwitchMode, bool) {
    let mut i = 0;
    let mut mode = SwitchMode::Exact;
    let mut nocase = false;
    while i < args.len() && args[i].starts_with('-') {
        match args[i].as_str() {
            "--" => {
                i += 1;
                break;
            }
            "-glob" => mode = SwitchMode::Glob,
            "-regexp" => mode = SwitchMode::Regexp,
            "-nocase" => nocase = true,
            _ => {}
        }
        i += 1;
    }
    (i, mode, nocase)
}

impl Lowerer<'_> {
    // ── if ────────────────────────────────────────────────────────

    /// Lower `if cond body ?elseif cond body ...? ?else body?`.
    pub(super) fn lower_if(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();

        if args.is_empty() {
            return Self::barrier(seg, "malformed if");
        }

        let mut clauses = Vec::new();
        let mut else_body = None;
        let mut else_span = None;
        let mut i = 0;

        while i < args.len() {
            if args[i] == "elseif" {
                i += 1;
                continue;
            }
            if args[i] == "else" {
                if i + 1 >= args.len() {
                    return Self::barrier(seg, "malformed if else clause");
                }
                let body_tok = arg_tokens.get(i + 1);
                else_body = Some(self.lower_body_from_tok(&args[i + 1], body_tok, namespace));
                else_span = body_tok.map(|t| t.span);
                break;
            }

            let cond_idx = i;
            i += 1;
            if i < args.len() && args[i] == "then" {
                i += 1;
            }
            if i >= args.len() {
                return Self::barrier(seg, "malformed if clause");
            }

            let body_idx = i;
            let body_tok = arg_tokens.get(body_idx);
            let cond_tok = arg_tokens.get(cond_idx);
            let body = self.lower_body_from_tok(&args[body_idx], body_tok, namespace);
            clauses.push(IfClause {
                condition: parse_expr(&args[cond_idx], None),
                condition_span: cond_tok.map_or(seg.span, |t| t.span),
                body,
                body_span: body_tok.map_or(seg.span, |t| t.span),
            });
            i += 1;
        }

        if clauses.is_empty() {
            return Self::barrier(seg, "malformed if");
        }

        Statement::If {
            span: seg.span,
            clauses,
            else_body,
            else_span,
        }
    }

    // ── for ───────────────────────────────────────────────────────

    /// Lower `for init cond next body`.
    pub(super) fn lower_for(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        if args.len() < 4 || arg_tokens.len() < 4 {
            return Self::barrier(seg, "malformed for");
        }
        if !(arg_single[0] && arg_single[1] && arg_single[2] && arg_single[3]) {
            return Self::barrier(seg, "for with dynamic arguments");
        }

        let init = self.lower_body_from_tok(&args[0], Some(&arg_tokens[0]), namespace);
        let next = self.lower_body_from_tok(&args[2], Some(&arg_tokens[2]), namespace);
        let body = self.lower_body_from_tok(&args[3], Some(&arg_tokens[3]), namespace);

        Statement::For {
            span: seg.span,
            init,
            init_span: arg_tokens[0].span,
            condition: parse_expr(&args[1], None),
            condition_span: arg_tokens[1].span,
            next,
            next_span: arg_tokens[2].span,
            body,
            body_span: arg_tokens[3].span,
            raw_args: args.to_vec(),
        }
    }

    // ── while ─────────────────────────────────────────────────────

    /// Lower `while cond body`.
    pub(super) fn lower_while(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        if args.len() < 2 || arg_tokens.len() < 2 {
            return Self::barrier(seg, "malformed while");
        }
        if !(arg_single[0] && arg_single[1]) {
            return Self::barrier(seg, "while with dynamic arguments");
        }

        let body = self.lower_body_from_tok(&args[1], Some(&arg_tokens[1]), namespace);

        Statement::While {
            span: seg.span,
            condition: parse_expr(&args[0], None),
            condition_span: arg_tokens[0].span,
            body,
            body_span: arg_tokens[1].span,
            raw_args: args.to_vec(),
        }
    }

    // ── foreach / lmap ────────────────────────────────────────────

    /// Lower `foreach varList list ?varList list ...? body`.
    pub(super) fn lower_foreach(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        is_lmap: bool,
    ) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        if args.len() < 3 || args.len() % 2 == 0 {
            return Self::barrier(seg, "malformed foreach");
        }

        let body_idx = args.len() - 1;
        let body_tok = arg_tokens.get(body_idx);
        if body_tok.is_none() || body_idx >= arg_single.len() || !arg_single[body_idx] {
            return Self::barrier(seg, "foreach with dynamic body");
        }

        let mut iterators = Vec::new();
        for i in (0..body_idx).step_by(2) {
            let var_names = parse_param_names(&args[i]);
            iterators.push(ForeachIterator {
                vars: var_names,
                list_arg: args[i + 1].clone(),
            });
        }

        let body = self.lower_body_from_tok(&args[body_idx], body_tok, namespace);

        Statement::Foreach {
            span: seg.span,
            iterators,
            body,
            body_span: body_tok.map_or(seg.span, |t| t.span),
            is_lmap,
            raw_args: args.to_vec(),
            is_dict_iteration: false,
        }
    }

    // ── catch ─────────────────────────────────────────────────────

    /// Lower `catch body ?resultVar? ?optionsVar?`.
    pub(super) fn lower_catch(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        if args.is_empty() {
            return Self::barrier(seg, "malformed catch");
        }
        if arg_tokens.is_empty() || !arg_single.first().copied().unwrap_or(false) {
            return Self::barrier(seg, "catch with dynamic body");
        }

        let body = self.lower_body_from_tok(&args[0], Some(&arg_tokens[0]), namespace);
        let result_var = args.get(1).map(|a| normalise_var_name(a).to_owned());
        let options_var = args.get(2).map(|a| normalise_var_name(a).to_owned());

        Statement::Catch {
            span: seg.span,
            body,
            body_span: arg_tokens[0].span,
            result_var,
            options_var,
            raw_args: args.to_vec(),
        }
    }

    // ── try ───────────────────────────────────────────────────────

    /// Lower `try body ?on|trap matchArg varList handlerBody ...? ?finally finallyBody?`.
    pub(super) fn lower_try(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        if args.is_empty() {
            return Self::barrier(seg, "malformed try");
        }
        if arg_tokens.is_empty() || !arg_single.first().copied().unwrap_or(false) {
            return Self::barrier(seg, "try with dynamic body");
        }

        let body = self.lower_body_from_tok(&args[0], Some(&arg_tokens[0]), namespace);
        let mut handlers = Vec::new();
        let mut finally_body = None;
        let mut finally_span = None;

        let mut i = 1;
        while i < args.len() {
            let keyword = &args[i];

            if keyword == "finally" && i + 1 < args.len() {
                let fin_tok = arg_tokens.get(i + 1);
                let fin_single = arg_single.get(i + 1).copied().unwrap_or(false);
                if fin_tok.is_some() && fin_single {
                    finally_body = Some(self.lower_body_from_tok(&args[i + 1], fin_tok, namespace));
                    finally_span = fin_tok.map(|t| t.span);
                }
                i += 2;
                continue;
            }

            if (keyword == "on" || keyword == "trap") && i + 3 < args.len() {
                let match_arg = args[i + 1].clone();
                let var_list = &args[i + 2];
                let handler_tok = arg_tokens.get(i + 3);

                let var_names = parse_param_names(var_list);
                let result_var = var_names.first().map(|v| normalise_var_name(v).to_owned());
                let options_var = var_names.get(1).map(|v| normalise_var_name(v).to_owned());

                let handler_body = self.lower_body_from_tok(&args[i + 3], handler_tok, namespace);

                handlers.push(TryHandler {
                    kind: keyword.clone(),
                    match_arg,
                    var_name: result_var,
                    options_var,
                    body: handler_body,
                    body_span: handler_tok.map_or(seg.span, |t| t.span),
                });
                i += 4;
                continue;
            }

            return Self::barrier(seg, "malformed try handler");
        }

        Statement::Try {
            span: seg.span,
            body,
            body_span: arg_tokens[0].span,
            handlers,
            finally_body,
            finally_span,
            raw_args: args.to_vec(),
        }
    }

    // ── switch ────────────────────────────────────────────────────

    /// Lower `switch ?options? subject pattern body ...`.
    pub(super) fn lower_switch(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        if args.len() < 2 {
            return Self::barrier(seg, "malformed switch");
        }

        let (mut i, mode, nocase) = parse_switch_options(args);

        if i >= args.len() {
            return Self::barrier(seg, "malformed switch options");
        }

        let subject = args[i].clone();
        i += 1;
        if i >= args.len() {
            return Self::barrier(seg, "switch missing arms");
        }

        // Collect pattern/body pairs.
        let mut pairs: Vec<(String, Span, String, Option<usize>)> = Vec::new();

        // Single braced body form: switch subject { pat1 body1 pat2 body2 ... }
        if i == args.len() - 1 && i < arg_single.len() && arg_single[i] {
            let body_text = &args[i];
            let elements = switch_body_elements(body_text);
            if elements.len() % 2 != 0 {
                return Self::barrier(seg, "switch odd pattern count");
            }
            let mut j = 0;
            while j + 1 < elements.len() {
                pairs.push((
                    elements[j].clone(),
                    seg.span,
                    elements[j + 1].clone(),
                    None, // no per-element arg token index
                ));
                j += 2;
            }
        } else {
            // Multi-arg form: remaining args are pattern body pairs.
            let remaining = args.len() - i;
            if remaining % 2 != 0 {
                return Self::barrier(seg, "switch odd pattern count");
            }
            while i + 1 < args.len() {
                let pattern = args[i].clone();
                let pattern_span = arg_tokens.get(i).map_or(seg.span, |t| t.span);
                let body_text = args[i + 1].clone();
                let body_tok_idx = i + 1;
                pairs.push((pattern, pattern_span, body_text, Some(body_tok_idx)));
                i += 2;
            }
        }

        let mut arms = Vec::new();
        let mut default_body = None;
        let mut default_span = None;

        for (pair_idx, (pattern, pattern_span, body_text, body_tok_idx)) in pairs.iter().enumerate()
        {
            if body_text == "-" {
                arms.push(SwitchArm {
                    pattern: pattern.clone(),
                    pattern_span: *pattern_span,
                    body: None,
                    body_span: None,
                    fallthrough: true,
                });
                continue;
            }

            let body_tok = body_tok_idx.and_then(|idx| arg_tokens.get(idx));
            let body = self.lower_body_from_tok(body_text, body_tok, namespace);
            let body_span_val = body_tok.map(|t| t.span);

            if pattern == "default" && pair_idx == pairs.len() - 1 {
                default_body = Some(body);
                default_span = body_span_val;
            } else {
                arms.push(SwitchArm {
                    pattern: pattern.clone(),
                    pattern_span: *pattern_span,
                    body: Some(body),
                    body_span: body_span_val,
                    fallthrough: false,
                });
            }
        }

        Statement::Switch {
            span: seg.span,
            subject,
            subject_span: arg_tokens.first().map_or(seg.span, |t| t.span),
            arms,
            default_body,
            default_span,
            mode,
            nocase,
            raw_args: args.to_vec(),
        }
    }

    // ── dict ──────────────────────────────────────────────────────

    /// Lower `dict` subcommands.
    pub(super) fn lower_dict(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();
        let sub = &args[0];
        let sub_args = &args[1..];

        match sub.as_str() {
            "for" | "map" if sub_args.len() >= 3 => {
                let var_names = parse_param_names(&sub_args[0]);
                let body_idx = 3; // index in original args
                let body_tok = arg_tokens.get(body_idx);
                if body_tok.is_none() || body_idx >= arg_single.len() || !arg_single[body_idx] {
                    return Self::barrier(seg, &format!("dict {sub} with dynamic body"));
                }
                let body = self.lower_body_from_tok(&sub_args[2], body_tok, namespace);

                Statement::Foreach {
                    span: seg.span,
                    iterators: vec![ForeachIterator {
                        vars: var_names,
                        list_arg: sub_args[1].clone(),
                    }],
                    body,
                    body_span: body_tok.map_or(seg.span, |t| t.span),
                    is_lmap: sub == "map",
                    raw_args: args.to_vec(),
                    is_dict_iteration: true,
                }
            }

            "set" | "unset" | "append" | "lappend" | "incr" if !sub_args.is_empty() => {
                let var_name = normalise_var_name(&sub_args[0]).to_owned();
                Statement::Call {
                    span: seg.span,
                    command: seg.name().into(),
                    args: args.to_vec(),
                    defs: vec![var_name],
                    reads: vec![],
                    reads_own_defs: true,
                    safe_on_uninit: false,
                    tokens: Some(Self::cmd_tokens(seg)),
                }
            }

            "update" | "with" => Statement::Barrier {
                span: seg.span,
                reason: format!("dict {sub}"),
                command: seg.name().into(),
                args: args.to_vec(),
                tokens: Some(Self::cmd_tokens(seg)),
            },

            _ => Statement::Call {
                span: seg.span,
                command: seg.name().into(),
                args: args.to_vec(),
                defs: vec![],
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: Some(Self::cmd_tokens(seg)),
            },
        }
    }

    // ── Helpers ───────────────────────────────────────────────────

    /// Create a barrier statement from a segmented command.
    fn barrier(seg: &SegmentedCommand, reason: &str) -> Statement {
        Statement::Barrier {
            span: seg.span,
            reason: reason.into(),
            command: seg.name().into(),
            args: seg.args().to_vec(),
            tokens: Some(Self::cmd_tokens(seg)),
        }
    }

    /// Lower a body argument using token offset info.
    pub(super) fn lower_body_from_tok(
        &mut self,
        text: &str,
        tok: Option<&tcl_lexer::Token>,
        namespace: &str,
    ) -> Script {
        let Some(tok) = tok else {
            return Script::new();
        };
        let offset = tok.span.start() + u32::from(tok.content_offset);
        self.lower_body(text, offset, namespace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::lower_to_ir;
    use tcl_registry::CommandRegistry;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn switch_exact() {
        let _m = lower_to_ir("switch $x { a {puts a} b {puts b} }", &reg());
        // Single braced body may not be fully parsed yet, but multi-arg works:
        let m2 = lower_to_ir("switch $x a {puts a} b {puts b}", &reg());
        assert!(matches!(
            &m2.top_level.statements[0],
            Statement::Switch { mode, .. } if *mode == SwitchMode::Exact
        ));
    }

    #[test]
    fn try_with_finally() {
        let m = lower_to_ir("try {error oops} finally {cleanup}", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Try {
                finally_body: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn try_with_handler() {
        let m = lower_to_ir("try {error oops} on error {e opts} {puts $e}", &reg());
        if let Statement::Try { handlers, .. } = &m.top_level.statements[0] {
            assert_eq!(handlers.len(), 1);
            assert_eq!(handlers[0].kind, "on");
        } else {
            panic!("expected Try");
        }
    }

    #[test]
    fn catch_with_vars() {
        let m = lower_to_ir("catch {expr 1/0} result opts", &reg());
        if let Statement::Catch {
            result_var,
            options_var,
            ..
        } = &m.top_level.statements[0]
        {
            assert_eq!(result_var.as_deref(), Some("result"));
            assert_eq!(options_var.as_deref(), Some("opts"));
        } else {
            panic!("expected Catch");
        }
    }

    #[test]
    fn dict_set() {
        let m = lower_to_ir("dict set d key value", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Call { command, defs, .. }
                if command == "dict" && defs.contains(&"d".to_string())
        ));
    }

    #[test]
    fn dict_for() {
        let m = lower_to_ir("dict for {k v} $d {puts $k}", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Foreach {
                is_dict_iteration: true,
                ..
            }
        ));
    }
}
