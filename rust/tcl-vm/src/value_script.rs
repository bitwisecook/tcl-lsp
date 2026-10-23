// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{IntRep, NativeJimObjectContext, NativeJimSourceInfo, Value};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
};
use tcl_syntax::{
    jim_script_objects::{JimScriptObjectConstruction, JimScriptObjects},
    native_jim_substitution::{JimOrdinaryScript, JimScriptStorage, JimSubstitutionObjects},
    value::ValueError,
};

#[cfg(test)]
#[path = "value_script_execution_tests.rs"]
mod execution_tests;

pub(crate) struct NativeJimScript {
    pub(crate) storage: JimScriptStorage<Value>,
    pub(crate) in_use: Cell<usize>,
    pub(crate) context: Weak<NativeJimObjectContext>,
}

impl NativeJimScript {
    pub(crate) fn ordinary(&self) -> Result<&JimOrdinaryScript<Value>, ValueError> {
        self.storage.ordinary()
    }

    pub(crate) fn source_info(&self) -> Result<NativeJimSourceInfo, ValueError> {
        let ordinary = self.ordinary()?;
        Ok(NativeJimSourceInfo {
            filename: ordinary.filename.clone(),
            line: ordinary.first_line,
        })
    }
}

pub(crate) struct NativeJimScriptHeader(pub(crate) Rc<NativeJimScript>);
impl Clone for NativeJimScriptHeader {
    fn clone(&self) -> Self {
        self.0.in_use.set(
            self.0
                .in_use
                .get()
                .checked_add(1)
                .expect("native Script use count"),
        );
        Self(Rc::clone(&self.0))
    }
}
impl Drop for NativeJimScriptHeader {
    fn drop(&mut self) {
        self.0.in_use.set(self.0.in_use.get() - 1);
    }
}

pub(crate) struct NativeJimScriptLease {
    original: Value,
    header: Option<NativeJimScriptHeader>,
    restore_primary: bool,
}
impl Drop for NativeJimScriptLease {
    fn drop(&mut self) {
        if let Some(header) = self.header.take().filter(|_| self.restore_primary) {
            self.original.replace_primary(IntRep::JimScript(header));
        }
    }
}

impl Value {
    pub(crate) fn native_jim_script_line(&self) -> Option<(i32, i32)> {
        match &*self.0.intrep.borrow() {
            IntRep::JimScriptLine { argc, line } => Some((*argc, *line)),
            _ => None,
        }
    }
    pub(crate) fn prepare_native_jim_script(
        &self,
        context: &Rc<NativeJimObjectContext>,
        config: tcl_lexer::LexerConfig,
    ) -> Result<Rc<NativeJimScript>, ValueError> {
        self.bind_native_jim_context(context)?;
        let null_script = self
            .is_same_object(&context.empty_object())
            .then(|| context.null_script_object());
        let original = null_script.as_deref().unwrap_or(self);
        if let IntRep::JimScript(header) = &*original.0.intrep.borrow() {
            if !Weak::ptr_eq(&header.0.context, &Rc::downgrade(context)) {
                return Err(ValueError::CommandProtocolUnavailable(
                    "Jim Script original interpreter",
                ));
            }
            if header.0.storage.flags() != 0 {
                return Err(ValueError::NativeFatalCondition(
                    tcl_syntax::raw_string::NativeFatalCondition::JimSubstitutionScriptReentry,
                ));
            }
            // A preparation view is not an additional native use owner.
            return Ok(Rc::clone(&header.0));
        }
        let bytes = original
            .native_string_bytes(tcl_syntax::native_string::NativeStringProtocol::Jim084)
            .map_err(|error| {
                ValueError::NativeStringAccess(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                )
            })?;
        let mut info = original.pin_native_jim_source_info(context)?;
        let image = tcl_lexer::SourceImage::native(bytes.as_ref());
        let roster = tcl_lexer::jim_script_tokens(&image, config).map_err(|_| {
            ValueError::CommandProtocolUnavailable("Jim Script original token roster")
        })?;
        let objects = JimScriptObjects::prepare(&roster, |construction| match construction {
            JimScriptObjectConstruction::Line { argc, line_delta } => {
                let value = Value::new_native_string_bytes(b"".as_slice());
                value.bind_native_jim_context(context)?;
                value.replace_primary(IntRep::JimScriptLine {
                    argc,
                    line: info.line.wrapping_add(line_delta.cast_signed()),
                });
                Ok(value)
            }
            JimScriptObjectConstruction::Word(count) => {
                let value = Value::int(i64::from(count));
                value.bind_native_jim_context(context)?;
                Ok(value)
            }
            JimScriptObjectConstruction::Source {
                bytes, line_delta, ..
            } => {
                let value = Value::new_native_string_bytes(bytes);
                value.install_native_jim_source(
                    NativeJimSourceInfo {
                        filename: info.filename.clone(),
                        line: info.line.wrapping_add(line_delta.cast_signed()),
                    },
                    context,
                )?;
                Ok(value)
            }
        })?;
        let linenr = match objects.completeness_line {
            tcl_lexer::JimScriptLine::Original(delta) => {
                info.line.wrapping_add(delta.cast_signed())
            }
            tcl_lexer::JimScriptLine::Zero => 0,
        };
        let baseline = info.line;
        info.line = info
            .line
            .wrapping_add(objects.first_line_delta.cast_signed());
        let backing = Rc::new(NativeJimScript {
            storage: JimScriptStorage::Ordinary(JimOrdinaryScript {
                objects,
                filename: info.filename,
                first_line: info.line,
                baseline,
                linenr: Cell::new(linenr),
            }),
            in_use: Cell::new(1),
            context: Rc::downgrade(context),
        });
        original.replace_primary(IntRep::JimScript(NativeJimScriptHeader(Rc::clone(
            &backing,
        ))));
        Ok(backing)
    }

    pub(crate) fn activate_native_jim_script(
        self,
        backing: &Rc<NativeJimScript>,
    ) -> NativeJimScriptLease {
        backing.in_use.set(
            backing
                .in_use
                .get()
                .checked_add(1)
                .expect("native Script use count"),
        );
        NativeJimScriptLease {
            original: self,
            header: Some(NativeJimScriptHeader(Rc::clone(backing))),
            restore_primary: true,
        }
    }

    pub(crate) fn prepare_native_jim_substitution(
        &self,
        context: &Rc<NativeJimObjectContext>,
        config: tcl_lexer::LexerConfig,
        flags: u8,
    ) -> Result<Rc<NativeJimScript>, ValueError> {
        self.bind_native_jim_context(context)?;
        if let IntRep::JimScript(header) = &*self.0.intrep.borrow() {
            if !Weak::ptr_eq(&header.0.context, &Rc::downgrade(context)) {
                return Err(ValueError::CommandProtocolUnavailable(
                    "Jim substitution original interpreter",
                ));
            }
            if header.0.storage.flags() == flags {
                return Ok(Rc::clone(&header.0));
            }
        }
        let bytes = self
            .native_string_bytes(tcl_syntax::native_string::NativeStringProtocol::Jim084)
            .map_err(|error| {
                ValueError::NativeStringAccess(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                )
            })?;
        let roster = tcl_lexer::jim_subst_tokens(
            &tcl_lexer::SourceImage::native(bytes.as_ref()),
            config,
            flags,
        )
        .map_err(|_| {
            ValueError::CommandProtocolUnavailable("Jim substitution original token roster")
        })?;
        let objects = JimSubstitutionObjects::prepare(&roster, |bytes| {
            let value = Value::new_native_string_bytes(bytes);
            value.bind_native_jim_context(context)?;
            Ok(value)
        })?;
        let backing = Rc::new(NativeJimScript {
            storage: JimScriptStorage::Substitution {
                objects,
                filename: context.empty_object().clone(),
            },
            in_use: Cell::new(1),
            context: Rc::downgrade(context),
        });
        self.replace_primary(IntRep::JimScript(NativeJimScriptHeader(Rc::clone(
            &backing,
        ))));
        Ok(backing)
    }

    pub(crate) fn activate_native_jim_substitution(
        self,
        backing: &Rc<NativeJimScript>,
    ) -> NativeJimScriptLease {
        let mut lease = self.activate_native_jim_script(backing);
        lease.restore_primary = false;
        lease
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;
    use tcl_syntax::jim_script_objects::JimScriptObjectKind;

    fn decode(hex: &str) -> Vec<u8> {
        if hex == "-" {
            return Vec::new();
        }
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    fn primary(value: &Value) -> &'static str {
        match &*value.0.intrep.borrow() {
            IntRep::Str => "NULL",
            IntRep::JimSource(_) => "source",
            IntRep::JimScript(_) => "script",
            IntRep::JimScriptLine { .. } => "scriptline",
            IntRep::Int(_) => "int",
            _ => panic!("unexpected measured Script primary"),
        }
    }
    fn observe_script_tokens(
        observed: &mut String,
        case: &str,
        backing: &NativeJimScript,
        filename: &Value,
    ) {
        for (index, token) in backing
            .ordinary()
            .unwrap()
            .objects
            .tokens()
            .iter()
            .enumerate()
        {
            let kind = match token.kind {
                JimScriptObjectKind::Line { .. } => 9,
                JimScriptObjectKind::Word(_) => 10,
                JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Escaped) => 2,
                JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::String) => 1,
                JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Variable) => 3,
                JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::IndexedVariable) => 4,
                JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Command) => 5,
                JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Expression) => 17,
                JimScriptObjectKind::Source(_) => panic!("unexpected real native Script token"),
            };
            let resident = token.value.resident_string_bytes();
            write!(
                observed,
                "TOKEN\t{case}\t{index}\t{kind}\t{}\t{}\t{}\t{}",
                primary(&token.value),
                usize::from(resident.is_some()),
                resident
                    .as_ref()
                    .map_or(-1, |bytes| i64::try_from(bytes.len()).unwrap()),
                token.value.native_object_reference_count()
            )
            .unwrap();
            match &*token.value.0.intrep.borrow() {
                IntRep::JimScriptLine { argc, line } => writeln!(observed, "\t{argc}\t{line}"),
                IntRep::Int(count) => writeln!(observed, "\t{count}\t-"),
                IntRep::JimSource(source) => writeln!(
                    observed,
                    "\t{}\t{}",
                    usize::from(source.info.filename.is_same_object(filename)),
                    source.info.line
                ),
                _ => panic!("unexpected measured cache"),
            }
            .unwrap();
        }
    }

    fn observe_script_input(
        case: &str,
        hex: &str,
        dialect: tcl_registry::InvocationDialect,
        config: tcl_lexer::LexerConfig,
        observed: &mut String,
    ) {
        let context = NativeJimObjectContext::new(dialect).unwrap();
        let filename = Value::new_native_string_bytes(b"FILE".as_slice());
        let original = Value::new_native_string_bytes(decode(hex).as_slice());
        original
            .install_native_jim_source(
                NativeJimSourceInfo {
                    filename: filename.clone(),
                    line: 7,
                },
                &context,
            )
            .unwrap();
        writeln!(
            observed,
            "SOURCE\t{case}\t{}\t{}",
            filename.native_object_reference_count(),
            original.native_object_reference_count()
        )
        .unwrap();
        let backing = original
            .prepare_native_jim_script(&context, config)
            .unwrap();
        writeln!(
            observed,
            "SCRIPT\t{case}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            backing.ordinary().unwrap().objects.tokens().len(),
            backing.ordinary().unwrap().first_line,
            backing.ordinary().unwrap().linenr.get(),
            backing
                .ordinary()
                .unwrap()
                .objects
                .missing
                .map_or(32, i32::from),
            backing.in_use.get(),
            usize::from(
                backing
                    .ordinary()
                    .unwrap()
                    .filename
                    .is_same_object(&filename)
            ),
            filename.native_object_reference_count(),
            usize::from(original.resident_string_bytes().is_some())
        )
        .unwrap();
        observe_script_tokens(observed, case, &backing, &filename);
        let duplicate = original
            .duplicate_native_object_in(tcl_syntax::native_string::NativeStringProtocol::Jim084);
        writeln!(
            observed,
            "DUP\t{case}\t{}\t{}\t{}\t{}",
            primary(&duplicate),
            usize::from(duplicate.resident_string_bytes().is_some()),
            duplicate.resident_string_bytes().unwrap().len(),
            filename.native_object_reference_count()
        )
        .unwrap();
        for (index, token) in backing
            .ordinary()
            .unwrap()
            .objects
            .tokens()
            .iter()
            .enumerate()
            .filter(|(_, token)| matches!(token.kind, JimScriptObjectKind::Line { .. }))
        {
            let duplicate = token.value.duplicate_native_object_in(
                tcl_syntax::native_string::NativeStringProtocol::Jim084,
            );
            writeln!(
                observed,
                "LINE_DUP\t{case}\t{index}\t{}\t{}\t{}",
                primary(&duplicate),
                usize::from(duplicate.resident_string_bytes().is_some()),
                duplicate.resident_string_bytes().unwrap().len()
            )
            .unwrap();
        }
        drop(backing);
        drop(original);
        drop(duplicate);
        writeln!(
            observed,
            "RETIRED\t{case}\t{}",
            filename.native_object_reference_count()
        )
        .unwrap();
    }

    #[test]
    fn original_script_storage_matches_156_native_windows() {
        let dialect = tcl_registry::InvocationDialect::of_profile(
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        );
        let config = tcl_lexer::LexerConfig::from_grammar(
            dialect
                .execution_point()
                .expect("actual Jim execution issuer")
                .grammar(),
        );
        let mut observed = String::new();
        for input in
            include_str!("../../tcl-syntax/testdata/native_jim_script_objects/inputs.tsv").lines()
        {
            let (case, hex) = input.split_once('\t').unwrap();
            observe_script_input(case, hex, dialect, config, &mut observed);
        }
        let context = NativeJimObjectContext::new(dialect).unwrap();
        let backing = context
            .empty_object()
            .prepare_native_jim_script(&context, config)
            .unwrap();
        writeln!(
            observed,
            "TRUE_EMPTY\t{}\t{}\t{}\t{}",
            primary(&context.empty_object()),
            primary(&context.null_script_object()),
            backing.ordinary().unwrap().objects.tokens().len(),
            usize::from(
                backing
                    .ordinary()
                    .unwrap()
                    .filename
                    .is_same_object(&context.empty_object())
            )
        )
        .unwrap();
        let other = Value::new_native_string_bytes(b"".as_slice());
        other.prepare_native_jim_script(&context, config).unwrap();
        writeln!(
            observed,
            "EQUAL_EMPTY\t{}\t{}",
            primary(&other),
            usize::from(other.is_same_object(&context.null_script_object()))
        )
        .unwrap();
        assert_eq!(observed.lines().count(), 156);
        assert_eq!(
            observed,
            include_str!("../../tcl-syntax/testdata/native_jim_script_objects/observations.tsv")
        );
    }
}
