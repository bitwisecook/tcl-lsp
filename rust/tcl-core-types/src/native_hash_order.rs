// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Persistent bucket-chain order, with an independently selected native hash recipe.

use alloc::vec;
use alloc::vec::Vec;

/// Integer width supplied by the actual native ABI owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeHashWordWidth {
    /// A 32-bit unsigned native word.
    Bits32,
    /// A 64-bit unsigned native word.
    Bits64,
}

/// Promotion of a native string byte before unsigned hash arithmetic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeHashBytePromotion {
    /// Native signed plain-char promotion.
    Signed,
    /// Unsigned-byte promotion.
    Unsigned,
}

/// Measured ABI facts; this vocabulary alone grants no interpreter authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeHashAbi {
    /// Actual plain-char promotion, independent of the selected Tcl release.
    pub plain_char: NativeHashBytePromotion,
    /// Actual unsigned-int width.
    pub unsigned_int: NativeHashWordWidth,
    /// Actual `size_t` width.
    pub size_t: NativeHashWordWidth,
    /// Actual retained Jim hash randomisation state, when available.
    pub jim_seed: Option<u32>,
}

/// Pure hash/table recipe. Selecting one is not a native cache or identity proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeHashRecipe {
    /// Tcl's forward times-nine string/object hash and fourfold table growth.
    Tcl {
        /// Original ABI byte promotion.
        promotion: NativeHashBytePromotion,
        /// Original native hash arithmetic width.
        width: NativeHashWordWidth,
    },
    /// Jim's reverse-byte hash and doubling table growth.
    Jim {
        /// Retained actual table seed; it is not guessed from pointer identity.
        seed: u32,
    },
}

impl NativeHashRecipe {
    /// Hash an already selected stored key; name extent belongs to its own owner.
    #[must_use]
    pub fn hash(self, key: &[u8]) -> u64 {
        let fold = |value: u64, byte: u8, promotion: NativeHashBytePromotion| {
            let byte = match promotion {
                NativeHashBytePromotion::Signed => {
                    if byte >= 128 {
                        0_u64.wrapping_sub(256 - u64::from(byte))
                    } else {
                        u64::from(byte)
                    }
                }
                NativeHashBytePromotion::Unsigned => u64::from(byte),
            };
            value.wrapping_mul(9).wrapping_add(byte)
        };
        match self {
            Self::Tcl { promotion, width } => {
                let value = key
                    .iter()
                    .fold(0, |value, &byte| fold(value, byte, promotion));
                match width {
                    NativeHashWordWidth::Bits32 => value & u64::from(u32::MAX),
                    NativeHashWordWidth::Bits64 => value,
                }
            }
            Self::Jim { seed } => {
                let value = key.iter().rev().fold(0, |value, &byte| {
                    fold(value, byte, NativeHashBytePromotion::Unsigned)
                });
                value.wrapping_add(u64::from(seed)) & u64::from(u32::MAX)
            }
        }
    }
}

/// A real entry ledger retaining insertion, removal, resize, and capacity history.
#[derive(Clone, Debug)]
pub struct NativeHashOrder {
    recipe: NativeHashRecipe,
    buckets: Vec<Vec<Vec<u8>>>,
    entries: usize,
    revision: u64,
}

impl NativeHashOrder {
    /// Begin an empty table under an independently selected recipe.
    #[must_use]
    pub fn new(recipe: NativeHashRecipe) -> Self {
        let count = match recipe {
            NativeHashRecipe::Tcl { .. } => 4,
            NativeHashRecipe::Jim { .. } => 0,
        };
        Self {
            recipe,
            buckets: vec![Vec::new(); count],
            entries: 0,
            revision: 0,
        }
    }

    /// Retained recipe; a caller must separately establish its native authority.
    #[must_use]
    pub const fn recipe(&self) -> NativeHashRecipe {
        self.recipe
    }

    fn bucket(&self, key: &[u8]) -> usize {
        let mask = self.buckets.len() - 1;
        usize::try_from(self.recipe.hash(key) & u64::try_from(mask).expect("bucket mask fits u64"))
            .expect("masked bucket fits usize")
    }

    /// Allocate a physical entry. Defining an existing shell does not reinsert it.
    pub fn insert(&mut self, key: &[u8]) -> bool {
        if self.buckets.is_empty() {
            self.buckets = vec![Vec::new(); 16];
        }
        if self.buckets[self.bucket(key)]
            .iter()
            .any(|entry| entry == key)
        {
            return false;
        }
        if matches!(self.recipe, NativeHashRecipe::Jim { .. }) && self.entries == self.buckets.len()
        {
            self.rebuild(self.buckets.len() * 2);
        }
        let bucket = self.bucket(key);
        self.buckets[bucket].insert(0, key.to_vec());
        self.entries += 1;
        self.revision = self.revision.wrapping_add(1);
        if matches!(self.recipe, NativeHashRecipe::Tcl { .. })
            && self.entries >= self.buckets.len() * 3
        {
            self.rebuild(self.buckets.len() * 4);
        }
        true
    }

    /// Retire a physical entry without shrinking the bucket array.
    pub fn remove(&mut self, key: &[u8]) -> bool {
        if self.buckets.is_empty() {
            return false;
        }
        let bucket = self.bucket(key);
        let Some(index) = self.buckets[bucket].iter().position(|entry| entry == key) else {
            return false;
        };
        self.buckets[bucket].remove(index);
        self.entries -= 1;
        self.revision = self.revision.wrapping_add(1);
        true
    }

    /// Physical live keys, including undefined entries retained by the owner.
    #[must_use]
    pub fn keys(&self) -> Vec<&[u8]> {
        self.buckets
            .iter()
            .flat_map(|chain| chain.iter().map(Vec::as_slice))
            .collect()
    }

    /// Actual live-entry count.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries
    }
    /// Whether the table has no physical entries.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries == 0
    }
    /// Actual retained bucket count.
    #[must_use]
    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }
    /// Physical entry-allocation/retirement revision, independent of defined values.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }
    /// Bucket chain lengths for the native statistics presenter.
    #[must_use]
    pub fn bucket_lengths(&self) -> Vec<usize> {
        self.buckets.iter().map(Vec::len).collect()
    }

    /// Retain an independently authenticated capacity without allocating keys.
    pub fn retain_bucket_count(&mut self, count: usize) {
        if self.buckets.is_empty() && count != 0 {
            self.buckets = vec![Vec::new(); 16];
        }
        let factor = match self.recipe {
            NativeHashRecipe::Tcl { .. } => 4,
            NativeHashRecipe::Jim { .. } => 2,
        };
        while self.buckets.len() < count {
            self.rebuild(self.buckets.len() * factor);
        }
    }

    fn rebuild(&mut self, count: usize) {
        let old = core::mem::replace(&mut self.buckets, vec![Vec::new(); count]);
        for chain in old {
            for key in chain {
                let bucket = self.bucket(&key);
                self.buckets[bucket].insert(0, key);
            }
        }
    }
}

/// Physical entry transitions retained before or after recipe selection.
///
/// Replaying these actual transitions preserves deleted-entry capacity and
/// undefined-shell births. It never reconstructs a native table from defined
/// variable names or a sorted lookup index.
#[derive(Clone, Debug, Default)]
pub struct NativeEntryLedger {
    history: Vec<(Vec<u8>, bool)>,
    present: alloc::collections::BTreeSet<Vec<u8>>,
    order: Option<NativeHashOrder>,
}

impl NativeEntryLedger {
    /// Select independent table arithmetic, retaining every physical transition.
    pub fn select_recipe(&mut self, recipe: Option<NativeHashRecipe>) {
        if self.order.as_ref().map(NativeHashOrder::recipe) == recipe {
            return;
        }
        self.order = recipe.map(|recipe| {
            let mut order = NativeHashOrder::new(recipe);
            for (key, present) in &self.history {
                if *present {
                    order.insert(key);
                } else {
                    order.remove(key);
                }
            }
            order
        });
    }
    /// Record authentic entry creation, independently of whether it is defined.
    pub fn insert(&mut self, key: &[u8]) {
        if !self.present.insert(key.to_vec()) {
            return;
        }
        self.history.push((key.to_vec(), true));
        if let Some(order) = &mut self.order {
            order.insert(key);
        }
    }
    /// Record authentic entry retirement, independently of the lookup index.
    pub fn remove(&mut self, key: &[u8]) {
        if !self.present.remove(key) {
            return;
        }
        self.history.push((key.to_vec(), false));
        if let Some(order) = &mut self.order {
            order.remove(key);
        }
    }
    /// Original physical key order. Missing recipe is explicit abstention.
    #[must_use]
    pub fn keys(&self) -> Option<Vec<&[u8]>> {
        self.order.as_ref().map(NativeHashOrder::keys)
    }
    /// Physical membership without native iteration-order authority.
    /// Analytical storage consumers may inspect these keys; native inventories
    /// require [`Self::keys`] and its independently selected recipe.
    #[must_use]
    pub fn physical_keys(&self) -> Vec<&[u8]> {
        self.present.iter().map(Vec::as_slice).collect()
    }

    /// Whether this exact physical key currently occupies the table.
    #[must_use]
    pub fn contains_key(&self, key: &[u8]) -> bool {
        self.present.contains(key)
    }

    /// Independently selected arithmetic, with no native lifetime authority.
    #[must_use]
    pub fn recipe(&self) -> Option<NativeHashRecipe> {
        self.order.as_ref().map(NativeHashOrder::recipe)
    }
    /// Physical transition revision once table arithmetic is selected.
    #[must_use]
    pub fn revision(&self) -> Option<u64> {
        self.order.as_ref().map(NativeHashOrder::revision)
    }
    /// Actual retained bucket count, including retired-entry growth.
    #[must_use]
    pub fn bucket_count(&self) -> Option<usize> {
        self.order.as_ref().map(NativeHashOrder::bucket_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                let digit = |byte| match byte {
                    b'0'..=b'9' => byte - b'0',
                    b'a'..=b'f' => byte - b'a' + 10,
                    _ => panic!("hex digit"),
                };
                digit(pair[0]) * 16 + digit(pair[1])
            })
            .collect()
    }

    #[test]
    fn native_c_array_order_retains_original_keys_growth_and_reinsertion() {
        // Native proof naming.variable-table.original-array-growth-and-reinsertion:
        // docs/design/analysis/name-resolution-proofs/variable-table-original-array-growth-and-reinsertion.md
        // Native proof naming.variable-table.original-opaque-key-extents:
        // docs/design/analysis/name-resolution-proofs/variable-table-original-opaque-key-extents.md
        let mut controls = 0;
        for line in include_str!("../tests/data/native_variable_tables/array-order.tsv").lines() {
            let row: Vec<_> = line.split('\t').collect();
            let version = row[0];
            let signed = matches!(version, "8.4.20" | "8.5.19");
            let recipe = NativeHashRecipe::Tcl {
                promotion: if signed {
                    NativeHashBytePromotion::Signed
                } else {
                    NativeHashBytePromotion::Unsigned
                },
                width: if version.starts_with('9') {
                    NativeHashWordWidth::Bits64
                } else {
                    NativeHashWordWidth::Bits32
                },
            };
            let mut table = NativeHashOrder::new(recipe);
            for index in 0..row[1].parse::<usize>().unwrap() {
                let key = if row[2] == "1" && index < 3 {
                    [b"k\0x".as_slice(), b"k\xff", b"k\xc0\x80"][index].to_vec()
                } else {
                    alloc::format!("k{index:02}").into_bytes()
                };
                let key = if version == "8.4.20" {
                    key.split(|byte| *byte == 0).next().unwrap()
                } else {
                    &key
                };
                table.insert(key);
            }
            if row[3] == "1" {
                table.remove(b"k03");
                table.insert(b"k03");
            }
            let observed = bytes(row[4]);
            let expected: Vec<_> = observed.split(|byte| *byte == b' ').collect();
            assert_eq!(table.keys(), expected, "{line}");
            controls += 1;
        }
        assert_eq!(controls, 320);
    }

    #[test]
    fn unselected_ledger_retains_undefined_shell_births_and_deleted_capacity() {
        let mut ledger = NativeEntryLedger::default();
        for index in 0..12 {
            ledger.insert(alloc::format!("k{index:02}").as_bytes());
        }
        ledger.remove(b"k03");
        assert_eq!(ledger.keys(), None);
        ledger.select_recipe(Some(NativeHashRecipe::Tcl {
            promotion: NativeHashBytePromotion::Unsigned,
            width: NativeHashWordWidth::Bits32,
        }));
        assert_eq!(ledger.bucket_count(), Some(16));
        assert_eq!(ledger.keys().unwrap().len(), 11);
        let revision = ledger.revision();
        ledger.insert(b"k04");
        assert_eq!(
            ledger.revision(),
            revision,
            "defining an existing shell preserves its entry"
        );
        ledger.select_recipe(None);
        assert_eq!(ledger.keys(), None);
    }

    #[test]
    fn jim_table_capacity_is_lazy_and_retained_after_last_retirement() {
        let mut table = NativeHashOrder::new(NativeHashRecipe::Jim { seed: 0 });
        assert_eq!(table.bucket_count(), 0);
        assert_eq!(table.keys(), Vec::<&[u8]>::new());
        assert!(!table.remove(b"absent"));
        table.insert(b"x");
        assert_eq!(table.bucket_count(), 16);
        table.remove(b"x");
        assert_eq!(table.bucket_count(), 16);
    }

    #[test]
    fn signedness_and_native_word_width_are_independent_inputs() {
        let signed = NativeHashRecipe::Tcl {
            promotion: NativeHashBytePromotion::Signed,
            width: NativeHashWordWidth::Bits32,
        };
        let unsigned = NativeHashRecipe::Tcl {
            promotion: NativeHashBytePromotion::Unsigned,
            width: NativeHashWordWidth::Bits32,
        };
        assert_ne!(signed.hash(b"k\xff"), unsigned.hash(b"k\xff"));
        let wide = NativeHashRecipe::Tcl {
            promotion: NativeHashBytePromotion::Unsigned,
            width: NativeHashWordWidth::Bits64,
        };
        assert_ne!(
            unsigned.hash(b"long-native-key"),
            wide.hash(b"long-native-key")
        );
    }
}
