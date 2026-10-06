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
//! original ranges and per-match write callbacks. Runtime supplies physical
//! object headers, variable writes and completion publication.

use crate::interp::{Code, Interp};
use crate::obj::{self, new_string_bytes, new_wide_int_obj, TclObj};
use tcl_cmd_core::regex::{self as core_re, RegexpResult, RegsubError};

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
const COMMAND_SUBST_FRAME: &[u8] = b"-command substitution computation script";

/// The pure-Rust Tcl 9 ARE engine as the shared plumbing's [`RegexEngine`]
/// provider. Reused by `lsearch -regexp` (`cmd_list`) and `switch -regexp`
/// (`cmd_switch`).
pub(crate) use tcl_regex::cmd_core::AreEngine;

/// Register `regexp` and `regsub`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"regexp", regexp_cmd);
    interp.register_builtin(b"regsub", regsub_cmd);
}

/// Jim match consumers call the actual original regexp command, including replacements.
pub(crate) fn invoke_jim_match_command(
    interp: &mut Interp,
    head: &*mut TclObj,
    pattern: &*mut TclObj,
    subject: &*mut TclObj,
    nocase: bool,
    option_end: bool,
) -> Result<i64, Code> {
    use tcl_syntax::value::ValueOps;
    let option = nocase.then(|| obj::new_string_bytes(b"-nocase"));
    let end =
        option_end.then(|| obj::Owned::fresh(obj::new_string_bytes(b"--")).into_native_unowned());
    let mut argv = vec![*head];
    if let Some(option) = &option {
        argv.push(*option);
    }
    if let Some(end) = end {
        argv.push(end);
    }
    argv.extend([*pattern, *subject]);
    let code = interp.eval_original_object_vector(&argv);
    if code != Code::Ok {
        return Err(code);
    }
    match interp.as_int(&interp.get_obj_result()) {
        Ok(value) => Ok(value),
        Err(error) if error.native_access_refusal().is_some() => {
            Err(interp.report_cmd_error(error.into()))
        }
        Err(error) => {
            interp.report_cmd_error(error.into());
            Ok(0)
        }
    }
}

pub(crate) fn invoke_jim_regexp(
    interp: &mut Interp,
    pattern: &*mut TclObj,
    subject: &*mut TclObj,
    nocase: bool,
) -> Result<bool, Code> {
    let head = obj::new_string_bytes(b"regexp");
    invoke_jim_match_command(interp, &head, pattern, subject, nocase, true).map(|eq| eq != 0)
}

fn regexp_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let version = interp.runtime_version();
    let outcome = core_re::regexp_original::<Interp, AreEngine, Code>(
        interp,
        &argv[1..],
        version,
        |interp, name, value| {
            let lifetime = obj::NativeObjectLifetime::retain(value);
            let outcome = interp.assign_original_regex_variable(*name, value);
            if obj::allocation_is_live(value) {
                // SAFETY: the lifetime lease keeps this checked header allocated.
                if unsafe { (*value).ref_count == 0 } {
                    crate::interp::drop_fresh(value);
                }
            }
            drop(lifetime);
            outcome
        },
    );
    match outcome {
        Ok(RegexpResult::Inline(v)) => {
            interp.set_result(v);
            Code::Ok
        }
        Ok(RegexpResult::Count { count, .. }) => {
            set_int(interp, count);
            Code::Ok
        }
        Err(core_re::RegexpExecutionError::Regex(error)) => {
            interp.report_cmd_error(error.into_cmd_error())
        }
        Err(core_re::RegexpExecutionError::Assignment(code)) => code,
    }
}

impl core_re::NativeRegexSource for Interp {
    fn jim_regex_recipe(
        &self,
    ) -> Result<Option<tcl_syntax::native_regex::JimRegexpRecipe>, tcl_syntax::value::ValueError>
    {
        Ok(self.native_invocation_dialect().native_jim_regex_protocol())
    }

    fn jim_regex_option(
        &mut self,
        original: &*mut TclObj,
        words: &'static [&'static str],
    ) -> Result<usize, tcl_cmd_core::CmdError> {
        let table =
            tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(words);
        match self.native_jim_enum_from_original(
            *original,
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

    fn jim_regex_index(&mut self, original: &*mut TclObj) -> Result<i32, tcl_cmd_core::CmdError> {
        use tcl_syntax::value::ValueOps;
        obj::check_native_liveness(*original)?;
        if std::ptr::eq(obj::obj_type_ptr(*original), &obj::TCL_INT_TYPE) {
            return self
                .as_int(original)
                .map(tcl_syntax::native_jim_index::JimIndex::from_integer)
                .map_err(Into::into);
        }
        if let Some(index) = crate::native_regexp::cached_jim_index(*original)? {
            return Ok(index);
        }
        let bytes = self.native_string_bytes(original)?;
        let index = tcl_syntax::native_jim_index::evaluate_index(&bytes, |input| match input {
            tcl_syntax::native_jim_index::JimIndexExpression::Original => {
                // SetIndexFromAny pins the SAME expression during GetWideExpr.
                let expression = obj::Owned::retain(*original);
                crate::builtins::native_jim_wide_expression(self, expression.as_ptr())
            }
            tcl_syntax::native_jim_index::JimIndexExpression::EndSuffix(source) => {
                let suffix = obj::Owned::fresh(obj::new_string_bytes(source));
                crate::builtins::native_jim_wide_expression(self, suffix.as_ptr())
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
        crate::native_regexp::install_jim_index(*original, index);
        Ok(index.0)
    }

    fn regex_recipe(
        &self,
    ) -> Result<Option<tcl_syntax::native_regex::NativeRegexpRecipe>, tcl_syntax::value::ValueError>
    {
        let dialect = self.native_invocation_dialect();
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
impl core_re::NativeRegexObjects<AreEngine> for Interp {
    fn regex_cached_jim_pattern(
        &self,
        original: &*mut TclObj,
        flags: u32,
    ) -> Result<
        Option<tcl_syntax::native_regex::JimRegexpArtifact<tcl_regex::cmd_core::CompiledRegex>>,
        tcl_syntax::value::ValueError,
    > {
        crate::native_regexp::cached_jim(*original, flags)
    }
    fn regex_install_jim_pattern(
        &mut self,
        original: &*mut TclObj,
        flags: u32,
        program: tcl_regex::cmd_core::CompiledRegex,
    ) -> Result<
        tcl_syntax::native_regex::JimRegexpArtifact<tcl_regex::cmd_core::CompiledRegex>,
        tcl_syntax::value::ValueError,
    > {
        crate::native_regexp::install_jim(*original, flags, program)
    }

    fn regex_jim_range(
        &mut self,
        bytes: &[u8],
        matched: bool,
    ) -> Result<*mut TclObj, tcl_syntax::value::ValueError> {
        use tcl_cmd_core::native_append::NativeAppendObjects;
        let protocol = tcl_syntax::native_string::NativeStringProtocol::Jim084;
        let value = obj::Owned::fresh(obj::new_string_bytes(bytes));
        if matched {
            let dialect = self.native_invocation_dialect();
            let objects = crate::value_ops::RuntimeAppendObjects {
                dialect,
                binary_recipe: dialect.byte_array_string_recipe(None),
            };
            objects.set_string(
                &crate::value_ops::RuntimeAppendValue::borrowed(value.as_ptr()),
                protocol,
                Some((
                    std::rc::Rc::from(bytes),
                    tcl_syntax::native_string::NativeStringStorageIdentity::Allocated,
                )),
                None,
                None,
            )?;
        }
        Ok(value.into_native_unowned())
    }

    fn regex_cached_pattern(
        &self,
        original: &*mut TclObj,
        recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        flags: u32,
    ) -> Result<
        Option<std::rc::Rc<std::cell::RefCell<tcl_regex::cmd_core::CompiledRegex>>>,
        tcl_syntax::value::ValueError,
    > {
        crate::native_regexp::cached(*original, recipe, flags)
    }
    fn regex_cached_glob(
        &self,
        original: &*mut TclObj,
        recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        flags: u32,
    ) -> Result<Option<std::rc::Rc<[u8]>>, tcl_syntax::value::ValueError> {
        crate::native_regexp::glob(*original, recipe, flags)
    }
    fn regex_install_pattern(
        &mut self,
        original: &*mut TclObj,
        recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        flags: u32,
        compiled: std::rc::Rc<std::cell::RefCell<tcl_regex::cmd_core::CompiledRegex>>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        crate::native_regexp::install(*original, recipe, flags, compiled)
    }
    fn regex_range_value(
        &mut self,
        range: tcl_syntax::native_regex::NativeRegexpRange,
    ) -> Result<*mut TclObj, tcl_syntax::value::ValueError> {
        use tcl_cmd_core::native_append::NativeAppendObjects;
        use tcl_syntax::native_regex::NativeRegexpRange;
        let dialect = self.native_invocation_dialect();
        let protocol = dialect.native_string_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("native regexp range issuer"),
        )?;
        match range {
            NativeRegexpRange::Empty => Ok(obj::new_string_bytes(b"")),
            NativeRegexpRange::Unicode(units) => {
                obj::new_native_unicode_obj(std::rc::Rc::from(units), dialect)
            }
            NativeRegexpRange::Bytes { bytes, count } => {
                let original = obj::Owned::fresh(obj::new_string_bytes(&bytes));
                let receiver = crate::value_ops::RuntimeAppendValue::borrowed(original.as_ptr());
                let objects = crate::value_ops::RuntimeAppendObjects {
                    dialect,
                    binary_recipe: dialect.byte_array_string_recipe(None),
                };
                objects.set_string(
                    &receiver,
                    protocol,
                    Some((
                        std::rc::Rc::from(bytes),
                        tcl_syntax::native_string::NativeStringStorageIdentity::Allocated,
                    )),
                    Some(count),
                    None,
                )?;
                Ok(original.into_native_unowned())
            }
        }
    }
}

impl core_re::NativeRegsubObjects for Interp {
    type Object = crate::value_ops::RuntimeAppendValue;
    type Error = Code;
    fn regex_jim_bytes(
        &mut self,
        bytes: &[u8],
        string_primary: bool,
    ) -> Result<Self::Object, tcl_syntax::value::ValueError> {
        <Self as core_re::NativeRegexObjects<AreEngine>>::regex_jim_range(
            self,
            bytes,
            string_primary,
        )
        .map(Self::Object::retain)
    }
    fn regex_result_bytes(&mut self) -> Result<std::rc::Rc<[u8]>, tcl_syntax::value::ValueError> {
        use tcl_syntax::value::ValueOps;
        self.native_string_bytes(&self.get_obj_result())
    }
    fn regex_object<'a>(&self, value: &'a Self::Object) -> &'a *mut TclObj {
        value.pointer()
    }
    fn regex_borrow(&self, value: &*mut TclObj) -> Self::Object {
        Self::Object::borrowed(*value)
    }
    fn regex_duplicate(
        &mut self,
        value: &*mut TclObj,
    ) -> Result<Self::Object, tcl_syntax::value::ValueError> {
        obj::check_native_liveness(*value)?;
        Ok(Self::Object::retain(obj::duplicate(*value)))
    }
    fn regex_members(
        &mut self,
        value: &*mut TclObj,
    ) -> Result<Vec<Self::Object>, tcl_syntax::value::ValueError> {
        use tcl_syntax::value::ValueOps;
        self.list_elements(value)
            .map(|members| members.into_iter().map(Self::Object::borrowed).collect())
    }
    fn regex_unicode(
        &mut self,
        units: &[u32],
    ) -> Result<Self::Object, tcl_syntax::value::ValueError> {
        obj::new_native_unicode_obj(std::rc::Rc::from(units), self.native_invocation_dialect())
            .map(Self::Object::retain)
    }
    fn regex_append_unicode(
        &mut self,
        result: &Self::Object,
        units: &[u32],
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let dialect = self.native_invocation_dialect();
        let protocol = dialect.native_object_append_protocol(None).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "actual regsub Unicode append",
            ),
        )?;
        let objects = crate::value_ops::RuntimeAppendObjects {
            dialect,
            binary_recipe: dialect.byte_array_string_recipe(None),
        };
        tcl_cmd_core::native_append::append_unicode_units(
            &objects,
            protocol.recipe(),
            result,
            units,
        )
    }
    fn regex_append_current_result(
        &mut self,
        result: &mut Self::Object,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let dialect = self.native_invocation_dialect();
        let protocol = dialect.native_object_append_protocol(None).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "actual regsub object append",
            ),
        )?;
        let objects = crate::value_ops::RuntimeAppendObjects {
            dialect,
            binary_recipe: dialect.byte_array_string_recipe(None),
        };
        let source = Self::Object::borrowed(self.get_obj_result());
        *result = tcl_cmd_core::native_append::append_object(
            &objects,
            protocol.recipe(),
            Some(result),
            &source,
        )?
        .into_value();
        Ok(())
    }
    fn regex_eval(&mut self, prefix: &*mut TclObj, arguments: &[Self::Object]) -> Result<(), Code> {
        use tcl_syntax::value::ValueOps;
        let mut call = self
            .list_elements(prefix)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        call.extend(arguments.iter().map(Self::Object::as_ptr));
        let code = self.dispatch(&call);
        if code == Code::Ok {
            Ok(())
        } else {
            Err(code)
        }
    }
    fn regex_reset_result(&mut self) -> Result<(), tcl_syntax::value::ValueError> {
        self.set_result_bytes(b"");
        Ok(())
    }
}

fn regsub_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let version = interp.runtime_version();
    if interp
        .native_invocation_dialect()
        .native_jim_regex_protocol()
        .is_some()
    {
        return match core_re::regsub_jim_original::<Interp, AreEngine>(interp, &argv[1..], version)
        {
            Ok(result) => {
                let pointer = result.result.as_ptr();
                if let Some(target) = result.target {
                    if let Err(code) = interp.assign_original_regex_variable(*target, pointer) {
                        return code;
                    }
                    set_int(interp, result.count);
                } else {
                    interp.set_result(pointer);
                }
                Code::Ok
            }
            Err(RegsubError::Regex(error)) => interp.report_cmd_error(error.into_cmd_error()),
            Err(RegsubError::Eval(code)) => code,
        };
    }
    let prepared = match core_re::regsub_prepare_original(interp, &argv[1..], version) {
        Ok(prepared) => prepared,
        Err(error) => {
            return interp.report_cmd_error(error.into_cmd_error());
        }
    };
    if prepared.is_native() {
        return match core_re::regsub_native_original::<Interp, AreEngine>(interp, &prepared) {
            Ok(result) => {
                let original = result.result.as_ptr();
                if let Some(target) = result.target {
                    if let Err(code) = interp.assign_original_regex_variable(*target, original) {
                        return code;
                    }
                    set_int(interp, result.count);
                } else {
                    interp.set_result(original);
                }
                Code::Ok
            }
            Err(RegsubError::Regex(error)) => interp.report_cmd_error(error.into_cmd_error()),
            Err(RegsubError::Eval(code)) => {
                if code == Code::Error {
                    interp.append_frame_noline(COMMAND_SUBST_FRAME);
                }
                code
            }
        };
    }
    let outcome = core_re::regsub_eval_original::<AreEngine, Code, _>(&prepared, |words| {
        let _ = words;
        Err(interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native regsub command plan was not selected",
            )
            .into(),
        ))
    });
    let core_re::OriginalRegsubResult {
        text,
        count,
        target,
    } = match outcome {
        Ok(r) => r,
        Err(RegsubError::Regex(e)) => {
            return interp.report_cmd_error(e.into_cmd_error());
        }
        Err(RegsubError::Eval(code)) => {
            // C adds the context frame only for a genuine error; a
            // `break`/`continue`/custom code from the prefix propagates
            // untouched (tclsh 9.0.4: `proc q args {return -code continue}`,
            // `catch {regsub -command {.x.} abcxdef q}` → 4, `::errorInfo`
            // never set).
            if code == Code::Error {
                interp.append_frame_noline(COMMAND_SUBST_FRAME);
            }
            return code;
        }
    };

    match target {
        Some(original) => {
            let result = obj::Owned::fresh(new_string_bytes(&text));
            match interp.assign_original_regex_variable(*original, result.as_ptr()) {
                Ok(()) => {
                    set_int(interp, count);
                    Code::Ok
                }
                Err(code) => code,
            }
        }
        None => {
            interp.set_result(new_string_bytes(&text));
            Code::Ok
        }
    }
}

fn set_int(interp: &mut Interp, n: i64) {
    interp.set_result(new_wide_int_obj(n));
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }

    fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?} → {:?}",
            String::from_utf8_lossy(src),
            String::from_utf8_lossy(&i.result_bytes())
        );
        i.result_bytes()
    }

    #[test]
    fn regexp_match_and_captures() {
        leak_free(|i| {
            assert_eq!(ok(i, b"regexp {ab+c} xxabbbcyy"), b"1");
            assert_eq!(ok(i, b"regexp {z} abc"), b"0");
            ok(i, br"regexp {(\w+)@(\w+)} user@host m u h");
            assert_eq!(ok(i, b"set m"), b"user@host");
            assert_eq!(ok(i, b"set u"), b"user");
            assert_eq!(ok(i, b"set h"), b"host");
        });
    }

    #[test]
    fn regexp_all_inline_indices_nocase() {
        leak_free(|i| {
            assert_eq!(ok(i, b"regexp -all {a} banana"), b"3");
            assert_eq!(ok(i, br"regexp -inline {(\d+)} abc123def"), b"123 123");
            ok(i, b"regexp -indices {bc} abcd m");
            assert_eq!(ok(i, b"set m"), b"1 2");
            assert_eq!(ok(i, b"regexp -nocase {ABC} xabcy"), b"1");
        });
    }

    #[test]
    fn regexp_nomatch_leaves_vars_untouched() {
        // tclsh: a failed match does not modify the match variables.
        leak_free(|i| {
            ok(i, b"set m PRESET");
            assert_eq!(ok(i, b"regexp {z} abc m"), b"0");
            assert_eq!(ok(i, b"set m"), b"PRESET");
        });
    }

    #[test]
    fn regsub_basic_all_and_backrefs() {
        leak_free(|i| {
            assert_eq!(ok(i, b"regsub {b} abc X"), b"aXc");
            assert_eq!(ok(i, b"regsub -all {a} banana _"), b"b_n_n_");
            assert_eq!(
                ok(i, br"regsub {(\w+)@(\w+)} user@host {\2.\1}"),
                b"host.user"
            );
            assert_eq!(
                ok(i, b"regsub -all {[aeiou]} {hello world} {}"),
                b"hll wrld"
            );
            // with a result variable, returns the match count.
            assert_eq!(ok(i, b"regsub -all {a} banana _ out"), b"3");
            assert_eq!(ok(i, b"set out"), b"b_n_n_");
            // no match leaves the string unchanged.
            assert_eq!(ok(i, b"regsub {z} abc X"), b"abc");
            // anchor edge: `^` matches once at the start (notbol suppresses it
            // at resumed offsets), per tclsh.
            assert_eq!(ok(i, b"regsub -all {^} abc >"), b">abc");
        });
    }

    #[test]
    fn start_option() {
        leak_free(|i| {
            assert_eq!(ok(i, b"regexp -start 3 {a} {a a a}"), b"1");
            assert_eq!(ok(i, b"regsub -start 2 -all {a} aaaa X"), b"aaXX");
            assert_eq!(ok(i, b"regexp -start 1+1 {a} aaaa"), b"1");
            assert_eq!(ok(i, b"regsub -start 0x2 {a} aaaa X"), b"aaXa");
            assert_eq!(i.eval_str(b"regexp -start bogus {a} aaaa"), Code::Error);
            assert!(i.result_bytes().starts_with(b"bad index \"bogus\""));
            assert_eq!(
                i.eval_str(b"regsub -start {end - 2} {a} aaaa X"),
                Code::Error
            );
            assert!(i.result_bytes().starts_with(b"bad index \"end - 2\""));
        });
    }

    #[test]
    fn bad_pattern_errors() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"regexp {a(} b"), Code::Error);
            assert!(i
                .result_bytes()
                .starts_with(b"cannot compile regular expression pattern"));
        });
    }

    /// `regsub -command` evaluates the prefix once per substitution with the
    /// whole match and each submatch appended. Verified on tclsh 9.0.4 and
    /// 9.1b0:
    ///
    /// ```text
    /// % regsub -command {.x.} {abcxdef} {string length}
    /// ab3ef
    /// % regsub -command -all {(.)(.)} {abcdef} {list ,}
    /// , ab a b, cd c d, ef e f
    /// % regsub -command {(a)|(b)} ab {list <}
    /// < a a {}b
    /// % set n [regsub -command {.x.} abcxdef {string length} out]; list $n $out
    /// 1 ab3ef
    /// % regsub -command {z} abc {string toupper}
    /// abc
    /// ```
    #[test]
    fn regsub_command_evaluates_the_prefix() {
        leak_free(|i| {
            assert_eq!(
                ok(i, b"regsub -command {.x.} {abcxdef} {string length}"),
                b"ab3ef"
            );
            assert_eq!(
                ok(i, b"regsub -command -all {(.)(.)} {abcdef} {list ,}"),
                b", ab a b, cd c d, ef e f"
            );
            // A submatch that did not participate is an empty word, not a
            // missing one — the prefix still sees one word per submatch.
            assert_eq!(
                ok(i, b"regsub -command {(a)|(b)} ab {list <}"),
                b"< a a {}b"
            );
            assert_eq!(
                ok(
                    i,
                    b"set n [regsub -command {.x.} abcxdef {string length} out]; list $n $out"
                ),
                b"1 ab3ef"
            );
            // A pattern that never matches never calls the prefix.
            assert_eq!(ok(i, b"regsub -command {z} abc {string toupper}"), b"abc");
        });
    }

    /// A script error inside the `-command` prefix propagates, and C's context
    /// frame is appended to `errorInfo`. tclsh 9.0.4 / 9.1b0:
    ///
    /// ```text
    /// % proc boomp args { error boom }
    /// % catch {regsub -command {.x.} abcxdef boomp} e; set ::errorInfo
    /// boom
    ///     while executing
    /// "error boom "
    ///     (procedure "boomp" line 1)
    ///     invoked from within
    /// "boomp cxd"
    ///     (-command substitution computation script)
    ///     invoked from within
    /// "regsub -command {.x.} abcxdef boomp"
    /// ```
    ///
    /// The trace omits the `invoked from within "boomp cxd"` frame because the
    /// prefix is invoked argv-wise (`Interp::dispatch`), which carries no
    /// source text to quote — the same pre-existing shape `lsort -command`
    /// has. The message, the `(-command substitution computation script)`
    /// frame and its position before the enclosing command's frame all match
    /// C.
    #[test]
    fn regsub_command_error_appends_the_c_error_info_trailer() {
        leak_free(|i| {
            let got = ok(
                i,
                b"proc boomp args { error boom }\n\
                  catch {regsub -command {.x.} abcxdef boomp} e\n\
                  list $e $::errorInfo",
            );
            assert_eq!(
                String::from_utf8_lossy(&got),
                "boom {boom\n    while executing\n\"error boom \"\n    \
                 (procedure \"boomp\" line 1)\n    \
                 (-command substitution computation script)\n    \
                 invoked from within\n\"regsub -command {.x.} abcxdef boomp\"}"
            );
        });
    }

    /// A non-error completion code from the prefix propagates untouched, with
    /// no `errorInfo` trailer. tclsh 9.0.4:
    ///
    /// ```text
    /// % proc q args { return -code continue }
    /// % catch {regsub -command {.x.} {abcxdef} q} r
    /// 4
    /// % info exists ::errorInfo
    /// 0
    /// ```
    #[test]
    fn regsub_command_non_error_code_propagates_untouched() {
        leak_free(|i| {
            let got = ok(
                i,
                b"proc q args { return -code continue }\n\
                  list [catch {regsub -command {.x.} {abcxdef} q} r] $r \
                  [info exists ::errorInfo]",
            );
            assert_eq!(String::from_utf8_lossy(&got), "4 {} 0");
        });
    }

    /// `-command` is a 9.0 option: before it, `regsub` rejects it in that
    /// release's own noun and enumeration. The adapter passes the
    /// interpreter's pinned release to the core, so the refusal follows
    /// `info patchlevel`:
    ///
    /// ```text
    /// $ tclsh8.4 / tclsh8.5   (8.4.20 / 8.5.19)
    /// % catch {regsub -command {a} abc {string toupper}} r; set r
    /// bad switch "-command": must be -all, -nocase, -expanded, -line, -linestop, -lineanchor, -start, or --
    /// $ tclsh8.6              (8.6.18)
    /// bad option "-command": must be -all, -nocase, -expanded, -line, -linestop, -lineanchor, -start, or --
    /// $ tclsh9.0 / tclsh9.1   (9.0.4 / 9.1b0)
    /// Abc
    /// ```
    #[test]
    fn regsub_command_is_a_9_0_option() {
        use tcl_dialect::TclVersion;
        const ENUM: &str =
            "must be -all, -nocase, -expanded, -line, -linestop, -lineanchor, -start, or --";
        const SRC: &[u8] = b"regsub -command {a} abc {string toupper}";
        for (version, noun) in [
            (TclVersion::V8_4, "switch"),
            (TclVersion::V8_5, "switch"),
            (TclVersion::V8_6, "option"),
        ] {
            leak_free(|i| {
                i.set_runtime_version(version);
                assert_eq!(i.eval_str(SRC), Code::Error, "for {version:?}");
                assert_eq!(
                    String::from_utf8_lossy(&i.result_bytes()),
                    format!("bad {noun} \"-command\": {ENUM}"),
                    "for {version:?}"
                );
            });
        }
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            leak_free(|i| {
                i.set_runtime_version(version);
                assert_eq!(i.eval_str(SRC), Code::Ok, "for {version:?}");
                assert_eq!(i.result_bytes(), b"Abc", "for {version:?}");
            });
        }
    }

    /// `\z` is an end-of-string anchor from Tcl 9.1.0 on, through every
    /// command that compiles an ARE; 9.0.4 rejects it. Verified on tclsh
    /// 9.1.0 and 9.0.4.
    #[test]
    fn z_anchor_is_a_9_1_escape() {
        use tcl_dialect::TclVersion;
        const CASES: &[(&[u8], &[u8])] = &[
            (br"regexp {a\z} xa", b"1"),
            (br"regsub {a\z} xaa b", b"xab"),
            (br"lsearch -regexp {ab xa} {a\z}", b"1"),
            (
                br"switch -regexp xa {{a\z} {set r hit} default {set r miss}}",
                b"hit",
            ),
        ];
        for &(src, want) in CASES {
            leak_free(|i| {
                i.set_runtime_version(TclVersion::V9_1);
                assert_eq!(ok(i, src), want);
            });
            leak_free(|i| {
                i.set_runtime_version(TclVersion::V9_0);
                assert_eq!(i.eval_str(src), Code::Error);
                assert_eq!(
                    String::from_utf8_lossy(&i.result_bytes()),
                    r"cannot compile regular expression pattern: invalid escape \ sequence"
                );
            });
        }
    }

    /// Tcl 9.0 reworded the compile-error prefix from `couldn't` to `cannot`,
    /// for every command that compiles an ARE. Verified on tclsh 8.4.20,
    /// 8.5.19, 8.6.18, 9.0.4 and 9.1.0.
    #[test]
    fn compile_error_prefix_follows_the_release() {
        use tcl_dialect::TclVersion;
        const CASES: &[&[u8]] = &[
            b"regexp {(} x",
            b"regexp -about {(}",
            b"regsub {(} x y",
            b"lsearch -regexp {a b} (",
            b"switch -regexp xa {( {set r hit}}",
        ];
        for (version, verb) in [
            (TclVersion::V8_4, "couldn't"),
            (TclVersion::V8_5, "couldn't"),
            (TclVersion::V8_6, "couldn't"),
            (TclVersion::V9_0, "cannot"),
            (TclVersion::V9_1, "cannot"),
        ] {
            for &src in CASES {
                leak_free(|i| {
                    i.set_runtime_version(version);
                    assert_eq!(i.eval_str(src), Code::Error);
                    assert_eq!(
                        String::from_utf8_lossy(&i.result_bytes()),
                        format!(
                            "{verb} compile regular expression pattern: parentheses () not balanced"
                        ),
                        "{version:?} `{}`",
                        String::from_utf8_lossy(src)
                    );
                });
            }
        }
    }
}

#[cfg(test)]
mod original_object_tests {
    use super::*;
    use obj::Owned;
    use tcl_cmd_core::regex::RegexFlags;
    use tcl_dialect::TclVersion;

    fn actual(environment: &str) -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(environment),
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .expect("authentic C constructor before bootstrap")
    }

    #[test]
    fn jim_original_start_indices_use_actual_safe_expression_owner_for_all_17_native_controls() {
        use core_re::NativeRegexSource;
        let rows = include_str!("../../../rust/tcl-syntax/tests/data/native_jim_index/rows.txt");
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
            let original = Owned::fresh(obj::new_string_bytes(&source));
            let result = ops.jim_regex_index(&original.as_ptr());
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
                    ops.report_cmd_error(error);
                }
            }
            let actual_kind = if obj::obj_type_ptr(original.as_ptr()).is_null() {
                "none".to_owned()
            } else {
                // SAFETY: a live object owns a valid static descriptor name.
                unsafe { std::ffi::CStr::from_ptr((*obj::obj_type_ptr(original.as_ptr())).name) }
                    .to_str()
                    .unwrap()
                    .to_owned()
            };
            assert_eq!(actual_kind, expected_kind, "case {case}");
            assert!(obj::has_string_rep(original.as_ptr()), "case {case}");
            // SAFETY: this original header is pinned by the actual Owned lease.
            assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1, "case {case}");
            let result = ops.get_obj_result();
            let result_kind = if obj::obj_type_ptr(result).is_null() {
                "none".to_owned()
            } else {
                // SAFETY: the actual result owns its static type descriptor.
                unsafe { std::ffi::CStr::from_ptr((*obj::obj_type_ptr(result)).name) }
                    .to_str()
                    .unwrap()
                    .to_owned()
            };
            assert_eq!(
                result_kind, expected_physical[0],
                "case {case} result primary"
            );
            assert_eq!(
                obj::has_string_rep(result),
                expected_physical[1] == "1",
                "case {case} result String presence"
            );
            // SAFETY: the actual interpreter pins this observed result header.
            assert_eq!(
                unsafe { (*result).ref_count },
                expected_physical[2].parse::<isize>().unwrap(),
                "case {case} result references"
            );
            cases += 1;
        }
        assert_eq!(cases, 17);
    }

    #[test]
    fn jim_original_regexp_cache_ranges_and_unsafe_lifecycles_match_native_controls() {
        use core_re::NativeRegexObjects;
        let mut interp = actual("jim");
        for bytes in [b"a".as_slice(), b"\xff", b"\xc0\x80", b"\xf0\x9f\x98\x80"] {
            let head = Owned::fresh(obj::new_string_bytes(b"regexp"));
            let pattern = Owned::fresh(obj::new_string_bytes(bytes));
            let subject = Owned::fresh(obj::new_string_bytes(bytes));
            let name = Owned::fresh(obj::new_string_bytes(b"m"));
            assert_eq!(
                regexp_cmd(
                    &mut interp,
                    &[
                        head.as_ptr(),
                        pattern.as_ptr(),
                        subject.as_ptr(),
                        name.as_ptr()
                    ]
                ),
                Code::Ok
            );
            assert!(std::ptr::eq(
                obj::obj_type_ptr(pattern.as_ptr()),
                &crate::native_regexp::JIM_REGEXP_TYPE
            ));
            assert!(obj::obj_type_ptr(subject.as_ptr()).is_null());
            assert!(std::ptr::eq(
                obj::obj_type_ptr(interp.get_obj_result()),
                &obj::TCL_INT_TYPE
            ));
            assert!(!obj::has_string_rep(interp.get_obj_result()));
            let range = interp.read_named_variable(b"m").unwrap();
            assert!(std::ptr::eq(
                obj::obj_type_ptr(range),
                &obj::JIM_STRING_TYPE
            ));
            assert_eq!(obj::bytes_of(range).as_slice(), bytes);
            let artifact = interp
                .regex_cached_jim_pattern(&pattern.as_ptr(), 0)
                .unwrap()
                .unwrap();
            obj::invalidate_string(pattern.as_ptr());
            assert!(artifact.same_program(
                &interp
                    .regex_cached_jim_pattern(&pattern.as_ptr(), 0)
                    .unwrap()
                    .unwrap()
            ));
            assert!(core_re::prepare_pattern_original::<Interp, AreEngine>(
                &mut interp,
                &pattern.as_ptr(),
                RegexFlags::for_release(TclVersion::V9_0),
                TclVersion::V9_0
            )
            .is_err());
        }
        let pattern = Owned::fresh(obj::new_string_bytes(b"a"));
        core_re::prepare_pattern_original::<Interp, AreEngine>(
            &mut interp,
            &pattern.as_ptr(),
            RegexFlags::for_release(TclVersion::V9_0),
            TclVersion::V9_0,
        )
        .unwrap();
        let duplicate = Owned::fresh(obj::duplicate(pattern.as_ptr()));
        assert!(interp
            .regex_cached_jim_pattern(&duplicate.as_ptr(), 0)
            .unwrap()
            .is_some());
        drop(duplicate);
        assert!(interp
            .regex_cached_jim_pattern(&pattern.as_ptr(), 0)
            .is_err());
    }
    #[test]
    fn jim_regsub_original_objects_match_all_nine_completed_native_controls() {
        let rows = include_str!("../tests/data/native_regexp_jim_regsub/rows.txt");
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
            let mut interp = actual("jim");
            let pattern = Owned::fresh(obj::new_string_bytes(patterns[case]));
            let subject = Owned::fresh(obj::new_string_bytes(subject));
            let replacement = Owned::fresh(obj::new_string_bytes(replacement));
            let mut holders = vec![
                Owned::fresh(obj::new_string_bytes(b"regsub")),
                Owned::fresh(obj::new_string_bytes(if case == 8 {
                    b"-command"
                } else {
                    b"-all"
                })),
            ];
            if case == 7 {
                holders.push(Owned::fresh(obj::new_string_bytes(b"-start")));
                holders.push(Owned::fresh(obj::new_string_bytes(b"end-1")));
            }
            let mut argv = holders.iter().map(Owned::as_ptr).collect::<Vec<_>>();
            argv.extend([pattern.as_ptr(), subject.as_ptr(), replacement.as_ptr()]);
            assert_eq!(
                regsub_cmd(&mut interp, &argv),
                Code::Ok,
                "native case {case}"
            );
            assert!(std::ptr::eq(
                obj::obj_type_ptr(interp.get_obj_result()),
                &obj::JIM_STRING_TYPE
            ));
            assert_eq!(obj::bytes_of(interp.get_obj_result()), expected[case]);
            assert!(obj::obj_type_ptr(pattern.as_ptr()).is_null());
            assert!(obj::obj_type_ptr(subject.as_ptr()).is_null());
            assert_eq!(obj::obj_type_ptr(replacement.as_ptr()).is_null(), case != 8);
            // SAFETY: these actual original holders remain live through the comparison.
            unsafe {
                assert_eq!((*pattern.as_ptr()).ref_count, 1);
                assert_eq!((*subject.as_ptr()).ref_count, 1);
                assert_eq!((*replacement.as_ptr()).ref_count, 1);
            }
        }
    }

    #[test]
    fn jim_regsub_duplicate_and_match_consumers_preserve_original_dispatch() {
        let mut interp = actual("jim");
        let args = [b"regsub".as_slice(), b"-all", b"a", b"aba", b"X"]
            .map(|bytes| Owned::fresh(obj::new_string_bytes(bytes)));
        let pointers = args.iter().map(Owned::as_ptr).collect::<Vec<_>>();
        assert_eq!(regsub_cmd(&mut interp, &pointers), Code::Ok);
        assert!(std::ptr::eq(
            obj::obj_type_ptr(interp.get_obj_result()),
            &obj::JIM_STRING_TYPE
        ));
        assert_eq!(obj::bytes_of(interp.get_obj_result()).as_slice(), b"XbX");
        assert!(obj::obj_type_ptr(args[2].as_ptr()).is_null());
        assert_eq!(
            interp.eval_str(b"rename regexp original_regexp; proc regexp args {return 1}"),
            Code::Ok
        );
        let pattern = Owned::fresh(obj::new_string_bytes(b"["));
        let subject = Owned::fresh(obj::new_string_bytes(b"anything"));
        assert!(
            invoke_jim_regexp(&mut interp, &pattern.as_ptr(), &subject.as_ptr(), false).unwrap()
        );
    }

    #[test]
    fn native_boolean_regexp_glob_preserves_subject_storage_and_cached_pattern() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut ops = actual(environment);
            let recipe = ops
                .native_invocation_dialect()
                .native_regex_protocol()
                .unwrap();
            let pattern = Owned::fresh(obj::new_string_bytes(b"\xff"));
            let subject = Owned::fresh(obj::new_string_bytes(b"\xff"));
            let mut flags = RegexFlags::for_release(recipe.version());
            flags.nosub = true;
            let mut prepared = core_re::prepare_search_pattern_original::<Interp, AreEngine>(
                &mut ops,
                &pattern.as_ptr(),
                flags,
                recipe.version(),
            )
            .unwrap();
            assert!(core_re::match_pattern_original::<Interp, AreEngine>(
                &mut ops,
                &mut prepared,
                &subject.as_ptr()
            )
            .unwrap());
            assert_eq!(
                if obj::obj_type_ptr(subject.as_ptr()).is_null() {
                    "none"
                } else {
                    "string"
                },
                if recipe.version() == TclVersion::V8_5 {
                    "none"
                } else {
                    "string"
                },
                "{environment}"
            );
            let compiled =
                crate::native_regexp::cached(pattern.as_ptr(), recipe, flags.cache_key())
                    .unwrap()
                    .unwrap();
            obj::invalidate_string(pattern.as_ptr());
            let mut cached = core_re::prepare_search_pattern_original::<Interp, AreEngine>(
                &mut ops,
                &pattern.as_ptr(),
                flags,
                recipe.version(),
            )
            .unwrap();
            assert!(core_re::match_pattern_original::<Interp, AreEngine>(
                &mut ops,
                &mut cached,
                &subject.as_ptr()
            )
            .unwrap());
            assert!(std::rc::Rc::ptr_eq(
                &compiled,
                &crate::native_regexp::cached(pattern.as_ptr(), recipe, flags.cache_key())
                    .unwrap()
                    .unwrap()
            ));
        }
    }

    #[test]
    fn native_regexp_cache_hit_precedes_getter_and_duplicate_keeps_artifact() {
        use std::rc::Rc;
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = actual(environment);
            let recipe = interp
                .native_invocation_dialect()
                .native_regex_protocol()
                .unwrap();
            let flags = RegexFlags::for_release(recipe.version());
            let pattern = Owned::fresh(obj::new_string_bytes(b"a"));
            let subject = Owned::fresh(obj::new_string_bytes(b"a"));
            assert!(core_re::compiled_match_original::<Interp, AreEngine>(
                &mut interp,
                &pattern.as_ptr(),
                &subject.as_ptr(),
                flags,
                recipe.version()
            )
            .unwrap());
            assert!(std::ptr::eq(
                obj::obj_type_ptr(pattern.as_ptr()),
                &crate::native_regexp::REGEXP_TYPE
            ));
            let compiled =
                crate::native_regexp::cached(pattern.as_ptr(), recipe, flags.cache_key())
                    .unwrap()
                    .unwrap();
            obj::invalidate_string(pattern.as_ptr());
            assert!(core_re::compiled_match_original::<Interp, AreEngine>(
                &mut interp,
                &pattern.as_ptr(),
                &subject.as_ptr(),
                flags,
                recipe.version()
            )
            .unwrap());
            assert!(!obj::has_string_rep(pattern.as_ptr()));
            assert!(Rc::ptr_eq(
                &compiled,
                &crate::native_regexp::cached(pattern.as_ptr(), recipe, flags.cache_key())
                    .unwrap()
                    .unwrap()
            ));
            let duplicate = Owned::fresh(obj::duplicate(pattern.as_ptr()));
            assert!(Rc::ptr_eq(
                &compiled,
                &crate::native_regexp::cached(duplicate.as_ptr(), recipe, flags.cache_key())
                    .unwrap()
                    .unwrap()
            ));
            assert!(!obj::has_string_rep(duplicate.as_ptr()));
            let changed = RegexFlags {
                nocase: true,
                ..flags
            };
            assert!(core_re::compiled_match_original::<Interp, AreEngine>(
                &mut interp,
                &pattern.as_ptr(),
                &subject.as_ptr(),
                changed,
                recipe.version()
            )
            .is_err());
            assert!(!obj::has_string_rep(pattern.as_ptr()));
            let invalid = Owned::fresh(obj::new_string_bytes(b"("));
            assert!(core_re::compiled_match_original::<Interp, AreEngine>(
                &mut interp,
                &invalid.as_ptr(),
                &subject.as_ptr(),
                flags,
                recipe.version()
            )
            .is_err());
            assert!(obj::obj_type_ptr(invalid.as_ptr()).is_null());
            assert_eq!(obj::bytes_of(invalid.as_ptr()), b"(");
        }
    }

    #[test]
    fn native_regexp_ranges_preserve_selected_original_units_and_storage() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for input in [b"a".as_slice(), b"\xff", b"\xc0\x80", b"\xf0\x9f\x98\x80"] {
                let mut interp = actual(environment);
                let words = [b"regexp".as_slice(), input, input, b"m"]
                    .map(|bytes| Owned::fresh(obj::new_string_bytes(bytes)));
                let argv = words.iter().map(Owned::as_ptr).collect::<Vec<_>>();
                assert_eq!(
                    interp.eval_original_object_vector(&argv),
                    Code::Ok,
                    "{environment}/{input:?}"
                );
                assert!(std::ptr::eq(
                    obj::obj_type_ptr(argv[1]),
                    &crate::native_regexp::REGEXP_TYPE
                ));
                assert!(matches!(
                    obj::native_object_snapshot(argv[2]).unwrap().cache,
                    tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
                ));
                let result = interp.read_named_variable(b"m").unwrap();
                assert!(matches!(
                    obj::native_object_snapshot(result).unwrap().cache,
                    tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
                ));
                let older = matches!(environment, "tcl8.4" | "tcl8.5");
                assert_eq!(
                    obj::has_string_rep(result),
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
                assert_eq!(obj::bytes_of(result), expected, "{environment}/{input:?}");
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
                let mut interp = actual(environment);
                let words = [b"regsub".as_slice(), b"-all", pattern, b"ab", replacement]
                    .map(|bytes| Owned::fresh(obj::new_string_bytes(bytes)));
                let argv = words.iter().map(Owned::as_ptr).collect::<Vec<_>>();
                assert_eq!(
                    interp.eval_original_object_vector(&argv),
                    Code::Ok,
                    "{environment}"
                );
                if mapping {
                    assert!(matches!(
                        obj::native_object_snapshot(argv[2]).unwrap().cache,
                        tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
                    ));
                } else {
                    assert!(std::ptr::eq(
                        obj::obj_type_ptr(argv[2]),
                        &crate::native_regexp::REGEXP_TYPE
                    ));
                }
                let result = interp.get_obj_result();
                assert_eq!(result == argv[3], same);
                assert_eq!(
                    obj::has_string_rep(result),
                    same || matches!(environment, "tcl8.4" | "tcl8.5")
                );
                assert!(matches!(
                    obj::native_object_snapshot(result).unwrap().cache,
                    tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
                ));
                assert_eq!(interp.result_bytes(), expected);
            }
        }
    }

    #[test]
    fn original_regexp_each_match_and_first_failure_match_native_controls() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = actual(environment);
            let trace = if environment.starts_with("tcl9") {
                "trace add variable m write observe"
            } else {
                "trace variable m w observe"
            };
            let source = format!(
                "set log {{}}; proc observe {{name index op}} {{upvar 1 $name v; lappend ::log $v}}; {trace}; regexp -all {{(.)}} abc m s; list $log $m $s"
            );
            assert_eq!(
                interp.eval_str(source.as_bytes()),
                Code::Ok,
                "{environment}"
            );
            assert_eq!(interp.result_bytes(), b"{a b c} c c", "{environment}");
            // The original C84 scripted failing-trace specimen aborts inside
            // the native engine; its successful match and scalar-error controls
            // are independent of that unavailable completion.
            if environment == "tcl8.4" {
                continue;
            }
            let mut interp = actual(environment);
            let source = format!(
                "set log {{}}; proc observe {{name index op}} {{upvar 1 $name v; lappend ::log $v; if {{$v == \"b\"}} {{error STOP}}}}; {trace}; set rc [catch {{regexp -all {{(.)}} abc m s}} msg]; list $rc $log $msg $s $::errorCode"
            );
            assert_eq!(
                interp.eval_str(source.as_bytes()),
                Code::Ok,
                "{environment}"
            );
            let expected = if environment == "tcl8.5" {
                b"1 {a b} {couldn't set variable \"m\"} a NONE".as_slice()
            } else {
                b"1 {a b} {can't set \"m\": STOP} a {TCL WRITE VARNAME}"
            };
            assert_eq!(interp.result_bytes(), expected, "{environment}");
        }
    }

    #[test]
    fn original_regex_unmatched_target_and_quiet_write_match_native_controls() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = actual(environment);
            let member = Owned::fresh(obj::new_string_bytes(b"untouched"));
            let target = Owned::fresh(interp.new_list_object(&[member.as_ptr()]));
            drop(member);
            let words = [b"regexp".as_slice(), b"z", b"abc"]
                .map(|bytes| Owned::fresh(obj::new_string_bytes(bytes)));
            let mut argv = words.iter().map(Owned::as_ptr).collect::<Vec<_>>();
            argv.push(target.as_ptr());
            assert_eq!(interp.eval_original_object_vector(&argv), Code::Ok);
            assert!(!obj::has_string_rep(target.as_ptr()), "{environment}");
            assert!(std::ptr::eq(
                obj::obj_type_ptr(target.as_ptr()),
                &crate::list::TCL_LIST_TYPE
            ));
            // SAFETY: target remains pinned by its one actual owning lease.
            assert_eq!(unsafe { (*target.as_ptr()).ref_count }, 1);
            assert_eq!(interp.result_bytes(), b"0");

            assert_eq!(
                interp.eval_str(b"set a scalar; regsub a a b a(k)"),
                Code::Error,
                "{environment}"
            );
            let primary = obj::obj_type_ptr(interp.get_obj_result());
            if environment == "tcl8.4" {
                assert!(primary.is_null());
            } else {
                assert!(matches!(
                    obj::native_object_snapshot(interp.get_obj_result())
                        .unwrap()
                        .cache,
                    tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
                ));
            }
            let expected = if matches!(environment, "tcl8.4" | "tcl8.5") {
                b"couldn't set variable \"a(k)\"".as_slice()
            } else {
                b"can't set \"a(k)\": variable isn't array"
            };
            assert_eq!(interp.result_bytes(), expected, "{environment}");
        }
    }

    thread_local! {
        static SEED: std::cell::Cell<*mut TclObj> = const { std::cell::Cell::new(std::ptr::null_mut()) };
        static OBSERVATIONS: std::cell::RefCell<Vec<(bool, i64, bool, i64, bool)>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    fn keep(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        let seed = SEED.with(std::cell::Cell::get);
        assert_eq!(argv.len(), 4);
        assert!(std::ptr::eq(
            obj::obj_type_ptr(argv[1]),
            &crate::list::TCL_LIST_TYPE
        ));
        assert!(matches!(
            obj::native_object_snapshot(argv[2]).unwrap().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
        ));
        // SAFETY: original prefix and generated invocation arguments pin headers.
        let row = unsafe {
            (
                argv[1] == seed,
                (*argv[1]).ref_count as i64,
                obj::has_string_rep(argv[1]),
                (*argv[2]).ref_count as i64,
                obj::has_string_rep(argv[2]),
            )
        };
        OBSERVATIONS.with(|rows| rows.borrow_mut().push(row));
        interp.set_result(seed);
        Code::Ok
    }

    #[test]
    fn original_regsub_prefix_and_unicode_arguments_match_native_c9_controls() {
        for environment in ["tcl9.0", "tcl9.1"] {
            let mut interp = actual(environment);
            interp.register_builtin(b"keep", keep);
            let child = Owned::fresh(obj::new_string_bytes(b"S"));
            let seed = Owned::fresh(interp.new_list_object(&[child.as_ptr()]));
            drop(child);
            let head = Owned::fresh(obj::new_string_bytes(b"keep"));
            let prefix = Owned::fresh(interp.new_list_object(&[head.as_ptr(), seed.as_ptr()]));
            SEED.with(|slot| slot.set(seed.as_ptr()));
            drop(seed);
            drop(head);
            OBSERVATIONS.with(|rows| rows.borrow_mut().clear());
            let words = [b"regsub".as_slice(), b"-all", b"-command", b"(.)", b"ab"]
                .map(|bytes| Owned::fresh(obj::new_string_bytes(bytes)));
            let mut argv = words.iter().map(Owned::as_ptr).collect::<Vec<_>>();
            argv.push(prefix.as_ptr());
            assert_eq!(
                interp.eval_original_object_vector(&argv),
                Code::Ok,
                "{environment}"
            );
            assert_eq!(
                OBSERVATIONS.with(|rows| rows.borrow().clone()),
                [(true, 1, false, 1, false), (true, 1, true, 1, false)],
                "{environment}"
            );
            let result = interp.get_obj_result();
            assert!(matches!(
                obj::native_object_snapshot(result).unwrap().cache,
                tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
            ));
            assert!(!obj::has_string_rep(result));
            // SAFETY: the physical interpreter result owns this header.
            assert_eq!(unsafe { (*result).ref_count }, 1);
            assert_eq!(interp.result_bytes(), b"SS");
        }
    }
}
