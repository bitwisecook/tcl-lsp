//! BPF-Tcl dialect command specifications.
//!
//! A purpose-built dialect for the BPF-Tcl framework, spanning every layer:
//! the low-level typed eBPF verbs (scalars, packet access, maps, verdicts) and
//! the F5-inspired `when <EVENT> priority N { … }` event layer. (A profile-based
//! top layer is a planned addition that will register here too.)
//!
//! Note: the `when` spec deliberately carries **no** lowering hook, so the
//! BPF-Tcl front-end re-lowers each handler body itself (a separate event space
//! from F5 iRules) rather than going through the `::when::` lowering.

// Layer 1 — typed scalars.
mod seti32;
mod setint;
mod setu32;
// Layer 1 — packet / context access.
mod load16;
mod load32;
mod load8;
mod pktlen;
mod setbuf;
// Layer 2 — maps.
mod map;
mod map_get;
mod map_set;
// Layer 2 — verdicts.
mod accept;
mod drop;
mod pass;
mod tx;
// Control flow.
mod loop_;
// Layer 3 — the event framework.
mod when;
// Layer 4 — the profile-based top layer (protocol facet).
mod field;
mod profile;

use crate::spec::CommandSpec;

/// Return all BPF-Tcl command specifications.
#[must_use]
pub fn bpf_command_specs() -> Vec<CommandSpec> {
    vec![
        setint::spec(),
        seti32::spec(),
        setu32::spec(),
        setbuf::spec(),
        load8::spec(),
        load16::spec(),
        load32::spec(),
        pktlen::spec(),
        map::spec(),
        map_get::spec(),
        map_set::spec(),
        accept::spec(),
        drop::spec(),
        pass::spec(),
        tx::spec(),
        loop_::spec(),
        when::spec(),
        profile::spec(),
        field::spec(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialects::DialectSet;

    #[test]
    fn all_bpf_commands_present_and_tagged() {
        let specs = bpf_command_specs();
        assert_eq!(specs.len(), 19);
        let names: Vec<&str> = specs.iter().map(|s| s.name).collect();
        for n in [
            "setint", "seti32", "setu32", "setbuf", "load8", "load16", "load32", "pktlen", "map",
            "map_get", "map_set", "accept", "drop", "pass", "tx", "loop", "when", "profile",
            "field",
        ] {
            assert!(names.contains(&n), "missing `{n}`");
        }
        assert!(specs.iter().all(|s| s.dialects == Some(DialectSet::BPF)));
    }

    #[test]
    fn when_has_no_lowering_hook() {
        // The BPF `when` must stay a generic call the front-end re-lowers — it
        // must NOT carry the F5 `LoweringHookId::When`.
        let specs = bpf_command_specs();
        let when = specs.iter().find(|s| s.name == "when").expect("when spec");
        assert!(when.lowering_hook.is_none());
    }

    #[test]
    fn bpf_verbs_known_in_any_dialect() {
        let reg = crate::registry::CommandRegistry::build_default();
        assert!(reg.known_in_any_dialect("setint"));
        assert!(reg.known_in_any_dialect("map"));
        assert!(reg.known_in_any_dialect("pktlen"));
    }
}
