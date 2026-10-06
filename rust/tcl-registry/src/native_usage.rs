// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native wrong-argument header presentation on retained original word bytes.

use tcl_dialect::TclVersion;

/// Actual header operand representation, independent of its written spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeUsageWord<'a> {
    /// Original materialized string, including literal NUL and non-Unicode bytes.
    Original(&'a [u8]),
    /// Independently authenticated C index cache's canonical table spelling.
    /// Native presenters append this `CString` without list-element quoting.
    CanonicalIndex(&'a [u8]),
    /// Authenticated canonical index expansion restored by an ensemble rewrite.
    /// C presenters scan this expanded `CString` as an ordinary header element.
    RewrittenIndex(&'a [u8]),
}

/// Explicitly authored logical wrong-argument presentation capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalUsageProvider {
    /// F5 iRules/iApps core presentation uses the C Tcl 8.4 recipe.
    Tcl84CoreSimulation,
}

/// Origin retained independently of the header recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeUsageOrigin {
    /// Authenticated actual native engine/build point.
    Native,
    /// Deliberately selected logical simulation; no native proof is granted.
    Logical(LogicalUsageProvider),
}

/// Selected usage header recipe. Rendering proves only bytes, never lookup,
/// original operand identity, native index storage, or callback effect closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeUsageProtocol {
    style: crate::NativeArgumentUsageHeader,
    origin: NativeUsageOrigin,
}

impl NativeUsageProtocol {
    /// Independently retained native-versus-logical origin.
    #[must_use]
    pub const fn origin(self) -> NativeUsageOrigin {
        self.origin
    }

    /// Selected byte presenter, without reconstructing the invocation header.
    #[must_use]
    pub const fn style(self) -> crate::NativeArgumentUsageHeader {
        self.style
    }

    /// Render the retained, already rewritten original words. Each C quoted
    /// word uses its own native element scan, including hash protection.
    /// A C index-cache word is unsupported by Jim's actual object protocol.
    #[must_use]
    pub fn render_header(self, words: &[NativeUsageWord<'_>]) -> Option<Vec<u8>> {
        use crate::NativeArgumentUsageHeader as Style;
        let mut output = Vec::new();
        for (index, word) in words.iter().enumerate() {
            if index != 0 {
                output.push(b' ');
            }
            let bytes = match word {
                NativeUsageWord::CanonicalIndex(bytes) => {
                    if self.style == Style::RawWords {
                        return None;
                    }
                    output.extend_from_slice(c_string(bytes));
                    continue;
                }
                NativeUsageWord::RewrittenIndex(bytes) => {
                    if self.style == Style::RawWords {
                        return None;
                    }
                    c_string(bytes)
                }
                NativeUsageWord::Original(bytes) => bytes,
            };
            match self.style {
                Style::RawCStringWords => output.extend_from_slice(c_string(bytes)),
                Style::RawWords => output.extend_from_slice(bytes),
                Style::RawFirstListWords if index == 0 => output.extend_from_slice(bytes),
                Style::RawFirstListWords | Style::ListWords => {
                    tcl_syntax::list::append_list_element(&mut output, bytes, true);
                }
            }
        }
        Some(output)
    }
}

fn c_string(bytes: &[u8]) -> &[u8] {
    tcl_core_types::c_string_extent(bytes)
}

impl crate::InvocationDialect {
    /// Actual native usage recipe; vendor compatibility versions are insufficient.
    #[must_use]
    pub fn native_usage_protocol(self) -> Option<NativeUsageProtocol> {
        use crate::NativeArgumentUsageHeader as Style;
        let string = self.native_string_protocol()?;
        let style = match string.tcl_version() {
            Some(TclVersion::V8_4) => Style::RawCStringWords,
            Some(TclVersion::V8_5 | TclVersion::V8_6) => Style::RawFirstListWords,
            Some(TclVersion::V9_0 | TclVersion::V9_1) => Style::ListWords,
            None if string.is_jim084() => Style::RawWords,
            None => return None,
        };
        Some(NativeUsageProtocol {
            style,
            origin: NativeUsageOrigin::Native,
        })
    }

    /// Select native presentation or a deliberately requested logical provider.
    #[must_use]
    pub fn usage_protocol(
        self,
        provider: Option<LogicalUsageProvider>,
    ) -> Option<NativeUsageProtocol> {
        if let Some(protocol) = self.native_usage_protocol() {
            return Some(protocol);
        }
        let provider = provider?;
        self.authored_f5_tcl84_core()?;
        Some(NativeUsageProtocol {
            style: crate::NativeArgumentUsageHeader::RawCStringWords,
            origin: NativeUsageOrigin::Logical(provider),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InvocationDialect;

    #[test]
    fn native_headers_retain_original_bytes_and_quote_each_native_word() {
        let words = [
            NativeUsageWord::Original(b"n s\0tail\xc0\x80\xff"),
            NativeUsageWord::Original(b"#x"),
        ];
        for version in TclVersion::ALL {
            let header = InvocationDialect::for_version(version)
                .native_usage_protocol()
                .unwrap()
                .render_header(&words)
                .unwrap();
            let expected: &[u8] = match version {
                TclVersion::V8_4 => b"n s #x",
                TclVersion::V8_5 | TclVersion::V8_6 => b"n s\0tail\xc0\x80\xff {#x}",
                TclVersion::V9_0 | TclVersion::V9_1 => b"{n s\0tail\xc0\x80\xff} {#x}",
            };
            assert_eq!(header, expected, "{version:?}");
        }
        let jim = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").analyser_profile(),
        );
        assert_eq!(
            jim.native_usage_protocol()
                .unwrap()
                .render_header(&words)
                .unwrap(),
            b"n s\0tail\xc0\x80\xff #x"
        );
    }

    #[test]
    fn authenticated_index_cache_is_distinct_from_same_spelled_original_word() {
        let protocol = InvocationDialect::for_version(TclVersion::V9_0)
            .native_usage_protocol()
            .unwrap();
        assert_eq!(
            protocol
                .render_header(&[
                    NativeUsageWord::Original(b"n s"),
                    NativeUsageWord::CanonicalIndex(b"#i x\0tail")
                ])
                .unwrap(),
            b"{n s} #i x"
        );
        assert_eq!(
            protocol
                .render_header(&[
                    NativeUsageWord::Original(b"n s"),
                    NativeUsageWord::Original(b"#i x\0tail")
                ])
                .unwrap(),
            b"{n s} {#i x\0tail}"
        );
        assert_eq!(
            protocol
                .render_header(&[
                    NativeUsageWord::Original(b"n s"),
                    NativeUsageWord::RewrittenIndex(b"#i x\0tail")
                ])
                .unwrap(),
            b"{n s} {#i x}"
        );
        let jim = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").analyser_profile(),
        );
        assert_eq!(
            jim.native_usage_protocol()
                .unwrap()
                .render_header(&[NativeUsageWord::CanonicalIndex(b"x")]),
            None
        );
    }

    #[test]
    fn logical_provider_does_not_grant_native_authority() {
        let dialect = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("irules").analyser_profile(),
        );
        assert_eq!(dialect.native_usage_protocol(), None);
        assert_eq!(dialect.usage_protocol(None), None);
        let protocol = dialect
            .usage_protocol(Some(LogicalUsageProvider::Tcl84CoreSimulation))
            .unwrap();
        assert_eq!(
            protocol.origin(),
            NativeUsageOrigin::Logical(LogicalUsageProvider::Tcl84CoreSimulation)
        );
        assert_eq!(
            protocol
                .render_header(&[NativeUsageWord::Original(b"n\0s")])
                .unwrap(),
            b"n"
        );
        let mut unknown = InvocationDialect::for_version(TclVersion::V9_0);
        unknown.core_point = None;
        unknown.native_family = None;
        assert_eq!(
            unknown.usage_protocol(Some(LogicalUsageProvider::Tcl84CoreSimulation)),
            None
        );
    }
}
