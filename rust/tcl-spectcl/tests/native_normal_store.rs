// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Pack implementation results enter only their owned original parent store.

use std::rc::Rc;
use tcl_compiler::analyses::{ConstValue, LatticeValue};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_registry::pack_hooks;
use tcl_registry::value_transfer::EvalRoute;
use tcl_spec_hooks::tclvm_host;
use tcl_spectcl::discovery::{Origin, PackFile, Tier};

struct HostReset;
impl Drop for HostReset {
    fn drop(&mut self) {
        pack_hooks::clear_host();
    }
}

fn installed_reference(definition: &str, arity: usize) -> tcl_registry::CommandRegistry {
    let directory = std::env::temp_dir().join(format!(
        "native-parent-store-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("native-parent.tclspec");
    let source = format!(
        "speclib vendor 2.0 {{command vendor::f {{arity {arity}\nruntime_backing tcl-body {{-pack-text {{{definition}}} -evaluate}}}}}}\n"
    );
    std::fs::write(&path, source).unwrap();
    let packs = tcl_spectcl::pack::load(&[PackFile {
        tier: Tier::Workspace,
        path,
        origin: Origin::DotDir,
        dependency_tier: None,
    }]);
    let host = Rc::new(tclvm_host());
    for programs in tcl_spectcl::hooks::plan_for(&packs).packs() {
        let installed = host.install_pack_hooks(programs.clone());
        assert!(
            installed.iter().all(|entry| entry.declined.is_none()),
            "{installed:?}"
        );
    }
    pack_hooks::install_host(host);
    tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &packs)
}

#[test]
fn native_parent_store_uses_the_actual_pack_implementation_normal_outcome() {
    let _reset = HostReset;
    for (definition, arity, call, expected) in [
        (
            "proc vendor::f {x} {expr {$x * 2}}",
            1,
            "vendor::f 21",
            Some("42"),
        ),
        (
            "proc vendor::f {a b} {string cat $a $b}",
            2,
            "vendor::f x y",
            Some("xy"),
        ),
        (
            "proc vendor::f {x} {expr {$x * 2}}",
            1,
            "vendor::f $::unknown",
            None,
        ),
        (
            "proc vendor::f {x} {upvar 1 $x y; expr {$y * 2}}",
            1,
            "vendor::f 21",
            None,
        ),
    ] {
        let registry = installed_reference(definition, arity);
        if expected.is_some() {
            let args = if arity == 1 {
                vec!["21"]
            } else {
                vec!["x", "y"]
            };
            let resolved = registry
                .resolve_invocation("vendor::f", &args, None)
                .unwrap();
            assert!(matches!(
                resolved.semantics.value.semantics().unwrap().route(),
                EvalRoute::Implementation(_)
            ));
        }
        let source = format!("proc ::p {{}} {{set known [{call}]; return $known}}\n");
        let unit = CompilationUnit::build_for_dialect(&source, &registry, false, "tcl8.6");
        let mut rows = unit
            .ir_module
            .procedures
            .iter()
            .chain(unit.ir_module.body_lowerings())
            .filter(|(body, procedure)| {
                procedure.qualified_name == "::p"
                    && unit
                        .original_procedure_for_body(**body)
                        .is_some_and(|original| std::ptr::eq(original, *procedure))
            });
        let (body, procedure) = rows.next().unwrap();
        assert!(rows.next().is_none());
        let function = unit.function_for_body(Some(*body)).unwrap();
        assert_eq!(function.body_id, Some(*body));
        assert!(procedure.body.executed_source.is_none());
        assert!(procedure.body.native_compilation_admission.is_none());
        let symbol = function.ssa.var_symbol("known").unwrap();
        let value = match function.sccp.values.get(&(symbol, 1)) {
            Some(LatticeValue::Const(ConstValue::Int(value))) => Some(value.to_string()),
            Some(LatticeValue::Const(ConstValue::String(value))) => Some(value.clone()),
            _ => None,
        };
        assert_eq!(value.as_deref(), expected, "{definition}\n{call}");
    }
}
