// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original alias-registration coordinates retain backend-owned command tokens.

use std::collections::HashMap;
use tcl_core_types::NameBytes;
use tcl_syntax::naming::{NameProjectionUnavailable, NativeNameProjection};

/// C8.4 registration records survive command relocation until token deletion.
/// Tokens remain backend-owned; this table grants no lookup or liveness proof.
pub struct NativeAliasRegistrationTable<Token> {
    registrations: HashMap<NameBytes, Token>,
}

impl<Token> Default for NativeAliasRegistrationTable<Token> {
    fn default() -> Self {
        Self {
            registrations: HashMap::new(),
        }
    }
}

impl<Token> NativeAliasRegistrationTable<Token> {
    /// Borrow the colliding registration while its actual token remains alive.
    /// Call again after retirement to observe callbacks' registration changes.
    ///
    /// # Errors
    /// Refuses a projection selected for another operation.
    pub fn prior(
        &self,
        selected: &NativeNameProjection<'_>,
    ) -> Result<Option<&Token>, NameProjectionUnavailable> {
        Ok(selected
            .alias_registration_replacement_key()?
            .and_then(|key| self.registrations.get(key)))
    }

    /// Retain the newly published actual token after loop and collision checks.
    /// Later C releases and Jim leave this C8.4 table untouched.
    ///
    /// # Errors
    /// Refuses a projection selected for another operation.
    pub fn install(
        &mut self,
        selected: &NativeNameProjection<'_>,
        token: Token,
    ) -> Result<(), NameProjectionUnavailable> {
        if let Some(key) = selected.alias_registration_replacement_key()? {
            self.registrations.insert(NameBytes::from(key), token);
        }
        Ok(())
    }

    /// Drop records when the backend confirms their actual token was deleted.
    /// Relocating a live command does not satisfy this predicate.
    pub fn forget_deleted(&mut self, mut deleted: impl FnMut(&Token) -> bool) {
        self.registrations.retain(|_, token| !deleted(token));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::TclVersion;
    use tcl_syntax::naming::{NativeNameContext, NativeNameProtocol};

    #[test]
    fn alias_registration_coordinates_are_provider_and_purpose_selected() {
        let c84 = NativeNameProtocol::C(TclVersion::V8_4);
        let c85 = NativeNameProtocol::C(TclVersion::V8_5);
        let selected = c84
            .child_alias_publication_input(NativeNameContext::root(), b"::alias\0suffix")
            .unwrap();
        assert_eq!(
            selected.alias_registration_report_bytes(),
            Some(b"::alias\0suffix".as_slice())
        );
        assert_eq!(
            selected.alias_registration_replacement_key().unwrap(),
            Some(b"::alias".as_slice())
        );
        let same_coordinate = c84
            .child_alias_publication_input(NativeNameContext::root(), b"::alias")
            .unwrap();
        let mut table = NativeAliasRegistrationTable::default();
        table.install(&selected, 17).unwrap();
        assert_eq!(table.prior(&same_coordinate).unwrap(), Some(&17));
        table.forget_deleted(|token| *token == 17);
        table.install(&same_coordinate, 29).unwrap();
        table.forget_deleted(|token| *token == 17);
        assert_eq!(table.prior(&selected).unwrap(), Some(&29));
        let later = c85
            .child_alias_publication_input(NativeNameContext::root(), b"::alias")
            .unwrap();
        table.install(&later, 41).unwrap();
        assert_eq!(table.prior(&later).unwrap(), None);
        let lookup = c84
            .namespace_address_input(NativeNameContext::root(), b"::alias")
            .unwrap();
        assert!(table.prior(&lookup).is_err());
    }
}
