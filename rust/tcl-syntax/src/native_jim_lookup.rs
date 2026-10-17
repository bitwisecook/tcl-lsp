// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pinned Jim original command and variable cache validity, without lookup authority.

/// Exact current Jim cache rules. A pure recipe is not an engine issuer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeJimLookupProtocol(());

/// Jim origin canonicalisation selects an original holder or its constructor;
/// the bytes supply no command token or alias-prefix authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeJimNamespaceOriginCanonical {
    /// Canonical bytes, independent of a command identity.
    pub selected: tcl_core_types::NameBytes,
    /// Constructor role retaining the original operand or actual namespace.
    pub construction: crate::naming::NativeJimNamespaceConstruction,
}

/// Native link target construction, independent of variable key equality.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeJimLinkTargetInput<'a> {
    /// Relative targets retain the same original operand in the selected frame.
    Original,
    /// Absolute targets own a fresh counted stripped object in the top frame.
    StrippedGlobal(&'a [u8]),
}

/// Typed `Jim_SetVariableLink` rejection, before any alias publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeJimAliasFailure {
    /// A local dictionary-sugar operand cannot name a scalar alias.
    LocalElement,
    /// A current local value already occupies the original alias name.
    Exists,
    /// The local frame cannot retain a shorter-lived target frame.
    Inverted,
    /// The original target-name chain returns to the selected local slot.
    SelfLink,
}

/// Physical variable-table access selected by the original counted pattern.
/// This pure choice grants no table, retained key or frame ownership.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeJimVariableInventoryLookup<'a> {
    /// No pattern: append eligible original keys without reading their bytes.
    All,
    /// `JimTrivialMatch` classifies the `CString` prefix, but hash lookup uses the
    /// unchanged counted original pattern.
    Exact(&'a [u8]),
    /// Walk the real table and match each eligible original key object.
    Scan(&'a [u8]),
}

impl NativeJimLookupProtocol {
    /// Select the original variable-table fast path without canonicalising,
    /// stripping roots or manufacturing a lookup result.
    #[must_use]
    pub fn variable_inventory_lookup(
        self,
        pattern: Option<&[u8]>,
    ) -> NativeJimVariableInventoryLookup<'_> {
        match pattern {
            None => NativeJimVariableInventoryLookup::All,
            Some(pattern)
                if crate::glob::is_literal_bytes(tcl_core_types::c_string_extent(pattern)) =>
            {
                NativeJimVariableInventoryLookup::Exact(pattern)
            }
            Some(pattern) => NativeJimVariableInventoryLookup::Scan(pattern),
        }
    }
    /// Original Jim alias error formatter after its counted String getter. This has no C error tuple,
    /// installed alias, selected frame or variable-cache authority.
    #[must_use]
    pub fn alias_failure_message(self, failure: NativeJimAliasFailure, original: &[u8]) -> Vec<u8> {
        if failure == NativeJimAliasFailure::SelfLink {
            return b"can't upvar from variable to itself".to_vec();
        }
        let mut message = if failure == NativeJimAliasFailure::Exists {
            b"variable \"".to_vec()
        } else {
            b"bad variable name \"".to_vec()
        };
        message.extend_from_slice(tcl_core_types::c_string_extent(original));
        message.extend_from_slice(match failure {
            NativeJimAliasFailure::LocalElement => {
                b"\": upvar won't create a scalar variable that looks like an array element"
            }
            NativeJimAliasFailure::Exists => b"\" already exists",
            NativeJimAliasFailure::Inverted => {
                b"\": upvar won't create namespace variable that refers to procedure variable"
            }
            NativeJimAliasFailure::SelfLink => unreachable!("handled above"),
        });
        message
    }

    /// `Jim_CreateNamespaceVariable` diagnostic after its counted local String getter.
    /// This supplies no alias, variable cell or selected frame.
    #[must_use]
    pub fn namespace_variable_failure_message(self, original_local: &[u8]) -> Vec<u8> {
        let mut message = b"can't define \"".to_vec();
        message.extend_from_slice(tcl_core_types::c_string_extent(original_local));
        message.extend_from_slice(b"\": name refers to an element in an array");
        message
    }

    /// Counted `namespace current` result from the actual frame's original
    /// namespace object. Rooting a result does not strip an existing prefix.
    /// This projection supplies no namespace lookup, object or frame owner.
    #[must_use]
    pub fn namespace_current_result(self, original_namespace: &[u8]) -> Vec<u8> {
        // naming.lambda.original-namespace-constructor-and-getter
        // docs/design/analysis/name-resolution-proofs/lambda-original-namespace-constructor-and-getter.md
        let mut bytes = b"::".to_vec();
        bytes.extend_from_slice(original_namespace);
        bytes
    }

    /// Counted target authored by the namespace-upvar helper after its actual
    /// namespace canonicalisation. Target bytes are appended literally.
    /// This constructs an operand without selecting a variable or frame.
    #[must_use]
    pub fn namespace_upvar_target(
        self,
        canonical_namespace: &[u8],
        original_target: &[u8],
    ) -> Vec<u8> {
        let mut target = self.namespace_current_result(canonical_namespace);
        target.extend_from_slice(b"::");
        target.extend_from_slice(original_target);
        target
    }

    /// Canonicalise an origin operand in the actual caller namespace. The
    /// helper's alias and command getters independently execute in root.
    ///
    /// # Errors
    /// Refuses an absent actual namespace object.
    pub fn namespace_origin_canonical(
        self,
        context: crate::naming::NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<NativeJimNamespaceOriginCanonical, crate::naming::NameProjectionUnavailable> {
        let names = crate::naming::NativeNameProtocol::Jim084;
        let selected = names.jim_namespace_canonical_input(context, original)?;
        Ok(NativeJimNamespaceOriginCanonical {
            selected: tcl_core_types::NameBytes::from(selected.selected()),
            construction: names.jim_namespace_construction(context, original)?,
        })
    }

    /// Report the successful original operand with a counted root prefix.
    /// This does not recover its command identity or split an alias prefix.
    #[must_use]
    pub fn namespace_origin_result(self, selected: &[u8]) -> Vec<u8> {
        let mut result = b"::".to_vec();
        result.extend_from_slice(selected);
        result
    }

    /// The script helper reports the complete recursive name as counted bytes.
    #[must_use]
    pub fn namespace_origin_missing(self, original: &[u8]) -> Vec<u8> {
        let mut message = b"invalid command name \"".to_vec();
        message.extend_from_slice(original);
        message.push(b'"');
        message
    }

    /// Select `Jim_SetVariableLink`'s original-object owner and actual target frame.
    #[must_use]
    pub fn link_target_input(self, original: &[u8]) -> NativeJimLinkTargetInput<'_> {
        if original.starts_with(b"::") {
            let offset = original
                .iter()
                .position(|byte| *byte != b':')
                .unwrap_or(original.len());
            NativeJimLinkTargetInput::StrippedGlobal(&original[offset..])
        } else {
            NativeJimLinkTargetInput::Original
        }
    }

    /// `Jim_GetVariable` reports the original enclosing alias when its
    /// recursive target getter returns NULL, independently of the target's
    /// error text. The original alias is a simple cached name at this point.
    ///
    /// # Errors
    /// Returns the shared diagnostic refusal if this reporting purpose is unavailable.
    pub fn linked_variable_read_error(
        self,
        original: &[u8],
    ) -> Result<Vec<u8>, crate::naming::NameProjectionUnavailable> {
        use crate::naming::{
            NativeNameProtocol, NativeVariableDiagnosticOperation as Operation,
            NativeVariableDiagnosticReason as Reason, NativeVariableFailureSite as Site,
            NativeVariableInputForm as Input, report_native_variable_diagnostic_at,
        };
        let diagnostic = report_native_variable_diagnostic_at(
            NativeNameProtocol::Jim084,
            Operation::Read,
            Reason::NoSuchVariable,
            Site::ValueRead,
            Input::Separate {
                root: original,
                element: None,
            },
        )?;
        let mut message = b"can't read \"".to_vec();
        message.extend_from_slice(&diagnostic.name);
        message.extend_from_slice(b"\": ");
        message.extend_from_slice(diagnostic.reason.message().as_bytes());
        Ok(message)
    }

    /// `JimDictSugarSet` reports the original indexed operand after a failed
    /// removal. Its final `JIM_NONE` parent getter, rather than the failed
    /// dictionary conversion, distinguishes an absent parent from a missing
    /// member of any existing parent (including a malformed dictionary).
    ///
    /// # Errors
    /// Preserves an unavailable original diagnostic projection.
    pub fn dictionary_unset_error(
        self,
        original: &[u8],
        parent_exists: bool,
    ) -> Result<Vec<u8>, crate::naming::NameProjectionUnavailable> {
        use crate::naming::{
            NativeNameProtocol, NativeVariableDiagnosticOperation as Operation,
            NativeVariableDiagnosticReason as Reason, NativeVariableFailureSite as Site,
            NativeVariableInputForm as Input, report_native_variable_diagnostic_at,
        };
        let diagnostic = report_native_variable_diagnostic_at(
            NativeNameProtocol::Jim084,
            Operation::Unset,
            // Obtain this purpose's original combined operand extent. The
            // generic NotArray reporter describes an existing malformed root;
            // JimDictSugarSet's final getter supplies a separate existence fact.
            Reason::NoSuchElement,
            Site::ValueUnset,
            Input::Combined(original),
        )?;
        let mut message = b"can't unset \"".to_vec();
        message.extend_from_slice(&diagnostic.name);
        message.extend_from_slice(b"\": ");
        let reason = if parent_exists {
            Reason::NoSuchElement
        } else {
            Reason::NotArray
        };
        message.extend_from_slice(reason.message().as_bytes());
        Ok(message)
    }

    /// Pure pinned recipe, independently of any live interpreter.
    #[must_use]
    pub const fn jim084() -> Self {
        Self(())
    }

    /// `Jim_GetCommand` checks epoch before touching its cached command pointer.
    /// Namespace equality is counted original-object string equality, not a
    /// rendered C namespace path. `live` comes from the actual node's inUse.
    #[must_use]
    pub const fn command_is_current(
        self,
        cached_epoch: u64,
        current_epoch: u64,
        same_namespace: bool,
        live: bool,
    ) -> bool {
        cached_epoch == current_epoch && same_namespace && live
    }

    /// Variable resolution selects the top frame for a cached global name,
    /// otherwise the current frame, and compares its actual unique ID.
    #[must_use]
    pub const fn variable_is_current(self, cached_frame: u64, selected_frame: u64) -> bool {
        cached_frame == selected_frame
    }

    /// `JimUpdateProcNamespace` selects the final counted namespace delimiter.
    /// A bare name has no namespace-shadow invalidation obligation.
    #[must_use]
    pub fn procedure_shadow_tail(self, original: &[u8]) -> Option<&[u8]> {
        let name = if original.starts_with(b"::") {
            &original[original
                .iter()
                .position(|byte| *byte != b':')
                .unwrap_or(original.len())..]
        } else {
            original
        };
        let last = name.iter().rposition(|byte| *byte == b':')?;
        (last > 0 && name[last - 1] == b':').then(|| &name[last + 1..])
    }

    /// An ordinary replacement invalidates the old node by inUse, not epoch.
    #[must_use]
    pub const fn ordinary_replacement_changes_epoch(self) -> bool {
        false
    }

    /// Native failed lookup leaves the original primary representation intact.
    #[must_use]
    pub const fn miss_preserves_primary(self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_unset_uses_the_final_parent_getter_category() {
        let protocol = NativeJimLookupProtocol::jim084();
        assert_eq!(
            protocol.dictionary_unset_error(b"a(FIRST)", false).unwrap(),
            b"can't unset \"a(FIRST)\": variable isn't array"
        );
        assert_eq!(
            protocol.dictionary_unset_error(b"a(FIRST)", true).unwrap(),
            b"can't unset \"a(FIRST)\": no such element in array"
        );
    }

    // Native proof: naming.jim.original-lookup-currency
    // docs/design/analysis/name-resolution-proofs/jim-original-lookup-currency.md
    #[test]
    fn original_native_lookup_outcomes_match_retained_cache_guards() {
        let protocol = NativeJimLookupProtocol::jim084();
        let fixture = include_str!("../tests/data/native_jim_lookup/observations.tsv");
        let mut checked = 0;
        for line in fixture.lines() {
            let fields: Vec<_> = line.split('\t').collect();
            let label = fields[0];
            let get = |key: &str| {
                fields
                    .iter()
                    .find_map(|field| field.strip_prefix(key))
                    .unwrap()
            };
            let number = |key: &str| get(key).parse::<u64>().unwrap();
            if matches!(
                label,
                "cmd-first"
                    | "cmd-replace-lookup"
                    | "cmd-rename-old-miss"
                    | "cmd-retired-miss"
                    | "cmd-deleted-active-original-hit"
                    | "cmd-after-active-retirement"
            ) {
                let live = get("cmdlive=") == "1";
                let hit =
                    protocol.command_is_current(number("cacheproc="), number("proc="), true, live);
                assert_eq!(hit, get("ok=") == "1", "{label}");
                checked += 1;
            }
            if matches!(
                label,
                "var-read"
                    | "var-duplicate"
                    | "var-local-frame-miss"
                    | "var-root-again"
                    | "var-unrelated-unset-lookup"
                    | "var-unset-miss"
                    | "var-absolute-read"
                    | "var-counted-nul-read"
                    | "var-opaque-read"
            ) {
                let hit = protocol.variable_is_current(number("cacheframe="), number("frame="));
                assert_eq!(hit, get("ok=") == "1", "{label}");
                checked += 1;
            }
        }
        assert_eq!(checked, 15);
    }

    #[test]
    fn epoch_and_scope_invalidation_match_original_native_controls() {
        let protocol = NativeJimLookupProtocol::jim084();
        assert!(protocol.command_is_current(1, 1, true, true));
        assert!(!protocol.command_is_current(1, 1, true, false));
        assert!(!protocol.command_is_current(1, 2, true, true));
        assert!(!protocol.command_is_current(2, 2, false, true));
        assert!(protocol.variable_is_current(1, 1));
        assert!(!protocol.variable_is_current(1, 39));
        assert!(!protocol.variable_is_current(1, 40));
        assert!(protocol.variable_is_current(40, 40));
        assert!(!protocol.ordinary_replacement_changes_epoch());
        assert!(protocol.miss_preserves_primary());
    }
}

#[cfg(test)]
#[test]
fn origin_canonical_roles_keep_query_names_independent_of_reports() {
    use crate::naming::{NativeJimNamespaceConstruction as Construction, NativeNameContext};
    let recipe = NativeJimLookupProtocol::jim084();
    let root = NativeNameContext::root();
    let relative = recipe
        .namespace_origin_canonical(root, b":__no_literal")
        .unwrap();
    assert_eq!(relative.construction, Construction::RetainOriginal);
    assert_eq!(relative.selected.as_bytes(), b":__no_literal");
    assert_eq!(
        recipe.namespace_origin_result(relative.selected.as_bytes()),
        b":::__no_literal"
    );
    let absolute = recipe
        .namespace_origin_canonical(root, b"::::__no_target")
        .unwrap();
    assert_eq!(absolute.construction, Construction::FreshString);
    assert_eq!(absolute.selected.as_bytes(), b"__no_target");
    let path = tcl_core_types::ByteNamespacePath::root().with_child(b"__no_context");
    let local = recipe
        .namespace_origin_canonical(
            NativeNameContext::with_jim_namespace(&path, b"__no_context"),
            b"__no_target",
        )
        .unwrap();
    assert_eq!(
        local.construction,
        Construction::DuplicateNamespaceAndAppend
    );
    assert_eq!(local.selected.as_bytes(), b"__no_context::__no_target");
    assert_eq!(
        recipe.namespace_origin_missing(b"__no_absent ARG"),
        b"invalid command name \"__no_absent ARG\""
    );
    assert_eq!(
        recipe.namespace_origin_canonical(NativeNameContext::new(&path), b"name"),
        Err(crate::naming::NameProjectionUnavailable::MissingJimNamespaceObject)
    );
}

#[cfg(test)]
#[test]
fn namespace_upvar_targets_keep_original_counted_helper_geometry() {
    let protocol = NativeJimLookupProtocol::jim084();
    for (canonical, other, expected) in [
        (b"".as_slice(), b"x".as_slice(), b"::::x".as_slice()),
        (b"N", b"x", b"::N::x"),
        (b"N", b"::x", b"::N::::x"),
        (b"N", b"", b"::N::"),
        (b"", b"", b"::::"),
        (b"outer::child", b"x", b"::outer::child::x"),
        (b"N\0tail", b"x\0tail", b"::N\0tail::x\0tail"),
    ] {
        assert_eq!(protocol.namespace_upvar_target(canonical, other), expected);
    }
}

#[cfg(test)]
mod variable_inventory_lookup_tests {
    use super::{NativeJimLookupProtocol, NativeJimVariableInventoryLookup as Lookup};
    #[test]
    fn original_variable_inventory_lookup_uses_cstring_classification_and_counted_key() {
        let protocol = NativeJimLookupProtocol::jim084();
        assert_eq!(protocol.variable_inventory_lookup(None), Lookup::All);
        for pattern in [b"123".as_slice(), b"::123", b"a\0*", b""] {
            assert_eq!(
                protocol.variable_inventory_lookup(Some(pattern)),
                Lookup::Exact(pattern)
            );
        }
        for pattern in [b"*".as_slice(), b"a?", b"a[", b"a\\", b"*\0x"] {
            assert_eq!(
                protocol.variable_inventory_lookup(Some(pattern)),
                Lookup::Scan(pattern)
            );
        }
    }
}
