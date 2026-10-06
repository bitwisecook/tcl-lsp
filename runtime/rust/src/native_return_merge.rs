// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original private Dictionary manufacture for admitted Return instructions.
use crate::{
    dict::PreparedNativeDictionary,
    obj::{self, Owned},
    return_options::NativeReturnOps,
    value_ops::{RuntimeAppendObjects, RuntimeAppendValue},
};
use tcl_cmd_core::{
    native_append,
    native_return_merge::{self, NativeReturnMergeObjects},
    return_options::ReturnOptionsOps,
    CmdError,
};
use tcl_runtime_api::native_return_literal::{NativeKnownWordLiteral, NativeReturnOptionsLiteral};
use tcl_syntax::value::ValueError;
impl NativeReturnMergeObjects for NativeReturnOps {
    type Dictionary = PreparedNativeDictionary;
    fn fresh_dictionary(&mut self) -> Result<Self::Dictionary, CmdError> {
        let original = Owned::fresh(obj::new_string_bytes(b""));
        Ok(PreparedNativeDictionary::prepare(
            Some(original.as_ptr()),
            self.string,
        )?)
    }
    fn put(
        &mut self,
        root: &mut Self::Dictionary,
        key: &Owned,
        value: &Owned,
    ) -> Result<(), CmdError> {
        Ok(root.set_member(key.as_ptr(), value.as_ptr())?)
    }
    fn get(&mut self, root: &Self::Dictionary, key: &[u8]) -> Result<Option<Owned>, CmdError> {
        Ok(root.with_member(self.new_string(key).as_ptr(), |value| {
            value.map(Owned::retain)
        })?)
    }
    fn remove(&mut self, root: &mut Self::Dictionary, key: &[u8]) -> Result<(), CmdError> {
        root.remove_member(self.new_string(key).as_ptr())?;
        Ok(())
    }
    fn dictionary_pairs(&mut self, value: &Owned) -> Result<Vec<(Owned, Owned)>, CmdError> {
        Ok(crate::dict::native_dict_pairs(value.as_ptr(), self.string)?
            .into_iter()
            .map(|(key, value)| (Owned::retain(key), Owned::retain(value)))
            .collect())
    }
    fn size(&mut self, root: &Self::Dictionary) -> Result<usize, CmdError> {
        Ok(crate::dict::native_dict_pairs(root.original(), self.string)?.len())
    }
    fn finish(&mut self, root: Self::Dictionary) -> Owned {
        root.into_value()
    }
}
fn known_word(ops: &NativeReturnOps, word: &NativeKnownWordLiteral) -> Result<Owned, CmdError> {
    let append = ops
        .dialect
        .native_object_append_protocol(None)
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native known word append",
        ))?
        .recipe();
    let objects = RuntimeAppendObjects {
        dialect: ops.dialect,
        binary_recipe: None,
    };
    let mut temporary = RuntimeAppendValue::fresh_string(b"");
    for piece in &word.pieces {
        temporary = native_append::append_counted_bytes(&objects, append, &temporary, piece)?;
    }
    let result = if word.composite {
        let receiver = RuntimeAppendValue::fresh_string(b"");
        native_append::append_object(&objects, append, Some(&receiver), &temporary)?.into_value()
    } else {
        temporary
    };
    Ok(Owned::retain(result.as_ptr()))
}
pub(crate) fn manufacture(recipe: &NativeReturnOptionsLiteral) -> Result<Owned, CmdError> {
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
    fn tag(value: *mut obj::TclObj) -> &'static str {
        match obj::native_object_snapshot(value).unwrap().cache {
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
            assert!(!obj::has_string_rep(root.as_ptr()));
            let pairs = crate::dict::native_dict_pairs(root.as_ptr(), literal.protocol).unwrap();
            assert_eq!(pairs.len(), 1);
            assert_eq!(
                crate::dict::native_object_bytes(pairs[0].0, literal.protocol).unwrap(),
                b"-custom"
            );
            assert_eq!(
                crate::dict::native_object_bytes(pairs[0].1, literal.protocol).unwrap(),
                if version == TclVersion::V8_5 {
                    b"B"
                } else {
                    b"C"
                }
            );
        }
    }

    #[test]
    fn private_return_options_factory_preserves_136_native_storage_windows() {
        let data =
            include_str!("../../../rust/tcl-registry/tests/data/return-private-storage136.tsv");
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
                let check = |window: &str,
                             root: Option<*mut obj::TclObj>,
                             words: &[Owned],
                             compared: &mut usize| {
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
                            words[arg as usize].as_ptr()
                        } else if arg == -1 {
                            root.unwrap()
                        } else {
                            panic!("separate retained member")
                        };
                        let snapshot = obj::native_object_snapshot(value).unwrap();
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
                            unsafe { (*value).ref_count as usize },
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
                let root = merged.options.into_native_unowned();
                check(
                    "merged-before-words-drop",
                    Some(root),
                    &originals,
                    &mut compared,
                );
                check("word-after-merge", Some(root), &originals, &mut compared);
                drop(originals);
                check("merged-after-words-drop", Some(root), &[], &mut compared);
                let pairs = crate::dict::native_dict_pairs(root, ops.string).unwrap();
                if let Some((_, value)) = pairs.into_iter().find(|(key, _)| {
                    crate::dict::native_object_bytes(*key, ops.string).unwrap() == b"-errorcode"
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
                    let snapshot = obj::native_object_snapshot(value).unwrap();
                    assert_eq!(tag(value), row[4]);
                    assert_eq!(snapshot.resident.is_some(), row[5] == "1");
                    assert_eq!(
                        snapshot.resident.unwrap().len() as i64,
                        row[6].parse::<i64>().unwrap()
                    );
                    assert_eq!(
                        unsafe { (*value).ref_count as usize },
                        row[7].parse::<usize>().unwrap()
                    );
                    compared += 1;
                }
                crate::interp::drop_fresh(root);
            }
        }
        assert_eq!(compared, 136);
    }
}
