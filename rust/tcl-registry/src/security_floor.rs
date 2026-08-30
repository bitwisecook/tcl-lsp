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
//! which nothing on the discovery path is told (ledger O9), so a tier-keyed
//! rule would protect nothing today; and a security fact that a *trusted*
//! pack may quietly drop is not much of a security fact. Every override, from
//! every tier, keeps the shipped command's floor.
//!
//! # What a pack may still do
//!
//! Everything the floor does not name: arity, options, arguments, hover,
//! completion, dialect gating, deprecation, effects that are not
//! security-bearing, and *adding* taint facts the shipped spec does not carry.
//! The floor only prevents a fact from going away.

use crate::spec::CommandSpec;
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
    /// - **Single-valued** facts (a sink name, a taint colour, a codegen hook)
    ///   take the **shipped** value whenever the shipped command has one. Not
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

/// `declared ∪ shipped`, order-preserving, allocated for the life of the
/// process.
///
/// The leak matches how the loader already publishes a pack's own static data
/// (`Box::leak` in `tcl-spectcl/src/loader.rs`) and is bounded by the number of
/// overrides in a workspace's packs, not by edits: a registry generation is
/// built per pack-set key, and the merge runs once per overriding command in
/// it. Ledger D10's generation work is what would reclaim these along with
/// everything else the loader leaks.
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
/// list against the struct itself: a new `taint_*` field, or a new codegen or
/// side-effect field, fails that test until someone decides how it merges.
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
    "taint_double_encode_colour",
    "taint_sink_safe_colour",
    "taint_sink_gate",
    "credential_options",
    "callback_taint_inputs",
    "codegen_hook",
    "inline_codegen_hook",
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
                || name == "side_switch_target";
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
