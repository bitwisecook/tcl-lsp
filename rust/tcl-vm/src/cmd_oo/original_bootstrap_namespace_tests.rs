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

use super::*;
use tcl_runtime_api::{Namespaces, ROOT_NS};

#[test]
fn original_c91_bootstrap_objects_publish_their_actual_private_namespaces() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.1"));
    for (name, object) in [
        ("::oo::object", vm.oo.object_root.unwrap()),
        ("::oo::class", vm.oo.class_root.unwrap()),
        ("::oo::configurable", vm.oo.configurable_root.unwrap()),
    ] {
        let namespace = vm.oo.objects[&object]
            .namespace_token
            .expect("original object namespace incarnation");
        let reported = vm
            .try_invoke_command(
                "info",
                &[
                    Value::string("object"),
                    Value::string("namespace"),
                    Value::string(name),
                ],
            )
            .expect("actual public object namespace observation");
        assert_eq!(reported.code, Code::Ok, "{name}");
        assert_eq!(
            Namespaces::find_namespace_bytes(&vm, ROOT_NS, &reported.result.string_bytes()),
            Some(namespace),
            "{name} must report its actually published namespace incarnation"
        );
        assert!(Namespaces::namespace_is_live(&vm, namespace), "{name}");
        let exists = vm
            .try_invoke_command("namespace", &[Value::string("exists"), reported.result])
            .expect("actual public namespace existence observation");
        assert_eq!(exists.code, Code::Ok, "{name}");
        assert_eq!(exists.result.string_bytes().as_ref(), b"1", "{name}");
    }
    let absent = vm
        .try_invoke_command(
            "namespace",
            &[
                Value::string("exists"),
                Value::string("::missing_original_oo_private_namespace"),
            ],
        )
        .expect("actual absent namespace observation");
    assert_eq!(absent.code, Code::Ok);
    assert_eq!(absent.result.string_bytes().as_ref(), b"0");
    assert_eq!(
        Namespaces::find_namespace_bytes(&vm, ROOT_NS, b"::missing_original_oo_private_namespace"),
        None
    );
}
