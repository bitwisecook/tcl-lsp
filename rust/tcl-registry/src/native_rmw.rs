// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native captured read/modify/write behaviour, including missing contents.

use crate::InvocationDialect;

/// Native operation whose captured read protocol has been measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeRmwOperation {
    /// Integer increment of the originally selected variable cell.
    Increment,
}

/// Missing contents and owner retirement at the captured read boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeRmwReadPolicy {
    /// C Tcl 8.4 requires contents, including an initially missing variable.
    RequireContents,
    /// C Tcl 8.5+ and Jim 0.84 start missing contents at zero.
    InitialiseZero,
}

/// Native amount conversion relative to variable lookup and read callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeRmwAmountValidation {
    /// Validate the increment before resolving or reading its receiver.
    BeforeRead,
    /// Resolve and fetch the receiver before validating the increment.
    AfterRead,
}

/// Independently selected actual legacy increment handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeLegacyIncrementProtocol {
    recipe: tcl_syntax::scalar_getter::NativeLegacyIncrementRecipe,
}
impl NativeLegacyIncrementProtocol {
    /// Pure transaction recipe; object conversion authority stays with the adapter.
    #[must_use]
    pub const fn recipe(self) -> tcl_syntax::scalar_getter::NativeLegacyIncrementRecipe {
        self.recipe
    }
}
impl InvocationDialect {
    /// Select the actual C8.4 or pinned Jim handler, independently of source grammar.
    #[must_use]
    pub fn native_legacy_increment_protocol(self) -> Option<NativeLegacyIncrementProtocol> {
        use tcl_syntax::scalar_getter::NativeLegacyIncrementRecipe as Recipe;
        let getter = self.native_scalar_getter_protocol()?;
        let recipe = if getter.tcl_version() == Some(tcl_dialect::TclVersion::V8_4) {
            Recipe::Tcl84
        } else if getter.is_jim084() {
            Recipe::Jim084
        } else {
            return None;
        };
        Some(NativeLegacyIncrementProtocol { recipe })
    }
}

/// Original-object conversions of the selected native Increment handler.
/// Captured old contents and frozen amount objects are separate obligations;
/// this receipt supplies no acceptance, effects, value or result cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeIncrementObjectProtocol {
    amount_argument: Option<usize>,
    amount_validation: NativeRmwAmountValidation,
    read_policy: NativeRmwReadPolicy,
    engine: tcl_syntax::scalar_getter::NativeScalarGetterProtocol,
}

impl NativeIncrementObjectProtocol {
    /// Original amount operand, absent for the native implicit increment.
    #[must_use]
    pub const fn amount_argument(self) -> Option<usize> {
        self.amount_argument
    }

    /// Ordered amount conversion relative to lookup and read callbacks.
    #[must_use]
    pub const fn amount_validation(self) -> NativeRmwAmountValidation {
        self.amount_validation
    }

    /// Whether a failed/missing read admits native zero initialization.
    #[must_use]
    pub const fn read_policy(self) -> NativeRmwReadPolicy {
        self.read_policy
    }

    /// Whether the selected C compiler embeds this small literal amount in
    /// an immediate opcode. Its compile-time temporary String is independent
    /// of runtime operand objects; generic and substituted values decline.
    #[must_use]
    pub fn embeds_amount(
        self,
        arguments: crate::InvocationArguments<'_>,
        selection: crate::native_compilation::NativeCompilationSelection,
        original_shape: crate::native_compilation::NativeCompilationWordShape,
    ) -> bool {
        use crate::native_compilation::{
            NativeCompilationSelection as Selection, NativeCompilationWordShape as Shape,
        };
        if !matches!(
            selection,
            Selection::Inline {
                operation: crate::SemanticOperationId::StructuredLowering(
                    crate::hooks::LoweringHookId::Incr
                ),
                ..
            }
        ) || !matches!(
            original_shape,
            Shape::Literal | Shape::QuotedLiteral | Shape::BracedLiteral
        ) {
            return false;
        }
        let Some(dialect) = arguments.dialect() else {
            return false;
        };
        if dialect.native_scalar_getter_protocol() != Some(self.engine) {
            return false;
        }
        let Some(version) = self.engine.tcl_version() else {
            return false;
        };
        let Some(amount) = self
            .amount_argument
            .and_then(|argument| arguments.literal_at(argument))
        else {
            return false;
        };
        matches!(
            tcl_syntax::number::parse_whole_with(
                amount,
                tcl_syntax::number::ParseFlags::for_syntax(version.number_syntax()),
            ),
            Some(tcl_syntax::number::Number::Int(value)) if (-127..=127).contains(&value)
        )
    }
}

impl crate::InvocationFacts {
    /// Authored Increment object-hook obligations at the actual native axis.
    /// The caller must retain each original object at its reached conversion
    /// phase, after any selected read callbacks. No variable-name lookup or
    /// normal numeric result can close these original-input obligations.
    #[must_use]
    pub fn increment_object_protocol(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<NativeIncrementObjectProtocol> {
        if self.native_result != Some(crate::native_result::NativeResultContract::IncrementStore)
            || self.operation
                != crate::SemanticOperationId::StructuredLowering(
                    crate::hooks::LoweringHookId::Incr,
                )
            || self.arity_accepts_frozen_arguments() != Some(true)
            || self.frozen_argument_count != arguments.exact_argv_len()
        {
            return None;
        }
        let count = arguments
            .exact_argv_len()?
            .checked_sub(self.argument_offset)?;
        if !matches!(count, 1 | 2) {
            return None;
        }
        let dialect = arguments.dialect()?;
        // Validate only the actual native engine axes; no primitive getter
        // conversion, acceptance or cache receipt is borrowed here.
        let engine = dialect.native_scalar_getter_protocol()?;
        Some(NativeIncrementObjectProtocol {
            amount_argument: (count == 2).then(|| self.argument_offset + 1),
            amount_validation: dialect
                .native_rmw_amount_validation(NativeRmwOperation::Increment)?,
            read_policy: dialect.native_rmw_read_policy(NativeRmwOperation::Increment)?,
            engine,
        })
    }
}

/// Numeric versus safe-expression failure supplied by the portable value owner.
pub use tcl_syntax::value::IntegerOperandError as NativeRmwAmountError;
/// Selected increment amount grammar supplied by the portable value owner.
pub use tcl_syntax::value::IntegerOperandGrammar as NativeRmwAmountGrammar;

/// Exact native error-code components for a measured receiver failure.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeRmwFailureErrorCode {
    /// Legacy native errors use the single NONE component.
    None,
    /// Lookup error with the original selected variable-name component.
    LookupVariable(Vec<u8>),
    /// A captured physical receiver failed at the write boundary.
    WriteVariable,
}

/// A captured read could not produce contents or its owner was retired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeRmwReadFailure {
    /// An existing protected scalar shell has no contents.
    MissingVariable,
    /// An existing protected array element has no contents.
    MissingElement,
    /// The originally selected array was destroyed.
    RetiredArray,
    /// The originally selected namespace was destroyed.
    RetiredNamespace,
    /// A scalar read selected an array-valued root.
    ArrayValue,
    /// An element read selected a scalar-valued root.
    NonArrayElement,
    /// The selected qualified variable has no parent namespace.
    MissingNamespace,
}

impl InvocationDialect {
    /// Selected amount grammar, independent of current variable contents.
    #[must_use]
    pub fn native_rmw_amount_grammar(
        self,
        operation: NativeRmwOperation,
    ) -> Option<NativeRmwAmountGrammar> {
        match operation {
            NativeRmwOperation::Increment => self
                .tcl_version
                .map(|_| NativeRmwAmountGrammar::Integer)
                .or_else(|| {
                    self.core_point
                        .is_some_and(|point| {
                            point.release() == tcl_dialect::model::Release::JIM_0_84
                        })
                        .then_some(NativeRmwAmountGrammar::SafeIntegerExpression)
                }),
        }
    }

    /// Amount-validation timing measured independently of missing contents.
    #[must_use]
    pub fn native_rmw_amount_validation(
        self,
        operation: NativeRmwOperation,
    ) -> Option<NativeRmwAmountValidation> {
        match operation {
            NativeRmwOperation::Increment => self
                .tcl_version
                .map(|version| {
                    if version == tcl_dialect::TclVersion::V8_4 {
                        NativeRmwAmountValidation::BeforeRead
                    } else {
                        NativeRmwAmountValidation::AfterRead
                    }
                })
                .or_else(|| {
                    self.core_point
                        .is_some_and(|point| {
                            point.release() == tcl_dialect::model::Release::JIM_0_84
                        })
                        .then_some(NativeRmwAmountValidation::BeforeRead)
                }),
        }
    }

    /// Exact failure code, retaining the original name and already parsed root
    /// separately. Unmeasured failure presentations remain absent.
    #[must_use]
    pub fn native_rmw_failure_error_code(
        self,
        name: &[u8],
        root: &[u8],
        failure: NativeRmwReadFailure,
    ) -> Option<NativeRmwFailureErrorCode> {
        use NativeRmwFailureErrorCode as Code;
        use NativeRmwReadFailure as Failure;
        let version = self.tcl_version?;
        if version < tcl_dialect::TclVersion::V8_6 {
            return Some(Code::None);
        }
        match failure {
            Failure::NonArrayElement => Some(Code::LookupVariable(root.to_vec())),
            Failure::MissingNamespace => Some(Code::LookupVariable(name.to_vec())),
            Failure::ArrayValue | Failure::RetiredArray | Failure::RetiredNamespace => {
                Some(Code::WriteVariable)
            }
            Failure::MissingVariable | Failure::MissingElement => None,
        }
    }

    /// Selected native read protocol, independent of command availability.
    /// Jim 0.84 uses zero for missing contents but has no variable-trace callback protocol.
    #[must_use]
    pub fn native_rmw_read_policy(
        self,
        operation: NativeRmwOperation,
    ) -> Option<NativeRmwReadPolicy> {
        match operation {
            NativeRmwOperation::Increment => self
                .tcl_version
                .map(|version| {
                    if version < tcl_dialect::TclVersion::V8_5 {
                        NativeRmwReadPolicy::RequireContents
                    } else {
                        NativeRmwReadPolicy::InitialiseZero
                    }
                })
                .or_else(|| {
                    self.core_point
                        .is_some_and(|point| {
                            point.release() == tcl_dialect::model::Release::JIM_0_84
                        })
                        .then_some(NativeRmwReadPolicy::InitialiseZero)
                }),
        }
    }
}

impl NativeRmwReadPolicy {
    /// Native message for a proved captured-cell failure; no unknown case is presented.
    #[must_use]
    pub fn failure_message(self, name: &str, failure: NativeRmwReadFailure) -> String {
        String::from_utf8(self.failure_message_bytes(name.as_bytes(), failure))
            .expect("UTF-8 name and native message fragments")
    }

    /// Native byte message, preserving names supplied through a byte-valued ABI.
    #[must_use]
    pub fn failure_message_bytes(self, name: &[u8], failure: NativeRmwReadFailure) -> Vec<u8> {
        use NativeRmwReadFailure as Failure;
        let (verb, reason) = match (self, failure) {
            (Self::InitialiseZero, Failure::RetiredArray) => {
                ("set", "upvar refers to element in deleted array")
            }
            (Self::InitialiseZero, Failure::RetiredNamespace) => {
                ("set", "upvar refers to variable in deleted namespace")
            }
            (Self::InitialiseZero, Failure::ArrayValue) => ("set", "variable is array"),
            (_, Failure::ArrayValue) => ("read", "variable is array"),
            (_, Failure::NonArrayElement) => ("read", "variable isn't array"),
            (Self::InitialiseZero, Failure::MissingNamespace) => {
                ("read", "parent namespace doesn't exist")
            }
            (_, Failure::MissingElement | Failure::RetiredArray) => {
                ("read", "no such element in array")
            }
            (
                _,
                Failure::MissingVariable | Failure::RetiredNamespace | Failure::MissingNamespace,
            ) => ("read", "no such variable"),
        };
        let mut message = format!("can't {verb} \"").into_bytes();
        message.extend_from_slice(name);
        message.extend_from_slice(b"\": ");
        message.extend_from_slice(reason.as_bytes());
        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn increment_object_hooks_retain_actual_conversion_schedule() {
        use crate::native_compilation::{
            NativeCompilationGuard, NativeCompilationSelection as Selection,
            NativeCompilationWordShape as Shape,
        };
        use crate::{InvocationArguments, InvocationWord, InvocationWords};
        let operands = [InvocationWord::Literal("input"), InvocationWord::Dynamic];
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let context = crate::model::ingress::static_context_for(environment);
            let dialect = InvocationDialect::of_profile(context.commands().profile().unwrap());
            let words = InvocationWords::structured(InvocationWord::Literal("incr"), &operands)
                .with_dialect(dialect);
            let facts = context
                .commands()
                .resolve_structured_invocation(words, dialect.authoring_query())
                .resolved()
                .unwrap()
                .facts();
            let protocol = facts.increment_object_protocol(words.arguments()).unwrap();
            let mut conflict = dialect;
            conflict.core_point = Some(if environment == "jim" {
                tcl_dialect::model::DialectPoint::for_tcl_version(tcl_dialect::TclVersion::V9_1)
            } else {
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84)
            });
            assert!(
                facts
                    .increment_object_protocol(
                        InvocationArguments::structured(&operands).with_dialect(conflict),
                    )
                    .is_none()
            );
            assert_eq!(protocol.amount_argument(), Some(1));
            let immediate = [
                InvocationWord::Literal("input"),
                InvocationWord::Literal("1"),
            ];
            let immediate = InvocationArguments::structured(&immediate).with_dialect(dialect);
            let inline = Selection::Inline {
                operation: facts.operation,
                guard: if environment == "tcl8.4" {
                    NativeCompilationGuard::ChunkEntry
                } else {
                    NativeCompilationGuard::BeforeArguments
                },
            };
            assert_eq!(
                protocol.embeds_amount(immediate, inline, Shape::Literal),
                environment != "jim"
            );
            assert!(!protocol.embeds_amount(immediate, Selection::Generic, Shape::Literal));
            assert!(!protocol.embeds_amount(immediate, inline, Shape::Substituted));
            assert!(!protocol.embeds_amount(
                immediate,
                Selection::Inline {
                    operation: crate::SemanticOperationId::Invoke,
                    guard: NativeCompilationGuard::BeforeArguments,
                },
                Shape::Literal,
            ));
            let large = [
                InvocationWord::Literal("input"),
                InvocationWord::Literal("128"),
            ];
            assert!(!protocol.embeds_amount(
                InvocationArguments::structured(&large).with_dialect(dialect),
                inline,
                Shape::Literal,
            ));
            assert_eq!(
                protocol.amount_validation(),
                if matches!(environment, "tcl8.4" | "jim") {
                    NativeRmwAmountValidation::BeforeRead
                } else {
                    NativeRmwAmountValidation::AfterRead
                }
            );
            assert!(
                facts
                    .increment_object_protocol(InvocationArguments::structured(&operands))
                    .is_none()
            );
            let mut unrelated = facts.clone();
            unrelated.native_result = None;
            assert!(
                unrelated
                    .increment_object_protocol(words.arguments())
                    .is_none()
            );
            let expanded = [InvocationWord::Expanded];
            assert!(
                facts
                    .increment_object_protocol(
                        InvocationArguments::structured(&expanded).with_dialect(dialect)
                    )
                    .is_none()
            );
        }
    }

    #[test]
    fn increment_missing_contents_policy_uses_actual_native_release() {
        use tcl_dialect::TclVersion;
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let actual = InvocationDialect::for_version(version)
                .native_rmw_read_policy(NativeRmwOperation::Increment);
            assert_eq!(
                actual,
                Some(if version == TclVersion::V8_4 {
                    NativeRmwReadPolicy::RequireContents
                } else {
                    NativeRmwReadPolicy::InitialiseZero
                })
            );
        }
        let mut unknown_release = InvocationDialect::for_version(TclVersion::V8_6);
        unknown_release.tcl_version = None;
        assert_eq!(
            unknown_release.native_rmw_read_policy(NativeRmwOperation::Increment),
            None
        );
        let jim = crate::model::ingress::resolve_environment("jim").unit_profile();
        assert_eq!(
            InvocationDialect::of_profile(jim)
                .native_rmw_read_policy(NativeRmwOperation::Increment),
            Some(NativeRmwReadPolicy::InitialiseZero)
        );
    }

    #[test]
    fn retired_receivers_have_native_stage_specific_messages() {
        use NativeRmwReadFailure::*;
        assert_eq!(
            NativeRmwReadPolicy::RequireContents.failure_message("a(k)", RetiredArray),
            "can't read \"a(k)\": no such element in array"
        );
        assert_eq!(
            NativeRmwReadPolicy::InitialiseZero.failure_message("a(k)", RetiredArray),
            "can't set \"a(k)\": upvar refers to element in deleted array"
        );
        assert_eq!(
            NativeRmwReadPolicy::InitialiseZero.failure_message("::N::a", RetiredNamespace),
            "can't set \"::N::a\": upvar refers to variable in deleted namespace"
        );
    }
}
