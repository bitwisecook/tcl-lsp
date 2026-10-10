// SPDX-License-Identifier: AGPL-3.0-or-later
//! Written command component boundaries through the selected native input.

use super::{NativeNameContext, NativeNameProtocol};

impl NativeNameProtocol {
    /// Exact original byte extent of the written terminal command component.
    /// C command lookup selects its `CString` prefix; Jim retains the complete
    /// root-flat object. This partitions a written value, independently of
    /// publication, namespace existence, dispatch, or writable source geometry.
    #[must_use]
    pub fn command_tail_extent(self, original: &[u8]) -> Option<std::ops::Range<usize>> {
        let selected = self
            .command_lookup_input(NativeNameContext::root(), original)
            .ok()?;
        let selected = selected.selected();
        if original.get(..selected.len()) != Some(selected) {
            return None;
        }
        let tail = super::written_command_tail(selected);
        if tail.is_empty() {
            return None;
        }
        Some(selected.len().checked_sub(tail.len())?..selected.len())
    }

    /// Replace only the selected written terminal component. The requested
    /// native value must be one complete nonempty component. Original
    /// qualification and any ignored `CString` suffix remain byte-identical.
    /// This value projection supplies no publication, lookup or source edit.
    #[must_use]
    pub fn replace_command_tail(self, original: &[u8], new_tail: &[u8]) -> Option<Vec<u8>> {
        let old = self.command_tail_extent(original)?;
        if self.command_tail_extent(new_tail)? != (0..new_tail.len()) {
            return None;
        }
        let mut value = Vec::with_capacity(original.len() - old.len() + new_tail.len());
        value.extend_from_slice(original.get(..old.start)?);
        value.extend_from_slice(new_tail);
        value.extend_from_slice(original.get(old.end..)?);
        let selected = self.command_tail_extent(&value)?;
        (value.get(selected) == Some(new_tail)).then_some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_command_tail_extents_preserve_native_input_and_written_qualifiers() {
        // Owner contract: naming.command.written-tail-extent
        // (docs/design/analysis/name-resolution-proofs/written-command-tail-extent.md).
        for version in tcl_dialect::TclVersion::ALL {
            let protocol = NativeNameProtocol::C(version);
            assert_eq!(protocol.command_tail_extent(b"::N:::p"), Some(6..7));
            assert_eq!(
                protocol.command_tail_extent(b"p\0ignored::other"),
                Some(0..1)
            );
            assert_eq!(protocol.command_tail_extent(b"p\xc0\x80suffix"), Some(0..9));
            assert!(protocol.command_tail_extent(b"N::").is_none());
        }
        let protocol = NativeNameProtocol::Jim084;
        assert_eq!(protocol.command_tail_extent(b"::N:::p"), Some(6..7));
        assert_eq!(protocol.command_tail_extent(b"p\0suffix"), Some(0..8));
        assert_eq!(protocol.command_tail_extent(b"relative::p"), Some(10..11));
        assert!(protocol.command_tail_extent(b"").is_none());
    }

    #[test]
    fn original_command_tail_replacement_preserves_qualifiers_and_selected_extent() {
        // Implementation contract: naming.editor.original-command-rename-plans
        // docs/design/analysis/name-resolution-proofs/original-command-rename-plans.md
        for version in tcl_dialect::TclVersion::ALL {
            let protocol = NativeNameProtocol::C(version);
            assert_eq!(
                protocol.replace_command_tail(b"::N:::p", b"new"),
                Some(b"::N:::new".to_vec())
            );
            assert_eq!(
                protocol.replace_command_tail(b"p\0ignored::other", b"new"),
                Some(b"new\0ignored::other".to_vec())
            );
            assert!(
                protocol
                    .replace_command_tail(b"p", b"new\0ignored")
                    .is_none()
            );
            assert!(protocol.replace_command_tail(b"p", b"N::new").is_none());
            assert!(protocol.replace_command_tail(b"N::", b"new").is_none());
        }
        let protocol = NativeNameProtocol::Jim084;
        assert_eq!(
            protocol.replace_command_tail(b"::N:::p", b"new"),
            Some(b"::N:::new".to_vec())
        );
        assert_eq!(
            protocol.replace_command_tail(b"p\0tail", b"new\0tail"),
            Some(b"new\0tail".to_vec())
        );
        assert!(protocol.replace_command_tail(b"p", b"").is_none());
        assert!(protocol.replace_command_tail(b"p", b"N::new").is_none());
    }
}
