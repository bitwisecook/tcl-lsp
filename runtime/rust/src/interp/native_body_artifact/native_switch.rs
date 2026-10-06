// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original switch bodies use this unit's actual local and literal ownership.
use super::*;
use tcl_registry::native_switch_compilation::{NativeSwitchMatch, NativeSwitchMode};
impl Builder<'_> {
    pub(super) fn prepare_body_switch(
        &mut self,
        switch: &mut SwitchOperation,
        depth: u32,
    ) -> Result<(), ValueError> {
        if let tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::LiteralExpansion {value,..} = &switch.recipe.subject {
            switch.subject_literal = Some(self.literals.intern_bytes(value));
        }
        for (index, arm) in switch.recipe.arms.iter().enumerate() {
            let pattern = match &arm.matcher {
                NativeSwitchMatch::Equal(bytes)
                | NativeSwitchMatch::Glob(bytes)
                | NativeSwitchMatch::Regexp(bytes) => Some(self.literals.intern_bytes(bytes)),
                NativeSwitchMatch::Always
                    if !(switch.recipe.terminal_default
                        && index + 1 == switch.recipe.arms.len()) =>
                {
                    Some(self.literals.intern_bytes(b"1"))
                }
                _ => None,
            };
            switch.patterns.push(pattern);
            if arm.compile_body {
                self.script(arm.body.expect("real compiled switch arm"), depth + 1)?;
            }
        }
        if !switch.recipe.terminal_default {
            switch.empty = Some(self.literals.intern_bytes(b""));
        }
        Ok(())
    }
}
impl Interp {
    pub(super) fn execute_body_switch(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        switch: &SwitchOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
        let original = match &switch.recipe.subject {
            NativeCompilerWordOperand::Original(index) => {
                self.body_word(artifact, &command.words[*index], execution)?
            }
            NativeCompilerWordOperand::LiteralExpansion { .. } => obj::Owned::retain(
                artifact
                    .literals
                    .original(switch.subject_literal.expect("expanded subject"))
                    .expect("registered subject"),
            ),
        };
        let mut selected = None;
        let mut transformed = None;
        match switch.recipe.mode {
            NativeSwitchMode::Exact => {
                let bytes = ValueOps::native_string_bytes(self, &original.as_ptr())
                    .map_err(|error| self.report_cmd_error(error.into()))?;
                let key = if switch.recipe.nocase {
                    let lowered = tcl_syntax::native_glob::lower_c_string_bytes(
                        switch.recipe.version,
                        &bytes,
                    );
                    if obj::is_shared(original.as_ptr()) {
                        let copy = obj::Owned::fresh(new_string(&lowered));
                        let materialization = self
                            .native_invocation_dialect()
                            .native_string_materialization(None)
                            .ok_or(ValueError::CommandProtocolUnavailable(
                                "compiled string lower issuer",
                            ))
                            .map_err(|error| self.report_cmd_error(error.into()))?;
                        obj::retain_native_string_representation(copy.as_ptr(), materialization)
                            .map_err(|error| self.report_cmd_error(error.into()))?;
                        transformed = Some(copy);
                    } else {
                        let canonical = obj::has_canonical_empty_string(original.as_ptr());
                        obj::change_type(original.as_ptr(), core::ptr::null(), 0);
                        obj::invalidate_string(original.as_ptr());
                        // SAFETY: the original live, unshared header owns the replaced string.
                        unsafe {
                            obj::set_native_updater_string_rep(
                                original.as_ptr(),
                                &lowered,
                                canonical && lowered.is_empty(),
                            )
                        };
                    }
                    lowered
                } else {
                    tcl_core_types::c_string_extent(&bytes).to_vec()
                };
                for (index, arm) in switch.recipe.arms.iter().enumerate() {
                    if switch.recipe.terminal_default && index + 1 == switch.recipe.arms.len() {
                        selected = Some(arm.target);
                        break;
                    }
                    let pattern = if switch.recipe.nocase {
                        tcl_syntax::native_glob::lower_c_string_bytes(
                            switch.recipe.version,
                            &arm.pattern,
                        )
                    } else {
                        tcl_core_types::c_string_extent(&arm.pattern).to_vec()
                    };
                    if key == pattern {
                        selected = Some(arm.target);
                        break;
                    }
                }
            }
            NativeSwitchMode::Integer => {
                let number = ValueOps::as_int(self, &original.as_ptr())
                    .map_err(|error| self.report_cmd_error(error.into()))?;
                for (index, arm) in switch.recipe.arms.iter().enumerate() {
                    if (switch.recipe.terminal_default && index + 1 == switch.recipe.arms.len())
                        || arm.integer == Some(number)
                    {
                        selected = Some(arm.target);
                        break;
                    }
                }
            }
            _ => {
                for (index, arm) in switch.recipe.arms.iter().enumerate() {
                    let matched = match &arm.matcher {
                        NativeSwitchMatch::Always => {
                            if let Some(literal) = switch.patterns[index] {
                                ValueOps::as_bool(
                                    self,
                                    &artifact
                                        .literals
                                        .original(literal)
                                        .expect("registered empty-regexp Boolean"),
                                )
                                .map_err(|error| self.report_cmd_error(error.into()))?
                            } else {
                                true
                            }
                        }
                        NativeSwitchMatch::Equal(_) => {
                            let pattern = artifact
                                .literals
                                .original(switch.patterns[index].expect("compiled pattern"))
                                .unwrap();
                            tcl_cmd_core::switch::compiled_equal(
                                self,
                                &pattern,
                                &original.as_ptr(),
                                switch.recipe.version,
                            )
                            .map_err(|error| self.report_cmd_error(error))?
                        }
                        NativeSwitchMatch::Glob(_) => {
                            let pattern = artifact
                                .literals
                                .original(switch.patterns[index].expect("compiled pattern"))
                                .unwrap();
                            tcl_cmd_core::switch::compiled_glob(
                                self,
                                &pattern,
                                &original.as_ptr(),
                                switch.recipe.version,
                                switch.recipe.nocase,
                            )
                            .map_err(|error| self.report_cmd_error(error))?
                        }
                        NativeSwitchMatch::Regexp(_) => {
                            let pattern = artifact
                                .literals
                                .original(switch.patterns[index].expect("compiled pattern"))
                                .unwrap();
                            let mut flags =
                                tcl_cmd_core::regex::RegexFlags::for_release(switch.recipe.version);
                            flags.nocase = switch.recipe.nocase;
                            tcl_cmd_core::regex::compiled_match_original::<
                                Interp,
                                crate::cmd_regex::AreEngine,
                            >(
                                self,
                                &pattern,
                                &original.as_ptr(),
                                flags,
                                switch.recipe.version,
                            )
                            .map_err(|error| self.report_cmd_error(error.into_cmd_error()))?
                        }
                        _ => unreachable!("chained switch matcher"),
                    };
                    if matched {
                        selected = Some(arm.target);
                        break;
                    }
                }
            }
        }
        // The native POP/JUMP_TABLE retires the subject before entering body.
        drop(transformed);
        drop(original);
        if let Some(index) = selected {
            Ok(self.execute_body_region(
                artifact,
                switch.recipe.arms[index]
                    .body
                    .expect("real fallthrough target"),
                execution,
            ))
        } else {
            self.set_result(
                artifact
                    .literals
                    .original(switch.empty.expect("implicit default"))
                    .expect("registered empty"),
            );
            Ok(Code::Ok)
        }
    }
}

impl tcl_cmd_core::switch::NativeCompiledSwitchObjects for Interp {
    fn compiled_binary_bytes(
        &mut self,
        value: &*mut TclObj,
        version: tcl_dialect::TclVersion,
    ) -> Result<Rc<[u8]>, tcl_cmd_core::CmdError> {
        let recipe = self
            .native_invocation_dialect()
            .byte_array_string_recipe(None)
            .filter(|recipe| {
                recipe.protocol().tcl_version() == Some(version)
                    && version < tcl_dialect::TclVersion::V9_0
            })
            .ok_or(ValueError::CommandProtocolUnavailable(
                "compiled C8 binary conversion",
            ))?;
        obj::check_native_liveness(*value)?;
        crate::bytearray::native_binary_bytes(
            *value,
            tcl_registry::native_binary_value::NativeBinaryByteConversion::Narrow(
                version.string_character_model(),
            ),
            true,
            recipe,
        )
        .map(Rc::from)
        .map_err(|error| {
            tcl_cmd_core::CmdError::with_error_code_bytes(
                error.message().as_bytes(),
                b"TCL VALUE BYTES",
            )
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn bytes(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    #[test]
    fn registered_switch_artifacts_match_56_native_controls() {
        let mut compared = 0;
        for row in
            include_str!("../../../../../rust/tcl-registry/tests/data/registered-switch56.tsv")
                .lines()
        {
            let fields = row.split('\t').collect::<Vec<_>>();
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(match fields[0] {
                    "8.5.19" => "tcl8.5",
                    "8.6.18" => "tcl8.6",
                    "9.0.4" => "tcl9.0",
                    "9.1.0" => "tcl9.1",
                    _ => panic!("native version"),
                }),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            let mut source = b"proc p x {".to_vec();
            source.extend(bytes(fields[2]));
            source.extend(b"}; p A");
            let code = interp.eval_str(&source);
            assert_eq!(
                code,
                Code::from_int(fields[3].parse().unwrap()),
                "{}/{}: {:?}",
                fields[0],
                fields[1],
                interp.result_bytes()
            );
            assert_eq!(
                interp.result_bytes(),
                bytes(fields[4]),
                "{}/{}",
                fields[0],
                fields[1]
            );
            compared += 1;
        }
        assert_eq!(compared, 56);
    }
    #[test]
    fn registered_switch_artifacts_match_56_native_branch_controls() {
        let mut compared = 0;
        for row in include_str!(
            "../../../../../rust/tcl-registry/tests/data/registered-switch-branches56.tsv"
        )
        .lines()
        {
            let fields = row.split('\t').collect::<Vec<_>>();
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(match fields[0] {
                    "8.5.19" => "tcl8.5",
                    "8.6.18" => "tcl8.6",
                    "9.0.4" => "tcl9.0",
                    "9.1.0" => "tcl9.1",
                    _ => panic!("native version"),
                }),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            let mut source = b"proc p x {".to_vec();
            source.extend(bytes(fields[2]));
            source.extend(b"}; set arg [binary format H* ");
            source.extend(fields[3].as_bytes());
            source.extend(b"]; p $arg");
            let code = interp.eval_str(&source);
            assert_eq!(
                code,
                Code::from_int(fields[4].parse().unwrap()),
                "{}/{}: {:?}",
                fields[0],
                fields[1],
                interp.result_bytes()
            );
            assert_eq!(
                interp.result_bytes(),
                bytes(fields[5]),
                "{}/{}",
                fields[0],
                fields[1]
            );
            compared += 1;
        }
        assert_eq!(compared, 56);
    }
}
