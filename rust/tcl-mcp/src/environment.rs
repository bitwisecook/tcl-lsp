// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The MCP server's dialect ingress — its face of the one shared seam,
//! [`tcl_registry::model::ingress`] (centralisation contract R-a; P1-F
//! wave 4, alongside the CLIs and the spec studio).
//!
//! Every dialect **name** an MCP tool accepts — a tool call's `dialect`
//! argument, the process-wide session dialect `set_dialect` holds, a
//! detector verdict, the fixed `f5-irules` the iRule tools simulate —
//! resolves here, once, and the availability questions a tool then asks
//! are answered by the resolved environment's [`ResolvedContext`] rather
//! than by `ProfileQueries` over a threaded profile.
//!
//! Nothing here changes a tool's answer. The catalogue names the tools
//! resolve map to their same-named environments, whose `unit_profile` is
//! the profile the retired validators returned, and whose document context
//! answers under the **document authoring mask** — test-pinned equal to
//! the threaded profile's `availability_mask` for every profile an ingress
//! can produce.

use tcl_dialect::DialectProfile;
use tcl_registry::model::ResolvedContext;

/// Resolve a dialect **name** to the profile a tool threads — the
/// environment-model form of `DialectProfile::by_name` and of the named
/// constructors (`plain_tcl`, `irules`, `tk`).
///
/// P1-G: the profile retires and the tools read their labels off the
/// environment instead.
pub fn profile_for_dialect(name: &str) -> &'static DialectProfile {
    tcl_registry::model::resolve_environment(name).unit_profile()
}

/// Resolve a dialect **name** only when it names a real environment — the
/// validator form, replacing `DialectProfile::find` / `resolve_known` at
/// the two MCP ingresses that must reject an unknown spelling rather than
/// serve the lenient fallback: `set_dialect` and `tk_layout` (ledger rows
/// F9/T6).
pub fn known_profile_for_dialect(name: &str) -> Option<&'static DialectProfile> {
    tcl_registry::model::resolve_known_environment(name)
        .map(|environment| environment.unit_profile())
}

/// The canonical environment id `name` resolves to, as a `&'static str`,
/// or `None` when it names no environment.
///
/// The `&'static` comes from the promoted document context, which the
/// generation cache retains for the process by design — the same promotion
/// [`context_for_dialect`] relies on. It replaces the manual
/// `KNOWN_DIALECTS` membership scan the session-dialect plumbing used to
/// recover a `&'static` spelling with, and folds aliases to the canonical
/// id on the way (the session already holds a canonical id, so no shipped
/// path changes answer).
pub fn canonical_id_for_dialect(name: &str) -> Option<&'static str> {
    tcl_registry::model::is_known_environment_name(name)
        .then(|| context_for_dialect(name).environment.id.as_str())
}

/// The **document context** a dialect name's answers are given under — the
/// assistance view that replaces the whole `ProfileQueries` surface
/// (ledger row F1's assistance half): command resolution, availability,
/// options, keyed version ranges.
pub fn context_for_dialect(name: &str) -> &'static ResolvedContext {
    tcl_registry::model::static_document_context_for(name)
}

/// The availability mask the **analyser** ingress gives a dialect name —
/// the exact `DialectProfile::by_name(name).availability_mask` twin,
/// resolved through the seam rather than read back off a registry
/// generation's profile *stamp*.
///
/// Deliberately not the document authoring mask: it sinks `tk` to the
/// permissive fallback's mask instead of adding the additive `TK` bit,
/// which is what the collision check in [`crate::spectcl`] has always
/// compared under. P1-G/C1: the mask itself is `DialectSet` plumbing and
/// goes when the profile does.
pub fn analyser_mask_for_dialect(name: &str) -> tcl_dialect::DialectSet {
    tcl_registry::model::resolve_environment(name)
        .analyser_profile()
        .availability_mask
}

/// The command **store** for a dialect name — the resolved environment's
/// registry generation, replacing `tcl_registry::registry_for_dialect` and
/// `registry_for_profile` at the sites that want the plain catalogue store
/// rather than the pack-layered one.
pub fn store_for_dialect(name: &str) -> &'static tcl_registry::CommandRegistry {
    tcl_registry::model::static_context_for(name).commands()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_ids_fold_aliases_and_reject_unknown_names() {
        assert_eq!(canonical_id_for_dialect("irules"), Some("f5-irules"));
        assert_eq!(canonical_id_for_dialect("tcl9.0"), Some("tcl9.0"));
        assert_eq!(canonical_id_for_dialect("klingon"), None);
        assert_eq!(canonical_id_for_dialect(""), None);
        // Every canonical catalogue name is its own id.
        for profile in DialectProfile::all() {
            assert_eq!(canonical_id_for_dialect(profile.name), Some(profile.name));
        }
    }

    #[test]
    fn the_document_context_carries_the_threaded_masks() {
        let names: Vec<&str> = DialectProfile::all()
            .iter()
            .map(|profile| profile.name)
            .chain(["tk", "tcl", "", "irules", "klingon"])
            .collect();
        for name in names {
            assert_eq!(
                context_for_dialect(name).authoring_mask(),
                profile_for_dialect(name).availability_mask,
                "{name}"
            );
        }
    }
}
