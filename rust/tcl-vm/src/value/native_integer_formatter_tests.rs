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
use tcl_platform::{NativeIntegerFormatter, NativeIntegerKind};

#[test]
fn actual_c84_formatter_materializes_original_minimum_children_without_cache_or_owner_changes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let formatter =
        tcl_test_support::native_integer_formatter::load_pinned_c84_integer_formatter(&root)
            .expect("explicit pinned native formatter");
    let dialect = tcl_registry::InvocationDialect::of_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.4").unit_profile(),
    );
    let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4);
    for (kind, cache) in [
        (
            NativeIntegerKind::Long,
            NativeScalarCache::Tcl84Long(i64::MIN),
        ),
        (
            NativeIntegerKind::Wide,
            NativeScalarCache::Number(Number::Int(i64::MIN)),
        ),
    ] {
        // Without a formatter the updater spells INT64_MIN as tclsh 8.4.20's
        // TclFormatInt does on LP64: `expr {-9223372036854775807 - 1}` and
        // `expr {wide(-9223372036854775807) - 1}` both print `-8`.
        let unformatted = Value::from_native_scalar_cache(cache.clone(), None, dialect).unwrap();
        assert_eq!(
            unformatted.native_string_bytes(protocol).unwrap().as_ref(),
            b"-8"
        );
        let original = Value::from_native_scalar_cache(cache.clone(), None, dialect).unwrap();
        assert!(original.resident_string_bytes().is_none());
        let list = Value::native_list_constructor(vec![original], protocol);
        let expected = formatter.format(kind, i64::MIN).unwrap();
        let members = list.cached_list_representation().unwrap().0;
        let child = &members[0];
        let identity = child.native_object_identity();
        let references = child.native_object_reference_count();
        let bytes = list
            .native_string_bytes_with_integer_formatter(protocol, Some(&formatter))
            .unwrap();
        assert_eq!(bytes.as_ref(), expected.as_slice());
        assert_eq!(child.native_object_identity(), identity);
        assert_eq!(child.native_object_reference_count(), references);
        assert_eq!(child.native_scalar_cache(), Some(cache));
        assert_eq!(
            child.resident_string_bytes().unwrap().as_ref(),
            expected.as_slice()
        );
    }
    let resident = Value::from_native_scalar_cache_with_storage(
        NativeScalarCache::Number(Number::Int(i64::MIN)),
        None,
        dialect,
        Some((
            Rc::from(&b"original resident"[..]),
            NativeStringStorageIdentity::Allocated,
        )),
    )
    .unwrap();
    let before = resident.resident_string_bytes().unwrap();
    let after = resident
        .native_string_bytes_with_integer_formatter(protocol, Some(&formatter))
        .unwrap();
    assert!(Rc::ptr_eq(&before, &after));
}
