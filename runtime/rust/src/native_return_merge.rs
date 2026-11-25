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

//! Original private Dictionary manufacture for admitted Return instructions.
use crate::{
    dict::PreparedNativeDictionary,
    obj::{self, Owned},
    return_options::NativeReturnOps,
    value_ops::{RuntimeAppendObjects, RuntimeAppendValue},
};
use tcl_cmd_core::native_dictionary::NativeDictionaryObjects;
use tcl_cmd_core::{
    native_return_merge::{self, NativeReturnMergeObjects},
    return_options::ReturnOptionsOps,
    CmdError,
};
use tcl_runtime_api::native_return_literal::NativeReturnOptionsLiteral;
impl NativeReturnMergeObjects for NativeReturnOps {
    type Dictionary = PreparedNativeDictionary;
    fn fresh_dictionary(&mut self) -> Result<Self::Dictionary, CmdError> {
        let original = Owned::fresh(obj::new_string_bytes(b""));
        if let Some(objects) = &self.original {
            return objects.prepare(Some(&RuntimeAppendValue::borrowed(original.as_ptr())));
        }
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
        if let Some(objects) = &self.original {
            return objects.set_member(
                root,
                &RuntimeAppendValue::borrowed(key.as_ptr()),
                RuntimeAppendValue::borrowed(value.as_ptr()),
            );
        }
        Ok(root.set_member(key.as_ptr(), value.as_ptr())?)
    }
    fn get(&mut self, root: &Self::Dictionary, key: &[u8]) -> Result<Option<Owned>, CmdError> {
        if let Some(objects) = self.original.clone() {
            let key = self.new_string(key);
            return objects.with_member(
                root,
                &RuntimeAppendValue::borrowed(key.as_ptr()),
                |value| Ok(value.map(|value| Owned::retain(value.as_ptr()))),
            );
        }
        Ok(root.with_member(self.new_string(key).as_ptr(), |value| {
            value.map(Owned::retain)
        })?)
    }
    fn remove(&mut self, root: &mut Self::Dictionary, key: &[u8]) -> Result<(), CmdError> {
        if let Some(objects) = self.original.clone() {
            let key = self.new_string(key);
            return objects.remove_member(root, &RuntimeAppendValue::borrowed(key.as_ptr()));
        }
        root.remove_member(self.new_string(key).as_ptr())?;
        Ok(())
    }
    fn dictionary_pairs(&mut self, value: &Owned) -> Result<Vec<(Owned, Owned)>, CmdError> {
        if let Some(original) = &self.original {
            return original.original_pairs(value.as_ptr()).map(|pairs| {
                pairs
                    .into_iter()
                    .map(|(key, value)| (Owned::retain(key), Owned::retain(value)))
                    .collect()
            });
        }
        Ok(crate::dict::native_dict_pairs(value.as_ptr(), self.string)?
            .into_iter()
            .map(|(key, value)| (Owned::retain(key), Owned::retain(value)))
            .collect())
    }
    fn size(&mut self, root: &Self::Dictionary) -> Result<usize, CmdError> {
        if let Some(original) = &self.original {
            return original
                .original_pairs(root.original())
                .map(|pairs| pairs.len());
        }
        Ok(crate::dict::native_dict_pairs(root.original(), self.string)?.len())
    }
    fn finish(&mut self, root: Self::Dictionary) -> Owned {
        root.into_value()
    }
}
pub(crate) fn manufacture(recipe: &NativeReturnOptionsLiteral) -> Result<Owned, CmdError> {
    native_return_merge::manufacture(
        recipe,
        |version| {
            let ops = NativeReturnOps::for_c(version)?;
            let objects = RuntimeAppendObjects {
                dialect: ops.dialect,
                binary_recipe: None,
            };
            Ok((ops, objects))
        },
        |result| Owned::retain(result.as_ptr()),
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
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
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
                retains_controls: false,
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
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
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
                let objects = RuntimeAppendObjects {
                    dialect: ops.dialect,
                    binary_recipe: None,
                };
                let originals = (1..capture.original_words().len())
                    .map(|index| {
                        let value = native_return_merge::materialise_known_word(
                            &objects,
                            ops.string,
                            &capture.known_word_literal(index).unwrap(),
                        )
                        .unwrap();
                        Owned::retain(value.as_ptr())
                    })
                    .collect::<Vec<_>>();
                let check = |window: &str,
                             root: Option<*mut obj::TclObj>,
                             words: &[Owned],
                             compared: &mut usize| {
                    for line in data.lines() {
                        let row = tcl_test_support::fixture::tsv_fields(line).collect::<Vec<_>>();
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
                let control = tcl_test_support::fixture::tsv_fields(
                    data.lines()
                        .find(|line| {
                            let fields =
                                tcl_test_support::fixture::tsv_fields(line).collect::<Vec<_>>();
                            fields[0] == name
                                && fields[1].parse::<usize>().unwrap() == case
                                && fields[2] == "controls"
                        })
                        .unwrap(),
                )
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
                    let row = tcl_test_support::fixture::tsv_fields(
                        data.lines()
                            .find(|line| {
                                let fields =
                                    tcl_test_support::fixture::tsv_fields(line).collect::<Vec<_>>();
                                fields[0] == name
                                    && fields[1].parse::<usize>().unwrap() == case
                                    && fields[2] == "retained-errorcode"
                            })
                            .unwrap(),
                    )
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
