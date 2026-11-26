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

//! May-write declarations over every loadable dialect and the shipped packs.

use super::*;

/// The may-write declarations: each
/// answers a may-bind of its `VarWrite` operand on the normal path, as the
/// kind the command binds — an array for `file stat` and `file lstat`, a
/// scalar for `file tempfile`'s name variable, `gets`, `chan gets` and
/// `tk_optionMenu`, either for the four `trace` forms — except `vwait`,
/// whose wait an unset ends too, so its transfer stays generic.
#[test]
fn each_may_write_declaration_answers_a_may_bind_of_its_target() {
    use tcl_registry::value_transfer::{BindingKind, LiteralInputs};
    type Declaration = (
        &'static str,
        Option<&'static str>,
        &'static [&'static str],
        Option<BindingKind>,
    );
    let declarations: [Declaration; 11] = [
        ("file", Some("stat"), &["f", "st"], Some(BindingKind::Array)),
        (
            "file",
            Some("lstat"),
            &["f", "st"],
            Some(BindingKind::Array),
        ),
        (
            "file",
            Some("tempfile"),
            &["path"],
            Some(BindingKind::Scalar),
        ),
        ("gets", None, &["chan", "line"], Some(BindingKind::Scalar)),
        (
            "chan",
            Some("gets"),
            &["chan", "line"],
            Some(BindingKind::Scalar),
        ),
        ("vwait", None, &["done"], None),
        (
            "tk_optionMenu",
            None,
            &[".m", "choice", "a", "b"],
            Some(BindingKind::Scalar),
        ),
        (
            "trace",
            Some("add"),
            &["variable", "v", "write", "cb"],
            Some(BindingKind::Either),
        ),
        (
            "trace",
            Some("remove"),
            &["variable", "v", "write", "cb"],
            Some(BindingKind::Either),
        ),
        (
            "trace",
            Some("variable"),
            &["v", "w", "cb"],
            Some(BindingKind::Either),
        ),
        (
            "trace",
            Some("vdelete"),
            &["v", "w", "cb"],
            Some(BindingKind::Either),
        ),
    ];
    let reg = full_registry();
    for (command, sub, args, kind) in declarations {
        let spec = reg.get(command).expect(command);
        let resolved = match sub {
            Some(name) => resolve_semantics(spec, Some(spec.subcommand(name).expect(name)), None),
            None => resolve_semantics(spec, None, None),
        };
        let semantics = resolved.semantics().expect("a declared semantics");
        assert_eq!(semantics.identity(), "may_write", "{command} {sub:?}");
        let words: Vec<&str> = sub.into_iter().chain(args.iter().copied()).collect();
        let targets = reg.arg_indices_for_role(command, &words, ArgRole::VarWrite);
        assert_eq!(targets.len(), 1, "{command} {sub:?}: one variable operand");
        let target = TargetId(OperandId(targets[0]));
        let inputs =
            LiteralInputs::new(command, sub, args, None).with_role(target.0, ArgRole::VarWrite);
        let answer = semantics.transfer(FactDomain::Existence, &inputs, &mut Budget::unbounded());
        match (kind, answer) {
            (Some(kind), TransferAnswer::Existence(transfer)) => {
                assert_eq!(transfer.paths.len(), 1, "{command} {sub:?}");
                assert_eq!(
                    transfer.paths[0].outcomes,
                    vec![(target, ExistenceOutcome::MayBind(kind))],
                    "{command} {sub:?}"
                );
            }
            (None, TransferAnswer::Generic) => {}
            (kind, answer) => panic!("{command} {sub:?}: {kind:?} answered {answer:?}"),
        }
        assert_eq!(
            semantics.transfer(FactDomain::Type, &inputs, &mut Budget::unbounded()),
            TransferAnswer::Generic,
            "{command} {sub:?}"
        );
    }
}
