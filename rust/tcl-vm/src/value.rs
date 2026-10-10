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

//! The VM value model — an idiomatic `Rc`-based dual-rep value reproducing Tcl
//! shimmering *behaviourally* without the 24-byte wasm32 ABI layout.
//!
//! A [`Value`] is a cheap-to-clone handle (`Rc` bump). It caches a lazily
//! generated string rep alongside a typed internal rep; reading a typed
//! accessor parses-and-caches (string→typed), `to_str` generates-and-caches
//! (typed→string). `Rc` strong-count is the shared/unshared signal for the
//! copy-on-write list ops. See the steering doc §4b.

#![allow(clippy::cast_precision_loss)] // i64→f64 numeric coercion is intentional

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

mod native_list_storage;
mod native_namespace_name;
mod native_retirement;

use tcl_cmd_core::namespace::TclStringHashOrder;
use tcl_core_types::RecursionLimit;
use tcl_syntax::list;
use tcl_syntax::native_string::{NativeStringProtocol, NativeStringStorageIdentity};
use tcl_syntax::number::{self, Number};
use tcl_syntax::raw_string::{RawString, UnicodeAccessError};
use tcl_syntax::scalar_getter::{
    NativeScalarCache, NativeScalarGetterKind, NativeScalarGetterProtocol, NativeScalarGetterValue,
};
use tcl_syntax::value::{ValueError, canonical_dict_slots};

use crate::error::TclError;

/// Depth cap for [`Value::to_str`]'s descent into nested `IntRep::List`
/// children. A plain `for {set i 0} {$i<N} {incr i} {set v
/// [list $v]}` loop builds a value that nests one list inside another N
/// times; forcing its string form (`string length $v`, `puts $v`, a
/// comparison, …) with no depth cap would recurse once per nesting level
/// — no `{*}` tricks needed, `dict`'s printing shares this path (a dict is
/// represented as a list here too). Empirically, unguarded input overflows
/// the native stack (SIGABRT) between
/// depth 1200 and 1250 on a 2 MiB thread (`cargo test`'s per-test default).
/// 256 leaves better than 4x margin under that measured crash floor while
/// staying far past any realistic nested-list depth. Past the cap, a
/// nested list element is rendered as [`TOO_DEEPLY_NESTED_PLACEHOLDER`]
/// instead of its true (recursively generated) string — see
/// [`Value::to_raw_string_at_depth`].
const MAX_LIST_TO_STR_DEPTH: RecursionLimit = RecursionLimit(256);

/// The placeholder text substituted for a nested list element past
/// [`MAX_LIST_TO_STR_DEPTH`] — see [`Value::to_raw_string_at_depth`]. Passed
/// through [`list::join_list`] like any other element, so it is quoted
/// exactly as safely as real data would be.
const TOO_DEEPLY_NESTED_PLACEHOLDER: &str = "<list too deeply nested to render, see issue #996>";

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct DoubleEngineKey {
    point: Option<tcl_dialect::model::DialectPoint>,
    version: Option<tcl_dialect::TclVersion>,
    policy: tcl_dialect::DoubleStringPolicy,
}

thread_local! {
    static DOUBLE_PRECISIONS: RefCell<HashMap<DoubleEngineKey, Rc<Cell<u8>>>> = RefCell::new(HashMap::new());
}

/// Lazy conversion retains engine identity and observes its current thread precision.
#[derive(Clone)]
pub(crate) struct DoubleFormatContext {
    policy: tcl_dialect::DoubleStringPolicy,
    precision: Rc<Cell<u8>>,
}

impl DoubleFormatContext {
    pub(crate) fn authored_tcl84(precision: Rc<Cell<u8>>) -> Self {
        Self {
            policy: tcl_dialect::DoubleStringPolicy::Tcl84Precision,
            precision,
        }
    }

    pub(crate) fn for_dialect(dialect: tcl_registry::InvocationDialect) -> Option<Self> {
        let policy = dialect.double_string_policy()?;
        let key = DoubleEngineKey {
            point: dialect.core_point,
            version: dialect.tcl_version,
            policy,
        };
        let precision = DOUBLE_PRECISIONS.with(|values| {
            Rc::clone(
                values
                    .borrow_mut()
                    .entry(key)
                    .or_insert_with(|| Rc::new(Cell::new(policy.default_precision()))),
            )
        });
        Some(Self { policy, precision })
    }

    pub(crate) fn precision(&self) -> u8 {
        self.precision.get()
    }

    pub(crate) fn set_precision(&self, precision: u8) {
        assert!(
            self.policy.format(precision).is_some(),
            "native precision must be validated"
        );
        self.precision.set(precision);
    }

    fn format(&self) -> tcl_dialect::DoubleFormat {
        self.policy
            .format(self.precision())
            .expect("validated native precision")
    }
}

/// A Tcl value handle. Cloning retains an actual native object reference.
pub struct Value(Rc<Obj>, NativeValueHandleOwnership);

#[derive(Clone, Copy, PartialEq, Eq)]
enum NativeValueHandleOwnership {
    Reference,
    LifetimeOnly,
}

impl Clone for Value {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0), NativeValueHandleOwnership::Reference)
    }
}

impl Drop for Value {
    fn drop(&mut self) {
        if self.1 == NativeValueHandleOwnership::LifetimeOnly {
            self.release_native_lifetime_pin();
        } else if Rc::strong_count(&self.0) == self.0.lifetime_pins.get() + 1 {
            self.retire_native_header();
        }
    }
}

/// Lifetime-only original header lease used by authenticated engine transports.
pub struct NativeObjectLifetimeLease(Option<Value>);

impl NativeObjectLifetimeLease {
    /// Inspect the same original header without adding a native object reference.
    #[must_use]
    pub fn value(&self) -> &Value {
        self.0.as_ref().expect("live original-object lease")
    }

    /// Transfer this lifetime lease into a completion transport.
    /// A receiver that owns a native reference must explicitly promote it.
    #[must_use]
    pub fn into_value(mut self) -> Value {
        let mut value = self.0.take().expect("live original-object lease");
        value.1 = NativeValueHandleOwnership::LifetimeOnly;
        value
    }
}
impl Clone for NativeObjectLifetimeLease {
    fn clone(&self) -> Self {
        self.value().native_lifetime_lease()
    }
}
impl Drop for NativeObjectLifetimeLease {
    fn drop(&mut self) {
        if let Some(value) = self.0.as_ref() {
            value.release_native_lifetime_pin();
        }
    }
}

/// Non-owning access to an original object. This handle changes neither native
/// sharing nor storage; its identity carries no namespace or literal authority.
#[derive(Clone)]
pub struct WeakNativeObject(std::rc::Weak<Obj>);

impl WeakNativeObject {
    /// Retain the same original object while a genuine owner still exists.
    #[must_use]
    pub fn upgrade(&self) -> Option<Value> {
        self.0
            .upgrade()
            .filter(|object| !object.retired.get())
            .map(|object| Value(object, NativeValueHandleOwnership::Reference))
    }
}

/// Actual per-interpreter Jim objects used by source conversion.
/// Sharing this holder adds no references to its original objects.
#[path = "value_script.rs"]
mod script;
pub(crate) use script::{NativeJimScript, NativeJimScriptLease};
mod native_jim_enum;
mod native_jim_lookup;
mod native_method_name;
mod native_property_name;
mod native_substitution;
mod native_variable_name;
pub(crate) use native_jim_lookup::{JimCommandCache, JimVariableCache};

pub(crate) struct NativeJimObjectContext {
    numeric_host: RefCell<Option<Rc<dyn tcl_platform::Host>>>,
    live: std::cell::Cell<bool>,
    objects: [RefCell<Option<Value>>; 10],
}

impl NativeJimObjectContext {
    pub(crate) fn new(
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<Rc<Self>, tcl_syntax::value::ValueError> {
        if dialect.native_string_protocol() != Some(NativeStringProtocol::Jim084) {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim original source context",
            ));
        }
        let empty = Value::new_native_string_bytes(b"".as_slice());
        let context = Rc::new(Self {
            numeric_host: RefCell::new(None),
            live: std::cell::Cell::new(true),
            objects: [
                Some(empty.clone()),
                Some(Value::int(1)),
                Some(Value::int(0)),
                Some(empty.clone()),
                None,
                Some(empty),
                Some(Value::new_native_string_bytes(b"unknown".as_slice())),
                Some(Value::new_native_string_bytes(b"jim::defer".as_slice())),
                Some(Value::new_native_string_bytes(b"".as_slice())),
                Some(Value::new_native_string_bytes(b"".as_slice())),
            ]
            .map(RefCell::new),
        });
        for object in &context.objects {
            let object = object.borrow();
            let Some(value) = object.as_ref() else {
                continue;
            };
            value.bind_native_jim_context(&context)?;
        }
        Ok(context)
    }

    pub(crate) fn select_numeric_host(&self, host: Rc<dyn tcl_platform::Host>) {
        *self.numeric_host.borrow_mut() = Some(host);
    }

    fn fresh_numeric_conversion(
        &self,
        protocol: tcl_syntax::scalar_getter::NativeScalarGetterProtocol,
        kind: NativeScalarGetterKind,
        original: &[u8],
    ) -> Result<
        tcl_syntax::scalar_getter::NativeScalarGetterConversion,
        tcl_syntax::value::ValueError,
    > {
        if !self.is_live() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "retired Jim interpreter",
            ));
        }
        let host = self
            .numeric_host
            .borrow()
            .clone()
            .ok_or(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)?;
        let environment = host
            .numeric_environment()
            .ok_or(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)?;
        tcl_cmd_core::native_numeric::fresh_jim_conversion(protocol, kind, original, environment)
    }

    pub(crate) fn is_live(&self) -> bool {
        self.live.get()
    }
    pub(crate) fn defer_object(&self) -> std::cell::Ref<'_, Value> {
        self.object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::Defer)
    }
    fn object(
        &self,
        role: tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole,
    ) -> std::cell::Ref<'_, Value> {
        std::cell::Ref::map(self.objects[role as usize].borrow(), |object| {
            object.as_ref().expect("live Jim interpreter object role")
        })
    }
    pub(crate) fn empty_object(&self) -> std::cell::Ref<'_, Value> {
        self.object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::Empty)
    }
    pub(crate) fn null_script_object(&self) -> std::cell::Ref<'_, Value> {
        self.object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::NullScript)
    }
    pub(crate) fn current_filename_object(&self) -> Value {
        self.object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::CurrentFilename)
            .clone()
    }
    pub(crate) fn release_object(
        &self,
        role: tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole,
    ) {
        let retired = self.objects[role as usize].borrow_mut().take();
        drop(retired);
    }
    pub(crate) fn retire(&self) {
        self.live.set(false);
        let host = self.numeric_host.borrow_mut().take();
        drop(host);
    }
    pub(crate) fn with_result_object<R>(&self, observer: impl FnOnce(&Value) -> R) -> R {
        observer(&self.object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::Result))
    }
    pub(crate) fn result_object(&self) -> Value {
        self.object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::Result)
            .clone()
    }
    pub(crate) fn publish_result(&self, value: &Value) {
        let retired = self.objects
            [tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::Result as usize]
            .borrow_mut()
            .replace(value.clone());
        drop(retired);
    }
    pub(crate) fn replace_current_filename(
        &self,
        value: &Value,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let context = self
            .object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::CurrentFilename)
            .native_jim_context()?;
        value.bind_native_jim_context(&context)?;
        let retired = self.objects
            [tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::CurrentFilename as usize]
            .borrow_mut()
            .replace(value.clone());
        drop(retired);
        Ok(())
    }
}

impl Drop for NativeJimObjectContext {
    fn drop(&mut self) {
        // Interpreter teardown already consumes these roles when installed.
        // Standalone selected contexts own the same ordered references.
        for object in &mut self.objects {
            drop(object.get_mut().take());
        }
    }
}

#[derive(Clone)]
pub(crate) struct NativeJimSourceInfo {
    pub(crate) filename: Value,
    pub(crate) line: i32,
}

#[derive(Clone)]
struct NativeJimSource {
    info: NativeJimSourceInfo,
    context: std::rc::Weak<NativeJimObjectContext>,
}

struct Obj {
    /// Explicit transport references that retain lifetime without native ownership.
    lifetime_pins: Cell<usize>,
    retired: Cell<bool>,
    /// Cached string rep (Tcl `bytes`). `None` until first generated from a
    /// typed rep. `RefCell` so a read can lazily cache without `&mut`.
    string: RefCell<Option<RawString>>,
    string_storage: Cell<NativeStringStorageIdentity>,
    /// Typed internal rep; `RefCell` so a value can shimmer in place.
    intrep: RefCell<IntRep>,
    double_format: RefCell<Option<DoubleFormatContext>>,
    source_location: RefCell<Option<tcl_runtime_api::script_source_location::ScriptSourceLocation>>,
    jim_context: RefCell<Option<std::rc::Weak<NativeJimObjectContext>>>,
}

#[derive(Clone)]
struct ExpressionCache {
    policy: crate::compiled::NativeCompilerPolicy,
    profile: tcl_dialect::DialectProfileKey,
    dialect: tcl_registry::InvocationDialect,
    node: Option<tcl_syntax::expr::NativeExprNode>,
    jim: Option<Rc<tcl_syntax::expr::native_objects::JimExpressionObjects<Value>>>,
}

pub(crate) struct JimExpressionInstall<'a> {
    pub(crate) dialect: tcl_registry::InvocationDialect,
    pub(crate) profile: tcl_dialect::DialectProfileKey,
    pub(crate) policy: crate::compiled::NativeCompilerPolicy,
    pub(crate) node: &'a tcl_syntax::expr::NativeExprNode,
    pub(crate) source: &'a [u8],
    pub(crate) preparation: &'a tcl_syntax::expr::native_objects::JimExpressionPreparation,
    pub(crate) info: &'a NativeJimSourceInfo,
}

/// Actual compiler context retained with the original executable source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeBytecodeContext {
    Script,
    Procedure,
    Substitution(tcl_runtime_api::native_substitution::NativeSubstitutionFlags),
}

/// Original admitted executable body, retained independently of activation pins.
#[derive(Clone)]
pub(crate) struct NativeBytecodeCache {
    pub(crate) version: tcl_dialect::TclVersion,
    pub(crate) context: NativeBytecodeContext,
    /// Actual borrowed local cache at Subst compilation, independent of procPtr.
    pub(crate) substitution_layout:
        Option<tcl_runtime_api::native_compilation::NativeCompiledLocalLayout>,
    pub(crate) substitution_table: Option<Rc<crate::literal_pool::NativeLocalNameTable>>,
    pub(crate) unit: crate::compiled::CompiledUnit,
    /// Native ByteCode.procPtr is nonowning and never adds a Proc reference.
    pub(crate) procedure: RefCell<std::rc::Weak<crate::command::ProcDef>>,
}

/// Concrete primary storage; snapshots alone grant no execution permission.
#[derive(Clone)]
enum IntRep {
    /// No typed rep: the value *is* its string (string is always `Some`).
    Str,
    /// Actual C string cache, installed only by selected native string access.
    NativeString {
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        num_chars: Option<usize>,
        unicode: Option<Rc<[u32]>>,
    },
    /// Pinned Jim's string intrep, including its recorded UTF8 character count.
    JimString(Option<usize>),
    /// Binary payload, independent of a lazily generated Unicode string.
    ByteArray(Rc<ByteArrayRep>),
    /// Genuine Jim Source owns its original filename and signed native line.
    JimSource(NativeJimSource),
    JimScript(script::NativeJimScriptHeader),
    JimDictionarySubstitution {
        name: Value,
        index: Value,
    },
    JimInterpolated(native_substitution::JimInterpolated),
    JimScriptLine {
        argc: i32,
        line: i32,
    },
    /// An expression source compiled under its exact native grammar.
    Expression(Rc<ExpressionCache>),
    /// Real original C compiled pattern; no string updater.
    NativeRegexp(tcl_syntax::native_regex::NativeRegexpCache<tcl_regex::cmd_core::CompiledRegex>),
    JimIndex(tcl_syntax::native_jim_index::JimIndex),
    NativeEndOffset(tcl_syntax::native_end_offset::NativeEndOffset),
    JimRegexp(tcl_syntax::native_regex::JimRegexpCache<tcl_regex::cmd_core::CompiledRegex>),
    NativePropertyName(native_property_name::NativePropertyName),
    NativeInstructionName(tcl_syntax::native_instruction_name::NativeInstructionName),
    NativeMethodName(Rc<crate::cmd_oo::native_method_cache::NativeMethodChain>),
    /// Actual admitted C executable body artifact on its original resident object.
    NativeBytecode(Rc<NativeBytecodeCache>),
    /// A wide integer.
    Int(i64),
    /// C Tcl 8.4 native long, distinct from its wide integer primary cache.
    Tcl84Long(i64),
    /// Complete native magnitude, independently of a wrapped getter return.
    Big {
        negative: bool,
        radix: number::Radix,
        digits: String,
    },
    /// A double.
    Double(f64),
    /// Jim's double view retaining the exact integer and integer string updater.
    CoercedDouble(i64),
    /// A boolean (canonicalises to `"0"`/`"1"` string-side).
    Bool(bool),
    /// Native word-Boolean descriptor, distinct from the integer constructor.
    WordBoolean {
        value: bool,
        version: tcl_dialect::TclVersion,
    },
    /// The original completion getter's exact keyword or Jim return-code cache.
    CompletionCode(tcl_cmd_core::return_options::CompletionCodeCache),
    /// Original native frame lookup's signed level-reference cache.
    FrameLevel {
        cache: tcl_registry::NativeFrameLevelCache,
        version: tcl_dialect::TclVersion,
    },
    /// Native option-table cache retaining its actual original table owner.
    NativeIndex {
        cache: tcl_core_types::NativeIndexCache,
        version: tcl_dialect::TclVersion,
    },
    /// Resident-only legacy native array-search handle conversion.
    NativeArraySearch {
        cache: tcl_core_types::NativeArraySearchCache,
        version: tcl_dialect::TclVersion,
    },
    /// Native named-command identities, without callable or body ownership.
    NativeCommandName(tcl_runtime_api::native_command_name::NativeCommandNameCache),
    /// C8 name lookup caches closed absence without a command node.
    NativeCommandNameUnresolved(tcl_dialect::TclVersion),
    /// Pinned Jim command descriptor, owning only its original namespace object.
    JimCommand(JimCommandCache),
    /// Pinned Jim variable descriptor with a weak actual `VarVal` birth receipt.
    JimVariable(JimVariableCache),
    /// Actual Jim Enum or immediate-string cache, with no updater.
    JimOption(tcl_core_types::NativeJimOptionCache),
    /// C parsed variable name owns original parts, never a selected receiver.
    NativeParsedVariableName(native_variable_name::ParsedVariableName),
    /// C indexed local lookup retains its actual procedure or canonical name.
    NativeLocalVariableName(native_variable_name::LocalVariableName),
    /// Original native namespace-name descriptor with actual lifetime receipts.
    NativeNamespaceName(tcl_runtime_api::native_namespace_name::NativeNamespaceNameCache),
    /// A list (`Rc` for O(1) clone + copy-on-write).
    List {
        items: crate::NativeListItems,
        canonical: Rc<Cell<bool>>,
        string_protocol: Cell<Option<NativeStringProtocol>>,
    },
    /// A dictionary with its retained Tcl hash-table growth history.
    Dict(Rc<DictRep>),
}

impl IntRep {
    fn native_list(
        items: Vec<Value>,
        canonical: bool,
        protocol: Option<NativeStringProtocol>,
    ) -> Self {
        let items = crate::NativeListItems::new(items, canonical);
        Self::List {
            canonical: items.canonical_state(),
            items,
            string_protocol: Cell::new(protocol),
        }
    }
}

#[derive(Clone)]
struct ByteArrayRep {
    bytes: Rc<[u8]>,
    /// Selected at native string ingress and retained with the physical backing.
    string_protocol: Cell<Option<tcl_syntax::native_string::NativeStringProtocol>>,
    /// A string conversion cache belongs to the selected conversion protocol.
    /// A real byte-array producer instead carries no conversion dependency.
    conversion: Option<tcl_registry::native_binary_value::NativeBinaryByteConversion>,
    /// Legacy narrowing can retain a wider original string; that cache is not
    /// a proper Tcl9 byte-array receipt after an explicit profile change.
    proper: bool,
}

#[derive(Clone, Debug)]
struct DictContents {
    pairs: Rc<Vec<(Value, Value)>>,
    hash_order: TclStringHashOrder,
}

/// An actual dictionary backing survives object shimmering and active searches.
/// Search ownership does not create a copy-on-write dictionary table.
#[derive(Debug)]
struct DictRep {
    contents: RefCell<DictContents>,
    epoch: Cell<u64>,
    string_protocol: Cell<Option<NativeStringProtocol>>,
}

impl DictRep {
    fn new(
        pairs: Vec<(Value, Value)>,
        hash_order: TclStringHashOrder,
        protocol: Option<NativeStringProtocol>,
    ) -> Self {
        Self {
            contents: RefCell::new(DictContents {
                pairs: Rc::new(
                    pairs
                        .into_iter()
                        .map(|(key, value)| {
                            (key.into_native_reference(), value.into_native_reference())
                        })
                        .collect(),
                ),
                hash_order,
            }),
            epoch: Cell::new(1),
            string_protocol: Cell::new(protocol),
        }
    }

    /// Lease current member storage without retaining each child object.
    fn pairs_backing(&self) -> Rc<Vec<(Value, Value)>> {
        Rc::clone(&self.contents.borrow().pairs)
    }

    fn with_pairs<R>(&self, operation: impl FnOnce(&[(Value, Value)]) -> R) -> R {
        operation(&self.contents.borrow().pairs)
    }

    fn bucket_count(&self) -> usize {
        self.contents.borrow().hash_order.bucket_count()
    }

    fn next_epoch(&self) -> Result<u64, tcl_syntax::value::ValueError> {
        self.epoch.get().checked_add(1).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native dictionary epoch capacity",
            ),
        )
    }
}

/// Failure of a native byte-array object conversion, separate from guest
/// conversion errors and checked host Unicode access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteArrayAccessError {
    /// The selected native proper-byte conversion rejected a Unicode unit.
    Conversion(tcl_registry::native_binary_value::NativeBinaryByteError),
    /// A Unicode-only conversion cannot represent the original native bytes.
    Unicode(UnicodeAccessError),
    /// Original storage has no checked native string materialisation recipe.
    Unavailable(tcl_syntax::native_string::NativeStringUnavailable),
}

impl From<ByteArrayAccessError> for tcl_cmd_core::CmdError {
    fn from(error: ByteArrayAccessError) -> Self {
        match error {
            ByteArrayAccessError::Conversion(error) => {
                Self::with_error_code(error.message(), "TCL VALUE BYTES")
            }
            ByteArrayAccessError::Unicode(error) => Self::from(error),
            ByteArrayAccessError::Unavailable(_) => {
                Self::from(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native binary operand string",
                ))
            }
        }
    }
}

/// Native dictionary search owns the transferred original root and its reached
/// backing. Exhaustion releases the backing while the iterator still owns root.
pub(crate) struct NativeDictionarySearch {
    root: Option<Value>,
    backing: Option<Rc<DictRep>>,
    next: usize,
    epoch: u64,
}

impl NativeDictionarySearch {
    pub(crate) fn next_pair(
        &mut self,
    ) -> Result<Option<(Value, Value)>, tcl_syntax::value::ValueError> {
        let Some(backing) = self.backing.as_ref() else {
            return Ok(None);
        };
        if backing.epoch.get() != self.epoch {
            return Err(tcl_syntax::value::ValueError::NativeFatalCondition(
                tcl_syntax::raw_string::NativeFatalCondition::DictionarySearchConcurrentMutation,
            ));
        }
        let pair = backing.with_pairs(|pairs| pairs.get(self.next).cloned());
        self.next += usize::from(pair.is_some());
        if pair.is_none() {
            self.backing = None;
        }
        Ok(pair)
    }

    /// Borrow the next original pair at the native search frontier. The
    /// lifetime leases add no Tcl references; backing epoch checks remain the
    /// same as the owning result projection.
    pub(crate) fn next_original_pair(
        &mut self,
    ) -> Result<Option<(Value, Value)>, tcl_syntax::value::ValueError> {
        let Some(backing) = self.backing.as_ref() else {
            return Ok(None);
        };
        if backing.epoch.get() != self.epoch {
            return Err(tcl_syntax::value::ValueError::NativeFatalCondition(
                tcl_syntax::raw_string::NativeFatalCondition::DictionarySearchConcurrentMutation,
            ));
        }
        let pair = backing.with_pairs(|pairs| {
            pairs.get(self.next).map(|(key, value)| {
                (
                    key.native_lifetime_lease().into_value(),
                    value.native_lifetime_lease().into_value(),
                )
            })
        });
        self.next += usize::from(pair.is_some());
        if pair.is_none() {
            self.backing = None;
        }
        Ok(pair)
    }

    /// Close an unfinished native search while retaining the iterator's root.
    pub(crate) fn close_search(&mut self) {
        self.backing = None;
    }

    /// Borrow the original root without manufacturing a native reference.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn original_root(&self) -> &Value {
        self.root.as_ref().expect("live native search root")
    }
}

impl Drop for NativeDictionarySearch {
    fn drop(&mut self) {
        // Native iterator destruction closes its DictSearch before releasing
        // the original root object, including empty and exhausted searches.
        self.close_search();
        drop(self.root.take());
    }
}

/// Prepared physical Dictionary root, with its original COW decision retained.
pub(crate) struct PreparedNativeDictionary {
    value: Value,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
}

impl PreparedNativeDictionary {
    pub(crate) fn invalidate_string(&self) {
        *self.value.0.string.borrow_mut() = None;
        self.value
            .0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *self.value.0.double_format.borrow_mut() = None;
        *self.value.0.source_location.borrow_mut() = None;
    }

    pub(crate) fn original(&self) -> &Value {
        &self.value
    }
    pub(crate) fn is_same_object(&self, value: &Value) -> bool {
        Rc::ptr_eq(&self.value.0, &value.0)
    }

    pub(crate) fn duplicate_value(&self, value: &Value) -> Value {
        value.duplicate_native_object_in(self.protocol)
    }

    pub(crate) fn with_member<R>(
        &self,
        key: &Value,
        operation: impl FnOnce(Option<&Value>) -> R,
    ) -> Result<R, tcl_syntax::value::ValueError> {
        let bytes = key.native_string_bytes(self.protocol).map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native dictionary key updater",
            )
        })?;
        Ok(self
            .value
            .with_cached_dictionary_member(&bytes, operation)
            .expect("prepared Dictionary primary cache"))
    }
    pub(crate) fn set_member(
        &mut self,
        key: Value,
        value: Value,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.value
            .set_native_dictionary_member_in_place(key, value, self.protocol)
    }
    pub(crate) fn remove_member(
        &mut self,
        key: &Value,
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        let bytes = key.native_string_bytes(self.protocol).map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native dictionary key updater",
            )
        })?;
        let cache = self.value.0.intrep.borrow();
        let IntRep::Dict(dict) = &*cache else {
            unreachable!("prepared Dictionary cache");
        };
        let mut contents = dict.contents.borrow_mut();
        let Some(index) = contents
            .pairs
            .iter()
            .position(|(name, _)| name.resident_string_bytes().is_some_and(|key| key == bytes))
        else {
            return Ok(false);
        };
        let epoch = dict.next_epoch()?;
        Rc::make_mut(&mut contents.pairs).remove(index);
        contents.hash_order.remove(&bytes);
        dict.epoch.set(epoch);
        *self.value.0.string.borrow_mut() = None;
        self.value
            .0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *self.value.0.double_format.borrow_mut() = None;
        *self.value.0.source_location.borrow_mut() = None;
        Ok(true)
    }
    pub(crate) fn into_value(self) -> Value {
        self.value
    }
}

/// Physical append operations; command adapters supply the independently issued recipe.
pub(crate) struct VmAppendObjects;

impl tcl_cmd_core::native_append::NativeAppendObjects for VmAppendObjects {
    type Value = Value;

    fn snapshot(
        &self,
        value: &Value,
    ) -> Result<tcl_syntax::native_object::NativeObjectSnapshot, tcl_syntax::value::ValueError>
    {
        Ok(value.native_object_snapshot())
    }

    fn prepare_receiver(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Value {
        // Observe original table/argv ownership before the working handle exists.
        if value.native_object_is_shared() {
            value.duplicate_native_object_in(protocol)
        } else {
            value.clone()
        }
    }

    fn duplicate_into(&self, receiver: &Value, source: &Value) {
        let string = source.0.string.borrow().clone();
        let cache = {
            let original = source.0.intrep.borrow();
            match &*original {
                IntRep::NativeString {
                    protocol,
                    num_chars,
                    ..
                } if !protocol.string_primary_survives_duplicate(*num_chars) => IntRep::Str,
                IntRep::JimRegexp(cache) => IntRep::JimRegexp(cache.duplicate()),
                _ => original.clone(),
            }
        };
        let format = source.0.double_format.borrow().clone();
        *receiver.0.string.borrow_mut() = string;
        receiver.0.string_storage.set(source.0.string_storage.get());
        *receiver.0.intrep.borrow_mut() = cache;
        *receiver.0.double_format.borrow_mut() = format;
        receiver
            .0
            .source_location
            .borrow_mut()
            .clone_from(&source.0.source_location.borrow());
    }

    fn string(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u8]>, tcl_syntax::value::ValueError> {
        value.native_string_bytes(protocol).map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native append string updater",
            )
        })
    }

    fn unicode(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u32]>, tcl_syntax::value::ValueError> {
        value.native_unicode_units(protocol)
    }

    fn set_string(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        resident: Option<(Rc<[u8]>, NativeStringStorageIdentity)>,
        count: Option<usize>,
        unicode: Option<Rc<[u32]>>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        if protocol.is_jim084() && (resident.is_none() || unicode.is_some()) {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim append String backing",
            ));
        }
        *value.0.string.borrow_mut() = resident
            .as_ref()
            .map(|(bytes, _)| RawString::from_bytes(Rc::clone(bytes)));
        value
            .0
            .string_storage
            .set(resident.map_or(NativeStringStorageIdentity::Unknown, |(_, storage)| storage));
        *value.0.intrep.borrow_mut() = if protocol.is_jim084() {
            IntRep::JimString(count)
        } else {
            IntRep::NativeString {
                protocol,
                num_chars: count,
                unicode,
            }
        };
        *value.0.double_format.borrow_mut() = None;
        *value.0.source_location.borrow_mut() = None;
        Ok(())
    }

    fn set_binary(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        bytes: Rc<[u8]>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        if protocol.tcl_version().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native append binary backing",
            ));
        }
        *value.0.string.borrow_mut() = None;
        value
            .0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *value.0.intrep.borrow_mut() = IntRep::ByteArray(Rc::new(ByteArrayRep {
            bytes,
            string_protocol: Cell::new(Some(protocol)),
            conversion: None,
            proper: true,
        }));
        *value.0.double_format.borrow_mut() = None;
        *value.0.source_location.borrow_mut() = None;
        Ok(())
    }

    fn new_string(&self, bytes: Rc<[u8]>) -> Value {
        Value::new_native_string_bytes(bytes)
    }
}

impl tcl_cmd_core::native_cat::NativeCatObjects for VmAppendObjects {
    fn empty_binary(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u8]>, tcl_syntax::value::ValueError> {
        let version = protocol.tcl_version().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "C compiled concat binary getter",
            ),
        )?;
        value
            .as_native_byte_array(
                tcl_registry::native_binary_value::NativeBinaryByteConversion::Narrow(
                    version.string_character_model(),
                ),
                protocol,
            )
            .map_err(|_| {
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native compiled concat empty binary conversion",
                )
            })
    }
    fn is_shared(&self, value: &Value) -> bool {
        value.native_object_is_shared()
    }
    fn set_plain_string(
        &self,
        value: &Value,
        bytes: Rc<[u8]>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        *value.0.string.borrow_mut() = Some(RawString::from_bytes(bytes));
        value
            .0
            .string_storage
            .set(NativeStringStorageIdentity::Allocated);
        *value.0.intrep.borrow_mut() = IntRep::Str;
        *value.0.double_format.borrow_mut() = None;
        *value.0.source_location.borrow_mut() = None;
        Ok(())
    }
}

impl Value {
    /// Whether the same original header still has native storage lifetime.
    #[must_use]
    pub fn native_object_is_live(&self) -> bool {
        !self.0.retired.get()
    }

    pub(crate) fn check_native_header(&self) -> Result<(), tcl_syntax::value::ValueError> {
        if self.native_object_is_live() {
            Ok(())
        } else {
            Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "retired native object header",
            ))
        }
    }

    fn retire_native_header(&self) {
        if self.0.retired.replace(true) {
            return;
        }
        // Detach everything before releasing children or filename owners.
        let primary = self.0.intrep.replace(IntRep::Str);
        let string = self.0.string.replace(None);
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        let format = self.0.double_format.replace(None);
        let location = self.0.source_location.replace(None);
        let context = self.0.jim_context.replace(None);
        native_retirement::release(native_retirement::RetiredNativeHeader {
            primary,
            string,
            format,
            location,
            context,
        });
    }

    /// Ordinary native release, distinct from Subst's deliberate no-free undo.
    pub(crate) fn retire_unowned_native_header(&self) {
        if Rc::strong_count(&self.0) == self.0.lifetime_pins.get() {
            self.retire_native_header();
        }
    }

    pub(crate) fn retain_native_lifetime_pin(&self) {
        self.0.lifetime_pins.set(
            self.0
                .lifetime_pins
                .get()
                .checked_add(1)
                .expect("native lifetime pins exhausted"),
        );
    }

    pub(crate) fn release_native_lifetime_pin(&self) {
        self.0.lifetime_pins.set(
            self.0
                .lifetime_pins
                .get()
                .checked_sub(1)
                .expect("native lifetime pin underflow"),
        );
    }

    /// Retain the actual header while keeping transport ownership out of Tcl sharing.
    #[must_use]
    pub fn native_lifetime_lease(&self) -> NativeObjectLifetimeLease {
        self.retain_native_lifetime_pin();
        NativeObjectLifetimeLease(Some(self.clone()))
    }
    /// Native Subst reverse-reference: preserve its live rc0 returned header.
    /// This consumes the actual owner without running ordinary last-owner release.
    pub(crate) fn into_native_unowned_lifetime(mut self) -> Self {
        if self.1 == NativeValueHandleOwnership::Reference {
            self.retain_native_lifetime_pin();
            self.1 = NativeValueHandleOwnership::LifetimeOnly;
        }
        self
    }

    /// Retain a completion transport at an actual native ownership boundary.
    #[must_use]
    pub fn into_native_reference(mut self) -> Self {
        if self.1 == NativeValueHandleOwnership::LifetimeOnly {
            self.release_native_lifetime_pin();
            self.1 = NativeValueHandleOwnership::Reference;
        }
        self
    }

    /// Reset an unshared C interpreter result on its same original header.
    pub(crate) fn reset_native_c_result(&self) {
        debug_assert!(!self.native_object_is_shared());
        *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::<[u8]>::from([])));
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::CanonicalEmpty);
        self.replace_primary(IntRep::Str);
    }

    /// Opaque identity of this retained original object, for equality and lifetime tracking.
    /// It supplies neither a native pointer ABI nor execution authority.
    #[must_use]
    pub fn native_object_identity(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }

    /// Inspect original sharing before acquiring bridge or working-handle references.
    /// Callers must capture this fact before retaining their capability clone.
    #[must_use]
    pub fn native_object_is_shared(&self) -> bool {
        Rc::strong_count(&self.0).saturating_sub(self.0.lifetime_pins.get()) > 1
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn native_object_reference_count(&self) -> usize {
        Rc::strong_count(&self.0).saturating_sub(self.0.lifetime_pins.get())
    }

    /// Apply a checked bridge mirror to this same original physical object.
    /// Getter failure does not suppress reached cache/string effects; aliases
    /// retain this object identity. This operation deliberately performs no COW.
    ///
    /// # Errors
    /// Refuses unknown engines, foreign cache origins, or unsupported opaque caches.
    pub fn adopt_native_object_representation(
        &self,
        donor: &Value,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.adopt_native_object_representation_with_string_mutation(
            donor,
            dialect,
            tcl_core_types::ResidentStringMutation::Replace,
        )
    }

    /// Apply a checked mirror using its authenticated resident-allocation receipt.
    /// Preserve retains this object's allocation even when its primary cache changes.
    /// Replace adopts the donor allocation, including equal-content replacements.
    ///
    /// # Errors
    /// Refuses inconsistent receipts or unsupported native cache origins before mutation.
    pub fn adopt_native_object_representation_with_string_mutation(
        &self,
        donor: &Value,
        dialect: tcl_registry::InvocationDialect,
        string_mutation: tcl_core_types::ResidentStringMutation,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        donor.check_native_header()?;
        let protocol =
            dialect
                .native_string_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native object mirror issuer",
                ))?;
        donor.validate_native_mirror_primary(protocol)?;
        let string = match string_mutation {
            tcl_core_types::ResidentStringMutation::Preserve => {
                if self.resident_string_bytes() != donor.resident_string_bytes()
                    || self.0.string_storage.get() != donor.0.string_storage.get()
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native object mirror preserved string receipt",
                    ));
                }
                self.0.string.borrow().clone()
            }
            tcl_core_types::ResidentStringMutation::Replace => donor.0.string.borrow().clone(),
            tcl_core_types::ResidentStringMutation::Discard => {
                if donor.0.string.borrow().is_some() {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native object mirror discarded string receipt",
                    ));
                }
                None
            }
        };
        let cache = donor.0.intrep.borrow().clone();
        let format = donor.0.double_format.borrow().clone();
        *self.0.string.borrow_mut() = string;
        self.0.string_storage.set(donor.0.string_storage.get());
        *self.0.intrep.borrow_mut() = cache;
        *self.0.double_format.borrow_mut() = format;
        Ok(())
    }

    fn validate_native_mirror_primary(
        &self,
        protocol: NativeStringProtocol,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        {
            let cache = self.0.intrep.borrow();
            match &*cache {
                IntRep::NativeString {
                    protocol: origin, ..
                } if *origin != protocol => {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native object mirror String origin",
                    ));
                }
                IntRep::JimString(_) if !protocol.is_jim084() => {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native object mirror Jim String origin",
                    ));
                }
                IntRep::WordBoolean { version, .. }
                    if protocol != NativeStringProtocol::C(*version) =>
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native object mirror Boolean origin",
                    ));
                }
                IntRep::NativeIndex { version, .. }
                    if protocol != NativeStringProtocol::C(*version) =>
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native object mirror Index origin",
                    ));
                }
                IntRep::NativeCommandName(cache)
                    if protocol != NativeStringProtocol::C(cache.version) =>
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native object mirror command-name origin",
                    ));
                }
                IntRep::NativeNamespaceName(cache)
                    if protocol != NativeStringProtocol::C(cache.version())
                        || (!tcl_syntax::native_namespace_name::NativeNamespaceNameRecipe::for_tcl_version(cache.version()).has_string_updater()
                            && self.resident_string_bytes().is_none()) =>
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native namespace-name mirror origin or storage",
                    ));
                }
                IntRep::NativeCommandNameUnresolved(version)
                    if protocol != NativeStringProtocol::C(*version) =>
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native unresolved command-name mirror origin",
                    ));
                }
                IntRep::NativeCommandName(_) | IntRep::NativeCommandNameUnresolved(_)
                    if self.resident_string_bytes().is_none() =>
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native command-name mirror resident storage",
                    ));
                }
                IntRep::JimCommand(_) | IntRep::JimVariable(_)
            | IntRep::JimOption(_)
                | IntRep::NativeParsedVariableName(_)
                | IntRep::NativeLocalVariableName(_)
                | IntRep::JimSource(_) | IntRep::JimScript(_)
            | IntRep::JimDictionarySubstitution { .. }
                | IntRep::JimInterpolated(_)
                | IntRep::JimScriptLine { .. }
                | IntRep::Expression(_)
            | IntRep::NativeBytecode(_)
                | IntRep::CompletionCode(_)
                | IntRep::NativeArraySearch { .. }
                | IntRep::FrameLevel { .. }
            | IntRep::NativePropertyName(_)
            | IntRep::NativeInstructionName(_)
            | IntRep::NativeMethodName(_)
            | IntRep::NativeRegexp(_)
            | IntRep::JimRegexp(_)
            | IntRep::JimIndex(_) => {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native object mirror opaque primary cache",
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Duplicate under the selected native object recipe without string generation.
    #[must_use]
    pub fn duplicate_native_object_in(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Self {
        if protocol.object_header_duplicate_action(
            self.resident_string_bytes()
                .as_ref()
                .map(|bytes| bytes.len()),
        ) == tcl_syntax::native_string::NativeObjectHeaderDuplicateAction::CanonicalEmptyString
        {
            let duplicate = Self::new_native_string_bytes(b"".as_slice());
            duplicate
                .0
                .jim_context
                .borrow_mut()
                .clone_from(&self.0.jim_context.borrow());
            return duplicate;
        }
        if protocol.is_jim084()
            && matches!(
                &*self.0.intrep.borrow(),
                IntRep::Expression(_) | IntRep::JimScript(_)
            )
        {
            let resident = self
                .resident_string_bytes()
                .expect("native parsed source retains resident bytes");
            let duplicate = Self::new_native_string_bytes(resident.as_ref());
            duplicate
                .0
                .jim_context
                .borrow_mut()
                .clone_from(&self.0.jim_context.borrow());
            return duplicate;
        }
        if matches!(&*self.0.intrep.borrow(), IntRep::NativeBytecode(_)) {
            let duplicate = Self::new_native_string_bytes(
                self.resident_string_bytes()
                    .expect("native bytecode requires resident source")
                    .as_ref(),
            );
            return duplicate;
        }
        let duplicate = self.duplicate_native_object();
        if let IntRep::NativeString {
            protocol,
            num_chars,
            ..
        } = &*self.0.intrep.borrow()
            && !protocol.string_primary_survives_duplicate(*num_chars)
        {
            *duplicate.0.intrep.borrow_mut() = IntRep::Str;
            return duplicate;
        }
        self.copy_native_variable_primary_to(&duplicate);
        let cache = self.0.intrep.borrow();
        match &*cache {
            IntRep::Dict(dict) => {
                let pairs = dict.with_pairs(<[(Value, Value)]>::to_vec);
                let mut hash_order = if protocol.is_jim084() {
                    dict.contents.borrow().hash_order.clone()
                } else {
                    TclStringHashOrder::default()
                };
                if !protocol.is_jim084() {
                    for (key, _) in &pairs {
                        hash_order.insert(&key.string_bytes());
                    }
                }
                *duplicate.0.intrep.borrow_mut() = IntRep::Dict(Rc::new(DictRep::new(
                    pairs,
                    hash_order,
                    dict.string_protocol.get(),
                )));
            }
            IntRep::ByteArray(backing) => {
                *duplicate.0.intrep.borrow_mut() = IntRep::ByteArray(Rc::new((**backing).clone()));
            }
            IntRep::List {
                items,
                string_protocol,
                ..
            } if protocol.is_jim084() => {
                *duplicate.0.intrep.borrow_mut() =
                    IntRep::native_list(items.as_ref().clone(), false, string_protocol.get());
            }
            _ => {}
        }
        duplicate
    }

    /// Duplicate physical representations without reaching their string updater.
    pub(crate) fn duplicate_native_object(&self) -> Self {
        Self(
            Rc::new(Obj {
                string: RefCell::new(self.0.string.borrow().clone()),
                string_storage: Cell::new(self.0.string_storage.get()),
                intrep: RefCell::new(match &*self.0.intrep.borrow() {
                    IntRep::JimRegexp(cache) => IntRep::JimRegexp(cache.duplicate()),
                    primary => primary.clone(),
                }),
                double_format: RefCell::new(self.0.double_format.borrow().clone()),
                source_location: RefCell::new(self.0.source_location.borrow().clone()),
                jim_context: RefCell::new(self.0.jim_context.borrow().clone()),
                lifetime_pins: Cell::new(0),
                retired: Cell::new(false),
            }),
            NativeValueHandleOwnership::Reference,
        )
    }

    /// An actual binary object with a byte payload and no generated string.
    #[must_use]
    pub fn byte_array(bytes: impl Into<Rc<[u8]>>) -> Self {
        Self::byte_array_with_resident_string(bytes, None)
    }

    /// Retain binary storage together with an existing native string cache.
    /// Inspecting or transferring this value does not generate a missing cache.
    #[must_use]
    pub fn byte_array_with_resident_string(
        bytes: impl Into<Rc<[u8]>>,
        resident: Option<Rc<[u8]>>,
    ) -> Self {
        Self::from_raw_parts(
            resident.map(RawString::from_bytes),
            IntRep::ByteArray(Rc::new(ByteArrayRep {
                bytes: bytes.into(),
                string_protocol: Cell::new(None),
                conversion: None,
                proper: true,
            })),
        )
    }

    /// Inspect the binary payload without converting or generating a string.
    #[must_use]
    pub fn byte_array_representation(&self) -> Option<Rc<[u8]>> {
        match &*self.0.intrep.borrow() {
            IntRep::ByteArray(bytes) => Some(Rc::clone(&bytes.bytes)),
            _ => None,
        }
    }

    /// Whether the native pure-byte-array decoder door may use this payload.
    #[must_use]
    pub fn is_pure_byte_array(&self) -> bool {
        self.0.string.borrow().is_none() && self.byte_array_representation().is_some()
    }

    /// Selected native string-to-byte conversion, preserving original string
    /// bytes before replacing the internal representation. Failure retains it.
    pub fn as_byte_array(
        &self,
        conversion: tcl_registry::native_binary_value::NativeBinaryByteConversion,
    ) -> Result<Rc<[u8]>, ByteArrayAccessError> {
        use tcl_registry::native_binary_value::NativeBinaryByteConversion;
        if conversion == NativeBinaryByteConversion::Utf8 {
            // Jim's binary commands read the string bytes; they do not install
            // a C Tcl byte-array representation or withdraw a cached count.
            return Ok(self.string_bytes());
        }
        if let IntRep::ByteArray(rep) = &*self.0.intrep.borrow()
            && (rep.conversion.is_none() || rep.conversion == Some(conversion))
            && (rep.proper || conversion != NativeBinaryByteConversion::CheckedLatin1)
        {
            return Ok(Rc::clone(&rep.bytes));
        }
        let text = self.try_to_str().map_err(ByteArrayAccessError::Unicode)?;
        let bytes: Rc<[u8]> = Rc::from(
            conversion
                .convert(&text)
                .map_err(ByteArrayAccessError::Conversion)?,
        );
        let proper = text.chars().all(|character| u32::from(character) <= 255);
        *self.0.intrep.borrow_mut() = IntRep::ByteArray(Rc::new(ByteArrayRep {
            bytes: Rc::clone(&bytes),
            string_protocol: Cell::new(None),
            conversion: Some(conversion),
            proper,
        }));
        Ok(bytes)
    }

    /// Convert the original object's actual native string units into bytes.
    /// Cache adoption follows successful conversion; a proper-byte rejection
    /// preserves its original cache and resident string.
    ///
    /// # Errors
    /// Returns a guest byte-conversion error or unavailable native storage.
    pub fn as_native_byte_array(
        &self,
        conversion: tcl_registry::native_binary_value::NativeBinaryByteConversion,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u8]>, ByteArrayAccessError> {
        use tcl_registry::native_binary_value::NativeBinaryByteConversion;
        if conversion == NativeBinaryByteConversion::Utf8 {
            return self
                .native_string_bytes(protocol)
                .map_err(ByteArrayAccessError::Unavailable);
        }
        let version = protocol
            .tcl_version()
            .ok_or(ByteArrayAccessError::Unavailable(
                tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable,
            ))?;
        if let IntRep::ByteArray(rep) = &*self.0.intrep.borrow()
            && (rep.conversion.is_none() || rep.conversion == Some(conversion))
            && (rep.proper || conversion != NativeBinaryByteConversion::CheckedLatin1)
        {
            return Ok(Rc::clone(&rep.bytes));
        }
        let original = self
            .native_string_bytes(protocol)
            .map_err(ByteArrayAccessError::Unavailable)?;
        let units = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version);
        let bytes: Rc<[u8]> = Rc::from(
            conversion
                .convert_native(&original, units)
                .map_err(ByteArrayAccessError::Conversion)?,
        );
        let proper = NativeBinaryByteConversion::CheckedLatin1
            .convert_native(&original, units)
            .is_ok();
        *self.0.intrep.borrow_mut() = IntRep::ByteArray(Rc::new(ByteArrayRep {
            bytes: Rc::clone(&bytes),
            string_protocol: Cell::new(Some(protocol)),
            conversion: Some(conversion),
            proper,
        }));
        Ok(bytes)
    }

    /// Select decoder bytes from the actual object/cache protocol; UTF8
    /// fallback retains its source interpretation for Unicode diagnostics.
    pub fn binary_decode_input(
        &self,
        policy: tcl_registry::native_binary_value::NativeBinaryDecodeSource,
    ) -> Result<tcl_registry::native_binary_value::NativeBinaryDecodeInput, UnicodeAccessError>
    {
        use tcl_registry::native_binary_value::{
            NativeBinaryByteConversion, NativeBinaryDecodeInput, NativeBinaryDecodeSource,
        };
        if policy == NativeBinaryDecodeSource::PureByteArrayOrString && self.is_pure_byte_array() {
            return Ok(NativeBinaryDecodeInput::Bytes(
                self.byte_array_representation()
                    .expect("pure byte array")
                    .to_vec(),
            ));
        }
        if policy == NativeBinaryDecodeSource::ProperByteArrayOrString {
            match self.as_byte_array(NativeBinaryByteConversion::CheckedLatin1) {
                Ok(bytes) => return Ok(NativeBinaryDecodeInput::Bytes(bytes.to_vec())),
                Err(ByteArrayAccessError::Unicode(error)) => return Err(error),
                Err(ByteArrayAccessError::Conversion(_)) => {}
                Err(ByteArrayAccessError::Unavailable(_)) => {
                    unreachable!("Unicode-only byte conversion has no native recipe branch")
                }
            }
        }
        Ok(NativeBinaryDecodeInput::String(
            self.try_to_str()?.to_string(),
        ))
    }
    fn replace_primary(&self, replacement: IntRep) {
        let retired = self.0.intrep.replace(replacement);
        drop(retired);
    }

    pub(crate) fn bind_native_jim_context(
        &self,
        context: &Rc<NativeJimObjectContext>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        let selected = Rc::downgrade(context);
        let mut retained = self.0.jim_context.borrow_mut();
        if retained
            .as_ref()
            .is_some_and(|previous| !std::rc::Weak::ptr_eq(previous, &selected))
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim original object interpreter",
            ));
        }
        *retained = Some(selected);
        Ok(())
    }

    pub(crate) fn native_jim_context(
        &self,
    ) -> Result<Rc<NativeJimObjectContext>, tcl_syntax::value::ValueError> {
        self.0
            .jim_context
            .borrow()
            .as_ref()
            .and_then(std::rc::Weak::upgrade)
            .filter(|context| context.is_live())
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim original source context",
            ))
    }

    /// Pin the actual filename once before native source conversion retires its owner.
    pub(crate) fn pin_native_jim_source_info(
        &self,
        context: &Rc<NativeJimObjectContext>,
    ) -> Result<NativeJimSourceInfo, tcl_syntax::value::ValueError> {
        match &*self.0.intrep.borrow() {
            IntRep::JimSource(source) => {
                if !std::rc::Weak::ptr_eq(&source.context, &Rc::downgrade(context)) {
                    return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "Jim original source interpreter",
                    ));
                }
                Ok(source.info.clone())
            }
            IntRep::JimScript(header) => {
                if !std::rc::Weak::ptr_eq(&header.0.context, &Rc::downgrade(context)) {
                    return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "Jim Script original interpreter",
                    ));
                }
                header.0.source_info()
            }
            _ => Ok(NativeJimSourceInfo {
                filename: context.empty_object().clone(),
                line: 1,
            }),
        }
    }

    /// Adopt Source on a real resident object; Source has no string updater.
    pub(crate) fn install_native_jim_source(
        &self,
        info: NativeJimSourceInfo,
        context: &Rc<NativeJimObjectContext>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        if self.resident_string_bytes().is_none()
            || self.is_same_object(&info.filename)
            || self.native_object_is_shared()
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim original Source storage",
            ));
        }
        self.bind_native_jim_context(context)?;
        self.replace_primary(IntRep::JimSource(NativeJimSource {
            info,
            context: Rc::downgrade(context),
        }));
        Ok(())
    }

    pub(crate) fn native_jim_source_cache_present(&self) -> bool {
        matches!(*self.0.intrep.borrow(), IntRep::JimSource(_))
    }

    pub(crate) fn source_location(
        &self,
    ) -> Option<tcl_runtime_api::script_source_location::ScriptSourceLocation> {
        self.0.source_location.borrow().clone()
    }

    pub(crate) fn retain_source_location(
        &self,
        location: tcl_runtime_api::script_source_location::ScriptSourceLocation,
    ) {
        *self.0.source_location.borrow_mut() = Some(location);
    }

    /// Inspect the current representation without requesting a conversion.
    #[must_use]
    pub fn has_list_representation(&self) -> bool {
        matches!(*self.0.intrep.borrow(), IntRep::List { .. })
    }
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
        let bytes = self.native_string_bytes(protocol).map_err(|error| {
            tcl_syntax::value::ValueError::NativeStringAccess(
                tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
            )
        })?;
        let matches = matches!(&*self.0.intrep.borrow(), IntRep::Expression(cache)
            if cache.dialect == dialect && &cache.profile == profile && cache.policy == *policy);
        if !matches
            && preparation
                == tcl_syntax::expr::parser::ExpressionSourceCachePreparation::BeforeParsing
        {
            self.reject_expression(dialect, *profile, policy);
        }
        Ok(bytes)
    }

    pub(crate) fn reject_expression(
        &self,
        dialect: tcl_registry::InvocationDialect,
        profile: tcl_dialect::DialectProfileKey,
        policy: &crate::compiled::NativeCompilerPolicy,
    ) {
        self.replace_primary(IntRep::Expression(Rc::new(ExpressionCache {
            profile,
            dialect,
            policy: *policy,
            node: None,
            jim: None,
        })));
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

    pub(crate) fn cache_expression(
        &self,
        dialect: tcl_registry::InvocationDialect,
        profile: tcl_dialect::DialectProfileKey,
        policy: &crate::compiled::NativeCompilerPolicy,
        node: &tcl_syntax::expr::NativeExprNode,
    ) {
        if self.native_jim_expression_objects().is_some()
            && self.cached_expression(dialect, &profile, policy).is_some()
        {
            return;
        }
        self.replace_primary(IntRep::Expression(Rc::new(ExpressionCache {
            profile,
            dialect,
            policy: *policy,
            node: Some(node.clone()),
            jim: None,
        })));
    }

    /// Inspect bytes already present on the object without triggering conversion.
    pub(crate) fn existing_string_representation(&self) -> Option<Rc<str>> {
        self.0
            .string
            .borrow()
            .as_ref()
            .and_then(|string| string.unicode().ok())
    }

    /// Inspect exact bytes already resident on the object without conversion.
    #[must_use]
    pub fn resident_string_bytes(&self) -> Option<Rc<[u8]>> {
        self.0.string.borrow().as_ref().map(RawString::bytes)
    }

    /// Test the public native invalidation operation without retiring a primary.
    #[cfg(test)]
    pub(crate) fn invalidate_native_string_for_test(&self) {
        drop(self.0.string.borrow_mut().take());
        drop(self.0.source_location.borrow_mut().take());
    }

    /// Independently retained resident storage identity; absence means no string.
    #[must_use]
    pub fn resident_string_storage_identity(&self) -> Option<NativeStringStorageIdentity> {
        self.0
            .string
            .borrow()
            .is_some()
            .then(|| self.0.string_storage.get())
    }

    /// Supply actual resident bytes and their independently authenticated storage.
    /// This constructs a new object, preserving the original typed backing.
    ///
    /// # Errors
    /// Refuses a canonical-empty receipt attached to nonempty bytes.
    pub fn with_resident_string_bytes_and_storage(
        self,
        bytes: Rc<[u8]>,
        storage: NativeStringStorageIdentity,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        if storage == NativeStringStorageIdentity::CanonicalEmpty && !bytes.is_empty() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native string storage identity",
            ));
        }
        let result = self.with_resident_string_bytes(bytes);
        result.0.string_storage.set(storage);
        Ok(result)
    }

    /// Retain a selected recipe on the actual compound backing. Resident
    /// bytes remain authoritative independently of the object's current engine.
    pub(crate) fn seal_compound_string_protocol(
        &self,
        protocol: NativeStringProtocol,
    ) -> Result<(), tcl_syntax::native_string::NativeStringUnavailable> {
        let primary = self.0.intrep.borrow();
        let retained = match &*primary {
            IntRep::List {
                string_protocol, ..
            } => string_protocol,
            IntRep::Dict(dict) => &dict.string_protocol,
            _ => return Ok(()),
        };
        if retained.get().is_some_and(|existing| existing != protocol) {
            return Err(tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable);
        }
        retained.set(Some(protocol));
        Ok(())
    }

    /// Obtain the native string at an independently selected materialisation
    /// boundary. Existing string bytes are authoritative; a pure byte array
    /// retains its first selected recipe and installs exact native bytes on the
    /// original object without replacing its binary internal representation.
    ///
    /// # Errors
    /// Returns a host refusal for an unaudited native storage/recipe pair.
    pub fn native_string_bytes(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u8]>, tcl_syntax::native_string::NativeStringUnavailable> {
        self.native_string_bytes_with_integer_formatter(protocol, None)
    }

    /// Materialize through an explicitly supplied native-build integer updater.
    /// Resident bytes still bypass the updater. The capability propagates to
    /// actual compound children without taking additional child references.
    pub fn native_string_bytes_with_integer_formatter(
        &self,
        protocol: NativeStringProtocol,
        formatter: Option<&dyn tcl_platform::NativeIntegerFormatter>,
    ) -> Result<Rc<[u8]>, tcl_syntax::native_string::NativeStringUnavailable> {
        if !self.native_object_is_live() {
            #[cfg(test)]
            self.report_native_string_unavailable(protocol, "retired-header");
            return Err(tcl_syntax::native_string::NativeStringUnavailable::StringUpdater);
        }
        if matches!(&*self.0.intrep.borrow(), IntRep::FrameLevel { version, .. }
            if protocol != NativeStringProtocol::C(*version))
        {
            return Err(tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable);
        }
        if self
            .native_instruction_name()
            .is_some_and(|name| protocol != NativeStringProtocol::C(name.version()))
        {
            return Err(tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable);
        }
        if self.native_jim_option_cache().is_some() && !protocol.is_jim084() {
            return Err(tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable);
        }
        if self.native_property_name_is_cached()
            && protocol != NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1)
        {
            return Err(tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable);
        }
        if let Some(resident) = self.resident_string_bytes() {
            return Ok(resident);
        }
        if matches!(&*self.0.intrep.borrow(), IntRep::NativeBytecode(_)) {
            return Err(tcl_syntax::native_string::NativeStringUnavailable::StringUpdater);
        }
        if let Some(result) = self.parsed_variable_string_bytes(protocol) {
            let bytes = result?;
            *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
            self.0
                .string_storage
                .set(NativeStringStorageIdentity::Allocated);
            return Ok(bytes);
        }
        if let Some(cache) = self.native_namespace_name_cache() {
            if protocol != NativeStringProtocol::C(cache.version()) {
                return Err(tcl_syntax::native_string::NativeStringUnavailable::StringUpdater);
            }
            let recipe =
                tcl_syntax::native_namespace_name::NativeNamespaceNameRecipe::for_tcl_version(
                    cache.version(),
                );
            let bytes: Rc<[u8]> = Rc::from(
                cache
                    .string_update_bytes(recipe)
                    .ok_or(tcl_syntax::native_string::NativeStringUnavailable::StringUpdater)?,
            );
            *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
            self.0.string_storage.set(if bytes.is_empty() {
                NativeStringStorageIdentity::CanonicalEmpty
            } else {
                NativeStringStorageIdentity::Allocated
            });
            return Ok(bytes);
        }
        if protocol == NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4) {
            let scalar = match self.native_scalar_cache() {
                Some(NativeScalarCache::Tcl84Long(value)) => {
                    Some((tcl_platform::NativeIntegerKind::Long, value))
                }
                Some(NativeScalarCache::Number(Number::Int(value))) => {
                    Some((tcl_platform::NativeIntegerKind::Wide, value))
                }
                _ => None,
            };
            if let Some((kind, value)) = scalar {
                if let Some(formatter) = formatter {
                    if formatter.build().version[..2] != [8, 4] {
                        return Err(
                            tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable,
                        );
                    }
                    let bytes: Rc<[u8]> =
                        Rc::from(formatter.format(kind, value).map_err(|_| {
                            tcl_syntax::native_string::NativeStringUnavailable::StringUpdater
                        })?);
                    *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
                    self.0
                        .string_storage
                        .set(NativeStringStorageIdentity::Allocated);
                    return Ok(bytes);
                }
                if value == i64::MIN {
                    return Err(tcl_syntax::native_string::NativeStringUnavailable::StringUpdater);
                }
            }
        }
        self.seal_compound_string_protocol(protocol)?;
        if matches!(
            *self.0.intrep.borrow(),
            IntRep::List { .. } | IntRep::Dict(_)
        ) {
            return self.materialize_native_compound(protocol, formatter);
        }
        self.materialize_native_cache_string(protocol)
    }

    fn materialize_native_cache_string(
        &self,
        protocol: NativeStringProtocol,
    ) -> Result<Rc<[u8]>, tcl_syntax::native_string::NativeStringUnavailable> {
        if let IntRep::JimIndex(index) = &*self.0.intrep.borrow() {
            if protocol != NativeStringProtocol::Jim084 {
                return Err(
                    tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable,
                );
            }
            let bytes: Rc<[u8]> = Rc::from(index.string_bytes());
            *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
            self.0
                .string_storage
                .set(NativeStringStorageIdentity::Allocated);
            return Ok(bytes);
        }
        if let Some(offset) = self.native_end_offset() {
            if protocol != NativeStringProtocol::C(offset.version()) {
                return Err(
                    tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable,
                );
            }
            let bytes: Rc<[u8]> = Rc::from(
                offset
                    .string_update()
                    .ok_or(tcl_syntax::native_string::NativeStringUnavailable::StringUpdater)?,
            );
            *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
            self.0
                .string_storage
                .set(NativeStringStorageIdentity::Allocated);
            return Ok(bytes);
        }
        let index_cache = self.native_index_cache();
        if let Some((cache, version)) = index_cache {
            if protocol != tcl_syntax::native_string::NativeStringProtocol::C(version) {
                return Err(
                    tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable,
                );
            }
            let bytes = cache
                .word()
                .map_err(|_| tcl_syntax::native_string::NativeStringUnavailable::StringUpdater)?;
            *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
            self.0.string_storage.set(
                if bytes.is_empty() && version >= tcl_dialect::TclVersion::V9_0 {
                    NativeStringStorageIdentity::CanonicalEmpty
                } else {
                    NativeStringStorageIdentity::Allocated
                },
            );
            return Ok(bytes);
        }
        if matches!(&*self.0.intrep.borrow(), IntRep::NativeString {
            protocol: origin, num_chars: Some(0), unicode: None,
        } if *origin == protocol)
        {
            let bytes: Rc<[u8]> = Rc::from([]);
            *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
            self.0
                .string_storage
                .set(NativeStringStorageIdentity::Allocated);
            return Ok(bytes);
        }
        if matches!(
            *self.0.intrep.borrow(),
            IntRep::CompletionCode(_)
                | IntRep::NativeArraySearch { .. }
                | IntRep::FrameLevel { .. }
                | IntRep::NativePropertyName(_)
                | IntRep::NativeMethodName(_)
                | IntRep::NativeRegexp(_)
                | IntRep::JimRegexp(_)
                | IntRep::NativeIndex { .. }
                | IntRep::NativeCommandName(_)
                | IntRep::NativeCommandNameUnresolved(_)
                | IntRep::JimCommand(_)
                | IntRep::JimVariable(_)
                | IntRep::JimOption(_)
                | IntRep::NativeParsedVariableName(_)
                | IntRep::NativeLocalVariableName(_)
                | IntRep::NativeNamespaceName(_)
                | IntRep::NativeString { unicode: None, .. }
                | IntRep::JimSource(_)
                | IntRep::JimScript(_)
                | IntRep::JimDictionarySubstitution { .. }
                | IntRep::JimInterpolated(_)
                | IntRep::JimScriptLine { .. }
                | IntRep::Expression(_)
        ) {
            #[cfg(test)]
            self.report_native_string_unavailable(protocol, "primary-has-no-updater");
            return Err(tcl_syntax::native_string::NativeStringUnavailable::StringUpdater);
        }
        self.materialize_native_bytearray_string(protocol)
    }

    #[cfg(test)]
    fn report_native_string_unavailable(&self, protocol: NativeStringProtocol, reason: &str) {
        // Failure context observes only the original header. No getter, object
        // clone, payload, address or result storage is produced by this report.
        eprintln!(
            "native-string-unavailable reason={reason} protocol={protocol:?} primary={} live={} references={} resident={}",
            self.native_object_type_name(),
            self.native_object_is_live(),
            self.native_object_reference_count(),
            self.0.string.borrow().is_some(),
        );
    }

    fn materialize_native_bytearray_string(
        &self,
        protocol: NativeStringProtocol,
    ) -> Result<Rc<[u8]>, tcl_syntax::native_string::NativeStringUnavailable> {
        let backing = match &*self.0.intrep.borrow() {
            IntRep::ByteArray(backing) => Some(Rc::clone(backing)),
            _ => None,
        };
        if let Some(backing) = backing {
            let selected = backing.string_protocol.get().unwrap_or(protocol);
            let materialized = selected.materialize(
                tcl_syntax::native_string::NativeStringInput::PureByteArray(&backing.bytes),
            )?;
            let bytes: Rc<[u8]> = Rc::from(materialized.as_ref());
            backing.string_protocol.set(Some(selected));
            *self.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
            self.0.string_storage.set(
                if bytes.is_empty()
                    && selected
                        .tcl_version()
                        .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0)
                {
                    NativeStringStorageIdentity::CanonicalEmpty
                } else {
                    NativeStringStorageIdentity::Allocated
                },
            );
            return Ok(bytes);
        }
        Ok(self.string_bytes())
    }

    fn materialize_native_compound(
        &self,
        protocol: NativeStringProtocol,
        formatter: Option<&dyn tcl_platform::NativeIntegerFormatter>,
    ) -> Result<Rc<[u8]>, tcl_syntax::native_string::NativeStringUnavailable> {
        enum Backing {
            List(crate::NativeListItems),
            Dictionary(Rc<Vec<(Value, Value)>>),
        }
        impl Backing {
            fn len(&self) -> usize {
                match self {
                    Self::List(items) => items.len(),
                    Self::Dictionary(pairs) => pairs.len() * 2,
                }
            }
            fn member(&self, index: usize) -> &Value {
                match self {
                    Self::List(items) => &items[index],
                    Self::Dictionary(pairs) => {
                        let (key, value) = &pairs[index / 2];
                        if index.is_multiple_of(2) { key } else { value }
                    }
                }
            }
        }
        #[derive(Clone, Copy)]
        enum Node {
            Root,
            Member { backing: usize, index: usize },
        }
        impl Node {
            fn value<'a>(self, root: &'a Value, backings: &'a [Backing]) -> &'a Value {
                match self {
                    Self::Root => root,
                    Self::Member { backing, index } => backings[backing].member(index),
                }
            }
        }
        // Typed backing leases retain original members without acquiring child
        // object references. Indices remain valid even if an object shimmers.
        let mut backings = Vec::<Backing>::new();
        let mut pending = vec![(Node::Root, false)];
        while let Some((node, expanded)) = pending.pop() {
            let current = node.value(self, &backings);
            if current.resident_string_bytes().is_some() {
                continue;
            }
            current.seal_compound_string_protocol(protocol)?;
            let backing = match &*current.0.intrep.borrow() {
                IntRep::List { items, .. } => Some(Backing::List(items.lifetime_view())),
                IntRep::Dict(dict) => Some(Backing::Dictionary(dict.pairs_backing())),
                _ => None,
            };
            let Some(backing) = backing else {
                drop(current.native_string_bytes_with_integer_formatter(protocol, formatter)?);
                continue;
            };
            if !expanded {
                let count = backing.len();
                let slot = backings.len();
                backings.push(backing);
                pending.push((node, true));
                pending.extend((0..count).rev().map(|index| {
                    (
                        Node::Member {
                            backing: slot,
                            index,
                        },
                        false,
                    )
                }));
                continue;
            }
            let materialized = (0..backing.len())
                .map(|index| {
                    backing
                        .member(index)
                        .resident_string_bytes()
                        .expect("reached child updater")
                })
                .collect::<Vec<_>>();
            let bytes: Rc<[u8]> = Rc::from(
                tcl_syntax::list_result::NativeListResultSerialization::for_string_protocol(
                    protocol,
                )
                .render(&materialized),
            );
            *current.0.string.borrow_mut() = Some(RawString::from_bytes(Rc::clone(&bytes)));
            if let IntRep::List {
                items, canonical, ..
            } = &*current.0.intrep.borrow()
            {
                let shared = items.native_is_shared();
                canonical.set(protocol.updated_list_canonical(canonical.get(), shared));
            }
            current.0.string_storage.set(if bytes.is_empty() {
                protocol.compound_updater_storage()
            } else {
                NativeStringStorageIdentity::Allocated
            });
        }
        Ok(self
            .resident_string_bytes()
            .expect("completed native compound updater"))
    }

    /// Construct the same typed value with an explicitly supplied resident
    /// string. No parsing, string generation or payload conversion occurs.
    /// Other handles to the original object retain their representations.
    #[must_use]
    pub fn with_resident_string_bytes(self, bytes: Rc<[u8]>) -> Self {
        Self(
            Rc::new(Obj {
                string: RefCell::new(Some(RawString::from_bytes(bytes))),
                string_storage: Cell::new(NativeStringStorageIdentity::Unknown),
                intrep: RefCell::new(match &*self.0.intrep.borrow() {
                    IntRep::JimRegexp(cache) => IntRep::JimRegexp(cache.duplicate()),
                    primary => primary.clone(),
                }),
                double_format: RefCell::new(self.0.double_format.borrow().clone()),
                source_location: RefCell::new(self.0.source_location.borrow().clone()),
                jim_context: RefCell::new(self.0.jim_context.borrow().clone()),
                lifetime_pins: Cell::new(0),
                retired: Cell::new(false),
            }),
            NativeValueHandleOwnership::Reference,
        )
    }

    /// Inspect the primary physical scalar cache without generating its string.
    /// This describes storage and grants no native execution permission.
    #[must_use]
    pub fn native_scalar_cache(&self) -> Option<NativeScalarCache> {
        match &*self.0.intrep.borrow() {
            IntRep::Int(value) => Some(NativeScalarCache::Number(Number::Int(*value))),
            IntRep::Tcl84Long(value) => Some(NativeScalarCache::Tcl84Long(*value)),
            IntRep::Bool(value) => Some(NativeScalarCache::Number(Number::Int(i64::from(*value)))),
            IntRep::Big {
                negative,
                radix,
                digits,
            } => Some(NativeScalarCache::Number(Number::Big {
                negative: *negative,
                radix: *radix,
                digits: digits.clone(),
            })),
            IntRep::Double(value) => Some(NativeScalarCache::Number(Number::Double(*value))),
            IntRep::CoercedDouble(value) => Some(NativeScalarCache::JimCoercedInteger(*value)),
            IntRep::WordBoolean { value, .. } => Some(NativeScalarCache::WordBoolean(*value)),
            _ => None,
        }
    }

    /// Tcl's constant compiler transfers its numeric primary only into an
    /// untyped registered header; an earlier typed literal remains untouched.
    pub(crate) fn adopt_native_expression_number_if_untyped(
        &self,
        number: Number,
        version: tcl_dialect::TclVersion,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        if version < tcl_dialect::TclVersion::V8_5 || self.resident_string_bytes().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "native constant cache transfer",
            ));
        }
        if matches!(*self.0.intrep.borrow(), IntRep::Str) {
            self.adopt_native_scalar_cache(
                NativeScalarCache::Number(number),
                tcl_registry::InvocationDialect::for_version(version)
                    .native_scalar_getter_protocol()
                    .ok_or(ValueError::ScalarNumericInputUnavailable)?,
                tcl_registry::InvocationDialect::for_version(version),
            )?;
        }
        Ok(())
    }

    /// Retain the unknown-count String primary of an authenticated native
    /// append or format producer on the same live original header.
    pub(crate) fn retain_native_string_representation(
        &self,
        materialization: tcl_registry::native_string_materialization::NativeStringMaterialization,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        if self.resident_string_bytes().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native String producer resident storage",
            ));
        }
        let protocol = materialization.protocol();
        *self.0.intrep.borrow_mut() = if protocol.is_jim084() {
            IntRep::JimString(None)
        } else {
            IntRep::NativeString {
                protocol,
                num_chars: None,
                unicode: None,
            }
        };
        Ok(())
    }

    /// Reconstruct an actual String primary cache without running a getter.
    /// Engine origin, resident allocation and retained units are independent.
    ///
    /// # Errors
    /// Refuses foreign engines, inconsistent unit/count storage, and missing updaters.
    pub fn from_native_string_cache(
        cache: tcl_syntax::native_object::NativeObjectCacheSnapshot,
        dialect: tcl_registry::InvocationDialect,
        resident: Option<(Rc<[u8]>, NativeStringStorageIdentity)>,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
        let selected =
            dialect
                .native_string_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native String carrier issuer",
                ))?;
        let primary = match cache {
            Cache::String {
                protocol,
                num_chars,
                unicode,
            } if selected == protocol && protocol.tcl_version().is_some() => {
                if unicode.as_ref().is_some_and(|units| {
                    Some(units.len()) != num_chars
                        || tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(
                            protocol.tcl_version().unwrap(),
                        )
                        .encode_units(units)
                        .is_none()
                }) || (resident.is_none() && unicode.is_none() && num_chars != Some(0))
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native String carrier storage",
                    ));
                }
                IntRep::NativeString {
                    protocol,
                    num_chars,
                    unicode,
                }
            }
            Cache::JimString { num_chars } if selected.is_jim084() && resident.is_some() => {
                IntRep::JimString(num_chars)
            }
            _ => {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native String carrier origin",
                ));
            }
        };
        if resident.as_ref().is_some_and(|(bytes, marker)| {
            *marker == NativeStringStorageIdentity::CanonicalEmpty && !bytes.is_empty()
        }) {
            return Err(ValueError::CommandProtocolUnavailable(
                "native String resident storage",
            ));
        }
        let storage = resident
            .as_ref()
            .map_or(NativeStringStorageIdentity::Unknown, |(_, marker)| *marker);
        let value = Self::from_raw_parts(
            resident.map(|(bytes, _)| RawString::from_bytes(bytes)),
            primary,
        );
        value.0.string_storage.set(storage);
        Ok(value)
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
        if !dialect
            .native_command_name_protocol()
            .is_some_and(|protocol| protocol.accepts_cache_origin(cache.version))
            || self.resident_string_bytes().is_none()
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native command-name cache origin or resident string",
            ));
        }
        *self.0.intrep.borrow_mut() = IntRep::NativeCommandName(cache);
        Ok(())
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
        let protocol = dialect.native_command_name_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native unresolved command-name cache",
            ),
        )?;
        if !protocol.installs_unresolved_on_miss() || self.resident_string_bytes().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native unresolved command-name cache origin",
            ));
        }
        *self.0.intrep.borrow_mut() = IntRep::NativeCommandNameUnresolved(protocol.version());
        Ok(())
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

    /// Original physical descriptor name for native conformance observers.
    #[cfg(test)]
    pub(crate) fn native_object_type_name(&self) -> &'static str {
        match &*self.0.intrep.borrow() {
            IntRep::Str => "none",
            IntRep::NativeString { .. } | IntRep::JimString(_) => "string",
            IntRep::List { .. } => "list",
            IntRep::Dict(_) => "dict",
            IntRep::ByteArray(_) => "bytearray",
            IntRep::NativeBytecode(cache) => {
                if matches!(cache.context, NativeBytecodeContext::Substitution(_)) {
                    "substcode"
                } else {
                    "bytecode"
                }
            }
            IntRep::NativeRegexp(_) | IntRep::JimRegexp(_) => "regexp",
            IntRep::JimIndex(_) => "index",
            IntRep::NativeEndOffset(_) => "end-offset",
            IntRep::NativePropertyName(_) => "tcl::oo property name",
            IntRep::NativeInstructionName(_) => "instname",
            IntRep::FrameLevel { .. } => "levelReference",
            IntRep::NativeMethodName(_) => "TclOO method name",
            IntRep::JimSource(_) => "source",
            IntRep::JimScript(_) => "script",
            IntRep::JimDictionarySubstitution { .. } => "dict-substitution",
            IntRep::JimInterpolated(_) => "interpolated",
            IntRep::JimVariable(_) => "variable",
            IntRep::JimOption(tcl_core_types::NativeJimOptionCache::Enum { .. }) => "get-enum",
            IntRep::JimOption(tcl_core_types::NativeJimOptionCache::ComparedString { .. }) => {
                "compared-string"
            }
            IntRep::JimCommand(_) => "command",
            IntRep::Expression(cache) => {
                if cache.dialect.native_string_protocol() == Some(NativeStringProtocol::Jim084) {
                    "expression"
                } else {
                    "expr"
                }
            }
            IntRep::NativeCommandName(_) | IntRep::NativeCommandNameUnresolved(_) => "cmdName",
            IntRep::NativeParsedVariableName(_) => "parsedVarName",
            IntRep::NativeLocalVariableName(_) => "localVarName",
            IntRep::Tcl84Long(_) | IntRep::Int(_) => "int",
            IntRep::Big { .. } => "bignum",
            IntRep::Double(_) | IntRep::CoercedDouble(_) => "double",
            _ => "opaque",
        }
    }

    /// Inspect the live exact integer primary without a getter or display projection.
    /// This query describes storage; the calling purpose selects its actual issuer.
    pub(crate) fn native_integer_primary(
        &self,
    ) -> Result<Option<i64>, tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        Ok(match &*self.0.intrep.borrow() {
            IntRep::Int(value) => Some(*value),
            _ => None,
        })
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
    pub(crate) fn install_native_jim_index(&self, index: tcl_syntax::native_jim_index::JimIndex) {
        self.replace_primary(IntRep::JimIndex(index));
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

    /// Snapshot original primary cache and storage without materialization.
    /// Object identity and selected operation authority remain separate inputs.
    #[must_use]
    pub fn native_object_snapshot(&self) -> tcl_syntax::native_object::NativeObjectSnapshot {
        use tcl_syntax::native_object::{NativeObjectCacheSnapshot as Cache, NativeObjectSnapshot};
        let resident = self.resident_string_bytes();
        let cache = match &*self.0.intrep.borrow() {
            IntRep::Str => Cache::None,
            IntRep::NativeString {
                protocol,
                num_chars,
                unicode,
            } => Cache::String {
                protocol: *protocol,
                num_chars: *num_chars,
                unicode: unicode.clone(),
            },
            IntRep::JimString(count) => Cache::JimString { num_chars: *count },
            IntRep::ByteArray(bytes) => Cache::ByteArray {
                bytes: Rc::clone(&bytes.bytes),
                proper: bytes.proper,
            },
            IntRep::WordBoolean { value, version } => Cache::WordBoolean {
                value: *value,
                version: *version,
            },
            IntRep::List {
                items, canonical, ..
            } => Cache::List {
                length: items.len(),
                canonical: canonical.get(),
            },
            IntRep::Dict(dict) => Cache::Dictionary {
                size: dict.with_pairs(<[(Value, Value)]>::len),
                pure: resident.is_none(),
            },
            IntRep::Int(_)
            | IntRep::Tcl84Long(_)
            | IntRep::Big { .. }
            | IntRep::Double(_)
            | IntRep::CoercedDouble(_)
            | IntRep::Bool(_) => Cache::Numeric(
                self.native_scalar_cache()
                    .expect("matched numeric primary representation"),
            ),
            _ => self.native_named_primary_snapshot(),
        };
        NativeObjectSnapshot {
            storage: self.resident_string_storage_identity(),
            resident,
            cache,
        }
    }

    fn native_named_primary_snapshot(
        &self,
    ) -> tcl_syntax::native_object::NativeObjectCacheSnapshot {
        use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
        match &*self.0.intrep.borrow() {
            IntRep::NativeCommandName(cache) => Cache::CommandName {
                version: cache.version,
                resolved: true,
            },
            IntRep::NativeCommandNameUnresolved(version) => Cache::CommandName {
                version: *version,
                resolved: false,
            },
            IntRep::JimCommand(cache) => Cache::JimCommand {
                procedure_epoch: cache.epoch,
            },
            IntRep::JimVariable(cache) => Cache::JimVariable {
                frame: cache.frame,
                global: cache.global,
            },
            IntRep::JimOption(tcl_core_types::NativeJimOptionCache::Enum { entry, flags }) => {
                Cache::JimEnum {
                    flags: *flags,
                    index: entry.index(),
                }
            }
            IntRep::JimOption(tcl_core_types::NativeJimOptionCache::ComparedString { .. }) => {
                Cache::JimComparedString
            }
            IntRep::NativeParsedVariableName(cache) => Cache::ParsedVariableName {
                version: cache.protocol.version(),
                array: cache.array.is_some(),
            },
            IntRep::NativeLocalVariableName(cache) => Cache::LocalVariableName {
                version: cache.protocol.version(),
                index: cache.index,
            },
            IntRep::NativeNamespaceName(cache) => Cache::NamespaceName {
                version: cache.version(),
                resolved: cache.namespace().is_some(),
            },
            IntRep::NativeBytecode(cache) => {
                if matches!(cache.context, NativeBytecodeContext::Substitution(_)) {
                    Cache::Other
                } else {
                    Cache::Bytecode {
                        version: cache.version,
                    }
                }
            }
            IntRep::JimScript(header) => Cache::JimScript {
                flags: header.0.storage.flags(),
                tokens: header.0.storage.len(),
            },
            IntRep::JimDictionarySubstitution { .. } => Cache::JimDictionarySubstitution,
            IntRep::JimInterpolated(_) => Cache::JimInterpolated,
            IntRep::NativeInstructionName(name) => Cache::InstructionName {
                version: name.version(),
                opcode: name.opcode(),
            },
            IntRep::FrameLevel { cache, version } => {
                let (relative, level) = match cache {
                    tcl_registry::NativeFrameLevelCache::Relative(level) => (true, *level),
                    tcl_registry::NativeFrameLevelCache::Absolute(level) => (false, *level),
                };
                Cache::FrameReference {
                    version: *version,
                    relative,
                    level,
                }
            }
            IntRep::NativeIndex { cache, version } => Cache::Index {
                version: *version,
                index: cache.index(),
                stride: cache.stride(),
            },
            _ => Cache::Other,
        }
    }

    /// Actual C release retained with a native word-Boolean descriptor.
    #[must_use]
    pub fn native_word_boolean_version(&self) -> Option<tcl_dialect::TclVersion> {
        match *self.0.intrep.borrow() {
            IntRep::WordBoolean { version, .. } => Some(version),
            _ => None,
        }
    }

    /// Current full numeric cache, excluding native word-Boolean descriptors.
    pub(crate) fn number_representation(&self) -> Option<Number> {
        match self.native_scalar_cache()? {
            NativeScalarCache::Number(number) => Some(number),
            NativeScalarCache::Tcl84Long(value) => Some(Number::Int(value)),
            NativeScalarCache::JimCoercedInteger(integer) => Some(Number::Double(integer as f64)),
            NativeScalarCache::WordBoolean(_) => None,
        }
    }

    /// Reconstruct a transported physical cache without invoking a getter.
    /// Cache origin and independently selected actual engine must agree.
    ///
    /// # Errors
    /// Refuses unavailable engines, foreign Boolean origins or malformed Big storage.
    pub fn from_native_scalar_cache(
        cache: NativeScalarCache,
        word_boolean_origin: Option<tcl_dialect::TclVersion>,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        Self::from_native_scalar_cache_with_storage(cache, word_boolean_origin, dialect, None)
    }

    /// Reconstruct a full transported cache and its independently resident string.
    /// Modern native word-Boolean descriptors require that resident string.
    ///
    /// # Errors
    /// Refuses missing authority, inconsistent storage or a foreign cache origin.
    pub fn from_native_scalar_cache_with_storage(
        cache: NativeScalarCache,
        word_boolean_origin: Option<tcl_dialect::TclVersion>,
        dialect: tcl_registry::InvocationDialect,
        resident: Option<(Rc<[u8]>, NativeStringStorageIdentity)>,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        let protocol = dialect
            .native_scalar_getter_protocol()
            .ok_or(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)?;
        if matches!(cache, NativeScalarCache::WordBoolean(_))
            && (word_boolean_origin.is_none() || word_boolean_origin != protocol.tcl_version())
        {
            return Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable);
        }
        let mut value = Self::empty();
        if let Some((bytes, storage)) = resident {
            value = value.with_resident_string_bytes_and_storage(bytes, storage)?;
        } else {
            *value.0.string.borrow_mut() = None;
            value
                .0
                .string_storage
                .set(NativeStringStorageIdentity::Unknown);
        }
        value.adopt_native_scalar_cache(cache, protocol, dialect)?;
        Ok(value)
    }

    /// Store a reached legacy increment cache on the original working object.
    ///
    /// # Errors
    /// Refuses caches outside authentic C 8.4 and Jim integer increment storage.
    pub(crate) fn store_native_legacy_increment_cache(
        &self,
        cache: NativeScalarCache,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let protocol = dialect
            .native_scalar_getter_protocol()
            .ok_or(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)?;
        let admitted = matches!(
            (protocol.tcl_version(), protocol.is_jim084(), &cache),
            (
                Some(tcl_dialect::TclVersion::V8_4),
                false,
                NativeScalarCache::Tcl84Long(_) | NativeScalarCache::Number(Number::Int(_))
            ) | (None, true, NativeScalarCache::Number(Number::Int(_)))
        );
        if !admitted {
            return Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable);
        }
        self.adopt_native_scalar_cache(cache, protocol, dialect)?;
        *self.0.string.borrow_mut() = None;
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *self.0.source_location.borrow_mut() = None;
        Ok(())
    }

    fn adopt_native_scalar_cache(
        &self,
        cache: NativeScalarCache,
        protocol: NativeScalarGetterProtocol,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let rep = match cache {
            NativeScalarCache::Tcl84Long(value) => {
                if protocol.tcl_version() != Some(tcl_dialect::TclVersion::V8_4) {
                    return Err(ValueError::ScalarNumericInputUnavailable);
                }
                IntRep::Tcl84Long(value)
            }
            NativeScalarCache::Number(Number::Int(value)) => IntRep::Int(value),
            NativeScalarCache::Number(Number::Double(value)) => IntRep::Double(value),
            NativeScalarCache::Number(Number::Nan { negative, payload }) => {
                IntRep::Double(f64::from_bits(
                    (u64::from(negative) << 63)
                        | 0x7ff8_0000_0000_0000
                        | (payload.unwrap_or(0) & 0x0007_ffff_ffff_ffff),
                ))
            }
            NativeScalarCache::Number(Number::Big {
                negative,
                radix,
                digits,
            }) => {
                if num_bigint::BigInt::parse_bytes(digits.as_bytes(), radix as u32).is_none() {
                    return Err(ValueError::ScalarNumericInputUnavailable);
                }
                IntRep::Big {
                    negative,
                    radix,
                    digits,
                }
            }
            NativeScalarCache::WordBoolean(value) => {
                let version = protocol
                    .tcl_version()
                    .ok_or(ValueError::ScalarNumericInputUnavailable)?;
                if version != tcl_dialect::TclVersion::V8_4 && self.0.string.borrow().is_none() {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native word-Boolean resident string",
                    ));
                }
                IntRep::WordBoolean { value, version }
            }
            NativeScalarCache::JimCoercedInteger(value) => {
                if !protocol.is_jim084() {
                    return Err(ValueError::ScalarNumericInputUnavailable);
                }
                IntRep::CoercedDouble(value)
            }
        };
        *self.0.intrep.borrow_mut() = rep;
        *self.0.double_format.borrow_mut() = DoubleFormatContext::for_dialect(dialect);
        Ok(())
    }

    /// Probe an original shared object without rendering guest failure.
    /// Full cache changes apply on failure. An error-neutral native probe does
    /// not materialise a cached object's string merely to describe its failure.
    ///
    /// # Errors
    /// Returns a typed missing-capability refusal outside the native outcome.
    pub fn native_scalar_probe(
        &self,
        dialect: tcl_registry::InvocationDialect,
        kind: NativeScalarGetterKind,
    ) -> Result<
        Result<NativeScalarGetterValue, tcl_syntax::scalar_getter::NativeScalarGetterFailure>,
        tcl_syntax::value::ValueError,
    > {
        self.native_scalar_probe_with_environment(dialect, kind, None)
    }

    pub(crate) fn native_scalar_probe_with_environment(
        &self,
        dialect: tcl_registry::InvocationDialect,
        kind: NativeScalarGetterKind,
        environment: Option<&dyn tcl_platform::NumericEnvironment>,
    ) -> Result<
        Result<NativeScalarGetterValue, tcl_syntax::scalar_getter::NativeScalarGetterFailure>,
        tcl_syntax::value::ValueError,
    > {
        self.check_native_header()?;
        let protocol = dialect
            .native_scalar_getter_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        if self
            .native_word_boolean_version()
            .is_some_and(|version| Some(version) != protocol.tcl_version())
        {
            return Err(ValueError::ScalarNumericInputUnavailable);
        }
        let string_protocol = dialect
            .native_string_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let current = self.native_scalar_cache();
        let conversion = if let Some(conversion) = current
            .as_ref()
            .and_then(|cache| protocol.cached_conversion(kind, cache))
        {
            conversion
        } else {
            let original = self
                .native_string_bytes(string_protocol)
                .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
            if protocol.is_jim084()
                && matches!(
                    kind,
                    NativeScalarGetterKind::Wide | NativeScalarGetterKind::Double
                )
            {
                self.native_jim_context()
                    .map_err(|_| ValueError::ScalarNumericInputUnavailable)?
                    .fresh_numeric_conversion(protocol, kind, &original)?
            } else if protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4)
                && let Some(environment) = environment
            {
                tcl_cmd_core::native_numeric::fresh_c84_conversion(
                    protocol,
                    kind,
                    &original,
                    environment,
                )?
            } else {
                protocol
                    .fresh_conversion(kind, &original)
                    .ok_or(ValueError::ScalarNumericInputUnavailable)?
            }
        };
        let (materialize, cache, outcome) = conversion.into_parts();
        if materialize {
            self.native_string_bytes(string_protocol)
                .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
        }
        if let Some(cache) = cache {
            self.adopt_native_scalar_cache(cache, protocol, dialect)?;
        }
        Ok(outcome)
    }

    /// Apply the reached C8.4 expression integer conversion to this same
    /// original header, retaining its spelling and existing integer primary.
    pub(crate) fn prepare_native_expression_integer84(
        &self,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        self.prepare_native_expression_integer84_with_environment(dialect, None)
    }

    pub(crate) fn prepare_native_expression_integer84_with_environment(
        &self,
        dialect: tcl_registry::InvocationDialect,
        environment: Option<&dyn tcl_platform::NumericEnvironment>,
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        let protocol = dialect
            .native_scalar_getter_protocol()
            .filter(|protocol| protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4))
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        if self
            .native_word_boolean_version()
            .is_some_and(|version| version != tcl_dialect::TclVersion::V8_4)
        {
            return Err(ValueError::ScalarNumericInputUnavailable);
        }
        let current = self.native_scalar_cache();
        if let Some(NativeScalarCache::WordBoolean(boolean)) = current
            .as_ref()
            .filter(|_| self.resident_string_bytes().is_none())
        {
            self.adopt_native_scalar_cache(
                NativeScalarCache::Tcl84Long(i64::from(*boolean)),
                protocol,
                dialect,
            )?;
            return Ok(true);
        }
        if matches!(
            &current,
            Some(NativeScalarCache::Tcl84Long(_) | NativeScalarCache::Number(Number::Int(_)))
        ) {
            return Ok(true);
        }
        if matches!(
            &current,
            Some(NativeScalarCache::Number(
                Number::Double(_) | Number::Nan { .. }
            ))
        ) && self.resident_string_bytes().is_none()
        {
            return Ok(true);
        }
        let string = dialect
            .native_string_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let original = self
            .native_string_bytes(string)
            .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
        if protocol.expression_integer_spelling84(&original)
            && current
                .as_ref()
                .and_then(|cache| protocol.cached_conversion(NativeScalarGetterKind::Wide, cache))
                .is_none()
            && let Some(environment) = environment
        {
            tcl_cmd_core::native_numeric::fresh_c84_conversion(
                protocol,
                NativeScalarGetterKind::Wide,
                &original,
                environment,
            )?;
        }
        if let Some(conversion) =
            protocol.expression_integer_conversion84(current.as_ref(), &original)
        {
            let (_, cache, outcome) = conversion.into_parts();
            if let Some(cache) = cache {
                self.adopt_native_scalar_cache(cache, protocol, dialect)?;
            }
            return Ok(outcome.is_ok());
        }
        Ok(true)
    }

    /// Withdraw a reached numeric string after C8.4 `TRY_CVT_TO_NUMERIC`.
    /// Shared resident headers copy only their physical numeric cache; absent
    /// resident storage keeps the original header and its real owners.
    pub(crate) fn normalize_native_expression_number84(
        self,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        if dialect
            .native_scalar_getter_protocol()
            .is_none_or(|protocol| protocol.tcl_version() != Some(tcl_dialect::TclVersion::V8_4))
        {
            return Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable);
        }
        let Some(
            cache @ (NativeScalarCache::Tcl84Long(_)
            | NativeScalarCache::Number(
                Number::Int(_) | Number::Double(_) | Number::Nan { .. },
            )),
        ) = self.native_scalar_cache()
        else {
            return Ok(self);
        };
        if self.native_object_is_shared() && self.resident_string_bytes().is_some() {
            return Self::from_native_scalar_cache(cache, None, dialect);
        }
        if !self.native_object_is_shared() {
            *self.0.string.borrow_mut() = None;
            self.0
                .string_storage
                .set(NativeStringStorageIdentity::Unknown);
            *self.0.source_location.borrow_mut() = None;
        }
        Ok(self)
    }

    /// C8.4's expression setter reuses an actually unshared arithmetic
    /// operand, installs a native-long primary and withdraws its string.
    pub(crate) fn store_native_expression_long84(
        &self,
        integer: i64,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        let protocol = dialect
            .native_scalar_getter_protocol()
            .filter(|protocol| protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4))
            .ok_or(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)?;
        if self.native_object_is_shared() {
            return Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable);
        }
        self.adopt_native_scalar_cache(NativeScalarCache::Tcl84Long(integer), protocol, dialect)?;
        *self.0.string.borrow_mut() = None;
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *self.0.source_location.borrow_mut() = None;
        Ok(())
    }

    pub(crate) fn store_native_increment_number(
        &self,
        number: Number,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let protocol = dialect
            .native_scalar_getter_protocol()
            .filter(|protocol| protocol.supports_number_getter())
            .ok_or(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)?;
        self.adopt_native_scalar_cache(NativeScalarCache::Number(number), protocol, dialect)?;
        *self.0.string.borrow_mut() = None;
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *self.0.source_location.borrow_mut() = None;
        Ok(())
    }

    /// Probe C's full Number or copying Bignum getter on this original object.
    /// Numeric magnitude and category remain independent of bounded extraction.
    ///
    /// # Errors
    /// Refuses engines without the selected primitive or unavailable original storage.
    pub fn native_number_probe(
        &self,
        dialect: tcl_registry::InvocationDialect,
        kind: tcl_syntax::scalar_getter::NativeNumberGetterKind,
    ) -> Result<
        Result<Number, tcl_syntax::scalar_getter::NativeScalarGetterFailure>,
        tcl_syntax::value::ValueError,
    > {
        self.check_native_header()?;
        let protocol = dialect
            .native_scalar_getter_protocol()
            .filter(|protocol| protocol.supports_number_getter())
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        if self
            .native_word_boolean_version()
            .is_some_and(|version| Some(version) != protocol.tcl_version())
        {
            return Err(ValueError::ScalarNumericInputUnavailable);
        }
        let current = self.native_scalar_cache();
        let cached = if kind == tcl_syntax::scalar_getter::NativeNumberGetterKind::IncrementNumber {
            protocol.increment_number_preflight(
                current.as_ref(),
                self.resident_string_bytes().map(|bytes| bytes.len()),
                matches!(*self.0.intrep.borrow(), IntRep::Str),
            )
        } else {
            current
                .as_ref()
                .and_then(|cache| protocol.cached_number_conversion(kind, cache))
        };
        let conversion = if let Some(conversion) = cached {
            conversion
        } else {
            let string_protocol = dialect
                .native_string_protocol()
                .ok_or(ValueError::ScalarNumericInputUnavailable)?;
            let original = self
                .native_string_bytes(string_protocol)
                .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
            protocol
                .fresh_number_conversion(kind, &original)
                .ok_or(ValueError::ScalarNumericInputUnavailable)?
        };
        let (cache, outcome) = conversion.into_parts();
        if let Some(cache) = cache {
            self.adopt_native_scalar_cache(cache, protocol, dialect)?;
        }
        Ok(outcome)
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

    /// Execute an actual primitive getter and render its reached guest failure.
    /// Interpreter publication belongs to the consuming command adapter.
    ///
    /// # Errors
    /// Returns the exact primitive guest receipt or a typed capability refusal.
    pub fn native_scalar_getter(
        &self,
        dialect: tcl_registry::InvocationDialect,
        kind: NativeScalarGetterKind,
    ) -> Result<NativeScalarGetterValue, tcl_syntax::value::ValueError> {
        self.native_scalar_getter_with_environment(dialect, kind, None)
    }
    pub(crate) fn native_scalar_getter_with_environment(
        &self,
        dialect: tcl_registry::InvocationDialect,
        kind: NativeScalarGetterKind,
        environment: Option<&dyn tcl_platform::NumericEnvironment>,
    ) -> Result<NativeScalarGetterValue, tcl_syntax::value::ValueError> {
        match self.native_scalar_probe_with_environment(dialect, kind, environment)? {
            Ok(value) => Ok(value),
            Err(failure) => {
                let protocol = dialect
                    .native_scalar_getter_protocol()
                    .ok_or(ValueError::ScalarNumericInputUnavailable)?;
                let string_protocol = dialect
                    .native_string_protocol()
                    .ok_or(ValueError::ScalarNumericInputUnavailable)?;
                let original = self
                    .native_string_bytes(string_protocol)
                    .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
                let record = protocol
                    .failure_presentation(kind, failure, &original)
                    .ok_or(ValueError::ScalarNumericInputUnavailable)?;
                Err(ValueError::NativeScalarGetter(Box::new(record)))
            }
        }
    }

    /// Inspect a native integer conversion without reparsing retained source bytes.
    pub(crate) fn integer_representation(&self) -> Option<i64> {
        match *self.0.intrep.borrow() {
            IntRep::Int(value) | IntRep::Tcl84Long(value) => Some(value),
            IntRep::Bool(value) => Some(i64::from(value)),
            _ => None,
        }
    }

    pub(crate) fn stock_list_input_class(
        &self,
    ) -> tcl_registry::native_stock_list::NativeStockListInputClass {
        use tcl_registry::native_stock_list::NativeStockListInputClass as Class;
        match &*self.0.intrep.borrow() {
            IntRep::Str | IntRep::NativeString { .. } | IntRep::JimString(_) => Class::String,
            IntRep::List { .. } => Class::List,
            IntRep::Dict(_) => Class::Dictionary,
            IntRep::ByteArray(_) => Class::ByteArray,
            IntRep::Int(_)
            | IntRep::Tcl84Long(_)
            | IntRep::Big { .. }
            | IntRep::Double(_)
            | IntRep::CoercedDouble(_)
            | IntRep::Bool(_) => Class::Numeric,
            IntRep::WordBoolean { .. } => Class::Boolean,
            IntRep::NativeCommandName(_) | IntRep::NativeCommandNameUnresolved(_) => {
                Class::CommandName
            }
            IntRep::NativeArraySearch { .. } => Class::ArraySearch,
            IntRep::NativeNamespaceName(_) => Class::NamespaceName,
            IntRep::JimCommand(_) | IntRep::JimVariable(_) | IntRep::JimOption(_) => {
                Class::JimLookup
            }
            IntRep::NativeParsedVariableName(_) => Class::ParsedVariableName,
            IntRep::NativeLocalVariableName(_) => Class::LocalVariableName,
            IntRep::JimSource(_) => Class::JimSource,
            IntRep::JimScript(_)
            | IntRep::JimDictionarySubstitution { .. }
            | IntRep::JimInterpolated(_)
            | IntRep::JimScriptLine { .. }
            | IntRep::Expression(_)
            | IntRep::NativeBytecode(_)
            | IntRep::CompletionCode(_)
            | IntRep::FrameLevel { .. }
            | IntRep::NativeRegexp(_)
            | IntRep::JimRegexp(_)
            | IntRep::JimIndex(_) => Class::Unknown,
            IntRep::NativePropertyName(_) => Class::PropertyName,
            IntRep::NativeInstructionName(_) => Class::InstructionName,
            IntRep::NativeMethodName(_) => Class::MethodName,
            IntRep::NativeIndex { .. } | IntRep::NativeEndOffset(_) => Class::Index,
        }
    }

    pub(crate) fn cached_list_length(&self) -> Option<usize> {
        match &*self.0.intrep.borrow() {
            IntRep::List { items, .. } => Some(items.len()),
            _ => None,
        }
    }

    fn restore_coerced_integer(&self) -> Option<i64> {
        let integer = match *self.0.intrep.borrow() {
            IntRep::CoercedDouble(value) => Some(value),
            _ => None,
        };
        if let Some(value) = integer {
            *self.0.intrep.borrow_mut() = IntRep::Int(value);
        }
        integer
    }

    /// Inspect a double without materialising its string representation.
    pub(crate) fn double_representation(&self) -> Option<f64> {
        match *self.0.intrep.borrow() {
            IntRep::Double(value) => Some(value),
            IntRep::CoercedDouble(value) => Some(value as f64),
            _ => None,
        }
    }

    /// Retain a successfully converted native operand on the shared object.
    /// Conversion preserves its existing source bytes, including list spelling.
    pub(crate) fn cache_integer_representation(&self, value: i64) {
        let _ = self.string_bytes();
        *self.0.intrep.borrow_mut() = IntRep::Int(value);
    }

    /// Retain a successfully converted double without changing its source bytes.
    pub(crate) fn cache_double_representation(&self, value: f64) {
        let _ = self.string_bytes();
        *self.0.intrep.borrow_mut() = IntRep::Double(value);
    }

    fn from_parts(string: Option<Rc<str>>, intrep: IntRep) -> Self {
        Self::from_raw_parts(string.map(RawString::from_unicode), intrep)
    }

    fn from_raw_parts(string: Option<RawString>, intrep: IntRep) -> Self {
        Self(
            Rc::new(Obj {
                string: RefCell::new(string),
                string_storage: Cell::new(NativeStringStorageIdentity::Unknown),
                intrep: RefCell::new(intrep),
                double_format: RefCell::new(None),
                source_location: RefCell::new(None),
                jim_context: RefCell::new(None),
                lifetime_pins: Cell::new(0),
                retired: Cell::new(false),
            }),
            NativeValueHandleOwnership::Reference,
        )
    }

    /// A pure-string value.
    pub fn string(s: impl Into<Rc<str>>) -> Self {
        let string = s.into();
        let storage = if string.is_empty() {
            NativeStringStorageIdentity::CanonicalEmpty
        } else {
            NativeStringStorageIdentity::Allocated
        };
        let value = Self::from_parts(Some(string), IntRep::Str);
        value.0.string_storage.set(storage);
        value
    }

    /// The empty string.
    #[must_use]
    pub fn empty() -> Self {
        Self::string("")
    }

    /// A wide-integer value.
    #[must_use]
    pub fn int(n: i64) -> Self {
        Self::from_parts(None, IntRep::Int(n))
    }

    /// Set a genuinely unshared compiled-loop counter through `Tcl_SetLongObj`.
    /// The same header remains owned by its anonymous local; conversion caches
    /// and integer getters do not implement this mutation boundary.
    pub(crate) fn set_native_loop_counter(
        &self,
        count: i64,
        version: tcl_dialect::TclVersion,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.set_native_unshared_integer(count, version)
    }

    /// `Tcl_SetInt/LongObj` on an actual unshared header, preserving its identity.
    pub(crate) fn set_native_unshared_integer(
        &self,
        count: i64,
        version: tcl_dialect::TclVersion,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        if self.native_object_is_shared() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "shared native loop counter",
            ));
        }
        *self.0.string.borrow_mut() = None;
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *self.0.intrep.borrow_mut() = if version == tcl_dialect::TclVersion::V8_4 {
            IntRep::Tcl84Long(count)
        } else {
            IntRep::Int(count)
        };
        Ok(())
    }

    /// A double value.
    #[must_use]
    pub fn double(f: f64) -> Self {
        Self::from_parts(None, IntRep::Double(f))
    }

    /// Retain native lazy double conversion before any caller requests its string.
    pub(crate) fn with_native_double_format(
        self,
        dialect: tcl_registry::InvocationDialect,
    ) -> Self {
        if self.0.string.borrow().is_none()
            && matches!(*self.0.intrep.borrow(), IntRep::Double(_))
            && self.0.double_format.borrow().is_none()
        {
            *self.0.double_format.borrow_mut() = DoubleFormatContext::for_dialect(dialect);
        }
        self
    }

    pub(crate) fn native_double(f: f64, dialect: tcl_registry::InvocationDialect) -> Self {
        Self::double(f).with_native_double_format(dialect)
    }

    pub(crate) fn with_authored_double_format(self, context: DoubleFormatContext) -> Self {
        if self.0.string.borrow().is_none() && matches!(*self.0.intrep.borrow(), IntRep::Double(_))
        {
            *self.0.double_format.borrow_mut() = Some(context);
        }
        self
    }

    /// Observe object lifetime without adding an owning native reference.
    #[must_use]
    pub fn downgrade_native_object(&self) -> WeakNativeObject {
        WeakNativeObject(Rc::downgrade(&self.0))
    }

    /// A string value retaining exact native bytes without Unicode conversion.
    /// Consumers requiring Unicode must use [`Self::try_to_str`].
    #[must_use]
    pub fn from_string_bytes(bytes: impl Into<Rc<[u8]>>) -> Self {
        Self::from_raw_parts(Some(RawString::from_bytes(bytes)), IntRep::Str)
    }

    /// Construct a native string object with the constructor's actual storage.
    /// Imported resident bytes instead use [`Self::from_string_bytes`].
    #[must_use]
    pub fn new_native_string_bytes(bytes: impl Into<Rc<[u8]>>) -> Self {
        let bytes = bytes.into();
        let storage = if bytes.is_empty() {
            NativeStringStorageIdentity::CanonicalEmpty
        } else {
            NativeStringStorageIdentity::Allocated
        };
        let value = Self::from_string_bytes(bytes);
        value.0.string_storage.set(storage);
        value
    }

    /// Construct the allocated byte storage of a native string-result producer,
    /// including an allocated empty result. This constructor installs no cache.
    #[must_use]
    pub(crate) fn new_native_allocated_string_bytes(bytes: Rc<[u8]>) -> Self {
        let value = Self::from_string_bytes(bytes);
        value
            .0
            .string_storage
            .set(NativeStringStorageIdentity::Allocated);
        value
    }

    /// Construct a native string from the complete bytes of a selected producer.
    /// Imported storage without a constructor receipt uses [`Self::from_string_bytes`].
    #[must_use]
    pub fn from_native_string_bytes(bytes: impl Into<Rc<[u8]>>) -> Self {
        Self::new_native_string_bytes(bytes)
    }

    /// Construct proper binary backing with its native updater recipe sealed
    /// before allocation. Imported untyped bytes are a separate carrier.
    pub fn from_native_byte_array(
        bytes: Rc<[u8]>,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        let recipe = dialect.byte_array_string_recipe(None).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native byte-array constructor",
            ),
        )?;
        Ok(Self::from_raw_parts(
            None,
            IntRep::ByteArray(Rc::new(ByteArrayRep {
                bytes,
                string_protocol: Cell::new(Some(recipe.protocol())),
                conversion: None,
                proper: true,
            })),
        ))
    }

    /// Construct an actual C native String with retained Unicode-unit backing.
    /// Its string updater preserves each unit, including adjacent surrogates.
    ///
    /// # Errors
    /// Refuses unavailable native authority or units outside the selected build width.
    pub fn from_native_unicode_units(
        unicode: Rc<[u32]>,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        let protocol =
            dialect
                .native_string_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native Unicode constructor",
                ))?;
        let version = protocol
            .tcl_version()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native Unicode constructor",
            ))?;
        tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
            .encode_units(&unicode)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native Unicode unit width",
            ))?;
        Ok(Self::from_raw_parts(
            None,
            IntRep::NativeString {
                protocol,
                num_chars: Some(unicode.len()),
                unicode: protocol
                    .unicode_constructor_has_unicode(unicode.len())
                    .then_some(unicode),
            },
        ))
    }

    pub(crate) fn native_jim_string(bytes: &[u8], character_count: usize) -> Self {
        Self::from_raw_parts(
            Some(RawString::from_bytes(bytes)),
            IntRep::JimString(Some(character_count)),
        )
    }

    /// A boolean value.
    #[must_use]
    pub fn bool(b: bool) -> Self {
        Self::from_parts(None, IntRep::Bool(b))
    }

    /// A list value.
    #[must_use]
    pub fn list(items: Vec<Value>) -> Self {
        Self::from_parts(None, IntRep::native_list(items, false, None))
    }

    /// Apply the selected native List constructor, independently of cached List transport.
    /// C's zero-element constructor returns canonical empty NULL-type storage;
    /// Jim's constructor retains a pure List primary representation.
    #[must_use]
    pub fn native_list_constructor(
        items: Vec<Value>,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Self {
        if items.is_empty() && protocol.tcl_version().is_some() {
            Self::new_native_string_bytes(&b""[..])
        } else {
            let value = Self::list(items);
            value
                .seal_compound_string_protocol(protocol)
                .expect("fresh native List recipe");
            value
        }
    }

    /// Replace the entire private List contents with original member objects.
    /// Receiver sharing is sampled before acquiring a working reference.
    pub(crate) fn native_list_replace_elements(
        original: &Self,
        elements: &[Self],
        protocol: NativeStringProtocol,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        let value = if original.native_object_is_shared() {
            original.duplicate_native_object_in(protocol)
        } else {
            original.clone()
        };
        drop(value.native_object_list_elements(protocol)?);
        let mut primary = value.0.intrep.borrow_mut();
        let IntRep::List {
            items, canonical, ..
        } = &mut *primary
        else {
            unreachable!("selected native List replacement cache");
        };
        let length = items.len();
        if let Some(version) = protocol.tcl_version() {
            items.replace_native(0, length, elements, version)?;
            *canonical = items.canonical_state();
        } else {
            let members = crate::NativeListItems::new(elements.to_vec(), false);
            *canonical = members.canonical_state();
            *items = members;
        }
        drop(primary);
        value.invalidate_native_list_string();
        Ok(value)
    }

    /// Append original member objects under the selected native List recipe.
    /// An empty dictionary-member batch deliberately avoids conversion.
    pub(crate) fn native_list_append_elements(
        original: Option<&Self>,
        elements: &[Self],
        protocol: NativeStringProtocol,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        let value = match original {
            Some(value) if value.native_object_is_shared() => {
                let duplicate = value.duplicate_native_object_in(protocol);
                if elements.is_empty() {
                    return Ok(duplicate);
                }
                drop(duplicate.native_object_list_elements(protocol)?);
                duplicate
            }
            Some(value) => {
                if elements.is_empty() {
                    return Ok(value.clone());
                }
                drop(value.native_object_list_elements(protocol)?);
                value.clone()
            }
            None => return Ok(Self::native_list_constructor(elements.to_vec(), protocol)),
        };
        value.native_list_append_prepared_elements(elements, protocol)?;
        Ok(value)
    }

    /// Mutate the physical header already selected by a native variable/list
    /// instruction, without treating its working reference as another COW test.
    pub(crate) fn native_list_append_prepared_elements(
        &self,
        elements: &[Self],
        protocol: NativeStringProtocol,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        drop(self.native_object_list_elements(protocol)?);
        let mut primary = self.0.intrep.borrow_mut();
        let IntRep::List {
            items, canonical, ..
        } = &mut *primary
        else {
            unreachable!("selected native List append cache");
        };
        if elements.is_empty() {
            let preserve = protocol
                .tcl_version()
                .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0 && canonical.get());
            drop(primary);
            if !preserve {
                self.invalidate_native_list_string();
            }
            return Ok(());
        }
        let flag = if items.native_is_shared() {
            protocol.copied_list_canonical(canonical.get())
        } else {
            canonical.get()
        };
        if let Some(version) = protocol.tcl_version() {
            items.replace_native(items.len(), 0, elements, version)?;
            if version < tcl_dialect::TclVersion::V9_0 {
                items.canonical_state().set(flag);
            }
        } else {
            let mut members = items.as_ref().clone();
            members.extend(elements.iter().cloned());
            *items = crate::NativeListItems::new(members, flag);
        }
        *canonical = items.canonical_state();
        drop(primary);
        self.invalidate_native_list_string();
        Ok(())
    }

    fn invalidate_native_list_string(&self) {
        *self.0.string.borrow_mut() = None;
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *self.0.double_format.borrow_mut() = None;
        *self.0.source_location.borrow_mut() = None;
    }

    /// `LAPPEND_LIST` validates/converts its original receiver before header COW.
    pub(crate) fn native_list_append_list_elements(
        &self,
        elements: &[Self],
        protocol: NativeStringProtocol,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        drop(self.native_object_list_elements(protocol)?);
        let value = if self.native_object_is_shared() {
            self.duplicate_native_object_in(protocol)
        } else {
            self.clone()
        };
        value.native_list_append_prepared_elements(elements, protocol)?;
        Ok(value)
    }

    /// `LIST_CONCAT` duplicates the target before source List conversion.
    pub(crate) fn native_list_concatenate(
        &self,
        source: &Self,
        protocol: NativeStringProtocol,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        let value = if self.native_object_is_shared() {
            self.duplicate_native_object_in(protocol)
        } else {
            self.clone()
        };
        let source = source.native_object_list_elements(protocol)?;
        value.native_list_append_prepared_elements(&source, protocol)?;
        Ok(value)
    }

    /// C9 full-range List opcode used for one expanded lappend operand.
    pub(crate) fn native_full_list_range(
        &self,
        protocol: NativeStringProtocol,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        if protocol
            .tcl_version()
            .is_none_or(|version| version < tcl_dialect::TclVersion::V9_0)
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "C9 full List range",
            ));
        }
        let members = self.native_object_list_elements(protocol)?;
        if members.is_empty() {
            return Ok(
                if self
                    .0
                    .string
                    .borrow()
                    .as_ref()
                    .is_none_or(|bytes| bytes.bytes().is_empty())
                {
                    self.clone()
                } else {
                    Self::new_native_string_bytes(&b""[..])
                },
            );
        }
        let value = if self.native_object_is_shared() {
            self.native_list_copy(protocol)?
        } else {
            self.clone()
        };
        value.invalidate_native_list_string();
        Ok(value)
    }

    /// Inspect original List storage without conversion or string generation.
    #[must_use]
    pub fn cached_list_representation(&self) -> Option<(crate::NativeListItems, bool)> {
        match &*self.0.intrep.borrow() {
            IntRep::List {
                items, canonical, ..
            } => Some((items.lifetime_view(), canonical.get())),
            _ => None,
        }
    }

    /// Reach `TclListObjCopy`: convert the original, then create a fresh header
    /// with absent string storage sharing the authentic whole List backing.
    /// This purpose belongs to C Tcl 8.5 and later, independently of ordinary
    /// header duplication and the lifetime-only inspection view.
    pub(crate) fn native_list_copy(
        &self,
        protocol: NativeStringProtocol,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        if protocol
            .tcl_version()
            .is_none_or(|version| version < tcl_dialect::TclVersion::V8_5)
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "TclListObjCopy",
            ));
        }
        let backing = self.native_object_list_elements(protocol)?;
        Self::from_retained_native_list_backing(&backing, protocol)
    }

    /// Retain original whole storage after independently authenticating its updater.
    ///
    /// # Errors
    /// Refuses a foreign retained recipe or compiler-private invocation storage.
    pub fn native_list_backing_in(
        &self,
        protocol: NativeStringProtocol,
    ) -> Result<Option<crate::NativeListItems>, tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        let Some((backing, _)) = self.cached_list_representation() else {
            return Ok(None);
        };
        if !backing.is_whole_backing() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native whole List backing shape",
            ));
        }
        self.seal_compound_string_protocol(protocol)
            .map_err(|error| {
                tcl_syntax::value::ValueError::from(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                )
            })?;
        Ok(Some(backing))
    }

    /// Check the current original-header attachment without reparsing members.
    #[must_use]
    pub fn native_list_attachment_matches(&self, backing: &crate::NativeListItems) -> bool {
        matches!(&*self.0.intrep.borrow(), IntRep::List { items, .. } if items.same_backing(backing))
    }

    /// Create one genuine header for a retained, authenticated whole backing.
    /// The engine bridge separately checks issuer, scope and private receipt.
    ///
    /// # Errors
    /// Refuses non-whole storage and backings with no remaining native header.
    pub fn from_retained_native_list_backing(
        backing: &crate::NativeListItems,
        protocol: NativeStringProtocol,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        if !backing.is_whole_backing() || !backing.has_native_header() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "retained native List backing",
            ));
        }
        let items = backing.native_header()?;
        Ok(Self::from_parts(
            None,
            IntRep::List {
                canonical: items.canonical_state(),
                items,
                string_protocol: Cell::new(Some(protocol)),
            },
        ))
    }

    /// Reconstruct original List storage with its explicitly transported canonical flag.
    #[must_use]
    pub fn list_with_native_canonical(items: Vec<Value>, canonical: bool) -> Self {
        Self::from_parts(None, IntRep::native_list(items, canonical, None))
    }

    /// Reconstruct cached List storage under its selected physical string recipe.
    #[must_use]
    pub fn list_with_native_canonical_in(
        items: Vec<Value>,
        canonical: bool,
        protocol: NativeStringProtocol,
    ) -> Self {
        let value = Self::list_with_native_canonical(items, canonical);
        value
            .seal_compound_string_protocol(protocol)
            .expect("new native List recipe");
        value
    }

    /// Retain a physical compound recipe without materializing members or bytes.
    ///
    /// # Errors
    /// Refuses replacement of an already selected backing recipe.
    pub fn with_native_compound_string_protocol(
        self,
        protocol: NativeStringProtocol,
    ) -> Result<Self, tcl_syntax::native_string::NativeStringUnavailable> {
        self.seal_compound_string_protocol(protocol)?;
        Ok(self)
    }

    /// Inspect original Dictionary members without list conversion or string generation.
    #[must_use]
    pub fn cached_dictionary_representation(&self) -> Option<Vec<(Value, Value)>> {
        match &*self.0.intrep.borrow() {
            IntRep::Dict(dict) => Some(dict.with_pairs(<[(Value, Value)]>::to_vec)),
            _ => None,
        }
    }

    /// Reconstruct a Dictionary cache from explicitly transported original members.
    #[must_use]
    pub fn from_native_dictionary_cache(pairs: Vec<(Value, Value)>) -> Self {
        Self::dict(pairs)
    }

    /// Borrow cached original Dictionary storage before retaining child capability handles.
    #[must_use]
    pub fn with_cached_dictionary_representation<R>(
        &self,
        operation: impl FnOnce(&[(Value, Value)], usize) -> R,
    ) -> Option<R> {
        match &*self.0.intrep.borrow() {
            IntRep::Dict(dict) => {
                Some(dict.with_pairs(|pairs| operation(pairs, dict.bucket_count())))
            }
            _ => None,
        }
    }

    /// Inspect original dictionary bucket capacity without conversion or string generation.
    #[must_use]
    pub fn cached_dictionary_bucket_count(&self) -> Option<usize> {
        match &*self.0.intrep.borrow() {
            IntRep::Dict(dict) => Some(dict.bucket_count()),
            _ => None,
        }
    }

    /// Reconstruct explicitly transported dictionary members and native bucket capacity.
    #[must_use]
    pub fn from_native_dictionary_cache_with_bucket_count(
        pairs: Vec<(Value, Value)>,
        buckets: Option<usize>,
    ) -> Self {
        Self::dict_with_hash_bucket_count(pairs, buckets)
    }

    /// Retain a dictionary member's existing native header after its view closes.
    pub(crate) fn retain_borrowed_member(original: Option<&Self>) -> Option<Self> {
        original.cloned()
    }

    /// Borrow an original cached dictionary member before acquiring a working value handle.
    pub(crate) fn with_cached_dictionary_member<R>(
        &self,
        key: &[u8],
        operation: impl FnOnce(Option<&Value>) -> R,
    ) -> Option<R> {
        let cache = self.0.intrep.borrow();
        let IntRep::Dict(dict) = &*cache else {
            return None;
        };
        Some(dict.with_pairs(|pairs| {
            let value = pairs
                .iter()
                .find(|(name, _)| {
                    name.resident_string_bytes()
                        .is_some_and(|bytes| bytes.as_ref() == key)
                })
                .map(|(_, value)| value);
            operation(value)
        }))
    }

    /// Prepare from borrowed original storage before retaining a working handle.
    pub(crate) fn prepare_native_dictionary(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<PreparedNativeDictionary, tcl_syntax::value::ValueError> {
        let value = if self.native_object_is_shared() {
            let value = self.duplicate_native_object_in(protocol);
            drop(value.native_object_dict_pairs(protocol)?);
            value
        } else {
            drop(self.native_object_dict_pairs(protocol)?);
            self.clone()
        };
        Ok(PreparedNativeDictionary { value, protocol })
    }

    /// Convert the borrowed original before observing and applying native COW.
    /// Existing path children and integer/body updates retain the conversion on aliases.
    pub(crate) fn prepare_native_dictionary_after_conversion(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<PreparedNativeDictionary, tcl_syntax::value::ValueError> {
        drop(self.native_object_dict_pairs(protocol)?);
        let value = if self.native_object_is_shared() {
            self.duplicate_native_object_in(protocol)
        } else {
            self.clone()
        };
        Ok(PreparedNativeDictionary { value, protocol })
    }

    /// Integer updates duplicate a shared C Dictionary without its resident string.
    pub(crate) fn prepare_native_dictionary_for_increment(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<PreparedNativeDictionary, tcl_syntax::value::ValueError> {
        let shared = self.native_object_is_shared();
        let prepared = self.prepare_native_dictionary_after_conversion(protocol)?;
        if shared && protocol.tcl_version().is_some() {
            *prepared.value.0.string.borrow_mut() = None;
            prepared
                .value
                .0
                .string_storage
                .set(NativeStringStorageIdentity::Unknown);
        }
        Ok(prepared)
    }

    /// Update a dictionary member from a borrowed original root, applying native COW first.
    pub(crate) fn native_dictionary_set_member(
        &self,
        key: Value,
        value: Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Value, tcl_syntax::value::ValueError> {
        let mut prepared = self.prepare_native_dictionary(protocol)?;
        prepared.set_member(key, value)?;
        Ok(prepared.into_value())
    }

    fn set_native_dictionary_member_in_place(
        &self,
        key: Value,
        value: Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let key_bytes = key.native_string_bytes(protocol).map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native dictionary key updater",
            )
        })?;
        if !matches!(*self.0.intrep.borrow(), IntRep::Dict(_)) {
            drop(self.native_object_dict_pairs(protocol)?);
        }
        let cache = self.0.intrep.borrow();
        let IntRep::Dict(dict) = &*cache else {
            unreachable!("native dict conversion");
        };
        let epoch = dict.next_epoch()?;
        let mut contents = dict.contents.borrow_mut();
        let existing = contents.pairs.iter().position(|(name, _)| {
            name.resident_string_bytes()
                .is_some_and(|bytes| bytes == key_bytes)
        });
        if let Some(index) = existing {
            Rc::make_mut(&mut contents.pairs)[index].1 = value;
        } else {
            contents.hash_order.insert(&key_bytes);
            Rc::make_mut(&mut contents.pairs).push((key, value));
        }
        dict.epoch.set(epoch);
        *self.0.string.borrow_mut() = None;
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *self.0.double_format.borrow_mut() = None;
        *self.0.source_location.borrow_mut() = None;
        Ok(())
    }

    /// Transfer an original Dictionary into an actual search backing lease.
    pub(crate) fn into_native_dictionary_search(
        self,
        protocol: NativeStringProtocol,
    ) -> Result<NativeDictionarySearch, tcl_syntax::value::ValueError> {
        if protocol
            .tcl_version()
            .is_none_or(|version| version < tcl_dialect::TclVersion::V8_5)
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native C Dictionary search",
            ));
        }
        if !matches!(*self.0.intrep.borrow(), IntRep::Dict(_)) {
            drop(self.native_object_dict_pairs(protocol)?);
        }
        self.seal_compound_string_protocol(protocol)
            .map_err(|error| {
                tcl_syntax::value::ValueError::NativeStringAccess(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                )
            })?;
        let backing = match &*self.0.intrep.borrow() {
            IntRep::Dict(dict) if dict.with_pairs(|pairs| !pairs.is_empty()) => {
                Some(Rc::clone(dict))
            }
            IntRep::Dict(_) => None,
            _ => unreachable!("native Dictionary search conversion"),
        };
        let epoch = backing.as_ref().map_or(0, |dict| dict.epoch.get());
        Ok(NativeDictionarySearch {
            root: Some(self),
            backing,
            next: 0,
            epoch,
        })
    }

    /// Convert through the actual native object-list recipe, retaining original member values.
    ///
    /// # Errors
    /// Preserves checked storage refusal and native dictionary parse failures.
    pub fn native_object_dict_pairs(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Vec<(Value, Value)>, tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        if let Some(pairs) = self.cached_dictionary_representation() {
            return Ok(pairs);
        }
        let cached_list = self.cached_list_representation();
        if protocol.is_jim084() && cached_list.is_some() && self.native_object_is_shared() {
            self.native_string_bytes(protocol).map_err(|_| {
                ValueError::CommandProtocolUnavailable("Jim shared List string updater")
            })?;
        }
        let items = match cached_list.as_ref() {
            Some((items, _)) => items.lifetime_view(),
            None if protocol.is_jim084() => self.native_object_list_elements(protocol)?,
            None => {
                let bytes = self.native_string_bytes(protocol).map_err(|_| {
                    ValueError::CommandProtocolUnavailable("native dictionary string updater")
                })?;
                let parsed = tcl_syntax::list::split_native_list_bytes(&bytes, protocol).map_err(
                    |error| ValueError::DictionaryParse {
                        error,
                        source: bytes.to_vec(),
                    },
                )?;
                crate::NativeListItems::new(
                    parsed
                        .into_iter()
                        .map(|element| Value::new_native_string_bytes(element.into_owned()))
                        .collect(),
                    false,
                )
            }
        };
        if items.len() % 2 != 0 {
            return Err(ValueError::MissingDictionaryValue);
        }
        let mut seen = std::collections::HashSet::new();
        let mut duplicates = false;
        for pair in items.as_chunks::<2>().0 {
            let bytes = pair[0].native_string_bytes(protocol).map_err(|_| {
                ValueError::CommandProtocolUnavailable("native dictionary key string updater")
            })?;
            duplicates |= !seen.insert(bytes);
        }
        if !protocol.is_jim084() && cached_list.is_some() && duplicates {
            self.native_string_bytes(protocol).map_err(|_| {
                ValueError::CommandProtocolUnavailable("native duplicate-key List string updater")
            })?;
        }
        let pairs = self.install_dictionary(&items);
        self.seal_compound_string_protocol(protocol)
            .map_err(|error| {
                ValueError::NativeStringAccess(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                )
            })?;
        Ok(pairs)
    }

    /// Retain one actual argv owner without copying its element handles.
    /// Diagnostic frame clones share this container and do not change the
    /// native sharing state of arguments before the command executes.
    #[cfg(test)]
    pub(crate) fn shared_list(items: Rc<Vec<Value>>) -> Self {
        Self::from_parts(None, {
            let items = crate::NativeListItems::invocation_view(items);
            IntRep::List {
                canonical: items.canonical_state(),
                items,
                string_protocol: Cell::new(None),
            }
        })
    }

    /// Retain original argv storage for diagnostics without another native header.
    pub(crate) fn invocation_list_view(items: &crate::NativeListItems) -> Self {
        let items = items.lifetime_view();
        Self::from_parts(
            None,
            IntRep::List {
                canonical: items.canonical_state(),
                items,
                string_protocol: Cell::new(None),
            },
        )
    }

    /// Materialize the native error list only at actual capture. This is when
    /// Jim's `Jim_NewListObj` acquires references to the original argv objects.
    pub(crate) fn capture_invocation_list(&self) -> Self {
        match &*self.0.intrep.borrow() {
            IntRep::List { items, .. } => Self::list((**items).clone()),
            _ => self.clone(),
        }
    }

    /// Apply Jim's native trim result to the actual object and count receipt.
    pub(crate) fn jim_string_trim_result(
        &self,
        plan: tcl_syntax::raw_string::JimStringTrimPlan,
    ) -> Self {
        let bytes = self.string_bytes();
        let start = plan.byte_start();
        let end = plan.byte_end();
        if start != 0 {
            // JimStringTrimLeft manufactures the original working header;
            // JimStringTrimRight then converts that SAME header before cutting.
            let working = Self::new_native_string_bytes(&bytes[start..]);
            return working.jim_trim_right_result(end - start, plan.right_conversion());
        }
        self.jim_trim_right_result(end, plan.right_conversion())
    }

    fn jim_trim_right_result(&self, end: usize, convert: bool) -> Self {
        let bytes = self.string_bytes();
        if convert {
            let count = match &*self.0.intrep.borrow() {
                IntRep::JimString(count) => *count,
                _ => None,
            };
            *self.0.intrep.borrow_mut() = IntRep::JimString(count);
            *self.0.double_format.borrow_mut() = None;
            if end == 0 {
                return Self::empty();
            }
        }
        if end < bytes.len() {
            if self.native_object_is_shared() {
                return Self::new_native_string_bytes(&bytes[..end]);
            }
            // Decide physical uniqueness before creating a return handle.
            // The native suffix cut retains an already cached Jim count.
            *self.0.string.borrow_mut() = Some(RawString::from_bytes(&bytes[..end]));
            self.0
                .string_storage
                .set(NativeStringStorageIdentity::Allocated);
        }
        self.clone()
    }

    /// A native dictionary value from canonical key/value pairs.
    #[must_use]
    pub(crate) fn dict(pairs: Vec<(Value, Value)>) -> Self {
        Self::dict_with_hash_bucket_count(pairs, None)
    }

    /// A native dictionary retaining the source table's bucket-array size.
    #[must_use]
    pub(crate) fn dict_with_hash_bucket_count(
        pairs: Vec<(Value, Value)>,
        bucket_count: Option<usize>,
    ) -> Self {
        let mut hash_order = TclStringHashOrder::default();
        if let Some(bucket_count) = bucket_count {
            hash_order.retain_bucket_count(bucket_count);
        }
        for (key, _) in &pairs {
            hash_order.insert(&key.string_bytes());
        }
        Self::from_parts(
            None,
            IntRep::Dict(Rc::new(DictRep::new(pairs, hash_order, None))),
        )
    }

    /// Construct a dictionary from original members under the actual string recipe.
    /// Key conversion reaches each original object before allocating its hash entry.
    ///
    /// # Errors
    /// Returns a checked native key materialisation refusal.
    pub fn native_dictionary_constructor(
        pairs: Vec<(Value, Value)>,
        bucket_count: Option<usize>,
        protocol: NativeStringProtocol,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        let mut hash_order = TclStringHashOrder::default();
        if let Some(bucket_count) = bucket_count {
            hash_order.retain_bucket_count(bucket_count);
        }
        let mut keys = Vec::with_capacity(pairs.len());
        for (key, _) in &pairs {
            let bytes = key.native_string_bytes(protocol).map_err(|error| {
                tcl_syntax::value::ValueError::NativeStringAccess(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                )
            })?;
            hash_order.insert(&bytes);
            keys.push(bytes);
        }
        let mut original_pairs: Vec<_> = pairs.into_iter().map(Some).collect();
        let pairs = canonical_dict_slots(keys.iter().map(AsRef::as_ref))
            .into_iter()
            .map(|(key, value)| {
                let first = original_pairs[key].take().expect("original dictionary key");
                if key == value {
                    first
                } else {
                    let last = original_pairs[value]
                        .take()
                        .expect("original dictionary value");
                    (first.0, last.1)
                }
            })
            .collect();
        Ok(Self::from_parts(
            None,
            IntRep::Dict(Rc::new(DictRep::new(pairs, hash_order, Some(protocol)))),
        ))
    }

    /// The string representation, generating and caching it from the typed rep
    /// on first call (Tcl's `Tcl_GetString` / lazy `updateStringProc`).
    #[must_use]
    pub fn to_str(&self) -> Rc<str> {
        self.try_to_str()
            .expect("the legacy Unicode accessor requires a checked value")
    }

    /// Exact string bytes, generating a typed representation without Unicode
    /// replacement. A list retains the original byte values of its elements.
    #[must_use]
    pub fn string_bytes(&self) -> Rc<[u8]> {
        self.to_raw_string_at_depth(0).0.bytes()
    }

    /// Reach the selected native character-length accessor on this object.
    /// Existing caches and byte-array shortcuts are inspected before strings.
    ///
    /// # Errors
    /// Refuses unavailable storage or a foreign retained String-unit recipe.
    pub fn native_character_count_with_protocol(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        representation: tcl_registry::native_string_length::NativeStringLengthRepresentation,
    ) -> Result<usize, tcl_syntax::value::ValueError> {
        if representation.preserves_short_string()
            && let Some(bytes) = self.resident_string_bytes()
            && bytes.len() < 2
        {
            return Ok(bytes.len());
        }
        if let IntRep::ByteArray(bytes) = &*self.0.intrep.borrow()
            && representation.counts_byte_array(self.0.string.borrow().is_none(), bytes.proper)
        {
            return Ok(bytes.bytes.len());
        }
        if protocol.is_jim084() {
            if let IntRep::JimString(Some(count)) = *self.0.intrep.borrow() {
                return Ok(count);
            }
            let bytes = self.native_string_bytes(protocol).map_err(|_| {
                ValueError::CommandProtocolUnavailable("native string length storage")
            })?;
            let count = RawString::from_bytes(bytes).jim084_characters().count();
            *self.0.intrep.borrow_mut() = IntRep::JimString(Some(count));
            return Ok(count);
        }
        if let IntRep::NativeString {
            protocol: origin,
            num_chars: Some(count),
            ..
        } = &*self.0.intrep.borrow()
        {
            if *origin != protocol {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native string cache origin",
                ));
            }
            return Ok(*count);
        }
        let bytes = self
            .native_string_bytes(protocol)
            .map_err(|_| ValueError::CommandProtocolUnavailable("native string length storage"))?;
        let version = protocol
            .tcl_version()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native string units",
            ))?;
        let count = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
            .decode_units(&bytes)
            .len();
        *self.0.intrep.borrow_mut() = IntRep::NativeString {
            protocol,
            num_chars: Some(count),
            unicode: None,
        };
        Ok(count)
    }

    /// Reach native C Unicode preparation and retain actual decoded units.
    ///
    /// # Errors
    /// Refuses unaudited storage, Jim's different string protocol or foreign cache units.
    pub fn native_unicode_units(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u32]>, tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        let version = protocol
            .tcl_version()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native C Unicode units",
            ))?;
        if let IntRep::NativeString {
            protocol: origin,
            unicode: Some(units),
            ..
        } = &*self.0.intrep.borrow()
        {
            if *origin != protocol {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native string cache origin",
                ));
            }
            return Ok(Rc::clone(units));
        }
        let bytes = self
            .native_string_bytes(protocol)
            .map_err(|_| ValueError::CommandProtocolUnavailable("native Unicode storage"))?;
        let units: Rc<[u32]> = Rc::from(
            tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version).decode_units(&bytes),
        );
        *self.0.intrep.borrow_mut() = IntRep::NativeString {
            protocol,
            num_chars: Some(units.len()),
            unicode: Some(Rc::clone(&units)),
        };
        Ok(units)
    }

    /// Count native character units without changing original string bytes.
    /// Jim's `Utf8Length` converts to its string intrep and retains that count;
    /// a later list/dictionary/numeric conversion withdraws the receipt.
    pub(crate) fn native_character_count(
        &self,
        model: tcl_dialect::StringCharacterModel,
        representation: Option<
            tcl_registry::native_string_length::NativeStringLengthRepresentation,
        >,
    ) -> Result<usize, UnicodeAccessError> {
        use tcl_registry::native_string_length::NativeStringLengthRepresentation;
        if representation.is_some_and(NativeStringLengthRepresentation::preserves_short_string)
            && let Some(string) = self.0.string.borrow().as_ref()
            && string.bytes().len() < 2
        {
            return Ok(string.bytes().len());
        }
        if let IntRep::ByteArray(bytes) = &*self.0.intrep.borrow()
            && representation.is_some_and(|policy| {
                policy.counts_byte_array(self.0.string.borrow().is_none(), bytes.proper)
            })
        {
            return Ok(bytes.bytes.len());
        }
        if model == tcl_dialect::StringCharacterModel::Jim084Utf8 {
            if let IntRep::JimString(Some(count)) = *self.0.intrep.borrow() {
                return Ok(count);
            }
            let count = self.to_raw_string_at_depth(0).0.character_count(model)?;
            *self.0.intrep.borrow_mut() = IntRep::JimString(Some(count));
            return Ok(count);
        }
        let count = self.to_raw_string_at_depth(0).0.character_count(model)?;
        if representation.is_some() {
            *self.0.intrep.borrow_mut() = IntRep::Str;
        }
        Ok(count)
    }

    /// Checked host Unicode projection of the actual string representation.
    /// Conversion failure preserves the original bytes and typed object.
    pub fn try_to_str(&self) -> Result<Rc<str>, UnicodeAccessError> {
        self.to_raw_string_at_depth(0).0.unicode()
    }

    /// [`Value::to_str`]'s recursive engine, with an explicit nesting-depth
    /// parameter (see [`MAX_LIST_TO_STR_DEPTH`]). `depth` is
    /// this value's nesting level within the *current* top-level `to_str()`
    /// call (0 at the root). Returns the string alongside whether rendering
    /// it anywhere in this subtree hit the depth cap — when it did, the
    /// result must not be cached (`self.0.string`): the same shared `Rc<Obj>`
    /// may also be reachable directly (or at a shallower offset) from
    /// elsewhere, where a fresh call starting back at depth 0 could
    /// legitimately render it in full, and caching the truncated result
    /// here would wrongly leak into that unrelated call. A value entirely
    /// within the cap is unaffected either way — same output, same caching
    /// — as the pre-fix implementation.
    fn to_raw_string_at_depth(&self, depth: u32) -> (RawString, bool) {
        if let Some(s) = self.0.string.borrow().as_ref() {
            return (s.clone(), false);
        }
        let selected = match &*self.0.intrep.borrow() {
            IntRep::List {
                string_protocol, ..
            } => string_protocol.get(),
            IntRep::Dict(dict) => dict.string_protocol.get(),
            _ => None,
        };
        if let Some(protocol) = selected {
            let bytes = self
                .native_string_bytes(protocol)
                .expect("selected compound updater requires checked child storage");
            return (RawString::from_bytes(bytes), false);
        }
        let (generated, past_cap) = match &*self.0.intrep.borrow() {
            IntRep::Str
            | IntRep::JimString(_)
            | IntRep::JimSource(_)
            | IntRep::JimScript(_)
            | IntRep::JimDictionarySubstitution { .. }
            | IntRep::JimInterpolated(_)
            | IntRep::JimScriptLine { .. }
            | IntRep::Expression(_) => (RawString::from_unicode(""), false),
            IntRep::NativeBytecode(_) => {
                unreachable!("native Bytecode has resident source and no string updater")
            }
            IntRep::NativeString {
                protocol,
                unicode: Some(unicode),
                ..
            } => {
                let version = protocol
                    .tcl_version()
                    .expect("native C String unit backing");
                let bytes = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
                    .encode_units(unicode)
                    .expect("validated native Unicode units");
                (RawString::from_bytes(bytes), false)
            }
            IntRep::NativeString {
                num_chars: Some(0),
                unicode: None,
                ..
            } => (RawString::from_bytes(b"".as_slice()), false),
            IntRep::NativeString { unicode: None, .. } => {
                panic!("native String has neither resident bytes nor Unicode backing")
            }
            IntRep::ByteArray(bytes) => (Self::raw_byte_array_string(&bytes.bytes), false),
            cache @ (IntRep::Int(_)
            | IntRep::Tcl84Long(_)
            | IntRep::CoercedDouble(_)
            | IntRep::Big { .. }
            | IntRep::Double(_)
            | IntRep::Bool(_)
            | IntRep::WordBoolean { .. }
            | IntRep::NativeIndex { .. }
            | IntRep::NativeNamespaceName(_)
            | IntRep::JimIndex(_)
            | IntRep::NativeEndOffset(_)
            | IntRep::NativeInstructionName(_)) => (self.render_native_cache_bytes(cache), false),
            IntRep::CompletionCode(_)
            | IntRep::NativeArraySearch { .. }
            | IntRep::FrameLevel { .. }
            | IntRep::NativePropertyName(_)
            | IntRep::NativeMethodName(_)
            | IntRep::NativeRegexp(_)
            | IntRep::JimRegexp(_)
            | IntRep::NativeCommandName(_)
            | IntRep::NativeCommandNameUnresolved(_)
            | IntRep::JimCommand(_)
            | IntRep::JimVariable(_)
            | IntRep::JimOption(_)
            | IntRep::NativeParsedVariableName(_)
            | IntRep::NativeLocalVariableName(_) => {
                panic!(
                    "completion-code cache has no native string updater; use checked native string access"
                )
            }
            IntRep::List { items, .. } => Self::render_list_bytes(items.iter(), depth),
            IntRep::Dict(dict) => Self::render_list_bytes(
                dict.pairs_backing()
                    .iter()
                    .flat_map(|(key, value)| [key, value]),
                depth,
            ),
        };
        if !past_cap {
            self.cache_generated_raw_string(&generated);
        }
        (generated, past_cap)
    }

    fn raw_byte_array_string(bytes: &[u8]) -> RawString {
        RawString::from_unicode(
            bytes
                .iter()
                .map(|&byte| char::from(byte))
                .collect::<String>(),
        )
    }

    fn cache_generated_raw_string(&self, generated: &RawString) {
        *self.0.string.borrow_mut() = Some(generated.clone());
        if let IntRep::List { canonical, .. } = &mut *self.0.intrep.borrow_mut() {
            canonical.set(true);
        }
        self.0.string_storage.set(
            if generated.bytes().is_empty()
                && matches!(*self.0.intrep.borrow(), IntRep::NativeNamespaceName(_))
            {
                NativeStringStorageIdentity::CanonicalEmpty
            } else {
                NativeStringStorageIdentity::Allocated
            },
        );
    }

    fn render_native_cache_bytes(&self, cache: &IntRep) -> RawString {
        let (bytes, _) = match cache {
            IntRep::NativeEndOffset(offset) => (
                RawString::from_bytes(offset.string_update().expect("legacy end-offset updater")),
                false,
            ),
            IntRep::Int(n) | IntRep::Tcl84Long(n) | IntRep::CoercedDouble(n) => {
                (RawString::from_unicode(n.to_string()), false)
            }
            IntRep::Big {
                negative,
                radix,
                digits,
            } => {
                let magnitude = num_bigint::BigInt::parse_bytes(digits.as_bytes(), *radix as u32)
                    .expect("validated native Big magnitude");
                let integer = if *negative { -magnitude } else { magnitude };
                (RawString::from_unicode(integer.to_string()), false)
            }
            IntRep::Double(f) => {
                let context = self.0.double_format.borrow();
                let rendered = match context.as_ref() {
                    Some(context) => {
                        number::format_double_native_selected(*f, context.policy, context.format())
                    }
                    None => number::format_double(*f),
                };
                (RawString::from_unicode(rendered), false)
            }
            IntRep::Bool(b) => (RawString::from_unicode(if *b { "1" } else { "0" }), false),
            IntRep::WordBoolean { value, version } => {
                assert_eq!(
                    *version,
                    tcl_dialect::TclVersion::V8_4,
                    "native word-Boolean descriptor has no string updater"
                );
                (
                    RawString::from_unicode(if *value { "1" } else { "0" }),
                    false,
                )
            }
            IntRep::NativeIndex { cache, .. } => (
                RawString::from_bytes(cache.word().expect("checked native Index table updater")),
                false,
            ),
            IntRep::NativeNamespaceName(cache) => {
                let recipe =
                    tcl_syntax::native_namespace_name::NativeNamespaceNameRecipe::for_tcl_version(
                        cache.version(),
                    );
                (
                    RawString::from_bytes(
                        cache
                            .string_update_bytes(recipe)
                            .expect("namespace primary without updater retains original string"),
                    ),
                    false,
                )
            }
            IntRep::JimIndex(index) => (RawString::from_bytes(index.string_bytes()), false),
            IntRep::NativeInstructionName(name) => (
                RawString::from_bytes(Rc::<[u8]>::from(name.string_bytes())),
                false,
            ),
            _ => unreachable!("scalar cache renderer selected for another primary"),
        };
        bytes
    }

    fn render_list_bytes<'a>(
        values: impl Iterator<Item = &'a Self>,
        depth: u32,
    ) -> (RawString, bool) {
        if MAX_LIST_TO_STR_DEPTH.exceeded(depth) {
            return (RawString::from_unicode(TOO_DEEPLY_NESTED_PLACEHOLDER), true);
        }
        let mut past_cap = false;
        let mut bytes = Vec::new();
        for (index, value) in values.enumerate() {
            if index != 0 {
                bytes.push(b' ');
            }
            let (element, capped) = value.to_raw_string_at_depth(depth + 1);
            past_cap |= capped;
            list::append_list_element(&mut bytes, &element.bytes(), index == 0);
        }
        (RawString::from_bytes(bytes), past_cap)
    }

    /// The value as a wide integer (`Tcl_GetWideIntFromObj`), caching the typed
    /// rep when it parses from the string side.
    pub fn as_int(&self) -> Result<i64, TclError> {
        if let Some(value) = self.restore_coerced_integer() {
            return Ok(value);
        }
        match &*self.0.intrep.borrow() {
            IntRep::Int(n) | IntRep::Tcl84Long(n) => return Ok(*n),
            IntRep::Bool(b) => return Ok(i64::from(*b)),
            IntRep::Str
            | IntRep::JimString(_)
            | IntRep::NativeString { .. }
            | IntRep::ByteArray(_)
            | IntRep::Big { .. }
            | IntRep::WordBoolean { .. }
            | IntRep::CompletionCode(_)
            | IntRep::NativeArraySearch { .. }
            | IntRep::FrameLevel { .. }
            | IntRep::NativePropertyName(_)
            | IntRep::NativeInstructionName(_)
            | IntRep::NativeMethodName(_)
            | IntRep::NativeRegexp(_)
            | IntRep::JimRegexp(_)
            | IntRep::JimIndex(_)
            | IntRep::NativeEndOffset(_)
            | IntRep::NativeIndex { .. }
            | IntRep::NativeCommandName(_)
            | IntRep::NativeCommandNameUnresolved(_)
            | IntRep::JimCommand(_)
            | IntRep::JimVariable(_)
            | IntRep::JimOption(_)
            | IntRep::NativeParsedVariableName(_)
            | IntRep::NativeLocalVariableName(_)
            | IntRep::NativeNamespaceName(_)
            | IntRep::JimSource(_)
            | IntRep::JimScript(_)
            | IntRep::JimDictionarySubstitution { .. }
            | IntRep::JimInterpolated(_)
            | IntRep::JimScriptLine { .. }
            | IntRep::Expression(_)
            | IntRep::NativeBytecode(_)
            | IntRep::Double(_)
            | IntRep::CoercedDouble(_)
            | IntRep::List { .. }
            | IntRep::Dict(_) => {}
        }
        let s = self.try_to_str().map_err(|_| {
            TclError::from(tcl_syntax::value::ValueError::NotIntegerBytes(
                self.string_bytes().to_vec(),
            ))
        })?;
        match number::parse_whole(&s) {
            Some(Number::Int(n)) => {
                *self.0.intrep.borrow_mut() = IntRep::Int(n);
                Ok(n)
            }
            _ => Err(TclError::new(format!(
                "expected integer but got {}",
                list::describe_bad_value(&s)
            ))),
        }
    }

    /// Convert the original object under an explicit native input extent and
    /// numeral grammar. Successful conversion changes this object's intrep;
    /// the complete original string, including bytes after NUL, is retained.
    pub(crate) fn native_int(
        &self,
        input: number::NativeScalarNumericInputPolicy,
        syntax: tcl_dialect::NumberSyntax,
    ) -> Result<i64, tcl_syntax::value::ValueError> {
        if let Some(value) = self.restore_coerced_integer() {
            return Ok(value);
        }
        match *self.0.intrep.borrow() {
            IntRep::Int(value) | IntRep::Tcl84Long(value) => return Ok(value),
            IntRep::Bool(value) => return Ok(i64::from(value)),
            _ => {}
        }
        let original = self.string_bytes();
        let text = std::str::from_utf8(input.input_bytes(&original))
            .map_err(|_| ValueError::NotIntegerBytes(original.to_vec()))?;
        let mut flags = number::ParseFlags::for_syntax(syntax);
        flags.integer_only = true;
        match number::parse_whole_with(text, flags) {
            Some(Number::Int(value)) => {
                self.cache_integer_representation(value);
                Ok(value)
            }
            _ => Err(ValueError::NotIntegerBytes(original.to_vec())),
        }
    }

    /// Native boolean getter. Jim accepts its boolean words after the scalar
    /// C-string boundary; expression numeric truth conversion is a separate door.
    pub(crate) fn native_bool(
        &self,
        input: number::NativeScalarNumericInputPolicy,
        syntax: tcl_dialect::NumberSyntax,
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        match *self.0.intrep.borrow() {
            IntRep::Int(value) | IntRep::Tcl84Long(value) => return Ok(value != 0),
            IntRep::Bool(value) => return Ok(value),
            IntRep::Double(value)
                if input == number::NativeScalarNumericInputPolicy::LengthDelimited =>
            {
                return Ok(value != 0.0);
            }
            _ => {}
        }
        let original = self.string_bytes();
        let text = std::str::from_utf8(input.input_bytes(&original))
            .map_err(|_| ValueError::NotBooleanBytes(original.to_vec()))?;
        if input == number::NativeScalarNumericInputPolicy::LengthDelimited
            && let Some(number) =
                number::parse_whole_with(text, number::ParseFlags::for_syntax(syntax))
        {
            return match number {
                Number::Int(value) => Ok(value != 0),
                Number::Double(value) => Ok(value != 0.0),
                Number::Big { .. } => Ok(true),
                Number::Nan { .. } => Err(ValueError::NotBooleanBytes(original.to_vec())),
            };
        }
        let value = tcl_syntax::boolean::parse_boolean_word(text)
            .ok_or_else(|| ValueError::NotBooleanBytes(original.to_vec()))?;
        if input == number::NativeScalarNumericInputPolicy::NulTerminatedJim084 {
            self.cache_integer_representation(i64::from(value));
        }
        Ok(value)
    }

    /// The value truncated to a 64-bit wide (`wide()`): an in-range integer
    /// as-is, else a bignum literal reduced modulo 2^64 and bit-cast to `i64`
    /// (two's-complement wrap — what C's `wide()` does, e.g.
    /// `wide(0x8000000000000000)` is `-9223372036854775808`). The VM has no
    /// arbitrary-precision rep, so this is the truncating window onto one.
    pub fn as_wide(&self) -> Result<i64, TclError> {
        if let Ok(n) = self.as_int() {
            return Ok(n);
        }
        let s = self.try_to_str().map_err(|_| {
            TclError::from(tcl_syntax::value::ValueError::NotIntegerBytes(
                self.string_bytes().to_vec(),
            ))
        })?;
        if let Some(Number::Big {
            negative,
            radix,
            digits,
        }) = number::parse_whole(&s)
        {
            let base = radix as u32;
            let mut acc: u64 = 0;
            for ch in digits.chars() {
                let d = ch.to_digit(base).unwrap_or(0);
                acc = acc.wrapping_mul(u64::from(base)).wrapping_add(u64::from(d));
            }
            let signed = acc.cast_signed();
            return Ok(if negative {
                signed.wrapping_neg()
            } else {
                signed
            });
        }
        Err(TclError::new(format!("expected integer but got \"{s}\"")))
    }

    /// The value as a 128-bit integer, parsing an out-of-`i64`-range integer
    /// literal whose magnitude fits `i128`. This is the VM's bounded stand-in
    /// for Tcl's arbitrary-precision integers: it covers the common large-value
    /// range (`2**70`, `10**21`, …) so `expr`/`mathop` arithmetic promotes on
    /// `i64` overflow instead of wrapping. `None` for non-integers or magnitudes
    /// beyond `i128`.
    #[must_use]
    pub fn as_i128(&self) -> Option<i128> {
        if let Ok(n) = self.as_int() {
            return Some(i128::from(n));
        }
        let s = self.try_to_str().ok()?;
        if let Some(Number::Big {
            negative,
            radix,
            digits,
        }) = number::parse_whole(&s)
        {
            let base = radix as u32;
            let mut acc: u128 = 0;
            for ch in digits.chars() {
                let d = ch.to_digit(base)?;
                acc = acc
                    .checked_mul(u128::from(base))?
                    .checked_add(u128::from(d))?;
            }
            let mag = i128::try_from(acc).ok()?;
            return Some(if negative { -mag } else { mag });
        }
        None
    }

    /// The value as a double (`Tcl_GetDoubleFromObj`).
    pub fn as_double(&self) -> Result<f64, TclError> {
        match &*self.0.intrep.borrow() {
            IntRep::Int(n) | IntRep::Tcl84Long(n) | IntRep::CoercedDouble(n) => {
                return Ok(*n as f64);
            }
            IntRep::Double(f) => return Ok(*f),
            IntRep::Bool(b) => return Ok(f64::from(i32::from(*b))),
            IntRep::Str
            | IntRep::JimString(_)
            | IntRep::NativeString { .. }
            | IntRep::ByteArray(_)
            | IntRep::Big { .. }
            | IntRep::WordBoolean { .. }
            | IntRep::CompletionCode(_)
            | IntRep::NativeArraySearch { .. }
            | IntRep::FrameLevel { .. }
            | IntRep::NativePropertyName(_)
            | IntRep::NativeInstructionName(_)
            | IntRep::NativeMethodName(_)
            | IntRep::NativeRegexp(_)
            | IntRep::JimRegexp(_)
            | IntRep::JimIndex(_)
            | IntRep::NativeEndOffset(_)
            | IntRep::NativeIndex { .. }
            | IntRep::NativeCommandName(_)
            | IntRep::NativeCommandNameUnresolved(_)
            | IntRep::JimCommand(_)
            | IntRep::JimVariable(_)
            | IntRep::JimOption(_)
            | IntRep::NativeParsedVariableName(_)
            | IntRep::NativeLocalVariableName(_)
            | IntRep::NativeNamespaceName(_)
            | IntRep::JimSource(_)
            | IntRep::JimScript(_)
            | IntRep::JimDictionarySubstitution { .. }
            | IntRep::JimInterpolated(_)
            | IntRep::JimScriptLine { .. }
            | IntRep::Expression(_)
            | IntRep::NativeBytecode(_)
            | IntRep::List { .. }
            | IntRep::Dict(_) => {}
        }
        let s = self.try_to_str().map_err(|_| {
            TclError::from(tcl_syntax::value::ValueError::NotDoubleBytes(
                self.string_bytes().to_vec(),
            ))
        })?;
        match number::parse_whole(&s) {
            Some(Number::Int(n)) => Ok(n as f64),
            Some(Number::Double(f)) => Ok(f),
            _ => Err(TclError::new(format!(
                "expected floating-point number but got {}",
                list::describe_bad_value(&s)
            ))),
        }
    }

    /// The value as a boolean (`Tcl_GetBooleanFromObj`).
    pub fn as_bool(&self) -> Result<bool, TclError> {
        match &*self.0.intrep.borrow() {
            IntRep::Bool(b) => return Ok(*b),
            IntRep::Int(n) | IntRep::Tcl84Long(n) | IntRep::CoercedDouble(n) => return Ok(*n != 0),
            IntRep::Double(f) => return Ok(*f != 0.0),
            IntRep::Str
            | IntRep::JimString(_)
            | IntRep::NativeString { .. }
            | IntRep::ByteArray(_)
            | IntRep::Big { .. }
            | IntRep::WordBoolean { .. }
            | IntRep::CompletionCode(_)
            | IntRep::NativeArraySearch { .. }
            | IntRep::FrameLevel { .. }
            | IntRep::NativePropertyName(_)
            | IntRep::NativeInstructionName(_)
            | IntRep::NativeMethodName(_)
            | IntRep::NativeRegexp(_)
            | IntRep::JimRegexp(_)
            | IntRep::JimIndex(_)
            | IntRep::NativeEndOffset(_)
            | IntRep::NativeIndex { .. }
            | IntRep::NativeCommandName(_)
            | IntRep::NativeCommandNameUnresolved(_)
            | IntRep::JimCommand(_)
            | IntRep::JimVariable(_)
            | IntRep::JimOption(_)
            | IntRep::NativeParsedVariableName(_)
            | IntRep::NativeLocalVariableName(_)
            | IntRep::NativeNamespaceName(_)
            | IntRep::JimSource(_)
            | IntRep::JimScript(_)
            | IntRep::JimDictionarySubstitution { .. }
            | IntRep::JimInterpolated(_)
            | IntRep::JimScriptLine { .. }
            | IntRep::Expression(_)
            | IntRep::NativeBytecode(_)
            | IntRep::List { .. }
            | IntRep::Dict(_) => {}
        }
        let s = self.try_to_str().map_err(|_| {
            TclError::from(tcl_syntax::value::ValueError::NotBooleanBytes(
                self.string_bytes().to_vec(),
            ))
        })?;
        if let Some(num) = number::parse_whole(&s) {
            return match num {
                Number::Int(n) => Ok(n != 0),
                Number::Double(f) => Ok(f != 0.0),
                // A parsed `Big` is beyond `i64`, hence never zero.
                Number::Big { .. } => Ok(true),
                // NaN is a domain error in a boolean context (tclsh 8.6/9.0:
                // "floating point value is Not a Number"), never truthy.
                Number::Nan { .. } => Err(TclError::new("floating point value is Not a Number")),
            };
        }
        // The canonical word acceptor (`ParseBoolean`, tclObj.c): any
        // unambiguous case-insensitive prefix of the six boolean words —
        // one home in `tcl_syntax::boolean`, oracle-table-pinned.
        match tcl_syntax::boolean::parse_boolean_word(&s) {
            Some(b) => Ok(b),
            None => Err(TclError::new(format!(
                "expected boolean value but got {}",
                list::describe_bad_value(&s)
            ))),
        }
    }

    /// The value as a list of elements (`Tcl_ListObjGetElements`), caching the
    /// typed rep on first parse.
    pub fn as_list(&self) -> Result<crate::NativeListItems, TclError> {
        self.native_list_elements(
            tcl_dialect::ListParse::Strict,
            tcl_dialect::EscapeSyntax::default(),
        )
        .map_err(|error| {
            TclError::with_error_code(
                error.full_message_bytes(&self.string_bytes()),
                error.error_code(),
            )
        })
    }

    /// Parse exact native bytes under retained list and escape grammars.
    /// Existing list elements remain the original objects; a dictionary keeps
    /// its original key/value objects when converted to a list.
    pub(crate) fn native_list_elements(
        &self,
        syntax: tcl_dialect::ListParse,
        escapes: tcl_dialect::EscapeSyntax,
    ) -> Result<crate::NativeListItems, list::ListError> {
        if let IntRep::List { items, .. } = &*self.0.intrep.borrow() {
            return Ok(items.lifetime_view());
        }
        let dict = match &*self.0.intrep.borrow() {
            IntRep::Dict(dict) if self.0.string.borrow().is_none() => Some(Rc::clone(dict)),
            _ => None,
        };
        let items: Rc<Vec<Value>> = if let Some(dict) = dict {
            Rc::new(
                dict.pairs_backing()
                    .iter()
                    .flat_map(|(key, value)| [key.clone(), value.clone()])
                    .collect(),
            )
        } else {
            let bytes = self.string_bytes();
            let elements = list::split_list_bytes_in(&bytes, syntax, escapes)?;
            Rc::new(
                elements
                    .into_iter()
                    .map(|element| Self::new_native_string_bytes(element.into_owned()))
                    .collect(),
            )
        };
        *self.0.intrep.borrow_mut() = IntRep::native_list(items.as_ref().clone(), false, None);
        Ok(self
            .cached_list_representation()
            .expect("installed native List")
            .0)
    }

    /// Reach native object-list conversion under its actual engine grammar.
    /// Current List members and pure Dictionary members retain object identity;
    /// resident dictionary spelling remains authoritative when parsing a list.
    ///
    /// # Errors
    /// Returns exact native list parse failure or checked storage refusal.
    pub fn native_object_list_elements(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<crate::NativeListItems, tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        self.seal_compound_string_protocol(protocol)
            .map_err(|error| {
                ValueError::NativeStringAccess(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                )
            })?;
        if let IntRep::List { items, .. } = &*self.0.intrep.borrow() {
            return Ok(items.lifetime_view());
        }
        let dict = match &*self.0.intrep.borrow() {
            IntRep::Dict(dict) if self.0.string.borrow().is_none() => Some(Rc::clone(dict)),
            _ => None,
        };
        let mut allocated = None;
        let items: Rc<Vec<Value>> = if let Some(dict) = dict {
            Rc::new(
                dict.pairs_backing()
                    .iter()
                    .flat_map(|(key, value)| [key.clone(), value.clone()])
                    .collect(),
            )
        } else {
            let context = if protocol.is_jim084() {
                Some(self.native_jim_context()?)
            } else {
                None
            };
            let source_info = context
                .as_ref()
                .map(|context| self.pin_native_jim_source_info(context))
                .transpose()?;
            let original = self.native_string_bytes(protocol).map_err(|_| {
                ValueError::CommandProtocolUnavailable("native object list storage")
            })?;
            if protocol
                .tcl_version()
                .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0)
            {
                allocated = Some(list::max_list_length_bytes(&original).max(1));
            }
            let elements =
                list::split_native_list_elements(&original, protocol).map_err(|error| {
                    ValueError::NativeListParse {
                        error,
                        source: original.to_vec(),
                        protocol,
                    }
                })?;
            let mut members = Vec::with_capacity(elements.len());
            for element in elements {
                let member = Self::new_native_string_bytes(element.value.into_owned());
                if let (Some(context), Some(info)) = (&context, &source_info) {
                    member.install_native_jim_source(
                        NativeJimSourceInfo {
                            filename: info.filename.clone(),
                            line: info.line.wrapping_add_unsigned(element.line_delta),
                        },
                        context,
                    )?;
                }
                members.push(member);
            }
            Rc::new(members)
        };
        self.replace_primary(IntRep::native_list(
            items.as_ref().clone(),
            false,
            Some(protocol),
        ));
        if let Some(allocated) = allocated
            && let IntRep::List { items, .. } = &*self.0.intrep.borrow()
        {
            items.set_capacity(allocated);
        }
        Ok(self
            .cached_list_representation()
            .expect("installed native List")
            .0)
    }

    pub(crate) fn native_dict_pairs(
        &self,
        syntax: tcl_dialect::ListParse,
        escapes: tcl_dialect::EscapeSyntax,
    ) -> Result<Vec<(Value, Value)>, tcl_syntax::value::ValueError> {
        if let IntRep::Dict(dict) = &*self.0.intrep.borrow() {
            return Ok(dict.with_pairs(<[(Value, Value)]>::to_vec));
        }
        let items = self
            .native_list_elements(syntax, escapes)
            .map_err(|error| tcl_syntax::value::ValueError::DictionaryParse {
                error,
                source: self.string_bytes().to_vec(),
            })?;
        if items.len() % 2 != 0 {
            return Err(tcl_syntax::value::ValueError::MissingDictionaryValue);
        }
        Ok(self.install_dictionary(&items))
    }

    fn install_dictionary(&self, items: &[Value]) -> Vec<(Value, Value)> {
        let string_protocol = match &*self.0.intrep.borrow() {
            IntRep::List {
                string_protocol, ..
            } => string_protocol.get(),
            IntRep::Dict(dict) => dict.string_protocol.get(),
            _ => None,
        };
        let keys: Vec<Rc<[u8]>> = items
            .as_chunks::<2>()
            .0
            .iter()
            .map(|chunk| chunk[0].string_bytes())
            .collect();
        let pairs: Vec<(Value, Value)> = canonical_dict_slots(keys.iter().map(AsRef::as_ref))
            .into_iter()
            .map(|(key_slot, value_slot)| {
                (
                    items[key_slot * 2].clone(),
                    items[value_slot * 2 + 1].clone(),
                )
            })
            .collect();
        let mut hash_order = TclStringHashOrder::default();
        for (key, _) in &pairs {
            hash_order.insert(&key.string_bytes());
        }
        *self.0.intrep.borrow_mut() = IntRep::Dict(Rc::new(DictRep::new(
            pairs.clone(),
            hash_order,
            string_protocol,
        )));
        pairs
    }

    /// Retained bucket count of the native dictionary representation.
    pub(crate) fn dict_hash_bucket_count(&self) -> Result<usize, tcl_syntax::value::ValueError> {
        self.native_dict_pairs(
            tcl_dialect::ListParse::Strict,
            tcl_dialect::EscapeSyntax::default(),
        )?;
        match &*self.0.intrep.borrow() {
            IntRep::Dict(dict) => Ok(dict.bucket_count()),
            _ => unreachable!("dict_pairs installs the dictionary representation"),
        }
    }

    /// Whether two handles refer to the same Tcl value object.
    #[must_use]
    pub(crate) fn is_same_object(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.try_to_str() {
            Ok(text) => write!(f, "Value({text:?})"),
            Err(_) => write!(f, "Value(bytes={:?})", self.string_bytes()),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_dictionary_constructor_retains_first_original_key_and_last_value() {
        let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0);
        let first_key = Value::new_native_string_bytes(b"k\0\xff".as_slice());
        let last_key = Value::new_native_string_bytes(b"k\0\xff".as_slice());
        let first_value = Value::int(1);
        let last_value = Value::int(2);
        let dictionary = Value::native_dictionary_constructor(
            vec![
                (first_key.clone(), first_value),
                (last_key.clone(), last_value.clone()),
            ],
            None,
            protocol,
        )
        .unwrap();
        dictionary
            .with_cached_dictionary_representation(|pairs, _| {
                assert_eq!(pairs.len(), 1);
                assert!(pairs[0].0.is_same_object(&first_key));
                assert!(!pairs[0].0.is_same_object(&last_key));
                assert!(pairs[0].1.is_same_object(&last_value));
                assert!(pairs[0].1.resident_string_bytes().is_none());
            })
            .unwrap();
        assert!(dictionary.resident_string_bytes().is_none());
    }

    #[test]
    fn selected_double_updaters_match_all_native_nonfinite_payloads() {
        let fixtures = [
            include_str!("../../tcl-syntax/testdata/native_nonfinite_double/8.4.20.tsv"),
            include_str!("../../tcl-syntax/testdata/native_nonfinite_double/8.5.19.tsv"),
            include_str!("../../tcl-syntax/testdata/native_nonfinite_double/8.6.18.tsv"),
            include_str!("../../tcl-syntax/testdata/native_nonfinite_double/9.0.4.tsv"),
            include_str!("../../tcl-syntax/testdata/native_nonfinite_double/9.1.0.tsv"),
            include_str!("../../tcl-syntax/testdata/native_nonfinite_double/jim.tsv"),
        ];
        let protocols = [
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1),
            NativeStringProtocol::Jim084,
        ];
        let bits = [
            0x7ff8_0000_0000_0000,
            0xfff8_0000_0000_0000,
            0x7ff0_0000_0000_0000,
            0xfff0_0000_0000_0000,
            0,
            0x8000_0000_0000_0000,
        ];
        let mut observations = 0;
        for (protocol, fixture) in protocols.into_iter().zip(fixtures) {
            for row in fixture.lines() {
                let (case, expected) = row.split_once('\t').unwrap();
                let case: usize = case.parse().unwrap();
                let value = f64::from_bits(bits[case]);
                let dialect = protocol.tcl_version().map_or_else(
                    || {
                        tcl_registry::InvocationDialect::of_profile(
                            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
                        )
                    },
                    tcl_registry::InvocationDialect::for_version,
                );
                let object = Value::from_native_scalar_cache(
                    NativeScalarCache::Number(Number::Double(value)),
                    None,
                    dialect,
                )
                .unwrap();
                assert!(object.resident_string_bytes().is_none());
                let bytes = object.native_string_bytes(protocol).unwrap();
                assert!(
                    matches!(object.number_representation(), Some(Number::Double(actual)) if actual.to_bits() == bits[case])
                );
                let hex: String = bytes.iter().fold(String::new(), |mut output, byte| {
                    use std::fmt::Write as _;
                    write!(output, "{byte:02x}").unwrap();
                    output
                });
                assert_eq!(hex, expected, "{protocol:?}/{case}");
                observations += 1;
            }
        }
        assert_eq!(observations, 36);
    }

    #[test]
    fn list_backing_canonical_flags_match_original_native_operations() {
        let fixtures = [
            include_str!("../../tcl-syntax/testdata/native_list_canonical/8.4.20.tsv"),
            include_str!("../../tcl-syntax/testdata/native_list_canonical/8.5.19.tsv"),
            include_str!("../../tcl-syntax/testdata/native_list_canonical/8.6.18.tsv"),
            include_str!("../../tcl-syntax/testdata/native_list_canonical/9.0.4.tsv"),
            include_str!("../../tcl-syntax/testdata/native_list_canonical/9.1.0.tsv"),
        ];
        let mut operations = 0;
        for (version, fixture) in tcl_dialect::TclVersion::ALL.into_iter().zip(fixtures) {
            let protocol = NativeStringProtocol::C(version);
            let rows: Vec<_> = fixture.lines().collect();
            for pair in rows.as_chunks::<2>().0 {
                let before_fields: Vec<_> = pair[0].split('\t').collect();
                let after_fields: Vec<_> = pair[1].split('\t').collect();
                let mode: usize = before_fields[0].parse().unwrap();
                let root = Value::native_list_constructor(
                    vec![
                        Value::new_native_string_bytes(b"a".as_slice()),
                        Value::new_native_string_bytes(b"b".as_slice()),
                    ],
                    protocol,
                );
                let root = if mode == 4 {
                    let parsed = Value::new_native_string_bytes(b"a   b".as_slice());
                    drop(parsed.native_object_list_elements(protocol).unwrap());
                    parsed
                } else {
                    root
                };
                if (2..5).contains(&mode) {
                    root.native_string_bytes(protocol).unwrap();
                }
                let duplicate =
                    (mode == 3 || mode == 5).then(|| root.duplicate_native_object_in(protocol));
                let selected = duplicate.as_ref().unwrap_or(&root);
                let before = selected.cached_list_representation().unwrap().1;
                assert_eq!(
                    selected.resident_string_bytes().is_some(),
                    before_fields[3] == "1"
                );
                if version != tcl_dialect::TclVersion::V8_4 {
                    assert_eq!(before, before_fields[4] == "1");
                }
                let result = if mode == 0 || mode == 5 {
                    selected.native_string_bytes(protocol).unwrap();
                    selected.clone()
                } else {
                    Value::native_list_append_elements(
                        Some(selected),
                        &[Value::new_native_string_bytes(b"c".as_slice())],
                        protocol,
                    )
                    .unwrap()
                };
                assert_eq!(
                    result.resident_string_bytes().is_some(),
                    after_fields[3] == "1"
                );
                if version != tcl_dialect::TclVersion::V8_4 {
                    assert_eq!(
                        result.cached_list_representation().unwrap().1,
                        after_fields[4] == "1"
                    );
                }
                operations += 1;
            }
        }
        assert_eq!(operations, 30);
    }

    #[test]
    fn selected_empty_compound_updaters_install_native_allocation_identity() {
        for protocol in [
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1),
            NativeStringProtocol::Jim084,
        ] {
            let list = Value::list_with_native_canonical_in(Vec::new(), false, protocol);
            assert!(list.native_string_bytes(protocol).unwrap().is_empty());
            assert_eq!(
                list.resident_string_storage_identity(),
                Some(protocol.compound_updater_storage())
            );
            assert_eq!(
                list.cached_list_representation().unwrap().1,
                protocol.updated_list_canonical(false, false)
            );
            if protocol == NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4) {
                continue;
            }
            let dict = Value::dict(Vec::new())
                .with_native_compound_string_protocol(protocol)
                .unwrap();
            assert!(dict.native_string_bytes(protocol).unwrap().is_empty());
            assert_eq!(
                dict.resident_string_storage_identity(),
                Some(protocol.compound_updater_storage())
            );
        }
    }

    fn compound_updater_original(
        case: u8,
        protocol: NativeStringProtocol,
        source_context: Option<&Rc<NativeJimObjectContext>>,
    ) -> Value {
        let string = |bytes: &[u8]| Value::new_native_string_bytes(bytes);
        let list = |items| Value::native_list_constructor(items, protocol);
        let dict = || {
            Value::dict(vec![(string(b"#key"), string(b"a\"b"))])
                .with_native_compound_string_protocol(protocol)
                .unwrap()
        };
        match case {
            0 => list(vec![string(b"#first"), string(b"#later")]),
            1 => list(vec![string(b"a\"b"), string(b"]")]),
            2 => list(vec![string(b"A\0\xff"), string(b"\\\n")]),
            3 => list(vec![
                list(vec![Value::int(17), string(b"a\"b")]),
                string(b"#later"),
            ]),
            4 => dict(),
            5 => list(vec![dict(), string(b"#later")]),
            6 => {
                let value = string(b"ORIGINAL");
                if let Some(context) = source_context {
                    value.bind_native_jim_context(context).unwrap();
                }
                drop(value.native_object_list_elements(protocol).unwrap());
                value
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn compound_updater_matches_all_six_native_original_object_fixtures() {
        use super::*;
        use tcl_dialect::TclVersion;
        const FIXTURES: [&str; 6] = [
            include_str!("../../tcl-syntax/testdata/native_compound_string_updaters/8.4.20.tsv"),
            include_str!("../../tcl-syntax/testdata/native_compound_string_updaters/8.5.19.tsv"),
            include_str!("../../tcl-syntax/testdata/native_compound_string_updaters/8.6.18.tsv"),
            include_str!("../../tcl-syntax/testdata/native_compound_string_updaters/9.0.4.tsv"),
            include_str!("../../tcl-syntax/testdata/native_compound_string_updaters/9.1.0.tsv"),
            include_str!("../../tcl-syntax/testdata/native_compound_string_updaters/jim.tsv"),
        ];
        let protocols = [
            NativeStringProtocol::C(TclVersion::V8_4),
            NativeStringProtocol::C(TclVersion::V8_5),
            NativeStringProtocol::C(TclVersion::V8_6),
            NativeStringProtocol::C(TclVersion::V9_0),
            NativeStringProtocol::C(TclVersion::V9_1),
            NativeStringProtocol::Jim084,
        ];
        let mut rows = 0;
        let mut unavailable = 0;
        for (protocol, fixture) in protocols.into_iter().zip(FIXTURES) {
            let source_context = protocol.is_jim084().then(|| {
                NativeJimObjectContext::new(tcl_registry::InvocationDialect::of_profile(
                    tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
                ))
                .unwrap()
            });
            for row in fixture.lines() {
                let fields: Vec<_> = row.split('\t').collect();
                rows += 1;
                if fields[1] == "unavailable" {
                    assert_eq!(protocol, protocols[0]);
                    unavailable += 1;
                    continue;
                }
                let case: u8 = fields[0].parse().unwrap();
                let root = compound_updater_original(case, protocol, source_context.as_ref());
                let state = || {
                    let primary = root.0.intrep.borrow();
                    let describe = |kind, child: &Value| {
                        let child_kind = match &*child.0.intrep.borrow() {
                            IntRep::List { .. } => "list",
                            IntRep::Dict(_) => "dict",
                            IntRep::Str => "none",
                            IntRep::JimSource(_) => "source",
                            IntRep::JimScript(_) => "script",
                            IntRep::JimScriptLine { .. } => "scriptline",
                            _ => "other",
                        };
                        (
                            kind,
                            root.resident_string_bytes().is_some(),
                            child_kind,
                            child.resident_string_bytes().is_some(),
                            child.native_object_reference_count(),
                        )
                    };
                    match &*primary {
                        IntRep::List { items, .. } => describe("list", &items[0]),
                        IntRep::Dict(dict) => {
                            dict.with_pairs(|pairs| describe("dict", &pairs[0].1))
                        }
                        _ => unreachable!(),
                    }
                };
                let before = state();
                assert_eq!(before.0, fields[1]);
                assert_eq!(before.1, fields[2] == "1");
                assert_eq!(before.2, fields[3]);
                assert_eq!(before.3, fields[4] == "1");
                assert_eq!(before.4, fields[10].parse::<usize>().unwrap());
                let bytes = root.native_string_bytes(protocol).unwrap();
                let hex = bytes.iter().fold(String::new(), |mut output, byte| {
                    use std::fmt::Write as _;
                    write!(output, "{byte:02x}").unwrap();
                    output
                });
                assert_eq!(hex, fields[5], "{protocol:?}: {row}");
                let after = state();
                assert_eq!(after.0, fields[6]);
                assert_eq!(after.1, fields[7] == "1");
                assert_eq!(after.2, fields[8]);
                assert_eq!(after.2, before.2);
                assert_eq!(after.3, fields[9] == "1");
                assert_eq!(after.4, fields[11].parse::<usize>().unwrap());
            }
        }
        assert_eq!(rows, 42);
        assert_eq!(unavailable, 2);
    }

    use super::*;

    #[test]
    fn native_dictionary_list_conversion_preserves_original_members_without_string() {
        use tcl_syntax::native_string::NativeStringProtocol;
        for protocol in [
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1),
            NativeStringProtocol::Jim084,
        ] {
            let key = Value::from_string_bytes(&b"k\0\xff"[..]);
            let child = Value::int(7);
            let parent = Value::from_native_dictionary_cache(vec![(key.clone(), child.clone())]);
            let alias = parent.clone();
            let members = parent.native_object_list_elements(protocol).unwrap();
            assert!(members[0].is_same_object(&key));
            assert!(members[1].is_same_object(&child));
            assert!(alias.resident_string_bytes().is_none());
            assert!(child.resident_string_bytes().is_none());
            assert!(!alias.cached_list_representation().unwrap().1);

            let resident = Value::from_native_dictionary_cache(vec![(key.clone(), child.clone())])
                .with_resident_string_bytes(Rc::from(&b"k FIRST k LAST"[..]));
            let source_context = protocol.is_jim084().then(|| {
                NativeJimObjectContext::new(tcl_registry::InvocationDialect::of_profile(
                    tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
                ))
                .unwrap()
            });
            if let Some(context) = &source_context {
                resident.bind_native_jim_context(context).unwrap();
            }
            let members = resident.native_object_list_elements(protocol).unwrap();
            assert_eq!(members.len(), 4);
            assert_eq!(
                members[1].resident_string_bytes().unwrap().as_ref(),
                b"FIRST"
            );
            assert_eq!(
                members[3].resident_string_bytes().unwrap().as_ref(),
                b"LAST"
            );
            assert!(!members[1].is_same_object(&child));
        }
    }

    #[test]
    fn native_name_materialisation_retains_binary_backing_and_resident_recipe() {
        use tcl_syntax::native_string::{NativeStringProtocol, NativeStringUnavailable};
        let binary = Value::byte_array(b"k\0\xff".as_slice());
        assert!(binary.resident_string_bytes().is_none());
        assert_eq!(
            binary
                .native_string_bytes(NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4))
                .unwrap()
                .as_ref(),
            b"k\xc0\x80\xc3\xbf"
        );
        assert_eq!(
            binary.byte_array_representation().unwrap().as_ref(),
            b"k\0\xff"
        );
        assert_eq!(
            binary
                .native_string_bytes(NativeStringProtocol::Jim084)
                .unwrap()
                .as_ref(),
            b"k\xc0\x80\xc3\xbf"
        );
        let resident = Value::byte_array_with_resident_string(
            b"PAYLOAD".as_slice(),
            Some(Rc::from(b"k\0\xff".as_slice())),
        );
        assert_eq!(
            resident
                .native_string_bytes(NativeStringProtocol::Jim084)
                .unwrap()
                .as_ref(),
            b"k\0\xff"
        );
        let jim_binary = Value::byte_array(b"k\0\xff".as_slice());
        assert_eq!(
            jim_binary.native_string_bytes(NativeStringProtocol::Jim084),
            Err(NativeStringUnavailable::JimByteArray)
        );
        assert!(jim_binary.resident_string_bytes().is_none());
        assert_eq!(
            jim_binary.byte_array_representation().unwrap().as_ref(),
            b"k\0\xff"
        );
    }

    #[test]
    fn jim_double_getter_retains_exact_integer_roundtrip_and_native_string_cache() {
        let jim = tcl_registry::InvocationDialect::of_profile(
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        );
        let context = NativeJimObjectContext::new(jim).unwrap();
        context.select_numeric_host(Rc::new(tcl_host_native::NativeHost::new()));
        for integer in [2, i64::MAX] {
            let value = Value::int(integer);
            value.bind_native_jim_context(&context).unwrap();
            let alias = value.clone();
            assert!(
                value
                    .native_scalar_getter(jim, NativeScalarGetterKind::Double)
                    .is_ok()
            );
            assert!(
                matches!(*alias.0.intrep.borrow(), IntRep::CoercedDouble(retained) if retained == integer)
            );
            assert_eq!(alias.resident_string_bytes().is_some(), integer == i64::MAX);
            assert_eq!(
                alias.native_scalar_getter(jim, NativeScalarGetterKind::Wide),
                Ok(NativeScalarGetterValue::Wide(integer))
            );
            assert_eq!(alias.integer_representation(), Some(integer));
        }
        let c = Value::int(2);
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        assert_eq!(
            c.native_scalar_getter(dialect, NativeScalarGetterKind::Double),
            Ok(NativeScalarGetterValue::Double(2.0))
        );
        assert_eq!(c.integer_representation(), Some(2));
        assert!(c.resident_string_bytes().is_none());
    }

    #[test]
    fn native_numeric_prefix_conversion_preserves_the_original_object_and_bytes() {
        use number::NativeScalarNumericInputPolicy;
        let syntax = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let syntax = tcl_registry::InvocationDialect::of_profile(syntax).numbers;
        let value = Value::native_jim_string(&[b'1', 0, 0xff][..], 3);
        let alias = value.clone();
        assert_eq!(
            value.native_int(NativeScalarNumericInputPolicy::NulTerminatedJim084, syntax),
            Ok(1)
        );
        assert!(value.is_same_object(&alias));
        assert_eq!(alias.integer_representation(), Some(1));
        assert_eq!(alias.string_bytes().as_ref(), &[b'1', 0, 0xff]);
        let c = Value::from_string_bytes(&[b'1', 0, 0xff][..]);
        assert_eq!(
            c.native_int(NativeScalarNumericInputPolicy::LengthDelimited, syntax),
            Err(tcl_syntax::value::ValueError::NotIntegerBytes(vec![
                b'1', 0, 0xff
            ]))
        );
        assert_eq!(c.integer_representation(), None);
        let word = Value::from_string_bytes(&b"true\0suffix"[..]);
        assert_eq!(
            word.native_bool(NativeScalarNumericInputPolicy::NulTerminatedJim084, syntax),
            Ok(true)
        );
        assert_eq!(word.integer_representation(), Some(1));
        assert_eq!(word.string_bytes().as_ref(), b"true\0suffix");
    }

    #[test]
    fn diagnostic_argv_observes_ownership_until_actual_error_capture() {
        let bytes = &[0xc3, b'A', 0xc3, 0xa9, b' ', b' '][..];
        let trim_set = RawString::from_bytes(&b" \t\n\r\0"[..]);
        let plan = RawString::from_bytes(bytes).jim084_trim_plan(&trim_set, false, true);
        let model = tcl_dialect::StringCharacterModel::Jim084Utf8;
        let argv = Rc::new(vec![Value::native_jim_string(bytes, 4)]);
        let diagnostic = Value::shared_list(Rc::clone(&argv));
        let retained_frame = diagnostic.clone();
        let result = argv[0].jim_string_trim_result(plan);
        assert!(result.is_same_object(&argv[0]));
        assert_eq!(result.string_bytes().as_ref(), &bytes[..4]);
        assert_eq!(result.native_character_count(model, None).unwrap(), 4);
        drop((result, retained_frame, diagnostic, argv));

        let argv = Rc::new(vec![Value::native_jim_string(bytes, 4)]);
        let diagnostic = Value::shared_list(Rc::clone(&argv));
        let captured = diagnostic.capture_invocation_list();
        let result = argv[0].jim_string_trim_result(plan);
        assert!(!result.is_same_object(&argv[0]));
        assert_eq!(result.string_bytes().as_ref(), &bytes[..4]);
        assert_eq!(result.native_character_count(model, None).unwrap(), 3);
        assert_eq!(argv[0].string_bytes().as_ref(), bytes);
        assert!(captured.as_list().unwrap()[0].is_same_object(&argv[0]));
    }

    #[test]
    fn byte_array_carrier_retains_original_storage_and_string_presence() {
        let payload: Rc<[u8]> = Rc::from(&[0xff, 0][..]);
        let pure = Value::byte_array(Rc::clone(&payload));
        assert!(pure.resident_string_bytes().is_none());
        assert_eq!(pure.byte_array_representation().unwrap(), payload);
        assert!(pure.is_pure_byte_array());

        let resident: Rc<[u8]> = Rc::from(&[0xc3, 0xbf, 0, 0xff][..]);
        let cached =
            Value::byte_array_with_resident_string(Rc::clone(&payload), Some(Rc::clone(&resident)));
        assert_eq!(cached.byte_array_representation().unwrap(), payload);
        assert_eq!(cached.resident_string_bytes().unwrap(), resident);
        assert!(!cached.is_pure_byte_array());
        assert!(cached.try_to_str().is_err());
        assert_eq!(cached.string_bytes(), resident);
        assert_eq!(cached.byte_array_representation().unwrap(), payload);
    }

    #[test]
    fn raw_string_projection_and_list_rendering_preserve_original_bytes() {
        let value = Value::from_raw_parts(
            Some(RawString::from_bytes(&[0xff, b' ', 0xed, 0xa0, 0x80][..])),
            IntRep::Str,
        );
        let alias = value.clone();
        assert!(value.try_to_str().is_err());
        assert_eq!(
            value.string_bytes().as_ref(),
            &[0xff, b' ', 0xed, 0xa0, 0x80]
        );
        let list = Value::list(vec![value.clone(), Value::string("tail")]);
        assert_eq!(
            list.string_bytes().as_ref(),
            &[
                b'{', 0xff, b' ', 0xed, 0xa0, 0x80, b'}', b' ', b't', b'a', b'i', b'l'
            ]
        );
        assert!(list.try_to_str().is_err());
        assert_eq!(
            list.as_list().unwrap()[0].string_bytes(),
            alias.string_bytes()
        );
        assert!(alias.existing_string_representation().is_none());
        assert!(format!("{value:?}").contains("bytes="));
    }

    #[test]
    fn byte_array_payload_survives_string_generation_and_shared_decoder_selection() {
        use tcl_registry::native_binary_value::{
            NativeBinaryDecodeInput, NativeBinaryDecodeSource,
        };
        let value = Value::byte_array(&[0xe9, 0][..]);
        let alias = value.clone();
        assert_eq!(
            alias
                .binary_decode_input(NativeBinaryDecodeSource::PureByteArrayOrString)
                .unwrap(),
            NativeBinaryDecodeInput::Bytes(vec![0xe9, 0])
        );
        assert_eq!(&*value.to_str(), "é\0");
        assert!(!alias.is_pure_byte_array());
        assert_eq!(
            alias
                .binary_decode_input(NativeBinaryDecodeSource::PureByteArrayOrString)
                .unwrap(),
            NativeBinaryDecodeInput::String("é\0".to_owned())
        );
        assert_eq!(
            alias
                .binary_decode_input(NativeBinaryDecodeSource::ProperByteArrayOrString)
                .unwrap(),
            NativeBinaryDecodeInput::Bytes(vec![0xe9, 0])
        );
    }

    #[test]
    fn character_length_obeys_actual_bytearray_conversion_and_purity() {
        use tcl_dialect::TclVersion;
        use tcl_registry::InvocationDialect;
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            let value = Value::byte_array(&[0xe9, b'a'][..]);
            assert_eq!(
                value.native_character_count(
                    dialect.characters.unwrap(),
                    dialect.string_length_representation()
                ),
                Ok(2)
            );
            assert_eq!(
                value.byte_array_representation().is_some(),
                version >= TclVersion::V8_6
            );
            assert_eq!(value.is_pure_byte_array(), version >= TclVersion::V8_6);

            let value = Value::byte_array(&[0xe9, b'a'][..]);
            assert_eq!(&*value.to_str(), "éa");
            assert_eq!(
                value.native_character_count(
                    dialect.characters.unwrap(),
                    dialect.string_length_representation()
                ),
                Ok(2)
            );
            assert_eq!(
                value.byte_array_representation().is_some(),
                version == TclVersion::V8_6
            );
        }
        // Modern C's short-string bypass retains a converted object too.
        let dialect = InvocationDialect::for_version(TclVersion::V9_0);
        let value = Value::byte_array(&b"a"[..]);
        assert_eq!(&*value.to_str(), "a");
        assert_eq!(
            value.native_character_count(
                dialect.characters.unwrap(),
                dialect.string_length_representation()
            ),
            Ok(1)
        );
        assert!(value.byte_array_representation().is_some());
    }

    #[test]
    fn byte_conversion_caches_are_selected_and_failed_conversion_preserves_source() {
        use tcl_registry::native_binary_value::NativeBinaryByteConversion;
        let cached = Value::native_jim_string(&[0xc3, b'A', 0xc3, 0xa9], 2);
        assert_eq!(
            cached
                .as_byte_array(NativeBinaryByteConversion::Utf8)
                .unwrap()
                .as_ref(),
            &[0xc3, b'A', 0xc3, 0xa9]
        );
        assert_eq!(
            cached
                .native_character_count(tcl_dialect::StringCharacterModel::Jim084Utf8, None)
                .unwrap(),
            2
        );
        assert!(cached.byte_array_representation().is_none());
        let value = Value::string("é");
        assert_eq!(
            &*value
                .as_byte_array(NativeBinaryByteConversion::Utf8)
                .unwrap(),
            &[0xc3, 0xa9]
        );
        assert_eq!(
            &*value
                .as_byte_array(NativeBinaryByteConversion::CheckedLatin1)
                .unwrap(),
            &[0xe9]
        );
        let wide = Value::list(vec![Value::string("€")]);
        assert_eq!(
            wide.as_byte_array(NativeBinaryByteConversion::CheckedLatin1)
                .unwrap_err(),
            ByteArrayAccessError::Conversion(
                tcl_registry::native_binary_value::NativeBinaryByteError {
                    offset: 0,
                    codepoint: 0x20ac
                }
            )
        );
        assert_eq!(&*wide.to_str(), "€");
        assert!(matches!(&*wide.0.intrep.borrow(), IntRep::List { .. }));
        assert_eq!(wide.as_list().unwrap().len(), 1);
    }

    #[test]
    fn int_shimmers_to_string() {
        assert_eq!(&*Value::int(42).to_str(), "42");
        assert_eq!(&*Value::double(2.0).to_str(), "2.0");
        assert_eq!(&*Value::bool(true).to_str(), "1");
        assert_eq!(&*Value::empty().to_str(), "");
    }

    #[test]
    fn string_shimmers_to_int() {
        assert_eq!(Value::string("42").as_int().unwrap(), 42);
        assert_eq!(Value::string("  -7 ").as_int().unwrap(), -7);
        assert!(Value::string("abc").as_int().is_err());
    }

    #[test]
    fn list_roundtrip() {
        let v = Value::list(vec![Value::int(1), Value::string("a b")]);
        assert_eq!(&*v.to_str(), "1 {a b}");
        let parsed = Value::string("1 {a b}").as_list().unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(&*parsed[1].to_str(), "a b");
    }

    #[test]
    fn boolean_words() {
        assert!(Value::string("yes").as_bool().unwrap());
        assert!(!Value::string("off").as_bool().unwrap());
        assert!(Value::string("3").as_bool().unwrap());
    }

    #[test]
    fn boolean_numeric_whitespace_matches_tcl() {
        assert!(Value::string("true").as_bool().unwrap());
        for value in [" true", "true ", "\ttrue\r"] {
            assert!(Value::string(value).as_bool().is_err(), "{value:?}");
        }
        assert!(Value::string("\t42\r").as_bool().unwrap());
        for value in ["\u{2003}42", "42\u{2003}"] {
            assert!(Value::string(value).as_bool().is_err(), "{value:?}");
        }
    }

    /// The boolean-context acceptor, oracle-pinned (tclsh 8.6/9.0): word
    /// prefixes resolve, any number compares against zero, `Inf` is truthy —
    /// and `NaN` is a domain error ("floating point value is Not a Number"),
    /// never a truth value.
    #[test]
    fn boolean_context_matches_oracle() {
        assert!(Value::string("tru").as_bool().unwrap());
        assert!(!Value::string("of").as_bool().unwrap());
        assert!(Value::string("0.5").as_bool().unwrap());
        assert!(!Value::string("0x0").as_bool().unwrap());
        assert!(Value::string("Inf").as_bool().unwrap());
        assert!(Value::string("18446744073709551616").as_bool().unwrap());
        let err = Value::string("NaN").as_bool().unwrap_err();
        assert!(
            err.message_unicode()
                .expect("Unicode fixture error")
                .contains("Not a Number"),
            "NaN must be the C domain error, got: {err:?}"
        );
        assert!(Value::string("o").as_bool().is_err(), "ambiguous prefix");
    }

    /// `Value::to_str`'s descent into `IntRep::List` children recursing once
    /// per nesting level with no depth cap — a plain `for {set i 0} {$i<N}
    /// {incr i} {set v [list $v]}` loop builds the input, no `{*}` tricks
    /// needed — empirically overflows the native stack (SIGABRT) between
    /// depth 1200 and 1250 on a 2 MiB thread (`cargo test`'s per-test
    /// default). 2000 is comfortably past both that crash range and
    /// `MAX_LIST_TO_STR_DEPTH` (256); the assertion is that `to_str` returns
    /// at all, not what it returns.
    ///
    /// Deliberately NOT 50,000+: constructing (and, at the end of this
    /// test, dropping) a `Value::list` chain nested that deep is its own,
    /// unrelated native-stack risk — `Value` has no custom `Drop` impl, so
    /// the compiler-generated recursive drop glue walks the same chain a
    /// naive `to_str` would (empirically, SIGABRT between depth 3500 and 4000
    /// on a 2 MiB thread for construction+drop alone, independent of
    /// `to_str` or any other operation). That is a separate, genuinely
    /// unbounded-depth concern in `Value`'s representation itself, and this
    /// test does not cover it.
    #[test]
    fn deeply_nested_list_to_str_survives() {
        const DEPTH: usize = 2_000;
        let mut v = Value::string("leaf");
        for _ in 0..DEPTH {
            v = Value::list(vec![v]);
        }
        let s = v.to_str();
        assert!(!s.is_empty());
    }

    /// A moderately nested `[list $v]` chain (well under
    /// `MAX_LIST_TO_STR_DEPTH`) is byte-for-byte unaffected by the depth
    /// cap: its output matches an independent oracle (hand-driving
    /// `list::join_list` the same number of times, entirely outside
    /// `Value`) exactly.
    #[test]
    fn moderately_nested_list_to_str_matches_naive_join() {
        const DEPTH: usize = 20;
        let mut v = Value::string("a b");
        let mut expected = "a b".to_string();
        for _ in 0..DEPTH {
            v = Value::list(vec![v]);
            expected = list::join_list(std::iter::once(expected.as_str()));
        }
        assert_eq!(&*v.to_str(), expected.as_str());
    }

    /// The depth-cap fallback must not poison the shared string cache: a
    /// value that only exceeds `MAX_LIST_TO_STR_DEPTH` because of *where*
    /// it happens to be nested inside a deeper structure (reached via a
    /// shared `Rc<Obj>`, e.g. the same sublist embedded in two different
    /// places) must still render its true value when addressed directly
    /// afterwards, starting fresh at depth 0 — not a cached placeholder
    /// left over from the capped render.
    #[test]
    fn depth_cap_fallback_does_not_poison_the_shared_string_cache() {
        // `chain`: 10 levels of `[list $v]` around a value that needs
        // brace-quoting ("a b"), well within the cap on its own.
        let mut chain = Value::string("a b");
        let mut expected = "a b".to_string();
        for _ in 0..10 {
            chain = Value::list(vec![chain]);
            expected = list::join_list(std::iter::once(expected.as_str()));
        }
        // `far_outer`: the *same* `chain` object (`Rc` clone), wrapped 300
        // more levels deep — deep enough that reaching `chain` from here
        // exceeds `MAX_LIST_TO_STR_DEPTH` (256), even though `chain` itself
        // is shallow.
        let mut far_outer = chain.clone();
        for _ in 0..300 {
            far_outer = Value::list(vec![far_outer]);
        }
        // Force the deep, capped render first: `chain`'s own `Obj` must come
        // out of this untouched (not cached with the placeholder).
        let far_str = far_outer.to_str();
        assert!(
            far_str.contains(TOO_DEEPLY_NESTED_PLACEHOLDER),
            "expected the placeholder somewhere in: {far_str}"
        );
        // A fresh call directly on the same shared value, starting back at
        // depth 0, must render its true (well-within-cap) value — not the
        // placeholder that would have leaked in had the capped render above
        // wrongly cached it.
        assert_eq!(&*chain.to_str(), expected.as_str());
    }
}

#[cfg(test)]
#[path = "value_scalar_tests.rs"]
mod native_scalar_tests;

#[cfg(test)]
#[path = "native_append_tests.rs"]
mod native_append_tests;

#[cfg(test)]
#[path = "native_number_tests.rs"]
mod native_number_tests;

#[cfg(test)]
#[path = "value_source_tests.rs"]
mod native_source_tests;

#[cfg(test)]
#[path = "value/native_integer_formatter_tests.rs"]
mod native_integer_formatter_tests;

#[cfg(test)]
mod jim_interpreter_retirement_tests {
    use super::*;
    use tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole as Role;

    #[test]
    fn externally_retained_original_survives_without_retired_interpreter_authority() {
        let dialect = tcl_registry::InvocationDialect::of_profile(
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        );
        let context = NativeJimObjectContext::new(dialect).unwrap();
        let original = context.empty_object().clone();
        assert!(Rc::ptr_eq(
            &original.native_jim_context().unwrap(),
            &context
        ));
        context.release_object(Role::Empty);
        context.retire();
        assert!(original.native_jim_context().is_err());
        assert_eq!(original.resident_string_bytes().unwrap().as_ref(), b"");
    }
}
