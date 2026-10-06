// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pinned Jim original command and variable cache validity, without lookup authority.

/// Exact current Jim cache rules. A pure recipe is not an engine issuer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeJimLookupProtocol(());

/// Native link target construction, independent of variable key equality.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeJimLinkTargetInput<'a> {
    /// Relative targets retain the same original operand in the selected frame.
    Original,
    /// Absolute targets own a fresh counted stripped object in the top frame.
    StrippedGlobal(&'a [u8]),
}

impl NativeJimLookupProtocol {
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
