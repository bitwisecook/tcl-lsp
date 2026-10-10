// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected basic timer callback and global variable-wait purposes.

use crate::InvocationDialect;
use tcl_dialect::TclVersion;
use tcl_syntax::native_object::{NativeObjectCacheSnapshot, NativeObjectSnapshot};
use tcl_syntax::scalar_getter::NativeScalarCache;

/// The selected wait trigger, independently of ordinary variable reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeEventWait {
    /// C installs a global write/unset trace on its `CString` subject.
    GlobalWriteOrUnset,
    /// Jim retains and compares global values after processing events.
    GlobalValueComparison,
}

/// Basic event purposes selected from the actual installed provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeEventProtocol {
    version: Option<TclVersion>,
    numbers: tcl_syntax::scalar_getter::NativeScalarGetterProtocol,
}
impl NativeEventProtocol {
    /// Actual wait trigger; no observer registration is granted by this recipe.
    #[must_use]
    pub const fn wait(self) -> NativeEventWait {
        if self.version.is_some() {
            NativeEventWait::GlobalWriteOrUnset
        } else {
            NativeEventWait::GlobalValueComparison
        }
    }
    /// Selected wait argv grammar, without notifier or variable lookup authority.
    #[must_use]
    pub const fn vwait(self) -> crate::native_vwait::NativeVwaitProtocol {
        match self.version {
            Some(TclVersion::V8_4 | TclVersion::V8_5 | TclVersion::V8_6) => {
                crate::native_vwait::NativeVwaitProtocol::CLegacy
            }
            Some(version) => crate::native_vwait::NativeVwaitProtocol::CExtended(version),
            None => crate::native_vwait::NativeVwaitProtocol::Jim084,
        }
    }
    /// C retains one script operand; Jim always enters its concat constructor.
    #[must_use]
    pub const fn retains_single_script(self) -> bool {
        self.version.is_some()
    }
    /// Event identifier numbering at the first registration.
    #[must_use]
    pub const fn first_id(self) -> u64 {
        if self.version.is_some() { 0 } else { 1 }
    }
    /// Actual static after-option table, retaining the Jim table's own order.
    #[must_use]
    pub const fn after_options(self) -> &'static [&'static str] {
        if self.version.is_some() {
            &["cancel", "idle", "info"]
        } else {
            &["cancel", "info", "idle"]
        }
    }
    /// Jim's after enum lookup requests exact matches.
    #[must_use]
    pub const fn after_options_exact(self) -> bool {
        self.version.is_none()
    }
    /// Jim's idletasks processes time events; C restricts it to idle handlers.
    #[must_use]
    pub const fn idle_update_runs_timers(self) -> bool {
        self.version.is_none()
    }
    /// Jim's idle registration is a zero-delay time event.
    #[must_use]
    pub const fn idle_is_timer(self) -> bool {
        self.version.is_none()
    }
    /// Jim searches event identifiers before comparing concatenated scripts.
    #[must_use]
    pub const fn cancel_id_first(self) -> bool {
        self.version.is_none()
    }
    /// Jim searches and reports its time-handler list by deadline.
    #[must_use]
    pub const fn deadline_order(self) -> bool {
        self.version.is_none()
    }
    /// A zero-delay Jim time handler reports the idle kind.
    #[must_use]
    pub const fn zero_timer_reports_idle(self) -> bool {
        self.version.is_none()
    }
    /// Parse the actual `CString` event identifier under the independently selected C ABI.
    /// Jim keeps its own decimal `Jim_StringToWide` conversion and signed ID domain.
    /// # Errors
    /// Declines unavailable C unsigned-long/int widths or a foreign numeric recipe.
    pub fn after_id(
        self,
        bytes: &[u8],
        abi: Option<tcl_core_types::NativeArraySearchAbi>,
    ) -> Result<Option<u64>, tcl_syntax::value::ValueError> {
        use tcl_syntax::value::ValueError;
        let Some(tail) = tcl_core_types::c_string_extent(bytes).strip_prefix(b"after#") else {
            return Ok(None);
        };
        if self.version.is_none() {
            let value = self.numbers.jim_decimal_wide_probe(tail).ok_or(
                ValueError::CommandProtocolUnavailable("Jim event ID numeric recipe"),
            )?;
            return Ok(value.ok().and_then(|id| u64::try_from(id).ok()));
        }
        let abi =
            abi.filter(|a| a.int_bits == 32)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "C event ID integer ABI",
                ))?;
        let Some((magnitude, extent)) =
            tcl_syntax::native_array_search::native_c_decimal_unsigned(tail, abi.unsigned_long)
        else {
            return Ok(None);
        };
        if extent != tail.len() {
            return Ok(None);
        }
        let low = u32::try_from(magnitude & u64::from(u32::MAX)).expect("native int mask");
        Ok(Some(u64::from(low)))
    }
    /// Actual after numeric getter. C84's int entry is separate from modern wide.
    #[must_use]
    pub const fn number_kind(self) -> tcl_syntax::scalar_getter::NativeScalarGetterKind {
        use tcl_syntax::scalar_getter::NativeScalarGetterKind as Kind;
        match self.version {
            Some(TclVersion::V8_4) => Kind::Int,
            Some(_) => Kind::Wide,
            None => Kind::Double,
        }
    }
    /// C84's already reached native-long fast path narrows to its actual int delay.
    #[must_use]
    pub fn cached_number_ms(self, object: &NativeObjectSnapshot) -> Option<u64> {
        if !matches!(self.version, Some(TclVersion::V8_4)) {
            return None;
        }
        let NativeObjectCacheSnapshot::Numeric(NativeScalarCache::Tcl84Long(ms)) = object.cache
        else {
            return None;
        };
        let bytes = ms.to_le_bytes();
        let narrowed = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        Some(u64::try_from(narrowed).unwrap_or(0))
    }
    /// After callbacks report every non-OK C completion; Jim additionally accepts RETURN.
    #[must_use]
    pub const fn reports_callback_code(self, code: i64) -> bool {
        code != 0 && (self.version.is_some() || code != 2)
    }
    /// Getter-versus-selector chronology over an independently retained primary.
    /// C84 additionally examines its original string's first byte.
    #[must_use]
    pub fn number_before_options(self, object: &NativeObjectSnapshot, first: Option<u8>) -> bool {
        match self.version {
            None | Some(TclVersion::V9_1) => true,
            Some(TclVersion::V8_4) => {
                matches!(
                    object.cache,
                    NativeObjectCacheSnapshot::Numeric(NativeScalarCache::Tcl84Long(_))
                ) || first.is_some_and(|c| c.is_ascii_digit() || c == b'+' || c == b'-')
            }
            Some(_) => matches!(
                &object.cache,
                NativeObjectCacheSnapshot::Numeric(NativeScalarCache::Number(
                    tcl_syntax::number::Number::Int(_) | tcl_syntax::number::Number::Big { .. }
                ))
            ),
        }
    }
    /// C84 propagates its numeric getter error rather than composing an option miss.
    #[must_use]
    pub const fn reports_number_error(self) -> bool {
        matches!(self.version, Some(TclVersion::V8_4))
    }
    /// C clears update's result; Jim retains the last processed callback result.
    #[must_use]
    pub const fn clears_update_result(self) -> bool {
        self.version.is_some()
    }
    /// Exact selected C no-source branch. This does not assert that a notifier
    /// without known timers returns, nor convert an external timeout into a guest error.
    #[must_use]
    pub fn no_sources_message(self, original: &[u8]) -> Option<Vec<u8>> {
        self.version?;
        let mut message = b"can't wait for variable \"".to_vec();
        message.extend_from_slice(tcl_core_types::c_string_extent(original));
        if matches!(self.version, Some(TclVersion::V8_4)) {
            message.extend_from_slice(b"\":  would wait forever");
        } else {
            message.extend_from_slice(b"\": would wait forever");
        }
        Some(message)
    }
    /// C composes this option miss independently of the silent `GetIndex` entry.
    #[must_use]
    pub fn after_miss_message(self, original: &[u8]) -> Option<Vec<u8>> {
        self.version?;
        let mut message = b"bad argument \"".to_vec();
        message.extend_from_slice(tcl_core_types::c_string_extent(original));
        message.extend_from_slice(b"\": must be cancel, idle, info, or ");
        message.extend_from_slice(self.number_noun());
        Some(message)
    }
    /// Current C error-code construction after a selected after miss.
    #[must_use]
    pub fn after_miss_code(self, original: &[u8]) -> Option<Vec<u8>> {
        if matches!(self.version, Some(TclVersion::V8_4)) {
            return Some(b"NONE".to_vec());
        }
        self.version?;
        let mut code = b"TCL LOOKUP INDEX argument".to_vec();
        tcl_syntax::list::append_list_element(
            &mut code,
            tcl_core_types::c_string_extent(original),
            false,
        );
        Some(code)
    }
    /// Current C code for a missing event identifier; Jim retains its own error state.
    #[must_use]
    pub fn missing_event_code(self, original: &[u8]) -> Option<Vec<u8>> {
        if matches!(self.version, Some(TclVersion::V8_4)) {
            return Some(b"NONE".to_vec());
        }
        self.version?;
        let mut code = b"TCL LOOKUP EVENT".to_vec();
        tcl_syntax::list::append_list_element(
            &mut code,
            tcl_core_types::c_string_extent(original),
            false,
        );
        Some(code)
    }
    /// Selected C prefix-format error code; the list parser's silent failure is replaced.
    #[must_use]
    pub const fn background_prefix_format_code(self) -> Option<&'static [u8]> {
        match self.version {
            Some(TclVersion::V8_4 | TclVersion::V8_5) => Some(b"NONE"),
            Some(_) => Some(b"TCL OPERATION INTERP BGERRORFORMAT"),
            None => None,
        }
    }
    /// The basic C no-source diagnostic and independently selected error code.
    #[must_use]
    pub const fn no_sources_code(self) -> Option<&'static [u8]> {
        match self.version {
            Some(TclVersion::V8_4) => Some(b"NONE"),
            Some(_) => Some(b"TCL EVENT NO_SOURCES"),
            None => None,
        }
    }
    /// The noun used in C's independently composed after miss.
    #[must_use]
    pub const fn number_noun(self) -> &'static [u8] {
        if matches!(self.version, Some(TclVersion::V8_4)) {
            b"a number"
        } else {
            b"an integer"
        }
    }
}
impl InvocationDialect {
    /// Select actual C84–C91 or pinned Jim event purposes; unknown axes decline.
    #[must_use]
    pub fn native_event_protocol(self) -> Option<NativeEventProtocol> {
        self.native_scalar_getter_protocol()
            .map(|p| NativeEventProtocol {
                version: p.tcl_version(),
                numbers: p,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn event_wait_and_script_purposes_keep_actual_providers_separate() {
        for version in TclVersion::ALL {
            let p = InvocationDialect::for_version(version)
                .native_event_protocol()
                .unwrap();
            assert_eq!(p.wait(), NativeEventWait::GlobalWriteOrUnset);
            assert!(p.retains_single_script());
            assert!(!p.idle_update_runs_timers());
            assert_eq!(p.first_id(), 0);
        }
        let abi = tcl_core_types::NativeArraySearchAbi {
            unsigned_long: tcl_core_types::NativeHashWordWidth::Bits64,
            int_bits: 32,
        };
        let c = InvocationDialect::for_version(TclVersion::V8_6)
            .native_event_protocol()
            .unwrap();
        assert_eq!(
            c.after_id(b"after# +4294967296", Some(abi)).unwrap(),
            Some(0)
        );
        assert_eq!(c.after_id(b"after#0\0tail", Some(abi)).unwrap(), Some(0));
        assert_eq!(c.after_id(b"after#0\xc0\x80tail", Some(abi)).unwrap(), None);
        assert!(c.after_id(b"after#0", None).is_err());
        let jim = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        let p = jim.native_event_protocol().unwrap();
        assert_eq!(p.wait(), NativeEventWait::GlobalValueComparison);
        assert!(!p.retains_single_script());
        assert!(p.after_options_exact());
        assert!(p.idle_update_runs_timers());
        assert_eq!(p.first_id(), 1);
        assert_eq!(p.after_id(b"after# +1 ", None).unwrap(), Some(1));
        assert_eq!(p.after_id(b"after#-1", None).unwrap(), None);
        let mut absent = InvocationDialect::for_version(TclVersion::V8_6);
        absent.core_point = None;
        absent.native_family = None;
        assert!(absent.native_event_protocol().is_none());
    }
}
