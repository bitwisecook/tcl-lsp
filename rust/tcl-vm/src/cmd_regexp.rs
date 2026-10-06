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

//! `regexp` and `regsub` over shared original-object command plumbing.
//! Actual C recipes retain compiled ARE artifacts, native Unicode subjects,
//! original ranges and per-match write callbacks. The VM supplies object
//! ownership and completion publication; compatibility byte APIs remain
//! separate from native cache authority.

use tcl_cmd_core::regex::{self as core_re, RegexEngine, RegexFlags, RegexpResult, RegsubError};
use tcl_dialect::TclVersion;
use tcl_runtime_api::{Code, Completion};

use crate::interp::{Vm, ok};
use crate::value::Value;

/// The `errorInfo` frame C appends when a `regsub -command` prefix fails
/// (`Tcl_RegsubObjCmd`'s `Tcl_AppendObjToErrorInfo`). tclsh 9.0.4 / 9.1b0,
/// `regsub -command {.x.} {abcxdef} error`:
///
/// ```text
/// cxd
///     while executing
/// "error cxd"
///     (-command substitution computation script)
///     invoked from within
/// "regsub -command {.x.} {abcxdef} error"
/// ```
const COMMAND_SUBST_FRAME: &str = "\n    (-command substitution computation script)";

/// The ARE engine as the shared plumbing's provider. Reused by `lsearch
/// -regexp` (`cmd_list`) and `switch -regexp` (`cmd_switch`).
pub(crate) use tcl_regex::cmd_core::AreEngine as CrateEngine;

/// Does `pattern` match anywhere in `subject` (ARE under `version`, optional
/// `-nocase`)? A small boolean helper for the bytecode `MatchesRegex`-style
/// opcode in `exec`. A compile failure is the engine's bare detail; the caller
/// adds whatever prefix its C counterpart reports.
pub(crate) fn regexp_matches(
    pattern: &str,
    subject: &str,
    nocase: bool,
    version: TclVersion,
) -> Result<bool, String> {
    let flags = RegexFlags {
        nocase,
        ..RegexFlags::for_release(version)
    };
    let mut re = CrateEngine::compile(pattern.as_bytes(), flags)
        .map_err(|e| String::from_utf8_lossy(&e).into_owned())?;
    let cps: Vec<i32> = subject.chars().map(|c| c as i32).collect();
    Ok(CrateEngine::exec(&mut re, &cps, 0, false).is_some())
}

fn regex_completion(vm: &mut Vm, error: core_re::RegexError) -> Completion<Value> {
    crate::command::completion_from_cmd_error(vm, error.into_cmd_error())
}

pub(crate) fn register(vm: &mut Vm) {
    vm.register_stock_builtin("regexp", cmd_regexp);
    vm.register_stock_builtin("regsub", cmd_regsub);
}

/// Jim match operators/list commands invoke the actual command on original argv.
pub(crate) fn invoke_jim_match_command(
    vm: &mut Vm,
    head: &Value,
    pattern: &Value,
    subject: &Value,
    nocase: bool,
    option_end: bool,
) -> Result<i64, Completion<Value>> {
    use tcl_syntax::value::ValueOps;
    let mut arguments = Vec::new();
    if nocase {
        arguments.push(
            Value::new_native_string_bytes(b"-nocase".as_slice()).into_native_unowned_lifetime(),
        );
    }
    if option_end {
        arguments
            .push(Value::new_native_string_bytes(b"--".as_slice()).into_native_unowned_lifetime());
    }
    arguments.push(pattern.native_lifetime_lease().into_value());
    arguments.push(subject.native_lifetime_lease().into_value());
    let completion = vm.invoke_host_original_object_vector(head, &arguments);
    if completion.code != Code::Ok {
        return Err(completion);
    }
    match vm.as_int(&completion.result) {
        Ok(value) => Ok(value),
        Err(error) if error.native_access_refusal().is_some() => {
            Err(crate::command::completion_from_cmd_error(vm, error.into()))
        }
        Err(error) => {
            let completion = crate::command::completion_from_cmd_error(vm, error.into());
            vm.publish_native_interp_completion(completion)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
            Ok(0)
        }
    }
}

pub(crate) fn invoke_jim_regexp(
    vm: &mut Vm,
    pattern: &Value,
    subject: &Value,
    nocase: bool,
) -> Result<bool, Completion<Value>> {
    let head = Value::new_native_string_bytes(b"regexp".as_slice()).into_native_unowned_lifetime();
    invoke_jim_match_command(vm, &head, pattern, subject, nocase, true).map(|eq| eq != 0)
}

fn cmd_regexp(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let version = vm.runtime_version();
    match core_re::regexp_original::<Vm, CrateEngine, Completion<Value>>(
        vm,
        args,
        version,
        |vm, name, value| vm.store_original_regex_variable(name, value).map(|_| ()),
    ) {
        Ok(RegexpResult::Inline(v)) => ok(v),
        Ok(RegexpResult::Count { count, .. }) => ok(Value::int(count)),
        Err(core_re::RegexpExecutionError::Regex(error)) => regex_completion(vm, error),
        Err(core_re::RegexpExecutionError::Assignment(completion)) => completion,
    }
}

impl core_re::NativeRegexSource for Vm {
    fn jim_regex_recipe(
        &self,
    ) -> Result<Option<tcl_syntax::native_regex::JimRegexpRecipe>, tcl_syntax::value::ValueError>
    {
        Ok(self
            .actual_native_invocation_dialect()
            .native_jim_regex_protocol())
    }

    fn jim_regex_option(
        &mut self,
        original: &Value,
        words: &'static [&'static str],
    ) -> Result<usize, tcl_cmd_core::CmdError> {
        let table =
            tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(words);
        match self.native_jim_enum_from_original(
            original,
            &table,
            tcl_registry::native_jim_enum::NativeJimEnumFlags::options(false),
            Some(b"option"),
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

    fn jim_regex_index(&mut self, original: &Value) -> Result<i32, tcl_cmd_core::CmdError> {
        use tcl_syntax::value::ValueOps;
        if let Some(value) = original.native_integer_primary()? {
            return Ok(tcl_syntax::native_jim_index::JimIndex::from_integer(value));
        }
        if let Some(index) = original.native_jim_index()? {
            return Ok(index);
        }
        let bytes = self.native_string_bytes(original)?;
        let index = tcl_syntax::native_jim_index::evaluate_index(&bytes, |input| match input {
            tcl_syntax::native_jim_index::JimIndexExpression::Original => {
                // SetIndexFromAny pins the SAME expression during GetWideExpr.
                let expression = original.clone();
                self.native_jim_wide_expression(&expression)
            }
            tcl_syntax::native_jim_index::JimIndexExpression::EndSuffix(source) => {
                let suffix = Value::new_native_string_bytes(source);
                self.native_jim_wide_expression(&suffix)
            }
        })
        .map_err(|error| {
            let error = match error {
                tcl_syntax::native_jim_index::JimIndexEvaluationError::Index(error) => {
                    tcl_cmd_core::CmdError::from(error)
                }
                tcl_syntax::native_jim_index::JimIndexEvaluationError::Expression(error) => error,
            };
            if error.native_access_refusal().is_some() {
                error
            } else {
                let mut message = b"bad index \"".to_vec();
                message.extend_from_slice(tcl_core_types::c_string_extent(&bytes));
                message.extend_from_slice(b"\": must be intexpr or end?[+-]intexpr?");
                tcl_cmd_core::CmdError::new_bytes(message)
            }
        })?;
        original.install_native_jim_index(index);
        Ok(index.0)
    }

    fn regex_recipe(
        &self,
    ) -> Result<Option<tcl_syntax::native_regex::NativeRegexpRecipe>, tcl_syntax::value::ValueError>
    {
        let dialect = self.actual_native_invocation_dialect();
        if let Some(recipe) = dialect.native_regex_protocol() {
            return Ok(Some(recipe));
        }
        if dialect.native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return Ok(None);
        }
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native regexp engine issuer",
        ))
    }
}
impl core_re::NativeRegexObjects<CrateEngine> for Vm {
    fn regex_cached_jim_pattern(
        &self,
        original: &Value,
        flags: u32,
    ) -> Result<
        Option<tcl_syntax::native_regex::JimRegexpArtifact<tcl_regex::cmd_core::CompiledRegex>>,
        tcl_syntax::value::ValueError,
    > {
        original.native_jim_regexp_cache(flags)
    }
    fn regex_install_jim_pattern(
        &mut self,
        original: &Value,
        flags: u32,
        program: tcl_regex::cmd_core::CompiledRegex,
    ) -> Result<
        tcl_syntax::native_regex::JimRegexpArtifact<tcl_regex::cmd_core::CompiledRegex>,
        tcl_syntax::value::ValueError,
    > {
        original.install_native_jim_regexp(flags, program)
    }

    fn regex_jim_range(
        &mut self,
        bytes: &[u8],
        matched: bool,
    ) -> Result<Value, tcl_syntax::value::ValueError> {
        use tcl_cmd_core::native_append::NativeAppendObjects;
        let protocol = tcl_syntax::native_string::NativeStringProtocol::Jim084;
        let value = Value::new_native_string_bytes(bytes.to_vec());
        if matched {
            crate::value::VmAppendObjects.set_string(
                &value,
                protocol,
                Some((
                    std::rc::Rc::from(bytes),
                    tcl_syntax::native_string::NativeStringStorageIdentity::Allocated,
                )),
                None,
                None,
            )?;
        }
        Ok(value)
    }

    fn regex_cached_pattern(
        &self,
        original: &Value,
        recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        flags: u32,
    ) -> Result<
        Option<std::rc::Rc<std::cell::RefCell<tcl_regex::cmd_core::CompiledRegex>>>,
        tcl_syntax::value::ValueError,
    > {
        original.native_regexp_cache(recipe, flags)
    }
    fn regex_cached_glob(
        &self,
        original: &Value,
        recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        flags: u32,
    ) -> Result<Option<std::rc::Rc<[u8]>>, tcl_syntax::value::ValueError> {
        original.native_regexp_glob(recipe, flags)
    }
    fn regex_install_pattern(
        &mut self,
        original: &Value,
        recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        flags: u32,
        compiled: std::rc::Rc<std::cell::RefCell<tcl_regex::cmd_core::CompiledRegex>>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        original.install_native_regexp(recipe, flags, compiled)
    }
    fn regex_range_value(
        &mut self,
        range: tcl_syntax::native_regex::NativeRegexpRange,
    ) -> Result<Value, tcl_syntax::value::ValueError> {
        use tcl_cmd_core::native_append::NativeAppendObjects;
        use tcl_syntax::native_regex::NativeRegexpRange;
        let dialect = self.actual_native_invocation_dialect();
        let protocol = dialect.native_string_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("native regexp range issuer"),
        )?;
        match range {
            NativeRegexpRange::Empty => Ok(Value::new_native_string_bytes(b"".as_slice())),
            NativeRegexpRange::Unicode(units) => {
                Value::from_native_unicode_units(std::rc::Rc::from(units), dialect)
            }
            NativeRegexpRange::Bytes { bytes, count } => {
                let bytes: std::rc::Rc<[u8]> = std::rc::Rc::from(bytes);
                let value = Value::new_native_string_bytes(std::rc::Rc::clone(&bytes));
                crate::value::VmAppendObjects.set_string(
                    &value,
                    protocol,
                    Some((
                        bytes,
                        tcl_syntax::native_string::NativeStringStorageIdentity::Allocated,
                    )),
                    Some(count),
                    None,
                )?;
                Ok(value)
            }
        }
    }
}

impl core_re::NativeRegsubObjects for Vm {
    type Object = Value;
    type Error = Completion<Value>;
    fn regex_jim_bytes(
        &mut self,
        bytes: &[u8],
        string_primary: bool,
    ) -> Result<Value, tcl_syntax::value::ValueError> {
        <Self as core_re::NativeRegexObjects<CrateEngine>>::regex_jim_range(
            self,
            bytes,
            string_primary,
        )
    }
    fn regex_result_bytes(&mut self) -> Result<std::rc::Rc<[u8]>, tcl_syntax::value::ValueError> {
        use tcl_syntax::value::ValueOps;
        let current =
            self.with_native_interp_result(|value| value.native_lifetime_lease().into_value())?;
        self.native_string_bytes(&current)
    }
    fn regex_object<'a>(&self, value: &'a Value) -> &'a Value {
        value
    }
    fn regex_borrow(&self, value: &Value) -> Value {
        value.native_lifetime_lease().into_value()
    }
    fn regex_duplicate(&mut self, value: &Value) -> Result<Value, tcl_syntax::value::ValueError> {
        value.check_native_header()?;
        let protocol = self
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "actual regsub duplicate",
            ))?;
        Ok(value.duplicate_native_object_in(protocol))
    }
    fn regex_members(
        &mut self,
        value: &Value,
    ) -> Result<Vec<Value>, tcl_syntax::value::ValueError> {
        let protocol = self
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "actual regsub prefix List",
            ))?;
        self.native_object_list_elements_in(value, protocol)
            .map(|members| {
                members
                    .iter()
                    .map(|member| member.native_lifetime_lease().into_value())
                    .collect()
            })
    }
    fn regex_unicode(&mut self, units: &[u32]) -> Result<Value, tcl_syntax::value::ValueError> {
        Value::from_native_unicode_units(
            std::rc::Rc::from(units),
            self.actual_native_invocation_dialect(),
        )
    }
    fn regex_append_unicode(
        &mut self,
        result: &Value,
        units: &[u32],
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let protocol = self
            .actual_native_invocation_dialect()
            .native_object_append_protocol(None)
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "actual regsub Unicode append",
            ))?;
        tcl_cmd_core::native_append::append_unicode_units(
            &crate::value::VmAppendObjects,
            protocol.recipe(),
            result,
            units,
        )
    }
    fn regex_append_current_result(
        &mut self,
        result: &mut Value,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let protocol = self
            .actual_native_invocation_dialect()
            .native_object_append_protocol(None)
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "actual regsub object append",
            ))?;
        let current =
            self.with_native_interp_result(|value| value.native_lifetime_lease().into_value())?;
        *result = tcl_cmd_core::native_append::append_object(
            &crate::value::VmAppendObjects,
            protocol.recipe(),
            Some(result),
            &current,
        )?
        .into_value();
        Ok(())
    }
    fn regex_eval(&mut self, prefix: &Value, arguments: &[Value]) -> Result<(), Completion<Value>> {
        let mut call = self
            .regex_members(prefix)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
        call.extend(
            arguments
                .iter()
                .map(|value| value.native_lifetime_lease().into_value()),
        );
        let (head, rest) = call.split_first().expect("validated prefix");
        let completion = self.invoke_command_value_at(
            self.current_ns_id(),
            head,
            rest,
            &[],
            tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
        );
        if completion.code == Code::Ok {
            Ok(())
        } else {
            Err(completion)
        }
    }
    fn regex_reset_result(&mut self) -> Result<(), tcl_syntax::value::ValueError> {
        self.reset_native_jim_result()
    }
}

fn cmd_regsub(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let version = vm.runtime_version();
    if vm
        .actual_native_invocation_dialect()
        .native_jim_regex_protocol()
        .is_some()
    {
        return match core_re::regsub_jim_original::<Vm, CrateEngine>(vm, args, version) {
            Ok(result) => {
                if let Some(target) = result.target {
                    match vm.store_original_regex_variable(target, result.result) {
                        Ok(_) => ok(Value::int(result.count)),
                        Err(completion) => completion,
                    }
                } else {
                    ok(result.result)
                }
            }
            Err(RegsubError::Regex(error)) => regex_completion(vm, error),
            Err(RegsubError::Eval(completion)) => completion,
        };
    }
    let prepared = match core_re::regsub_prepare_original(vm, args, version) {
        Ok(prepared) => prepared,
        Err(error) => return regex_completion(vm, error),
    };
    if prepared.is_native() {
        return match core_re::regsub_native_original::<Vm, CrateEngine>(vm, &prepared) {
            Ok(result) => {
                if let Some(target) = result.target {
                    if let Err(completion) = vm.store_original_regex_variable(target, result.result)
                    {
                        return completion;
                    }
                    ok(Value::int(result.count))
                } else {
                    ok(result.result)
                }
            }
            Err(RegsubError::Regex(error)) => regex_completion(vm, error),
            Err(RegsubError::Eval(completion)) => {
                if completion.code == Code::Error {
                    vm.seed_error_info_frame(completion.result.string_bytes(), COMMAND_SUBST_FRAME);
                }
                completion
            }
        };
    }
    let outcome =
        core_re::regsub_eval_original::<CrateEngine, Completion<Value>, _>(&prepared, |words| {
            let _ = words;
            Err(vm.refuse_host_command("native regsub command plan was not selected".into()))
        });
    let core_re::OriginalRegsubResult {
        text,
        count,
        target,
    } = match outcome {
        Ok(r) => r,
        Err(RegsubError::Regex(error)) => return regex_completion(vm, error),
        Err(RegsubError::Eval(completion)) => {
            // C adds the context frame only for a genuine error; a
            // `break`/`continue`/custom code from the prefix propagates
            // untouched (tclsh 9.0.4: `proc q args {return -code continue}`,
            // `catch {regsub -command {.x.} abcxdef q}` → 4, `::errorInfo`
            // never set).
            if completion.code == Code::Error {
                vm.seed_error_info_frame(completion.result.string_bytes(), COMMAND_SUBST_FRAME);
            }
            return completion;
        }
    };
    let result = Value::from_string_bytes(text);
    match target {
        Some(name) => {
            if let Err(c) = vm.store_original_regex_variable(name, result) {
                return c;
            }
            ok(Value::int(count))
        }
        None => ok(result),
    }
}

#[cfg(test)]
mod original_object_tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    use tcl_runtime_api::native_variable_trace::{
        NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
    };
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;

    fn actual(environment: &str) -> Vm {
        let profile = tcl_registry::model::ingress::resolve_environment(environment).unit_profile();
        crate::native_fixture::core(profile)
    }

    #[test]
    fn jim_original_start_indices_use_actual_safe_expression_owner_for_all_17_native_controls() {
        use core_re::NativeRegexSource;
        let rows = include_str!("../../tcl-syntax/tests/data/native_jim_index/rows.txt");
        let mut cases = 0;
        for record in rows.split("case\t").skip(1) {
            let lines: Vec<_> = record.lines().collect();
            let (case, source) = lines[0].split_once('\t').unwrap();
            let source: Vec<u8> = source
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            let code = lines[1].split_once('\t').unwrap().1.parse::<i32>().unwrap();
            let index = lines[2].split_once('\t').unwrap().1.parse::<i32>().unwrap();
            let expected_kind = lines[3].split('\t').nth(1).unwrap();
            let expected_physical: Vec<_> = lines
                .iter()
                .find_map(|line| line.strip_prefix("result_object\t"))
                .unwrap()
                .split('\t')
                .collect();
            let expected_result: Vec<u8> = lines
                .iter()
                .find_map(|line| line.strip_prefix("result\t"))
                .unwrap()
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            let mut ops = actual("jim");
            let original = Value::new_native_string_bytes(source);
            let result = ops.jim_regex_index(&original);
            match result {
                Ok(actual) => {
                    assert_eq!(code, 0, "case {case}");
                    assert_eq!(actual, index, "case {case}");
                }
                Err(error) => {
                    assert_eq!(code, 1, "case {case}: {error:?}");
                    assert!(
                        error.native_access_refusal().is_none(),
                        "case {case}: {error:?}"
                    );
                    assert_eq!(error.message_bytes(), expected_result, "case {case}");
                    let completion = crate::command::completion_from_cmd_error(&mut ops, error);
                    ops.publish_native_interp_completion(completion).unwrap();
                }
            }
            assert_eq!(
                original.native_object_type_name(),
                expected_kind,
                "case {case}"
            );
            assert!(original.resident_string_bytes().is_some(), "case {case}");
            assert_eq!(original.native_object_reference_count(), 1, "case {case}");
            ops.with_native_interp_result(|result| {
                assert_eq!(
                    result.native_object_type_name(),
                    expected_physical[0],
                    "case {case} result primary"
                );
                assert_eq!(
                    result.resident_string_bytes().is_some(),
                    expected_physical[1] == "1",
                    "case {case} result String presence"
                );
                assert_eq!(
                    result.native_object_reference_count(),
                    expected_physical[2].parse::<usize>().unwrap(),
                    "case {case} result references"
                );
            })
            .unwrap();
            cases += 1;
        }
        assert_eq!(cases, 17);
    }

    #[test]
    fn jim_original_regexp_cache_ranges_and_unsafe_lifecycles_match_native_controls() {
        use core_re::NativeRegexObjects;
        let mut vm = actual("jim");
        for bytes in [b"a".as_slice(), b"\xff", b"\xc0\x80", b"\xf0\x9f\x98\x80"] {
            let pattern = Value::new_native_string_bytes(bytes);
            let subject = Value::new_native_string_bytes(bytes);
            let name = Value::new_native_string_bytes(b"m".as_slice());
            let outcome = cmd_regexp(
                &mut vm,
                &[
                    pattern.native_lifetime_lease().into_value(),
                    subject.native_lifetime_lease().into_value(),
                    name,
                ],
            );
            assert_eq!(outcome.code, Code::Ok);
            assert_eq!(pattern.native_object_type_name(), "regexp");
            assert_eq!(subject.native_object_type_name(), "none");
            assert_eq!(outcome.result.native_object_type_name(), "int");
            assert!(outcome.result.resident_string_bytes().is_none());
            let range = vm.get_var_bytes(b"m").unwrap();
            assert_eq!(range.native_object_type_name(), "string");
            assert_eq!(range.resident_string_bytes().unwrap().as_ref(), bytes);
            let artifact = vm.regex_cached_jim_pattern(&pattern, 0).unwrap().unwrap();
            pattern.invalidate_native_string_for_test();
            assert!(
                artifact.same_program(&vm.regex_cached_jim_pattern(&pattern, 0).unwrap().unwrap())
            );
            assert!(
                core_re::prepare_pattern_original::<Vm, CrateEngine>(
                    &mut vm,
                    &pattern,
                    RegexFlags::for_release(TclVersion::V9_0),
                    TclVersion::V9_0
                )
                .is_err()
            );
        }
        let pattern = Value::new_native_string_bytes(b"a".as_slice());
        core_re::prepare_pattern_original::<Vm, CrateEngine>(
            &mut vm,
            &pattern,
            RegexFlags::for_release(TclVersion::V9_0),
            TclVersion::V9_0,
        )
        .unwrap();
        let copy = pattern
            .duplicate_native_object_in(tcl_syntax::native_string::NativeStringProtocol::Jim084);
        assert!(vm.regex_cached_jim_pattern(&copy, 0).unwrap().is_some());
        drop(copy);
        assert!(vm.regex_cached_jim_pattern(&pattern, 0).is_err());
    }
    #[test]
    fn jim_regsub_original_objects_match_all_nine_completed_native_controls() {
        let rows =
            include_str!("../../../runtime/rust/tests/data/native_regexp_jim_regsub/rows.txt");
        let patterns: [&[u8]; 9] = [
            b"a",
            b"\xff",
            b"\xc0\x80",
            b"\xf0\x9f\x98\x80",
            b"z",
            b"(a)",
            b"",
            b"a",
            b"a",
        ];
        let mut case = 0usize;
        let mut expected = Vec::new();
        for line in rows.lines() {
            let fields: Vec<_> = line.split('\t').collect();
            if fields[0] == "case" {
                case = fields[1].parse().unwrap();
            }
            if fields[0] == "bytes" && case < 9 {
                expected.push(
                    fields[1]
                        .as_bytes()
                        .chunks_exact(2)
                        .map(|pair| {
                            u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap()
                        })
                        .collect::<Vec<_>>(),
                );
            }
        }
        assert_eq!(expected.len(), 9);
        for case in 0..9 {
            let subject = if (1..=3).contains(&case) {
                patterns[case]
            } else {
                b"aba"
            };
            let replacement: &[u8] = if case == 5 {
                b"\\1&".as_slice()
            } else if case == 8 {
                b"string toupper"
            } else {
                b"X"
            };
            let mut vm = actual("jim");
            let pattern = Value::new_native_string_bytes(patterns[case]);
            let subject = Value::new_native_string_bytes(subject);
            let replacement = Value::new_native_string_bytes(replacement);
            let mut args = vec![Value::new_native_string_bytes(if case == 8 {
                b"-command".as_slice()
            } else {
                b"-all"
            })];
            if case == 7 {
                args.push(Value::new_native_string_bytes(b"-start".as_slice()));
                args.push(Value::new_native_string_bytes(b"end-1".as_slice()));
            }
            args.extend([
                pattern.native_lifetime_lease().into_value(),
                subject.native_lifetime_lease().into_value(),
                replacement.native_lifetime_lease().into_value(),
            ]);
            let outcome = cmd_regsub(&mut vm, &args);
            assert_eq!(outcome.code, Code::Ok, "native case {case}");
            assert_eq!(outcome.result.native_object_type_name(), "string");
            assert_eq!(
                outcome.result.resident_string_bytes().unwrap().as_ref(),
                expected[case]
            );
            assert_eq!(pattern.native_object_type_name(), "none");
            assert_eq!(subject.native_object_type_name(), "none");
            assert_eq!(
                replacement.native_object_type_name(),
                if case == 8 { "list" } else { "none" }
            );
            assert_eq!(pattern.native_object_reference_count(), 1);
            assert_eq!(subject.native_object_reference_count(), 1);
            assert_eq!(replacement.native_object_reference_count(), 1);
        }
    }

    #[test]
    fn jim_regsub_duplicate_and_match_consumers_preserve_original_dispatch() {
        let mut vm = actual("jim");
        let args = [
            Value::new_native_string_bytes(b"-all".as_slice()),
            Value::new_native_string_bytes(b"a".as_slice()),
            Value::new_native_string_bytes(b"aba".as_slice()),
            Value::new_native_string_bytes(b"X".as_slice()),
        ];
        let result = cmd_regsub(&mut vm, &args);
        assert_eq!(result.code, Code::Ok);
        assert_eq!(result.result.native_object_type_name(), "string");
        assert_eq!(
            result.result.resident_string_bytes().unwrap().as_ref(),
            b"XbX"
        );
        assert_eq!(args[1].native_object_type_name(), "none");
        vm.register("regexp", |_, _| ok(Value::int(1)));
        assert!(
            invoke_jim_regexp(
                &mut vm,
                &Value::new_native_string_bytes(b"[".as_slice()),
                &Value::new_native_string_bytes(b"anything".as_slice()),
                false
            )
            .unwrap()
        );
    }

    #[test]
    fn native_boolean_regexp_glob_preserves_subject_storage_and_cached_pattern() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut ops = actual(environment);
            let recipe = ops
                .actual_native_invocation_dialect()
                .native_regex_protocol()
                .unwrap();
            let pattern = Value::new_native_string_bytes(b"\xff".as_slice());
            let subject = Value::new_native_string_bytes(b"\xff".as_slice());
            let mut flags = RegexFlags::for_release(recipe.version());
            flags.nosub = true;
            let mut prepared = core_re::prepare_search_pattern_original::<Vm, CrateEngine>(
                &mut ops,
                &pattern,
                flags,
                recipe.version(),
            )
            .unwrap();
            assert!(
                core_re::match_pattern_original::<Vm, CrateEngine>(
                    &mut ops,
                    &mut prepared,
                    &subject
                )
                .unwrap()
            );
            assert_eq!(
                subject.native_object_type_name(),
                if recipe.version() == TclVersion::V8_5 {
                    "none"
                } else {
                    "string"
                },
                "{environment}"
            );
            let compiled = pattern
                .native_regexp_cache(recipe, flags.cache_key())
                .unwrap()
                .unwrap();
            pattern.invalidate_native_string_for_test();
            let mut cached = core_re::prepare_search_pattern_original::<Vm, CrateEngine>(
                &mut ops,
                &pattern,
                flags,
                recipe.version(),
            )
            .unwrap();
            assert!(
                core_re::match_pattern_original::<Vm, CrateEngine>(&mut ops, &mut cached, &subject)
                    .unwrap()
            );
            assert!(std::rc::Rc::ptr_eq(
                &compiled,
                &pattern
                    .native_regexp_cache(recipe, flags.cache_key())
                    .unwrap()
                    .unwrap()
            ));
        }
    }

    #[test]
    fn native_regexp_cache_hit_precedes_getter_and_duplicate_keeps_artifact() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = actual(environment);
            let recipe = vm
                .actual_native_invocation_dialect()
                .native_regex_protocol()
                .unwrap();
            let flags = RegexFlags::for_release(recipe.version());
            let pattern = Value::new_native_string_bytes(b"a".as_slice());
            let subject = Value::new_native_string_bytes(b"a".as_slice());
            assert!(
                core_re::compiled_match_original::<Vm, CrateEngine>(
                    &mut vm,
                    &pattern,
                    &subject,
                    flags,
                    recipe.version()
                )
                .unwrap()
            );
            assert_eq!(pattern.native_object_type_name(), "regexp");
            let compiled = pattern
                .native_regexp_cache(recipe, flags.cache_key())
                .unwrap()
                .unwrap();
            pattern.invalidate_native_string_for_test();
            assert!(
                core_re::compiled_match_original::<Vm, CrateEngine>(
                    &mut vm,
                    &pattern,
                    &subject,
                    flags,
                    recipe.version()
                )
                .unwrap()
            );
            assert!(pattern.resident_string_bytes().is_none());
            assert!(Rc::ptr_eq(
                &compiled,
                &pattern
                    .native_regexp_cache(recipe, flags.cache_key())
                    .unwrap()
                    .unwrap()
            ));
            let duplicate = pattern.duplicate_native_object_in(
                tcl_syntax::native_string::NativeStringProtocol::C(recipe.version()),
            );
            assert!(Rc::ptr_eq(
                &compiled,
                &duplicate
                    .native_regexp_cache(recipe, flags.cache_key())
                    .unwrap()
                    .unwrap()
            ));
            assert!(duplicate.resident_string_bytes().is_none());
            let changed = RegexFlags {
                nocase: true,
                ..flags
            };
            // The corresponding native missing-updater attempt aborts. It
            // cannot license a fabricated pattern or successful completion.
            assert!(
                core_re::compiled_match_original::<Vm, CrateEngine>(
                    &mut vm,
                    &pattern,
                    &subject,
                    changed,
                    recipe.version()
                )
                .is_err()
            );
            assert!(pattern.resident_string_bytes().is_none());
            let invalid = Value::new_native_string_bytes(b"(".as_slice());
            assert!(
                core_re::compiled_match_original::<Vm, CrateEngine>(
                    &mut vm,
                    &invalid,
                    &subject,
                    flags,
                    recipe.version()
                )
                .is_err()
            );
            assert_eq!(invalid.native_object_type_name(), "none");
            assert_eq!(invalid.resident_string_bytes().unwrap().as_ref(), b"(");
        }
    }

    #[test]
    fn native_regexp_ranges_preserve_selected_original_units_and_storage() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for input in [b"a".as_slice(), b"\xff", b"\xc0\x80", b"\xf0\x9f\x98\x80"] {
                let mut vm = actual(environment);
                let args = [
                    Value::new_native_string_bytes(input),
                    Value::new_native_string_bytes(input),
                    Value::new_native_string_bytes(b"m".as_slice()),
                ];
                assert_eq!(
                    cmd_regexp(&mut vm, &args).code,
                    Code::Ok,
                    "{environment}/{input:?}"
                );
                assert_eq!(args[0].native_object_type_name(), "regexp");
                assert_eq!(args[1].native_object_type_name(), "string");
                let result = vm.get_var("m").unwrap();
                assert_eq!(result.native_object_type_name(), "string");
                let older = matches!(environment, "tcl8.4" | "tcl8.5");
                assert_eq!(
                    result.resident_string_bytes().is_some(),
                    older && input != b"\xc0\x80",
                    "{environment}/{input:?}"
                );
                let expected = if input == b"\xff" && !older {
                    b"\xc3\xbf".as_slice()
                } else if input == b"\xf0\x9f\x98\x80" && environment == "tcl8.6" {
                    b"\xed\xa0\xbd\xed\xb8\x80".as_slice()
                } else {
                    input
                };
                assert_eq!(
                    vm.native_name_operand_bytes(&result).unwrap().as_ref(),
                    expected,
                    "{environment}/{input:?}"
                );
            }
        }
    }

    #[test]
    fn native_regsub_mapping_and_regexp_keep_distinct_original_primaries() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for (pattern, replacement, expected, mapping, same) in [
                (
                    b"(a)".as_slice(),
                    b"\\1X".as_slice(),
                    b"aXb".as_slice(),
                    false,
                    false,
                ),
                (b"a", b"X", b"Xb", true, false),
                (b"z", b"X", b"ab", true, true),
            ] {
                let mut vm = actual(environment);
                let args = [
                    Value::new_native_string_bytes(b"-all".as_slice()),
                    Value::new_native_string_bytes(pattern),
                    Value::new_native_string_bytes(b"ab".as_slice()),
                    Value::new_native_string_bytes(replacement),
                ];
                let outcome = cmd_regsub(&mut vm, &args);
                assert_eq!(outcome.code, Code::Ok, "{environment}");
                assert_eq!(
                    args[1].native_object_type_name(),
                    if mapping { "string" } else { "regexp" }
                );
                assert_eq!(args[2].native_object_type_name(), "string");
                assert_eq!(args[3].native_object_type_name(), "string");
                assert_eq!(
                    outcome.result.native_object_identity() == args[2].native_object_identity(),
                    same
                );
                assert_eq!(
                    outcome.result.resident_string_bytes().is_some(),
                    same || matches!(environment, "tcl8.4" | "tcl8.5")
                );
                assert_eq!(outcome.result.native_object_type_name(), "string");
                assert_eq!(
                    vm.native_name_operand_bytes(&outcome.result)
                        .unwrap()
                        .as_ref(),
                    expected
                );
            }
        }
    }

    struct RecordWrites {
        rows: Rc<RefCell<Vec<Vec<u8>>>>,
        fail_second: bool,
    }
    impl NativeVariableObserver<Vm> for RecordWrites {
        type Error = tcl_cmd_core::CmdError;
        fn observe(
            &self,
            vm: &mut Vm,
            access: NativeVariableTraceAccess<'_>,
        ) -> Result<(), Self::Error> {
            let value = vm.get_var_bytes(access.name1).expect("reached write cell");
            self.rows.borrow_mut().push(
                vm.native_name_operand_bytes(&value)
                    .expect("original callback value getter")
                    .to_vec(),
            );
            if self.fail_second && self.rows.borrow().len() == 2 {
                Err(tcl_cmd_core::CmdError::new("STOP"))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn original_regexp_each_match_and_first_failure_match_native_controls() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for fail_second in [false, true] {
                // The C84 scripted failure completion aborts natively. Its
                // successful callback sequence is a separate valid control.
                if environment == "tcl8.4" && fail_second {
                    continue;
                }
                let mut vm = actual(environment);
                let name = Value::new_native_string_bytes(b"m".as_slice());
                let rows = Rc::new(RefCell::new(Vec::new()));
                vm.add_native_variable_observer(
                    &name,
                    &[NativeVariableTraceOperation::Write],
                    Rc::new(RecordWrites {
                        rows: Rc::clone(&rows),
                        fail_second,
                    }),
                )
                .unwrap();
                let args = [b"-all".as_slice(), b"(.)", b"abc", b"m", b"s"]
                    .map(Value::new_native_string_bytes);
                let outcome = cmd_regexp(&mut vm, &args);
                if fail_second {
                    assert_eq!(outcome.code, Code::Error, "{environment}");
                    assert_eq!(&*rows.borrow(), &[b"a".to_vec(), b"b".to_vec()]);
                    assert_eq!(vm.get_var("s").unwrap().string_bytes().as_ref(), b"a");
                    let expected = if environment == "tcl8.5" {
                        b"couldn't set variable \"m\"".as_slice()
                    } else {
                        b"can't set \"m\": STOP"
                    };
                    assert_eq!(
                        outcome.result.string_bytes().as_ref(),
                        expected,
                        "{environment}"
                    );
                } else {
                    assert_eq!(outcome.code, Code::Ok, "{environment}");
                    assert_eq!(
                        &*rows.borrow(),
                        &[b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]
                    );
                    assert_eq!(vm.get_var("s").unwrap().string_bytes().as_ref(), b"c");
                    assert_eq!(outcome.result.string_bytes().as_ref(), b"3");
                }
            }
        }
    }

    #[test]
    fn original_regex_unmatched_target_and_quiet_write_match_native_controls() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = actual(environment);
            let target = Value::list(vec![Value::new_native_string_bytes(
                b"untouched".as_slice(),
            )]);
            let args = [
                Value::new_native_string_bytes(b"z".as_slice()),
                Value::new_native_string_bytes(b"abc".as_slice()),
                target.native_lifetime_lease().into_value(),
            ];
            assert_eq!(cmd_regexp(&mut vm, &args).code, Code::Ok);
            assert!(target.resident_string_bytes().is_none());
            assert!(matches!(
                target.native_object_snapshot().cache,
                Cache::List { .. }
            ));
            assert_eq!(target.native_object_reference_count(), 1);
            vm.set_var("a", Value::new_native_string_bytes(b"scalar".as_slice()))
                .unwrap();
            let args = [b"a".as_slice(), b"a", b"b", b"a(k)"].map(Value::new_native_string_bytes);
            let outcome = cmd_regsub(&mut vm, &args);
            assert_eq!(outcome.code, Code::Error);
            let cache = outcome.result.native_object_snapshot().cache;
            if environment == "tcl8.4" {
                assert!(matches!(cache, Cache::None));
            } else {
                assert!(
                    matches!(cache, Cache::String { .. }),
                    "{environment}: {cache:?}"
                );
            }
            let expected = if matches!(environment, "tcl8.4" | "tcl8.5") {
                b"couldn't set variable \"a(k)\"".as_slice()
            } else {
                b"can't set \"a(k)\": variable isn't array"
            };
            assert_eq!(
                outcome.result.string_bytes().as_ref(),
                expected,
                "{environment}"
            );
        }
    }

    thread_local! {
        static SEED_ID: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
        static OBSERVATIONS: RefCell<Vec<(bool, usize, bool, usize, bool)>> = const { RefCell::new(Vec::new()) };
    }
    fn keep(_vm: &mut Vm, argv: &[Value]) -> Completion<Value> {
        assert_eq!(argv.len(), 3);
        assert!(matches!(
            argv[0].native_object_snapshot().cache,
            Cache::List { .. }
        ));
        assert!(matches!(
            argv[1].native_object_snapshot().cache,
            Cache::String { .. }
        ));
        let row = (
            SEED_ID.with(std::cell::Cell::get) == argv[0].native_object_identity(),
            argv[0].native_object_reference_count(),
            argv[0].resident_string_bytes().is_some(),
            argv[1].native_object_reference_count(),
            argv[1].resident_string_bytes().is_some(),
        );
        OBSERVATIONS.with(|rows| rows.borrow_mut().push(row));
        ok(argv[0].native_lifetime_lease().into_value())
    }

    #[test]
    fn original_regsub_prefix_and_unicode_arguments_match_native_c9_controls() {
        for environment in ["tcl9.0", "tcl9.1"] {
            let mut vm = actual(environment);
            vm.register("keep", keep);
            let seed = Value::list(vec![Value::new_native_string_bytes(b"S".as_slice())]);
            SEED_ID.with(|slot| slot.set(seed.native_object_identity()));
            let prefix = Value::list(vec![
                Value::new_native_string_bytes(b"keep".as_slice()),
                seed,
            ]);
            OBSERVATIONS.with(|rows| rows.borrow_mut().clear());
            let mut argv = [b"-all".as_slice(), b"-command", b"(.)", b"ab"]
                .map(Value::new_native_string_bytes)
                .to_vec();
            argv.push(prefix);
            let outcome = cmd_regsub(&mut vm, &argv);
            assert_eq!(outcome.code, Code::Ok, "{environment}");
            assert_eq!(
                OBSERVATIONS.with(|rows| rows.borrow().clone()),
                [(true, 1, false, 1, false), (true, 1, true, 1, false)],
                "{environment}"
            );
            assert!(matches!(
                outcome.result.native_object_snapshot().cache,
                Cache::String { .. }
            ));
            assert!(outcome.result.resident_string_bytes().is_none());
            assert_eq!(outcome.result.native_object_reference_count(), 1);
            assert_eq!(
                vm.native_name_operand_bytes(&outcome.result)
                    .unwrap()
                    .as_ref(),
                b"SS"
            );
        }
    }
}
