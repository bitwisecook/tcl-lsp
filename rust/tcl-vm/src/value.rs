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

mod native_boolean_truth;
mod native_compound;
mod native_list_storage;
mod native_namespace_name;
mod native_retirement;

use tcl_cmd_core::namespace::TclStringHashOrder;
use tcl_core_types::RecursionLimit;
#[cfg(test)]
use tcl_syntax::list;
use tcl_syntax::native_string::{NativeStringProtocol, NativeStringStorageIdentity};
use tcl_syntax::number;
#[cfg(test)]
use tcl_syntax::number::Number;
use tcl_syntax::raw_string::{RawString, UnicodeAccessError};
#[cfg(test)]
use tcl_syntax::scalar_getter::{NativeScalarCache, NativeScalarGetterValue};
use tcl_syntax::scalar_getter::{NativeScalarGetterKind, NativeScalarGetterProtocol};
use tcl_syntax::value::ValueError;

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
        // After the table's destruction at thread exit, a context gets its
        // own cell at the release default rather than panicking.
        let precision = DOUBLE_PRECISIONS
            .try_with(|values| {
                Rc::clone(
                    values
                        .borrow_mut()
                        .entry(key)
                        .or_insert_with(|| Rc::new(Cell::new(policy.default_precision()))),
                )
            })
            .unwrap_or_else(|_| Rc::new(Cell::new(policy.default_precision())));
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

type NativeStringAccess<'a, E> = dyn FnMut(&Value) -> Result<Rc<[u8]>, E> + 'a;
type NativeValueCheck<'a, E> = dyn FnMut(&Value) -> Result<(), E> + 'a;

#[derive(Clone, Copy)]
struct NativeScalarProbeRecipe<'a> {
    protocol: NativeScalarGetterProtocol,
    kind: NativeScalarGetterKind,
    string_protocol: NativeStringProtocol,
    environment: Option<&'a dyn tcl_platform::NumericEnvironment>,
    target: Option<tcl_syntax::scalar_getter::NativeScalarGetterTarget>,
    checked_jim: bool,
    jim_abi: &'a Cell<Option<tcl_platform::NativeCIntegerAbi>>,
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
            // The native reference is released before its callback. The Rust
            // handle still protects allocation until its own field is dropped.
            self.retain_native_lifetime_pin();
            let _retiring = RetiringNativeReference(self);
            self.retire_native_header();
        }
    }
}

struct RetiringNativeReference<'a>(&'a Value);
impl Drop for RetiringNativeReference<'_> {
    fn drop(&mut self) {
        self.0.release_native_lifetime_pin();
    }
}

#[cfg(test)]
type NativeRetirementObserver = Box<dyn FnOnce(&Value)>;

#[cfg(test)]
thread_local! {
    static NATIVE_RETIREMENT_OBSERVERS: RefCell<HashMap<usize, NativeRetirementObserver>> = RefCell::new(HashMap::new());
}

mod byte_array;
mod cached_representations;
mod dictionary_representation;
mod jim_object_context;
mod list_representation;
mod native_append;
mod native_header;
mod native_scalar_getters;
mod native_string;
mod numeric_conversion;
mod observation;
mod scalar_cache;
mod string_rendering;

pub(crate) use native_append::VmAppendObjects;

#[cfg(test)]
fn observe_native_retirement(original: &Value) {
    let observer = NATIVE_RETIREMENT_OBSERVERS.with(|observers| {
        observers
            .borrow_mut()
            .remove(&(Rc::as_ptr(&original.0) as usize))
    });
    if let Some(observer) = observer {
        observer(original);
    }
}

/// Whether `protocol`'s string updater formats an integer through the host's
/// C formatter. Only C 8.4's wide updater calls `sprintf`; later releases and
/// Jim format integers themselves.
pub(crate) fn host_formats_integers(protocol: NativeStringProtocol) -> bool {
    protocol == NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4)
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
mod native_jim_pool;
mod native_jim_subcommand;
pub(crate) use native_jim_pool::{JimAllocationScope, JimCommandNameObservation};
mod native_jim_lookup;
mod native_method_name;
mod native_property_name;
mod native_substitution;
mod native_variable_name;
pub(crate) use native_jim_lookup::{JimCommandCache, JimVariableCache};

pub(crate) struct NativeJimObjectContext {
    numeric_host: RefCell<Option<Rc<dyn tcl_platform::Host>>>,
    live: std::cell::Cell<bool>,
    pool: Rc<native_jim_pool::JimPool>,
    objects: [RefCell<Option<Value>>; 10],
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
    jim_allocation: RefCell<Option<native_jim_pool::ActualAllocation>>,
    incarnation: tcl_runtime_api::native_object_incarnation::NativeObjectIncarnation,
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

struct ObjectRepresentations {
    string: RefCell<Option<RawString>>,
    string_storage: Cell<NativeStringStorageIdentity>,
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

pub(crate) struct ExpressionSourceInput<'a> {
    pub(crate) dialect: tcl_registry::InvocationDialect,
    pub(crate) profile: &'a tcl_dialect::DialectProfileKey,
    pub(crate) protocol: tcl_syntax::native_string::NativeStringProtocol,
    pub(crate) policy: &'a crate::compiled::NativeCompilerPolicy,
    pub(crate) preparation: tcl_syntax::expr::parser::ExpressionSourceCachePreparation,
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
    /// Actual selected C84 wideInt storage; unrelated to generic Int construction.
    Tcl84Wide(i64),
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
    /// Original `ParseSubCmd` table cache, distinct from Jim Enum and C Index.
    JimSubcommand(tcl_core_types::NativeJimSubcommandCache),
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
    /// Constructor provenance is independent of the selected getter protocol.
    /// Imported backing and string-to-byte conversions have no constructor receipt.
    origin: Option<tcl_registry::native_string_materialization::ByteArrayStringRecipe>,
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

/// Readonly observation of the original table during an actual member scan.
/// It owns neither a Dictionary backing nor a native child reference.
pub(crate) struct OriginalNativeDictionaryCursor {
    backing: std::rc::Weak<DictRep>,
    epoch: u64,
}

/// Allocation-only header/immutable-backing receipt for C84's captured pointer.
/// An independently existing shallow owner can keep this exact allocation live;
/// this receipt itself does not provide that owner or a general primary epoch.
pub struct NativeBinaryStorageReceipt {
    header: NativeObjectLifetimeLease,
    backing: std::rc::Weak<ByteArrayRep>,
}
impl NativeBinaryStorageReceipt {
    pub(crate) fn check(&self, value: &Value) -> Result<(), ValueError> {
        value.check_native_header()?;
        let original = value.0.intrep.borrow();
        if self.header.value().native_object_identity() != value.native_object_identity()
            || !matches!(&*original, IntRep::ByteArray(backing) if std::rc::Weak::ptr_eq(&Rc::downgrade(backing), &self.backing))
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "retired original C binary storage",
            ));
        }
        Ok(())
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

/// Prepared physical Dictionary root, with its original COW decision retained.
pub(crate) struct PreparedNativeDictionary {
    value: Value,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
}

impl Value {
    fn native_stage_check<E>(
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<(), E> {
        match check {
            Some(check) => check(),
            None => Ok(()),
        }
    }

    fn native_stage_result<E: From<ValueError>, T>(
        reached: Result<T, ValueError>,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<T, E> {
        Self::native_stage_check(check)?;
        reached.map_err(Into::into)
    }

    fn replace_primary_using<E: From<ValueError>>(
        &self,
        replacement: IntRep,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<(), E> {
        self.replace_primary_using_with(|| Ok(replacement), check)
    }

    fn replace_primary_using_with<E: From<ValueError>>(
        &self,
        replacement: impl FnOnce() -> Result<IntRep, E>,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<(), E> {
        Self::native_stage_check(check)?;
        self.check_native_header()?;
        if check.is_some() {
            let retired = self.0.intrep.replace(IntRep::Str);
            drop(retired);
            Self::native_stage_check(check)?;
            self.check_native_header()?;
        }
        let reached = replacement();
        Self::native_stage_check(check)?;
        self.check_native_header()?;
        self.replace_primary(reached?);
        Self::native_stage_check(check)
    }

    fn replace_primary(&self, replacement: IntRep) {
        let retired = self.0.intrep.replace(replacement);
        drop(retired);
    }

    /// Inspect the current representation without requesting a conversion.
    #[must_use]
    pub fn has_list_representation(&self) -> bool {
        matches!(*self.0.intrep.borrow(), IntRep::List { .. })
    }

    /// C9.1 `Tcl_IsEmpty`: a resident string's length; otherwise a
    /// dictionary's size, a list's length, or a length-one number or boolean;
    /// otherwise the generated string's.
    #[must_use]
    pub fn native_is_empty(&self) -> bool {
        if let Some(bytes) = self.resident_string_bytes() {
            return bytes.is_empty();
        }
        let known = match &*self.0.intrep.borrow() {
            IntRep::Dict(rep) => Some(rep.with_pairs(<[_]>::is_empty)),
            IntRep::List { items, .. } => Some(items.is_empty()),
            IntRep::Int(_) | IntRep::Big { .. } | IntRep::Double(_) | IntRep::Bool(_) => {
                Some(false)
            }
            _ => None,
        };
        known.unwrap_or_else(|| self.string_bytes().is_empty())
    }

    fn from_parts(string: Option<Rc<str>>, intrep: IntRep) -> Self {
        Self::from_raw_parts(string.map(RawString::from_unicode), intrep)
    }

    fn from_raw_parts(string: Option<RawString>, intrep: IntRep) -> Self {
        Self(
            native_jim_pool::construct(ObjectRepresentations {
                string: RefCell::new(string),
                string_storage: Cell::new(NativeStringStorageIdentity::Unknown),
                intrep: RefCell::new(intrep),
                double_format: RefCell::new(None),
                source_location: RefCell::new(None),
                jim_context: RefCell::new(None),
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

    /// Whether the primary representation is a list or a dictionary.
    pub(crate) fn is_native_compound(&self) -> bool {
        matches!(
            &*self.0.intrep.borrow(),
            IntRep::List { .. } | IntRep::Dict(_)
        )
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

    /// Test an actual carried option set without generating its string or
    /// reading nested error objects. Resident emptiness retains its original
    /// meaning; otherwise only an owning List/Dictionary size is available.
    pub(crate) fn native_return_options_nonempty(
        &self,
        protocol: Option<NativeStringProtocol>,
    ) -> Result<bool, ValueError> {
        self.check_native_header()?;
        // Existing bytes require no native getter or compound recipe. Unsealed
        // owning shapes below remain a pure compatibility query, never a native
        // List/Dictionary construction or execution receipt.
        if let Some(bytes) = self.resident_string_bytes() {
            return Ok(!bytes.is_empty());
        }
        let primary = self.0.intrep.borrow();
        let structural = match &*primary {
            IntRep::List {
                items,
                string_protocol,
                ..
            } => {
                if string_protocol
                    .get()
                    .is_some_and(|retained| Some(retained) != protocol)
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "foreign native return-options List backing",
                    ));
                }
                Some(items.checked_length()? != 0)
            }
            IntRep::Dict(dictionary) => {
                if dictionary
                    .string_protocol
                    .get()
                    .is_some_and(|retained| Some(retained) != protocol)
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "foreign native return-options Dictionary backing",
                    ));
                }
                Some(dictionary.with_pairs(|pairs| !pairs.is_empty()))
            }
            _ => None,
        };
        structural.ok_or(ValueError::CommandProtocolUnavailable(
            "native return-options storage is unavailable",
        ))
    }

    /// Retain original arguments at a reached C error-context capture.
    /// A diagnostic argv view owns no native member references before this
    /// event. Capture creates an owning List without reconstructing strings;
    /// an already owned context keeps its original header.
    pub(crate) fn capture_native_error_context(
        &self,
        protocol: NativeStringProtocol,
    ) -> Result<Self, ValueError> {
        self.check_native_header()?;
        if !protocol
            .tcl_version()
            .is_some_and(tcl_dialect::TclVersion::has_error_stack)
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "native C error-context capture",
            ));
        }
        let primary = self.0.intrep.borrow();
        if let IntRep::List {
            items,
            string_protocol,
            ..
        } = &*primary
        {
            if string_protocol
                .get()
                .is_some_and(|retained| retained != protocol)
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "foreign native error-context List backing",
                ));
            }
            let members = items.elements()?;
            if !items.owns_native_header() {
                return Ok(Self::native_list_constructor(members.to_vec(), protocol));
            }
        }
        Ok(self.clone())
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

    /// Whether two handles refer to the same Tcl value object.
    #[must_use]
    pub(crate) fn is_same_object(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0) && self.0.incarnation.same_as(&other.0.incarnation)
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
mod tests;

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
mod jim_interpreter_retirement_tests;

#[cfg(test)]
#[path = "value_scalar_target_tests.rs"]
mod native_scalar_target_tests;

#[cfg(test)]
#[path = "value/native_duplicate_controls.rs"]
mod native_duplicate_controls;

#[cfg(test)]
mod jim_result_publication_tests;
