#![allow(
    clippy::doc_markdown,
    clippy::similar_names,
    clippy::unused_self,
    clippy::match_same_arms
)]

//! TclOO class / method body parsing + unknown-proc detection —
//! Rust port of `core/analysis/_analyser/_oo.py`.
//!
//! Walks the body of an ``oo::class create Name { ... }`` or
//! ``oo::define Name { ... }`` block and populates the
//! [`super::types::ClassDef`] fields that **C41e0** seeded.
//! **C41e3** completes the field set (``constructors``,
//! ``destructor``, ``variables``, ``properties``, ``filters``,
//! ``exports``, ``unexports``) and adds
//! [`Analyser::extract_unknown_proc_info`] — the W123 gating
//! analysis for user-defined ``unknown`` procs.
//!
//! Subcommand coverage (mirrors Python's
//! ``_parse_oo_definition_body``):
//!
//! - ``superclass <names>`` — assigns ``ClassDef::superclasses``.
//! - ``mixin ?-append? <names>`` — assigns ``ClassDef::mixins``
//!   (the ``-append`` flag is consumed and ignored —
//!   class-hierarchy state machines belong to the workspace
//!   index, not the per-file analyser).
//! - ``method NAME PARAMS BODY`` — adds to ``ClassDef::methods``.
//! - ``classmethod NAME PARAMS BODY`` — adds to
//!   ``ClassDef::class_methods``.
//! - ``constructor PARAMS BODY`` — appends a synthetic-named
//!   ``MethodDef`` to ``ClassDef::constructors``.
//! - ``destructor BODY`` — sets ``ClassDef::destructor``.
//! - ``forward NAME ?TARGET ARGS?`` — adds to ``methods`` with
//!   ``kind = "forward"``.
//! - ``variable <names>`` — assigns ``ClassDef::variables``.
//! - ``filter <names>`` — assigns ``ClassDef::filters``.
//! - ``export <names>`` / ``unexport <names>`` — extends the
//!   matching ``HashSet`` field.
//! - ``property NAME ?-get BODY? ?-set BODY? ?-kind K?`` —
//!   extracts a [`super::types::PropertyDef`] per name.
//! - ``initialise`` / ``initialize`` — recognised; the body
//!   recursion is deferred (Python walks it for variable
//!   tracking but the Rust dispatcher isn't yet threaded with
//!   the active scope).

use tcl_lexer::{Span, Token, TokenType};

use super::state::Analyser;
use super::types::{ClassDef, MethodDef, PropertyDef, UnknownProcInfo};
use super::utils::parse_param_list;
use crate::ir::{Module, Statement, SwitchMode};
use crate::signature_scan::types::ParamDef;

/// Command names that, when called from inside a user-defined
/// ``unknown`` proc, indicate the handler chains to the original
/// Tcl ``unknown`` rather than dispatching itself.
///
/// Mirrors the ``_CHAIN_TARGETS`` constant in
/// ``core/analysis/_analyser/_oo.py:459-466``.  Names match
/// exactly — when any IR call inside the body resolves to one of
/// these, ``UnknownProcInfo::chains_original`` flips to ``true``.
const CHAIN_TARGETS: &[&str] = &[
    "_original_unknown",
    "_orig_unknown",
    "::tcl::unknown",
    "tcl::unknown",
    "original_unknown",
];

impl Analyser {
    /// Walk the body of a ``oo::class create`` / ``oo::define``
    /// block, populating `class_def` from each subcommand.
    ///
    /// Mirrors `_parse_oo_definition_body` in
    /// `core/analysis/_analyser/_oo.py:146-237`.  The body is
    /// re-segmented via [`crate::segmenter::segment_commands_with_offset`]
    /// (no recovery — recovery is top-level only, mirroring
    /// Python).  Dynamic bodies (non-`Str` tokens) skip the
    /// walk because they can't be statically re-segmented.
    pub(super) fn parse_oo_definition_body(
        &mut self,
        body_text: &str,
        body_tok: Token,
        class_def: &mut ClassDef,
    ) {
        if body_tok.kind != TokenType::Str {
            return;
        }
        let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let cmds = crate::segmenter::segment_commands_with_offset(body_text, base_offset);
        for cmd in cmds {
            if cmd.is_partial || cmd.argv.is_empty() {
                continue;
            }
            apply_oo_subcommand(&cmd.texts, &cmd.argv, class_def);
        }
    }

    /// Detect dispatch shape of a user-defined ``unknown`` proc.
    ///
    /// Mirrors `_extract_unknown_proc_info` in
    /// `core/analysis/_analyser/_oo.py:469-558`.  Lowers the
    /// proc body to IR, then walks the resulting top-level
    /// [`Statement`]s looking for:
    ///
    /// - `IRSwitch` whose subject is `$<first_param>` (or
    ///   `${first_param}`) — exact arms become explicit
    ///   dispatch targets; glob/regexp modes flip
    ///   ``has_pattern_dispatch``.  ``string tolower`` /
    ///   ``string toupper`` in the subject sets
    ///   ``case_insensitive``.
    /// - `IRCall` / `IRBarrier` whose command name matches one
    ///   of [`CHAIN_TARGETS`] — sets ``chains_original``.
    /// - ``exec`` calls — set ``has_exec``.
    /// - ``auto_load`` calls — set ``has_auto_load``.
    ///
    /// Empty bodies set ``empty_stub = true`` and skip the IR
    /// walk.  Lowering failures fall back to the conservative
    /// "fully dynamic" shape (every flag set, no targets) so
    /// downstream W123 emission stays suppressed.
    pub fn extract_unknown_proc_info(
        &mut self,
        body: &str,
        params: &[ParamDef],
    ) -> UnknownProcInfo {
        if body.trim().is_empty() {
            return UnknownProcInfo {
                empty_stub: true,
                ..Default::default()
            };
        }

        let first_param = params
            .first()
            .map_or_else(|| "cmd".to_string(), |p| p.name.clone());

        // Lower to IR.  On panic mirror Python's "be
        // conservative — assume fully dynamic" fallback by
        // returning an ``UnknownProcInfo`` with every dynamic
        // flag set so the W123 emitter suppresses unresolved-
        // command warnings file-wide (the safe direction when
        // we couldn't analyse the handler body).
        let module: Module = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::lowering::lower_to_ir(body, &tcl_registry::CommandRegistry::build_default())
        })) {
            Ok(module) => module,
            Err(_) => {
                return UnknownProcInfo {
                    chains_original: true,
                    case_insensitive: true,
                    has_pattern_dispatch: true,
                    has_exec: true,
                    has_auto_load: true,
                    ..Default::default()
                };
            }
        };

        let mut info = UnknownProcInfo::default();

        for stmt in &module.top_level.statements {
            walk_unknown_stmt(stmt, &first_param, &mut info);
        }

        info
    }

    /// Walk an inline ``oo::define Class subcmd ...`` form,
    /// dispatching the same per-subcommand logic.
    ///
    /// Mirrors `_parse_oo_define_inline` in `_oo.py:239-289`.
    /// The Rust port reuses [`apply_oo_subcommand`] — the
    /// inline form differs from the body form only in how
    /// arguments are framed; the per-subcommand handling is
    /// identical.
    pub(super) fn parse_oo_define_inline(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        class_def: &mut ClassDef,
    ) {
        if args.is_empty() {
            return;
        }
        // Synthesise a single fake "command" matching what the
        // body walker would have produced.
        apply_oo_subcommand(args, arg_tokens, class_def);
    }
}

/// Inspect a single IR statement for unknown-proc dispatch
/// markers.
///
/// Recurses through control-flow bodies (`if` clauses, `for` /
/// `while` / `foreach` bodies, `try` / `catch` bodies) so a
/// ``switch`` arm or ``exec`` call buried inside a guard or
/// loop is still detected.  Mirrors the behaviour of Python's
/// ``iter_ir_statements`` helper which walks the recursive
/// statement tree.
fn walk_unknown_stmt(stmt: &Statement, first_param: &str, info: &mut UnknownProcInfo) {
    match stmt {
        Statement::Switch {
            subject,
            arms,
            mode,
            ..
        } => {
            // Subject reference: ``$first`` or ``${first}``
            // (Python checks both forms).
            let dollar = format!("${first_param}");
            let braced = format!("${{{first_param}}}");
            let subject_refs_first = subject.contains(&dollar) || subject.contains(&braced);

            if subject_refs_first {
                if subject.contains("string tolower") || subject.contains("string toupper") {
                    info.case_insensitive = true;
                }
                if *mode == SwitchMode::Exact {
                    for arm in arms {
                        if arm.pattern != "default" {
                            info.dispatch_targets.insert(arm.pattern.clone());
                        }
                    }
                } else {
                    info.has_pattern_dispatch = true;
                }
            }
            // Recurse into arm bodies (a switch arm may contain
            // an exec or auto_load that should still register).
            for arm in arms {
                if let Some(body) = &arm.body {
                    for inner in &body.statements {
                        walk_unknown_stmt(inner, first_param, info);
                    }
                }
            }
        }
        Statement::Call { command, .. } | Statement::Barrier { command, .. } => {
            if CHAIN_TARGETS.contains(&command.as_str()) {
                info.chains_original = true;
            } else if command == "exec" {
                info.has_exec = true;
            } else if command == "auto_load" {
                info.has_auto_load = true;
            }
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for clause in clauses {
                for inner in &clause.body.statements {
                    walk_unknown_stmt(inner, first_param, info);
                }
            }
            if let Some(body) = else_body {
                for inner in &body.statements {
                    walk_unknown_stmt(inner, first_param, info);
                }
            }
        }
        Statement::For { body, .. }
        | Statement::While { body, .. }
        | Statement::Foreach { body, .. } => {
            for inner in &body.statements {
                walk_unknown_stmt(inner, first_param, info);
            }
        }
        Statement::Catch { body, .. } => {
            for inner in &body.statements {
                walk_unknown_stmt(inner, first_param, info);
            }
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            for inner in &body.statements {
                walk_unknown_stmt(inner, first_param, info);
            }
            for handler in handlers {
                for inner in &handler.body.statements {
                    walk_unknown_stmt(inner, first_param, info);
                }
            }
            if let Some(body) = finally_body {
                for inner in &body.statements {
                    walk_unknown_stmt(inner, first_param, info);
                }
            }
        }
        Statement::Block { body, .. } | Statement::UpFrame { body, .. } => {
            for inner in &body.statements {
                walk_unknown_stmt(inner, first_param, info);
            }
        }
        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. }
        | Statement::ExprEval { .. }
        | Statement::Return { .. } => {}
    }
}

/// Per-subcommand dispatcher shared by the body-form and
/// inline-form walkers.
///
/// `texts` and `argv` are parallel: `texts[0]` / `argv[0]` is
/// the subcommand name (``superclass`` / ``method`` / etc.).
// Long match dispatcher over OO command/subcommand shapes.
#[allow(clippy::too_many_lines)]
fn apply_oo_subcommand(texts: &[String], argv: &[Token], class_def: &mut ClassDef) {
    let Some(subcmd) = texts.first().map(String::as_str) else {
        return;
    };
    let sub_args: &[String] = if texts.len() > 1 { &texts[1..] } else { &[] };
    let sub_tokens: &[Token] = if argv.len() > 1 { &argv[1..] } else { &[] };

    match subcmd {
        "superclass" => {
            class_def.superclasses = sub_args.to_vec();
        }
        "mixin" => {
            // Skip ``-append`` and similar flags — mirrors
            // Python's ``[a for a in sub_args if not a.startswith("-")]``.
            class_def.mixins = sub_args
                .iter()
                .filter(|a| !a.starts_with('-'))
                .cloned()
                .collect();
        }
        "method" => {
            if let Some(md) = extract_method_def(sub_args, sub_tokens, "method", "public", "") {
                class_def.methods.insert(md.name.clone(), md);
            }
        }
        "classmethod" => {
            if let Some(md) = extract_method_def(sub_args, sub_tokens, "classmethod", "public", "")
            {
                class_def.class_methods.insert(md.name.clone(), md);
            }
        }
        "constructor" => {
            if let Some(md) = extract_method_def(
                sub_args,
                sub_tokens,
                "constructor",
                "public",
                "<constructor>",
            ) {
                class_def.constructors.push(md);
            }
        }
        "destructor" => {
            if let Some(md) =
                extract_method_def(sub_args, sub_tokens, "destructor", "public", "<destructor>")
            {
                class_def.destructor = Some(md);
            }
        }
        "variable" => {
            class_def.variables = sub_args.to_vec();
        }
        "filter" => {
            class_def.filters = sub_args.to_vec();
        }
        "export" => {
            class_def.exports.extend(sub_args.iter().cloned());
        }
        "unexport" => {
            class_def.unexports.extend(sub_args.iter().cloned());
        }
        "property" => {
            extract_property_defs(sub_args, sub_tokens, class_def);
        }
        "forward" => {
            if let Some(name) = sub_args.first() {
                let span = sub_tokens
                    .first()
                    .map_or_else(|| tcl_lexer::Span::new(0, 0), |t| t.span);
                let md = MethodDef {
                    name: name.clone(),
                    params: Vec::new(),
                    name_span: span,
                    body_span: span,
                    kind: "forward".to_string(),
                    visibility: "public".to_string(),
                    doc: String::new(),
                };
                class_def.methods.insert(md.name.clone(), md);
            }
        }
        "private" => {
            // ``private`` wraps another definition subcommand.
            // The wrapped subcommand fires with ``visibility =
            // "private"``.
            if sub_args.is_empty() {
                return;
            }
            let inner_subcmd = sub_args[0].as_str();
            let inner_args: &[String] = &sub_args[1..];
            let inner_tokens: &[Token] = if sub_tokens.len() > 1 {
                &sub_tokens[1..]
            } else {
                &[]
            };
            match inner_subcmd {
                "method" => {
                    if let Some(md) =
                        extract_method_def(inner_args, inner_tokens, "method", "private", "")
                    {
                        class_def.methods.insert(md.name.clone(), md);
                    }
                }
                "classmethod" => {
                    if let Some(md) =
                        extract_method_def(inner_args, inner_tokens, "classmethod", "private", "")
                    {
                        class_def.class_methods.insert(md.name.clone(), md);
                    }
                }
                _ => {}
            }
        }
        // ``initialise`` / ``initialize`` are class-level
        // initialisation scripts; the body is recorded by the
        // Python analyser via ``_analyse_body`` (it walks the
        // body for variable tracking).  The Rust port doesn't
        // yet thread the active scope / scope-path into
        // ``apply_oo_subcommand``, so the body recursion is
        // deferred — the subcommand is recognised here so the
        // _ arm doesn't silently drop it.
        "initialise" | "initialize" => {}
        _ => {}
    }
}

/// Extract property definitions from a ``property`` subcommand.
///
/// Mirrors `_extract_property_defs` in `_oo.py:390-453`.
/// Walks the args, splitting names from option values
/// (``-get BODY``, ``-set BODY``, ``-kind readable|writable|readwrite``).
/// Each property gets a [`PropertyDef`] entry in
/// ``class_def.properties``.
///
/// All property options take a value (``-get``, ``-set``,
/// ``-kind``); there are no flag-only options.  When ``-kind``
/// is omitted the property defaults to ``"readwrite"``, matching
/// Python's dataclass default.
///
/// Body recursion into accessor (`-get` / `-set`) bodies is
/// deferred — the Python equivalent walks each body for
/// variable tracking, but the Rust [`apply_oo_subcommand`] dispatcher
/// isn't yet threaded with the active scope.  The class-level
/// PropertyDef entries are still emitted, so consumers see the
/// property records.
fn extract_property_defs(args: &[String], arg_tokens: &[Token], class_def: &mut ClassDef) {
    let zero = Span::new(0, 0);

    // Collect property names + their per-arg index, then the
    // trailing options.  Mirrors Python's two-pass shape.
    let mut names: Vec<(String, usize)> = Vec::new();
    let mut kind = "readwrite".to_string();
    let mut has_getter = false;
    let mut has_setter = false;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if let Some(stripped) = arg.strip_prefix('-') {
            // All property options take a value.
            if i + 1 >= args.len() {
                i += 1;
                continue;
            }
            let value = &args[i + 1];
            match stripped {
                "kind" => kind.clone_from(value),
                "get" => has_getter = true,
                "set" => has_setter = true,
                _ => {}
            }
            i += 2;
            continue;
        }
        names.push((arg.clone(), i));
        i += 1;
    }

    for (name, idx) in names {
        let span = arg_tokens.get(idx).map_or(zero, |t| t.span);
        class_def.properties.insert(
            name.clone(),
            PropertyDef {
                name,
                name_span: span,
                kind: kind.clone(),
                has_getter,
                has_setter,
            },
        );
    }
}

/// Extract a [`MethodDef`] from method-style args.
///
/// Mirrors `_extract_method_def` in `_oo.py:290-349`.  Three
/// shapes:
///
/// - **method / classmethod**: `args = [name, params, body]`.
/// - **constructor**: `args = [params, body]`; `synthetic_name`
///   provides the placeholder name (``"<constructor>"``).
/// - **destructor**: `args = [body]`; same synthetic-name trick.
///
/// Returns `None` when the argument count is too short to
/// match any of the shapes.
fn extract_method_def(
    args: &[String],
    arg_tokens: &[Token],
    kind: &str,
    visibility: &str,
    synthetic_name: &str,
) -> Option<MethodDef> {
    let zero = tcl_lexer::Span::new(0, 0);
    match kind {
        "constructor" => {
            // ``constructor PARAMS BODY``.
            if args.len() < 2 {
                return None;
            }
            let params = parse_param_list(&args[0]);
            let name_span = zero;
            let body_span = arg_tokens.get(1).map_or(zero, |t| t.span);
            Some(MethodDef {
                name: synthetic_name.to_string(),
                params,
                name_span,
                body_span,
                kind: kind.to_string(),
                visibility: visibility.to_string(),
                doc: String::new(),
            })
        }
        "destructor" => {
            // ``destructor BODY``.
            if args.is_empty() {
                return None;
            }
            let name_span = zero;
            let body_span = arg_tokens.first().map_or(zero, |t| t.span);
            Some(MethodDef {
                name: synthetic_name.to_string(),
                params: Vec::new(),
                name_span,
                body_span,
                kind: kind.to_string(),
                visibility: visibility.to_string(),
                doc: String::new(),
            })
        }
        _ => {
            // ``method NAME PARAMS BODY`` / ``classmethod NAME PARAMS BODY``.
            if args.len() < 3 {
                return None;
            }
            let name = args[0].clone();
            let params = parse_param_list(&args[1]);
            let name_span = arg_tokens.first().map_or(zero, |t| t.span);
            let body_span = arg_tokens.get(2).map_or(zero, |t| t.span);
            Some(MethodDef {
                name,
                params,
                name_span,
                body_span,
                kind: kind.to_string(),
                visibility: visibility.to_string(),
                doc: String::new(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class() -> ClassDef {
        ClassDef {
            name: "C".to_string(),
            qualified_name: "::C".to_string(),
            name_span: tcl_lexer::Span::new(0, 0),
            body_span: tcl_lexer::Span::new(0, 0),
            ..Default::default()
        }
    }

    fn tok(span: (u32, u32)) -> Token {
        Token::new(TokenType::Esc, tcl_lexer::Span::new(span.0, span.1))
    }

    fn str_tok(span: (u32, u32)) -> Token {
        Token {
            kind: TokenType::Str,
            span: tcl_lexer::Span::new(span.0, span.1),
            content_offset: 1,
            in_quote: false,
        }
    }

    #[test]
    fn superclass_subcommand_assigns_supers() {
        let mut cd = class();
        let texts: Vec<String> = ["superclass", "::A", "::B"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 10)), tok((11, 14)), tok((15, 18))];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert_eq!(cd.superclasses, vec!["::A", "::B"]);
    }

    #[test]
    fn mixin_subcommand_strips_dash_flags() {
        let mut cd = class();
        let texts: Vec<String> = ["mixin", "-append", "::M1", "::M2"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 5)), tok((6, 13)), tok((14, 18)), tok((19, 23))];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert_eq!(cd.mixins, vec!["::M1", "::M2"]);
    }

    #[test]
    fn method_subcommand_records_method_def() {
        let mut cd = class();
        let texts: Vec<String> = ["method", "greet", "name", "puts $name"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 6)), tok((7, 12)), tok((13, 17)), str_tok((18, 32))];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert!(cd.methods.contains_key("greet"));
        let md = &cd.methods["greet"];
        assert_eq!(md.kind, "method");
        assert_eq!(md.visibility, "public");
    }

    #[test]
    fn classmethod_subcommand_records_class_method() {
        let mut cd = class();
        let texts: Vec<String> = ["classmethod", "build", "args", "return $args"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [
            tok((0, 11)),
            tok((12, 17)),
            tok((18, 22)),
            str_tok((23, 38)),
        ];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert!(cd.class_methods.contains_key("build"));
        assert!(!cd.methods.contains_key("build"));
    }

    #[test]
    fn constructor_appends_to_constructors_list() {
        let mut cd = class();
        let texts: Vec<String> = ["constructor", "args", "puts ctor"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 11)), tok((12, 16)), str_tok((17, 28))];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert_eq!(cd.constructors.len(), 1);
        assert_eq!(cd.constructors[0].kind, "constructor");
        assert_eq!(cd.constructors[0].name, "<constructor>");
        // Constructors are no longer mirrored into the methods map.
        assert!(!cd.methods.contains_key("<constructor>"));
    }

    #[test]
    fn destructor_sets_destructor_field() {
        let mut cd = class();
        let texts: Vec<String> = ["destructor", "puts dtor"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 10)), str_tok((11, 22))];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        let dtor = cd.destructor.as_ref().expect("destructor recorded");
        assert_eq!(dtor.kind, "destructor");
        assert_eq!(dtor.name, "<destructor>");
        assert!(!cd.methods.contains_key("<destructor>"));
    }

    #[test]
    fn forward_records_method_with_forward_kind() {
        let mut cd = class();
        let texts: Vec<String> = ["forward", "delegate", "::other::cmd"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 7)), tok((8, 16)), tok((17, 29))];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert!(cd.methods.contains_key("delegate"));
        assert_eq!(cd.methods["delegate"].kind, "forward");
    }

    #[test]
    fn private_method_records_with_private_visibility() {
        let mut cd = class();
        let texts: Vec<String> = ["private", "method", "internal", "args", "puts hi"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [
            tok((0, 7)),
            tok((8, 14)),
            tok((15, 23)),
            tok((24, 28)),
            str_tok((29, 37)),
        ];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert!(cd.methods.contains_key("internal"));
        assert_eq!(cd.methods["internal"].visibility, "private");
    }

    #[test]
    fn unrecognised_subcommand_is_silent_noop() {
        let mut cd = class();
        let texts: Vec<String> = ["whatever", "x"].iter().map(|s| (*s).to_string()).collect();
        let argv = [tok((0, 8)), tok((9, 10))];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        // No fields populated; no panic.
        assert!(cd.methods.is_empty());
        assert!(cd.superclasses.is_empty());
        assert!(cd.mixins.is_empty());
    }

    #[test]
    fn variable_subcommand_records_class_variables() {
        let mut cd = class();
        let texts: Vec<String> = ["variable", "x", "y"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 8)), tok((9, 10)), tok((11, 12))];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert_eq!(cd.variables, vec!["x", "y"]);
    }

    #[test]
    fn filter_subcommand_records_filters() {
        let mut cd = class();
        let texts: Vec<String> = ["filter", "log", "trace"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 6)), tok((7, 10)), tok((11, 16))];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert_eq!(cd.filters, vec!["log", "trace"]);
    }

    #[test]
    fn export_and_unexport_record_sets() {
        let mut cd = class();
        let texts1: Vec<String> = ["export", "foo", "bar"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv1 = [tok((0, 6)), tok((7, 10)), tok((11, 14))];
        apply_oo_subcommand(&texts1, &argv1, &mut cd);
        assert!(cd.exports.contains("foo"));
        assert!(cd.exports.contains("bar"));

        let texts2: Vec<String> = ["unexport", "baz"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv2 = [tok((0, 8)), tok((9, 12))];
        apply_oo_subcommand(&texts2, &argv2, &mut cd);
        assert!(cd.unexports.contains("baz"));
    }

    #[test]
    fn property_subcommand_records_property_def() {
        let mut cd = class();
        let texts: Vec<String> = [
            "property",
            "colour",
            "-kind",
            "readable",
            "-get",
            "return red",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
        let argv = [
            tok((0, 8)),
            tok((9, 15)),
            tok((16, 21)),
            tok((22, 30)),
            tok((31, 35)),
            str_tok((36, 47)),
        ];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        let pd = cd.properties.get("colour").expect("colour recorded");
        assert_eq!(pd.kind, "readable");
        assert!(pd.has_getter);
        assert!(!pd.has_setter);
    }

    #[test]
    fn property_subcommand_with_no_kind_defaults_to_readwrite() {
        let mut cd = class();
        let texts: Vec<String> = ["property", "x"].iter().map(|s| (*s).to_string()).collect();
        let argv = [tok((0, 8)), tok((9, 10))];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        let pd = cd.properties.get("x").expect("x recorded");
        assert_eq!(pd.kind, "readwrite");
        assert!(!pd.has_getter);
        assert!(!pd.has_setter);
    }

    #[test]
    fn property_subcommand_records_multiple_names() {
        let mut cd = class();
        let texts: Vec<String> = ["property", "x", "y", "-kind", "writable"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [
            tok((0, 8)),
            tok((9, 10)),
            tok((11, 12)),
            tok((13, 18)),
            tok((19, 27)),
        ];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert_eq!(cd.properties.len(), 2);
        assert_eq!(cd.properties["x"].kind, "writable");
        assert_eq!(cd.properties["y"].kind, "writable");
    }

    #[test]
    fn extract_method_def_too_few_args_returns_none() {
        // ``method`` with only 1 arg (just the name) — needs 3.
        let args: Vec<String> = vec!["foo".to_string()];
        let argv: Vec<Token> = vec![tok((0, 3))];
        let md = extract_method_def(&args, &argv, "method", "public", "");
        assert!(md.is_none());
    }

    fn param(name: &str) -> ParamDef {
        ParamDef {
            name: name.to_string(),
            has_default: false,
            default_value: None,
        }
    }

    #[test]
    fn extract_unknown_proc_info_empty_body_marks_empty_stub() {
        let mut a = Analyser::new();
        let info = a.extract_unknown_proc_info("", &[param("cmd"), param("args")]);
        assert!(info.empty_stub);
        assert!(info.dispatch_targets.is_empty());
    }

    #[test]
    fn extract_unknown_proc_info_whitespace_body_marks_empty_stub() {
        let mut a = Analyser::new();
        let info = a.extract_unknown_proc_info("   \n  \t ", &[param("cmd"), param("args")]);
        assert!(info.empty_stub);
    }

    #[test]
    fn extract_unknown_proc_info_exact_switch_collects_dispatch_targets() {
        let mut a = Analyser::new();
        let body = r"switch -exact $cmd {
            foo { return 1 }
            bar { return 2 }
            default { return 0 }
        }";
        let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
        assert!(!info.empty_stub);
        assert!(info.dispatch_targets.contains("foo"));
        assert!(info.dispatch_targets.contains("bar"));
        assert!(!info.dispatch_targets.contains("default"));
        assert!(!info.has_pattern_dispatch);
    }

    #[test]
    fn extract_unknown_proc_info_glob_switch_marks_pattern_dispatch() {
        let mut a = Analyser::new();
        let body = r"switch -glob $cmd {
            foo* { return 1 }
            *bar { return 2 }
        }";
        let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
        assert!(info.has_pattern_dispatch);
        assert!(info.dispatch_targets.is_empty());
    }

    #[test]
    fn extract_unknown_proc_info_chains_original_via_known_target() {
        let mut a = Analyser::new();
        let body = r"_original_unknown $cmd $args";
        let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
        assert!(info.chains_original);
    }

    #[test]
    fn extract_unknown_proc_info_detects_exec_call() {
        let mut a = Analyser::new();
        let body = r"exec $cmd {*}$args";
        let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
        assert!(info.has_exec);
    }

    #[test]
    fn extract_unknown_proc_info_detects_auto_load_call() {
        let mut a = Analyser::new();
        let body = r"auto_load $cmd";
        let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
        assert!(info.has_auto_load);
    }

    #[test]
    fn extract_unknown_proc_info_case_insensitive_via_string_tolower() {
        let mut a = Analyser::new();
        let body = r"switch -exact [string tolower $cmd] {
            foo { return 1 }
        }";
        let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
        assert!(info.case_insensitive);
        assert!(info.dispatch_targets.contains("foo"));
    }

    #[test]
    fn extract_unknown_proc_info_no_first_param_defaults_to_cmd() {
        // Empty params list — the helper should fall back to
        // ``"cmd"`` as the dispatch-subject variable name.
        let mut a = Analyser::new();
        let body = r"switch -exact $cmd { foo { return 1 } }";
        let info = a.extract_unknown_proc_info(body, &[]);
        assert!(info.dispatch_targets.contains("foo"));
    }
}
