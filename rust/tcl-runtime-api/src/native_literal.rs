// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native global literal registrations with concrete original object owners.

use std::collections::HashMap;
use tcl_core_types::NsId;
use tcl_syntax::native_string::NativeStringProtocol;

/// Exact native registration partition, independent of displayed object text.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NativeLiteralKey {
    /// Independently selected native constructor/storage recipe.
    pub protocol: NativeStringProtocol,
    /// Native namespace partition; None is the unpartitioned literal table.
    pub namespace: Option<NsId>,
    /// Original counted key used by this registration.
    pub original: Vec<u8>,
}
struct Registration<V> {
    key: NativeLiteralKey,
    value: V,
    local_arrays: usize,
}

/// One actual interpreter's global literal owners and local-array registrations.
/// Concrete ports retain original objects in V and supply their real string
/// getter and constructor. This kernel authenticates neither callback.
pub struct NativeLiteralWorld<V> {
    entries: Vec<Option<Registration<V>>>,
    index: HashMap<NativeLiteralKey, Vec<usize>>,
    free: Vec<usize>,
}
impl<V> Default for NativeLiteralWorld<V> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            index: HashMap::new(),
            free: Vec::new(),
        }
    }
}
impl<V: Clone> NativeLiteralWorld<V> {
    /// Acquire one actual local-array owner, sharing a matching live original.
    /// A changed resident spelling remains registered but is not reused.
    ///
    /// # Errors
    /// Preserves the concrete native getter or constructor refusal.
    pub fn register<E>(
        &mut self,
        key: NativeLiteralKey,
        mut spelling: impl FnMut(&V) -> Result<Vec<u8>, E>,
        make: impl FnOnce() -> Result<V, E>,
    ) -> Result<(usize, V), E> {
        if let Some(entries) = self.index.get(&key) {
            for &index in entries {
                let entry = self.entries[index]
                    .as_mut()
                    .expect("live global literal index");
                if spelling(&entry.value)? == key.original {
                    entry.local_arrays = entry
                        .local_arrays
                        .checked_add(1)
                        .expect("literal registration owners exhausted");
                    return Ok((index, entry.value.clone()));
                }
            }
        }
        let value = make()?;
        let entry = Some(Registration {
            key: key.clone(),
            value: value.clone(),
            local_arrays: 1,
        });
        let index = if let Some(index) = self.free.pop() {
            self.entries[index] = entry;
            index
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        self.index.entry(key).or_default().push(index);
        Ok((index, value))
    }
    /// Acquire an additional local-array lease for this retained registration.
    /// The index must come from the same live world; no key lookup is replayed.
    pub fn acquire_registration(&mut self, index: usize) -> Option<V> {
        let entry = self.entries.get_mut(index)?.as_mut()?;
        entry.local_arrays = entry
            .local_arrays
            .checked_add(1)
            .expect("literal registration owners exhausted");
        Some(entry.value.clone())
    }
    /// Release one real local-array registration before its local member owner.
    pub fn release(&mut self, index: usize) {
        let entry = self.entries[index]
            .as_mut()
            .expect("retained global literal registration");
        entry.local_arrays = entry
            .local_arrays
            .checked_sub(1)
            .expect("literal registration owner underflow");
        if entry.local_arrays != 0 {
            return;
        }
        let entry = self.entries[index]
            .take()
            .expect("last global literal registration");
        self.free.push(index);
        let bucket = self
            .index
            .get_mut(&entry.key)
            .expect("global literal registration bucket");
        bucket.retain(|candidate| *candidate != index);
        if bucket.is_empty() {
            self.index.remove(&entry.key);
        }
    }
    /// Borrow all actual global owners without acquiring native references.
    pub fn registered_values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().flatten().map(|entry| &entry.value)
    }
    /// Borrow exact-partition owners for an independently selected invalidation.
    pub fn key_values(&self, key: &NativeLiteralKey) -> impl Iterator<Item = &V> {
        self.index
            .get(key)
            .into_iter()
            .flatten()
            .filter_map(|&index| self.entries[index].as_ref().map(|entry| &entry.value))
    }
    /// Borrow the actual retained global object without acquiring a native reference.
    /// The index is meaningful only in this same live registration world.
    #[must_use]
    pub fn registration_value(&self, index: usize) -> Option<&V> {
        self.entries.get(index)?.as_ref().map(|entry| &entry.value)
    }

    /// Actual retained registration owner count, without object/cache authority.
    #[must_use]
    pub fn registration_owners(&self, index: usize) -> Option<usize> {
        self.entries
            .get(index)?
            .as_ref()
            .map(|entry| entry.local_arrays)
    }
}

/// One reusable native procedure localCache name table. Sharing its header
/// retains no additional references to the original name members.
pub struct NativeLocalNameTable<V: Clone> {
    /// Declaration-ordered actual canonical names; synthetic slots stay None.
    pub names: Vec<Option<V>>,
    registrations: Vec<usize>,
    world: std::rc::Weak<std::cell::RefCell<NativeLiteralWorld<V>>>,
}
impl<V: Clone> NativeLocalNameTable<V> {
    /// Register actual canonical members in the unpartitioned global table.
    /// Authentication of the native procedure/layout belongs to the concrete
    /// caller; this factory supplies no lookup receiver or physical authority.
    ///
    /// # Errors
    /// Preserves the selected original constructor or string-getter refusal.
    pub fn create<E>(
        world: &std::rc::Rc<std::cell::RefCell<NativeLiteralWorld<V>>>,
        names: &[Option<tcl_core_types::NameBytes>],
        protocol: NativeStringProtocol,
        mut spelling: impl FnMut(&V) -> Result<Vec<u8>, E>,
        mut make: impl FnMut(&[u8]) -> Result<V, E>,
    ) -> Result<std::rc::Rc<Self>, E> {
        let mut table = Self {
            names: Vec::with_capacity(names.len()),
            registrations: Vec::new(),
            world: std::rc::Rc::downgrade(world),
        };
        for name in names {
            let original = if let Some(name) = name {
                let (registration, value) = world.borrow_mut().register(
                    NativeLiteralKey {
                        protocol,
                        namespace: None,
                        original: name.as_bytes().to_vec(),
                    },
                    &mut spelling,
                    || make(name.as_bytes()),
                )?;
                table.registrations.push(registration);
                Some(value)
            } else {
                None
            };
            table.names.push(original);
        }
        Ok(std::rc::Rc::new(table))
    }
}
impl<V: Clone> Drop for NativeLocalNameTable<V> {
    fn drop(&mut self) {
        if let Some(world) = self.world.upgrade() {
            let mut world = world.borrow_mut();
            for &registration in &self.registrations {
                world.release(registration);
            }
        }
    }
}

/// `TclRegisterLiteral`'s native command namespace partition. The caller
/// supplies the retained actual namespace incarnation and selected absolute-name
/// fact; displayed paths and source qualification cannot mint a partition.
#[must_use]
pub fn command_literal_partition(
    protocol: NativeStringProtocol,
    namespace: NsId,
    fully_qualified: bool,
) -> Option<NsId> {
    use tcl_dialect::TclVersion;
    match protocol {
        NativeStringProtocol::C(TclVersion::V8_4) | NativeStringProtocol::Jim084 => None,
        NativeStringProtocol::C(TclVersion::V8_5) if fully_qualified => None,
        NativeStringProtocol::C(_) => Some(if fully_qualified {
            tcl_core_types::ROOT_NS
        } else {
            namespace
        }),
    }
}

/// Tcl84 registered-literal initialization, distinct from a primitive getter.
/// Only canonical decimal `CString` prefixes admitted by `TclLooksLikeInt` acquire
/// the actual native long primary. The caller supplies the physical long width.
#[must_use]
pub fn registered_c84_long(bytes: &[u8], native_long_bits: u8) -> Option<i64> {
    let extent = tcl_core_types::c_string_extent(bytes);
    let number = std::str::from_utf8(extent).ok()?.parse::<i64>().ok()?;
    let max = match native_long_bits {
        32 => i64::from(i32::MAX),
        64 => i64::MAX,
        _ => return None,
    };
    // TclGetLong's absolute-value spelling excludes native LONG_MIN.
    (number >= -max && number <= max && number.to_string().as_bytes() == extent).then_some(number)
}

/// The source-object rule at native Bytecode object-array publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeSourceLiteralAction {
    /// No identical source object occurs in the local literal array.
    Preserve,
    /// C86+ replaces the identical source literal with a new string-only object.
    CopySourceString,
    /// C84 retains registered self-literals until real global table cleanup.
    RetainSourceCycle,
    /// The selected producer has no supported source self-reference.
    UnsupportedSelfReference,
}
/// Select only the actual source-pointer comparison rule, never byte equality.
#[must_use]
pub fn source_literal_action(
    protocol: NativeStringProtocol,
    same_original: bool,
) -> NativeSourceLiteralAction {
    if !same_original {
        return NativeSourceLiteralAction::Preserve;
    }
    match protocol {
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4) => {
            NativeSourceLiteralAction::RetainSourceCycle
        }
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5) | NativeStringProtocol::Jim084 => {
            NativeSourceLiteralAction::UnsupportedSelfReference
        }
        NativeStringProtocol::C(_) => NativeSourceLiteralAction::CopySourceString,
    }
}

/// C85's no-instructions script trailer uses `TclAddLiteralObj` on a fresh empty object.
/// The caller must independently prove that the original script emitted no instructions.
#[must_use]
pub fn source_literal_empty_result_is_unshared(protocol: NativeStringProtocol) -> bool {
    protocol == NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5)
}

#[cfg(test)]
mod source_literal_tests {
    use super::*;

    #[test]
    fn command_partitions_retain_actual_namespace_and_native_absolute_rule() {
        use tcl_dialect::TclVersion;
        let namespace = NsId(47);
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let protocol = NativeStringProtocol::C(version);
            assert_eq!(
                command_literal_partition(protocol, namespace, false),
                (version != TclVersion::V8_4).then_some(namespace)
            );
            assert_eq!(
                command_literal_partition(protocol, namespace, true),
                (version >= TclVersion::V8_6).then_some(tcl_core_types::ROOT_NS)
            );
        }
        assert_eq!(
            command_literal_partition(NativeStringProtocol::Jim084, namespace, false),
            None
        );
    }

    #[test]
    fn canonical_registered_long_respects_actual_native_width_and_counted_nul() {
        assert_eq!(registered_c84_long(b"17\0tail", 64), Some(17));
        assert_eq!(registered_c84_long(b"+17", 64), None);
        assert_eq!(registered_c84_long(b"017", 64), None);
        assert_eq!(registered_c84_long(b"-0", 64), None);
        assert_eq!(registered_c84_long(b"2147483648", 32), None);
        assert_eq!(registered_c84_long(b"-2147483648", 32), None);
        assert_eq!(registered_c84_long(b"2147483648", 64), Some(2147483648));
        assert_eq!(registered_c84_long(b"-9223372036854775808", 64), None);
        assert_eq!(registered_c84_long(b"17", 0), None);
    }

    #[test]
    fn original_native_pointer_controls_select_distinct_source_rules() {
        use tcl_dialect::TclVersion;
        let mut count = 0;
        for line in include_str!("../tests/data/native_source_literals/observations.tsv")
            .lines()
            .skip(1)
        {
            let columns: Vec<_> = line.split('\t').collect();
            assert_eq!(columns.len(), 5);
            let version = match columns[0] {
                "8.4.20" => TclVersion::V8_4,
                "8.5.19" => TclVersion::V8_5,
                "8.6.18" => TclVersion::V8_6,
                "9.0.4" => TclVersion::V9_0,
                "9.1.0" => TclVersion::V9_1,
                unknown => panic!("unmeasured native source version {unknown}"),
            };
            let protocol = NativeStringProtocol::C(version);
            assert_eq!(
                source_literal_action(protocol, false),
                NativeSourceLiteralAction::Preserve
            );
            let actual_self = columns[4] == "1";
            assert_eq!(actual_self, version == TclVersion::V8_4);
            assert_eq!(
                source_literal_empty_result_is_unshared(protocol),
                version == TclVersion::V8_5
            );
            match version {
                TclVersion::V8_4 => {
                    assert_eq!(
                        source_literal_action(protocol, true),
                        NativeSourceLiteralAction::RetainSourceCycle
                    );
                    assert_eq!(columns[2], "3");
                    assert_eq!(columns[3], "4");
                }
                TclVersion::V8_5 => assert_eq!(
                    source_literal_action(protocol, true),
                    NativeSourceLiteralAction::UnsupportedSelfReference
                ),
                _ => assert_eq!(
                    source_literal_action(protocol, true),
                    NativeSourceLiteralAction::CopySourceString
                ),
            }
            count += 1;
        }
        assert_eq!(count, 10);
    }
}
