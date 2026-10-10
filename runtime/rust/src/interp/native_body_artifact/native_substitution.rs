// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original C substitution templates retain their own program and cache.

use super::*;
use tcl_runtime_api::native_substitution::NativeSubstitutionFlags;

struct NativeSubstitutionArtifact {
    program: NativeBodyArtifact,
    arena: ArenaInstruction,
    flags: NativeSubstitutionFlags,
    empty: Option<usize>,
    // Subst ByteCode.localCachePtr adds a reference, independently of procPtr.
    _local_cache: Option<RetainedBodyLocalTable>,
}

extern "C" fn free_substitution(value: *mut TclObj) {
    // SAFETY: the exact descriptor owns one boxed Rc of this program.
    unsafe {
        drop(Box::from_raw(
            obj::internal_rep(value) as usize as *mut Rc<NativeSubstitutionArtifact>
        ));
    }
}
extern "C" fn duplicate_substitution(_original: *mut TclObj, _duplicate: *mut TclObj) {}
static SUBSTITUTION_TYPE: obj::TclObjType = obj::TclObjType {
    name: c"substcode".as_ptr(),
    free_int_rep_proc: Some(free_substitution),
    dup_int_rep_proc: Some(duplicate_substitution),
    update_string_proc: None,
    set_from_any_proc: None,
};
fn cache(original: *mut TclObj) -> Option<Rc<NativeSubstitutionArtifact>> {
    if !core::ptr::eq(obj::obj_type_ptr(original), &SUBSTITUTION_TYPE) {
        return None;
    }
    // SAFETY: exact descriptor identity authenticates the retained program.
    Some(
        unsafe {
            &*(obj::internal_rep(original) as usize as *const Rc<NativeSubstitutionArtifact>)
        }
        .clone(),
    )
}

impl Interp {
    fn prepare_original_c_substitution(
        &mut self,
        original: *mut TclObj,
        flags: NativeSubstitutionFlags,
    ) -> Result<Rc<NativeSubstitutionArtifact>, Code> {
        let namespace = self.current_ns.get();
        let mut stamp = self.native_body_stamp(namespace, None).ok_or_else(|| {
            self.report_cmd_error(unavailable("native substitution compiler context").into())
        })?;
        if !matches!(
            stamp.physical,
            tcl_dialect::TclVersion::V8_6
                | tcl_dialect::TclVersion::V9_0
                | tcl_dialect::TclVersion::V9_1
        ) || stamp.source_protocol != NativeStringProtocol::C(stamp.physical)
        {
            return Err(
                self.report_cmd_error(unavailable("native C substitution compiler release").into())
            );
        }
        if let Some(current) = cache(original)
            .filter(|current| current.flags == flags && current.program.stamp == stamp)
        {
            return Ok(current);
        }
        if cache(original).is_some() {
            obj::change_type(original, core::ptr::null(), 0);
        }
        let bytes = ValueOps::native_string_bytes(self, &original)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        stamp = self
            .native_body_stamp(self.current_ns.get(), None)
            .ok_or_else(|| {
                self.report_cmd_error(
                    unavailable("native substitution source callback context").into(),
                )
            })?;
        if !matches!(
            stamp.physical,
            tcl_dialect::TclVersion::V8_6
                | tcl_dialect::TclVersion::V9_0
                | tcl_dialect::TclVersion::V9_1
        ) || stamp.source_protocol != NativeStringProtocol::C(stamp.physical)
        {
            return Err(self.report_cmd_error(
                unavailable("native substitution source callback engine").into(),
            ));
        }
        let image = SourceImage::native(bytes.as_ref());
        let end = u32::try_from(image.len()).map_err(|_| {
            self.report_cmd_error(unavailable("native substitution source extent").into())
        })?;
        let region = Span::new(0, end);
        let policy = tcl_registry::substitution::TemplateParseErrors::for_dialect(
            self.native_invocation_dialect(),
        )
        .ok_or_else(|| self.report_cmd_error(unavailable("native substitution grammar").into()))?;
        let arena = ExecutablePartArena::decompose_template(
            image.clone(),
            region,
            flags.lexer_flags(),
            stamp.grammar,
            policy.variable_syntax(),
        )
        .map_err(|_| {
            self.report_cmd_error(unavailable("native substitution template geometry").into())
        })?;
        let (lvt, local_cache) = self.native_body_local_table(&stamp, &[])?;
        let mut builder = Builder {
            interp: self,
            image: image.clone(),
            context: Context {
                mode: NativeCompilationMode::BytecodeObject,
                frame: NativeCompilationFrame::ScriptCode,
                loop_depth: 0,
                catch_depth: Some(1),
            },
            stamp: stamp.clone(),
            lvt,
            literals: LiteralTable::new(),
            scripts: HashMap::new(),
            parse_failure: None,
            compilation_failure: None,
            private_objects: HashMap::new(),
            procedure: None,
        };
        let empty = arena
            .list(arena.root())
            .first()
            .is_none_or(|part| !matches!(part.part, ExecutablePart::Text(_)))
            .then(|| builder.literals.intern_bytes(b""));
        let arena = builder
            .arena_in(&arena, 0, true)
            .map_err(|error| builder.report_compilation_error(error))?;
        let literals = builder
            .materialize_body_literals(original)
            .map_err(|error| builder.report_compilation_error(error))?;
        if builder
            .interp
            .native_body_stamp(stamp.namespace, None)
            .as_ref()
            != Some(&stamp)
        {
            return Err(builder.interp.report_cmd_error(
                unavailable("native substitution compiler currency changed").into(),
            ));
        }
        let artifact = Rc::new(NativeSubstitutionArtifact {
            program: NativeBodyArtifact {
                stamp,
                owner: BodyContext::Script,
                image,
                region,
                scripts: builder.scripts,
                literals,
                locals: None,
                _borrowed_table: local_cache.as_ref().map(Rc::downgrade),
            },
            arena,
            flags,
            empty,
            _local_cache: local_cache,
        });
        obj::change_type(
            original,
            &SUBSTITUTION_TYPE,
            Box::into_raw(Box::new(Rc::clone(&artifact))) as usize as u64,
        );
        Ok(artifact)
    }

    pub(crate) fn substitute_original_c_template(
        &mut self,
        original: *mut TclObj,
        flags: NativeSubstitutionFlags,
    ) -> Code {
        let artifact = match self.prepare_original_c_substitution(original, flags) {
            Ok(artifact) => artifact,
            Err(code) => return code,
        };
        if !self.codegen_activation_enter() {
            return Code::Error;
        }
        if let Err(code) = self.reset_native_ensemble_rewrite(
            tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent::BytecodeEntry,
        ) {
            self.codegen_activation_leave(code);
            return code;
        }
        let result = self.execute_original_c_substitution(&artifact);
        let code = match result {
            Ok(value) => {
                self.set_result(value.as_ptr());
                Code::Ok
            }
            Err(code) => code,
        };
        self.codegen_activation_leave(code);
        code
    }
    fn execute_original_c_substitution(
        &mut self,
        artifact: &NativeSubstitutionArtifact,
    ) -> Result<obj::Owned, Code> {
        let arena = &artifact.arena;
        let mut values = Vec::new();
        if let Some(empty) = artifact.empty {
            values.push(obj::Owned::retain(
                artifact
                    .program
                    .literals
                    .original(empty)
                    .expect("original empty substitution literal"),
            ));
        }
        for (position, component) in arena
            .original
            .list(arena.original.root())
            .iter()
            .enumerate()
        {
            let exceptional = match &component.part {
                ExecutablePart::Command { .. } => true,
                ExecutablePart::Variable {
                    index: Some(index), ..
                } => arena
                    .original
                    .list(*index)
                    .iter()
                    .any(|part| matches!(part.part, ExecutablePart::Command { .. })),
                _ => false,
            };
            if exceptional {
                self.flush_substitution_values(&mut values)?;
            }
            let mut execution = BodyExecution::default();
            let value = self.body_arena_range(
                &artifact.program,
                arena,
                &mut execution,
                position..position + 1,
            );
            let value = match value {
                Ok(value) => value,
                Err(code) if exceptional && !self.host_refusal_pending() => match code {
                    Code::Error => return Err(code),
                    Code::Break => break,
                    Code::Continue => continue,
                    _ => obj::Owned::retain(self.result_obj()),
                },
                Err(code) => return Err(code),
            };
            values.push(value);
            if exceptional {
                self.flush_substitution_values(&mut values)?;
            }
        }
        self.flush_substitution_values(&mut values)?;
        values.pop().ok_or_else(|| {
            self.report_cmd_error(unavailable("native substitution result stack").into())
        })
    }
    fn flush_substitution_values(&mut self, values: &mut Vec<obj::Owned>) -> Result<(), Code> {
        while values.len() > 255 {
            let rest = values.split_off(255);
            let value = self.concatenate_body_values(std::mem::take(values))?;
            values.push(value);
            values.extend(rest);
        }
        if values.len() > 1 {
            let value = self.concatenate_body_values(std::mem::take(values))?;
            values.push(value);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::interpreter;
    use super::*;

    fn decode(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    fn primary(value: *mut TclObj) -> String {
        let kind = obj::obj_type_ptr(value);
        if kind.is_null() {
            return "none".into();
        }
        // SAFETY: the installed descriptor owns an immutable terminated name.
        unsafe { core::ffi::CStr::from_ptr((*kind).name) }
            .to_string_lossy()
            .into_owned()
    }
    #[test]
    fn original_compiled_substitution_matches_native_template_windows() {
        // Native proof: naming.substitution.counted-template-completions
        // docs/design/analysis/name-resolution-proofs/substitution-counted-template-completions.md
        // Native proof: naming.substitution.original-cache-fields
        // docs/design/analysis/name-resolution-proofs/substitution-original-cache-fields.md
        let inputs: Vec<_> = include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_substitution_owner/inputs.json"
        )
        .lines()
        .filter_map(|line| {
            line.trim()
                .trim_end_matches(',')
                .strip_prefix('"')?
                .strip_suffix('"')
        })
        .collect();
        assert_eq!(inputs.len(), 10);
        let mut compared = 0;
        for (engine, rows) in [
            (
                "tcl8.6",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/8.6.18/stdout.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/9.0.4/stdout.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/9.1.0/stdout.tsv"
                ),
            ),
        ] {
            for (source, input) in inputs.iter().enumerate() {
                let mut interp = interpreter(engine);
                let original = obj::Owned::fresh(new_string(&decode(input)));
                for flags in (0..=7).rev() {
                    for label in ["ROOT", "ROOT_REPEAT"] {
                        assert_eq!(
                            interp.eval_str(b"set x X; set y Y; set a(k) K; set k k"),
                            Code::Ok
                        );
                        let fields: Vec<_> = rows
                            .lines()
                            .find(|row| row.starts_with(&format!("R|{label}|{source}|{flags}|")))
                            .unwrap()
                            .split('|')
                            .collect();
                        let mut argv = vec![obj::Owned::fresh(new_string(b"subst"))];
                        for (bit, option) in [
                            (4, b"-nobackslashes".as_slice()),
                            (1, b"-nocommands"),
                            (2, b"-novariables"),
                        ] {
                            if flags & bit == 0 {
                                argv.push(obj::Owned::fresh(new_string(option)));
                            }
                        }
                        let mut pointers: Vec<_> = argv.iter().map(obj::Owned::as_ptr).collect();
                        pointers.push(original.as_ptr());
                        let code = interp.dispatch(&pointers);
                        let context = format!("{engine}/{source}/{flags}/{label}");
                        assert_eq!(
                            code.as_int(),
                            fields[4].parse::<i64>().unwrap(),
                            "{context}"
                        );
                        assert_eq!(primary(original.as_ptr()), fields[5], "{context}");
                        assert_eq!(
                            usize::from(obj::has_string_rep(original.as_ptr())),
                            fields[6].parse::<usize>().unwrap(),
                            "{context}"
                        );
                        assert_eq!(primary(interp.result_obj()), fields[7], "{context}");
                        assert_eq!(
                            usize::from(obj::has_string_rep(interp.result_obj())),
                            fields[8].parse::<usize>().unwrap(),
                            "{context}"
                        );
                        assert_eq!(interp.result_bytes(), decode(fields[9]), "{context}");
                        assert!(super::super::cache(original.as_ptr()).is_none());
                        let current = cache(original.as_ptr()).unwrap();
                        assert_eq!(current.flags.bits(), flags);
                        assert!(current._local_cache.is_none());
                        compared += 1;
                    }
                }
            }
        }
        assert_eq!(compared, 480);
    }
}

#[cfg(test)]
mod compiler_currency_tests {
    use super::*;

    #[test]
    fn substitution_does_not_recover_an_unknown_original_compiler_epoch() {
        let mut interp = super::super::tests::interpreter("tcl8.6");
        let original = obj::Owned::fresh(obj::new_string_bytes(b"literal"));
        assert!(interp.native_body_stamp(GLOBAL, None).is_some());
        interp
            .namespaces
            .borrow_mut()
            .note_native_compiler_mutation(
                None,
                tcl_registry::native_procedure::NativeCompilerCacheMutation::CommandToken {
                    hook: tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Unknown,
                },
            );
        assert!(interp.native_compiler_cache_epochs(GLOBAL).is_none());
        assert!(interp
            .prepare_original_c_substitution(
                original.as_ptr(),
                NativeSubstitutionFlags::new(true, true, true)
            )
            .is_err());
        assert!(interp.host_refusal_pending());
        assert!(!core::ptr::eq(
            obj::obj_type_ptr(original.as_ptr()),
            &SUBSTITUTION_TYPE
        ));
        interp.set_runtime_version(tcl_dialect::TclVersion::V9_1);
        assert!(interp.native_compiler_cache_epochs(GLOBAL).is_none());
    }
}
