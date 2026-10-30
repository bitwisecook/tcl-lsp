// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source binding controls for TclOO definition slot dispatch.

use tcl_compiler::command_binding::{SourceAnalysisOptions, SourceCommandBindings};

#[test]
fn inherited_slot_override_refuses_original_later_command() {
    for name in ["tcl8.6", "tcl9.0"] {
        let context = tcl_registry::model::ingress::static_context_for(name);
        let registry = context.commands();
        let profile = registry.profile().unwrap();
        let analyse = |source: &str| {
            // This is a fresh source analysis with an explicit dialect and the
            // default entry; it does not grant captured native VM authority.
            let bindings = SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::for_profile(Some(profile)),
                registry,
                SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    ..Default::default()
                },
            );
            let offset = u32::try_from(source.rfind("puts MARK").unwrap()).unwrap();
            bindings.invocation_at_source("puts", offset)
        };
        let stock = analyse("oo::class create C {superclass oo::class}; puts MARK");
        assert!(
            stock
                .proved_target()
                .is_some_and(|target| target.command == "::puts"),
            "{name}: stock target"
        );
        // Tcl 9.0.4 and 8.6.16 execute the override, moving puts while
        // class creation succeeds. The inherited slot-class method matters.
        let changed = analyse(
            "oo::define ::oo::Slot method -set args {rename ::puts ::held_puts}; oo::class create C {superclass oo::class}; puts MARK",
        );
        assert!(
            changed.unknown,
            "{name}: mutated dispatch must remain uncertain"
        );
        assert!(
            changed.proved_target().is_none(),
            "{name}: no original puts proof"
        );
    }
}
