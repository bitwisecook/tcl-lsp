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

//! Actual TclOO private info ensemble registrations and original operands.

use crate::interp::{Code, Interp};
use crate::obj::TclObj;
use tcl_registry::commands::tcl::{info_oo_subcommands, InfoOoEnsembleKind};
use tcl_registry::invocation_words::EnsembleImplementationFamily;

pub(super) fn install(interp: &mut Interp) {
    let dialect = interp.native_invocation_dialect();
    let Some(version) = dialect.tcl_version else {
        return;
    };
    for (member, kind, callback, workers) in [
        (
            "class",
            InfoOoEnsembleKind::Class,
            super::stock_class as crate::interp::BuiltinFn,
            CLASS_WORKERS,
        ),
        (
            "object",
            InfoOoEnsembleKind::Object,
            super::stock_object as crate::interp::BuiltinFn,
            OBJECT_WORKERS,
        ),
    ] {
        let Some(namespace) = dialect.info_oo_ensemble_namespace(member) else {
            continue;
        };
        let admitted = info_oo_subcommands(kind, version);
        let selected: Vec<_> = workers
            .iter()
            .copied()
            .filter(|(name, _)| {
                admitted
                    .names()
                    .iter()
                    .any(|admitted| admitted.as_bytes() == *name)
            })
            .collect();
        interp.register_stock_nested_ensemble_with_prefixes(
            EnsembleImplementationFamily::Info,
            namespace.as_bytes(),
            callback,
            &selected,
            true,
        );
    }
}

macro_rules! workers {
    ($table:ident, $parent:literal, $($function:ident => $member:literal),+ $(,)?) => {
        const $table: &[(&[u8], crate::interp::BuiltinFn)] = &[
            $(($member.as_bytes(), $function)),+
        ];
        $(fn $function(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
            interp.invoke_stock_worker(argv, &[b"info", $parent, $member.as_bytes()], super::info_cmd)
        })+
    };
}

workers! {
    CLASS_WORKERS, b"class",
    class_call => "call",
    class_constructor => "constructor",
    class_definition => "definition",
    class_definitionnamespace => "definitionnamespace",
    class_destructor => "destructor",
    class_filters => "filters",
    class_forward => "forward",
    class_instances => "instances",
    class_methods => "methods",
    class_methodtype => "methodtype",
    class_mixins => "mixins",
    class_properties => "properties",
    class_subclasses => "subclasses",
    class_superclasses => "superclasses",
    class_variables => "variables",
}

workers! {
    OBJECT_WORKERS, b"object",
    object_call => "call",
    object_class => "class",
    object_creationid => "creationid",
    object_definition => "definition",
    object_filters => "filters",
    object_forward => "forward",
    object_isa => "isa",
    object_methods => "methods",
    object_methodtype => "methodtype",
    object_mixins => "mixins",
    object_namespace => "namespace",
    object_properties => "properties",
    object_variables => "variables",
    object_vars => "vars",
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::obj;

    #[test]
    fn original_tcloo_info_bootstrap_preserves_private_ensemble_targets() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let available = interp
                .native_invocation_dialect()
                .info_oo_ensemble_namespace("class")
                .is_some();
            for name in [b"::oo::InfoClass".as_slice(), b"::oo::InfoObject"] {
                if !available {
                    assert!(
                        interp.resolve_cmd_token(name).is_none(),
                        "{engine} {name:?}"
                    );
                    continue;
                }
                let binding = interp
                    .native_compilation_binding_at(crate::namespace::GLOBAL, name)
                    .unwrap();
                assert_eq!(
                    binding.as_ref().is_some_and(|binding| binding
                        .compiler
                        .as_ref()
                        .is_some_and(|compiler| compiler.ensemble.is_some())),
                    available,
                    "{engine} {name:?}"
                );
            }
            for name in [b"::tcl::info::class".as_slice(), b"::tcl::info::object"] {
                assert!(interp.resolve_cmd_token(name).is_none(), "{engine}");
            }
            if !available {
                continue;
            }
            for source in [
                b"info class superclasses oo::class".as_slice(),
                b"::oo::InfoClass::superclasses oo::class",
            ] {
                let objects: Vec<_> = source
                    .split(|byte| *byte == b' ')
                    .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
                    .collect();
                let argv: Vec<_> = objects.iter().map(obj::Owned::as_ptr).collect();
                assert_eq!(
                    interp.eval_original_object_vector(&argv),
                    Code::Ok,
                    "{engine}"
                );
                assert_eq!(interp.result_bytes(), b"::oo::object", "{engine}");
            }
        }
    }
}
