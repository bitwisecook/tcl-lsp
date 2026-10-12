// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual interp root/child option declarations and selected two-stage lookup.
use tcl_dialect::TclVersion;
use tcl_syntax::native_string::NativeStringProtocol;

/// Actual C-release interp root/child tables and two-stage root lookup policy.
#[derive(Debug, Clone, Copy)]
pub struct NativeInterpreterOptionProtocol(TclVersion);

const ROOT84: &[&str] = &[
    "alias",
    "aliases",
    "create",
    "delete",
    "eval",
    "exists",
    "expose",
    "hide",
    "hidden",
    "issafe",
    "invokehidden",
    "marktrusted",
    "recursionlimit",
    "slaves",
    "share",
    "target",
    "transfer",
];
const CHILD84: &[&str] = &[
    "alias",
    "aliases",
    "eval",
    "expose",
    "hide",
    "hidden",
    "issafe",
    "invokehidden",
    "marktrusted",
    "recursionlimit",
];
const ROOT85: &[&str] = &[
    "alias",
    "aliases",
    "bgerror",
    "create",
    "debug",
    "delete",
    "eval",
    "exists",
    "expose",
    "hide",
    "hidden",
    "issafe",
    "invokehidden",
    "limit",
    "marktrusted",
    "recursionlimit",
    "slaves",
    "share",
    "target",
    "transfer",
];
const CHILD85: &[&str] = &[
    "alias",
    "aliases",
    "bgerror",
    "debug",
    "eval",
    "expose",
    "hide",
    "hidden",
    "issafe",
    "invokehidden",
    "limit",
    "marktrusted",
    "recursionlimit",
];
const ROOT86: &[&str] = &[
    "alias",
    "aliases",
    "bgerror",
    "cancel",
    "children",
    "create",
    "debug",
    "delete",
    "eval",
    "exists",
    "expose",
    "hide",
    "hidden",
    "issafe",
    "invokehidden",
    "limit",
    "marktrusted",
    "recursionlimit",
    "slaves",
    "share",
    "target",
    "transfer",
];
const CHILD86: &[&str] = &[
    "alias",
    "aliases",
    "bgerror",
    "debug",
    "eval",
    "expose",
    "hide",
    "hidden",
    "issafe",
    "invokehidden",
    "limit",
    "marktrusted",
    "recursionlimit",
];
const ROOT90: &[&str] = &[
    "alias",
    "aliases",
    "bgerror",
    "cancel",
    "children",
    "create",
    "debug",
    "delete",
    "eval",
    "exists",
    "expose",
    "hide",
    "hidden",
    "issafe",
    "invokehidden",
    "limit",
    "marktrusted",
    "recursionlimit",
    "share",
    "slaves",
    "target",
    "transfer",
];
const MISS90: &[&str] = &[
    "alias",
    "aliases",
    "bgerror",
    "cancel",
    "children",
    "create",
    "debug",
    "delete",
    "eval",
    "exists",
    "expose",
    "hide",
    "hidden",
    "issafe",
    "invokehidden",
    "limit",
    "marktrusted",
    "recursionlimit",
    "share",
    "target",
    "transfer",
];
const CHILD90: &[&str] = &[
    "alias",
    "aliases",
    "bgerror",
    "debug",
    "eval",
    "expose",
    "hide",
    "hidden",
    "issafe",
    "invokehidden",
    "limit",
    "marktrusted",
    "recursionlimit",
];
const ROOT91: &[&str] = &[
    "alias",
    "aliases",
    "bgerror",
    "cancel",
    "children",
    "create",
    "debug",
    "delete",
    "eval",
    "exists",
    "expose",
    "hide",
    "hidden",
    "issafe",
    "invokehidden",
    "limit",
    "marktrusted",
    "recursionlimit",
    "set",
    "share",
    "slaves",
    "target",
    "transfer",
];
const MISS91: &[&str] = &[
    "alias",
    "aliases",
    "bgerror",
    "cancel",
    "children",
    "create",
    "debug",
    "delete",
    "eval",
    "exists",
    "expose",
    "hide",
    "hidden",
    "issafe",
    "invokehidden",
    "limit",
    "marktrusted",
    "recursionlimit",
    "set",
    "share",
    "target",
    "transfer",
];
const CHILD91: &[&str] = &[
    "alias",
    "aliases",
    "bgerror",
    "debug",
    "eval",
    "expose",
    "hide",
    "hidden",
    "issafe",
    "invokehidden",
    "limit",
    "marktrusted",
    "recursionlimit",
    "set",
];

impl NativeInterpreterOptionProtocol {
    /// Exact native static dispatch table, independently of implemented workers.
    #[must_use]
    pub const fn root(self) -> &'static [&'static str] {
        match self.0 {
            TclVersion::V8_4 => ROOT84,
            TclVersion::V8_5 => ROOT85,
            TclVersion::V8_6 => ROOT86,
            TclVersion::V9_0 => ROOT90,
            TclVersion::V9_1 => ROOT91,
        }
    }
    /// Native C9 preliminary lookup is silent; its miss reuses the original
    /// object against this second table, even if that lookup succeeds.
    #[must_use]
    pub const fn root_miss(self) -> Option<&'static [&'static str]> {
        match self.0 {
            TclVersion::V9_0 => Some(MISS90),
            TclVersion::V9_1 => Some(MISS91),
            TclVersion::V8_4 | TclVersion::V8_5 | TclVersion::V8_6 => None,
        }
    }
    /// Return the native ordered table for commands invoked on a child interpreter.
    #[must_use]
    pub const fn child(self) -> &'static [&'static str] {
        match self.0 {
            TclVersion::V8_4 => CHILD84,
            TclVersion::V8_5 => CHILD85,
            TclVersion::V8_6 => CHILD86,
            TclVersion::V9_0 => CHILD90,
            TclVersion::V9_1 => CHILD91,
        }
    }
}
impl crate::InvocationDialect {
    /// Select interpreter option tables from an authenticated actual C string issuer.
    /// Jim and unavailable issuers return `None`.
    #[must_use]
    pub fn native_interpreter_option_protocol(self) -> Option<NativeInterpreterOptionProtocol> {
        match self.native_string_protocol()? {
            NativeStringProtocol::C(version) => Some(NativeInterpreterOptionProtocol(version)),
            NativeStringProtocol::Jim084 => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURES: [&str; 5] = [
        include_str!("../tests/data/native_interpreter_enum/8.4.20.tsv"),
        include_str!("../tests/data/native_interpreter_enum/8.5.19.tsv"),
        include_str!("../tests/data/native_interpreter_enum/8.6.18.tsv"),
        include_str!("../tests/data/native_interpreter_enum/9.0.4.tsv"),
        include_str!("../tests/data/native_interpreter_enum/9.1.0.tsv"),
    ];
    fn hex(bytes: &[u8]) -> String {
        use std::fmt::Write;
        bytes.iter().fold(String::new(), |mut text, byte| {
            write!(text, "{byte:02x}").unwrap();
            text
        })
    }
    #[test]
    fn original_root_and_child_declarations_match_every_native_entry() {
        let mut entries = 0;
        for (version, fixture) in TclVersion::ALL.into_iter().zip(FIXTURES) {
            let recipe = NativeInterpreterOptionProtocol(version);
            for (label, table) in [
                ("table-root-inventory", recipe.root()),
                ("table-child-inventory", recipe.child()),
            ] {
                let rows: Vec<_> = fixture
                    .lines()
                    .filter(|row| row.starts_with(label))
                    .collect();
                assert_eq!(rows.len(), table.len(), "{version:?} {label}");
                for (index, (row, word)) in rows.into_iter().zip(table).enumerate() {
                    let fields: Vec<_> = row.split('\t').collect();
                    assert_eq!(fields.len(), 3);
                    assert_eq!(fields[1], index.to_string());
                    assert_eq!(fields[2], hex(word.as_bytes()));
                    entries += 1;
                }
            }
        }
        assert_eq!(entries, 167);
    }
    #[test]
    fn actual_c9_second_table_preserves_its_independent_order() {
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            let recipe = NativeInterpreterOptionProtocol(version);
            assert_eq!(
                recipe.root_miss().unwrap(),
                recipe
                    .root()
                    .iter()
                    .copied()
                    .filter(|word| *word != "slaves")
                    .collect::<Vec<_>>()
            );
        }
        assert!(
            NativeInterpreterOptionProtocol(TclVersion::V8_6)
                .root_miss()
                .is_none()
        );
    }
}
