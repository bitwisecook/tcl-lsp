// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conditional native stock-object list conversions. Original object class,
//! effect closure and actual normal continuation remain independent proofs.

use crate::native_compilation::{NativeCompilationSelection, SuccessfulHandlerSpec};
use crate::native_result::{NativeListMethodProvider, NativeResultContract};
use crate::{IntrinsicId, InvocationArguments, InvocationFacts, SemanticOperationId};
use tcl_dialect::TclVersion;
use tcl_syntax::native_string::NativeStringProtocol;

/// Independently authenticated original stock cache, never inferred from bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeStockListInputClass {
    /// Stock hooks are sealed, but no current cache class is proved.
    Unknown,
    /// Plain native string object with no intrep.
    String,
    /// Ordinary native List cache.
    List,
    /// Ordinary native Dictionary cache.
    Dictionary,
    /// Native C Index primary with ordinary stock List conversion.
    Index,
    /// Native C command-name cache with ordinary stock List conversion.
    CommandName,
    /// Genuine C91 property-name primary with resident ordinary List conversion.
    PropertyName,
    /// Genuine native C86+ `TclOO` method-name primary with no string updater.
    MethodName,
    /// Native C86+ instruction-name primary with its authentic string updater.
    InstructionName,
    /// Authentic pinned Jim command/variable lookup primary with ordinary List conversion.
    JimLookup,
    /// Native namespace-name primary with ordinary stock List conversion.
    NamespaceName,
    /// Authentic C parsed variable-name primary.
    ParsedVariableName,
    /// Authentic C indexed local-name primary.
    LocalVariableName,
    /// Resident-only native C array-search handle cache.
    ArraySearch,
    /// Native C `ByteArray` cache.
    ByteArray,
    /// Native numeric cache, with subtype retained separately by its owner.
    Numeric,
    /// Native word-Boolean cache, distinct from numeric expression booleans.
    Boolean,
}

/// Current cache after an independently reached normal stock Length conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeStockListCacheDisposition {
    /// The selected native conversion commits an ordinary List cache.
    List,
    /// Preserve only the independently proved original cache summary.
    Preserved,
    /// Empty preservation or an unproved native class prevents a cache guarantee.
    Unknown,
}

/// Explicit logical object-length provider, independent of actual native proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalListLengthProvider {
    /// Explicit F5 logical list conversion using the C Tcl 8.4 recipe.
    Tcl84CoreSimulation,
}

/// Selected physical operation, before any string materialization or conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeObjectLengthAction {
    /// Read the existing ordinary list backing directly.
    CachedList,
    /// Return the proved native constant without changing either representation.
    Constant(usize),
    /// Run the selected ordinary list conversion, retaining guest parse errors.
    ConvertToList,
}

/// Actual native or explicitly authored logical stock object-length recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeObjectLengthProtocol {
    engine: NativeStringProtocol,
    logical: Option<LogicalListLengthProvider>,
}

impl NativeObjectLengthProtocol {
    /// Retained logical origin, which grants no actual native object proof.
    #[must_use]
    pub const fn logical_provider(self) -> Option<LogicalListLengthProvider> {
        self.logical
    }

    /// Choose the physical operation from the exact original stock class and
    /// authenticated canonical-empty storage identity. Empty bytes alone cannot
    /// establish that identity; allocated empty strings must pass false.
    #[must_use]
    pub fn action(
        self,
        class: NativeStockListInputClass,
        canonical_empty: bool,
    ) -> Option<NativeObjectLengthAction> {
        use NativeObjectLengthAction as Action;
        use NativeStockListInputClass as Class;
        let version = self.engine.tcl_version();
        if version.is_some_and(|version| version >= TclVersion::V9_0) && canonical_empty {
            return Some(Action::Constant(0));
        }
        if class == Class::List {
            return Some(Action::CachedList);
        }
        if version.is_some_and(|version| version >= TclVersion::V8_5) && canonical_empty {
            return Some(Action::Constant(0));
        }
        if (matches!(class, Class::MethodName | Class::InstructionName)
            && version.is_none_or(|version| version < TclVersion::V8_6))
            || (class == Class::PropertyName && version != Some(TclVersion::V9_1))
            || class == Class::Unknown
            || (class == Class::JimLookup && !self.engine.is_jim084())
            || (self.engine.is_jim084()
                && matches!(class, Class::ParsedVariableName | Class::LocalVariableName))
            || (version == Some(TclVersion::V8_4) && class == Class::Dictionary)
            || (self.engine.is_jim084() && class == Class::ByteArray)
        {
            return None;
        }
        if version.is_some_and(|version| version >= TclVersion::V9_0)
            && matches!(class, Class::Numeric | Class::Boolean)
        {
            return Some(Action::Constant(1));
        }
        Some(Action::ConvertToList)
    }
}

impl crate::InvocationDialect {
    /// Authenticate actual native object-length behavior independently of scalar parsing.
    #[must_use]
    pub fn native_object_length_protocol(self) -> Option<NativeObjectLengthProtocol> {
        Some(NativeObjectLengthProtocol {
            engine: self.native_string_protocol()?,
            logical: None,
        })
    }

    /// Request a separately authored logical recipe when no actual native issuer exists.
    #[must_use]
    pub fn object_length_protocol(
        self,
        provider: Option<LogicalListLengthProvider>,
    ) -> Option<NativeObjectLengthProtocol> {
        if let Some(protocol) = self.native_object_length_protocol() {
            return Some(protocol);
        }
        let provider = provider?;
        self.authored_f5_tcl84_core()?;
        Some(NativeObjectLengthProtocol {
            engine: NativeStringProtocol::for_tcl_version(TclVersion::V8_4),
            logical: Some(provider),
        })
    }
}

/// Selected Length protocol, conditional on closed original stock hooks/read.
/// It grants no actual read, success, object identity, bytes or effect closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeStockListLengthProtocol {
    argument: usize,
    engine: NativeObjectLengthProtocol,
}

impl NativeStockListLengthProtocol {
    /// Absolute original post-head operand position.
    #[must_use]
    pub const fn argument(self) -> usize {
        self.argument
    }

    /// Bound completion after independently sealed conversion hooks. Observers
    /// and arbitrary original objects cannot borrow this OK/Error envelope.
    #[must_use]
    pub const fn completion(self) -> crate::completion_route::InvocationCompletionRoute {
        use crate::completion::CompletionCode::{Error, Ok};
        crate::completion_route::InvocationCompletionRoute::TclAlternatives(&[Ok, Error])
    }

    /// Native normal cache projection from an independently proved original
    /// class and original materialized bytes. Empty bytes alone do not prove
    /// the canonical resident empty-string shortcut; unknown classes on Tcl 9
    /// may use numeric Length methods and cannot become List from nonempty text.
    #[must_use]
    pub fn normal_cache_disposition(
        self,
        class: NativeStockListInputClass,
        original_materialized_bytes: Option<&[u8]>,
    ) -> Option<NativeStockListCacheDisposition> {
        use NativeStockListCacheDisposition as Cache;
        use NativeStockListInputClass as Class;
        if self.engine.engine.is_jim084() {
            return (class != Class::ByteArray).then_some(Cache::List);
        }
        let version = self.engine.engine.tcl_version()?;
        if version == TclVersion::V8_4 {
            return (class != Class::Dictionary).then_some(Cache::List);
        }
        if class == Class::List
            || (version >= TclVersion::V9_0 && matches!(class, Class::Numeric | Class::Boolean))
        {
            return Some(Cache::Preserved);
        }
        // An authenticated stock numeric cache has a nonempty original numeric
        // string or a stock updater that produces one. It cannot take the
        // canonical resident empty-string shortcut in Tcl 8.5/8.6.
        if version < TclVersion::V9_0 && class == Class::Numeric {
            return Some(Cache::List);
        }
        if version >= TclVersion::V9_0 && class == Class::Unknown {
            return Some(Cache::Unknown);
        }
        Some(
            if original_materialized_bytes.is_some_and(|bytes| !bytes.is_empty()) {
                Cache::List
            } else {
                Cache::Unknown
            },
        )
    }
}

impl InvocationFacts {
    /// Authored Length handler only; stock object/class and normal continuation
    /// must be proved at ingress. Generic representation metadata grants nothing.
    #[must_use]
    pub fn stock_list_length_protocol(
        &self,
        arguments: InvocationArguments<'_>,
    ) -> Option<NativeStockListLengthProtocol> {
        if self.operation != SemanticOperationId::Intrinsic(IntrinsicId::ListLength)
            || self.native_result != Some(NativeResultContract::ListLength)
            || self.successful_handler != Some(SuccessfulHandlerSpec::Leaf)
            || self.arity_accepts_frozen_arguments() != Some(true)
            || self.frozen_argument_count != arguments.exact_argv_len()
            || self.argument_offset.checked_add(1) != arguments.exact_argv_len()
        {
            return None;
        }
        Some(NativeStockListLengthProtocol {
            argument: self.argument_offset,
            engine: arguments.dialect()?.native_object_length_protocol()?,
        })
    }

    /// Root-method effects of the native zero-argument generic list result.
    /// Compiled pooled objects decline: prior cache/class history is independent.
    /// This grants neither List representation nor allocation freshness.
    #[must_use]
    pub fn normal_empty_list_root_provider(
        &self,
        arguments: InvocationArguments<'_>,
        selection: NativeCompilationSelection,
    ) -> Option<NativeListMethodProvider> {
        if selection != NativeCompilationSelection::Generic
            || self.native_result != Some(NativeResultContract::ListArguments { from: 0 })
            || self.successful_handler != Some(SuccessfulHandlerSpec::Leaf)
            || self.arity_accepts_frozen_arguments() != Some(true)
            || self.frozen_argument_count != arguments.exact_argv_len()
            || arguments.exact_argv_len() != Some(self.argument_offset)
        {
            return None;
        }
        arguments.dialect()?.native_object_length_protocol()?;
        Some(NativeListMethodProvider::EmptyListRoot)
    }
}

#[cfg(test)]
mod tests;
