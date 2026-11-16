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
use crate::{InvocationArguments, InvocationDialect, InvocationWord};

#[test]
fn created_handle_eval_body_owns_only_the_canonical_single_script_ordinal() {
    // naming.interpreter.original-created-handle-source-body
    // docs/design/analysis/name-resolution-proofs/interpreter-original-created-handle-source-body.md
    let create = InterpreterTransition::Create {
        interpreter: Some(TransitionSubject::LocatedLiteral {
            value: "s".to_owned(),
            argument_index: 1,
        }),
        safety: ChildInterpreterSafety::Safe,
    };
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        let argv = [InvocationWord::Literal("eval"), InvocationWord::Dynamic];
        let body = create
            .created_handle_eval_body(
                dialect,
                InvocationArguments::Structured(&argv).with_dialect(dialect),
            )
            .unwrap();
        assert_eq!(body.argument_index(), Some(1));
        assert!(body.literal().is_none());
        for argv in [
            vec![
                InvocationWord::Literal("e"),
                InvocationWord::Literal("script"),
            ],
            vec![InvocationWord::Dynamic, InvocationWord::Literal("script")],
            vec![InvocationWord::Literal("eval"), InvocationWord::Expanded],
            vec![
                InvocationWord::Literal("eval"),
                InvocationWord::Literal("script"),
                InvocationWord::Literal("more"),
            ],
        ] {
            assert!(
                create
                    .created_handle_eval_body(
                        dialect,
                        InvocationArguments::Structured(&argv).with_dialect(dialect)
                    )
                    .is_none()
            );
        }
    }
    let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
        tcl_dialect::model::Release::JIM_0_84,
    ));
    assert!(
        create
            .created_handle_eval_body(
                jim,
                InvocationArguments::Literals(&["eval", "script"]).with_dialect(jim)
            )
            .is_none()
    );
}
