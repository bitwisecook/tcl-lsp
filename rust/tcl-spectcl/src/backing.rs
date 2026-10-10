// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The `runtime_backing` statement — one spelling for the loader, the
//! renderers and the Spec Studio.
//!
//! ```text
//! runtime_backing none
//! runtime_backing host-native
//! runtime_backing shipped-builtin ID
//! runtime_backing tcl-body {-package-source PATH ?-evaluate?}
//! runtime_backing tcl-body {-pack-text {TEXT} ?-evaluate?}
//! ```
//!
//! `-evaluate` is the author's assertion that the body may be run to fold a call
//! at analysis time: nothing derives a declared implementation from a Tcl body
//! whose author did not say so, because the engine that runs it emulates an older
//! release imperfectly and only the author can vouch that the body does not meet
//! the difference. It may stand before or after the source, and is spelled after
//! it.
//!
//! Each line is one variant of [`RuntimeBacking`]
//! (`docs/design/compiler/registry-consumer-contracts.md` § *Four rungs of
//! codegen meeting `.tclspec`*, rung 4). [`BackingSyntax`] is the statement
//! read into owned strings: the loader gives them a `'static` lifetime with
//! [`BackingSyntax::leak`], as it does for the rest of a pack's data, while the
//! Studio parses and re-spells a draft on every edit and must not leak.

use tcl_registry::{BodySource, RuntimeBacking};
use tcl_syntax::list::{join_list, list_element, split_list_lenient};

/// What the statement's words say when they do not read.
const EXPECTED: &str = "expected `none`, `host-native`, `shipped-builtin ID`, \
                        `tcl-body {-package-source PATH ?-evaluate?}` or \
                        `tcl-body {-pack-text {TEXT} ?-evaluate?}`";

/// A `runtime_backing` declaration, as the statement spells it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackingSyntax {
    /// `none` — nothing executes the command in the target runtime.
    None,
    /// `host-native` — the host registered the command natively.
    HostNative,
    /// `shipped-builtin ID` — a shipped builtin known by `identity`.
    ShippedBuiltin {
        /// The builtin's registry identity.
        identity: String,
    },
    /// `tcl-body {-package-source PATH}` — a body in the package's own
    /// installed source.
    PackageSource {
        /// The path, relative to the package's installed source.
        relative_path: String,
        /// Whether the author asserted the body may be evaluated (`-evaluate`).
        evaluate: bool,
    },
    /// `tcl-body {-pack-text {TEXT}}` — a body carried in the pack.
    PackText {
        /// The body text.
        text: String,
        /// Whether the author asserted the body may be evaluated (`-evaluate`).
        evaluate: bool,
    },
}

impl BackingSyntax {
    /// Read the words after `runtime_backing`.
    ///
    /// # Errors
    ///
    /// The words name no variant, or name one with the wrong operands.
    pub fn parse(words: &[String]) -> Result<Self, String> {
        let words: Vec<&str> = words.iter().map(String::as_str).collect();
        match words.as_slice() {
            ["none"] => Ok(Self::None),
            ["host-native"] => Ok(Self::HostNative),
            ["shipped-builtin", identity] if !identity.is_empty() => Ok(Self::ShippedBuiltin {
                identity: (*identity).to_owned(),
            }),
            ["tcl-body", source] => {
                let source = split_list_lenient(source);
                let words = source.iter().map(AsRef::as_ref).collect::<Vec<&str>>();
                // The flag stands before the pair or after it, so a word that is
                // the flag's spelling and a pair's value is told by where it is.
                let (pair, evaluate) = match words[..] {
                    ["-evaluate", ref pair @ ..] | [ref pair @ .., "-evaluate"]
                        if pair.len() == 2 =>
                    {
                        (pair, true)
                    }
                    ref pair => (pair, false),
                };
                match *pair {
                    ["-package-source", path] if !path.is_empty() => Ok(Self::PackageSource {
                        relative_path: path.to_owned(),
                        evaluate,
                    }),
                    ["-pack-text", text] if !text.is_empty() => Ok(Self::PackText {
                        text: text.to_owned(),
                        evaluate,
                    }),
                    _ => Err("`tcl-body` takes `{-package-source PATH ?-evaluate?}` or \
                              `{-pack-text {TEXT} ?-evaluate?}`"
                        .to_owned()),
                }
            }
            _ => Err(EXPECTED.to_owned()),
        }
    }

    /// Read a spelling held as one string — the Studio's draft value, which
    /// is the statement without its leading word.
    ///
    /// # Errors
    ///
    /// As [`Self::parse`].
    pub fn parse_spelling(text: &str) -> Result<Self, String> {
        let words: Vec<String> = split_list_lenient(text)
            .into_iter()
            .map(std::borrow::Cow::into_owned)
            .collect();
        Self::parse(&words)
    }

    /// The spelling [`Self::parse_spelling`] reads back: the statement
    /// without its leading word.
    #[must_use]
    pub fn spelling(&self) -> String {
        match self {
            Self::None => "none".to_owned(),
            Self::HostNative => "host-native".to_owned(),
            Self::ShippedBuiltin { identity } => {
                format!("shipped-builtin {}", list_element(identity))
            }
            Self::PackageSource {
                relative_path,
                evaluate,
            } => body_spelling("-package-source", relative_path, *evaluate),
            Self::PackText { text, evaluate } => body_spelling("-pack-text", text, *evaluate),
        }
    }

    /// The spelling of a backing a spec already carries.
    #[must_use]
    pub fn from_backing(backing: RuntimeBacking) -> Self {
        match backing {
            RuntimeBacking::None => Self::None,
            RuntimeBacking::HostNative => Self::HostNative,
            RuntimeBacking::ShippedBuiltin { identity } => Self::ShippedBuiltin {
                identity: identity.to_owned(),
            },
            RuntimeBacking::TclBody {
                source: BodySource::PackageSource { relative_path },
                evaluate,
            } => Self::PackageSource {
                relative_path: relative_path.to_owned(),
                evaluate,
            },
            RuntimeBacking::TclBody {
                source: BodySource::PackText { text },
                evaluate,
            } => Self::PackText {
                text: text.to_owned(),
                evaluate,
            },
        }
    }

    /// The backing this spells, its strings given the `'static` lifetime a
    /// pack's own data has (interned, so a re-read of an unchanged pack leaks
    /// nothing new).
    #[must_use]
    pub fn leak(&self) -> RuntimeBacking {
        use crate::loader::leak_str;
        match self {
            Self::None => RuntimeBacking::None,
            Self::HostNative => RuntimeBacking::HostNative,
            Self::ShippedBuiltin { identity } => RuntimeBacking::shipped(leak_str(identity)),
            Self::PackageSource {
                relative_path,
                evaluate,
            } => {
                let backing = RuntimeBacking::package_source(leak_str(relative_path));
                if *evaluate {
                    backing.evaluated()
                } else {
                    backing
                }
            }
            Self::PackText { text, evaluate } => {
                let backing = RuntimeBacking::pack_text(leak_str(text));
                if *evaluate {
                    backing.evaluated()
                } else {
                    backing
                }
            }
        }
    }
}

/// The `tcl-body` spelling of a body of one source, `-evaluate` after it when the
/// author asserted it.
fn body_spelling(option: &str, operand: &str, evaluate: bool) -> String {
    let mut words = vec![option, operand];
    if evaluate {
        words.push("-evaluate");
    }
    format!("tcl-body {}", list_element(&join_list(words)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn each_variant_reads_from_its_statement() {
        assert_eq!(
            BackingSyntax::parse(&words("none")),
            Ok(BackingSyntax::None)
        );
        assert_eq!(
            BackingSyntax::parse(&words("host-native")),
            Ok(BackingSyntax::HostNative)
        );
        assert_eq!(
            BackingSyntax::parse(&words("shipped-builtin lassign")),
            Ok(BackingSyntax::ShippedBuiltin {
                identity: "lassign".to_owned()
            })
        );
        assert_eq!(
            BackingSyntax::parse(&["tcl-body".to_owned(), "-package-source init.tcl".to_owned()]),
            Ok(BackingSyntax::PackageSource {
                relative_path: "init.tcl".to_owned(),
                evaluate: false,
            })
        );
        assert_eq!(
            BackingSyntax::parse(&[
                "tcl-body".to_owned(),
                "-pack-text {proc p {} {return 1}}".to_owned()
            ]),
            Ok(BackingSyntax::PackText {
                text: "proc p {} {return 1}".to_owned(),
                evaluate: false,
            })
        );
    }

    /// The author's assertion is `-evaluate`, before the source or after it, and
    /// a text that happens to be that word is a text when it stands where one does.
    #[test]
    fn the_evaluate_flag_stands_before_the_source_or_after_it() {
        let read = |source: &str| BackingSyntax::parse(&["tcl-body".to_owned(), source.to_owned()]);
        let text = |text: &str, evaluate| {
            Ok(BackingSyntax::PackText {
                text: text.to_owned(),
                evaluate,
            })
        };
        assert_eq!(
            read("-pack-text {proc p {} {}} -evaluate"),
            text("proc p {} {}", true)
        );
        assert_eq!(
            read("-evaluate -pack-text {proc p {} {}}"),
            text("proc p {} {}", true)
        );
        assert_eq!(
            read("-pack-text {proc p {} {}}"),
            text("proc p {} {}", false)
        );
        assert_eq!(read("-pack-text -evaluate"), text("-evaluate", false));
        assert_eq!(
            read("-package-source lib/a.tcl -evaluate"),
            Ok(BackingSyntax::PackageSource {
                relative_path: "lib/a.tcl".to_owned(),
                evaluate: true,
            })
        );
        for bad in [
            "-evaluate",
            "-evaluate -evaluate",
            "-pack-text {x} -evaluate -evaluate",
            "-pack-text {x} -evaluate extra",
            "-package-source a b -evaluate",
        ] {
            assert!(read(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn what_does_not_read_is_refused_with_the_shapes_it_could_have_been() {
        for bad in [
            "",
            "none extra",
            "host-native extra",
            "shipped-builtin",
            "shipped-builtin a b",
            "tcl-body",
            "native",
        ] {
            let error = BackingSyntax::parse(&words(bad)).expect_err(bad);
            assert!(
                error.contains("expected") || error.contains("tcl-body"),
                "{bad}: {error}"
            );
        }
        for bad in [
            ["tcl-body", "-package-source"],
            ["tcl-body", "-package-source a b"],
            ["tcl-body", "-pack-text"],
            ["tcl-body", "-source x"],
        ] {
            let bad: Vec<String> = bad.iter().map(|word| (*word).to_owned()).collect();
            assert!(BackingSyntax::parse(&bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_spelling_reads_back_as_itself() {
        for syntax in [
            BackingSyntax::None,
            BackingSyntax::HostNative,
            BackingSyntax::ShippedBuiltin {
                identity: "::tcl::dict::get".to_owned(),
            },
            BackingSyntax::PackageSource {
                relative_path: "lib/parray.tcl".to_owned(),
                evaluate: false,
            },
            BackingSyntax::PackageSource {
                relative_path: "a dir/with spaces.tcl".to_owned(),
                evaluate: true,
            },
            BackingSyntax::PackText {
                text: "proc p {a} {\n    return [list $a \"x\"]\n}".to_owned(),
                evaluate: false,
            },
            // Unbalanced braces are backslash-quoted rather than braced.
            BackingSyntax::PackText {
                text: "puts \"{\"".to_owned(),
                evaluate: true,
            },
            // A text that is the flag's own spelling.
            BackingSyntax::PackText {
                text: "-evaluate".to_owned(),
                evaluate: true,
            },
        ] {
            let spelling = syntax.spelling();
            assert_eq!(
                BackingSyntax::parse_spelling(&spelling),
                Ok(syntax),
                "{spelling}"
            );
        }
    }

    #[test]
    fn a_spec_s_backing_round_trips_through_the_spelling() {
        for backing in [
            RuntimeBacking::None,
            RuntimeBacking::HostNative,
            RuntimeBacking::shipped("lassign"),
            RuntimeBacking::package_source("init.tcl"),
            RuntimeBacking::pack_text("return 1"),
            RuntimeBacking::pack_text("return 1").evaluated(),
            RuntimeBacking::package_source("init.tcl").evaluated(),
        ] {
            let syntax = BackingSyntax::from_backing(backing);
            let read = BackingSyntax::parse_spelling(&syntax.spelling()).expect("reads");
            assert_eq!(read.leak(), backing);
        }
    }
}
