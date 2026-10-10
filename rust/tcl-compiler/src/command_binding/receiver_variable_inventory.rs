// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Own-provider original variable declarations, separate from lookup and storage.

use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_registry::definer::{DefinerFamily, DefinitionBodyGrammar, SlotOp, SlotSpec};
use tcl_syntax::naming::{NamePolicyProtocol, NativeOoVariableSlotOperation};

/// Counted declarations under one independently selected `TclOO` field recipe.
/// The caller retains the actual declaring provider; this inventory supplies
/// neither an entered receiver nor a compiler/local-table capability.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct OriginalReceiverVariableInventory {
    policy: NamePolicyProtocol,
    declarations: Vec<SignatureSourceNameInput>,
}

impl OriginalReceiverVariableInventory {
    pub(super) fn empty(
        grammar: &DefinitionBodyGrammar,
        policy: NamePolicyProtocol,
    ) -> Option<Self> {
        if grammar.family != DefinerFamily::TclOo {
            return None;
        }
        tcl_syntax::naming::validate_native_oo_variable(policy.recipe(), b"").ok()?;
        Some(Self {
            policy,
            declarations: Vec::new(),
        })
    }

    pub(super) fn apply(
        &mut self,
        slot: Option<SlotSpec>,
        arguments: Vec<SignatureSourceNameInput>,
    ) -> Option<()> {
        if arguments.iter().any(|input| input.policy() != self.policy) {
            return None;
        }
        // An empty field invocation does not change the retained declarations.
        if arguments.is_empty() {
            return Some(());
        }
        let bytes = arguments
            .iter()
            .map(SignatureSourceNameInput::bytes)
            .collect::<Vec<_>>();
        let (operation, start) = if let Some(slot) = slot {
            let (operation, selected) = slot.split_original_call(&bytes, self.policy.recipe())?;
            (operation, bytes.len().checked_sub(selected.len())?)
        } else {
            (SlotOp::Append, 0)
        };
        let operation = match operation {
            SlotOp::Set => NativeOoVariableSlotOperation::Set,
            SlotOp::Append => NativeOoVariableSlotOperation::Append,
            SlotOp::AppendIfNew => NativeOoVariableSlotOperation::AppendIfNew,
            SlotOp::Prepend => NativeOoVariableSlotOperation::Prepend,
            SlotOp::Remove => NativeOoVariableSlotOperation::Remove,
            SlotOp::Clear => NativeOoVariableSlotOperation::Clear,
        };
        let incoming = arguments.into_iter().skip(start).collect();
        let selected = tcl_syntax::naming::apply_native_oo_variable_slot(
            self.declarations.clone(),
            incoming,
            operation,
            SignatureSourceNameInput::bytes,
        );
        for input in &selected {
            if tcl_syntax::naming::validate_native_oo_variable(self.policy.recipe(), input.bytes())
                .ok()?
                .is_some()
            {
                return None;
            }
        }
        self.declarations = selected;
        Some(())
    }

    pub(crate) fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }

    pub(crate) fn declarations(&self) -> &[SignatureSourceNameInput] {
        &self.declarations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signature_scan::scope::SignatureSourceNameKey;
    use crate::{
        allocated_instance::AllocatedInstanceLinkOutcome as Outcome,
        command_binding::{
            AllocationIncarnation, CommandAllocationSite, SourceObjectAllocation, SourceOriginId,
            formal_topology::OriginalFormalTopology,
        },
        var_resolve::{ResolveContext, VariableExecutionFrame},
    };
    use std::sync::Arc;

    fn inputs(version: tcl_dialect::TclVersion, source: &[u8]) -> Vec<SignatureSourceNameInput> {
        let profile = match version {
            tcl_dialect::TclVersion::V8_6 => "tcl8.6",
            tcl_dialect::TclVersion::V9_0 => "tcl9.0",
            tcl_dialect::TclVersion::V9_1 => "tcl9.1",
            _ => panic!("unsupported fixture recipe"),
        };
        let config = tcl_lexer::LexerConfig::from_grammar(
            tcl_dialect::DialectProfile::find(profile).unwrap().grammar,
        );
        let image = tcl_lexer::SourceImage::native(source);
        let plan = tcl_lexer::native_script_words_in(
            image.clone(),
            tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
            config,
        )
        .unwrap();
        plan.commands[0]
            .words
            .iter()
            .skip(1)
            .map(|word| {
                SignatureSourceNameInput::OriginalWord(
                    SignatureSourceNameKey::from_original_native_word(
                        word,
                        tcl_syntax::word_rules::WordValueRules::from_config(&config),
                        NamePolicyProtocol::authored_tcl(version),
                    )
                    .unwrap(),
                )
            })
            .collect()
    }

    #[test]
    fn original_receiver_declarations_keep_counted_keys_and_slot_selected_origins() {
        use tcl_dialect::TclVersion;
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let mut inventory = OriginalReceiverVariableInventory::empty(
                &tcl_registry::definer::TCLOO_GRAMMAR,
                NamePolicyProtocol::authored_tcl(version),
            )
            .unwrap();
            let declarations = inputs(version, b"variable k\0tail k\0other k\0tail");
            let first = declarations[0].clone();
            let slot = SlotSpec {
                default_op: SlotOp::Append,
                dedup: true,
            };
            inventory.apply(Some(slot), declarations).unwrap();
            assert_eq!(inventory.declarations().len(), 2);
            assert_eq!(inventory.declarations()[0], first);
            assert_eq!(inventory.declarations()[0].bytes(), b"k\0tail");
            assert_eq!(inventory.declarations()[1].bytes(), b"k\0other");
            if version >= TclVersion::V9_0 {
                inventory
                    .apply(Some(slot), inputs(version, b"variable -remove k\0tail"))
                    .unwrap();
                assert_eq!(inventory.declarations()[0].bytes(), b"k\0other");
            }
            assert!(
                inventory
                    .apply(Some(slot), inputs(version, b"variable -clear extra"))
                    .is_none()
            );
            assert!(
                inventory
                    .apply(Some(slot), inputs(version, b"variable ::qualified"))
                    .is_none()
            );
        }
    }

    struct ReceiverFixture {
        version: tcl_dialect::TclVersion,
        policy: NamePolicyProtocol,
        context: ResolveContext,
        allocation: SourceObjectAllocation,
        formals: OriginalFormalTopology,
    }

    fn receiver_fixture(registry: &tcl_registry::CommandRegistry) -> ReceiverFixture {
        let version = tcl_dialect::TclVersion::V8_6;
        let policy = NamePolicyProtocol::authored_tcl(version);
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let image = tcl_lexer::SourceImage::native(b"proc p {v\xed\xa0\x80} {}".as_slice());
        let plan = tcl_lexer::native_script_words_in(
            image.clone(),
            tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
            config,
        )
        .unwrap();
        let formals =
            crate::command_binding::formal_topology::OriginalFormalTopology::from_original_key(
                SignatureSourceNameKey::from_original_native_word(
                    &plan.commands[0].words[2],
                    tcl_syntax::word_rules::WordValueRules::from_config(&config),
                    policy,
                )
                .unwrap(),
                dialect,
            )
            .unwrap();
        let mut inventory =
            OriginalReceiverVariableInventory::empty(&tcl_registry::definer::TCLOO_GRAMMAR, policy)
                .unwrap();
        inventory
            .apply(
                None,
                inputs(version, b"variable v\xed\xa0\x80 v\xed\xa0\x81"),
            )
            .unwrap();
        let frame = VariableExecutionFrame::ReceiverMethod {
            identity: "entered method fixture".into(),
        };
        let mut incoming = ResolveContext::for_namespace("::");
        incoming.invocation_dialect = Some(dialect);
        incoming.execution_name_policy = Some(
            tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(policy),
        );
        let mut context = incoming.enter_called_frame(&frame);
        let allocation = SourceObjectAllocation {
            site: CommandAllocationSite {
                source: Arc::new(SourceOriginId::authored(&Arc::from("manufacturer fixture"))),
                offset: 0,
            },
            frame: VariableExecutionFrame::Global,
            incarnation: AllocationIncarnation::First,
        };
        assert_eq!(
            context.link_original_instance_declarations(
                &allocation,
                &inventory,
                Some(&formals),
                registry
            ),
            Outcome::Linked
        );
        let formal_key = crate::var_resolve::VariableCellKey::Activation {
            identity: context.activation.clone().unwrap(),
            simple: b"v\xed\xa0\x80".into(),
        };
        assert!(!context.alias_bindings.contains_key(&formal_key));
        let linked = crate::var_resolve::resolve_evaluated_variable_input(
            tcl_syntax::naming::NativeVariableInputForm::Combined(b"v\xed\xa0\x81"),
            &context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert!(
            matches!(&linked.cell.unwrap().owner, crate::place::CellOwner::AllocatedInstance(owner) if **owner == allocation)
        );
        ReceiverFixture {
            version,
            policy,
            context,
            allocation,
            formals,
        }
    }

    fn assert_zero_resolver_purposes(
        fixture: &mut ReceiverFixture,
        registry: &tcl_registry::CommandRegistry,
    ) -> OriginalReceiverVariableInventory {
        let version = fixture.version;
        let policy = fixture.policy;
        let context = &mut fixture.context;
        let allocation = &fixture.allocation;
        let formals = &fixture.formals;
        let mut zero =
            OriginalReceiverVariableInventory::empty(&tcl_registry::definer::TCLOO_GRAMMAR, policy)
                .unwrap();
        zero.apply(None, inputs(version, b"variable first k\0tail"))
            .unwrap();
        assert_eq!(
            context.link_original_instance_declarations(allocation, &zero, Some(formals), registry),
            Outcome::Linked
        );
        let candidates = context
            .original_receiver_variable_candidates
            .as_ref()
            .unwrap();
        let selected = candidates
            .select(
                b"k\0tail",
                tcl_syntax::naming::NativeOoVariableResolverPurpose::CompiledPrimary,
                context,
            )
            .unwrap();
        assert!(
            matches!(selected.cell.as_ref().map(|cell| &cell.owner), Some(crate::place::CellOwner::AllocatedInstance(owner)) if owner.as_ref() == allocation)
        );
        assert_eq!(
            candidates
                .select(
                    b"k\0tail",
                    tcl_syntax::naming::NativeOoVariableResolverPurpose::RuntimeRoot,
                    context,
                )
                .unwrap()
                .kind,
            crate::place::PlaceKind::Unknown
        );
        // A comparison receipt supplies no complete CPP primary selection.
        let compiler = tcl_syntax::naming::NativeCompiledVariableProtocol::authored_tcl(version);
        assert_eq!(
            crate::var_resolve::resolve_original_compiled_variable(
                b"k\0tail",
                None,
                tcl_syntax::naming::NativeCompiledVariableLookup::CreateLocal,
                compiler,
                context,
                registry,
                tcl_registry::TraceOperation::Read,
            )
            .kind,
            crate::place::PlaceKind::Unknown
        );
        for name in [b"first".as_slice(), b"k\0tail"] {
            assert!(!context.alias_bindings.contains_key(
                &crate::var_resolve::VariableCellKey::Activation {
                    identity: context.activation.clone().unwrap(),
                    simple: name.into()
                }
            ));
        }
        assert!(!context.contents_presence_slots.keys().any(|key| {
            matches!(key.root(), crate::var_resolve::VariableCellKey::AllocatedInstance { simple, .. } if simple.as_bytes() == b"first")
        }));
        zero
    }

    fn assert_receiver_candidate_withdrawal(
        fixture: &ReceiverFixture,
        zero: &OriginalReceiverVariableInventory,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let context = &fixture.context;
        let allocation = &fixture.allocation;
        let formals = &fixture.formals;
        let mut invalid = context.clone();
        invalid
            .unknown_bindings
            .insert(crate::var_resolve::VariableCellKey::Activation {
                identity: invalid.activation.clone().unwrap(),
                simple: b"k\0tail".into(),
            });
        let before = invalid.clone();
        assert_eq!(
            invalid.link_original_instance_declarations(allocation, zero, Some(formals), registry),
            Outcome::Unknown
        );
        assert_eq!(invalid, before);
        let mut unequal = context.clone();
        unequal.original_receiver_variable_candidates = None;
        let mut joined = context.clone();
        joined.join(&unequal);
        assert!(joined.original_receiver_variable_candidates.is_none());
        assert!(joined.dynamic_bindings);
        assert!(
            context
                .enter_called_frame(&VariableExecutionFrame::Procedure {
                    namespace: "::".into(),
                    identity: "other".into()
                })
                .original_receiver_variable_candidates
                .is_none()
        );
        let mut withdrawn = context.clone();
        withdrawn.widen();
        assert!(withdrawn.original_receiver_variable_candidates.is_none());
    }

    #[test]
    fn original_receiver_candidates_keep_byte_formals_and_separate_zero_resolver_purposes() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut fixture = receiver_fixture(registry);
        let zero = assert_zero_resolver_purposes(&mut fixture, registry);
        assert_receiver_candidate_withdrawal(&fixture, &zero, registry);
    }
}
