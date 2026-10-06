// SPDX-License-Identifier: AGPL-3.0-or-later
//! Jim's flat command enumeration and namespace helper projection.

use super::{NativeNameContext, NativeNameProtocol};
use crate::native_glob::{NativeGlobUnavailable, NativeNameGlobPurpose, match_native_name_pattern};
use crate::raw_string::RawString;
use tcl_core_types::c_string_extent;

fn matches(pattern: &[u8], candidate: &[u8]) -> Result<bool, NativeGlobUnavailable> {
    match_native_name_pattern(
        NativeNameProtocol::Jim084,
        NativeNameGlobPurpose::InfoCommandsSearch,
        super::native::strip_jim_root(pattern),
        super::native::strip_jim_root(candidate),
    )
}

impl NativeNameProtocol {
    /// Enumerate Jim's actual flat table through its namespace-aware `info` helper.
    /// Candidates are retained primary command keys, already filtered by command
    /// kind and availability. A constructed C namespace path supplies no holder.
    ///
    /// # Errors
    /// Refuses a non-Jim recipe or a reached native string operation.
    pub fn jim_info_command_names(
        self,
        actual_namespace: &[u8],
        original_pattern: Option<&[u8]>,
        candidates: &[Vec<u8>],
        procs_only: bool,
    ) -> Result<Vec<Vec<u8>>, NativeGlobUnavailable> {
        if !self.is_jim084() {
            return Err(NativeGlobUnavailable::PurposeUnavailable);
        }
        let pattern = original_pattern.unwrap_or(b"*");
        let absolute = pattern.starts_with(b"::");
        let delegated = !actual_namespace.is_empty() || absolute;
        let path = tcl_core_types::ByteNamespacePath::root();
        let context = NativeNameContext::with_jim_namespace(&path, actual_namespace);
        let canonical = self
            .jim_namespace_canonical_input(context, pattern)
            .map_err(|_| NativeGlobUnavailable::PurposeUnavailable)?;
        let namespace_count = RawString::from_bytes(actual_namespace)
            .jim084_characters()
            .count();
        let mut names = std::collections::BTreeSet::new();
        for candidate in candidates {
            // Native Jim excludes internal command names containing CString spaces.
            if c_string_extent(candidate).contains(&b' ') {
                continue;
            }
            if !delegated {
                if matches(pattern, candidate)? {
                    names.insert(candidate.clone());
                }
                continue;
            }
            if matches(canonical.selected(), candidate)? {
                if absolute {
                    let mut rooted = b"::".to_vec();
                    rooted.extend_from_slice(candidate);
                    names.insert(rooted);
                } else {
                    let original = RawString::from_bytes(candidate.as_slice());
                    let count = original.jim084_characters().count();
                    let (tail, _) = original
                        .jim084_range_bytes(namespace_count.saturating_add(2), usize::MAX, count)
                        .map_err(NativeGlobUnavailable::JimAccess)?;
                    names.insert(tail);
                }
            }
            if !absolute && !procs_only && matches(pattern, candidate)? {
                names.insert(candidate.clone());
            }
        }
        Ok(names.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn jim_helper_keeps_flat_keys_and_relative_global_fallback_separate() {
        let keys = [
            b"p".to_vec(),
            b"n::p".to_vec(),
            b"n::child::q".to_vec(),
            b"else::q".to_vec(),
            b"internal command".to_vec(),
        ];
        let jim = NativeNameProtocol::Jim084;
        assert_eq!(
            jim.jim_info_command_names(b"n", Some(b"*"), &keys, true)
                .unwrap(),
            [b"child::q".to_vec(), b"p".to_vec()]
        );
        assert_eq!(
            jim.jim_info_command_names(b"n", Some(b"p"), &keys, false)
                .unwrap(),
            [b"p".to_vec()]
        );
        assert_eq!(
            jim.jim_info_command_names(b"n", Some(b"::n::*"), &keys, true)
                .unwrap(),
            [b"::n::child::q".to_vec(), b"::n::p".to_vec()]
        );
        let opaque = [b"n\0z::p\xff".to_vec()];
        assert_eq!(
            jim.jim_info_command_names(b"n\0z", Some(b"*"), &opaque, true)
                .unwrap(),
            [b"p\xff".to_vec()]
        );
    }
}
