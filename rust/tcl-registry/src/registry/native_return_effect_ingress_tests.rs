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

use super::native_return_state_effect;
use crate::completion_route::ReturnStateEffect;
use crate::{InvocationArguments, InvocationDialect, InvocationWord};

#[test]
fn authored_return_effect_uses_unanimous_unknown_grammar_only() {
    let logical = InvocationDialect::of_profile(tcl_dialect::DialectProfile::plain_tcl());
    let literal = [InvocationWord::Literal("NS")];
    let arguments = InvocationArguments::structured(&literal).with_dialect(logical);
    assert_eq!(
        super::authored_source_return_state_effect(arguments),
        ReturnStateEffect::ResultAndCompletion
    );
    assert_eq!(
        native_return_state_effect(arguments),
        ReturnStateEffect::MayMaterialiseError
    );
    let dynamic = [InvocationWord::Dynamic, InvocationWord::Literal("value")];
    let arguments = InvocationArguments::structured(&dynamic).with_dialect(logical);
    assert_eq!(
        super::authored_source_return_state_effect(arguments),
        ReturnStateEffect::MayMaterialiseError
    );
    let bad = [
        InvocationWord::Literal("-code"),
        InvocationWord::Literal("error"),
        InvocationWord::Literal("message"),
    ];
    let arguments = InvocationArguments::structured(&bad).with_dialect(logical);
    assert_eq!(
        super::authored_source_return_state_effect(arguments),
        ReturnStateEffect::MayMaterialiseError
    );
}

#[test]
fn native_return_effect_requires_its_independent_selected_protocol() {
    // naming.minifier.logical-formal-binding-alpha
    // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
    let arguments = [InvocationWord::Dynamic];
    let logical = InvocationDialect::of_profile(tcl_dialect::DialectProfile::plain_tcl());
    assert!(logical.authored_name_policy().is_some());
    assert!(logical.native_name_protocol().is_none());
    for dialect in [None, Some(logical)] {
        let words = InvocationArguments::structured(&arguments);
        let words = dialect.map_or(words, |dialect| words.with_dialect(dialect));
        assert_eq!(
            native_return_state_effect(words),
            ReturnStateEffect::MayMaterialiseError
        );
    }
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        assert_eq!(
            native_return_state_effect(
                InvocationArguments::structured(&arguments).with_dialect(dialect)
            ),
            ReturnStateEffect::ResultAndCompletion,
            "{version:?}"
        );
    }
    let jim = InvocationDialect::of_profile(
        crate::model::ingress::resolve_environment("jim").unit_profile(),
    );
    assert_eq!(
        native_return_state_effect(InvocationArguments::structured(&arguments).with_dialect(jim)),
        ReturnStateEffect::ResultAndCompletion
    );
    let unmeasured = InvocationDialect {
        core_point: Some(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_79,
        )),
        tcl_version: None,
        ..jim
    };
    assert!(unmeasured.native_name_protocol().is_none());
    assert_eq!(
        native_return_state_effect(
            InvocationArguments::structured(&arguments).with_dialect(unmeasured)
        ),
        ReturnStateEffect::MayMaterialiseError
    );
    // TMM's authored Tcl 8.4 core selects the Tcl 8.4 return handler.
    let irules = InvocationDialect::of_profile(
        crate::model::ingress::resolve_environment("f5-irules").unit_profile(),
    );
    assert!(irules.native_name_protocol().is_none());
    assert!(irules.authored_f5_tcl84_core().is_some());
    assert_eq!(
        native_return_state_effect(
            InvocationArguments::structured(&arguments).with_dialect(irules)
        ),
        ReturnStateEffect::ResultAndCompletion
    );
}
