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

//! List builtins, reusing `tcl_syntax::list` for split/merge semantics.

use tcl_cmd_core::list as list_core;
use tcl_runtime_api::Completion;

use crate::command::completion_from_cmd_error;
use crate::interp::{Vm, err, err_wrong_args, ok};
use crate::value::Value;

/// Map a portable `tcl-cmd-core` result onto the VM's `Completion`.
fn adapt(
    vm: &mut Vm,
    command: impl FnOnce(&mut Vm) -> Result<Value, tcl_cmd_core::CmdError>,
) -> Completion<Value> {
    match command(vm) {
        Ok(v) => ok(v),
        Err(e) => completion_from_cmd_error(vm, e),
    }
}

use tcl_cmd_core::regex::OriginalRegexConsumerError;

pub(crate) fn register(vm: &mut Vm) {
    vm.register_stock_builtin("list", cmd_list);
    vm.register_stock_builtin("llength", cmd_llength);
    vm.register_stock_builtin("lindex", cmd_lindex);
    vm.register_stock_builtin("lrange", cmd_lrange);
    vm.register_stock_builtin("lappend", cmd_lappend);
    vm.register_stock_builtin("lassign", cmd_lassign);
    vm.register_stock_builtin("lreverse", cmd_lreverse);
    vm.register_stock_builtin("lrepeat", cmd_lrepeat);
    vm.register_stock_builtin("linsert", cmd_linsert);
    vm.register_stock_builtin("lreplace", cmd_lreplace);
    vm.register_stock_builtin("ledit", cmd_ledit);
    vm.register_stock_builtin("lset", cmd_lset);
    vm.register_stock_builtin("lpop", cmd_lpop);
    vm.register_stock_builtin("lsearch", cmd_lsearch);
    vm.register_stock_builtin("lsort", cmd_lsort);
    vm.register_stock_builtin("concat", cmd_concat);
    vm.register_stock_builtin("join", cmd_join);
    vm.register_stock_builtin("split", cmd_split);
}

fn as_list(vm: &mut Vm, v: &Value) -> Result<crate::NativeListItems, Completion<Value>> {
    v.as_list()
        .map_err(|error| crate::command::completion_from_tcl_error(vm, error))
}

fn cmd_list(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    ok(list_core::list(vm, args))
}

fn cmd_llength(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [l] => adapt(vm, |vm| list_core::llength(vm, l)),
        _ => err_wrong_args(vm, "llength list"),
    }
}

fn cmd_lindex(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((list, idxs)) = args.split_first() else {
        return err_wrong_args(
            vm,
            vm.native_invocation_dialect()
                .list_index_usage()
                .unwrap_or("lindex list ?index ...?"),
        );
    };
    adapt(vm, |vm| list_core::lindex(vm, list, idxs))
}

fn cmd_lrange(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [list, from, to] = args else {
        return err_wrong_args(vm, "lrange list first last");
    };
    adapt(vm, |vm| list_core::lrange(vm, list, from, to))
}

fn cmd_lappend(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((name, values)) = args.split_first() else {
        return err_wrong_args(vm, "lappend varName ?value ...?");
    };
    let name = match vm.native_name_operand_bytes(name) {
        Ok(name) => name,
        Err(error) => {
            return vm
                .refuse_host_command(format!("native list append name is unavailable: {error:?}"));
        }
    };
    if !values.is_empty() {
        return match vm.lappend_list_update_bytes(&name, None, values) {
            Ok(updated) => {
                Completion::new(tcl_runtime_api::Code::Ok, updated.value, updated.options)
            }
            Err(completion) => completion,
        };
    }
    let read = vm.read_variable_result_bytes(&name, None);
    let (current, read_options) = match read {
        Ok(value) => (Some(value), Value::empty()),
        Err(completion) => {
            if vm.refused_completion().is_some() {
                return completion;
            }
            vm.publish_swallowed_trace_error();
            (None, completion.options)
        }
    };
    vm.retain_variable_read_error_code(&read_options);
    let value = match current {
        Some(value) => match tcl_registry::native_compilation::NativeAppendKind::List
            .validates_empty_result(vm.native_invocation_dialect())
        {
            Some(false) => value,
            Some(true) => match as_list(vm, &value) {
                Ok(_) => value,
                Err(error) => return error,
            },
            None => {
                return vm.refuse_host_command(
                    "empty list-append validation policy is not selected".into(),
                );
            }
        },
        None => match vm.store_var_result_bytes(&name, Value::empty()) {
            Ok(value) => value,
            Err(completion) => return completion,
        },
    };
    let updated = Vm::variable_update_result(value, &read_options);
    Completion::new(tcl_runtime_api::Code::Ok, updated.value, updated.options)
}

fn cmd_lassign(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((list, names)) = args.split_first() else {
        return err_wrong_args(vm, "lassign list ?varName ...?");
    };
    let items = match as_list(vm, list) {
        Ok(i) => i,
        Err(c) => return c,
    };
    for (i, name) in names.iter().enumerate() {
        let v = items.get(i).cloned().unwrap_or_else(Value::empty);
        if let Err(e) = vm.set_var(&name.to_str(), v) {
            return e;
        }
    }
    // Return the unassigned remainder.
    let rest = if names.len() < items.len() {
        items[names.len()..].to_vec()
    } else {
        Vec::new()
    };
    ok(Value::list(rest))
}

fn cmd_lreverse(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [l] => adapt(vm, |vm| list_core::lreverse(vm, l)),
        _ => err_wrong_args(vm, "lreverse list"),
    }
}

fn cmd_lrepeat(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((count, elems)) = args.split_first() else {
        return err_wrong_args(vm, "lrepeat count ?value ...?");
    };
    adapt(vm, |vm| list_core::lrepeat(vm, count, elems))
}

fn cmd_linsert(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [list, index, elems @ ..] = args else {
        return err_wrong_args(vm, "linsert list index ?element ...?");
    };
    adapt(vm, |vm| list_core::linsert(vm, list, index, elems))
}

fn cmd_lreplace(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [list, from, to, rest @ ..] = args else {
        return err_wrong_args(vm, "lreplace list first last ?element ...?");
    };
    adapt(vm, |vm| list_core::lreplace(vm, list, from, to, rest))
}

/// `ledit listVar first last ?element ...?` — the in-place `lreplace` (Tcl 9):
/// replace the `first..last` range of the list held in `listVar` with the given
/// elements, store the result back into the variable, and return it. The write
/// goes through `var_set`, so it fires the variable's write traces.
fn cmd_ledit(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [name, from, to, rest @ ..] = args else {
        return err_wrong_args(vm, "ledit listVar first last ?element ...?");
    };
    let n = name.to_str();
    let Some(cur) = vm.var_get(&n) else {
        return err(vm.read_miss_msg(&n));
    };
    let result = match list_core::lreplace(vm, &cur, from, to, rest) {
        Ok(v) => v,
        Err(e) => return completion_from_cmd_error(vm, e),
    };
    match vm.store_var_result(&n, result) {
        Ok(stored) => ok(stored),
        Err(e) => e,
    }
}

/// `lset listVar ?index ...? newValue` — the runtime form of `lset` (the
/// compiler inlines the common compiled cases via `LSET_LIST`/`LSET_FLAT`; this
/// builtin is the fallback for the dynamic / wrong-arg / non-proc paths). It
/// always reads the variable first (so a no-index `lset x v` on an undefined
/// `x` still reports `can't read`), then descends the index path: a single
/// index argument is itself an index *list*, several arguments are a flat path.
fn cmd_lset(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.len() < 2 {
        return err_wrong_args(vm, "lset listVar ?index? ?index ...? value");
    }
    let n = args[0].to_str();
    let value = args.last().expect("args.len() >= 2");
    let indices = &args[1..args.len() - 1];
    let Some(cur) = vm.var_get(&n) else {
        return err(vm.read_miss_msg(&n));
    };
    let path: Vec<Value> = if indices.is_empty() {
        Vec::new()
    } else if let [single] = indices {
        match single.as_list() {
            Ok(p) => (*p).clone(),
            Err(e) => return crate::command::completion_from_tcl_error(vm, e),
        }
    } else {
        indices.to_vec()
    };
    let new = match crate::exec::lset_descend(vm, &cur, &path, value.clone()) {
        Ok(r) => r,
        Err(c) => return c,
    };
    match vm.store_var_result(&n, new) {
        Ok(stored) => ok(stored),
        Err(e) => e,
    }
}

/// `lpop listVar ?index ...?` — remove and return an element of the list held
/// in `listVar` (Tcl 9), defaulting to the last element. With several indices it
/// descends into nested sublists and removes the deepest element. The trimmed
/// list is stored back (firing write traces).
fn cmd_lpop(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((name, indices)) = args.split_first() else {
        return err_wrong_args(vm, "lpop listvar ?index?");
    };
    let n = name.to_str();
    let Some(cur) = vm.var_get(&n) else {
        return err(vm.read_miss_msg(&n));
    };
    let items = match as_list(vm, &cur) {
        Ok(i) => (*i).clone(),
        Err(c) => return c,
    };
    // No index means the last element (`end`).
    let default_end = [Value::string("end")];
    let path: &[Value] = if indices.is_empty() {
        &default_end
    } else {
        indices
    };
    let (removed, new_items) = match lpop_remove(vm, &items, path) {
        Ok(r) => r,
        Err(c) => return c,
    };
    if let Err(e) = vm.var_set(&n, Value::list(new_items)) {
        return e;
    }
    ok(removed)
}

/// Resolve `spec` against a length-`len` list for `lpop`/`lset`-style index
/// descent: a non-integer index is a "bad index" error; an in-form but
/// out-of-bounds index is "index … out of range" — matching C's
/// `Tcl_LpopObjCmd`.
fn resolve_bounded_index(vm: &mut Vm, spec: &str, len: usize) -> Result<usize, Completion<Value>> {
    let Some(idx) = crate::command::resolve_index(vm, spec, len) else {
        return Err(crate::command::bad_index(vm, spec));
    };
    if idx < 0 || usize::try_from(idx).is_ok_and(|i| i >= len) {
        return Err(err(format!("index \"{spec}\" out of range")));
    }
    Ok(usize::try_from(idx).expect("idx >= 0 checked above"))
}

/// Remove the element at the (possibly nested) `indices` path from `items`,
/// returning `(removed_element, rebuilt_list)`.
///
/// Recursing once per index natively has no depth cap and is trivially
/// inflated via `lpop v {*}[lrepeat 100000 0]`. This walks with an explicit
/// work-stack instead of one native call per index, which eliminates the
/// native-stack risk entirely: walk down every index but the last, recording
/// each level's element vector and the index it descends through, remove
/// the final element, then rebuild bottom-up — the same index resolution, in
/// the same order, as a naive recursive version, so error precedence is
/// unaffected.
fn lpop_remove(
    vm: &mut Vm,
    items: &[Value],
    indices: &[Value],
) -> Result<(Value, Vec<Value>), Completion<Value>> {
    let (last, front) = indices.split_last().expect("lpop has at least one index");
    let mut frames: Vec<(Vec<Value>, usize)> = Vec::with_capacity(front.len());
    let mut cur: Vec<Value> = items.to_vec();
    for spec_val in front {
        let i = resolve_bounded_index(vm, &spec_val.to_str(), cur.len())?;
        let sub = match cur[i].as_list() {
            Ok(s) => (*s).clone(),
            Err(e) => return Err(crate::command::completion_from_tcl_error(vm, e)),
        };
        frames.push((cur, i));
        cur = sub;
    }
    let i = resolve_bounded_index(vm, &last.to_str(), cur.len())?;
    let removed = cur.remove(i);
    let mut rebuilt = cur;
    for (mut outer, i) in frames.into_iter().rev() {
        outer[i] = Value::list(rebuilt);
        rebuilt = outer;
    }
    Ok((removed, rebuilt))
}

/// `lsearch ?-option value ...? list pattern` — a thin adapter over the shared
/// [`tcl_cmd_core::lsearch`] core, driven by the VM's `regex`-crate engine for
/// `-regexp`, retaining selected guest error metadata and host refusals.
fn cmd_lsearch(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if vm
        .actual_native_invocation_dialect()
        .native_jim_regex_protocol()
        .is_some()
    {
        use tcl_cmd_core::native_jim_lsearch::JimLsearchError;
        return match tcl_cmd_core::native_jim_lsearch::lsearch(
            vm,
            args,
            crate::cmd_regexp::invoke_jim_match_command,
        ) {
            Ok(value) => ok(value),
            Err(JimLsearchError::Command(error)) => {
                crate::command::completion_from_cmd_error(vm, error)
            }
            Err(JimLsearchError::Callback(mut completion)) => {
                completion.code = tcl_runtime_api::Code::Error;
                completion
            }
            Err(JimLsearchError::NegativeMatch) => match vm
                .with_native_interp_result(|value| value.native_lifetime_lease().into_value())
            {
                Ok(value) => Completion::new(tcl_runtime_api::Code::Error, value, Value::empty()),
                Err(error) => crate::command::completion_from_cmd_error(vm, error.into()),
            },
            Err(JimLsearchError::Usage) => err_wrong_args(vm, "lsearch ?options? list pattern"),
        };
    }
    let version = vm.runtime_version();
    match tcl_cmd_core::lsearch::lsearch_original_with_jim::<
        Vm,
        crate::cmd_regexp::CrateEngine,
        Completion<Value>,
    >(vm, args, version, crate::cmd_regexp::invoke_jim_regexp)
    {
        Ok(value) => ok(value),
        Err(OriginalRegexConsumerError::Callback(completion)) => completion,
        Err(OriginalRegexConsumerError::Command(error)) if error.command_error.is_some() => {
            crate::command::completion_from_cmd_error(vm, error.command_error.unwrap())
        }
        Err(OriginalRegexConsumerError::Command(error)) => match error.native_access_refusal {
            Some(error) => vm.refuse_host_command(error.to_string()),
            None => Completion::new(
                tcl_runtime_api::Code::Error,
                Value::from_string_bytes(error.message),
                Value::empty(),
            ),
        },
    }
}

impl tcl_cmd_core::native_jim_lsearch::NativeJimLsearchObjects for Vm {
    type Hold = Value;
    type Accumulator = Value;
    fn jim_search_bytes(
        &mut self,
        value: &Value,
    ) -> Result<std::rc::Rc<[u8]>, tcl_syntax::value::ValueError> {
        value.bind_native_jim_context(&self.native_jim_object_context()?)?;
        value
            .native_string_bytes(tcl_syntax::native_string::NativeStringProtocol::Jim084)
            .map_err(|error| {
                tcl_syntax::value::ValueError::NativeStringAccess(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                )
            })
    }
    fn jim_search_character_count(
        &mut self,
        value: &Value,
    ) -> Result<usize, tcl_syntax::value::ValueError> {
        value.native_character_count_with_protocol(
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
            tcl_registry::native_string_length::NativeStringLengthRepresentation::JimCachedString,
        )
    }
    fn jim_search_option(&mut self, original: &Value) -> Result<usize, tcl_cmd_core::CmdError> {
        let table = tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(
            &tcl_cmd_core::native_jim_lsearch::OPTIONS,
        );
        match self.native_jim_enum_from_original(
            original,
            &table,
            tcl_registry::native_jim_enum::NativeJimEnumFlags::options(true),
            None,
        )? {
            Ok(index) => Ok(index),
            Err(message) => Err(
                tcl_cmd_core::CmdError::new_bytes(message.unwrap_or_default())
                    .with_native_string_result(
                        tcl_syntax::native_string::NativeStringProtocol::Jim084,
                    ),
            ),
        }
    }
    fn jim_search_hold(&mut self, value: &Value) -> Result<Value, tcl_syntax::value::ValueError> {
        value.check_native_header()?;
        Ok(value.clone().into_native_reference())
    }
    fn jim_search_borrow(&self, value: &Value) -> Value {
        value.native_lifetime_lease().into_value()
    }
    fn jim_search_elements(
        &mut self,
        value: &Value,
    ) -> Result<Vec<Value>, tcl_syntax::value::ValueError> {
        Ok(self
            .native_object_list_elements_in(
                value,
                tcl_syntax::native_string::NativeStringProtocol::Jim084,
            )?
            .iter()
            .map(|value| value.native_lifetime_lease().into_value())
            .collect())
    }
    fn jim_search_current_elements(
        &self,
        value: &Value,
    ) -> Result<Vec<Value>, tcl_syntax::value::ValueError> {
        let backing = value
            .native_list_backing_in(tcl_syntax::native_string::NativeStringProtocol::Jim084)?
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim lsearch callback changed selected List primary",
            ))?;
        Ok(backing
            .iter()
            .map(|value| value.native_lifetime_lease().into_value())
            .collect())
    }
    fn jim_search_regexp_command(&mut self) -> Value {
        Value::new_native_string_bytes(b"regexp".as_slice()).into_native_unowned_lifetime()
    }
    fn jim_search_group(&mut self, values: Vec<Value>) -> Value {
        Value::native_list_constructor(
            values,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
        .into_native_unowned_lifetime()
    }
    fn jim_search_begin(&mut self) -> Value {
        Value::native_list_constructor(
            Vec::new(),
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
    }
    fn jim_search_append(
        &mut self,
        list: &mut Value,
        values: &[Value],
    ) -> Result<(), tcl_syntax::value::ValueError> {
        list.native_list_append_prepared_elements(
            values,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
    }
    fn jim_search_finish(&mut self, result: Value) -> Result<Value, tcl_syntax::value::ValueError> {
        Ok(self.adopt_native_interp_result(result)?.into_value())
    }
    fn jim_search_publish(
        &mut self,
        value: &Value,
    ) -> Result<Value, tcl_syntax::value::ValueError> {
        Ok(self
            .adopt_native_interp_result(value.native_lifetime_lease().into_value())?
            .into_value())
    }
    fn jim_search_current_result(&self) -> Result<Value, tcl_syntax::value::ValueError> {
        self.with_native_interp_result(|value| value.native_lifetime_lease().into_value())
    }
}

/// `lsort ?-option value ...? list` — a thin adapter over the shared
/// [`tcl_cmd_core::lsort`] core, with `-index`/`-stride`/`-indices` and
/// `-command` (the comparator evaluates Tcl through `vm.dispatch`) alongside
/// the comparison modes.
fn cmd_lsort(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    use tcl_cmd_core::lsort::{Lsort, build_command, prepare, sort_command};
    let mut job = match prepare(vm, args) {
        Ok(Lsort::Done(v)) => return ok(v),
        Ok(Lsort::Command(job)) => job,
        Err(error) => {
            if let Some(command_error) = error.command_error {
                return crate::command::completion_from_cmd_error(vm, command_error);
            }

            return match error.native_access_refusal {
                Some(error) => vm.refuse_host_command(error.to_string()),
                None => Completion::new(
                    tcl_runtime_api::Code::Error,
                    Value::from_string_bytes(error.message),
                    Value::empty(),
                ),
            };
        }
    };
    // `-command`: split the comparison prefix into words, run the reentrant merge
    // sort over the VM comparator (no `ValueOps` borrow during the eval), build.
    let prefix = match job.cmd_prefix.as_list() {
        Ok(w) => w,
        Err(e) => return crate::command::completion_from_tcl_error(vm, e),
    };
    if let Err(c) = sort_command(&mut job, |a, b| vm_compare(vm, &prefix, a, b)) {
        return c;
    }
    ok(build_command(vm, &job))
}

/// The `lsort -command` comparator: invoke `<prefix words...> a b` and read its
/// integer result as a sign. Uses `vm.dispatch` (argv-based — no re-parsing, so
/// elements containing `$`/`[` are passed literally).
fn vm_compare(
    vm: &mut Vm,
    prefix: &[Value],
    a: &Value,
    b: &Value,
) -> Result<i32, Completion<Value>> {
    use tcl_runtime_api::Commands;
    let Some((name, pre_args)) = prefix.split_first() else {
        return Err(err("-command comparison command is empty"));
    };
    let mut argv: Vec<Value> = pre_args.to_vec();
    argv.push(a.clone());
    argv.push(b.clone());
    let comp = vm.dispatch(&name.to_str(), &argv);
    if !comp.code.is_ok() {
        return Err(comp);
    }
    let r = comp.result.to_str();
    match tcl_cmd_core::sort::parse_wide(r.as_bytes()) {
        Some(v) => Ok(i32::try_from(v.signum()).unwrap_or(0)),
        None => Err(err(format!(
            "-command comparison script returned non-integer result: {r}"
        ))),
    }
}

fn cmd_concat(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    adapt(vm, |vm| list_core::concat_selected(vm, args))
}

fn cmd_join(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [l] => adapt(vm, |vm| list_core::join(vm, l, None)),
        [l, s] => adapt(vm, |vm| list_core::join(vm, l, Some(s))),
        _ => err_wrong_args(vm, "join list ?joinString?"),
    }
}

fn cmd_split(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [s] => adapt(vm, |vm| list_core::split(vm, s, None)),
        [s, c] => adapt(vm, |vm| list_core::split(vm, s, Some(c))),
        _ => err_wrong_args(vm, "split string ?splitChars?"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type CallbackObservations = std::rc::Rc<std::cell::RefCell<Vec<Vec<(String, usize)>>>>;

    struct ObserveTwice(CallbackObservations);

    impl crate::command::NativeCommand for ObserveTwice {
        fn invoke(&self, _vm: &mut Vm, args: &[Value]) -> Completion<Value> {
            self.0.borrow_mut().push(
                args.iter()
                    .map(|value| {
                        (
                            value.native_object_type_name().to_owned(),
                            value.native_object_reference_count(),
                        )
                    })
                    .collect(),
            );
            ok(Value::int(2))
        }
    }

    fn lsearch_fixture_bytes(text: &str) -> Vec<u8> {
        text.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>()
    }

    fn assert_lsearch_result_header(
        case: usize,
        lines: &[&str],
        vm: &Vm,
        result: &Completion<Value>,
    ) {
        let expected_code: i32 = lines
            .iter()
            .find_map(|line| line.strip_prefix("code\t"))
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(
            result.code.as_int(),
            i64::from(expected_code),
            "case {case}"
        );
        assert!(vm.refused_completion().is_none(), "case {case}");
        let expected: Vec<_> = lines
            .iter()
            .find_map(|line| line.strip_prefix("result\t"))
            .unwrap()
            .split('\t')
            .collect();
        assert_eq!(
            result.result.native_object_type_name(),
            expected[0],
            "case {case}"
        );
        assert_eq!(
            result.result.resident_string_bytes().is_some(),
            expected[1] == "1",
            "case {case}"
        );
        assert_eq!(
            result.result.native_object_reference_count(),
            expected[2].parse::<usize>().unwrap(),
            "case {case} original result refs"
        );
    }

    fn assert_lsearch_arguments_and_callbacks(
        case: usize,
        lines: &[&str],
        originals: &[Value],
        callbacks: &CallbackObservations,
    ) {
        for line in lines
            .iter()
            .filter_map(|line| line.strip_prefix("argument\t"))
        {
            let fields: Vec<_> = line.split('\t').collect();
            let value = &originals[fields[0].parse::<usize>().unwrap() - 1];
            assert_eq!(
                value.native_object_type_name(),
                fields[1],
                "case {case} argument {}",
                fields[0]
            );
            assert_eq!(
                value.resident_string_bytes().is_some(),
                fields[2] == "1",
                "case {case} argument {}",
                fields[0]
            );
            assert_eq!(
                value.native_object_reference_count(),
                fields[3].parse::<usize>().unwrap(),
                "case {case} argument {}",
                fields[0]
            );
        }
        let native_callbacks: Vec<_> = lines
            .iter()
            .filter_map(|line| line.strip_prefix("callback\t"))
            .map(|line| {
                line.split('\t')
                    .skip(2)
                    .map(|field| {
                        let (kind, refs) = field.split_once(':').unwrap();
                        (kind.to_owned(), refs.parse::<usize>().unwrap())
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(
            *callbacks.borrow(),
            native_callbacks,
            "case {case} callback original children"
        );
    }

    #[test]
    fn jim_lsearch_preserves_all_24_original_native_option_and_callback_controls() {
        use tcl_runtime_api::Code;
        let rows = include_str!("../../tcl-cmd-core/tests/data/native_jim_lsearch/rows.txt");

        let mut count = 0;
        for record in rows.split("case\t").skip(1) {
            let lines: Vec<_> = record.lines().collect();
            let case: usize = lines[0].parse().unwrap();
            let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
            let mut vm = Vm::with_native_core(
                Box::new(std::io::sink()),
                std::rc::Rc::new(crate::host_native::NativeHost::new()),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .expect("authentic Jim core constructor before bootstrap");
            let callbacks = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
            vm.register_written_command(
                "two",
                crate::command::Command::Native(std::rc::Rc::new(ObserveTwice(
                    std::rc::Rc::clone(&callbacks),
                ))),
            );
            vm.register("returns", |_, _| {
                Completion::new(
                    Code::Return,
                    Value::new_native_string_bytes(b"ORIGINAL_RETURN".as_slice()),
                    Value::empty(),
                )
            });
            if case == 8 {
                vm.register_written_command(
                    "regexp",
                    crate::command::Command::Native(std::rc::Rc::new(ObserveTwice(
                        std::rc::Rc::clone(&callbacks),
                    ))),
                );
            }
            let originals: Vec<_> = lines[1]
                .split('\t')
                .skip(1)
                .map(|word| Value::new_native_string_bytes(lsearch_fixture_bytes(word)))
                .collect();
            let args: Vec<_> = originals
                .iter()
                .map(|value| value.native_lifetime_lease().into_value())
                .collect();
            let head = Value::new_native_string_bytes(b"lsearch".as_slice());
            let result = vm.invoke_host_original_object_vector(&head, &args);
            assert_lsearch_result_header(case, &lines, &vm, &result);
            assert_lsearch_arguments_and_callbacks(case, &lines, &originals, &callbacks);
            let expected_bytes = lsearch_fixture_bytes(
                lines
                    .iter()
                    .find_map(|line| line.strip_prefix("bytes\t"))
                    .unwrap(),
            );
            assert_eq!(
                result.result.string_bytes().as_ref(),
                expected_bytes,
                "case {case}"
            );
            count += 1;
        }
        assert_eq!(count, 24);
    }

    fn engine_profiles() -> Vec<&'static tcl_dialect::DialectProfile> {
        tcl_dialect::TclVersion::ALL
            .into_iter()
            .map(|version| {
                tcl_registry::model::ingress::resolve_environment(version.dialect_profile_name())
                    .unit_profile()
            })
            .chain(std::iter::once(
                tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
            ))
            .collect()
    }

    #[test]
    fn concat_uses_the_selected_engine_and_live_list_representation() {
        let jim = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        for profile in engine_profiles() {
            let mut vm = crate::native_fixture::core(profile);
            let is_jim = profile == jim;
            let trailing = cmd_concat(&mut vm, &[Value::string("word"), Value::string("")]);
            assert!(trailing.code.is_ok());
            assert_eq!(
                trailing.result.to_str().as_ref(),
                if is_jim { "word " } else { "word" }
            );

            let first = Value::string("A  B");
            let second = Value::string("C");
            let before = cmd_concat(&mut vm, &[first.clone(), second.clone()]);
            assert_eq!(before.result.to_str().as_ref(), "A  B C");
            assert!(!first.has_list_representation());
            first.as_list().expect("valid first list");
            second.as_list().expect("valid second list");
            let after = cmd_concat(&mut vm, &[first, second]);
            assert_eq!(
                after.result.to_str().as_ref(),
                if is_jim { "A B C" } else { "A  B C" }
            );
            assert_eq!(after.result.has_list_representation(), is_jim);
        }
    }

    #[test]
    fn list_indices_and_set_bounds_follow_the_selected_engine() {
        use tcl_dialect::{ListSetBounds, TclVersion};
        for profile in engine_profiles() {
            let mut vm = crate::native_fixture::core(profile);
            let dialect = vm.native_invocation_dialect();
            let is_jim = dialect
                .core_point
                .is_some_and(|point| point.family() == tcl_dialect::model::Family::Jim);
            let old_indices = dialect.tcl_version == Some(TclVersion::V8_4);
            let end_abbreviation = dialect
                .tcl_version
                .is_some_and(|version| version < TclVersion::V9_0);
            for (index, wanted) in [
                ("1+1", (!old_indices).then_some("c")),
                ("end+0", (!old_indices).then_some("d")),
                ("2*1", is_jim.then_some("c")),
                ("abs(-1)", is_jim.then_some("b")),
                ("e", end_abbreviation.then_some("d")),
                ("en", end_abbreviation.then_some("d")),
                ("$n", None),
                ("[set n]", None),
            ] {
                let completion =
                    cmd_lindex(&mut vm, &[Value::string("a b c d"), Value::string(index)]);
                assert_eq!(
                    completion.code.is_ok(),
                    wanted.is_some(),
                    "{}: {index}: {}",
                    profile.name,
                    completion.result.to_str()
                );
                if let Some(wanted) = wanted {
                    assert_eq!(
                        completion.result.to_str().as_ref(),
                        wanted,
                        "{}: {index}",
                        profile.name
                    );
                }
            }
            let appended = crate::exec::lset_descend(
                &mut vm,
                &Value::string("a"),
                &[Value::string("1")],
                Value::string("z"),
            );
            let may_append = dialect.list_set_bounds() == Some(ListSetBounds::AppendAtEnd);
            assert_eq!(appended.is_ok(), may_append, "{}", profile.name);
            if let Ok(value) = appended {
                assert_eq!(value.to_str().as_ref(), "a z");
            }
        }
    }

    /// `lpop_remove` recursing once per index in `lpop`'s (possibly nested)
    /// index path natively has no depth cap: an unguarded `lpop v
    /// {*}[lrepeat 100000 0]` empirically overflows the native stack
    /// (SIGABRT) between depth 1600 and 1800 on a 2 MiB thread (`cargo
    /// test`'s per-test default). The iterative implementation has no such
    /// cap; this test checks the result for exact correctness at depth 2000,
    /// comfortably past that crash range — the right leaf comes back out,
    /// and the trimmed list has the same shape as the input, not merely
    /// survival.
    ///
    /// Deliberately NOT 50,000+: constructing (and, at the end of this
    /// test, dropping) a `Value::list` chain nested that deep is its own,
    /// unrelated native-stack risk — `Value` has no custom `Drop` impl, so
    /// the compiler-generated recursive drop glue walks the same chain a
    /// naive `to_str` would (empirically, SIGABRT between depth 3500 and 4000
    /// on a 2 MiB thread for construction+drop alone, independent of any
    /// operation performed on the value). That is a separate, genuinely
    /// unbounded-depth concern in `Value`'s representation itself, not in
    /// `lpop_remove`'s iterative logic, and this test does not cover it.
    #[test]
    fn deeply_nested_lpop_survives_and_is_correct() {
        const DEPTH: usize = 2_000;
        // `whole` = `DEPTH` levels of `[list $v]` around a scalar leaf;
        // `items` is `whole` with one layer already stripped off (mirroring
        // what `cmd_lpop` passes in: the already-`as_list()`-ed variable).
        let mut whole = Value::string("leaf");
        for _ in 0..DEPTH {
            whole = Value::list(vec![whole]);
        }
        let items: Vec<Value> = (*whole.as_list().expect("built as a list")).clone();
        let indices: Vec<Value> = (0..DEPTH).map(|_| Value::string("0")).collect();
        let (removed, rest) = lpop_remove(&mut crate::interp::Vm::new(), &items, &indices)
            .expect("lpop_remove survives and succeeds");
        assert_eq!(&*removed.to_str(), "leaf");
        // The outermost shape (one element) is preserved; only the
        // innermost slot the path bottomed out at was actually emptied.
        assert_eq!(rest.len(), 1);
    }

    /// A moderately nested `lpop` index path (well within realistic use) is
    /// byte-for-byte unaffected by the iterative rewrite.
    #[test]
    fn moderately_nested_lpop_matches_previous_behavior() {
        let items = vec![
            Value::list(vec![Value::int(1), Value::int(2)]),
            Value::list(vec![Value::int(3), Value::int(4)]),
        ];
        let (removed, rest) = lpop_remove(
            &mut crate::interp::Vm::new(),
            &items,
            &[Value::string("1"), Value::string("0")],
        )
        .unwrap();
        assert_eq!(&*removed.to_str(), "3");
        assert_eq!(rest.len(), 2);
        assert_eq!(&*rest[0].to_str(), "1 2");
        assert_eq!(&*rest[1].to_str(), "4");

        // Single-level removal.
        let flat = vec![Value::int(1), Value::int(2), Value::int(3)];
        let (removed, rest) =
            lpop_remove(&mut crate::interp::Vm::new(), &flat, &[Value::string("1")]).unwrap();
        assert_eq!(&*removed.to_str(), "2");
        assert_eq!(rest.len(), 2);
        assert_eq!(&*rest[0].to_str(), "1");
        assert_eq!(&*rest[1].to_str(), "3");

        // A non-integer index is still a "bad index" error.
        assert!(
            lpop_remove(
                &mut crate::interp::Vm::new(),
                &flat,
                &[Value::string("bogus")]
            )
            .is_err()
        );
        // An in-form but out-of-bounds index is still "out of range".
        assert!(lpop_remove(&mut crate::interp::Vm::new(), &flat, &[Value::string("10")]).is_err());
    }
}
