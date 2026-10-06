// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native list-object string representations, separate from script quoting.
//!
//! A caller supplies the actual selected engine policy. Rendering establishes
//! bytes only: it proves no list conversion, callback effects, object identity,
//! representation lifetime or permission to erase the producing operation.

/// Audited native list-object serialization policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeListResultSerialization {
    /// Tcl 8.4 `UpdateStringOfList` uses counted-element conversion without
    /// first-element hash protection.
    Tcl84,
    /// Pinned Tcl 8.5–9.1 list objects protect a hash only in position zero.
    Tcl85Plus,
    /// Pinned Jim 0.84 `JimMakeListStringRep` has its own brace/escape choice.
    Jim084,
}

impl NativeListResultSerialization {
    /// Select a pure rendering recipe from an independently authenticated
    /// native string protocol. This mapping grants no execution authority.
    #[must_use]
    pub const fn for_string_protocol(protocol: crate::native_string::NativeStringProtocol) -> Self {
        match protocol {
            crate::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4) => {
                Self::Tcl84
            }
            crate::native_string::NativeStringProtocol::C(_) => Self::Tcl85Plus,
            crate::native_string::NativeStringProtocol::Jim084 => Self::Jim084,
        }
    }
    /// Render already-decoded element bytes under the selected native policy.
    /// Unknown engines must not select a policy through an assistance default.
    #[must_use]
    pub fn render(self, elements: &[impl AsRef<[u8]>]) -> Vec<u8> {
        let mut output = Vec::new();
        for (index, element) in elements.iter().enumerate() {
            if index != 0 {
                output.push(b' ');
            }
            let element = element.as_ref();
            match self {
                Self::Tcl84 => crate::list::append_list_element(&mut output, element, false),
                Self::Tcl85Plus => {
                    crate::list::append_list_element(&mut output, element, index == 0);
                }
                Self::Jim084 => append_jim_element(&mut output, element, index == 0),
            }
        }
        output
    }
}

/// Byte agreement across the audited native policies. Missing agreement is
/// uncertainty, even when every rendered string denotes the same elements.
#[must_use]
pub fn portable_list_result(elements: &[impl AsRef<[u8]>]) -> Option<Vec<u8>> {
    use NativeListResultSerialization as Policy;
    let bytes = Policy::Tcl84.render(elements);
    [Policy::Tcl85Plus, Policy::Jim084]
        .into_iter()
        .all(|policy| policy.render(elements) == bytes)
        .then_some(bytes)
}

#[derive(Clone, Copy)]
enum JimQuote {
    Bare,
    Brace,
    Escape,
}

/// Pinned Jim `ListElementQuotingType`; square-bracket balance and brace
/// preference differ from C's counted-element conversion.
fn jim_quote(element: &[u8]) -> JimQuote {
    if element.is_empty() {
        return JimQuote::Brace;
    }
    let simple = !matches!(element[0], b'"' | b'{');
    if simple
        && !element
            .iter()
            .any(|byte| jim_special(*byte) || matches!(byte, b'{' | b'}'))
    {
        return JimQuote::Bare;
    }
    if element.last() == Some(&b'\\') {
        return JimQuote::Escape;
    }
    let mut braces = 0_i64;
    let mut brackets = 0_i64;
    let mut index = 0;
    while index < element.len() {
        match element[index] {
            b'{' => braces += 1,
            b'}' => {
                braces -= 1;
                if braces < 0 {
                    return JimQuote::Escape;
                }
            }
            b'[' => brackets += 1,
            b']' => brackets -= 1,
            b'\\' => match element.get(index + 1) {
                Some(b'\n') => return JimQuote::Escape,
                Some(next) if *next != 0 => index += 1,
                _ => {}
            },
            _ => {}
        }
        index += 1;
    }
    if braces != 0 || brackets < 0 {
        JimQuote::Escape
    } else if !simple || element.iter().any(|byte| jim_special(*byte)) {
        JimQuote::Brace
    } else {
        JimQuote::Bare
    }
}

const fn jim_special(byte: u8) -> bool {
    matches!(
        byte,
        b' ' | b'$' | b'"' | b'[' | b']' | b';' | b'\\' | b'\r' | b'\n' | b'\t' | 0x0c | 0x0b
    )
}

fn append_jim_element(output: &mut Vec<u8>, element: &[u8], first: bool) {
    let mut quote = jim_quote(element);
    if first && element.first() == Some(&b'#') && matches!(quote, JimQuote::Bare) {
        quote = JimQuote::Brace;
    }
    match quote {
        JimQuote::Bare => output.extend_from_slice(element),
        JimQuote::Brace => {
            output.push(b'{');
            output.extend_from_slice(element);
            output.push(b'}');
        }
        JimQuote::Escape => {
            if first && element.first() == Some(&b'#') {
                output.push(b'\\');
            }
            for &byte in element {
                match byte {
                    b' ' | b'$' | b'"' | b'[' | b']' | b'{' | b'}' | b';' | b'\\' => {
                        output.push(b'\\');
                        output.push(byte);
                    }
                    b'\n' => output.extend_from_slice(b"\\n"),
                    b'\r' => output.extend_from_slice(b"\\r"),
                    b'\t' => output.extend_from_slice(b"\\t"),
                    0x0c => output.extend_from_slice(b"\\f"),
                    0x0b => output.extend_from_slice(b"\\v"),
                    _ => output.push(byte),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_renderer_matches_six_native_ascii_object_serializations() {
        use NativeListResultSerialization as Policy;
        let decode = |hex: &str| {
            hex.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| {
                    let byte = std::str::from_utf8(pair).unwrap();
                    u8::from_str_radix(byte, 16).unwrap()
                })
                .collect::<Vec<_>>()
        };
        let fixture = include_str!("testdata/native_list_result_ascii.tsv");
        let mut observations = 0;
        for row in fixture.lines().filter(|row| !row.starts_with('#')) {
            let fields = row.split_whitespace().collect::<Vec<_>>();
            assert_eq!(fields.len(), 7);
            let element = decode(fields[0]);
            for (index, policy) in [
                Policy::Tcl84,
                Policy::Tcl85Plus,
                Policy::Tcl85Plus,
                Policy::Tcl85Plus,
                Policy::Tcl85Plus,
                Policy::Jim084,
            ]
            .into_iter()
            .enumerate()
            {
                assert_eq!(
                    policy.render(&[&element]),
                    decode(fields[index + 1]),
                    "{policy:?}: {}",
                    fields[0]
                );
                observations += 1;
            }
        }
        assert_eq!(observations, 2304);
    }

    #[test]
    fn first_hash_has_a_selected_native_byte_protocol() {
        use NativeListResultSerialization as Policy;
        assert_eq!(Policy::Tcl84.render(&[b"#value"]), b"#value");
        assert_eq!(Policy::Tcl85Plus.render(&[b"#value"]), b"{#value}");
        assert_eq!(Policy::Jim084.render(&[b"#value"]), b"{#value}");
        assert_eq!(Policy::Jim084.render(&[b"#}"]), b"\\#\\}");
        assert_eq!(portable_list_result(&[b"#value"]), None);
        assert_eq!(
            portable_list_result(&[b"first".as_slice(), b"#value"]),
            Some(b"first #value".to_vec())
        );
    }

    #[test]
    fn jim_brace_choice_is_independent_of_c_hash_quoting() {
        use NativeListResultSerialization as Policy;
        assert_eq!(Policy::Tcl84.render(&[b"a\"b"]), b"a\\\"b");
        assert_eq!(Policy::Tcl85Plus.render(&[b"a\"b"]), b"a\\\"b");
        assert_eq!(Policy::Jim084.render(&[b"a\"b"]), b"{a\"b}");
        assert_eq!(portable_list_result(&[b"a\"b"]), None);
        assert_eq!(
            portable_list_result(&[b"a b".as_slice(), b"c"]),
            Some(b"{a b} c".to_vec())
        );
    }
}
