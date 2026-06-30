#![allow(
    clippy::doc_markdown,
    clippy::similar_names,
    clippy::unused_self,
    clippy::match_same_arms
)]

//! TclOO class / method body parsing + unknown-proc detection.
//!
//! Walks the body of an ``oo::class create Name { ... }`` or
//! ``oo::define Name { ... }`` block and populates the
//! [`super::types::ClassDef`] fields: the full field set
//! (``constructors``, ``destructor``, ``variables``,
//! ``properties``, ``filters``, ``exports``, ``unexports``) plus
//! [`Analyser::extract_unknown_proc_info`] — the W123 gating
//! analysis for user-defined ``unknown`` procs.
//!
//! Subcommand coverage:
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
//! - ``initialise`` / ``initialize`` — recognised; the body is
//!   walked in the enclosing scope for variable tracking.

use tcl_lexer::{Span, Token, TokenType};

use super::scope::scope_at_mut;
use super::state::Analyser;
use super::types::{ClassDef, MethodDef, PropertyDef, Scope, ScopeKind, UnknownProcInfo};
use super::utils::{param_name_spans, parse_param_list};
use crate::ir::{Module, Statement, SwitchMode};
use crate::signature_scan::types::ParamDef;

/// Command names that, when called from inside a user-defined
/// ``unknown`` proc, indicate the handler chains to the original
/// Tcl ``unknown`` rather than dispatching itself.
///
/// Names match exactly — when any IR call inside the body
/// resolves to one of these, ``UnknownProcInfo::chains_original``
/// flips to ``true``.
const CHAIN_TARGETS: &[&str] = &[
    "_original_unknown",
    "_orig_unknown",
    "::tcl::unknown",
    "tcl::unknown",
    "original_unknown",
];

/// snit (tcllib) type/widget definers, both bare and `::`-qualified.
const SNIT_DEFINERS: &[&str] = &[
    "snit::type",
    "snit::widget",
    "snit::widgetadaptor",
    "::snit::type",
    "::snit::widget",
    "::snit::widgetadaptor",
];

/// Implicit instance variables snit injects into `method` / `constructor` /
/// `destructor` / `onconfigure` / `oncget` bodies.
const SNIT_INSTANCE_IMPLICIT: &[&str] = &["self", "selfns", "type", "options"];

/// Implicit variable snit injects into `typemethod` / `typeconstructor` bodies.
const SNIT_TYPE_IMPLICIT: &[&str] = &["type"];

impl Analyser {
    /// Walk the body of a ``oo::class create`` / ``oo::define``
    /// block, populating `class_def` from each subcommand.
    ///
    /// The body is re-segmented via
    /// [`crate::segmenter::segment_commands_with_offset`] (no
    /// recovery — recovery is top-level only).  Dynamic bodies
    /// (non-`Str` tokens) skip the walk because they can't be
    /// statically re-segmented.
    pub(super) fn parse_oo_definition_body(
        &mut self,
        body_text: &str,
        body_tok: Token,
        class_def: &mut ClassDef,
        class_qualified: &str,
        scope_path: &[usize],
    ) {
        if body_tok.kind != TokenType::Str {
            return;
        }
        let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let cmds = crate::segmenter::segment_commands_with_offset_and_config(
            body_text,
            base_offset,
            self.lexer_config(),
        );
        // Phase 1: populate the `ClassDef` (methods, instance variables,
        // superclasses, …) and collect each method body to walk afterwards.
        // The walk is deferred so every class-level `variable` declaration is
        // visible as a pre-bound local in *every* method body, regardless of
        // source order.
        let mut method_bodies: Vec<CollectedMethodBody> = Vec::new();
        // `property -get`/`-set` accessor bodies — walked in a method scope
        // seeded with the class variables.
        let mut accessor_bodies: Vec<CollectedMethodBody> = Vec::new();
        // `initialise`/`initialize { body }` — a class-level script walked in
        // the *enclosing* scope (not a method scope).
        let mut init_bodies: Vec<(String, Token)> = Vec::new();
        for cmd in &cmds {
            if cmd.is_partial || cmd.argv.is_empty() {
                continue;
            }
            apply_oo_subcommand(&cmd.texts, &cmd.argv, class_def);
            if let Some(mb) = collect_method_body(&cmd.texts, &cmd.argv) {
                method_bodies.push(mb);
            }
            match cmd.texts.first().map(String::as_str) {
                Some("property") => {
                    collect_property_accessor_bodies(&cmd.texts, &cmd.argv, &mut accessor_bodies);
                }
                Some("initialise" | "initialize") => {
                    if let (Some(body), Some(tok)) = (cmd.texts.get(1), cmd.argv.get(1).copied())
                        && tok.kind == TokenType::Str
                    {
                        init_bodies.push((body.clone(), tok));
                    }
                }
                _ => {}
            }
        }
        // Phase 2: walk each method / accessor body in its own `Method` scope
        // with the formal parameters and the class's instance variables
        // pre-bound; the `initialise` body walks in the enclosing scope.
        let class_variables = class_def.variables.clone();
        for mb in method_bodies.iter().chain(accessor_bodies.iter()) {
            self.walk_method_body(&class_variables, class_qualified, scope_path, mb);
        }
        for (body, tok) in init_bodies {
            self.analyse_body(&body, tok, scope_path);
        }
    }

    /// Walk a single TclOO method body in a fresh [`ScopeKind::Method`] scope.
    ///
    /// Pre-binds the method's formal parameters and the class's instance
    /// `variable`s as defined-but-not-warned locals (so reads of them do not
    /// false-fire W210 read-before-set / W214 unused), then re-walks the body
    /// through [`Self::analyse_body`] so its `$obj method` / `[cmd] method`
    /// dispatch sites are recorded (with `in_method = true`) for the W307 /
    /// W308 post-pass.
    fn walk_method_body(
        &mut self,
        class_variables: &[String],
        class_qualified: &str,
        scope_path: &[usize],
        mb: &CollectedMethodBody,
    ) {
        if mb.body_tok.kind != TokenType::Str {
            return;
        }
        let method_qn = if class_qualified.is_empty() {
            mb.name.clone()
        } else {
            format!("{class_qualified}::{}", mb.name)
        };
        let Some(method_idx) = ({
            scope_at_mut(&mut self.result.global_scope, scope_path).map(|parent| {
                let mut child = Scope::new(ScopeKind::Method, method_qn.clone());
                child.body_span = Some(mb.body_tok.span);
                parent.children.push(child);
                parent.children.len() - 1
            })
        }) else {
            return;
        };
        let mut method_path = scope_path.to_vec();
        method_path.push(method_idx);
        // Formal parameters — defined, never unused-warned.  Anchor each
        // param's definition span at its name in the param-list literal (issue
        // #727) so go-to-definition / references / rename resolve to the
        // parameter, not the whole method body.  Falls back to the body token
        // when the param-list word or a name can't be located.
        let param_spans = mb.params_tok.and_then(|pt| {
            self.source
                .get(pt.span.start() as usize..pt.span.end() as usize)
                .map(|raw| param_name_spans(raw, pt.span.start()))
        });
        for (i, p) in mb.params.iter().enumerate() {
            let def_span = param_spans.as_ref().and_then(|s| s.get(i).copied());
            self.define_var(&p.name, mb.body_tok, &method_path, false, def_span);
        }
        // Class instance variables — visible in every method body.
        for var in class_variables {
            let base = crate::naming::normalise_var_name(var);
            if base.is_empty() || mb.params.iter().any(|p| p.name == base) {
                continue;
            }
            self.define_var(base, mb.body_tok, &method_path, false, None);
        }
        // Per-item shell pass: defer the method body for an isolated pass like
        // `handle_proc_command`.  Carry the method's qualified name as
        // `scope_name` (so the duplicate detector keys each method distinctly),
        // the class's *defining* namespace (so command/var resolution in the
        // isolated `Method` scope matches the whole-file walk), the formal
        // params, and the class instance variables (pre-bound in every method).
        if self.defer_proc_bodies {
            let namespace = self.namespace_from_scope_path(scope_path);
            self.deferred_bodies.push(super::per_item::DeferredBody {
                body_text: mb.body_text.clone(),
                body_tok: mb.body_tok,
                scope_path: method_path,
                is_method: true,
                namespace,
                scope_name: method_qn,
                params: mb.params.clone(),
                class_variables: class_variables.to_vec(),
            });
        } else {
            self.analyse_body(&mb.body_text, mb.body_tok, &method_path);
        }
    }

    /// Handle a snit (tcllib) type/widget definition —
    /// ``snit::type``/``snit::widget``/``snit::widgetadaptor Name { body }``
    /// (and their `::`-qualified forms).  Snit reinterprets its body as a
    /// class description, exactly like an `oo::class` body, so we model it as
    /// a real [`ClassDef`] with method scopes — object dispatch inside method
    /// bodies (`$self foo`, `$component bar`) is then recognised as in-method
    /// dispatch (no false W307) and snit's implicit instance variables don't
    /// surface as read-before-set / unused.
    pub(super) fn handle_snit_type_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if !SNIT_DEFINERS.contains(&cmd_name) || args.len() < 2 || arg_tokens.len() < 2 {
            return false;
        }
        let raw_name = &args[0];
        let body = &args[1];
        let ns_prefix = self.namespace_from_scope_path(scope_path);
        let ns_for_qualify = ns_prefix.trim_start_matches(':');
        let qualified = super::handlers::qualify(ns_for_qualify, raw_name);
        let simple = qualified.rsplit("::").next().unwrap_or("").to_string();
        let name_span = arg_tokens[0].span;
        let body_tok = arg_tokens[1];
        let is_widget = cmd_name.ends_with("widget") || cmd_name.ends_with("widgetadaptor");
        let doc = std::mem::take(&mut self.last_comment);
        let mut class = ClassDef {
            name: simple.clone(),
            qualified_name: qualified.clone(),
            name_span,
            body_span: body_tok.span,
            metaclass: cmd_name.to_string(),
            doc,
            ..Default::default()
        };
        if !body.is_empty() {
            self.parse_snit_definition_body(
                body, body_tok, &mut class, &qualified, scope_path, is_widget,
            );
        }
        self.result.all_classes.insert(qualified, class.clone());
        let path = scope_path.to_vec();
        if let Some(scope) = scope_at_mut(&mut self.result.global_scope, &path) {
            scope.classes.insert(simple, class);
        }
        true
    }

    /// Parse a snit type/widget body into methods + variable declarations, in
    /// two passes (so a method can reference any instance/type variable
    /// regardless of declaration order).
    fn parse_snit_definition_body(
        &mut self,
        body: &str,
        body_tok: Token,
        class_def: &mut ClassDef,
        class_qualified: &str,
        scope_path: &[usize],
        is_widget: bool,
    ) {
        if body_tok.kind != TokenType::Str {
            return;
        }
        let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let cmds = crate::segmenter::segment_commands_with_offset_and_config(
            body,
            base_offset,
            self.lexer_config(),
        );

        // Snit injects these implicit variables into method / type-method bodies.
        let mut instance_vars: Vec<String> = SNIT_INSTANCE_IMPLICIT
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        if is_widget {
            instance_vars.push("win".to_string());
            instance_vars.push("hull".to_string());
        }
        let mut type_vars: Vec<String> = SNIT_TYPE_IMPLICIT
            .iter()
            .map(|s| (*s).to_string())
            .collect();

        // First pass: collect declared instance / type variable + component names.
        for cmd in &cmds {
            if cmd.is_partial {
                continue;
            }
            let Some((sub, sub_args)) = cmd.texts.split_first() else {
                continue;
            };
            match sub.as_str() {
                "variable" | "component" => {
                    if let Some(name) = sub_args.first() {
                        instance_vars.push(name.clone());
                    }
                }
                "typevariable" | "typecomponent" => {
                    if let Some(name) = sub_args.first() {
                        type_vars.push(name.clone());
                    }
                }
                _ => {}
            }
        }

        // Record the *explicit* instance + type variables on the class —
        // method-scope seeding and the W307 dispatch-source suppression both
        // read `ClassDef::variables`.  Only the four implicit scalars
        // (`self`/`selfns`/`type`/`options`) and the type-implicit `type` are
        // filtered; a widget's injected `win`/`hull` are kept.
        class_def.variables = instance_vars
            .iter()
            .filter(|v| !SNIT_INSTANCE_IMPLICIT.contains(&v.as_str()))
            .chain(
                type_vars
                    .iter()
                    .filter(|v| !SNIT_TYPE_IMPLICIT.contains(&v.as_str())),
            )
            .cloned()
            .collect();

        // Second pass: analyse method-bearing declarations in method scopes.
        for cmd in &cmds {
            if cmd.is_partial {
                continue;
            }
            if let Some((sub, sub_args)) = cmd.texts.split_first() {
                let sub_tokens = cmd.argv.get(1..).unwrap_or(&[]);
                self.dispatch_snit_member(
                    sub,
                    sub_args,
                    sub_tokens,
                    class_def,
                    class_qualified,
                    scope_path,
                    &instance_vars,
                    &type_vars,
                );
            }
        }
    }

    /// Dispatch one snit body subcommand to the matching method extractor (or,
    /// for `proc`, the ordinary proc handler).  Split out of
    /// [`Self::parse_snit_definition_body`] so the two-pass walk stays small.
    #[allow(clippy::too_many_arguments)]
    fn dispatch_snit_member(
        &mut self,
        sub: &str,
        sub_args: &[String],
        sub_tokens: &[Token],
        class_def: &mut ClassDef,
        class_qualified: &str,
        scope_path: &[usize],
        instance_vars: &[String],
        type_vars: &[String],
    ) {
        match sub {
            // snit allows a type-private `proc name args body` — analyse it as
            // an ordinary proc in the enclosing scope.
            "proc" => {
                self.handle_proc_command("proc", sub_args, sub_tokens, scope_path);
            }
            "method" => self.extract_snit_method(
                sub_args,
                sub_tokens,
                class_def,
                class_qualified,
                scope_path,
                instance_vars,
                "method",
                false,
                "",
            ),
            "typemethod" => self.extract_snit_method(
                sub_args,
                sub_tokens,
                class_def,
                class_qualified,
                scope_path,
                type_vars,
                "classmethod",
                false,
                "",
            ),
            "constructor" => self.extract_snit_method(
                sub_args,
                sub_tokens,
                class_def,
                class_qualified,
                scope_path,
                instance_vars,
                "constructor",
                false,
                "<constructor>",
            ),
            "destructor" => self.extract_snit_method(
                sub_args,
                sub_tokens,
                class_def,
                class_qualified,
                scope_path,
                instance_vars,
                "destructor",
                true,
                "<destructor>",
            ),
            "typeconstructor" => self.extract_snit_method(
                sub_args,
                sub_tokens,
                class_def,
                class_qualified,
                scope_path,
                type_vars,
                "classmethod",
                true,
                "<typeconstructor>",
            ),
            // `onconfigure -opt valuevar { body }` / `oncget -opt { body }`
            // (snit 1.x) — the leading `-opt` word is dropped.
            "onconfigure" => {
                let label = sub_args
                    .first()
                    .map_or(String::new(), |o| format!("<onconfigure {o}>"));
                self.extract_snit_method(
                    sub_args.get(1..).unwrap_or(&[]),
                    sub_tokens.get(1..).unwrap_or(&[]),
                    class_def,
                    class_qualified,
                    scope_path,
                    instance_vars,
                    "method",
                    false,
                    &label,
                );
            }
            "oncget" => {
                let label = sub_args
                    .first()
                    .map_or(String::new(), |o| format!("<oncget {o}>"));
                self.extract_snit_method(
                    sub_args.get(1..).unwrap_or(&[]),
                    sub_tokens.get(1..).unwrap_or(&[]),
                    class_def,
                    class_qualified,
                    scope_path,
                    instance_vars,
                    "method",
                    true,
                    &label,
                );
            }
            _ => {}
        }
    }

    /// Analyse one snit method / constructor / etc. body in a method scope
    /// seeded with `seed_vars` (snit's implicit names + declared instance or
    /// type variables) and the method's formal parameters.  `no_arglist` is
    /// for declarations whose body immediately follows the keyword
    /// (`destructor` / `typeconstructor` / `oncget`); `synthetic_name` names
    /// the synthetic constructor/handler forms.
    #[allow(clippy::too_many_arguments)]
    fn extract_snit_method(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        class_def: &mut ClassDef,
        class_qualified: &str,
        scope_path: &[usize],
        seed_vars: &[String],
        kind: &str,
        no_arglist: bool,
        synthetic_name: &str,
    ) {
        let (name, params, body_text, body_tok, params_tok) = if no_arglist {
            let Some(body) = args.first() else {
                return;
            };
            let nm = if synthetic_name.is_empty() {
                "<body>".to_string()
            } else {
                synthetic_name.to_string()
            };
            (
                nm,
                Vec::new(),
                body.clone(),
                arg_tokens.first().copied(),
                None,
            )
        } else if synthetic_name.is_empty() {
            // method / typemethod: NAME ARGLIST BODY.
            if args.len() < 3 {
                return;
            }
            (
                args[0].clone(),
                parse_param_list(&args[1]),
                args[2].clone(),
                arg_tokens.get(2).copied(),
                arg_tokens.get(1).copied(),
            )
        } else {
            // constructor / onconfigure: ARGLIST BODY (name is synthetic).
            if args.len() < 2 {
                return;
            }
            (
                synthetic_name.to_string(),
                parse_param_list(&args[0]),
                args[1].clone(),
                arg_tokens.get(1).copied(),
                arg_tokens.first().copied(),
            )
        };

        let zero = Span::new(0, 0);
        let name_span = arg_tokens.first().map_or(zero, |t| t.span);
        let body_span = body_tok.map_or(name_span, |t| t.span);
        let method_def = MethodDef {
            name: name.clone(),
            params: params.clone(),
            name_span,
            body_span,
            kind: kind.to_string(),
            visibility: "public".to_string(),
            doc: String::new(),
        };
        match kind {
            "constructor" => class_def.constructors.push(method_def),
            "destructor" => class_def.destructor = Some(method_def),
            "classmethod" => {
                class_def.class_methods.insert(name.clone(), method_def);
            }
            _ => {
                class_def.methods.insert(name.clone(), method_def);
            }
        }

        // Walk the body in a method scope seeded with the params + seed vars,
        // reusing the TclOO method-body walker (it pre-binds the params and the
        // supplied vars as never-warn locals, then analyses the body).
        if let Some(bt) = body_tok {
            let mb = CollectedMethodBody {
                name,
                params,
                body_text,
                body_tok: bt,
                params_tok,
            };
            self.walk_method_body(seed_vars, class_qualified, scope_path, &mb);
        }
    }

    /// Detect dispatch shape of a user-defined ``unknown`` proc.
    ///
    /// Lowers the proc body to IR, then walks the resulting
    /// top-level [`Statement`]s looking for:
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

        // Lower to IR.  On panic, be conservative — assume fully
        // dynamic — by returning an ``UnknownProcInfo`` with
        // every dynamic flag set so the W123 emitter suppresses
        // unresolved-command warnings file-wide (the safe
        // direction when we couldn't analyse the handler body).
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
    /// Reuses [`apply_oo_subcommand`] — the inline form differs
    /// from the body form only in how arguments are framed; the
    /// per-subcommand handling is identical.
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
/// loop is still detected.
fn walk_unknown_stmt(stmt: &Statement, first_param: &str, info: &mut UnknownProcInfo) {
    match stmt {
        Statement::Switch {
            subject,
            arms,
            mode,
            ..
        } => {
            // Subject reference: ``$first`` or ``${first}``
            // (both forms are checked).
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
/// `oo::define Cls private <subcmd> ...` — wraps a method-defining
/// subcommand with `visibility = "private"`.  Extracted from
/// [`apply_oo_subcommand`] to keep the dispatch under threshold.
/// A TclOO method body collected during the class-body walk, to be analysed
/// in a [`ScopeKind::Method`] scope once the whole `ClassDef` is populated.
struct CollectedMethodBody {
    /// Method name (`<constructor>` / `<destructor>` for those forms).
    name: String,
    /// Formal parameters (empty for `destructor`).
    params: Vec<ParamDef>,
    /// Inner body text (braces stripped), as `analyse_body` expects.
    body_text: String,
    /// The body word token (carries the absolute span + `content_offset`).
    body_tok: Token,
    /// The raw param-list word token (`{a b}`), used to anchor each formal
    /// parameter's definition span at its name (issue #727). `None` for
    /// `destructor` (no parameter list).
    params_tok: Option<Token>,
}

/// Recognise a method-defining subcommand in a class body and return its body
/// to walk.  Covers the direct forms (`method` / `classmethod` / `constructor`
/// / `destructor`); the `forward` form has no body, and dynamic (non-braced)
/// bodies are filtered downstream by [`Analyser::walk_method_body`].
fn collect_method_body(texts: &[String], argv: &[Token]) -> Option<CollectedMethodBody> {
    match texts.first().map(String::as_str)? {
        "method" | "classmethod" if texts.len() >= 4 => Some(CollectedMethodBody {
            name: texts[1].clone(),
            params: parse_param_list(&texts[2]),
            body_text: texts[3].clone(),
            body_tok: *argv.get(3)?,
            params_tok: argv.get(2).copied(),
        }),
        "constructor" if texts.len() >= 3 => Some(CollectedMethodBody {
            name: "<constructor>".to_string(),
            params: parse_param_list(&texts[1]),
            body_text: texts[2].clone(),
            body_tok: *argv.get(2)?,
            params_tok: argv.get(1).copied(),
        }),
        "destructor" if texts.len() >= 2 => Some(CollectedMethodBody {
            name: "<destructor>".to_string(),
            params: Vec::new(),
            body_text: texts[1].clone(),
            body_tok: *argv.get(1)?,
            params_tok: None,
        }),
        _ => None,
    }
}

fn apply_oo_private(sub_args: &[String], sub_tokens: &[Token], class_def: &mut ClassDef) {
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
            if let Some(md) = extract_method_def(inner_args, inner_tokens, "method", "private", "")
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

/// `oo::define Cls forward name target ...` — records a forward
/// alias as a method.
fn apply_oo_forward(sub_args: &[String], sub_tokens: &[Token], class_def: &mut ClassDef) {
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

fn apply_oo_subcommand(texts: &[String], argv: &[Token], class_def: &mut ClassDef) {
    let Some(subcmd) = texts.first().map(String::as_str) else {
        return;
    };
    let sub_args: &[String] = if texts.len() > 1 { &texts[1..] } else { &[] };
    let sub_tokens: &[Token] = if argv.len() > 1 { &argv[1..] } else { &[] };

    match subcmd {
        "superclass" => {
            class_def.superclasses = sub_args.to_vec();
            for (i, name) in sub_args.iter().enumerate() {
                if let Some(tok) = sub_tokens.get(i) {
                    class_def.superclass_refs.push((name.clone(), tok.span));
                }
            }
        }
        "mixin" => {
            // Skip ``-append`` and similar flags.
            class_def.mixins = sub_args
                .iter()
                .filter(|a| !a.starts_with('-'))
                .cloned()
                .collect();
            for (i, name) in sub_args.iter().enumerate() {
                if name.starts_with('-') {
                    continue;
                }
                if let Some(tok) = sub_tokens.get(i) {
                    class_def.mixin_refs.push((name.clone(), tok.span));
                }
            }
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
            if let Some(mut md) = extract_method_def(
                sub_args,
                sub_tokens,
                "constructor",
                "public",
                "<constructor>",
            ) {
                // Anchor the name span on the `constructor`
                // keyword token (argv[0]) — there's no name
                // token of its own, so editors land on the
                // keyword for go-to-definition / hover.
                if let Some(kw) = argv.first() {
                    md.name_span = kw.span;
                }
                class_def.constructors.push(md);
            }
        }
        "destructor" => {
            if let Some(mut md) =
                extract_method_def(sub_args, sub_tokens, "destructor", "public", "<destructor>")
            {
                if let Some(kw) = argv.first() {
                    md.name_span = kw.span;
                }
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
        "forward" => apply_oo_forward(sub_args, sub_tokens, class_def),
        "private" => apply_oo_private(sub_args, sub_tokens, class_def),
        // ``initialise`` / ``initialize`` are class-level
        // initialisation scripts; their bodies are collected and
        // walked separately in
        // [`Analyser::parse_oo_definition_body`].  The subcommand
        // is recognised here so the `_` arm doesn't silently drop
        // it.
        "initialise" | "initialize" => {}
        _ => {}
    }
}

/// Extract property definitions from a ``property`` subcommand.
///
/// Walks the args, splitting names from option values
/// (``-get BODY``, ``-set BODY``, ``-kind readable|writable|readwrite``).
/// Each property gets a [`PropertyDef`] entry in
/// ``class_def.properties``.
///
/// All property options take a value (``-get``, ``-set``,
/// ``-kind``); there are no flag-only options.  When ``-kind``
/// is omitted the property defaults to ``"readwrite"``.
///
/// This records only the class-level PropertyDef entries; the
/// accessor (`-get` / `-set`) bodies are collected and walked
/// separately by [`collect_property_accessor_bodies`].
fn extract_property_defs(args: &[String], arg_tokens: &[Token], class_def: &mut ClassDef) {
    let zero = Span::new(0, 0);

    // Collect property names + their per-arg index, then the
    // trailing options, in a two-pass shape.
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

/// Collect the `-get` / `-set` accessor bodies of a `property` subcommand as
/// walkable method bodies (named `<get>` / `<set>`).  Only braced (`Str`)
/// bodies are walkable.
fn collect_property_accessor_bodies(
    texts: &[String],
    argv: &[Token],
    out: &mut Vec<CollectedMethodBody>,
) {
    let mut i = 0;
    while i < texts.len() {
        if let Some(opt) = texts[i].strip_prefix('-') {
            // Every property option takes a value; only `-get`/`-set` carry a
            // body to analyse.
            if i + 1 < texts.len() {
                if matches!(opt, "get" | "set")
                    && let Some(tok) = argv.get(i + 1).copied()
                    && tok.kind == TokenType::Str
                {
                    out.push(CollectedMethodBody {
                        name: format!("<{opt}>"),
                        params: Vec::new(),
                        body_text: texts[i + 1].clone(),
                        body_tok: tok,
                        params_tok: None,
                    });
                }
                i += 2;
                continue;
            }
        }
        i += 1;
    }
}

/// Extract a [`MethodDef`] from method-style args.
///
/// Three shapes:
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
        // Name span anchors on the `constructor` keyword token
        // (argv[0] = 0..11), not the default (0, 0).
        assert_eq!(cd.constructors[0].name_span, tcl_lexer::Span::new(0, 11));
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
        // Name span anchors on the `destructor` keyword token
        // (argv[0] = 0..10).
        assert_eq!(dtor.name_span, tcl_lexer::Span::new(0, 10));
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

    // snit (tcllib) type / widget support.  Verified against real
    // `tclsh8.6` + tcllib.

    #[test]
    fn snit_type_recorded_as_class_with_members() {
        let src = "snit::type ::foo::Bar {\n\
                   variable v1\n\
                   typevariable tv1\n\
                   method m1 {a b} { return [expr {$a+$b+$v1}] }\n\
                   typemethod tm1 {} { return $tv1 }\n\
                   constructor {args} { set v1 0 }\n\
                   destructor { unset v1 }\n\
                   typeconstructor { set tv1 0 }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let c = r.all_classes.get("::foo::Bar").expect("Bar class recorded");
        assert_eq!(c.metaclass, "snit::type");
        assert!(c.methods.contains_key("m1"));
        assert!(c.class_methods.contains_key("tm1"));
        assert!(c.class_methods.contains_key("<typeconstructor>"));
        assert_eq!(c.constructors.len(), 1);
        assert!(c.destructor.is_some());
        assert_eq!(c.variables, vec!["v1".to_string(), "tv1".to_string()]);
    }

    #[test]
    fn snit_widget_keeps_win_and_hull_vars() {
        // A snit::widget injects `win` and `hull` instance variables — both
        // are recorded (only the four implicit scalars are filtered).
        let src = "snit::widget Dial {\n\
                   variable state\n\
                   method draw {} { return $win }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let c = r.all_classes.get("::Dial").expect("Dial recorded");
        assert_eq!(c.metaclass, "snit::widget");
        assert_eq!(
            c.variables,
            vec!["win".to_string(), "hull".to_string(), "state".to_string()]
        );
    }

    #[test]
    fn snit_method_body_suppresses_self_dispatch_and_implicit_vars() {
        // Inside a snit method, `$self`/`$component` dispatch and reads of
        // instance variables must not false-fire W307 (non-literal command),
        // W210 (read-before-set), or W211/W214 (unused).
        let src = "snit::widget mywidget {\n\
                   variable helper\n\
                   component inner\n\
                   method draw {} {\n\
                       $self configure -bg white\n\
                       $inner render\n\
                       $helper compute\n\
                       return $win\n\
                   }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        for code in ["W210", "W211", "W214", "W307", "W308"] {
            assert!(
                !r.diagnostics.iter().any(|d| d.code.as_str() == code),
                "{code} must not fire in a snit method body: {:?}",
                r.diagnostics
            );
        }
    }

    #[test]
    fn snit_widgetadaptor_and_qualified_definer() {
        let mut a = Analyser::new();
        let r = a.analyse("::snit::widgetadaptor Foo { method m {} {} }", "tcl8.6");
        let c = r.all_classes.get("::Foo").expect("Foo recorded");
        assert_eq!(c.metaclass, "::snit::widgetadaptor");
        assert!(c.methods.contains_key("m"));
    }

    #[test]
    fn non_snit_command_is_not_a_class() {
        // A plain command that merely starts with `snit` is not a definer.
        let mut a = Analyser::new();
        let r = a.analyse("snitch foo { bar }", "tcl8.6");
        assert!(r.all_classes.is_empty());
    }

    // OO body-walks: `initialise` body, `property -get/-set` accessor bodies,
    // and the `new` / `createWithNamespace` class-command variants.

    #[test]
    fn oo_initialise_body_is_walked() {
        // A `variable` read inside `initialise { … }` must not false-fire
        // W210 read-before-set — the class-level init body is walked with the
        // class's instance variables visible.
        let src = "oo::class create Foo {\n\
                   variable cache\n\
                   initialise { set cache [dict create] }\n\
                   method get {k} { return [dict get $cache $k] }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| matches!(d.code.as_str(), "W210" | "W211")),
            "initialise body should be walked cleanly: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn oo_property_accessor_bodies_are_walked() {
        // `-get`/`-set` accessor bodies are walked with the instance variable
        // `val` and the implicit `value` visible — no false W210 / W307.
        let src = "oo::configurable create Bar {\n\
                   variable val\n\
                   property color -get { return $val } -set { set val $value }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        assert!(r.all_classes.contains_key("::Bar"));
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| matches!(d.code.as_str(), "W210" | "W307")),
            "property accessor bodies should be walked cleanly: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn oo_class_create_with_namespace_is_recognised() {
        // The `createWithNamespace` class-command variant introduces a class.
        let mut a = Analyser::new();
        let r = a.analyse("oo::class createWithNamespace MyCls ::ns { }", "tcl8.6");
        assert!(
            r.all_classes.keys().any(|k| k.contains("MyCls")),
            "createWithNamespace should record a class: {:?}",
            r.all_classes.keys().collect::<Vec<_>>()
        );
    }
}
