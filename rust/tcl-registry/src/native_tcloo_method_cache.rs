// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native `TclOO` call-chain cache validation, independent of method ownership.
use tcl_dialect::TclVersion;
use tcl_syntax::native_string::NativeStringProtocol;

/// Actual observations supplied by the original `TclOO` object owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeTclOoMethodCacheStamp {
    /// Never-reused receiver or class-representative creation identity.
    pub creation: u64,
    /// Original Foundation mutation epoch.
    pub foundation: u64,
    /// Selected object or class-representative mutation epoch.
    pub object: u64,
    /// Actual native public/private/class-cache flags.
    pub flags: u32,
}
/// Selected native cache recipe; snapshots cannot create a method owner.
#[derive(Clone, Copy, Debug)]
pub struct NativeTclOoMethodCacheProtocol {
    strings: NativeStringProtocol,
}
impl NativeTclOoMethodCacheProtocol {
    /// The actual string getter used before a newly issued cache is installed.
    #[must_use]
    pub const fn strings(self) -> NativeStringProtocol {
        self.strings
    }
    /// Native flags for an ordinary public or private lookup and its selected cache.
    #[must_use]
    pub const fn flags(self, public: bool, class_cache: bool) -> u32 {
        (if public { 1 } else { 0 }) | (if class_cache { 0x4000 } else { 0 })
    }
    /// Validate actual original receiver epochs; a public chain may serve an internal call.
    #[must_use]
    pub const fn reusable(
        self,
        captured: NativeTclOoMethodCacheStamp,
        current: NativeTclOoMethodCacheStamp,
    ) -> bool {
        let mask = if current.flags & 1 != 0 { u32::MAX } else { !1 };
        captured.creation == current.creation
            && captured.foundation == current.foundation
            && captured.object == current.object
            && captured.flags & mask == current.flags & mask
    }
}
impl crate::InvocationDialect {
    /// Native C `TclOO` owns this cache only on releases that install `TclOO`.
    #[must_use]
    pub fn native_tcloo_method_cache_protocol(self) -> Option<NativeTclOoMethodCacheProtocol> {
        let strings = self.native_string_protocol()?;
        (strings.tcl_version()? >= TclVersion::V8_6)
            .then_some(NativeTclOoMethodCacheProtocol { strings })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_receiver_epochs_and_visibility_are_independent() {
        let recipe = crate::InvocationDialect::for_version(TclVersion::V8_6)
            .native_tcloo_method_cache_protocol()
            .unwrap();
        let stamp = NativeTclOoMethodCacheStamp {
            creation: 3,
            foundation: 5,
            object: 7,
            flags: recipe.flags(true, true),
        };
        assert!(recipe.reusable(
            stamp,
            NativeTclOoMethodCacheStamp {
                flags: recipe.flags(false, true),
                ..stamp
            }
        ));
        for current in [
            NativeTclOoMethodCacheStamp {
                creation: 4,
                ..stamp
            },
            NativeTclOoMethodCacheStamp {
                foundation: 6,
                ..stamp
            },
            NativeTclOoMethodCacheStamp { object: 8, ..stamp },
            NativeTclOoMethodCacheStamp {
                flags: recipe.flags(true, false),
                ..stamp
            },
        ] {
            assert!(!recipe.reusable(stamp, current));
        }
        assert!(!recipe.reusable(
            NativeTclOoMethodCacheStamp {
                flags: recipe.flags(false, true),
                ..stamp
            },
            stamp
        ));
    }
}
