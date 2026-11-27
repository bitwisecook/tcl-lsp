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

//! Executable C original-object bodies, with their actual local/literal owners.

use super::native_literal_pool::{
    NativeRuntimeLiteral, NativeRuntimeLiteralAction, NativeRuntimeLiteralArray,
};
#[cfg(all(test, debug_assertions))]
use super::CmdFrame;
use super::{
    drop_fresh, line_of, new_string, obj, Cell, Code, Command, Interp, NsId, ProcDef, Rc, TclObj,
};
mod native_dictionary;
mod native_dictionary_mutation;
mod native_dictionary_scope;
mod native_error;
use native_error::ErrorOperation;
mod native_coroutine;
use native_coroutine::CoroutineOperation;
mod native_named;
use native_dictionary::DictionaryLookupOperation;
use native_dictionary_mutation::DictionaryMutationOperation;
use native_dictionary_scope::DictionaryScopeOperation;
mod native_switch;
mod native_try;
use native_named::NamedOperation;
use native_try::TryOperation;
mod native_control;
#[cfg(have_tommath)]
mod native_i64_add;
mod native_string;
mod native_string_operand;
mod native_string_range;
mod native_string_trim;
use native_string::StringMatchOperation;
use native_string_operand::StringOperandOperation;
use native_string_range::StringRangeOperation;
use native_string_trim::StringTrimOperation;
mod native_list_index;
use native_list_index::ListIndexOperation;
mod native_list_operations;
use native_list_operations::ListOperationsOperation;
mod native_array;
mod native_compiler_pass;
mod native_introspection;
mod native_mathop;
mod native_namespace_string;
mod native_scalar;
use native_namespace_string::NamespaceStringOperation;
mod native_substitution;
use native_array::ArrayOperation;
use native_introspection::IntrospectionOperation;
use native_mathop::MathopOperation;
use native_scalar::ScalarOperation;
mod native_each;
mod native_lappend;
mod native_list_conversion;
mod native_oo_next;
use native_list_conversion::{BodyListArguments, BodyListFrame};
mod native_info_exists;
mod native_unset;
mod native_upvar;
#[cfg(test)]
mod native_upvar_info_exists_tests;
use native_each::EachOperation;
use native_info_exists::InfoExistsOperation;
use native_unset::UnsetOperation;
use native_upvar::UpvarOperation;
mod native_control_preparation;
use native_control_preparation::PreparedControlOperands;
use std::collections::HashMap;
use std::rc::Weak;
use tcl_bytecode_tables::{
    LiteralTable, LocalVarTable, NativeLiteralAction, NativeLiteralAllocation,
};
use tcl_lexer::{ExecutablePart, ExecutablePartArena, NativeWord, PartListId, SourceImage, Span};
use tcl_registry::native_compilation::{
    NativeCompilationContext as Context, NativeCompilationFrame, NativeCompilationMode,
};
use tcl_registry::native_compiler_words::{NativeCompiledListRecipe, NativeCompilerWords};
use tcl_registry::native_instruction_plan::{
    NativeAppendInstruction, NativeAppendOperands, NativeArgumentListStep,
};
use tcl_runtime_api::native_compilation::NativeCompiledLocalLayout;
use tcl_syntax::native_string::NativeStringProtocol;
use tcl_syntax::native_variable_words::NativeVariableWordOperand;
use tcl_syntax::value::{ValueError, ValueOps};

mod instruction_execution;
mod operand_building;
mod operation_building;
mod preparation;
mod script_building;
mod variable_targets;
mod word_evaluation;

/// The owning declaration is non-owning in the object's native Bytecode primary.
#[derive(Clone)]
enum BodyContext {
    Script,
    Procedure(Weak<ProcDef>),
}

#[derive(Clone, PartialEq, Eq)]
struct CacheStamp {
    execution_name_policy: Option<tcl_syntax::naming::ExecutionNamePolicy>,
    interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    namespace: NsId,
    epochs: (u64, u64),
    physical: tcl_dialect::TclVersion,
    grammar: tcl_lexer::LexerConfig,
    source_protocol: NativeStringProtocol,
    observers: Vec<(u64, Option<u64>, u8)>,
    steps: Vec<(Option<u64>, u8, Vec<u8>)>,
    borrowed_table: Option<usize>,
    fixed_math: Option<tcl_runtime_api::native_compilation::NativeMathFunctionTable>,
    expression_policy: Option<tcl_runtime_api::expression_policy::ExpressionEvaluationPolicy>,
}

pub(super) struct NativeBodyArtifact {
    stamp: CacheStamp,
    owner: BodyContext,
    image: SourceImage,
    region: Span,
    scripts: HashMap<Span, Script>,
    literals: Rc<NativeRuntimeLiteralArray>,
    locals: Option<NativeCompiledLocalLayout>,
    // Nonowning borrowed layout: a cached script must not extend procedure
    // resources merely by remembering their original table identity.
    _borrowed_table:
        Option<Weak<tcl_runtime_api::native_literal::NativeLocalNameTable<obj::Owned>>>,
}

struct Script {
    commands: Vec<CommandInstruction>,
    fatal: Option<SyntaxInstruction>,
    empty: Option<usize>,
}
struct SyntaxInstruction {
    cut: tcl_lexer::NativeScriptWordCut,
    message: usize,
    options: usize,
}
struct CommandInstruction {
    source_extent: Span,
    words: Vec<WordInstruction>,
    operation: Operation,
    guard: Option<tcl_registry::native_compilation::NativeCompilationGuard>,
}
struct WordInstruction {
    original: NativeWord,
    literal: Option<usize>,
    arena: ArenaInstruction,
}
struct ArenaInstruction {
    original: ExecutablePartArena,
    texts: HashMap<Span, usize>,
    locals: HashMap<Span, usize>,
    roots: HashMap<Span, usize>,
    syntax: HashMap<Span, (usize, usize)>,
}
type RetainedBodyLocalTable = Rc<tcl_runtime_api::native_literal::NativeLocalNameTable<obj::Owned>>;

type NativeArenaFrame = (
    PartListId,
    usize,
    Vec<obj::Owned>,
    Option<(Span, Option<usize>)>,
);

enum Operation {
    Scalar(ScalarOperation),
    NamespaceString(NamespaceStringOperation),
    MathOperator(MathopOperation),
    Introspection(IntrospectionOperation),
    Array(Box<ArrayOperation>),
    ListIndex(ListIndexOperation),
    ListOperations(ListOperationsOperation),
    StringMatch(StringMatchOperation),
    StringTrim(StringTrimOperation),
    StringOperand(StringOperandOperation),
    StringRange(StringRangeOperation),
    Error(ErrorOperation),
    Coroutine(CoroutineOperation),
    DictionaryLookup(DictionaryLookupOperation),
    DictionaryScope(DictionaryScopeOperation),
    DictionaryMutation(DictionaryMutationOperation),
    NamedInvocation(NamedOperation),
    Expression(Box<native_control::ExpressionOperation>),
    Try(TryOperation),
    Break,
    Continue,
    Control(Box<native_control::ControlOperation>),
    Each(EachOperation),
    NamespaceBindings(NamespaceOperation),
    Upvar(UpvarOperation),
    InfoExists(InfoExistsOperation),
    Switch(SwitchOperation),
    TclOoHelper(
        tcl_registry::native_tcloo_compilation::NativeTclOoInstruction,
        Vec<Option<usize>>,
    ),
    Invoke,
    List(NativeCompiledListRecipe, Option<usize>),
    /// A List the parser flattened from a literal `{*}` word.
    ListOperands {
        operands: Vec<NamespaceOperand>,
        prepared_words: HashMap<usize, WordInstruction>,
    },
    FormatStringConcat {
        operands: Vec<NamespaceOperand>,
        prepared_words: HashMap<usize, WordInstruction>,
    },
    Concat {
        literal: Option<usize>,
        operands: Vec<NamespaceOperand>,
        prepared_words: HashMap<usize, WordInstruction>,
    },
    Unset(UnsetOperation),
    Load(Target),
    Store(Target, usize),
    Increment(Target, Option<usize>, Option<i32>),
    Append(Target, NativeAppendInstruction),
    SelectedReturn(
        tcl_registry::native_return_compilation::NativeReturnInstruction,
        Option<usize>,
        Option<usize>,
    ),
    Uplevel(
        tcl_registry::native_instruction_plan::NativeUplevelInstruction,
        Option<usize>,
    ),
}

enum NamespaceOperand {
    Original(usize),
    Literal(usize),
}
struct NamespaceBindingInstruction {
    name: NamespaceOperand,
    slot: usize,
    local: Vec<u8>,
    value: Option<NamespaceOperand>,
}
struct NamespaceOperation {
    kind: tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingKind,
    bindings: Vec<NamespaceBindingInstruction>,
    generic: Option<Vec<NamespaceOperand>>,
    prepared_words: HashMap<usize, WordInstruction>,
    prefix: Option<usize>,
    namespace: Option<NamespaceOperand>,
    empty: Option<usize>,
}

struct SwitchOperation {
    recipe: tcl_registry::native_switch_compilation::NativeSwitchInstruction,
    patterns: Vec<Option<usize>>,
    subject_literal: Option<usize>,
    empty: Option<usize>,
}

/// `DONE` exits this complete compiled procedure, including bracket programs,
/// without creating Tcl return-option state.
#[derive(Default)]
struct BodyExecution {
    done: bool,
    #[cfg(have_tommath)]
    native_i64_add: Option<Box<native_i64_add::EnteredI64AddPlan>>,
    #[cfg(have_tommath)]
    native_i64_ran: bool,
}
struct Target {
    original: NativeVariableWordOperand,
    slot: Option<usize>,
    index: Option<ArenaInstruction>,
    root_literal: Option<usize>,
    index_literal: Option<usize>,
}

struct EvaluatedTarget {
    root: Vec<u8>,
    element: Option<Vec<u8>>,
    original_name: Option<obj::Owned>,
    original_index: Option<obj::Owned>,
    combined: bool,
}

extern "C" fn free_body(value: *mut TclObj) {
    // SAFETY: the exact descriptor owns one boxed Rc, not its source object.
    unsafe {
        drop(Box::from_raw(
            obj::internal_rep(value) as usize as *mut Rc<NativeBodyArtifact>
        ));
    }
}
extern "C" fn duplicate_body(_source: *mut TclObj, _target: *mut TclObj) {
    // Native Bytecode duplication leaves the fresh duplicate string-only.
}
static BYTECODE_TYPE: obj::TclObjType = obj::TclObjType {
    name: c"bytecode".as_ptr(),
    free_int_rep_proc: Some(free_body),
    dup_int_rep_proc: Some(duplicate_body),
    update_string_proc: None,
    set_from_any_proc: None,
};

/// Whether `value` holds the core `bytecode` representation.
pub(crate) fn is_bytecode(value: *mut TclObj) -> bool {
    core::ptr::eq(obj::obj_type_ptr(value), &BYTECODE_TYPE)
}

fn cache(value: *mut TclObj) -> Option<Rc<NativeBodyArtifact>> {
    if !core::ptr::eq(obj::obj_type_ptr(value), &BYTECODE_TYPE) {
        return None;
    }
    // SAFETY: exact descriptor identity authenticates this boxed owner.
    Some(unsafe { &*(obj::internal_rep(value) as usize as *const Rc<NativeBodyArtifact>) }.clone())
}
fn install<T>(
    value: *mut TclObj,
    artifact: &Rc<T>,
    descriptor: *const obj::TclObjType,
    entered: &native_list_conversion::BodyObjectOperation,
    actual: &Interp,
) -> Result<(), tcl_cmd_core::CmdError> {
    entered.check(actual)?;
    let incoming = Box::into_raw(Box::new(Rc::clone(artifact)));
    let reached =
        obj::change_type_with_stage_check(value, descriptor, incoming as usize as u64, &mut || {
            entered.check(actual)?;
            obj::check_native_liveness(value).map_err(|error| entered.refuse(error))
        });
    match reached {
        Ok(()) => Ok(()),
        Err(error) => {
            let first = entered.retain(error);
            // The same primary worker leaves this exact incoming owner unadopted.
            unsafe { drop(Box::from_raw(incoming)) };
            entered.check(actual)?;
            Err(first)
        }
    }
}

struct Builder<'a> {
    interp: &'a mut Interp,
    image: SourceImage,
    context: Context,
    stamp: CacheStamp,
    lvt: LocalVarTable,
    literals: LiteralTable,
    scripts: HashMap<Span, Script>,
    parse_failure: Option<tcl_lexer::NativeScriptWordCut>,
    compilation_failure: Option<tcl_registry::native_compilation::NativeCompilationFailure>,
    private_objects: HashMap<usize, obj::Owned>,
    procedure: Option<Weak<ProcDef>>,
}

static NEXT_LAYOUT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
fn next_layout_token() -> u64 {
    tcl_runtime_api::checked_counter::allocate(&NEXT_LAYOUT)
        .expect("native compiled layout tokens exhausted")
}

pub(crate) fn cache_snapshot(
    value: *mut TclObj,
) -> Option<tcl_syntax::native_object::NativeObjectCacheSnapshot> {
    cache(value).map(
        |artifact| tcl_syntax::native_object::NativeObjectCacheSnapshot::Bytecode {
            version: artifact.stamp.physical,
        },
    )
}

fn unavailable(purpose: &'static str) -> ValueError {
    ValueError::CommandProtocolUnavailable(purpose)
}

impl NativeBodyArtifact {
    pub(super) fn compiled_local_layout(&self) -> Option<&NativeCompiledLocalLayout> {
        self.locals.as_ref()
    }
}

#[cfg(test)]
mod control_expression_tests;
#[cfg(test)]
#[path = "native_body_artifact/format_tests.rs"]
mod format_tests;
#[cfg(test)]
#[path = "native_body_artifact/namespace_tests.rs"]
mod namespace_tests;
#[cfg(test)]
mod object_info_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "native_body_artifact/original_return_parent_tests.rs"]
mod original_return_parent_tests;

#[cfg(test)]
#[path = "native_body_artifact/original_word_input_tests.rs"]
mod original_word_input_tests;

#[cfg(test)]
#[path = "native_body_artifact/original_preparation_boundary_tests.rs"]
mod original_preparation_boundary_tests;

#[cfg(test)]
#[path = "native_body_artifact/original_variable_boundary_tests.rs"]
mod original_variable_boundary_tests;
