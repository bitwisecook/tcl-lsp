// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional deferred names from a retained original installer world.
//! Materialized callbacks receive no original child words, future lookup or frame.

use std::cell::OnceCell;
use std::collections::HashMap;

use tcl_lexer::LexerConfig;
use tcl_registry::{ArgRole, BodyKind, CommandRegistry, ScriptLookupScope, ScriptTiming, Traits};

use crate::cfg_builder::global_write_info::{
    GlobalWriteInfo, detect_global_write_procs_with_registry,
    own_frame_global_writes_with_metadata_context,
};
use crate::command_binding::ModuleCommandBindings;
use crate::ir::{CommandTokens, DeferredWrites, ExecutionNamespace, Module, Script, Statement};
use crate::ir_helpers::{CommandWord, VariableWriteEffects, tokenise_command_words};
use crate::registry_invocation::{
    InvocationMetadataContext, original_callback_invocation_with_metadata_context,
};

pub(super) fn scan_module(module: &Module, registry: &CommandRegistry) -> DeferredWrites {
    if InvocationMetadataContext::for_module(registry, module).is_none() {
        return DeferredWrites {
            any: true,
            ..Default::default()
        };
    }
    let mut scan = SourceScan {
        module,
        registry,
        procedures: OnceCell::new(),
        out: DeferredWrites::default(),
    };
    let mut pending = vec![&module.top_level];
    pending.extend(module.procedures.values().map(|procedure| &procedure.body));
    pending.extend(module.methods.values().map(|method| &method.body));
    pending.extend(
        module
            .redefined_methods
            .values()
            .flatten()
            .map(|method| &method.body),
    );
    pending.extend(module.body_units.values().map(|unit| &unit.body));
    while let Some(script) = pending.pop() {
        for statement in &script.statements {
            pending.extend(statement.child_scripts());
            scan.statement(script, statement);
        }
    }
    scan.out
}

struct SourceScan<'a> {
    module: &'a Module,
    registry: &'a CommandRegistry,
    procedures: OnceCell<HashMap<String, GlobalWriteInfo>>,
    out: DeferredWrites,
}

struct ScriptValue {
    text: Option<String>,
    scope: Option<ScriptLookupScope>,
}

struct SelectedValues {
    deferred: Vec<ScriptValue>,
    immediate: Vec<String>,
    lambdas: Vec<Option<String>>,
}

struct ValueContext<'a> {
    metadata: InvocationMetadataContext<'a>,
    namespace: ExecutionNamespace,
    global_namespace: Option<ExecutionNamespace>,
    config: LexerConfig,
}

impl SourceScan<'_> {
    fn statement(&mut self, script: &Script, statement: &Statement) {
        if !statement.is_executable_invocation() {
            return;
        }
        let Some(tokens) = script.retained_source_tokens_for_statement(statement) else {
            self.out.any = true;
            return;
        };
        self.original_registration(tokens);
        for (_, binding) in &tokens.nested_bindings {
            let Some((_, mut nested)) = binding.original_recorded_command() else {
                self.out.any = true;
                continue;
            };
            // The original vector comes from this exact nested source receipt.
            // Setting its retained binding restores that receipt, not a new lookup.
            nested.source_binding = Some(binding.clone());
            self.original_registration(&nested);
        }
    }

    fn original_registration(&mut self, tokens: &CommandTokens) {
        let Some(binding) = tokens.source_binding.as_ref() else {
            self.out.any = true;
            return;
        };
        let Some(footprint) =
            binding.original_materialized_footprint_for_module(tokens, self.module, self.registry)
        else {
            self.out.any = true;
            return;
        };
        let metadata = footprint.metadata_context();
        let Some(invocation) =
            original_callback_invocation_with_metadata_context(self.registry, metadata, tokens)
        else {
            self.out.any = true;
            return;
        };
        let selected = invocation.with_metadata_schema(
            self.registry,
            metadata,
            footprint.bindings().invocation_realm(),
            |schema| selected_values(schema, metadata.permits_logical_source_names()),
        );
        let Some(selected) = selected else {
            self.out.any = true;
            return;
        };
        let global = footprint.deferred_namespace(Some(ScriptLookupScope::GlobalFrame));
        for value in selected.deferred {
            let Some(namespace) = footprint.deferred_namespace(value.scope) else {
                self.out.any = true;
                continue;
            };
            let Some(text) = value.text else {
                self.out.any = true;
                continue;
            };
            let context = ValueContext {
                metadata,
                namespace,
                global_namespace: global.clone(),
                config: footprint.lexer_config(),
            };
            self.value_script(&text, footprint.bindings(), &context, true, 0);
        }
    }

    fn include(&mut self, effects: &VariableWriteEffects) {
        self.out.any |= effects.opaque;
        for name in &effects.names {
            super::Scan::note_name(&mut self.out, name);
        }
    }

    fn value_script(
        &mut self,
        text: &str,
        initial: &ModuleCommandBindings,
        context: &ValueContext<'_>,
        include_names: bool,
        depth: u32,
    ) {
        if crate::depth_guard::MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
            self.out.any = true;
            return;
        }
        if include_names {
            let effects = crate::ir_helpers::script_value_name_ownership_with_metadata_context(
                text,
                self.registry,
                initial,
                &context.namespace,
                context.metadata,
                context.config,
                crate::script_binds::Ownership::Bindings,
            );
            self.include(&effects);
        }
        let mut state = initial.clone();
        for words in tokenise_command_words(text, context.config) {
            self.value_command(&words, &state, context, depth);
            state.source_order_registry_barrier_for_command_with_metadata_context(
                &words,
                false,
                self.registry,
                &context.namespace,
                Traits::EVALUATES_CODE,
                Some(context.metadata),
            );
        }
    }

    fn value_command(
        &mut self,
        words: &[CommandWord],
        state: &ModuleCommandBindings,
        context: &ValueContext<'_>,
        depth: u32,
    ) {
        for word in words.iter().filter(|word| word.substituted) {
            for text in crate::var_refs::command_subst_texts_with_config(&word.raw, context.config)
            {
                self.value_script(&text, state, context, false, depth + 1);
            }
        }
        let Some(holder) = words
            .first()
            .and_then(CommandWord::literal)
            .and_then(|head| context.namespace.for_head_context(head))
        else {
            self.out.any = true;
            return;
        };
        let mut selected = Vec::new();
        let mut unknown = false;
        state.for_each_resolved_command_words(words, holder.as_ref(), |target, arguments| {
            if !target.registry_backed {
                unknown = true;
                return;
            }
            let answer =
                tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                    self.registry,
                    Some(context.metadata.context()),
                    arguments,
                    state.invocation_realm(),
                );
            match answer.resolved().and_then(|schema| {
                selected_values(&schema, context.metadata.permits_logical_source_names())
            }) {
                Some(values) => selected.push(values),
                None => unknown = true,
            }
        });
        self.out.any |= unknown || selected.is_empty();
        for values in selected {
            for text in values.immediate {
                self.value_script(&text, state, context, false, depth + 1);
            }
            for value in values.deferred {
                let Some(namespace) = (value.scope == Some(ScriptLookupScope::GlobalFrame))
                    .then(|| context.global_namespace.clone())
                    .flatten()
                else {
                    self.out.any = true;
                    continue;
                };
                let Some(text) = value.text else {
                    self.out.any = true;
                    continue;
                };
                let future = ValueContext {
                    metadata: context.metadata,
                    namespace,
                    global_namespace: context.global_namespace.clone(),
                    config: context.config.nested(),
                };
                self.value_script(&text, state, &future, true, depth + 1);
            }
            for lambda in values.lambdas {
                match lambda {
                    Some(lambda) => self.lambda(&lambda, state, context, depth + 1),
                    None => self.out.any = true,
                }
            }
        }
    }

    fn lambda(
        &mut self,
        text: &str,
        state: &ModuleCommandBindings,
        context: &ValueContext<'_>,
        depth: u32,
    ) {
        if crate::depth_guard::MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
            self.out.any = true;
            return;
        }
        let Ok(elements) =
            tcl_syntax::word_rules::WordValueRules::from_config(&context.config).split_list(text)
        else {
            return;
        };
        let (Some(body), 2..=3) = (elements.get(1), elements.len()) else {
            return;
        };
        let Some(input) = context.metadata.source_analysis_input() else {
            self.out.any = true;
            return;
        };
        let config = context.config.nested().normalized();
        let lowered = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(self.registry, config)
                .with_resolved_analysis_input(input.for_nested_source()),
            body,
        );
        let namespace = elements.get(2).map_or_else(
            || "::".to_owned(),
            |namespace| tcl_syntax::naming::qualify("::", namespace),
        );
        let procedures = self
            .procedures
            .get_or_init(|| detect_global_write_procs_with_registry(self.module, self.registry));
        let writes = own_frame_global_writes_with_metadata_context(
            &lowered.top_level,
            self.registry,
            state,
            &namespace,
            procedures,
            Some(context.metadata),
            config,
        );
        self.out.any |= writes.opaque_global_frame;
        for name in writes.names {
            super::Scan::note_name(&mut self.out, &name);
        }
        // Locals of the applied lambda do not become deferred global names.
        // Its own selected installers and lambdas retain separate queries.
        let body_context = ValueContext {
            metadata: context.metadata,
            namespace: ExecutionNamespace::exact(namespace),
            global_namespace: context.global_namespace.clone(),
            config,
        };
        self.value_script(body, state, &body_context, false, depth + 1);
    }
}

fn selected_values(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    logical: bool,
) -> Option<SelectedValues> {
    let (roles, complete) = if logical {
        schema.authored_logical_source_argument_roles()
    } else {
        schema.authored_source_argument_roles()
    };
    if !complete {
        return None;
    }
    let scripts = if logical {
        schema.authored_logical_source_plain_script_arguments()
    } else {
        schema.authored_source_plain_script_arguments()
    }?;
    let arguments = schema.words.arguments();
    let facts = schema.facts();
    let mut values = SelectedValues {
        deferred: Vec::new(),
        immediate: Vec::new(),
        lambdas: Vec::new(),
    };
    for ordinal in scripts {
        let value = if schema
            .semantics
            .traits
            .contains(Traits::SCRIPT_CONCATENATES_ARGS)
        {
            let literals = (ordinal..arguments.len())
                .map(|index| arguments.literal_at(index))
                .collect::<Option<Vec<_>>>();
            literals.and_then(|literals| {
                match arguments.dialect().and_then(|dialect| dialect.family()) {
                    Some(tcl_dialect::model::Family::Jim) => {
                        Some(tcl_syntax::list::concat_values_jim(literals))
                    }
                    Some(
                        tcl_dialect::model::Family::Tcl
                        | tcl_dialect::model::Family::F5Tcl
                        | tcl_dialect::model::Family::F5Irules,
                    ) => Some(tcl_syntax::list::concat_values(literals)),
                    None if logical => Some(tcl_syntax::list::concat_values(literals)),
                    _ => None,
                }
            })
        } else {
            arguments.literal_at(ordinal).map(str::to_owned)
        };
        match schema.authored_source_script_timing_at(ordinal) {
            Some(ScriptTiming::Deferred) => values.deferred.push(ScriptValue {
                text: value,
                scope: facts.script_lookup_scope(ordinal),
            }),
            Some(ScriptTiming::SameInvocation)
                if schema.semantics.body_kind == BodyKind::Plain
                    && schema.semantics.body_interpreter.resolve(arguments)
                        == tcl_registry::InterpreterScope::Current =>
            {
                if let Some(value) = value {
                    values.immediate.push(value);
                }
            }
            Some(ScriptTiming::SameInvocation | ScriptTiming::ReferenceOnly) => {}
            None => return None,
        }
    }
    for (index, role) in roles {
        if role == ArgRole::LambdaLiteral {
            values.lambdas.push(
                arguments
                    .literal_at(usize::from(index) + schema.semantics.argument_offset)
                    .map(str::to_owned),
            );
        }
    }
    Some(values)
}
