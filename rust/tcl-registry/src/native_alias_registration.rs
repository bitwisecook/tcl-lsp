// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original alias-registration coordinates retain backend-owned command tokens.

use std::collections::HashMap;
use tcl_core_types::{NameBytes, NativeEntryLedger, NativeHashAbi, NativeHashRecipe};
use tcl_syntax::naming::{NameProjectionUnavailable, NativeNameProjection};

/// C registration records survive command relocation until token deletion.
/// C8.4 collision replacement and later-C unique tokens use separate doors.
/// Backend tokens supply no callable ownership or liveness proof.
pub struct NativeAliasRegistrationTable<Token> {
    registrations: HashMap<NameBytes, Token>,
    entries: NativeEntryLedger,
}

impl<Token> Default for NativeAliasRegistrationTable<Token> {
    fn default() -> Self {
        Self {
            registrations: HashMap::new(),
            entries: NativeEntryLedger::default(),
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
            self.entries.insert(key);
            self.registrations.insert(NameBytes::from(key), token);
        }
        Ok(())
    }

    /// Borrow a C alias registration independently of command relocation.
    ///
    /// # Errors
    /// Refuses an operand selected for another naming purpose.
    pub fn query(
        &self,
        selected: &NativeNameProjection<'_>,
    ) -> Result<Option<&Token>, NameProjectionUnavailable> {
        Ok(selected
            .alias_registration_key()?
            .and_then(|key| self.registrations.get(key)))
    }

    /// Retain a later-C registration at its original unique token coordinate.
    ///
    /// # Errors
    /// Refuses another purpose or a still-occupied registration coordinate.
    pub fn install_unique(
        &mut self,
        selected: &NativeNameProjection<'_>,
        token: Token,
    ) -> Result<(), NameProjectionUnavailable> {
        if !matches!(
            selected.purpose(),
            tcl_syntax::naming::NativeNamePurpose::AliasPublication
                | tcl_syntax::naming::NativeNamePurpose::ChildAliasPublication
        ) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        if let Some(key) = selected.alias_registration_key()? {
            if self.registrations.contains_key(key) {
                return Err(NameProjectionUnavailable::PurposeNotModelled);
            }
            self.entries.insert(key);
            self.registrations.insert(NameBytes::from(key), token);
        }
        Ok(())
    }

    /// Select original C `STRING_KEYS` arithmetic with independently supplied ABI.
    /// This grants no interpreter authority; physical entry history is retained.
    pub fn select_order(
        &mut self,
        names: tcl_syntax::naming::NativeNameProtocol,
        abi: NativeHashAbi,
    ) {
        let recipe = match names {
            tcl_syntax::naming::NativeNameProtocol::C(version) => Some(NativeHashRecipe::Tcl {
                promotion: abi.plain_char,
                width: if version < tcl_dialect::TclVersion::V9_0 {
                    abi.unsigned_int
                } else {
                    abi.size_t
                },
            }),
            tcl_syntax::naming::NativeNameProtocol::Jim084 => None,
        };
        self.entries.select_recipe(recipe);
    }

    /// Retained registrations in original physical table order.
    /// Missing independently selected arithmetic remains explicit abstention.
    #[must_use]
    pub fn ordered_tokens(&self) -> Option<Vec<&Token>> {
        Some(
            self.entries
                .keys()?
                .into_iter()
                .filter_map(|key| self.registrations.get(key))
                .collect(),
        )
    }

    /// Drop records when the backend confirms their actual token was deleted.
    /// Relocating a live command does not satisfy this predicate.
    pub fn forget_deleted(&mut self, mut deleted: impl FnMut(&Token) -> bool) {
        let entries = &mut self.entries;
        self.registrations.retain(|key, token| {
            if deleted(token) {
                entries.remove(key.as_bytes());
                false
            } else {
                true
            }
        });
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
    #[test]
    fn modern_alias_query_retains_registration_coordinate_without_replacement_authority() {
        let modern = NativeNameProtocol::C(TclVersion::V8_6);
        let original = modern
            .alias_publication_input(NativeNameContext::root(), b"::original")
            .unwrap();
        let query = modern
            .alias_registration_query_input(b"::original")
            .unwrap();
        let moved = modern.alias_registration_query_input(b"::moved").unwrap();
        let mut table = NativeAliasRegistrationTable::default();
        table.install_unique(&original, 17).unwrap();
        assert_eq!(table.query(&query).unwrap(), Some(&17));
        assert_eq!(table.query(&moved).unwrap(), None);
        assert_eq!(table.prior(&original).unwrap(), None);
        assert!(table.install_unique(&query, 23).is_err());
        table.forget_deleted(|token| *token == 17);
        assert_eq!(table.query(&query).unwrap(), None);
    }
}
