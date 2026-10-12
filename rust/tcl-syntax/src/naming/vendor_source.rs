// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Vendor source advice and bounded appliance names keep different issuers.

use super::{
    MeasuredBigIpNameScope, NameProjectionUnavailable, NativeVariableInputForm,
    ObservedBigIpNamePolicy, ObservedVariableNamePurpose,
};
use tcl_dialect::model::bigip_execution_context::BigIpExecutionContext;

/// Authority for vendor naming, without a native C/Jim implementation recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VendorSourceNameAuthority {
    /// Explicit authoring context; neither a build nor a running appliance.
    AuthoredContext,
    /// Exact appliance build/event of the independently selected observation.
    Observed(MeasuredBigIpNameScope),
}

/// Independently selected F5 hosted context and source-name authority.
/// The context does not select another context's string producer, table, frame,
/// command implementation, compiler recipe or variable lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VendorSourceNamePolicy {
    context: BigIpExecutionContext,
    authority: VendorSourceNameAuthority,
}

impl VendorSourceNamePolicy {
    /// Source assistance for an explicitly selected Tcl hosted context.
    /// APL and its unmeasured callbacks are not assumed to be implementation Tcl;
    /// the host shell remains an independent C provider.
    #[must_use]
    pub fn authored(context: BigIpExecutionContext) -> Option<Self> {
        (context.is_tcl() && context.is_appliance_hosted() && context.measurement().is_measured())
            .then_some(Self {
                context,
                authority: VendorSourceNameAuthority::AuthoredContext,
            })
    }

    /// Retain the exact measured TMM `HTTP_REQUEST` name issuer. No floating
    /// profile or other hosted context can construct this observation instead.
    #[must_use]
    pub const fn observed(policy: ObservedBigIpNamePolicy) -> Self {
        Self {
            context: BigIpExecutionContext::TmmIRule,
            authority: VendorSourceNameAuthority::Observed(policy.scope()),
        }
    }

    /// Actual selected hosted context, independently of grammar compatibility.
    #[must_use]
    pub const fn context(self) -> BigIpExecutionContext {
        self.context
    }

    /// Authoring assistance or an independently selected exact observation.
    #[must_use]
    pub const fn authority(self) -> VendorSourceNameAuthority {
        self.authority
    }

    /// Counted dynamic variable address within the actual observed grammar.
    /// Caller-owned produced bytes remain independent of this name projection.
    ///
    /// # Errors
    /// Authoring-only, other contexts and unmeasured operand purposes abstain.
    pub fn observed_variable_input(
        self,
        input: NativeVariableInputForm<'_>,
        purpose: ObservedVariableNamePurpose,
    ) -> Result<super::ExecutionVariableNameProjection<'_>, NameProjectionUnavailable> {
        let VendorSourceNameAuthority::Observed(scope) = self.authority else {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        };
        ObservedBigIpNamePolicy::for_measured_scope(scope).variable_input(input, purpose)
    }
}

/// Source naming question; a header is not a variable receiver or live slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VendorSourceNamePurpose {
    /// Literal command head advice, without lookup or command presence.
    CommandHead,
    /// Original procedure publication operand, without installation authority.
    ProcedureName,
    /// Original namespace operand, without namespace existence or incarnation.
    NamespaceName,
    /// Original package-name operand, without availability or loader identity.
    PackageName,
    /// Original formal-list value, without binder or activation installation.
    FormalList,
    /// Original script/source operand, without execution, loader or file identity.
    SourceName,
    /// Original filename-only navigation source units, without filesystem
    /// encoding, file availability or native source-handler authority.
    SourcePath,
    /// Original variable-name operand, without selected table or alias authority.
    VariableOperand,
    /// Exact lexical variable root; a separate index stays a different input.
    VariableRoot,
}

/// Plain ASCII source-literal units shared by the recorded hosted Tcl grammars.
/// This deliberately supplies no Unicode, numeric escape, NUL, backslash,
/// computed string or materialised-object producer recipe. Those sources remain
/// owned, but their bytes are unavailable at this narrow projection.
#[must_use]
pub fn vendor_source_literal_units(
    policy: VendorSourceNamePolicy,
    word: &tcl_lexer::NativeWord,
    _purpose: VendorSourceNamePurpose,
) -> Option<&[u8]> {
    VendorSourceNamePolicy::authored(policy.context())?;
    if word.group().expand {
        return None;
    }
    if word
        .executable_parts()
        .list(word.executable_parts().root())
        .iter()
        .any(|component| !matches!(component.part, tcl_lexer::ExecutablePart::Text(_)))
    {
        return None;
    }
    let content = word.content_span().ok()?;
    let bytes = word.image().bytes().get(content.as_range())?;
    (bytes.is_ascii() && !bytes.contains(&0) && !bytes.contains(&b'\\')).then_some(bytes)
}

/// A formal's original source extent within its complete parameter-list word.
/// This is list geometry for source advice, without a native binding recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorSourceFormalExtent {
    /// Actual outer parameter-list ordinal, including duplicate names.
    pub ordinal: usize,
    /// Name units relative to the original word's content.
    pub name: std::ops::Range<usize>,
    /// The original specifier has a second default-value field.
    pub has_default: bool,
    /// Final original name is the source grammar's `args` spelling.
    pub is_rest: bool,
}

/// Locate supported literal formal names through the actual source list
/// grammar. Unsupported units or invalid topology withdraw the whole vector;
/// consumers still retain the original parameter-list producer separately.
#[must_use]
pub fn vendor_source_formal_extents(
    policy: VendorSourceNamePolicy,
    word: &tcl_lexer::NativeWord,
) -> Option<Vec<VendorSourceFormalExtent>> {
    let units = vendor_source_literal_units(policy, word, VendorSourceNamePurpose::FormalList)?;
    let config = word.config();
    let parsed = crate::formal_params::parse_formal_parameters_in(
        std::str::from_utf8(units).ok()?,
        tcl_dialect::ParameterGrammar::Tcl,
    )
    .ok()?;
    let mut result = Vec::with_capacity(parsed.len());
    let mut offset = 0;
    while let Some(specifier) =
        crate::list::find_element_bytes_with_syntax(units, offset, config.list_parse).ok()?
    {
        if !specifier.literal {
            return None;
        }
        let fields = units.get(specifier.value.clone())?;
        let name =
            crate::list::find_element_bytes_with_syntax(fields, 0, config.list_parse).ok()??;
        if !name.literal {
            return None;
        }
        let ordinal = result.len();
        let parameter = parsed.get(ordinal)?;
        let start = specifier.value.start.checked_add(name.value.start)?;
        let end = specifier.value.start.checked_add(name.value.end)?;
        if units.get(start..end)? != parameter.name.as_bytes() {
            return None;
        }
        result.push(VendorSourceFormalExtent {
            ordinal,
            name: start..end,
            has_default: parameter.default.is_some(),
            is_rest: ordinal + 1 == parsed.len() && parameter.name == "args",
        });
        offset = specifier.next;
    }
    (result.len() == parsed.len()).then_some(result)
}

/// Render a supported unqualified scalar source reference and check it with
/// the selected hosted source grammar. This supplies no runtime string,
/// variable availability, receiver completion or edit authority.
#[must_use]
pub fn vendor_scalar_source_reference(
    policy: VendorSourceNamePolicy,
    units: &[u8],
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    VendorSourceNamePolicy::authored(policy.context())?;
    if units.is_empty()
        || !units.is_ascii()
        || units.contains(&0)
        || units.contains(&b'\\')
        || units.contains(&b'(')
        || units.contains(&b')')
        || units.windows(2).any(|pair| pair == b"::")
    {
        return None;
    }
    let name = std::str::from_utf8(units).ok()?;
    let source = format!("${{{name}}}");
    let reference = tcl_lexer::word_parts::scan_var_ref(source.as_bytes(), 0, config).ok()??;
    (reference.next == source.len() && reference.index.is_none() && reference.name == units)
        .then_some(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Implementation contract: naming.vendor.original-source-context-input
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
    fn vendor_source_context_and_observed_variable_purpose_are_independent() {
        for context in [
            BigIpExecutionContext::TmmIRule,
            BigIpExecutionContext::TmshCliScript,
            BigIpExecutionContext::IAppImplementation,
            BigIpExecutionContext::ICallScript,
        ] {
            let authored = VendorSourceNamePolicy::authored(context).unwrap();
            assert_eq!(authored.context(), context);
            assert_eq!(
                authored.authority(),
                VendorSourceNameAuthority::AuthoredContext
            );
            assert!(
                authored
                    .observed_variable_input(
                        NativeVariableInputForm::Combined(b"x\0y"),
                        ObservedVariableNamePurpose::ScalarReceiver
                    )
                    .is_err()
            );
        }
        for context in [
            BigIpExecutionContext::IAppPresentationApl,
            BigIpExecutionContext::IAppPresentationTclCallback,
            BigIpExecutionContext::HostShellTcl,
        ] {
            assert!(VendorSourceNamePolicy::authored(context).is_none());
        }
        let observed =
            VendorSourceNamePolicy::observed(ObservedBigIpNamePolicy::for_measured_scope(
                MeasuredBigIpNameScope::BigIp21_1_0_1Build0_0_26TmmHttpRequest,
            ));
        let projected = observed
            .observed_variable_input(
                NativeVariableInputForm::Combined(b"x\0y(k)"),
                ObservedVariableNamePurpose::CombinedElement,
            )
            .unwrap();
        assert_eq!(projected.root(), b"x\0y");
        assert_eq!(projected.element(), Some(b"k".as_slice()));
        assert!(projected.native_projection().is_none());
        assert!(
            observed
                .observed_variable_input(
                    NativeVariableInputForm::Combined(b"static::x"),
                    ObservedVariableNamePurpose::ScalarReceiver
                )
                .is_err()
        );
    }
}
