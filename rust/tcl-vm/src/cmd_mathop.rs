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

//! `::tcl::mathop::*` commands use their retained handler identity and shared
//! `tcl_cmd_core::mathop` fold/chain logic over the VM's `ExprEval`.

use tcl_runtime_api::Completion;

use crate::expr::ExprEval;
use crate::interp::Vm;
use crate::value::Value;

use tcl_syntax::expr::operators::{ALL_BIN_OPS, ALL_UNARY_OPS};

/// Every operator spelling with a `::tcl::mathop` command form.
///
/// Derived from the operator grammar in `tcl_syntax::expr::operators` —
/// layer 1 already knows which operators exist and which have a command
/// form at all — rather than from a hand-typed macro invocation that could
/// silently drift from it. This is the same derivation
/// `runtime/rust/src/cmd_mathop.rs::mathop_names` performs.
///
/// `BinOp`/`UnaryOp` share a spelling for `-`/`+` (`Sub`/`Neg`, `Add`/`Pos`) —
/// one command handles both the fold and the single-argument reading, so the
/// binary pass alone already covers them; the unary pass only contributes
/// truly unary-only spellings (`~`, `!`).
fn mathop_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = ALL_BIN_OPS
        .iter()
        .filter_map(|op| op.spec().mathop_shape.map(|_| op.spec().spelling))
        .collect();
    for op in ALL_UNARY_OPS {
        if op.spec().mathop_shape.is_some() {
            let spelling = op.spec().spelling;
            if !names.contains(&spelling) {
                names.push(spelling);
            }
        }
    }
    names
}

/// The selected handler identity owns the operation. Original invocation words
/// are accessed only when the native arity presenter needs them.
fn mathop(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    use tcl_cmd_core::mathop::MathopError;
    let Some(op) = vm.invoked_builtin_identity().and_then(|identity| {
        tcl_cmd_core::mathop::operation_for_handler_identity(identity.as_bytes())
    }) else {
        return vm.refuse_host_command("math operator handler identity is unavailable".into());
    };
    let mut ops = ExprEval::new(vm);
    match tcl_cmd_core::mathop::eval(&mut ops, op, args.to_vec()) {
        Ok(v) => ops.finish(v),
        Err(MathopError::WrongArgs(usage)) => {
            let Some(original) = vm.invoked_name_value() else {
                return vm
                    .refuse_host_command("math operator invocation word is unavailable".into());
            };
            let mut header = match vm.native_argument_usage_header(&[original]) {
                Ok(header) => header,
                Err(error) => return error,
            };
            header.push(b' ');
            header.extend_from_slice(usage.as_bytes());
            crate::command::native_wrong_args_bytes(vm, &header)
        }
        Err(MathopError::Op(e)) => crate::command::completion_from_tcl_error(ops.vm, e),
    }
}

/// Register `::tcl::mathop::*`.
pub(crate) fn register(vm: &mut Vm) {
    for op in mathop_names() {
        vm.register_stock_builtin(&format!("::tcl::mathop::{op}"), mathop);
    }
    // C exports every operator from `::tcl::mathop`, so
    // `namespace import ::tcl::mathop::*` works (mathop-25.*).
    vm.declare_namespace_exports("tcl::mathop", &["*"]);
}

#[cfg(test)]
mod tests {
    use tcl_syntax::expr::operators::{ALL_BIN_OPS, ALL_UNARY_OPS};

    use crate::interp::Vm;

    #[test]
    fn selected_mathop_identity_matches_all_60_native_name_and_argv_controls() {
        let rows = include_str!("../../tcl-cmd-core/tests/data/native_mathop_identity/rows.txt");
        let decode = |text: &str| {
            text.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect::<Vec<_>>()
        };
        let mut count = 0;
        for row in rows.lines() {
            let fields: Vec<_> = row.split('\t').collect();
            let profile =
                tcl_registry::model::ingress::resolve_environment(fields[0]).unit_profile();
            let mut vm = crate::native_fixture::interpreter(profile);
            let source = String::from_utf8(decode(fields[4])).unwrap();
            let completion = vm.eval_source(&source).unwrap_or_else(|error| {
                panic!(
                    "{}/{} native mathop source: {error:?}",
                    fields[0], fields[1]
                )
            });
            assert_eq!(
                completion.code.as_int(),
                fields[2].parse::<i64>().unwrap(),
                "{}/{}",
                fields[0],
                fields[1]
            );
            assert_eq!(
                vm.native_name_operand_bytes(&completion.result)
                    .unwrap()
                    .as_ref(),
                decode(fields[3]),
                "{}/{}",
                fields[0],
                fields[1]
            );
            count += 1;
        }
        assert_eq!(count, 60);
    }

    #[test]
    fn original_mathop_compilation_matches_all_84_native_controls() {
        let rows = include_str!("../../tcl-cmd-core/tests/data/native_mathop_compilation/rows.txt");
        let decode = |text: &str| {
            text.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect::<Vec<_>>()
        };
        let mut count = 0;
        for row in rows.lines() {
            let fields: Vec<_> = row.split('\t').collect();
            let profile =
                tcl_registry::model::ingress::resolve_environment(fields[0]).unit_profile();
            let mut vm = crate::native_fixture::interpreter(profile);
            let source = String::from_utf8(decode(fields[4])).unwrap();
            let completion = vm.eval_source(&source).unwrap_or_else(|error| {
                panic!(
                    "{}/{} native mathop source: {error:?}",
                    fields[0], fields[1]
                )
            });
            assert_eq!(
                completion.code.as_int(),
                fields[2].parse::<i64>().unwrap(),
                "{}/{}",
                fields[0],
                fields[1]
            );
            assert_eq!(
                vm.native_name_operand_bytes(&completion.result)
                    .unwrap()
                    .as_ref(),
                decode(fields[3]),
                "{}/{}",
                fields[0],
                fields[1]
            );
            count += 1;
        }
        assert_eq!(count, 84);
    }

    /// Operator spellings owned by the shared expression grammar, compared
    /// against the installed command set.
    fn expected_mathop_spellings() -> Vec<&'static str> {
        let mut names: Vec<&'static str> = ALL_BIN_OPS
            .iter()
            .filter_map(|op| op.spec().mathop_shape.map(|_| op.spec().spelling))
            .collect();
        for op in ALL_UNARY_OPS {
            if op.spec().mathop_shape.is_some() {
                let spelling = op.spec().spelling;
                if !names.contains(&spelling) {
                    names.push(spelling);
                }
            }
        }
        names
    }

    #[test]
    fn every_layer1_mathop_operator_is_registered_in_the_vm() {
        let vm = Vm::new();
        for spelling in expected_mathop_spellings() {
            let full = format!("::tcl::mathop::{spelling}");
            assert!(
                vm.lookup_command(&full).is_some(),
                "{full}: exists in tcl_syntax::expr::operators but not registered in the VM"
            );
        }
    }

    #[test]
    fn the_vm_registers_no_mathop_command_beyond_layer1() {
        let vm = Vm::new();
        let expected = expected_mathop_spellings();
        // The explicit expected surface is checked against both installed
        // commands and shared expression grammar.
        let registered = [
            "~", "!", "+", "-", "*", "/", "%", "**", "&", "|", "^", "<<", ">>", "==", "!=", "<",
            "<=", ">", ">=", "eq", "ne", "lt", "le", "gt", "ge", "in", "ni",
        ];
        for spelling in registered {
            let full = format!("::tcl::mathop::{spelling}");
            assert!(
                vm.lookup_command(&full).is_some(),
                "{full}: in this test's own registered list but not actually registered"
            );
            assert!(
                expected.contains(&spelling),
                "{full}: registered in the VM but has no `mathop_shape` in \
                 tcl_syntax::expr::operators — stale entry?"
            );
        }
        assert_eq!(
            registered.len(),
            expected.len(),
            "the VM's registered mathop spellings and layer 1's mathop-shaped \
             operators have drifted apart (different counts)"
        );
    }
}
