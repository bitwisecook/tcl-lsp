// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original Jim dictionary-substitution children and borrowed interpolation names.

use super::{IntRep, NativeJimObjectContext, Value, WeakNativeObject};
use std::rc::Rc;
use tcl_syntax::{native_jim_substitution::dictionary_substitution_extents, value::ValueError};

#[derive(Clone)]
pub(super) struct JimInterpolated {
    name: WeakNativeObject,
    index: Value,
}

impl Value {
    /// Install the actual four-token optimization: name is borrowed, index owned.
    pub(crate) fn install_native_jim_interpolated(
        &self,
        name: &Value,
        index: &Value,
        context: &Rc<NativeJimObjectContext>,
    ) -> Result<(), ValueError> {
        self.bind_native_jim_context(context)?;
        name.bind_native_jim_context(context)?;
        index.bind_native_jim_context(context)?;
        if self.resident_string_bytes().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim interpolated resident string",
            ));
        }
        self.replace_primary(IntRep::JimInterpolated(JimInterpolated {
            name: name.downgrade_native_object(),
            index: index.clone().into_native_reference(),
        }));
        Ok(())
    }

    /// Prepare exact original children before retiring any previous primary.
    pub(crate) fn ensure_native_jim_dictionary_substitution(
        &self,
        context: &Rc<NativeJimObjectContext>,
    ) -> Result<(), ValueError> {
        self.bind_native_jim_context(context)?;
        let converted =
            {
                let primary = self.0.intrep.borrow();
                match &*primary {
                    IntRep::JimDictionarySubstitution { .. } => return Ok(()),
                    IntRep::JimInterpolated(original) => {
                        let name = original.name.upgrade().ok_or(
                            ValueError::CommandProtocolUnavailable(
                                "retired borrowed Jim interpolation name",
                            ),
                        )?;
                        Some((name, original.index.clone().into_native_reference()))
                    }
                    _ => None,
                }
            };
        let (name, index) = if let Some(children) = converted {
            children
        } else {
            let bytes = self
                .native_string_bytes(tcl_syntax::native_string::NativeStringProtocol::Jim084)
                .map_err(|error| {
                    ValueError::NativeStringAccess(
                        tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                    )
                })?;
            let extents = dictionary_substitution_extents(&bytes)?;
            let name = Value::new_native_string_bytes(&bytes[extents.name]);
            let index = Value::new_native_string_bytes(&bytes[extents.key]);
            name.bind_native_jim_context(context)?;
            index.bind_native_jim_context(context)?;
            (name, index)
        };
        self.replace_primary(IntRep::JimDictionarySubstitution {
            name: name.into_native_reference(),
            index: index.into_native_reference(),
        });
        Ok(())
    }

    /// Borrow the actual owned tuple without preparing it or cloning children.
    /// The callback must not invoke guest code or replace this parent's primary.
    pub(crate) fn with_native_jim_dictionary_substitution<R>(
        &self,
        with: impl FnOnce(&Value, &Value) -> R,
    ) -> Option<R> {
        match &*self.0.intrep.borrow() {
            IntRep::JimDictionarySubstitution { name, index } => Some(with(name, index)),
            _ => None,
        }
    }

    /// Inspect the borrowed original name and owned index without retaining them.
    #[cfg(test)]
    pub(crate) fn with_native_jim_interpolated<R>(
        &self,
        with: impl FnOnce(&Value, &Value) -> R,
    ) -> Result<R, ValueError> {
        match &*self.0.intrep.borrow() {
            IntRep::JimInterpolated(original) => {
                let name =
                    original
                        .name
                        .upgrade()
                        .ok_or(ValueError::CommandProtocolUnavailable(
                            "retired borrowed Jim interpolation name",
                        ))?;
                let lease = name.native_lifetime_lease();
                drop(name);
                Ok(with(lease.value(), &original.index))
            }
            _ => Err(ValueError::CommandProtocolUnavailable(
                "Jim original interpolated primary",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Code, Vm};
    use std::fmt::Write;
    use tcl_syntax::{
        native_jim_substitution::JimScriptStorage, native_string::NativeStringProtocol,
    };

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().fold(String::new(), |mut output, byte| {
            use std::fmt::Write as _;
            write!(output, "{byte:02x}").unwrap();
            output
        })
    }
    fn kind(kind: tcl_lexer::JimScriptTokenKind) -> i32 {
        match kind {
            tcl_lexer::JimScriptTokenKind::String => 1,
            tcl_lexer::JimScriptTokenKind::Escaped => 2,
            tcl_lexer::JimScriptTokenKind::Variable => 3,
            tcl_lexer::JimScriptTokenKind::IndexedVariable => 4,
            tcl_lexer::JimScriptTokenKind::Command => 5,
            tcl_lexer::JimScriptTokenKind::Expression => 6,
            _ => panic!("native Subst real token purpose"),
        }
    }
    fn native_type(value: &Value) -> &'static str {
        match value.native_object_type_name() {
            "none" => "NULL",
            other => other,
        }
    }
    fn children(
        observed: &mut String,
        stage: &str,
        value: &Value,
        filename: &Value,
        original_name: &Value,
        original_index: &Value,
        interpolated: bool,
    ) {
        let with = |name: &Value, index: &Value| {
            writeln!(
                observed,
                "CHILDREN\t{stage}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                native_type(value),
                usize::from(name.is_same_object(original_name)),
                usize::from(index.is_same_object(original_index)),
                name.native_object_reference_count(),
                index.native_object_reference_count(),
                filename.native_object_reference_count(),
                native_type(name),
                native_type(index),
                hex(&name.resident_string_bytes().unwrap()),
                hex(&index.resident_string_bytes().unwrap())
            )
            .unwrap();
        };
        if interpolated {
            value.with_native_jim_interpolated(with).unwrap();
        } else {
            value.with_native_jim_dictionary_substitution(with).unwrap();
        }
    }
    fn observe_substitution_input(
        case: usize,
        source: &[u8],
        flags: u8,
        context: &Rc<NativeJimObjectContext>,
        config: tcl_lexer::LexerConfig,
        observed: &mut String,
    ) {
        let filename = Value::new_native_string_bytes(b"FILE".as_slice());
        let original = Value::new_native_string_bytes(source);
        original
            .install_native_jim_source(
                super::super::NativeJimSourceInfo {
                    filename: filename.clone(),
                    line: 7,
                },
                context,
            )
            .unwrap();
        writeln!(
            observed,
            "BEFORE\t{case}\t{flags}\t{}\t{}\t{}",
            native_type(&original),
            original.native_object_reference_count(),
            filename.native_object_reference_count()
        )
        .unwrap();
        let backing = original
            .prepare_native_jim_substitution(context, config, flags)
            .unwrap();
        let JimScriptStorage::Substitution {
            objects,
            filename: script_file,
        } = &backing.storage
        else {
            panic!("fresh Subst storage")
        };
        writeln!(
            observed,
            "SUBST\t{case}\t{flags}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            native_type(&original),
            objects.tokens().len(),
            objects.flags,
            backing.in_use.get(),
            usize::from(script_file.is_same_object(&context.empty_object())),
            filename.native_object_reference_count(),
            original.native_object_reference_count()
        )
        .unwrap();
        for (index, token) in objects.tokens().iter().enumerate() {
            let bytes = token.value.resident_string_bytes().unwrap();
            writeln!(
                observed,
                "TOKEN\t{case}\t{flags}\t{index}\t{}\t{}\t{}\t{}\t{}",
                kind(token.kind),
                native_type(&token.value),
                token.value.native_object_reference_count(),
                bytes.len(),
                hex(&bytes)
            )
            .unwrap();
        }
        let duplicate = original.duplicate_native_object_in(NativeStringProtocol::Jim084);
        writeln!(
            observed,
            "DUP\t{case}\t{flags}\t{}\t{}\t{}",
            native_type(&duplicate),
            usize::from(duplicate.resident_string_bytes().is_some()),
            filename.native_object_reference_count()
        )
        .unwrap();
        drop(duplicate);
        drop(original);
        drop(backing);
        writeln!(
            observed,
            "RETIRE\t{case}\t{flags}\t{}",
            filename.native_object_reference_count()
        )
        .unwrap();
    }

    fn observe_dictionary_windows(
        vm: &mut Vm,
        context: &Rc<NativeJimObjectContext>,
        observed: &mut String,
        filename: &Value,
        tokens: &[Value],
        index: &Value,
    ) {
        let value = Value::new_native_string_bytes(b"d(KEY)".as_slice());
        value
            .install_native_jim_interpolated(&tokens[0], index, context)
            .unwrap();
        children(
            observed,
            "interpolated",
            &value,
            filename,
            &tokens[0],
            index,
            true,
        );
        let duplicate = value.duplicate_native_object_in(NativeStringProtocol::Jim084);
        children(
            observed,
            "interpolated-duplicate",
            &duplicate,
            filename,
            &tokens[0],
            index,
            true,
        );
        drop(duplicate);
        value
            .ensure_native_jim_dictionary_substitution(context)
            .unwrap();
        children(
            observed,
            "dict-converted",
            &value,
            filename,
            &tokens[0],
            index,
            false,
        );
        let result = vm
            .expand_native_jim_dictionary_substitution(&value)
            .unwrap();
        writeln!(
            observed,
            "EXPAND\t1\t{}\t{}\t{}",
            native_type(&value),
            filename.native_object_reference_count(),
            hex(&result.resident_string_bytes().unwrap())
        )
        .unwrap();
        drop(result);
        children(
            observed, "expanded", &value, filename, &tokens[0], index, false,
        );
        drop(value);
    }

    fn observe_original_dictionary(
        vm: &mut Vm,
        context: &Rc<NativeJimObjectContext>,
        observed: &mut String,
    ) {
        let setup =
            Value::new_native_string_bytes(b"set k KEY; set d [dict create KEY VALUE]".as_slice());
        assert_eq!(
            vm.eval_original_script_value(
                &setup,
                tcl_registry::native_eval_object::EvalObjectPurpose::Eval,
                None,
            )
            .unwrap()
            .code,
            Code::Ok
        );
        drop(setup);
        let filename = Value::new_native_string_bytes(b"FILE".as_slice());
        let tokens: Vec<Value> = [b"d".as_slice(), b"(", b"k", b")"]
            .into_iter()
            .map(Value::new_native_string_bytes)
            .collect();
        tokens[0]
            .install_native_jim_source(
                super::super::NativeJimSourceInfo {
                    filename: filename.clone(),
                    line: 7,
                },
                context,
            )
            .unwrap();
        let index = vm.read_original_named_variable(&tokens[2]).unwrap();
        let index_lease = index.native_lifetime_lease();
        drop(index);
        let index = index_lease.value();
        writeln!(
            observed,
            "OPT_BEFORE\t{}\t{}\t{}",
            tokens[0].native_object_reference_count(),
            index.native_object_reference_count(),
            filename.native_object_reference_count()
        )
        .unwrap();
        observe_dictionary_windows(vm, context, observed, &filename, &tokens, index);
        drop(tokens);
        writeln!(
            observed,
            "OPT_RETIRE\t{}",
            filename.native_object_reference_count()
        )
        .unwrap();
    }

    #[test]
    fn original_substitution_storage_matches_205_native_windows() {
        let mut vm = Vm::new();
        vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        );
        let context = vm.native_jim_object_context().unwrap();
        let config = vm.lexer_config();
        let sources: [&[u8]; 4] = [
            b"a\n$k[set x X]\\n{q};z",
            b"A\0B$k[set x X]",
            b"${x\ny}P\nQ",
            b"d($k)",
        ];
        let mut observed = String::new();
        for (case, source) in sources.into_iter().enumerate() {
            for flags in 0..8 {
                observe_substitution_input(case, source, flags, &context, config, &mut observed);
            }
        }
        observe_original_dictionary(&mut vm, &context, &mut observed);
        let original = Value::new_native_string_bytes(b"set x 1".as_slice());
        let ordinary = original
            .prepare_native_jim_script(&context, config)
            .unwrap();
        let reused = original
            .prepare_native_jim_substitution(&context, config, 0)
            .unwrap();
        writeln!(
            observed,
            "REUSE\t{}\t{}\t{}",
            usize::from(Rc::ptr_eq(&ordinary, &reused)),
            reused.storage.flags(),
            reused.storage.len()
        )
        .unwrap();
        let reparsed = original
            .prepare_native_jim_substitution(&context, config, 4)
            .unwrap();
        writeln!(
            observed,
            "REPARSE\t{}\t{}\t{}",
            usize::from(Rc::ptr_eq(&ordinary, &reparsed)),
            reparsed.storage.flags(),
            reparsed.storage.len()
        )
        .unwrap();
        assert_eq!(observed.lines().count(), 205);
        assert_eq!(
            observed,
            include_str!("../../../tcl-syntax/testdata/native_jim_indexed_substitution/normal.tsv")
        );
    }
}
