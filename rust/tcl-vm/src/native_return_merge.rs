// SPDX-License-Identifier: AGPL-3.0-or-later
//! Manufacture private Return literals through original native append and merge.
use crate::{
    Value,
    return_options::NativeReturnOps,
    value::{PreparedNativeDictionary, VmAppendObjects},
};
use tcl_cmd_core::{
    CmdError,
    native_append::{self, NativeAppendObjects},
    native_return_merge::{self, NativeReturnMergeObjects},
    return_options::ReturnOptionsOps,
};
use tcl_runtime_api::native_return_literal::{NativeKnownWordLiteral, NativeReturnOptionsLiteral};
use tcl_syntax::value::ValueError;

impl NativeReturnMergeObjects for NativeReturnOps {
    type Dictionary = PreparedNativeDictionary;
    fn fresh_dictionary(&mut self) -> Result<Self::Dictionary, CmdError> {
        let original = Value::new_native_string_bytes(&b""[..]);
        Ok(original.prepare_native_dictionary(self.string)?)
    }
    fn put(
        &mut self,
        root: &mut Self::Dictionary,
        key: &Value,
        value: &Value,
    ) -> Result<(), CmdError> {
        Ok(root.set_member(key.clone(), value.clone())?)
    }
    fn get(&mut self, root: &Self::Dictionary, key: &[u8]) -> Result<Option<Value>, CmdError> {
        Ok(root.with_member(&self.new_string(key), |value| value.cloned())?)
    }
    fn remove(&mut self, root: &mut Self::Dictionary, key: &[u8]) -> Result<(), CmdError> {
        root.remove_member(&self.new_string(key))?;
        Ok(())
    }
    fn dictionary_pairs(&mut self, value: &Value) -> Result<Vec<(Value, Value)>, CmdError> {
        Ok(value.native_object_dict_pairs(self.string)?)
    }
    fn size(&mut self, root: &Self::Dictionary) -> Result<usize, CmdError> {
        root.original()
            .with_cached_dictionary_representation(|pairs, _| pairs.len())
            .ok_or_else(|| {
                ValueError::CommandProtocolUnavailable("merged Dictionary backing").into()
            })
    }
    fn finish(&mut self, root: Self::Dictionary) -> Value {
        root.into_value()
    }
}
fn known_word(ops: &NativeReturnOps, word: &NativeKnownWordLiteral) -> Result<Value, CmdError> {
    let append = ops
        .dialect
        .native_object_append_protocol(None)
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native known word append",
        ))?
        .recipe();
    let objects = VmAppendObjects;
    let mut temporary = objects.new_string(std::rc::Rc::from(&b""[..]));
    for piece in &word.pieces {
        temporary = native_append::append_counted_bytes(&objects, append, &temporary, piece)?;
    }
    if word.composite {
        let receiver = objects.new_string(std::rc::Rc::from(&b""[..]));
        Ok(
            native_append::append_object(&objects, append, Some(&receiver), &temporary)?
                .into_value(),
        )
    } else {
        Ok(temporary)
    }
}
pub(crate) fn manufacture(recipe: &NativeReturnOptionsLiteral) -> Result<Value, CmdError> {
    let version = recipe
        .protocol
        .tcl_version()
        .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
        .ok_or(ValueError::CommandProtocolUnavailable(
            "private Return compiler origin",
        ))?;
    let mut ops = NativeReturnOps::for_c(version)?;
    let protocol =
        ops.dialect
            .return_options_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native Return merger",
            ))?;
    let originals = recipe
        .words
        .iter()
        .map(|word| known_word(&ops, word))
        .collect::<Result<Vec<_>, _>>()?;
    let merged = native_return_merge::merge(&mut ops, protocol, &originals)?;
    if (merged.code, merged.level, merged.size) != (recipe.code, recipe.level, recipe.size) {
        return Err(ValueError::CommandProtocolUnavailable(
            "native Return literal controls mismatch",
        )
        .into());
    }
    Ok(merged.options)
}

/// Apply the reached original merged header without regenerating option words.
pub(crate) fn process(
    vm: &mut crate::Vm,
    purpose: tcl_registry::native_return_options::NativeReturnOptionsApplication,
    code: i32,
    level: i64,
    options: Value,
    result: Value,
) -> tcl_core_types::Completion<Value> {
    use tcl_core_types::{Code, Completion, CompletionOptionOrigin};
    let Some(application) = vm
        .native_invocation_dialect()
        .native_return_options_application(purpose)
        .filter(|recipe| recipe.retains_merged_header() && recipe.accepts_control(code, level))
    else {
        return crate::command::completion_from_cmd_error(
            vm,
            ValueError::CommandProtocolUnavailable("original merged return instruction").into(),
        );
    };
    if let Err(error) = options.native_object_dict_pairs(application.strings()) {
        return crate::command::completion_from_cmd_error(vm, error.into());
    }
    vm.retain_native_return_options(&options);
    let mut ops = match crate::return_options::NativeReturnOps::selected(vm) {
        Ok((ops, _)) => ops,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    if code == 1 {
        vm.clear_return_error_info();
        vm.observe_native_error_result(&result);
        let error = options
            .with_cached_dictionary_representation(|pairs, _| {
                for (key, value) in pairs {
                    let name = key
                        .native_string_bytes(application.strings())
                        .map_err(|error| {
                            CmdError::from(ValueError::NativeStringAccess(
                                tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                            ))
                        })?;
                    match name.as_ref() {
                        b"-errorinfo" => {
                            let bytes = ops.bytes(value)?;
                            if !bytes.is_empty() {
                                vm.seed_error_info_original(value, &bytes);
                            }
                        }
                        b"-errorstack" if vm.supports_error_stack() => vm.seed_error_stack(value),
                        b"-errorline" => {
                            if let Some(line) = ops.integer_probe(value, false)? {
                                vm.set_error_line(line as i32 as u32);
                            }
                        }
                        _ => {}
                    }
                }
                let original = pairs
                    .iter()
                    .find(|(key, _)| {
                        key.resident_string_bytes()
                            .is_some_and(|bytes| bytes.as_ref() == b"-errorcode")
                    })
                    .map(|(_, value)| value);
                let _ = vm.retain_return_error_code(original, true);
                Ok::<(), CmdError>(())
            })
            .expect("reached original Dictionary");
        if let Err(error) = error {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        if level == 0 {
            vm.mark_native_error_copy();
        }
    }
    vm.set_native_c_return_state(code, level);
    let transport = options.native_lifetime_lease().into_value();
    let mut completion = Completion::new(
        if level == 0 {
            Code::from_int(code)
        } else {
            Code::Return
        },
        result,
        transport,
    );
    completion.option_origin = CompletionOptionOrigin::MergedReturnOptions { code, level };
    completion
}

pub(crate) fn process_stack(
    vm: &mut crate::Vm,
    original: Value,
    result: Value,
) -> tcl_core_types::Completion<Value> {
    let (mut ops, protocol) = match crate::return_options::NativeReturnOps::selected(vm) {
        Ok(selected) => selected,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let merged = match native_return_merge::merge_stack(&mut ops, protocol, &original) {
        Ok(merged) => merged,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    process(
        vm,
        tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate,
        merged.code,
        i64::from(merged.level),
        merged.options,
        result,
    )
}

#[cfg(test)]
mod physical_tests {
    use super::*;
    use tcl_dialect::TclVersion;
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    fn version(name: &str) -> TclVersion {
        match name {
            "8.5.19" => TclVersion::V8_5,
            "8.6.18" => TclVersion::V8_6,
            "9.0.4" => TclVersion::V9_0,
            "9.1.0" => TclVersion::V9_1,
            _ => panic!("native version"),
        }
    }
    fn tag(value: &Value) -> &'static str {
        match value.native_object_snapshot().cache {
            Cache::String { .. } => "string",
            Cache::Numeric(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                tcl_syntax::number::Number::Int(_),
            )) => "int",
            Cache::Index { .. } => "index",
            Cache::List { .. } => "list",
            Cache::Dictionary { .. } => "dict",
            Cache::None => "none",
            _ => panic!("unexpected original cache"),
        }
    }
    #[test]
    fn private_return_merger_preserves_four_native_nested_option_results() {
        for name in ["8.5.19", "8.6.18", "9.0.4", "9.1.0"] {
            let version = version(name);
            let profile =
                tcl_dialect::DialectProfile::find(&format!("tcl{}", version.version_string()))
                    .unwrap();
            let source =
                b"return -level 0 -options {-custom A -options {-custom B} -custom C} BODY";
            let image = tcl_lexer::SourceImage::native(source.as_slice());
            let script = tcl_lexer::native_script_words_in(
                image.clone(),
                tcl_lexer::Span::new(0, image.len() as u32),
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            let capture = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                &script.commands[0].words,
                tcl_syntax::native_string::NativeStringProtocol::C(version),
            )
            .unwrap();
            let literal = tcl_runtime_api::native_return_literal::NativeReturnOptionsLiteral {
                protocol: tcl_syntax::native_string::NativeStringProtocol::C(version),
                words: (1..5)
                    .map(|index| capture.known_word_literal(index).unwrap())
                    .collect(),
                code: 0,
                level: 0,
                size: 1,
            };
            let root = manufacture(&literal).unwrap();
            assert!(root.resident_string_bytes().is_none());
            root.with_cached_dictionary_representation(|pairs, _| {
                assert_eq!(pairs.len(), 1);
                assert_eq!(
                    pairs[0].0.resident_string_bytes().unwrap().as_ref(),
                    b"-custom"
                );
                assert_eq!(
                    pairs[0].1.resident_string_bytes().unwrap().as_ref(),
                    if version == TclVersion::V8_5 {
                        b"B"
                    } else {
                        b"C"
                    }
                );
            })
            .unwrap();
        }
    }

    #[test]
    fn private_return_options_factory_preserves_136_native_storage_windows() {
        let data = include_str!("../../tcl-registry/tests/data/return-private-storage136.tsv");
        let sources = [
            b"return".as_slice(),
            b"return -level 0",
            b"return -errorcode {A B}",
            b"return -level 0 -code error -options {-custom kept -errorcode CUSTOM}",
        ];
        let mut compared = 0;
        for name in ["8.5.19", "8.6.18", "9.0.4", "9.1.0"] {
            let version = version(name);
            let profile =
                tcl_dialect::DialectProfile::find(&format!("tcl{}", version.version_string()))
                    .unwrap();
            for (case, source) in sources.iter().enumerate() {
                let image = tcl_lexer::SourceImage::native(*source);
                let script = tcl_lexer::native_script_words_in(
                    image.clone(),
                    tcl_lexer::Span::new(0, image.len() as u32),
                    tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                )
                .unwrap();
                let capture = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                    &script.commands[0].words,
                    tcl_syntax::native_string::NativeStringProtocol::C(version),
                )
                .unwrap();
                let mut ops = NativeReturnOps::for_c(version).unwrap();
                let originals = (1..capture.original_words().len())
                    .map(|index| {
                        known_word(&ops, &capture.known_word_literal(index).unwrap()).unwrap()
                    })
                    .collect::<Vec<_>>();
                let check =
                    |window: &str, root: Option<&Value>, words: &[Value], compared: &mut usize| {
                        for line in data.lines() {
                            let row = line.split('\t').collect::<Vec<_>>();
                            if row[0] != name
                                || row[1].parse::<usize>().unwrap() != case
                                || row[2] != window
                            {
                                continue;
                            }
                            let arg = row[3].parse::<i32>().unwrap();
                            let value = if arg >= 0 {
                                &words[arg as usize]
                            } else if arg == -1 {
                                root.unwrap()
                            } else {
                                panic!("separate retained member")
                            };
                            let snapshot = value.native_object_snapshot();
                            assert_eq!(tag(value), row[4], "{name}/{case}/{window}/{arg}");
                            assert_eq!(
                                snapshot.resident.is_some(),
                                row[5] == "1",
                                "{name}/{case}/{window}/{arg}"
                            );
                            assert_eq!(
                                snapshot
                                    .resident
                                    .as_ref()
                                    .map_or(-1, |bytes| bytes.len() as i64),
                                row[6].parse::<i64>().unwrap(),
                                "{name}/{case}/{window}/{arg}"
                            );
                            assert_eq!(
                                value.native_object_reference_count(),
                                row[7].parse::<usize>().unwrap(),
                                "{name}/{case}/{window}/{arg}"
                            );
                            *compared += 1;
                        }
                    };
                check("known-word", None, &originals, &mut compared);
                let protocol = ops.dialect.return_options_protocol().unwrap();
                let merged = native_return_merge::merge(&mut ops, protocol, &originals).unwrap();
                let control = data
                    .lines()
                    .find(|line| {
                        let fields = line.split('\t').collect::<Vec<_>>();
                        fields[0] == name
                            && fields[1].parse::<usize>().unwrap() == case
                            && fields[2] == "controls"
                    })
                    .unwrap()
                    .split('\t')
                    .collect::<Vec<_>>();
                assert_eq!(merged.code, control[3].parse::<i32>().unwrap());
                assert_eq!(merged.level, control[4].parse::<i32>().unwrap());
                assert_eq!(merged.size, control[5].parse::<usize>().unwrap());
                compared += 1;
                let root = merged.options.into_native_unowned_lifetime();
                check(
                    "merged-before-words-drop",
                    Some(&root),
                    &originals,
                    &mut compared,
                );
                check("word-after-merge", Some(&root), &originals, &mut compared);
                drop(originals);
                check("merged-after-words-drop", Some(&root), &[], &mut compared);
                root.with_cached_dictionary_representation(|pairs, _| {
                    if let Some((_, value)) = pairs.iter().find(|(key, _)| {
                        key.resident_string_bytes()
                            .is_some_and(|bytes| bytes.as_ref() == b"-errorcode")
                    }) {
                        let row = data
                            .lines()
                            .find(|line| {
                                let fields = line.split('\t').collect::<Vec<_>>();
                                fields[0] == name
                                    && fields[1].parse::<usize>().unwrap() == case
                                    && fields[2] == "retained-errorcode"
                            })
                            .unwrap()
                            .split('\t')
                            .collect::<Vec<_>>();
                        let snapshot = value.native_object_snapshot();
                        assert_eq!(tag(value), row[4]);
                        assert_eq!(snapshot.resident.is_some(), row[5] == "1");
                        assert_eq!(
                            snapshot.resident.unwrap().len() as i64,
                            row[6].parse::<i64>().unwrap()
                        );
                        assert_eq!(
                            value.native_object_reference_count(),
                            row[7].parse::<usize>().unwrap()
                        );
                        compared += 1;
                    }
                })
                .unwrap();
            }
        }
        assert_eq!(compared, 136);
    }
}

#[cfg(test)]
mod execution_tests {
    use super::*;
    use std::rc::Rc;
    use tcl_core_types::Code;
    fn bytes(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    #[test]
    fn registered_return_instructions_match_44_native_controls() {
        let mut compared = 0;
        for line in include_str!("../../tcl-registry/tests/data/registered-return44.tsv").lines() {
            let fields = line.split('\t').collect::<Vec<_>>();
            let profile = tcl_dialect::DialectProfile::find(match fields[0] {
                "8.5.19" => "tcl8.5",
                "8.6.18" => "tcl8.6",
                "9.0.4" => "tcl9.0",
                "9.1.0" => "tcl9.1",
                _ => panic!("original C version"),
            })
            .unwrap();
            let mut vm = crate::Vm::with_native_core(
                Box::new(Vec::<u8>::new()),
                Rc::new(crate::host_native::NativeHost::new()),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let body = Value::new_native_string_bytes(bytes(fields[2]));
            assert_eq!(
                vm.invoke_command(
                    "proc",
                    &[
                        Value::new_native_string_bytes(b"p".as_slice()),
                        Value::new_native_string_bytes(b"opts msg code".as_slice()),
                        body
                    ]
                )
                .code,
                Code::Ok
            );
            let result = vm.invoke_command(
                "p",
                &[
                    Value::new_native_string_bytes(b"-custom DYNAMIC".as_slice()),
                    Value::new_native_string_bytes(b"MSG".as_slice()),
                    Value::new_native_string_bytes(b"error".as_slice()),
                ],
            );
            assert_eq!(
                result.code,
                Code::from_int(fields[3].parse().unwrap()),
                "{}/{}: {:?}",
                fields[0],
                fields[1],
                result
            );
            assert_eq!(
                result
                    .result
                    .native_string_bytes(
                        vm.native_invocation_dialect()
                            .native_string_protocol()
                            .unwrap()
                    )
                    .unwrap()
                    .as_ref(),
                bytes(fields[4]),
                "{}/{}",
                fields[0],
                fields[1]
            );
            compared += 1;
        }
        assert_eq!(compared, 44);
    }
}
