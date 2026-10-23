// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Document-declared frame metadata, separate from original source or runtime lookup.

use crate::ir::DeclaredFrameEffects;
use tcl_registry::model::{DeclaredSurface, DocumentCommandSurface};
use tcl_registry::{CommandRegistry, FrameEffectSpec};

/// Retain stated effects from unambiguous selected declaration descriptor keys.
/// The registry's selected command takes precedence; conflicting descriptor
/// coordinates withdraw metadata instead of choosing a binding by display text.
#[must_use]
pub fn collect_declared_frame_effects(
    registry: &CommandRegistry,
    declared: Option<&DeclaredSurface>,
) -> DeclaredFrameEffects {
    let surface = DocumentCommandSurface::new(registry, declared);
    let effects: std::collections::BTreeMap<_, _> = surface
        .plain_call_frame_effect_descriptor_keys()
        .map(|(key, effect)| (key.label(), effect))
        .collect();
    tcl_registry::unique_command_descriptor_coordinates(surface.declared_descriptor_keys())
        .filter(|(_, key)| registry.get(key.label()).is_none())
        .filter_map(|(coordinate, key)| Some((coordinate, *effects.get(key.label())?)))
        .collect()
}

/// Read catalogue-only frame metadata with the registry's descriptor coordinate.
/// The caller must independently select a standalone metadata path; this cannot
/// replace original source-schema selection or establish an entered Native frame.
#[must_use]
pub fn declared_frame_effect_for_metadata(
    effects: &DeclaredFrameEffects,
    label: &str,
) -> Option<FrameEffectSpec> {
    effects
        .get(&tcl_registry::rooted_command_descriptor_label(label))
        .copied()
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::Provenance;
    use tcl_registry::model::{DeclaredCommand, DeclaredFrameEffect};

    fn stated(name: &str) -> DeclaredCommand {
        DeclaredCommand::new(name.to_owned(), Vec::new(), Provenance::Document).with_frame_effect(
            DeclaredFrameEffect::Stated(Some(DeclaredFrameEffect::CALLER)),
        )
    }

    #[test]
    fn selected_declaration_coordinates_preserve_literal_geometry_and_withdraw_collisions() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut declared = DeclaredSurface::new();
        for label in [
            "",
            "NAME::",
            "NAME:::member",
            "::::NAME",
            ":NAME",
            "collision",
            "::collision",
        ] {
            declared.declare(stated(label));
        }
        let effects = collect_declared_frame_effects(registry, Some(&declared));
        for label in ["", "NAME::", "NAME:::member", "::::NAME", ":NAME"] {
            assert_eq!(
                declared_frame_effect_for_metadata(&effects, label),
                Some(DeclaredFrameEffect::CALLER)
            );
        }
        for label in [
            "NAME",
            "NAME::member",
            "::NAME",
            "collision",
            "::collision",
            "missing",
        ] {
            assert_eq!(declared_frame_effect_for_metadata(&effects, label), None);
        }
        assert_eq!(effects.len(), 5);
    }

    #[test]
    fn unstated_colliding_declaration_and_selected_registry_command_withdraw_assistance() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut declared = DeclaredSurface::new();
        declared.declare(stated("expr"));
        declared.declare(stated("::expr"));
        declared.declare(stated("candidate"));
        declared.declare(DeclaredCommand::new(
            "::candidate".to_owned(),
            Vec::new(),
            Provenance::Document,
        ));
        declared.declare(DeclaredCommand::new(
            "unstated".to_owned(),
            Vec::new(),
            Provenance::Document,
        ));
        declared.declare(
            DeclaredCommand::new("own".to_owned(), Vec::new(), Provenance::Document)
                .with_frame_effect(DeclaredFrameEffect::Stated(None)),
        );
        let effects = collect_declared_frame_effects(registry, Some(&declared));
        assert_eq!(effects.len(), 1);
        assert_eq!(effects.get("::own"), Some(&None));
        assert_eq!(declared_frame_effect_for_metadata(&effects, "expr"), None);
        assert_eq!(
            declared_frame_effect_for_metadata(&effects, "candidate"),
            None
        );
        assert!(collect_declared_frame_effects(registry, None).is_empty());
    }
}
