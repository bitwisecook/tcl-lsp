// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lossless private storage for copied source-walk control facts.

use super::SourceBodyEntryPurpose;
use tcl_dialect::model::InvocationRealm;
use tcl_lexer::SourceSegmentationPurpose;
use tcl_registry::native_compilation::{
    NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
};

const CATCH_KNOWN: u8 = 1;
const NESTED_SCRIPT: u8 = 2;
const CONDITIONAL_BODY: u8 = 4;
const INTERPRETER_REALM: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct SourceExecutionControl {
    mode: NativeCompilationMode,
    frame: NativeCompilationFrame,
    loop_depth: u32,
    catch_depth: u32,
    flags: u8,
}

impl SourceExecutionControl {
    pub(super) fn new(
        compilation: NativeCompilationContext,
        purpose: SourceSegmentationPurpose,
        body: SourceBodyEntryPurpose,
        realm: InvocationRealm,
    ) -> Self {
        Self {
            mode: NativeCompilationMode::Unknown,
            frame: NativeCompilationFrame::Unknown,
            loop_depth: 0,
            catch_depth: 0,
            flags: 0,
        }
        .with_compilation(compilation)
        .with_source_purpose(purpose)
        .with_body_entry(body)
        .with_realm(realm)
    }

    pub(super) fn compilation(self) -> NativeCompilationContext {
        NativeCompilationContext {
            mode: self.mode,
            frame: self.frame,
            loop_depth: self.loop_depth,
            catch_depth: if self.flags & CATCH_KNOWN != 0 {
                Some(self.catch_depth)
            } else {
                None
            },
        }
    }

    pub(super) fn source_purpose(self) -> SourceSegmentationPurpose {
        if self.flags & NESTED_SCRIPT == 0 {
            SourceSegmentationPurpose::WholeInput
        } else {
            SourceSegmentationPurpose::NestedScript
        }
    }

    pub(super) fn body_entry(self) -> SourceBodyEntryPurpose {
        if self.flags & CONDITIONAL_BODY == 0 {
            SourceBodyEntryPurpose::Selected
        } else {
            SourceBodyEntryPurpose::Conditional
        }
    }

    pub(super) fn realm(self) -> InvocationRealm {
        if self.flags & INTERPRETER_REALM == 0 {
            InvocationRealm::RuleLoader
        } else {
            InvocationRealm::InterpreterRuntime
        }
    }

    pub(super) fn with_compilation(mut self, compilation: NativeCompilationContext) -> Self {
        let NativeCompilationContext {
            mode,
            frame,
            loop_depth,
            catch_depth,
        } = compilation;
        self.mode = mode;
        self.frame = frame;
        self.loop_depth = loop_depth;
        self.catch_depth = catch_depth.unwrap_or(0);
        self.flags = (self.flags & !CATCH_KNOWN) | u8::from(catch_depth.is_some());
        self
    }

    pub(super) fn with_source_purpose(mut self, purpose: SourceSegmentationPurpose) -> Self {
        match purpose {
            SourceSegmentationPurpose::WholeInput => self.flags &= !NESTED_SCRIPT,
            SourceSegmentationPurpose::NestedScript => self.flags |= NESTED_SCRIPT,
        }
        self
    }

    pub(super) fn with_body_entry(mut self, body: SourceBodyEntryPurpose) -> Self {
        match body {
            SourceBodyEntryPurpose::Selected => self.flags &= !CONDITIONAL_BODY,
            SourceBodyEntryPurpose::Conditional => self.flags |= CONDITIONAL_BODY,
        }
        self
    }

    pub(super) fn with_realm(mut self, realm: InvocationRealm) -> Self {
        match realm {
            InvocationRealm::RuleLoader => self.flags &= !INTERPRETER_REALM,
            InvocationRealm::InterpreterRuntime => self.flags |= INTERPRETER_REALM,
        }
        self
    }
}

const _: () = assert!(std::mem::size_of::<SourceExecutionControl>() <= 12);
const _: () = assert!(std::mem::size_of::<super::SourceExecutionContext<'static>>() <= 256);

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{Hash, Hasher};

    #[test]
    fn compact_control_roundtrips_full_native_boundaries_and_independent_source_facts() {
        for mode in [
            NativeCompilationMode::Direct,
            NativeCompilationMode::BytecodeObject,
            NativeCompilationMode::Unknown,
        ] {
            for frame in [
                NativeCompilationFrame::ProcedureCode,
                NativeCompilationFrame::ScriptCode,
                NativeCompilationFrame::Unknown,
            ] {
                for loop_depth in [0, 1, u32::MAX] {
                    for catch_depth in [None, Some(0), Some(1), Some(u32::MAX)] {
                        assert_control_facts(NativeCompilationContext {
                            mode,
                            frame,
                            loop_depth,
                            catch_depth,
                        });
                    }
                }
            }
        }
    }

    fn assert_control_facts(native: NativeCompilationContext) {
        for purpose in [
            SourceSegmentationPurpose::WholeInput,
            SourceSegmentationPurpose::NestedScript,
        ] {
            for body in [
                SourceBodyEntryPurpose::Selected,
                SourceBodyEntryPurpose::Conditional,
            ] {
                for realm in [
                    InvocationRealm::RuleLoader,
                    InvocationRealm::InterpreterRuntime,
                ] {
                    let control = SourceExecutionControl::new(native, purpose, body, realm);
                    assert_eq!(control.compilation(), native);
                    assert_eq!(control.source_purpose(), purpose);
                    assert!(control.body_entry() == body);
                    assert_eq!(control.realm(), realm);
                    assert_eq!(control.flags & !15, 0);
                    assert_eq!(control.catch_depth, native.catch_depth.unwrap_or(0));
                    let restored = SourceExecutionControl::new(
                        control.compilation(),
                        control.source_purpose(),
                        control.body_entry(),
                        control.realm(),
                    );
                    assert_eq!(control, restored);
                    assert_eq!(digest(control), digest(restored));
                    assert_mutation_preserves_other_facts(control, native, purpose, body, realm);
                }
            }
        }
    }

    fn digest(value: SourceExecutionControl) -> u64 {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        value.hash(&mut state);
        state.finish()
    }

    fn assert_mutation_preserves_other_facts(
        control: SourceExecutionControl,
        native: NativeCompilationContext,
        purpose: SourceSegmentationPurpose,
        body: SourceBodyEntryPurpose,
        realm: InvocationRealm,
    ) {
        let entered = native.with_inline_exception_range();
        let changed = control.with_compilation(entered);
        assert_eq!(changed.compilation(), entered);
        assert_eq!(changed.source_purpose(), purpose);
        assert!(changed.body_entry() == body);
        assert_eq!(changed.realm(), realm);
        if entered.catch_depth.is_none() {
            assert_eq!(changed.catch_depth, 0);
        }
        let changed = control.with_source_purpose(SourceSegmentationPurpose::NestedScript);
        assert_eq!(changed.compilation(), native);
        assert!(changed.body_entry() == body);
        assert_eq!(changed.realm(), realm);
        let changed = control.with_body_entry(SourceBodyEntryPurpose::Conditional);
        assert_eq!(changed.compilation(), native);
        assert_eq!(changed.source_purpose(), purpose);
        assert_eq!(changed.realm(), realm);
        let changed = control.with_realm(InvocationRealm::InterpreterRuntime);
        assert_eq!(changed.compilation(), native);
        assert_eq!(changed.source_purpose(), purpose);
        assert!(changed.body_entry() == body);
    }
}
