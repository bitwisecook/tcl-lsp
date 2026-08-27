// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! **Live environment registration, end to end** (P2-H deliverable E):
//! a pack-declared `environment` block becomes resolvable through the one
//! ingress seam — `tcl_registry::model::ingress::resolve_environment` —
//! with its declared detection facts and ambient placements; a
//! reserved-name claim fails with the provenance error; and the
//! registration bumps the registry generation the per-context caches key
//! on.

use tcl_dialect::model::Placement;
use tcl_registry::model::{EnvironmentRegistrationError, resolve_environment};
use tcl_spectcl::registration::register_pack_environments;
use tcl_spectcl::{Tier, load_pack};

/// The full path: load a pack with an `environment` block, register it,
/// resolve it through the ingress, and read its facts back.
#[test]
fn a_pack_declared_environment_becomes_resolvable_with_its_facts() {
    let pack = load_pack(
        "speclib probe 2.0 {\n\
         environment vivaldi-shell-tcl {\n\
         \x20   display_name {Vivaldi Shell}\n\
         \x20   core tcl 8.6\n\
         \x20   ambient VivaldiCmds 3.1\n\
         \x20   hosted Tk 8.5-\n\
         \x20   alias vivaldi\n\
         \x20   file_extension vsh -name {Vivaldi Shell Script}\n\
         \x20   filename vivaldi.rc\n\
         \x20   signature {vivaldi_open_project}\n\
         \x20   policy ambient-plus-require\n\
         }\n\
         command vivaldi_open_project { arity 1 }\n\
         }\n",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let before = resolve_environment("tcl9.0").identity.generation;

    let outcome = register_pack_environments(&pack, Tier::User).expect("registration succeeds");
    assert_eq!(outcome.declared, 1);
    assert_eq!(outcome.extended, 0);
    let generation = outcome.generation.expect("something registered");
    assert!(generation > before);

    // Resolvable through the one ingress — canonical id and alias alike.
    for name in ["vivaldi-shell-tcl", "vivaldi"] {
        let resolved = resolve_environment(name);
        assert_eq!(resolved.id(), "vivaldi-shell-tcl", "{name}");
        assert!(resolved.identity.generation >= generation, "{name}");
        assert_eq!(resolved.definition.display_name.as_ref(), "Vivaldi Shell");

        // The declared detection facts.
        let detection = &resolved.definition.server_detection;
        assert!(
            detection
                .file_extensions
                .iter()
                .any(|claim| claim.extension.as_ref() == "vsh"
                    && claim.display_name.as_ref() == "Vivaldi Shell Script"),
            "{detection:?}"
        );
        assert!(
            detection
                .filenames
                .iter()
                .any(|filename| filename.as_ref() == "vivaldi.rc")
        );
        assert!(
            detection
                .content_signatures
                .iter()
                .any(|signature| signature.as_ref() == "vivaldi_open_project")
        );

        // The declared placements: one pinned ambient, one hosted
        // requirement on Tk's own axis.
        let ambient = resolved
            .definition
            .expected_packages
            .iter()
            .find(|placement| placement.package.as_ref() == "VivaldiCmds")
            .expect("the ambient placement registers");
        assert!(ambient.ambient);
        assert!(matches!(&ambient.version, Placement::Pinned(v) if v.to_string() == "3.1"));
        let hosted = resolved
            .definition
            .expected_packages
            .iter()
            .find(|placement| placement.package.as_ref() == "Tk")
            .expect("the hosted placement registers");
        assert!(!hosted.ambient);
        assert!(matches!(&hosted.version, Placement::Requirement(_)));
    }

    // The known-name validator accepts the registered spellings too.
    assert!(tcl_registry::model::is_known_environment_name(
        "vivaldi-shell-tcl"
    ));
    assert!(tcl_registry::model::is_known_environment_name("vivaldi"));

    // Idempotent re-registration: a pack reload replaces, never stacks.
    register_pack_environments(&pack, Tier::User).expect("re-registration succeeds");
    let resolved = resolve_environment("vivaldi-shell-tcl");
    assert_eq!(
        resolved
            .definition
            .expected_packages
            .iter()
            .filter(|placement| placement.package.as_ref() == "VivaldiCmds")
            .count(),
        1
    );

    // And the environment answers a per-context registry generation.
    let registry = resolved.default_context_registry();
    assert_eq!(
        registry.context().environment.id.as_str(),
        "vivaldi-shell-tcl"
    );
}

/// The trust lattice: an untrusted (workspace) tier cannot extend a
/// compiled environment, and a reserved-name claim fails with the
/// provenance error whichever seam it reaches.
#[test]
fn reserved_and_untrusted_claims_fail_with_the_provenance_error() {
    // The loader itself rejects a *declaration* claiming a compiled name,
    // so the workspace pack carries only an extend block…
    let pack = load_pack(
        "speclib probe 2.0 {\n\
         environment expect -extend {\n\
         \x20   file_extension expx -name {Probe Expect}\n\
         }\n\
         }\n",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let error = register_pack_environments(&pack, Tier::Workspace)
        .expect_err("a workspace tier may not extend a compiled environment");
    assert!(
        matches!(
            &error,
            EnvironmentRegistrationError::UntrustedExtension { base, .. } if base == "expect"
        ),
        "{error:?}"
    );
    // …and the compiled environment is untouched.
    assert!(
        !resolve_environment("expect")
            .definition
            .server_detection
            .file_extensions
            .iter()
            .any(|claim| claim.extension.as_ref() == "expx")
    );

    // A definition claiming a reserved compiled name, constructed without
    // the loader, is refused by the registry-side seam with the claiming
    // provenance in the error.
    let mut claim = tcl_dialect::model::compiled_definitions()
        .into_iter()
        .find(|definition| definition.id.as_str() == "expect")
        .expect("the compiled seed set has `expect`");
    claim.provenance = tcl_dialect::model::Provenance::WorkspaceTrusted;
    let error = tcl_registry::model::register_environments(vec![claim], Vec::new())
        .expect_err("a compiled id is reserved");
    match &error {
        EnvironmentRegistrationError::Reserved {
            name, provenance, ..
        } => {
            assert_eq!(name, "expect");
            assert_eq!(
                *provenance,
                tcl_dialect::model::Provenance::WorkspaceTrusted
            );
        }
        other => panic!("expected Reserved, got {other:?}"),
    }
    assert!(error.to_string().contains("trusted workspace"));
}

/// A trusted tier's `-extend` of a compiled environment lands additively:
/// the compiled facts stay, the contributed detection row joins them, and
/// the generation moves so downstream caches re-key.
#[test]
fn a_trusted_extension_of_a_compiled_environment_is_additive() {
    let pack = load_pack(
        "speclib probe 2.0 {\n\
         environment synopsys-eda-tcl -extend {\n\
         \x20   file_extension upfx -name {Probe UPF Extension}\n\
         }\n\
         }\n",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let outcome =
        register_pack_environments(&pack, Tier::Bundled).expect("a bundled extension lands");
    assert_eq!(outcome.extended, 1);

    let resolved = resolve_environment("synopsys-eda-tcl");
    let detection = &resolved.definition.server_detection;
    assert!(
        detection
            .file_extensions
            .iter()
            .any(|claim| claim.extension.as_ref() == "upfx"),
        "the contributed row joins"
    );
    assert!(
        detection
            .file_extensions
            .iter()
            .any(|claim| claim.extension.as_ref() == "sdc"),
        "the compiled rows stay"
    );
}

/// **The production wiring**, at the seam every consumer publishes
/// through: a whole loaded pack set registers its environments *and* its
/// pack-declared dialects, and a set that no longer carries a pack retires
/// what that pack declared.
///
/// One test rather than three, because the source channel is a global
/// replace: two tests syncing concurrently would retire each other's
/// sources, so the whole life cycle is asserted in one place.
#[test]
fn a_published_pack_set_registers_its_environments_and_dialects_and_retires_them() {
    use std::path::PathBuf;
    use tcl_spectcl::{PackFile, discovery::Origin, pack::load_in_memory, publish_pack_set};

    const SOURCE: &str = r"
speclib picolpack 2.0 {
    dialect picol2 {
        release 2.0
        axis expand_syntax off
        axis braced_var first-close
        axis numbers tcl84
    }
    environment picol-shell {
        display_name {Picol Shell}
        core picol2 2.0
        file_extension pcl -name {Picol Script}
    }
    command picol_eval { arity 1 }
}
";
    let set = load_in_memory(vec![(
        PackFile {
            tier: tcl_spectcl::Tier::User,
            path: PathBuf::from("/probe/picolpack.tclspec"),
            origin: Origin::UserDir,
        },
        SOURCE.to_owned(),
    )]);
    assert!(set.notices.is_empty(), "{:?}", set.notices);

    let registration = publish_pack_set(&set);
    assert_eq!(registration.declared, 1);
    assert_eq!(registration.dialects, 1);
    assert_eq!(registration.dynamic_cores, 1);
    assert!(
        registration.dialects_refused.is_empty(),
        "{:?}",
        registration.dialects_refused
    );
    assert!(
        registration.rejected.is_empty(),
        "{:?}",
        registration.rejected
    );

    // The environment resolves through the one ingress…
    let resolved = resolve_environment("picol-shell");
    assert_eq!(resolved.id(), "picol-shell");
    assert_eq!(resolved.definition.display_name.as_ref(), "Picol Shell");
    // …its extension routes to it through the detection seam…
    assert_eq!(
        tcl_registry::dialect_from_extension("/tmp/thing.pcl"),
        Some("picol-shell")
    );
    // …and the pack-declared core is a real, resolvable grammar: the
    // conversion produced lexer data, not a description of it.
    let grammar =
        tcl_dialect::model::dynamic_core_grammar("picol-shell").expect("the bound grammar");
    assert!(!grammar.expand_syntax);
    assert_eq!(grammar.braced_var, tcl_dialect::BracedVarStyle::FirstClose);
    let family =
        tcl_dialect::model::resolve_dynamic_family("picolpack/picol2").expect("the family");
    assert_eq!(family.provenance, tcl_dialect::model::Provenance::User);
    // Namespaced per the trust lattice; the bare name resolves too while
    // it is unambiguous.
    assert!(tcl_dialect::model::resolve_dynamic_family("picol2").is_some());

    // Publish an empty set — the pack has left the workspace.
    let retired = publish_pack_set(&tcl_spectcl::PackSet::default());
    assert_eq!(retired.retired, 1);
    assert!(!tcl_registry::model::is_known_environment_name(
        "picol-shell"
    ));
    assert!(tcl_dialect::model::dynamic_core_grammar("picol-shell").is_none());
    assert!(tcl_dialect::model::resolve_dynamic_family("picolpack/picol2").is_none());
}
