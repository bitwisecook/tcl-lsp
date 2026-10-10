// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact slot inputs for source rewrites and retained native frame layouts.

use crate::command_binding::formal_topology::OriginalFormalTopology;
use crate::ir::{Module, Procedure};
use crate::var_resolve::{VariableCellKey, VariableExecutionFrame};
use tcl_core_types::NameBytes;

/// Original fixed scalar argument slots for a proposed source calling convention.
/// These ordinals are argument positions, never a physical compiled-local layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalScalarArgumentSlots {
    input: crate::signature_scan::original_name::SignatureSourceNameKey,
    names: Vec<NameBytes>,
}

impl OriginalScalarArgumentSlots {
    pub(crate) fn from_topology(topology: &OriginalFormalTopology) -> Option<Self> {
        if topology
            .parameters()
            .iter()
            .any(|formal| formal.default.is_some())
        {
            return None;
        }
        let names = topology.fixed_scalar_binding_names(topology.parameters().len())?;
        if names.len() != topology.parameters().len() {
            return None;
        }
        if names
            .iter()
            .enumerate()
            .any(|(index, name)| names[..index].contains(name))
        {
            return None;
        }
        Some(Self {
            input: topology.original_input().clone(),
            names,
        })
    }

    pub(crate) fn independent_compiler_arguments(
        &self,
        protocol: tcl_syntax::naming::NativeCompiledVariableProtocol,
    ) -> bool {
        self.names.iter().enumerate().all(|(index, name)| {
            self.names.iter().position(|candidate| {
                protocol.compiled_local_names_equal(candidate.as_bytes(), name.as_bytes())
            }) == Some(index)
        })
    }

    /// Actual counted scalar formal keys in argument order.
    #[must_use]
    pub fn names(&self) -> &[NameBytes] {
        &self.names
    }

    /// Complete original `ParamList` producer, independent of activation entry.
    #[must_use]
    pub fn original_input(&self) -> &crate::signature_scan::original_name::SignatureSourceNameKey {
        &self.input
    }

    /// Exact argument position; display labels cannot select this ordinal.
    #[must_use]
    pub fn ordinal(&self, original: &[u8]) -> Option<u32> {
        u32::try_from(
            self.names
                .iter()
                .position(|name| name.as_bytes() == original)?,
        )
        .ok()
    }
}

pub(crate) fn original_procedure_topology<'a>(
    module: &'a Module,
    procedure: &Procedure,
) -> Option<&'a OriginalFormalTopology> {
    let mut selected = None;
    for body in module
        .procedure_implementation_bodies
        .iter()
        .filter(|body| {
            body.allocation
                .matches_source_declaration_image(&module.source, procedure.span.start())
                && procedure.body.executed_source.as_deref() == Some(&body.source)
        })
    {
        let topology = body.original_parameters.as_ref()?;
        let input = topology.original_input();
        if input.source_image() != &module.source
            || input.lexer_config() != module.native_lexer_config()
        {
            return None;
        }
        if selected.is_some_and(|previous| previous != topology) {
            return None;
        }
        selected = Some(topology);
    }
    selected
}

pub(crate) fn original_procedure_argument_slots(
    module: &Module,
    procedure: &Procedure,
) -> Option<OriginalScalarArgumentSlots> {
    let topology = original_procedure_topology(module, procedure)?;
    let slots = OriginalScalarArgumentSlots::from_topology(topology)?;
    let protocol = module.source_entry.options().compiled_variable_protocol()?;
    // A formal prefix precedes every body allocation. Validate first-primary
    // selection independently of argument binding; full byte uniqueness alone
    // does not close C's bounded CString compiler comparison.
    slots
        .independent_compiler_arguments(protocol)
        .then_some(slots)
}

/// One actual retained native layout slot. Source-formal and escape inventories
/// cannot construct this receipt or establish its current layout incarnation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalNativeFrameSlot {
    entry: std::sync::Arc<tcl_runtime_api::NativeCompilationEntry>,
    layout: tcl_runtime_api::native_compilation::NativeCompiledLocalLayout,
    frame: VariableExecutionFrame,
    name: NameBytes,
    ordinal: u32,
}

impl OriginalNativeFrameSlot {
    pub(crate) fn from_entry(module: &Module, cell: &VariableCellKey) -> Option<Self> {
        use tcl_runtime_api::native_compilation::NativeCompilationFrame;
        use tcl_syntax::naming::{
            NativeCompiledVariableAuthority, NativeCompiledVariableEnvironment,
        };
        let entry = module.source_entry.native_entry.as_ref()?;
        let layout = entry.compiled_local_layout.as_ref()?;
        let protocol = entry.compiled_variable_protocol?;
        if module.top_level_kind != crate::ir::TopLevelKind::Script
            || entry.frame != NativeCompilationFrame::Procedure
            || layout.owner != entry.interpreter
            || layout.token == 0
            || protocol.authority() != NativeCompiledVariableAuthority::Native
            || !protocol.supports_environment(NativeCompiledVariableEnvironment::BorrowFrameSlots)
        {
            return None;
        }
        let frame = crate::command_binding::source_analysis_frame(
            Some(entry),
            &module.top_level_namespace,
            module.top_level_kind,
        );
        let VariableExecutionFrame::Procedure { identity, .. } = frame.layout() else {
            return None;
        };
        // A lifetime or element key is not an ordinary incoming scalar slot.
        let VariableCellKey::Activation {
            identity: owner,
            simple,
        } = cell
        else {
            return None;
        };
        if owner != identity {
            return None;
        }
        let table = tcl_bytecode::LocalVarTable::from_native_slot_names(&layout.names);
        let ordinal = table.find_native(protocol, simple.as_bytes())?;
        // The comparator may select another counted primary. Its actual key,
        // rather than the requested spelling, must be the represented SSA cell.
        if layout.names.get(ordinal)?.as_ref()? != simple {
            return None;
        }
        Some(Self {
            entry: std::sync::Arc::clone(entry),
            layout: layout.clone(),
            frame,
            name: simple.clone(),
            ordinal: u32::try_from(ordinal).ok()?,
        })
    }

    /// Independently selected entry activation owning this layout use.
    #[must_use]
    pub fn frame(&self) -> &VariableExecutionFrame {
        &self.frame
    }

    /// Exact retained primary, without a display conversion.
    #[must_use]
    pub fn name(&self) -> &NameBytes {
        &self.name
    }

    /// Actual native index, including preceding anonymous slots.
    #[must_use]
    pub const fn ordinal(&self) -> u32 {
        self.ordinal
    }

    /// Retained layout owner and incarnation required by later execution.
    #[must_use]
    pub fn layout(&self) -> &tcl_runtime_api::native_compilation::NativeCompiledLocalLayout {
        &self.layout
    }

    /// Check the actual point's activation, namespace owner and name policy.
    /// Layout currency alone cannot lend this slot to a different frame.
    #[must_use]
    pub(crate) fn matches_current_activation(
        &self,
        context: &crate::var_resolve::ResolveContext,
    ) -> bool {
        let VariableExecutionFrame::Procedure { identity, .. } = self.frame.layout() else {
            return false;
        };
        let Some(namespace) = self.frame.namespace_identity() else {
            return false;
        };
        let Some(policy) = self.entry.execution_name_policy() else {
            return false;
        };
        context.frame_kind == crate::var_resolve::VariableFrameKind::Local
            && context.activation.as_ref() == Some(identity)
            && context.selected_frame.is_none()
            && context.namespace_identity.as_ref() == Some(namespace)
            && context.execution_name_policy == Some(policy)
    }

    /// Recheck the complete native compilation world, separately from contents
    /// and observers. Equal labels or a later source layout cannot replace it.
    #[must_use]
    pub fn matches_entry(&self, entry: &tcl_runtime_api::NativeCompilationEntry) -> bool {
        self.entry.same_compilation_world(entry)
            && entry.compiled_local_layout.as_ref() == Some(&self.layout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span};
    use tcl_registry::InvocationDialect;
    use tcl_syntax::word_rules::WordValueRules;

    fn topology(parameters: &[u8], engine: &str) -> OriginalFormalTopology {
        let dialect = InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some(engine)).unwrap(),
        );
        let config = LexerConfig::from_grammar(dialect.lexer_grammar);
        let mut source = b"proc p ".to_vec();
        source.extend_from_slice(parameters);
        source.extend_from_slice(b" {return}");
        let image = SourceImage::native(source.as_slice());
        let words = tcl_lexer::native_script_words_in(
            image.clone(),
            Span::new(0, u32::try_from(image.len()).unwrap()),
            config,
        )
        .unwrap();
        let input = crate::signature_scan::original_name::SignatureSourceNameKey::from_original_native_word(
            &words.commands[0].words[2], WordValueRules::from_config(&config), dialect.authored_name_policy().unwrap(),
        ).unwrap();
        OriginalFormalTopology::from_original_key(input, dialect).unwrap()
    }

    #[test]
    // Implementation contract: naming.variable.aot-original-slot-purpose
    // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
    fn original_argument_slots_keep_counted_order_and_reject_nonpositional_bindings() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let original = topology(br"{a {b} n\uD800}", engine);
            let slots = OriginalScalarArgumentSlots::from_topology(&original).unwrap();
            assert_eq!(
                slots.names(),
                &[
                    NameBytes::from(b"a"),
                    NameBytes::from(b"b"),
                    NameBytes::from(b"n\xed\xa0\x80")
                ],
                "{engine}"
            );
            assert_eq!(slots.ordinal(b"n\xed\xa0\x80"), Some(2), "{engine}");
            assert_eq!(slots.ordinal(br"n\uD800"), None, "{engine}");
            assert_eq!(slots.original_input(), original.original_input());
            for rejected in [b"{a a}".as_slice(), b"{a {optional DEFAULT}}", b"{a args}"] {
                assert!(
                    OriginalScalarArgumentSlots::from_topology(&topology(rejected, engine))
                        .is_none(),
                    "{engine}"
                );
            }
        }
        assert!(
            OriginalScalarArgumentSlots::from_topology(&topology(b"{&linked}", "jim")).is_none()
        );
    }

    fn native_module(names: Vec<Option<NameBytes>>) -> Module {
        use tcl_runtime_api::native_compilation::{
            NativeCompilationFrame, NativeCompiledLocalLayout, NativeCompiledLocalLayoutKind,
        };
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let mut entry = crate::environment_ingress::captured_native_entry(profile);
        entry.frame = NativeCompilationFrame::Procedure;
        entry.compiled_local_layout = Some(NativeCompiledLocalLayout {
            owner: entry.interpreter,
            token: 7,
            epoch: 3,
            kind: NativeCompiledLocalLayoutKind::RetainedLocalCache,
            names,
        });
        Module {
            source_entry: crate::command_binding::SourceAnalysisEntry {
                native_entry: Some(std::sync::Arc::new(entry)),
                ..Default::default()
            },
            top_level_namespace: "::".to_owned(),
            ..Default::default()
        }
    }

    fn local_cell(module: &Module, name: &[u8]) -> VariableCellKey {
        let frame = crate::command_binding::source_analysis_frame(
            module.source_entry.native_entry.as_deref(),
            &module.top_level_namespace,
            module.top_level_kind,
        );
        let VariableExecutionFrame::Procedure { identity, .. } = frame.layout() else {
            panic!("genuine fixture procedure frame");
        };
        VariableCellKey::Activation {
            identity: identity.clone(),
            simple: name.into(),
        }
    }

    #[test]
    // Implementation contract: naming.variable.aot-original-slot-purpose
    // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
    fn original_native_slots_require_selected_primary_frame_and_current_layout() {
        let module = native_module(vec![
            None,
            Some(b"k\0a".as_slice().into()),
            Some(b"k\0b".as_slice().into()),
            Some(b"\xff".as_slice().into()),
        ]);
        let first = local_cell(&module, b"k\0a");
        let slot = OriginalNativeFrameSlot::from_entry(&module, &first).unwrap();
        assert_eq!(slot.ordinal(), 1);
        assert_eq!(slot.name().as_bytes(), b"k\0a");
        assert_eq!(slot.layout().names[0], None);
        assert!(slot.matches_entry(module.source_entry.native_entry.as_deref().unwrap()));
        assert!(
            OriginalNativeFrameSlot::from_entry(&module, &local_cell(&module, b"k\0b")).is_none()
        );
        assert_eq!(
            OriginalNativeFrameSlot::from_entry(&module, &local_cell(&module, b"\xff"))
                .unwrap()
                .ordinal(),
            3
        );
        assert!(
            OriginalNativeFrameSlot::from_entry(
                &module,
                &VariableCellKey::Authored("k\0a".to_owned())
            )
            .is_none()
        );
        let mut foreign = first.clone();
        if let VariableCellKey::Activation { identity, .. } = &mut foreign {
            identity.push_str(":foreign");
        }
        assert!(OriginalNativeFrameSlot::from_entry(&module, &foreign).is_none());
        let mut replacement = module.clone();
        std::sync::Arc::make_mut(replacement.source_entry.native_entry.as_mut().unwrap())
            .compiled_local_layout
            .as_mut()
            .unwrap()
            .epoch += 1;
        assert!(!slot.matches_entry(replacement.source_entry.native_entry.as_deref().unwrap()));
        let mut unavailable = module.clone();
        std::sync::Arc::make_mut(unavailable.source_entry.native_entry.as_mut().unwrap())
            .compiled_variable_protocol = None;
        assert!(OriginalNativeFrameSlot::from_entry(&unavailable, &first).is_none());
        let mut different_owner = module.clone();
        std::sync::Arc::make_mut(different_owner.source_entry.native_entry.as_mut().unwrap())
            .compiled_local_layout
            .as_mut()
            .unwrap()
            .owner
            .owner += 1;
        assert!(OriginalNativeFrameSlot::from_entry(&different_owner, &first).is_none());
        let mut new_procedure = module;
        new_procedure.top_level_kind = crate::ir::TopLevelKind::ProcedureBody;
        assert!(OriginalNativeFrameSlot::from_entry(&new_procedure, &first).is_none());
    }

    #[test]
    // Implementation contract: naming.variable.aot-original-slot-purpose
    // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
    fn original_native_slot_usage_requires_the_actual_selected_activation() {
        let module = native_module(vec![Some(b"x".as_slice().into())]);
        let slot =
            OriginalNativeFrameSlot::from_entry(&module, &local_cell(&module, b"x")).unwrap();
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let image = SourceImage::document("info exists x");
        let bindings =
            crate::command_binding::SourceCommandBindings::analyse_image_in_frame_with_options(
                &image,
                slot.frame(),
                module.native_lexer_config(),
                registry,
                module.source_entry.options(),
            )
            .unwrap();
        let point = bindings.invocation_at_source("info", 0);
        let context = point.variable_context.as_ref();
        assert!(slot.matches_current_activation(context));

        let mut display_only = context.clone();
        display_only.namespace = "unrelated presentation".into();
        assert!(slot.matches_current_activation(&display_only));

        let counterfactuals: &[fn(&mut crate::var_resolve::ResolveContext)] = &[
            |context| context.activation.as_mut().unwrap().push_str(":foreign"),
            |context| context.frame_kind = crate::var_resolve::VariableFrameKind::Namespace,
            |context| context.selected_frame = Some(tcl_registry::FrameLevel::Relative(1)),
            |context| context.namespace_identity = None,
            |context| context.execution_name_policy = None,
        ];
        for (index, change) in counterfactuals.iter().enumerate() {
            let mut changed = context.clone();
            change(&mut changed);
            assert!(
                !slot.matches_current_activation(&changed),
                "independent current-frame input {index}"
            );
            assert!(slot.matches_entry(module.source_entry.native_entry.as_deref().unwrap()));
        }
    }

    #[test]
    // Native proof: naming.variable.compiled-versus-runtime-root-purpose
    // docs/design/analysis/name-resolution-proofs/variable.compiled-versus-runtime-root-purpose.md
    fn original_argument_primary_collision_matches_retained_native_compiled_results() {
        // Only the C8.6+ equal-count comparison/result is borrowed from this
        // capture. The separate receipt tests do not infer native layout data.
        for (engine, captured) in [
            (
                "tcl8.6",
                include_str!(
                    "../../../tcl-vm/tests/data/native_formal_slots/compiled-local-names-8.6.18.jsonl"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../tcl-vm/tests/data/native_formal_slots/compiled-local-names-9.0.4.jsonl"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../tcl-vm/tests/data/native_formal_slots/compiled-local-names-9.1.0.jsonl"
                ),
            ),
        ] {
            let row = captured
                .lines()
                .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
                .find(|row| {
                    row["case"] == "nul-equal-length"
                        && row["path"] == "two-original-formals"
                        && row["op"] == "compiled-and-dynamic-invocation"
                })
                .unwrap();
            assert_eq!(row["code"], 0);
            assert_eq!(
                row["result"],
                "30204f4e452030204f4e452030204f4e4520302054574f"
            );
            let original = topology(b"{k\0a k\0b}", engine);
            let arguments = OriginalScalarArgumentSlots::from_topology(&original).unwrap();
            assert_eq!(arguments.ordinal(b"k\0b"), Some(1));
            let protocol = InvocationDialect::of_point(
                tcl_dialect::model::DialectPoint::of_dialect_name(Some(engine)).unwrap(),
            )
            .native_compiled_variable_protocol()
            .unwrap();
            assert!(
                !arguments.independent_compiler_arguments(protocol),
                "{engine}"
            );
            assert_eq!(
                original.first_compiled_formal_name(b"k\0b", protocol, false),
                Some(b"k\0a".as_slice())
            );
        }
    }
}
