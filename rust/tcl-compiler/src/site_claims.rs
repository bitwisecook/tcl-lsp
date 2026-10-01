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

//! The spec-pack facts a specialised site rests on, as the claims codegen
//! records beside its bindings (`tcl_runtime_api::SiteClaim`) —
//! `docs/design/compiler/registry-consumer-contracts.md` § *What the
//! artefact records per rung*.
//!
//! A spec an installed pack supplied carries a [`PackOrigin`] in the
//! registry it was installed into; any site whose emitted code rests on
//! such a spec records the origin's [`PackFactStamp`], stamped with the
//! registry's overlay generation and the building thread's evaluator
//! revision. [`pack_fact_stamp`] is the one construction: an embedder's held
//! facts for a pack set (`tcl_spectcl::PackSet::fact_stamps`) are built by
//! it too, so a site and the VM that admits it agree by construction.

use tcl_registry::CommandRegistry;
use tcl_registry::pack_origin::PackOrigin;
use tcl_registry::registry::ResolvedCall;
use tcl_runtime_api::{CommandBindingIdentity, PackFactStamp, SiteClaim};

/// The stamp of `origin`'s facts under an overlay generation and an
/// evaluator revision.
#[must_use]
pub fn pack_fact_stamp(
    origin: &PackOrigin,
    overlay_generation: u64,
    evaluator_revision: u64,
) -> PackFactStamp {
    PackFactStamp {
        pack: origin.pack.clone(),
        content_hash: origin.content_hash,
        vocabulary_version: origin.vocabulary_version.clone(),
        overlay_generation,
        evaluator_revision,
    }
}

/// The building thread's evaluator revision: the generation of the host
/// that serves declared implementations
/// ([`tcl_registry::pack_hooks::evaluator_generation`]).
#[must_use]
pub fn evaluator_revision() -> u64 {
    u64::from(tcl_registry::pack_hooks::evaluator_generation().0)
}

/// The stamp a site compiled against `registry` records for a spec from
/// `origin`.
fn site_stamp(registry: &CommandRegistry, origin: &PackOrigin) -> PackFactStamp {
    pack_fact_stamp(
        origin,
        registry.overlay_generation().unwrap_or(0),
        evaluator_revision(),
    )
}

/// Rung 1: the claim a site makes when a pack-supplied `spec` answered it at
/// compile time — a constant its `const_fold` computed. `None` for a spec no
/// pack supplied.
#[must_use]
pub fn pack_facts_claim(
    registry: &CommandRegistry,
    spec: &tcl_registry::CommandSpec,
) -> Option<SiteClaim> {
    registry
        .pack_origin(spec)
        .map(|origin| SiteClaim::PackFacts(site_stamp(registry, origin)))
}

/// Rung 3: the pack facts behind a pack command's declared backing, for a site
/// that inlines the definition the backing names. `None` for a spec no pack
/// supplied.
#[must_use]
pub fn reference_body_facts(
    registry: &CommandRegistry,
    spec: &tcl_registry::CommandSpec,
) -> Option<PackFactStamp> {
    registry
        .pack_origin(spec)
        .map(|origin| site_stamp(registry, origin))
}

/// Rung 2: the claim a site makes when its `binding` reached a builtin
/// through the resolved pack command's `alias_of` — its identity is not the
/// command's own name. `None` for every other binding.
#[must_use]
pub fn builtin_alias_claim(
    registry: &CommandRegistry,
    resolved: &ResolvedCall<'_>,
    binding: &CommandBindingIdentity,
) -> Option<SiteClaim> {
    if binding.identity == resolved.spec.name {
        return None;
    }
    let origin = registry.pack_origin(resolved.spec)?;
    Some(SiteClaim::BuiltinAlias {
        binding: binding.clone(),
        facts: site_stamp(registry, origin),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::CommandSpec;
    use tcl_registry::hooks::InlineCodegenHookId;

    fn origin() -> PackOrigin {
        PackOrigin {
            pack: "vendor".to_owned(),
            content_hash: 7,
            vocabulary_version: "2".to_owned(),
        }
    }

    /// Insert `spec` as a pack's installer does: indexed, with its origin.
    fn install(registry: &mut CommandRegistry, spec: CommandSpec) {
        let spec: &'static CommandSpec = Box::leak(Box::new(spec));
        registry.insert_static(spec);
        registry.insert_pack_origin(spec, origin());
    }

    fn compile(source: &str, registry: &CommandRegistry) -> tcl_bytecode::ModuleAsm {
        let config = tcl_lexer::LexerConfig::default();
        let ir = crate::lowering::lower_script_module_for_bytecode(
            source, "", registry, config, None, false,
        );
        let cfg = crate::cfg_builder::build_cfg_codegen_with_registry_and_config(
            &ir, false, registry, config,
        );
        crate::codegen::codegen_module(&cfg, &ir, registry)
    }

    fn stamp(registry: &CommandRegistry) -> PackFactStamp {
        pack_fact_stamp(
            &origin(),
            registry.overlay_generation().unwrap_or(0),
            evaluator_revision(),
        )
    }

    fn double(args: &[&str]) -> Option<String> {
        let n: i64 = args.first()?.parse().ok()?;
        Some((n * 2).to_string())
    }

    /// Rung 1: a pack spec's `const_fold` answering a call at compile time
    /// claims the pack's facts; the same fold from a spec no pack supplied
    /// claims nothing.
    #[test]
    fn a_pack_fold_claims_the_pack_s_facts() {
        let spec = CommandSpec {
            name: "vendor::double",
            const_fold: Some(double),
            ..CommandSpec::DEFAULT
        };
        let mut registry = CommandRegistry::build_default();
        install(&mut registry, spec.clone());
        let module = compile("set x [vendor::double 21]\nset x", &registry);
        assert_eq!(
            module.top_level.site_claims,
            vec![SiteClaim::PackFacts(stamp(&registry))],
            "{:#?}",
            module.top_level
        );

        let mut embedder = CommandRegistry::build_default();
        embedder.insert(spec);
        let module = compile("set x [vendor::double 21]\nset x", &embedder);
        assert!(module.top_level.site_claims.is_empty());
    }

    /// The manifest's packs are the claims' stamps, sorted, each once,
    /// however many sites and functions claim it.
    #[test]
    fn a_manifests_packs_are_the_claims_stamps_each_once() {
        fn other_origin() -> PackOrigin {
            PackOrigin {
                pack: "another".to_owned(),
                content_hash: 9,
                vocabulary_version: "2".to_owned(),
            }
        }
        let fold = |name: &'static str| CommandSpec {
            name,
            const_fold: Some(double),
            ..CommandSpec::DEFAULT
        };
        let mut registry = CommandRegistry::build_default();
        install(&mut registry, fold("vendor::double"));
        let spec: &'static CommandSpec = Box::leak(Box::new(fold("another::double")));
        registry.insert_static(spec);
        registry.insert_pack_origin(spec, other_origin());

        let module = compile(
            "set a [vendor::double 1]\nset b [vendor::double 2]\nset c [another::double 3]\n\
             proc p {} {set d [vendor::double 4]; set e [another::double 5]}",
            &registry,
        );
        let vendor = stamp(&registry);
        let another = pack_fact_stamp(
            &other_origin(),
            registry.overlay_generation().unwrap_or(0),
            evaluator_revision(),
        );
        let mut expected = vec![vendor, another];
        expected.sort();
        let manifest = module.manifest.as_ref().expect("the compiler fills it");
        assert_eq!(manifest.packs, expected);
        assert_eq!(manifest.packs, module.claimed_packs());
        assert!(
            module
                .procedures
                .values()
                .any(|p| !p.site_claims.is_empty()),
            "the procedure's claims are in the union too"
        );

        let bare = compile(
            "set x [vendor::double 21]",
            &CommandRegistry::build_default(),
        );
        assert!(
            bare.manifest
                .expect("the compiler fills it")
                .packs
                .is_empty()
        );
    }

    /// A pack command backed by a Tcl body: `vdouble` is its definition text.
    fn body_backed(name: &'static str, text: &'static str) -> CommandSpec {
        CommandSpec {
            name,
            runtime_backing: tcl_registry::RuntimeBacking::TclBody {
                source: tcl_registry::BodySource::PackText { text },
            },
            ..CommandSpec::DEFAULT
        }
    }

    const DOUBLE: &str = "proc vdouble {x} {expr {$x * 2}}";

    fn binding() -> tcl_runtime_api::ProcedureBindingIdentity {
        tcl_runtime_api::ProcedureBindingIdentity::new("vdouble", "::vdouble", "x", "expr {$x * 2}")
    }

    fn service_over(registry: CommandRegistry) -> crate::compile_service::BytecodeCompileService {
        crate::compile_service::BytecodeCompileService::new(registry)
    }

    fn calls_a_command(function: &tcl_bytecode::FunctionAsm) -> bool {
        function.instructions.iter().any(|instruction| {
            matches!(
                instruction.op,
                tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
            )
        })
    }

    /// Rung 3: a procedure that calls a pack command whose backing is a Tcl
    /// body has the body inlined, and the function records the binding that
    /// holds the live command to it and the claim on the pack that declared
    /// it. The artefact's source is the module's own, without the definition
    /// the inliner appended, and the manifest lists the pack.
    #[test]
    fn a_reference_body_is_inlined_and_claims_the_pack_s_facts() {
        use tcl_runtime_api::CompileService;

        let mut registry = CommandRegistry::build_default();
        install(&mut registry, body_backed("vdouble", DOUBLE));
        let facts = stamp(&registry);
        let source = "proc caller {n} {vdouble $n}\nset x 1";
        let module = service_over(registry).compile(source).unwrap();

        let caller = &module.procedures["::caller"];
        assert_eq!(caller.procedure_bindings, vec![binding()]);
        assert_eq!(
            caller.site_claims,
            vec![SiteClaim::ReferenceBody {
                procedure: binding(),
                backing: tcl_runtime_api::BackingKind::TclBody,
                facts: facts.clone(),
            }]
        );
        assert!(!calls_a_command(caller), "{:#?}", caller.instructions);
        assert!(
            caller
                .instructions
                .iter()
                .any(|instruction| instruction.source_cmd_text == "expr {$x * 2}"),
            "the inlined commands keep the text of the definition they came from: {:#?}",
            caller.instructions
        );
        assert_eq!(
            module.source, source,
            "the appended definition is no part of it"
        );
        assert_eq!(
            module
                .manifest
                .as_ref()
                .expect("the compiler fills it")
                .packs,
            vec![facts]
        );
        assert!(module.top_level.site_claims.is_empty());
    }

    /// A script's global level is Tcl frame zero's and holds no local variable
    /// table, so a call there stays a call; a procedure-body compile has a frame
    /// of its own, and the definition is inlined and claimed there too.
    #[test]
    fn a_global_level_call_stays_a_call_and_a_procedure_body_compile_inlines() {
        use tcl_runtime_api::{CompileService, ProcedureCompileTarget, ProcedureDispatch};

        let mut registry = CommandRegistry::build_default();
        install(&mut registry, body_backed("vdouble", DOUBLE));
        let facts = stamp(&registry);
        let service = service_over(registry);

        let script = service.compile("vdouble 21").unwrap();
        assert!(calls_a_command(&script.top_level));
        assert!(script.top_level.procedure_bindings.is_empty());
        assert!(script.top_level.site_claims.is_empty());

        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let parameters = vec!["n".to_owned()];
        let body = service
            .compile_procedure_for_profile(
                ProcedureCompileTarget {
                    source: "vdouble $n",
                    parameters: &parameters,
                    namespace: "",
                },
                profile,
                ProcedureDispatch::Optimised,
            )
            .unwrap();
        assert_eq!(body.top_level.procedure_bindings, vec![binding()]);
        assert_eq!(
            body.top_level.site_claims,
            vec![SiteClaim::ReferenceBody {
                procedure: binding(),
                backing: tcl_runtime_api::BackingKind::TclBody,
                facts,
            }]
        );
        assert!(!calls_a_command(&body.top_level));

        let plain = service
            .compile_procedure_for_profile(
                ProcedureCompileTarget {
                    source: "vdouble $n",
                    parameters: &parameters,
                    namespace: "",
                },
                profile,
                ProcedureDispatch::Plain,
            )
            .unwrap();
        assert!(plain.top_level.procedure_bindings.is_empty());
        assert!(plain.top_level.site_claims.is_empty());
        assert!(calls_a_command(&plain.top_level));
    }

    /// What is never inlined: a command no pack supplied, whatever its
    /// backing says; a command the module defines for itself, whose definition
    /// the module keeps; and a text that is not exactly the definition of the
    /// command it backs.
    #[test]
    fn only_the_definition_a_pack_declared_for_the_command_is_inlined() {
        use tcl_runtime_api::CompileService;
        let caller = "proc caller {n} {vdouble $n}";
        let inlined = |registry: CommandRegistry, source: &str| {
            let module = service_over(registry).compile(source).unwrap();
            let function = &module.procedures["::caller"];
            (
                function.procedure_bindings.clone(),
                function.site_claims.clone(),
            )
        };

        // No pack supplied the spec: the embedder's own.
        let mut embedder = CommandRegistry::build_default();
        embedder.insert(body_backed("vdouble", DOUBLE));
        assert_eq!(inlined(embedder, caller), (vec![], vec![]));

        // A later spec at the name shadows the pack's, whose body is then not
        // the command any more.
        let mut shadowed = CommandRegistry::build_default();
        install(&mut shadowed, body_backed("vdouble", DOUBLE));
        shadowed.insert(CommandSpec {
            name: "vdouble",
            runtime_backing: tcl_registry::RuntimeBacking::HostNative,
            ..CommandSpec::DEFAULT
        });
        assert_eq!(inlined(shadowed, caller), (vec![], vec![]));

        // The module defines the name itself: its definition is the live one.
        let mut registry = CommandRegistry::build_default();
        install(&mut registry, body_backed("vdouble", DOUBLE));
        let own = format!("proc vdouble {{x}} {{expr {{$x * 3}}}}\n{caller}");
        assert_eq!(inlined(registry, &own), (vec![], vec![]));

        for text in [
            "proc other {x} {expr {$x * 2}}",
            "proc vdouble {x} {expr {$x * 2}}\nset y 1",
            "proc vdouble {x} {expr {$x * 2}}\nproc vdouble {x} {expr {$x * 3}}",
            "set body {expr {$x * 2}}\nproc vdouble {x} $body",
            "expr {1 + 1}",
        ] {
            let mut registry = CommandRegistry::build_default();
            install(&mut registry, body_backed("vdouble", text));
            assert_eq!(inlined(registry, caller), (vec![], vec![]), "{text}");
        }
    }

    /// A backing that is not a Tcl body supplies nothing to inline, even when
    /// a procedure of the same text is what runs.
    #[test]
    fn a_host_native_backing_supplies_no_body() {
        use tcl_runtime_api::CompileService;
        let mut registry = CommandRegistry::build_default();
        install(
            &mut registry,
            CommandSpec {
                name: "vdouble",
                runtime_backing: tcl_registry::RuntimeBacking::HostNative,
                ..CommandSpec::DEFAULT
            },
        );
        let module = service_over(registry)
            .compile("proc caller {n} {vdouble $n}")
            .unwrap();
        let caller = &module.procedures["::caller"];
        assert!(caller.procedure_bindings.is_empty());
        assert!(caller.site_claims.is_empty());
        assert!(calls_a_command(caller));
    }

    /// Rung 2 on the inline path: a pack command whose `alias_of lindex`
    /// carries `lindex`'s own inline hook records the target's binding and
    /// the claim beside it.
    #[test]
    fn an_inline_alias_site_claims_the_builtin() {
        let mut registry = CommandRegistry::build_default();
        install(
            &mut registry,
            CommandSpec {
                name: "vendor::nth",
                alias_of: Some("lindex"),
                inline_codegen_hook: Some(InlineCodegenHookId::Lindex),
                ..CommandSpec::DEFAULT
            },
        );
        let module = compile("set l {a b c}\nset x [vendor::nth $l 1]\nset x", &registry);
        let binding = CommandBindingIdentity::new("vendor::nth", "lindex");
        assert!(
            module.top_level.command_bindings.contains(&binding),
            "{:#?}",
            module.top_level.command_bindings
        );
        assert_eq!(
            module.top_level.site_claims,
            vec![SiteClaim::BuiltinAlias {
                binding,
                facts: stamp(&registry),
            }]
        );
    }
}
