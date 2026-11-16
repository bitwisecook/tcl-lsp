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

//! Internal representations cached on an object besides numbers and containers: expressions, command names, indexes, array searches, bytecode, substitutions, end offsets, regular expressions, completion codes and frame levels.

use super::{
    ExpressionCache, ExpressionSourceInput, IntRep, JimExpressionInstall, NativeBytecodeCache,
    NativeBytecodeContext, NativeJimSourceInfo, Value,
};
use std::cell::RefCell;
use std::rc::Rc;
use tcl_syntax::native_string::NativeStringStorageIdentity;
use tcl_syntax::raw_string::RawString;
use tcl_syntax::value::ValueError;

impl Value {
    #[cfg(test)]
    pub(crate) fn native_expression_cache_present(&self) -> bool {
        matches!(*self.0.intrep.borrow(), IntRep::Expression(_))
    }

    /// Materialise source bytes before replacing the original typed rep.
    /// The selected engine determines whether failed preparation retires the
    /// old representation or preserves it until a complete tree is prepared.
    pub(crate) fn expression_source(
        &self,
        dialect: tcl_registry::InvocationDialect,
        profile: &tcl_dialect::DialectProfileKey,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        policy: &crate::compiled::NativeCompilerPolicy,
        preparation: tcl_syntax::expr::parser::ExpressionSourceCachePreparation,
    ) -> Result<Rc<[u8]>, tcl_syntax::value::ValueError> {
        self.expression_source_using(
            &ExpressionSourceInput {
                dialect,
                profile,
                protocol,
                policy,
                preparation,
            },
            None,
            &mut None,
        )
    }

    pub(crate) fn expression_source_checked<E: From<ValueError>>(
        &self,
        input: &ExpressionSourceInput<'_>,
        materialized: &Rc<[u8]>,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<Rc<[u8]>, E> {
        self.expression_source_using(input, Some(materialized), &mut Some(check))
    }

    fn expression_source_using<E: From<ValueError>>(
        &self,
        input: &ExpressionSourceInput<'_>,
        materialized: Option<&Rc<[u8]>>,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<Rc<[u8]>, E> {
        Self::native_stage_check(check)?;
        let bytes = if let Some(bytes) = materialized {
            Rc::clone(bytes)
        } else {
            let reached = self.native_string_bytes(input.protocol).map_err(|error| {
                ValueError::NativeStringAccess(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                )
            });
            Self::native_stage_result(reached, check)?
        };
        let matches = matches!(&*self.0.intrep.borrow(), IntRep::Expression(cache)
            if cache.dialect == input.dialect && &cache.profile == input.profile && cache.policy == *input.policy);
        if !matches
            && input.preparation
                == tcl_syntax::expr::parser::ExpressionSourceCachePreparation::BeforeParsing
        {
            let replacement =
                Self::expression_primary(input.dialect, input.profile, input.policy, None);
            if check.is_some() {
                self.replace_primary_using(replacement, check)?;
            } else {
                self.replace_primary(replacement);
            }
        }
        Self::native_stage_check(check)?;
        Ok(bytes)
    }

    fn expression_primary(
        dialect: tcl_registry::InvocationDialect,
        profile: &tcl_dialect::DialectProfileKey,
        policy: &crate::compiled::NativeCompilerPolicy,
        node: Option<tcl_syntax::expr::NativeExprNode>,
    ) -> IntRep {
        IntRep::Expression(Rc::new(ExpressionCache {
            profile: *profile,
            dialect,
            policy: *policy,
            node,
            jim: None,
        }))
    }

    pub(crate) fn reject_expression(
        &self,
        dialect: tcl_registry::InvocationDialect,
        profile: &tcl_dialect::DialectProfileKey,
        policy: &crate::compiled::NativeCompilerPolicy,
    ) {
        self.replace_primary(Self::expression_primary(dialect, profile, policy, None));
    }

    pub(crate) fn cached_expression(
        &self,
        dialect: tcl_registry::InvocationDialect,
        profile: &tcl_dialect::DialectProfileKey,
        policy: &crate::compiled::NativeCompilerPolicy,
    ) -> Option<tcl_syntax::expr::NativeExprNode> {
        match &*self.0.intrep.borrow() {
            IntRep::Expression(cache)
                if cache.dialect == dialect
                    && &cache.profile == profile
                    && cache.policy == *policy =>
            {
                cache.node.clone()
            }
            _ => None,
        }
    }

    pub(crate) fn native_jim_optimistic_integer(&self) -> bool {
        matches!(*self.0.intrep.borrow(), IntRep::Str | IntRep::JimSource(_))
    }

    pub(crate) fn native_jim_expression_objects(
        &self,
    ) -> Option<Rc<tcl_syntax::expr::native_objects::JimExpressionObjects<Value>>> {
        match &*self.0.intrep.borrow() {
            IntRep::Expression(cache) => cache.jim.clone(),
            _ => None,
        }
    }

    pub(crate) fn expression_rejected(
        &self,
        dialect: tcl_registry::InvocationDialect,
        profile: &tcl_dialect::DialectProfileKey,
        policy: &crate::compiled::NativeCompilerPolicy,
    ) -> bool {
        matches!(&*self.0.intrep.borrow(), IntRep::Expression(cache) if cache.dialect == dialect && &cache.profile == profile && cache.policy == *policy && cache.node.is_none())
    }

    pub(crate) fn install_jim_expression(
        &self,
        input: &JimExpressionInstall<'_>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        use tcl_syntax::expr::native_objects::{JimExpressionObjects, JimExpressionTermValue};
        let context = self.native_jim_context()?;
        let objects =
            JimExpressionObjects::prepare(input.source, input.preparation, |term, payload| {
                let value = match payload {
                    JimExpressionTermValue::Number(
                        tcl_syntax::scalar_getter::JimExpressionNumber::Integer(number),
                    ) => Value::int(number),
                    JimExpressionTermValue::Number(
                        tcl_syntax::scalar_getter::JimExpressionNumber::Double(number),
                    ) => Value::double(number).with_native_double_format(input.dialect),
                    JimExpressionTermValue::String(bytes) => Value::new_native_string_bytes(bytes),
                };
                value.bind_native_jim_context(&context)?;
                if term.kind == tcl_lexer::ExprTermKind::Command {
                    value.install_native_jim_source(
                        NativeJimSourceInfo {
                            filename: input.info.filename.clone(),
                            line: input.info.line.wrapping_add_unsigned(term.line_delta),
                        },
                        &context,
                    )?;
                }
                Ok(value)
            })?;
        self.replace_primary(IntRep::Expression(Rc::new(ExpressionCache {
            dialect: input.dialect,
            profile: input.profile,
            policy: input.policy,
            node: Some(input.node.clone()),
            jim: Some(Rc::new(objects)),
        })));
        Ok(())
    }

    pub(crate) fn retain_expression_primary(&self) -> Option<Box<dyn FnOnce()>> {
        let cache = match &*self.0.intrep.borrow() {
            IntRep::Expression(cache) if cache.jim.is_some() => Rc::clone(cache),
            _ => return None,
        };
        let original = self.clone();
        Some(Box::new(move || {
            original.replace_primary(IntRep::Expression(cache));
        }))
    }

    fn expression_cache_replacement(
        &self,
        dialect: tcl_registry::InvocationDialect,
        profile: &tcl_dialect::DialectProfileKey,
        policy: &crate::compiled::NativeCompilerPolicy,
        node: &tcl_syntax::expr::NativeExprNode,
    ) -> Option<IntRep> {
        if self.native_jim_expression_objects().is_some()
            && self.cached_expression(dialect, profile, policy).is_some()
        {
            return None;
        }
        Some(Self::expression_primary(
            dialect,
            profile,
            policy,
            Some(node.clone()),
        ))
    }

    pub(crate) fn cache_expression(
        &self,
        dialect: tcl_registry::InvocationDialect,
        profile: &tcl_dialect::DialectProfileKey,
        policy: &crate::compiled::NativeCompilerPolicy,
        node: &tcl_syntax::expr::NativeExprNode,
    ) {
        if let Some(replacement) = self.expression_cache_replacement(dialect, profile, policy, node)
        {
            self.replace_primary(replacement);
        }
    }

    pub(crate) fn cache_expression_checked<E: From<ValueError>>(
        &self,
        dialect: tcl_registry::InvocationDialect,
        profile: &tcl_dialect::DialectProfileKey,
        policy: &crate::compiled::NativeCompilerPolicy,
        node: &tcl_syntax::expr::NativeExprNode,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        let mut check = Some(check);
        Self::native_stage_check(&mut check)?;
        if let Some(replacement) = self.expression_cache_replacement(dialect, profile, policy, node)
        {
            self.replace_primary_using(replacement, &mut check)?;
        }
        Self::native_stage_check(&mut check)
    }

    /// Inspect the original named-command cache without lookup or conversion.
    /// This descriptor supplies no current command dispatch authority.
    #[must_use]
    pub fn native_command_name_cache(
        &self,
    ) -> Option<tcl_runtime_api::native_command_name::NativeCommandNameCache> {
        match &*self.0.intrep.borrow() {
            IntRep::NativeCommandName(cache) => Some(cache.clone()),
            _ => None,
        }
    }

    /// Actual command-name cache origin, including a C8 null descriptor.
    /// No lookup or live command-node validation is performed.
    #[must_use]
    pub fn native_command_name_cache_origin(&self) -> Option<tcl_dialect::TclVersion> {
        match &*self.0.intrep.borrow() {
            IntRep::NativeCommandName(cache) => Some(cache.version),
            IntRep::NativeCommandNameUnresolved(version) => Some(*version),
            _ => None,
        }
    }

    /// Install a descriptor on this same original resident-string object.
    /// Live command-world validation remains the caller's responsibility.
    /// Compiler priming uses the separate native `SetCmdName` early-return operation.
    ///
    /// # Errors
    /// Refuses foreign origins or absent original resident spelling.
    pub fn install_native_command_name_cache(
        &self,
        cache: tcl_runtime_api::native_command_name::NativeCommandNameCache,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.install_native_command_name_cache_using(cache, dialect, &mut None)
    }

    pub(crate) fn install_native_command_name_cache_checked<E: From<ValueError>>(
        &self,
        cache: tcl_runtime_api::native_command_name::NativeCommandNameCache,
        dialect: tcl_registry::InvocationDialect,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        self.install_native_command_name_cache_using(cache, dialect, &mut Some(check))
    }

    fn install_native_command_name_cache_using<E: From<ValueError>>(
        &self,
        cache: tcl_runtime_api::native_command_name::NativeCommandNameCache,
        dialect: tcl_registry::InvocationDialect,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<(), E> {
        if !dialect
            .native_command_name_protocol()
            .is_some_and(|protocol| protocol.accepts_cache_origin(cache.version))
            || self.resident_string_bytes().is_none()
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native command-name cache origin or resident string",
            )
            .into());
        }
        self.replace_primary_using(IntRep::NativeCommandName(cache), check)
    }

    /// Execute compile-time `SetCmdName`'s independently selected early-return rule.
    ///
    /// # Errors
    /// Refuses unsupported origins or absent resident spelling.
    pub fn prime_native_command_name_cache(
        &self,
        cache: tcl_runtime_api::native_command_name::NativeCommandNameCache,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let protocol = dialect.native_command_name_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native command-name priming",
            ),
        )?;
        if !protocol.accepts_cache_origin(cache.version) {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native command-name priming origin",
            ));
        }
        let existing = self.native_command_name_cache();
        let unresolved = matches!(*self.0.intrep.borrow(), IntRep::NativeCommandNameUnresolved(version) if version == cache.version);
        if protocol.preserves_primed_cache(existing.as_ref(), unresolved, &cache) {
            return Ok(());
        }
        self.install_native_command_name_cache(cache, dialect)
    }

    /// Install C8's reached null command-name descriptor after closed lookup absence.
    ///
    /// # Errors
    /// Refuses unsupported engines or absent original resident spelling.
    pub fn install_unresolved_native_command_name_cache(
        &self,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.install_unresolved_native_command_name_cache_using(dialect, &mut None)
    }

    pub(crate) fn install_unresolved_native_command_name_cache_checked<E: From<ValueError>>(
        &self,
        dialect: tcl_registry::InvocationDialect,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        self.install_unresolved_native_command_name_cache_using(dialect, &mut Some(check))
    }

    fn install_unresolved_native_command_name_cache_using<E: From<ValueError>>(
        &self,
        dialect: tcl_registry::InvocationDialect,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<(), E> {
        let protocol = dialect.native_command_name_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native unresolved command-name cache",
            ),
        )?;
        if !protocol.installs_unresolved_on_miss() || self.resident_string_bytes().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native unresolved command-name cache origin",
            )
            .into());
        }
        self.replace_primary_using(
            IntRep::NativeCommandNameUnresolved(protocol.version()),
            check,
        )
    }

    /// Withdraw only cmdName, preserving the original resident allocation.
    pub fn retire_native_command_name_cache(&self) {
        if matches!(
            *self.0.intrep.borrow(),
            IntRep::NativeCommandName(_) | IntRep::NativeCommandNameUnresolved(_)
        ) {
            *self.0.intrep.borrow_mut() = IntRep::Str;
        }
    }

    /// Inspect a retained native Index cache without materialisation.
    #[must_use]
    pub fn native_index_cache(
        &self,
    ) -> Option<(tcl_core_types::NativeIndexCache, tcl_dialect::TclVersion)> {
        match &*self.0.intrep.borrow() {
            IntRep::NativeIndex { cache, version } => Some((cache.clone(), *version)),
            _ => None,
        }
    }

    /// Install a reached native Index on the same original resident header.
    pub(crate) fn install_native_index_cache(
        &self,
        cache: tcl_core_types::NativeIndexCache,
        protocol: tcl_registry::native_index_lookup::NativeIndexLookupProtocol,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        if self.resident_string_bytes().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native Index resident storage",
            ));
        }
        *self.0.intrep.borrow_mut() = IntRep::NativeIndex {
            cache,
            version: protocol.version(),
        };
        Ok(())
    }

    /// Inspect a legacy array-search primary without conferring cursor authority.
    pub(crate) fn native_array_search_cache_in(
        &self,
        protocol: tcl_syntax::native_array_search::NativeArraySearchProtocol,
    ) -> Result<Option<tcl_core_types::NativeArraySearchCache>, tcl_syntax::value::ValueError> {
        if let IntRep::NativeArraySearch { cache, version } = &*self.0.intrep.borrow() {
            if !protocol.accepts_cache_origin(*version) || self.resident_string_bytes().is_none() {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native array-search cache origin",
                ));
            }
            return Ok(Some(*cache));
        }
        Ok(None)
    }

    /// Install the actually reached legacy parse on the SAME original object.
    pub(crate) fn install_native_array_search_cache(
        &self,
        cache: tcl_core_types::NativeArraySearchCache,
        protocol: tcl_syntax::native_array_search::NativeArraySearchProtocol,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        if !protocol.caches_handle()
            || self
                .resident_string_bytes()
                .is_none_or(|bytes| cache.name_offset > bytes.len())
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native array-search cache storage",
            ));
        }
        *self.0.intrep.borrow_mut() = IntRep::NativeArraySearch {
            cache,
            version: protocol.version(),
        };
        Ok(())
    }

    /// Reconstruct an original Index cache under its actual C engine origin.
    /// Resident storage identity is transported independently of table identity.
    ///
    /// # Errors
    /// Refuses foreign engines and inconsistent resident-string provenance.
    pub fn from_native_index_cache(
        cache: tcl_core_types::NativeIndexCache,
        origin: tcl_dialect::TclVersion,
        dialect: tcl_registry::InvocationDialect,
        resident: Option<(Rc<[u8]>, NativeStringStorageIdentity)>,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        if dialect.native_string_protocol()
            != Some(tcl_syntax::native_string::NativeStringProtocol::C(origin))
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "native Index carrier origin",
            ));
        }
        if resident.as_ref().is_some_and(|(bytes, marker)| {
            *marker == NativeStringStorageIdentity::CanonicalEmpty && !bytes.is_empty()
        }) {
            return Err(ValueError::CommandProtocolUnavailable(
                "native Index resident storage",
            ));
        }
        let storage = resident
            .as_ref()
            .map_or(NativeStringStorageIdentity::Unknown, |(_, marker)| *marker);
        let value = Self::from_raw_parts(
            resident.map(|(bytes, _)| RawString::from_bytes(bytes)),
            IntRep::NativeIndex {
                cache,
                version: origin,
            },
        );
        value.0.string_storage.set(storage);
        Ok(value)
    }

    /// Inspect the original admitted executable, without granting cache validity.
    pub(crate) fn native_bytecode_cache(&self) -> Option<Rc<NativeBytecodeCache>> {
        match &*self.0.intrep.borrow() {
            IntRep::NativeBytecode(cache)
                if !matches!(cache.context, NativeBytecodeContext::Substitution(_)) =>
            {
                Some(Rc::clone(cache))
            }
            _ => None,
        }
    }

    pub(crate) fn native_substitution_cache(&self) -> Option<Rc<NativeBytecodeCache>> {
        match &*self.0.intrep.borrow() {
            IntRep::NativeBytecode(cache)
                if matches!(cache.context, NativeBytecodeContext::Substitution(_)) =>
            {
                Some(Rc::clone(cache))
            }
            _ => None,
        }
    }

    pub(crate) fn retire_native_substitution_cache(&self) {
        if matches!(&*self.0.intrep.borrow(), IntRep::NativeBytecode(cache) if matches!(cache.context, NativeBytecodeContext::Substitution(_)))
        {
            self.replace_primary(IntRep::Str);
        }
    }

    pub(crate) fn install_native_substitution_cache(
        &self,
        version: tcl_dialect::TclVersion,
        flags: tcl_runtime_api::native_substitution::NativeSubstitutionFlags,
        layout: Option<tcl_runtime_api::native_compilation::NativeCompiledLocalLayout>,
        table: Option<Rc<crate::literal_pool::NativeLocalNameTable>>,
        unit: crate::compiled::CompiledUnit,
    ) -> Result<(), ValueError> {
        if !matches!(
            version,
            tcl_dialect::TclVersion::V8_6
                | tcl_dialect::TclVersion::V9_0
                | tcl_dialect::TclVersion::V9_1
        ) || self.resident_string_bytes().is_none()
            || unit.native_cache.is_none()
            || unit.jim_script.is_some()
            || unit.fatal_tail.is_some()
            || unit.asm.validate_native_compilation_entry().is_err()
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "original C substitution cache",
            ));
        }
        self.replace_primary(IntRep::NativeBytecode(Rc::new(NativeBytecodeCache {
            version,
            context: NativeBytecodeContext::Substitution(flags),
            substitution_layout: layout,
            substitution_table: table,
            unit,
            procedure: RefCell::new(std::rc::Weak::new()),
        })));
        Ok(())
    }

    /// Publish only a genuine compiler-produced C artifact on resident source.
    pub(crate) fn install_native_bytecode_cache(
        &self,
        version: tcl_dialect::TclVersion,
        context: NativeBytecodeContext,
        unit: crate::compiled::CompiledUnit,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        if self.resident_string_bytes().is_none()
            || unit.jim_script.is_some()
            || unit.native_cache.is_none()
            || (unit.fatal_tail.is_some()
                && !tcl_syntax::native_bytecode::NativeBytecodeStorageRecipe::for_version(version)
                    .retains_parse_failure())
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original admitted C bytecode storage",
            ));
        }
        self.replace_primary(IntRep::NativeBytecode(Rc::new(NativeBytecodeCache {
            version,
            context,
            substitution_layout: None,
            substitution_table: None,
            unit,
            procedure: RefCell::new(std::rc::Weak::new()),
        })));
        Ok(())
    }

    pub(crate) fn install_native_procedure_bytecode_cache(
        &self,
        version: tcl_dialect::TclVersion,
        unit: crate::compiled::CompiledUnit,
        procedure: &Rc<crate::command::ProcDef>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.install_native_bytecode_cache(version, NativeBytecodeContext::Procedure, unit)?;
        let mut primary = self.0.intrep.borrow_mut();
        if let IntRep::NativeBytecode(cache) = &mut *primary {
            *cache.procedure.borrow_mut() = Rc::downgrade(procedure);
        }
        Ok(())
    }

    pub(crate) fn clear_native_procedure_bytecode_context(
        &self,
        procedure: &Rc<crate::command::ProcDef>,
    ) {
        if let IntRep::NativeBytecode(cache) = &*self.0.intrep.borrow() {
            let mut owner = cache.procedure.borrow_mut();
            if tcl_syntax::native_bytecode::NativeBytecodeStorageRecipe::for_version(cache.version)
                .clears_retired_procedure_context()
                && std::rc::Weak::ptr_eq(&owner, &Rc::downgrade(procedure))
            {
                *owner = std::rc::Weak::new();
            }
        }
    }

    pub(crate) fn native_instruction_name(
        &self,
    ) -> Option<tcl_syntax::native_instruction_name::NativeInstructionName> {
        if let IntRep::NativeInstructionName(name) = *self.0.intrep.borrow() {
            Some(name)
        } else {
            None
        }
    }

    pub(crate) fn new_native_instruction_name(
        name: tcl_syntax::native_instruction_name::NativeInstructionName,
    ) -> Self {
        Self::from_raw_parts(None, IntRep::NativeInstructionName(name))
    }

    pub(crate) fn native_jim_index(&self) -> Result<Option<i32>, tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        Ok(match &*self.0.intrep.borrow() {
            IntRep::JimIndex(index) => Some(index.0),
            _ => None,
        })
    }

    pub(crate) fn clear_native_index_arithmetic_primary(&self) {
        self.replace_primary(IntRep::Str);
    }

    pub(crate) fn clear_native_index_arithmetic_primary_checked<E: From<ValueError>>(
        &self,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        self.replace_primary_using(IntRep::Str, &mut Some(check))
    }

    pub(crate) fn native_end_offset(
        &self,
    ) -> Option<tcl_syntax::native_end_offset::NativeEndOffset> {
        if let IntRep::NativeEndOffset(offset) = *self.0.intrep.borrow() {
            Some(offset)
        } else {
            None
        }
    }

    pub(crate) fn install_native_end_offset(
        &self,
        offset: tcl_syntax::native_end_offset::NativeEndOffset,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        if self.resident_string_bytes().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original end-offset resident string",
            ));
        }
        self.replace_primary(IntRep::NativeEndOffset(offset));
        Ok(())
    }

    pub(crate) fn install_native_end_offset_checked<E: From<ValueError>>(
        &self,
        offset: tcl_syntax::native_end_offset::NativeEndOffset,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        check()?;
        self.check_native_header()?;
        if self.resident_string_bytes().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "original end-offset resident string",
            )
            .into());
        }
        self.replace_primary_using(IntRep::NativeEndOffset(offset), &mut Some(check))
    }

    pub(crate) fn install_native_jim_index_checked<E: From<ValueError>>(
        &self,
        index: tcl_syntax::native_jim_index::JimIndex,
        current: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        if self.resident_string_bytes().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Jim Index resident string",
            )
            .into());
        }
        self.replace_primary_using(IntRep::JimIndex(index), &mut Some(current))
    }

    pub(crate) fn native_jim_regexp_cache(
        &self,
        flags: u32,
    ) -> Result<
        Option<tcl_syntax::native_regex::JimRegexpArtifact<tcl_regex::cmd_core::CompiledRegex>>,
        tcl_syntax::value::ValueError,
    > {
        self.check_native_header()?;
        match &*self.0.intrep.borrow() {
            IntRep::JimRegexp(cache) => cache.compiled_for(flags),
            IntRep::NativeRegexp(_) => {
                Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "foreign C regexp cache in Jim",
                ))
            }
            _ => Ok(None),
        }
    }

    pub(crate) fn install_native_jim_regexp(
        &self,
        flags: u32,
        program: tcl_regex::cmd_core::CompiledRegex,
    ) -> Result<
        tcl_syntax::native_regex::JimRegexpArtifact<tcl_regex::cmd_core::CompiledRegex>,
        tcl_syntax::value::ValueError,
    > {
        self.check_native_header()?;
        if self.resident_string_bytes().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim regexp resident CString source",
            ));
        }
        let cache = tcl_syntax::native_regex::JimRegexpCache::new(flags, program);
        let artifact = cache
            .compiled_for(flags)?
            .expect("live published Jim program");
        self.replace_primary(IntRep::JimRegexp(cache));
        Ok(artifact)
    }

    pub(crate) fn native_regexp_cache(
        &self,
        recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        flags: u32,
    ) -> Result<
        Option<Rc<RefCell<tcl_regex::cmd_core::CompiledRegex>>>,
        tcl_syntax::value::ValueError,
    > {
        self.check_native_header()?;
        if matches!(&*self.0.intrep.borrow(), IntRep::JimRegexp(_)) {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "foreign Jim regexp cache in C",
            ));
        }
        if let IntRep::NativeRegexp(cache) = &*self.0.intrep.borrow() {
            if cache.recipe() != recipe {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native regexp cache origin",
                ));
            }
            return Ok(cache.compiled_for(recipe, flags));
        }
        Ok(None)
    }

    pub(crate) fn native_regexp_glob(
        &self,
        recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        flags: u32,
    ) -> Result<Option<Rc<[u8]>>, tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        if let IntRep::NativeRegexp(cache) = &*self.0.intrep.borrow() {
            return Ok(cache.glob_for(recipe, flags));
        }
        Ok(None)
    }

    pub(crate) fn install_native_regexp(
        &self,
        recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        flags: u32,
        compiled: Rc<RefCell<tcl_regex::cmd_core::CompiledRegex>>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        if self.resident_string_bytes().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native regexp counted pattern storage",
            ));
        }
        *self.0.intrep.borrow_mut() =
            IntRep::NativeRegexp(tcl_syntax::native_regex::NativeRegexpCache::new(
                recipe,
                flags,
                compiled,
                &self
                    .resident_string_bytes()
                    .expect("validated regexp source"),
            ));
        Ok(())
    }

    /// Inspect the completion getter's original cache without accessing strings.
    #[must_use]
    pub fn completion_code_cache(
        &self,
    ) -> Option<tcl_cmd_core::return_options::CompletionCodeCache> {
        match *self.0.intrep.borrow() {
            IntRep::CompletionCode(cache) => Some(cache),
            _ => None,
        }
    }

    /// Retain a reached completion conversion on the original shared object.
    /// Jim's cache deliberately keeps an absent string absent.
    ///
    /// # Errors
    /// Refuses a Tcl keyword cache without its original resident spelling.
    pub fn adopt_completion_code_cache(
        &self,
        cache: tcl_cmd_core::return_options::CompletionCodeCache,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        if matches!(
            cache,
            tcl_cmd_core::return_options::CompletionCodeCache::TclKeyword(_)
        ) && self.resident_string_bytes().is_none()
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "completion keyword resident string",
            ));
        }
        *self.0.intrep.borrow_mut() = IntRep::CompletionCode(cache);
        Ok(())
    }

    /// Discard a reached native internal representation, preserving its original bytes.
    ///
    /// # Errors
    /// Refuses absence of the resident spelling required before this native stage.
    pub fn discard_native_internal_representation(
        &self,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        if self.resident_string_bytes().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native cache discard resident string",
            ));
        }
        *self.0.intrep.borrow_mut() = IntRep::Str;
        *self.0.double_format.borrow_mut() = None;
        Ok(())
    }

    /// Inspect the original native level-reference cache without string access.
    #[must_use]
    pub fn native_frame_level_cache(&self) -> Option<tcl_registry::NativeFrameLevelCache> {
        if !self.native_object_is_live() {
            return None;
        }
        match *self.0.intrep.borrow() {
            IntRep::FrameLevel { cache, .. } => Some(cache),
            _ => None,
        }
    }

    /// Obtain a level-reference cache under the original actual release.
    ///
    /// # Errors
    /// Refuses a cache donated by a different native engine release.
    pub fn native_frame_level_cache_in(
        &self,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<Option<tcl_registry::NativeFrameLevelCache>, tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        let protocol = dialect
            .native_frame_level_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable("native frame cache"))?;
        match *self.0.intrep.borrow() {
            IntRep::FrameLevel { cache, version } => {
                if protocol.tcl_version() != Some(version) || !protocol.accepts_cache(cache) {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native frame cache origin",
                    ));
                }
                Ok(Some(cache))
            }
            _ => Ok(None),
        }
    }

    /// Install a reached native level-reference cache on its original object.
    ///
    /// # Errors
    /// Refuses unsupported signed forms, missing authority or absent original bytes.
    pub fn install_native_frame_level_cache(
        &self,
        cache: tcl_registry::NativeFrameLevelCache,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        let protocol = dialect
            .native_frame_level_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable("native frame cache"))?;
        let version = protocol
            .tcl_version()
            .ok_or(ValueError::CommandProtocolUnavailable("native frame cache"))?;
        if !protocol.accepts_cache(cache) || self.resident_string_bytes().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "native frame cache storage",
            ));
        }
        *self.0.intrep.borrow_mut() = IntRep::FrameLevel { cache, version };
        Ok(())
    }
}
