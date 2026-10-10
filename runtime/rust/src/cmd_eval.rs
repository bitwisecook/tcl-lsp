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

//! `eval` + `uplevel` — running scripts in the current / an enclosing scope.
//!
//! Both are **transparent**: the body's completion code (including `return`)
//! propagates unchanged (unlike a proc/`source`, which map `return`→Ok). `eval`
//! runs in the current scope; `uplevel ?level?` runs in an enclosing frame's
//! variable scope *and* namespace (C Tcl `tclProc.c` `Tcl_UplevelObjCmd` —
//! restore caller ns + depth together). Multiple args are space-joined (the `concat`-style
//! eval form). Level resolution uses the shared original-object frame protocol.

use crate::interp::{Code, Interp};
use crate::obj::TclObj;
use tcl_registry::frame_effect::{NativeFrameLevelFailure, NativeFrameLevelObject};
use tcl_syntax::scalar_getter::{
    NativeScalarGetterFailure, NativeScalarGetterKind, NativeScalarGetterValue,
};
use tcl_syntax::value::ValueError;

#[cfg(test)]
#[path = "cmd_eval/native_frame_reference_tests.rs"]
mod native_frame_reference_tests;

/// Register `eval` and `uplevel`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"eval", eval_cmd);
    interp.register_builtin(b"uplevel", uplevel_cmd);
}

/// Assemble multiple original script operands with the selected native concat
/// owner. The fresh script keeps its full counted bytes and its own storage.
fn joined_body(interp: &mut Interp, args: &[*mut TclObj]) -> Result<crate::obj::Owned, Code> {
    tcl_cmd_core::list::concat_selected(interp, args)
        .map(crate::obj::Owned::fresh)
        .map_err(|error| interp.report_cmd_error(error))
}

/// `eval arg ?arg ...?` — concatenate the args and evaluate in the current scope.
fn eval_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"eval arg ?arg ...?");
    }
    // A single body argument keeps its object identity, so a literal from a
    // sourced file evaluates as `type source` (TIP 280 LABC); multiple args are
    // concatenated into a fresh dynamic script (`type eval`).
    let code = if argv.len() == 2 {
        interp.eval_body_obj(argv[1])
    } else {
        match joined_body(interp, &argv[1..]) {
            Ok(body) => interp.eval_body_obj(body.as_ptr()),
            Err(code) => return code,
        }
    };
    if code == Code::Error {
        // `("eval" body line N)` — a body evaluated through a fresh frame.
        interp.append_body_frame(b"eval");
    }
    code
}

struct OriginalLevel {
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
}

impl NativeFrameLevelObject for OriginalLevel {
    type Error = ValueError;
    fn native_frame_string(&mut self) -> Result<Vec<u8>, Self::Error> {
        let protocol =
            self.dialect
                .native_string_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native frame string",
                ))?;
        crate::dict::native_object_bytes(self.value, protocol)
    }
    fn native_frame_probe(
        &mut self,
        kind: NativeScalarGetterKind,
    ) -> Result<Result<i64, NativeScalarGetterFailure>, Self::Error> {
        crate::typed_value::native_scalar_probe(self.value, self.dialect, kind).map(|result| {
            result.map(|value| match value {
                NativeScalarGetterValue::Wide(integer) => integer,
                _ => unreachable!("integer frame getter"),
            })
        })
    }
    fn native_frame_is_integer(&self) -> bool {
        matches!(
            crate::obj::native_scalar_cache(self.value),
            Ok(Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                tcl_syntax::number::Number::Int(_) | tcl_syntax::number::Number::Big { .. }
            )))
        )
    }
    fn native_frame_is_machine_integer(&self) -> bool {
        matches!(
            crate::obj::native_scalar_cache(self.value),
            Ok(Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                tcl_syntax::number::Number::Int(_)
            )))
        )
    }
    fn native_frame_cache(
        &self,
    ) -> Result<Option<tcl_registry::NativeFrameLevelCache>, Self::Error> {
        crate::obj::native_frame_level_cache_in(self.value, self.dialect)
    }
    fn native_frame_set_cache(
        &mut self,
        cache: tcl_registry::NativeFrameLevelCache,
    ) -> Result<(), Self::Error> {
        crate::obj::install_native_frame_level_cache(self.value, cache, self.dialect)
    }
}

fn frame_failure(interp: &mut Interp, failure: NativeFrameLevelFailure) -> Code {
    interp.report_cmd_error(tcl_cmd_core::native_frame_error::present(failure))
}

pub(crate) fn select_frame(
    interp: &mut Interp,
    args: &[*mut TclObj],
    effect: tcl_registry::FrameEffectSpec,
) -> Result<(usize, usize), Code> {
    let dialect = interp.eval_frame_dialect();
    let Some(protocol) = dialect.native_frame_level_protocol() else {
        return Err(interp.report_cmd_error(
            ValueError::CommandProtocolUnavailable("native original-object frame protocol").into(),
        ));
    };
    let current = interp.current_level();
    let default = |interp: &mut Interp| {
        current
            .checked_sub(1)
            .map(|target| (0, target))
            .ok_or_else(|| {
                frame_failure(
                    interp,
                    NativeFrameLevelFailure::BadLevel {
                        name: b"1".to_vec(),
                        string_result: protocol.bad_level_string_result(),
                        lookup_code: protocol
                            .tcl_version()
                            .is_some_and(|version| version >= tcl_dialect::TclVersion::V8_6),
                    },
                )
            })
    };
    let width = effect.level_word_len_for_native_bytes(args.len(), None, dialect);
    if width == Some(0) {
        return default(interp);
    }
    let original = args[0];
    if effect == tcl_registry::FrameEffectSpec::UPLEVEL
        && args.len() == 1
        && protocol.probes_single_script_list_first()
        && !crate::obj::has_string_rep(original)
    {
        let string = dialect
            .native_string_protocol()
            .expect("native frame protocol owns string recipe");
        match crate::list::list_elements_native_checked(original, string) {
            Ok(elements) if elements.len() > 1 => return default(interp),
            Err(error) if error.native_access_refusal().is_some() => {
                return Err(interp.report_cmd_error(error.into()));
            }
            _ => {}
        }
    }
    let mut operand = OriginalLevel {
        value: original,
        dialect,
    };
    let result = if width == Some(1) {
        protocol.resolve_required_object(current, &mut operand)
    } else {
        protocol.resolve_leading_object(current, &mut operand)
    };
    match result {
        Ok(Ok(selected)) => Ok((
            width.unwrap_or(usize::from(selected.explicit)),
            selected.target,
        )),
        Ok(Err(failure)) => Err(frame_failure(interp, failure)),
        Err(error) => Err(interp.report_cmd_error(error.into())),
    }
}

/// The C9.1 UPLEVEL instruction already has an explicit original level operand.
pub(crate) fn select_compiled_uplevel_frame(
    interp: &mut Interp,
    original: *mut TclObj,
) -> Result<usize, Code> {
    let dialect = interp.eval_frame_dialect();
    let Some(protocol) = dialect
        .native_frame_level_protocol()
        .filter(|protocol| protocol.tcl_version() == Some(tcl_dialect::TclVersion::V9_1))
    else {
        return Err(interp.report_cmd_error(
            ValueError::CommandProtocolUnavailable("native UPLEVEL frame protocol").into(),
        ));
    };
    let mut operand = OriginalLevel {
        value: original,
        dialect,
    };
    match protocol.resolve_object(interp.current_level(), &mut operand) {
        Ok(Ok(selected)) => Ok(selected.target),
        Ok(Err(failure)) => Err(frame_failure(interp, failure)),
        Err(error) => Err(interp.report_cmd_error(error.into())),
    }
}

/// `uplevel ?level? arg ?arg ...?` — evaluate in an enclosing frame's scope.
/// The optional level is `#N` (absolute) or `N` (relative; default 1); it is
/// present iff the first arg is level-shaped (`#`digits / all-digits).
fn uplevel_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let usage = b"uplevel ?level? command ?arg ...?";
    if argv.len() < 2 {
        return interp.wrong_args(usage);
    }
    let (width, target) =
        match select_frame(interp, &argv[1..], tcl_registry::FrameEffectSpec::UPLEVEL) {
            Ok(selected) => selected,
            Err(code) => return code,
        };
    let body_start = width + 1;
    if body_start >= argv.len() {
        return interp.wrong_args(usage);
    }
    let code = if body_start == argv.len() - 1 {
        interp.eval_uplevel_obj(target, argv[body_start])
    } else {
        match joined_body(interp, &argv[body_start..]) {
            Ok(body) => interp.eval_uplevel_obj(target, body.as_ptr()),
            Err(code) => return code,
        }
    };
    if code == Code::Error {
        // `("uplevel" body line N)`.
        interp.append_body_frame(b"uplevel");
    }
    code
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

    fn run(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?}",
            String::from_utf8_lossy(src)
        );
        i.result_bytes()
    }

    #[test]
    fn original_list_eval_retains_members_and_does_not_generate_source() {
        for version in tcl_dialect::TclVersion::ALL {
            leak_free(|interp| {
                interp.set_runtime_version(version);
                let command = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"return"));
                let result = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"\xff\0TAIL"));
                let script = crate::obj::Owned::fresh(crate::list::new_list_obj(&[
                    command.as_ptr(),
                    result.as_ptr(),
                ]));
                assert_eq!(interp.eval_body_obj(script.as_ptr()), Code::Return);
                assert_eq!(interp.result_obj(), result.as_ptr());
                assert!(!crate::obj::has_string_rep(script.as_ptr()));
                assert_eq!(interp.result_bytes(), b"\xff\0TAIL");
            });
        }
    }

    #[test]
    fn uplevel_object_dispatch_preserves_release_specific_list_entry() {
        for version in tcl_dialect::TclVersion::ALL {
            leak_free(|interp| {
                interp.set_runtime_version(version);
                let command = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"set"));
                let name = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"x"));
                let value = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"VALUE"));
                let script = crate::obj::Owned::fresh(crate::list::new_list_obj(&[
                    command.as_ptr(),
                    name.as_ptr(),
                    value.as_ptr(),
                ]));
                assert_eq!(interp.eval_uplevel_obj(0, script.as_ptr()), Code::Ok);
                assert_eq!(
                    crate::obj::has_string_rep(script.as_ptr()),
                    version == tcl_dialect::TclVersion::V8_4
                );
                assert_eq!(interp.result_bytes(), b"VALUE");
            });
        }
    }

    #[test]
    fn eval_compiles_original_counted_opaque_string_source() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            leak_free(|interp| {
                interp.set_dialect_profile(crate::environment::profile_for_dialect(engine));
                let script = crate::obj::Owned::fresh(crate::obj::new_string_bytes(
                    b"set opaque {\xff\0TAIL}; set opaque",
                ));
                assert_eq!(interp.eval_body_obj(script.as_ptr()), Code::Ok, "{engine}");
                assert_eq!(interp.result_bytes(), b"\xff\0TAIL", "{engine}");
                assert_eq!(
                    crate::obj::bytes_of(script.as_ptr()),
                    b"set opaque {\xff\0TAIL}; set opaque"
                );
            });
        }
    }

    #[test]
    fn eval_runs_in_current_scope() {
        leak_free(|i| {
            assert_eq!(run(i, b"eval {set x 5}"), b"5");
            assert_eq!(run(i, b"set x"), b"5");
            // multi-arg eval concatenates.
            assert_eq!(run(i, b"eval set y 7"), b"7");
            assert_eq!(run(i, b"set y"), b"7");
            i.eval_str(b"unset x y");
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn uplevel_runs_in_callers_scope() {
        leak_free(|i| {
            // setit uplevels into its caller's frame to set a local there.
            run(i, b"proc setit {} {uplevel 1 {set local fromproc}}");
            run(i, b"proc caller {} {setit; return $local}");
            assert_eq!(run(i, b"caller"), b"fromproc");
            // uplevel #0 reaches the global scope.
            run(i, b"proc setglobal {} {uplevel #0 {set g globalval}}");
            run(i, b"setglobal");
            assert_eq!(run(i, b"set g"), b"globalval");
            i.eval_str(b"unset g");
        });
    }

    #[test]
    fn uplevel_bad_level() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"uplevel #5 {set x 1}"), Code::Error);
            assert_eq!(i.result_bytes(), b"bad level \"#5\"");
            // from the global scope, default level 1 has no caller.
            assert_eq!(i.eval_str(b"uplevel {set x 1}"), Code::Error);
            assert_eq!(i.result_bytes(), b"bad level \"1\"");
        });
    }
}
