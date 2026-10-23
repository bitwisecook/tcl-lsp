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

//! Shared C9 bootstrap descriptor realized by actual Runtime class workers.
use super::*;

#[test]
fn original_abstract_bootstrap_realizes_class_side_visibility_and_release_absence() {
    // Implementation contract: naming.embedding.original-host-publication-and-fact-transport

    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(version),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let original = interpreter.oo_resolve_object_bytes(b"::oo::abstract");
        if version == "tcl8.6" {
            assert!(original.is_err());
            continue;
        }
        let original = original.unwrap();
        {
            let state = interpreter.oo.borrow();
            let class_root = state.class_root.unwrap();
            assert_eq!(state.objects[&original].class, class_root);
            assert_eq!(state.classes[&original].supers, [class_root]);
            assert!(state.classes[&original].constructor.owner().is_none());
            assert!(state.classes[&original].destructor.owner().is_none());
            assert!(state.objects[&original].unexported.is_empty());
            for name in [b"create".as_slice(), b"createWithNamespace", b"new"] {
                assert!(state.classes[&original].unexported.contains(name));
            }
        }
        assert_eq!(
            interpreter.eval_str(b"oo::abstract create AbstractActual {}"),
            Code::Ok
        );
        assert_eq!(interpreter.eval_str(b"AbstractActual new"), Code::Error);
        assert_eq!(interpreter.eval_str(b"oo::class create ConcreteActual {superclass AbstractActual}; ConcreteActual create ActualObject"), Code::Ok);
    }
}
