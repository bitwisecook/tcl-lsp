// SPDX-License-Identifier: MIT
//! Invariant **I6** — the monotone security merge.
//!
//! §6.4 of the redesign says untrusted data "can add sinks and restrictions,
//! never remove or weaken built-in taint, side-effect, safety, closed-world, or
//! codegen facts". Until R12 nothing implemented it, and the consequence was
//! measurable rather than theoretical: a workspace pack containing
//!
//! ```tcl
//! speclib probe 2.0 {
//!   command exec -override {
//!     arity 1..
//!   }
//! }
//! ```
//!
//! loaded without error and produced an `exec` carrying neither `TAINT_SINK`
//! nor `TAINT_SOURCE`, because an `-override` command replaces the shipped one
//! wholesale — "simply by being inserted, with no removal step"
//! (`tcl-spectcl/src/install.rs`). A repository could therefore silence taint
//! diagnostics about its own code by committing four lines to `.tcl-lsp/`.
//!
//! [`SecurityFloor`] is the fix, and it is deliberately **not** keyed on the
//! tier. §6.4 keys its untrusted class on the editor's Workspace Trust state,
//! which nothing on the discovery path is told, so a tier-keyed rule would
//! protect nothing today; and a security fact that a *trusted*
//! pack may quietly drop is not much of a security fact. Every override, from
//! every tier, keeps the shipped command's floor.
//!
//! # The codegen and dispatch axis
//!
//! The floor also keeps a shipped command's identity on the axis that decides
//! what emitted code and the analyser's dispatch do with it: the two codegen
//! hooks, the six catalogue fields `lowering_hook`, `analyser_hook`,
//! `semantic_operation`, `state_transitions`, `native_lowering` and `bpf_op`,
//! the `runtime_backing` fact that says how the command's behaviour reaches the
//! runtime, and the per-release windows beside the four stamps
//! (`codegen_hook_windows`, `inline_codegen_hook_windows`,
//! `semantic_operation_windows`, `native_lowering_windows`). An override that
//! swapped any of them would change which shipped implementation a compiled
//! site rests on without the site knowing, so each takes the shipped value; a
//! stamp the shipped command carries in any form — beside its windows or in
//! them — leaves the override no window of its own, because a window selected
//! at one release is a swap at that release. This is a contract about the
//! closed catalogues, not a trust gate on analysis facts: an override still
//! changes arity, roles and hover.
//!
//! # What a pack may still do
//!
//! Everything the floor does not name: arity, options, arguments, hover,
//! completion, dialect gating, deprecation, effects that are not
//! security-bearing, and *adding* taint facts the shipped spec does not carry.
//! The floor only prevents a fact from going away.

use crate::spec::CommandSpec;
use crate::stamp_window::StampWindow;
use crate::traits::{Trait, TraitCategory, Traits};

/// The security facts of one shipped command, as a floor an override cannot
/// sink below.
///
/// Captured by value from the shipped spec so the borrow is over before the
/// caller mutates and re-inserts.
#[derive(Debug, Clone, Copy)]
pub struct SecurityFloor {
    spec: &'static CommandSpec,
}

impl SecurityFloor {
    /// The floor a shipped command imposes on anything that overrides it.
    #[must_use]
    pub const fn of(shipped: &'static CommandSpec) -> Self {
        Self { spec: shipped }
    }

    /// Every trait in [`TraitCategory::Security`] — the union side of the
    /// merge, derived from the category each trait already declares rather
    /// than from a second hand-kept list that could drift from it.
    #[must_use]
    pub fn security_traits(traits: Traits) -> Traits {
        Trait::ALL
            .iter()
            .copied()
            .filter(|item| item.category() == TraitCategory::Security)
            .map(|item| Traits::of(&[item]))
            .filter(|flag| traits.contains(*flag))
            .fold(Traits::empty(), Traits::union)
    }

    /// Raise `spec` to this floor, in place.
    ///
    /// Three merge rules, one per shape of fact:
    ///
    /// - **Set-valued** facts (traits, side effects, the sink subcommand and
    ///   credential lists) are **unioned**: the override keeps everything it
    ///   declared and gains everything the shipped command declared.
    /// - **Single-valued** facts (a sink name, a taint colour, a codegen hook,
    ///   a lowering or analyser hook, a semantic operation, a state-transition
    ///   descriptor, a native lowering, a BPF op) take the **shipped** value
    ///   whenever the shipped command has one. Not
    ///   "keep the override's if it set one": restating a built-in taint colour
    ///   as `Clean` is exactly the weakening this exists to stop, and the
    ///   override has no standing to reclassify a command the server ships.
    /// - Everything else is left alone.
    pub fn apply(&self, spec: &mut CommandSpec) {
        let shipped = self.spec;

        spec.traits = spec.traits.union(Self::security_traits(shipped.traits));

        spec.side_effects = union_leaked(spec.side_effects, shipped.side_effects);
        spec.taint_output_sink_subcommands = union_leaked(
            spec.taint_output_sink_subcommands,
            shipped.taint_output_sink_subcommands,
        );
        spec.taint_interp_eval_subcommands = union_leaked(
            spec.taint_interp_eval_subcommands,
            shipped.taint_interp_eval_subcommands,
        );
        spec.credential_options = union_leaked(spec.credential_options, shipped.credential_options);

        take_shipped(&mut spec.taint_output_sink, shipped.taint_output_sink);
        take_shipped(&mut spec.taint_log_sink, shipped.taint_log_sink);
        take_shipped(
            &mut spec.taint_network_sink_args,
            shipped.taint_network_sink_args,
        );
        take_shipped(&mut spec.taint_code_sink_args, shipped.taint_code_sink_args);
        take_shipped(&mut spec.taint_source, shipped.taint_source);
        take_shipped(&mut spec.taint_transform, shipped.taint_transform);
        // The condition travels with the colour it qualifies, so a pack
        // cannot keep a shipped command-level `taint_transform` while dropping
        // the proof that earns it. The floor reads `CommandSpec` fields only:
        // the same pairing inside a `SubCommand` is not restored here.
        take_shipped(&mut spec.taint_transform_when, shipped.taint_transform_when);
        take_shipped(
            &mut spec.taint_double_encode_colour,
            shipped.taint_double_encode_colour,
        );
        take_shipped(
            &mut spec.taint_sink_safe_colour,
            shipped.taint_sink_safe_colour,
        );
        take_shipped(&mut spec.taint_sink_gate, shipped.taint_sink_gate);
        take_shipped(&mut spec.codegen_hook, shipped.codegen_hook);
        take_shipped(&mut spec.inline_codegen_hook, shipped.inline_codegen_hook);
        take_shipped_windows(
            &mut spec.codegen_hook_windows,
            shipped.codegen_hook.is_some(),
            shipped.codegen_hook_windows,
        );
        take_shipped_windows(
            &mut spec.inline_codegen_hook_windows,
            shipped.inline_codegen_hook.is_some(),
            shipped.inline_codegen_hook_windows,
        );
        take_shipped_windows(
            &mut spec.semantic_operation_windows,
            shipped.semantic_operation.is_some(),
            shipped.semantic_operation_windows,
        );
        take_shipped_windows(
            &mut spec.native_lowering_windows,
            shipped.native_lowering.is_some(),
            shipped.native_lowering_windows,
        );
        // The rest of the codegen and dispatch axis. Command-level values
        // only, like the two hooks above: the same fields inside a
        // `SubCommand` or a form are not restored here.
        take_shipped(&mut spec.lowering_hook, shipped.lowering_hook);
        take_shipped(&mut spec.analyser_hook, shipped.analyser_hook);
        take_shipped(&mut spec.semantic_operation, shipped.semantic_operation);
        take_shipped(&mut spec.state_transitions, shipped.state_transitions);
        take_shipped(&mut spec.native_lowering, shipped.native_lowering);
        take_shipped(&mut spec.bpf_op, shipped.bpf_op);
        // A backing is a variant rather than an `Option`, whose `None` is
        // "declares nothing" — the default a shipped command yields to.
        if !shipped.runtime_backing.is_none() {
            spec.runtime_backing = shipped.runtime_backing;
        }
        spec.callback_taint_inputs =
            union_leaked(spec.callback_taint_inputs, shipped.callback_taint_inputs);
    }
}

/// The shipped value wins wherever the shipped command has one.
fn take_shipped<T>(target: &mut Option<T>, shipped: Option<T>) {
    if shipped.is_some() {
        *target = shipped;
    }
}

/// The shipped windows win wherever the shipped command carries the stamp at
/// all — unversioned or in a window — and then an override has no window of its
/// own: its windows would select a different stamp at some release.
fn take_shipped_windows<T>(
    target: &mut &'static [StampWindow<T>],
    shipped_has_stamp: bool,
    shipped: &'static [StampWindow<T>],
) {
    if shipped_has_stamp || !shipped.is_empty() {
        *target = shipped;
    }
}

/// `declared ∪ shipped`, order-preserving, allocated for the life of the
/// process.
///
/// The leak matches how the loader already publishes a pack's own static data
/// (`Box::leak` in `tcl-spectcl/src/loader.rs`) and is bounded by the number of
/// overrides in a workspace's packs, not by edits: a registry generation is
/// built per pack-set key, and the merge runs once per overriding command in
/// it. The generation-arena work tracked at redesign §11 D10 is what would
/// reclaim these along with everything else the loader leaks.
fn union_leaked<T: Clone + PartialEq + 'static>(
    declared: &'static [T],
    shipped: &'static [T],
) -> &'static [T] {
    if shipped.is_empty() {
        return declared;
    }
    let missing: Vec<&T> = shipped
        .iter()
        .filter(|item| !declared.contains(item))
        .collect();
    if missing.is_empty() {
        return declared;
    }
    if declared.is_empty() {
        return shipped;
    }
    let mut merged: Vec<T> = declared.to_vec();
    merged.extend(missing.into_iter().cloned());
    Box::leak(merged.into_boxed_slice())
}

/// The security-bearing field names [`SecurityFloor::apply`] merges.
///
/// Held here so `every_security_bearing_field_is_in_the_floor` can hold the
/// list against the struct itself: a new `taint_*` field, or a new codegen,
/// dispatch or side-effect field, fails that test until someone decides how
/// it merges.
pub const MERGED_FIELDS: &[&str] = &[
    "traits",
    "side_effects",
    "taint_output_sink",
    "taint_output_sink_subcommands",
    "taint_log_sink",
    "taint_network_sink_args",
    "taint_code_sink_args",
    "taint_interp_eval_subcommands",
    "taint_source",
    "taint_transform",
    "taint_transform_when",
    "taint_double_encode_colour",
    "taint_sink_safe_colour",
    "taint_sink_gate",
    "credential_options",
    "callback_taint_inputs",
    "codegen_hook",
    "codegen_hook_windows",
    "inline_codegen_hook",
    "inline_codegen_hook_windows",
    "lowering_hook",
    "analyser_hook",
    "semantic_operation",
    "semantic_operation_windows",
    "state_transitions",
    "native_lowering",
    "native_lowering_windows",
    "bpf_op",
    "runtime_backing",
];

/// Security-bearing by name but deliberately not part of the floor, with the
/// reason. A field lands here only when dropping it cannot weaken a security
/// fact.
pub const NOT_MERGED: &[(&str, &str)] = &[(
    "side_switch_target",
    "names which side a command switches to, not whether it may — the \
     side-effect union already carries the permission half",
)];

#[cfg(test)]
mod tests {
    use super::*;

    /// Every field of `CommandSpec` whose name marks it security-bearing is
    /// either merged by the floor or listed as a deliberate exclusion.
    ///
    /// The gate on I6 drifting: a taint or codegen field added later cannot
    /// quietly fall outside the floor.
    #[test]
    fn every_security_bearing_field_is_in_the_floor() {
        let source = include_str!("spec.rs");
        let start = source
            .find("pub struct CommandSpec {")
            .expect("CommandSpec is declared in spec.rs");
        let body = &source[start..];
        let end = body.find("\n}\n").expect("the struct ends");
        let mut found = Vec::new();
        for line in body[..end].lines() {
            let line = line.trim();
            let Some(rest) = line.strip_prefix("pub ") else {
                continue;
            };
            let Some((name, _)) = rest.split_once(':') else {
                continue;
            };
            let security_bearing = name.contains("taint")
                || name.contains("codegen")
                || name.contains("side_effect")
                || name.contains("credential")
                || name == "traits"
                || name == "side_switch_target"
                // The rest of the codegen and dispatch axis, whose names
                // carry none of the words above.
                || matches!(
                    name,
                    "lowering_hook"
                        | "analyser_hook"
                        | "semantic_operation"
                        | "semantic_operation_windows"
                        | "state_transitions"
                        | "native_lowering"
                        | "native_lowering_windows"
                        | "bpf_op"
                        | "runtime_backing"
                );
            if security_bearing {
                found.push(name.to_owned());
            }
        }
        assert!(
            found.len() >= MERGED_FIELDS.len(),
            "scan found only {found:?}"
        );
        for name in &found {
            let merged = MERGED_FIELDS.contains(&name.as_str());
            let excluded = NOT_MERGED.iter().any(|(field, _)| field == name);
            assert!(
                merged || excluded,
                "CommandSpec::{name} looks security-bearing but the I6 floor \
                 neither merges it nor records why it does not. Add it to \
                 MERGED_FIELDS with a merge rule, or to NOT_MERGED with a \
                 reason."
            );
        }
        for name in MERGED_FIELDS {
            assert!(
                found.iter().any(|field| field == name),
                "MERGED_FIELDS names {name}, which CommandSpec no longer has"
            );
        }
    }

    static SHIPPED_OP: crate::bpf_op::BpfOpSpec = crate::bpf_op::BpfOpSpec::verdict(
        crate::bpf_op::BpfVerdictKind::Pass,
        crate::bpf_op::BpfProgTypeSet::PASS_LIKE,
    );

    /// A shipped command with all seven fields of the codegen and dispatch
    /// axis set.
    static SHIPPED_AXIS: CommandSpec = CommandSpec {
        name: "probe",
        lowering_hook: Some(crate::hooks::LoweringHookId::If),
        analyser_hook: Some(crate::hooks::AnalyserHookId::Source),
        semantic_operation: Some(crate::semantic_operation::SemanticOperationId::Intrinsic(
            crate::intrinsic::IntrinsicId::ChannelWrite,
        )),
        state_transitions: Some(crate::state_transition::StateTransitionDescriptor::EMPTY),
        native_lowering: Some(crate::native_lowering::NativeLowering::Completion(
            crate::completion::CompletionCode::Break,
        )),
        bpf_op: Some(&SHIPPED_OP),
        runtime_backing: crate::runtime_backing::RuntimeBacking::shipped("probe"),
        ..CommandSpec::DEFAULT
    };

    #[test]
    fn the_floor_takes_the_shipped_codegen_and_dispatch_axis() {
        use crate::hooks::{AnalyserHookId, LoweringHookId};
        use crate::native_lowering::NativeLowering;
        use crate::semantic_operation::SemanticOperationId;

        // An override that swaps every field it can, and drops the ones it
        // cannot name.
        let mut swapped = CommandSpec {
            name: "probe",
            lowering_hook: Some(LoweringHookId::While),
            analyser_hook: Some(AnalyserHookId::Rename),
            semantic_operation: Some(SemanticOperationId::Invoke),
            native_lowering: Some(NativeLowering::Completion(
                crate::completion::CompletionCode::Continue,
            )),
            ..CommandSpec::DEFAULT
        };
        SecurityFloor::of(&SHIPPED_AXIS).apply(&mut swapped);

        assert_eq!(swapped.lowering_hook, SHIPPED_AXIS.lowering_hook);
        assert_eq!(swapped.analyser_hook, SHIPPED_AXIS.analyser_hook);
        assert_eq!(swapped.semantic_operation, SHIPPED_AXIS.semantic_operation);
        assert_eq!(swapped.native_lowering, SHIPPED_AXIS.native_lowering);
        assert_eq!(swapped.runtime_backing, SHIPPED_AXIS.runtime_backing);
        assert_eq!(
            format!("{:?}", swapped.state_transitions),
            format!("{:?}", SHIPPED_AXIS.state_transitions),
            "a dropped descriptor comes back"
        );
        assert!(
            std::ptr::eq(
                swapped.bpf_op.expect("a dropped BPF op comes back"),
                SHIPPED_AXIS.bpf_op.expect("the shipped op"),
            ),
            "the shipped op, not a copy of another"
        );
    }

    /// An override gets no window of its own where the shipped command carries
    /// the stamp at all: a window is a swap at the releases it covers.
    #[test]
    fn the_floor_takes_the_shipped_stamp_windows_and_leaves_an_override_none_of_its_own() {
        use crate::hooks::CodegenHookId;
        use crate::lifecycle::Lifecycle;

        const FROM_9: &[StampWindow<CodegenHookId>] = &[StampWindow {
            lifecycle: Lifecycle::introduced_in("9.0"),
            value: CodegenHookId::Lassign,
        }];
        const FROM_8_4: &[StampWindow<CodegenHookId>] = &[StampWindow {
            lifecycle: Lifecycle::introduced_in("8.4"),
            value: CodegenHookId::Llength,
        }];

        // The shipped command carries an unversioned hook and no window.
        let unversioned = CommandSpec {
            name: "probe",
            codegen_hook: Some(CodegenHookId::Lassign),
            ..CommandSpec::DEFAULT
        };
        let mut swapped = CommandSpec {
            name: "probe",
            codegen_hook_windows: FROM_8_4,
            ..CommandSpec::DEFAULT
        };
        SecurityFloor::of(Box::leak(Box::new(unversioned))).apply(&mut swapped);
        assert!(swapped.codegen_hook_windows.is_empty());
        assert_eq!(swapped.codegen_hook, Some(CodegenHookId::Lassign));

        // The shipped command carries the hook in a window only.
        let windowed = CommandSpec {
            name: "probe",
            codegen_hook_windows: FROM_9,
            ..CommandSpec::DEFAULT
        };
        let mut swapped = CommandSpec {
            name: "probe",
            codegen_hook_windows: FROM_8_4,
            ..CommandSpec::DEFAULT
        };
        SecurityFloor::of(Box::leak(Box::new(windowed))).apply(&mut swapped);
        assert_eq!(swapped.codegen_hook_windows, FROM_9);

        // Nothing shipped, nothing to keep: the declared windows stand.
        let mut declared = CommandSpec {
            name: "probe",
            codegen_hook_windows: FROM_8_4,
            ..CommandSpec::DEFAULT
        };
        SecurityFloor::of(&BARE).apply(&mut declared);
        assert_eq!(declared.codegen_hook_windows, FROM_8_4);
    }

    #[test]
    fn the_floor_adds_nothing_the_shipped_command_lacks() {
        use crate::hooks::LoweringHookId;

        // The floor only stops a fact going away: an override of a command
        // that ships no lowering hook keeps the one it declared, and the
        // fields it never set stay unset.
        let mut declared = CommandSpec {
            name: "probe",
            lowering_hook: Some(LoweringHookId::While),
            ..CommandSpec::DEFAULT
        };
        SecurityFloor::of(&BARE).apply(&mut declared);
        assert_eq!(declared.lowering_hook, Some(LoweringHookId::While));
        assert!(declared.analyser_hook.is_none());
        assert!(declared.semantic_operation.is_none());
        assert!(declared.state_transitions.is_none());
        assert!(declared.native_lowering.is_none());
        assert!(declared.bpf_op.is_none());
        assert!(declared.runtime_backing.is_none());
    }

    #[test]
    fn a_declared_backing_stands_where_the_shipped_command_declares_none() {
        use crate::runtime_backing::RuntimeBacking;

        let mut declared = CommandSpec {
            name: "probe",
            runtime_backing: RuntimeBacking::HostNative,
            ..CommandSpec::DEFAULT
        };
        SecurityFloor::of(&BARE).apply(&mut declared);
        assert_eq!(declared.runtime_backing, RuntimeBacking::HostNative);
    }

    static BARE: CommandSpec = CommandSpec {
        name: "probe",
        ..CommandSpec::DEFAULT
    };

    #[test]
    fn security_traits_keeps_only_the_security_category() {
        let mixed = Traits::TAINT_SINK
            .union(Traits::TAINT_SOURCE)
            .union(Traits::PURE);
        let kept = SecurityFloor::security_traits(mixed);
        assert!(kept.contains(Traits::TAINT_SINK));
        assert!(kept.contains(Traits::TAINT_SOURCE));
        assert!(!kept.contains(Traits::PURE));
    }

    #[test]
    fn union_leaked_is_order_preserving_and_deduplicating() {
        static DECLARED: &[&str] = &["a", "b"];
        static SHIPPED: &[&str] = &["b", "c"];
        assert_eq!(union_leaked(DECLARED, SHIPPED), &["a", "b", "c"]);
        assert_eq!(union_leaked(DECLARED, &[]), &["a", "b"]);
        assert_eq!(union_leaked(&[], SHIPPED), &["b", "c"]);
    }
}
