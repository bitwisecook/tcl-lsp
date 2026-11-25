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

use super::*;
use crate::literal_validation::{LiteralArgumentValidation, LiteralValidationDecline};
use crate::{CommandRegistry, InvocationArguments, InvocationDialect, InvocationWord};

#[test]
fn channel_configuration_spellings_share_one_option_projection() {
    let registry = CommandRegistry::build_default();
    let chan = registry.get("chan").unwrap();
    let configure = chan
        .subcommands
        .iter()
        .find(|sub| sub.name == "configure")
        .unwrap();
    let fconfigure = registry.get("fconfigure").unwrap();
    assert!(std::ptr::eq(chan.options, CONFIGURE_OPTIONS));
    assert!(std::ptr::eq(configure.options, CONFIGURE_OPTIONS));
    assert!(std::ptr::eq(fconfigure.options, CONFIGURE_OPTIONS));
    let mut names = std::collections::HashSet::new();
    for row in &CHANNEL_OPTIONS {
        assert!(names.insert(row.option.name));
    }
    assert_eq!(names.len(), CONFIGURE_OPTIONS.len());
}

#[test]
fn channel_driver_catalogues_retain_release_and_access_facts() {
    assert!(
        !options_for(ChannelKind::File, TclVersion::V8_6).any(|row| row.option.name == "-stat")
    );
    assert!(
        options_for(ChannelKind::File, TclVersion::V9_0)
            .any(|row| row.option.name == "-stat" && row.access == ChannelOptionAccess::Query)
    );
    assert!(
        !options_for(ChannelKind::Socket, TclVersion::V8_5)
            .any(|row| row.option.name == "-connecting")
    );
    assert!(
        options_for(ChannelKind::Socket, TclVersion::V8_6)
            .any(|row| row.option.name == "-connecting")
    );
    assert!(
        !options_for(ChannelKind::Serial, TclVersion::V8_6)
            .any(|row| row.option.name == "-closemode")
    );
    assert!(
        options_for(ChannelKind::Serial, TclVersion::V8_4)
            .any(|row| row.option.name == "-handshake")
    );
    assert!(
        !options_for(ChannelKind::Console, TclVersion::V8_6)
            .any(|row| row.option.name == "-inputmode")
    );
    assert!(
        options_for(ChannelKind::Console, TclVersion::V9_0)
            .any(|row| row.option.name == "-inputmode")
    );
    assert!(
        !options_for(ChannelKind::Reflected, TclVersion::V9_0)
            .any(|row| row.option.name == "-peername")
    );
}

#[test]
fn channel_boolean_setters_use_the_strict_string_acceptor() {
    let values = [
        "0", "1", "true", "FALSE", "t", "of", "2", "-1", "0x1", "01", " 1", "1.0", "o", "",
    ];
    for version in TclVersion::ALL {
        for (index, value) in values.iter().enumerate() {
            let valid = index < 6;
            assert_eq!(
                ChannelValueGrammar::Boolean.validate(value, Some(version)),
                ChannelValueGrammar::Boolean.validate(value, None)
            );
            assert_eq!(
                ChannelValueGrammar::Boolean.validate(value, Some(version))
                    == ChannelValueValidation::Valid,
                valid,
                "{version:?}: {value:?}"
            );
            let words = ["stdout", "-blocking", value];
            assert_eq!(
                matches!(
                    validate_fconfigure_configuration(
                        InvocationArguments::literals(&words)
                            .with_dialect(InvocationDialect::for_version(version))
                    ),
                    LiteralArgumentValidation::Valid
                ),
                valid,
                "{version:?}: {value:?}"
            );
        }
    }
}

#[test]
fn channel_boolean_setters_match_reference_tcl() {
    const SCRIPT: &str = r"set original [fconfigure stdout -blocking]
set results {}
foreach value {0 1 true FALSE t of 2 -1 0x1 01 { 1} 1.0 o {}} {
    lappend results [catch {fconfigure stdout -blocking $value}]
}
fconfigure stdout -blocking $original
puts [list [info patchlevel] $results]
";
    for version in TclVersion::ALL {
        let Some(interpreter) = tcl_test_support::witness_tclsh(version) else {
            continue;
        };
        let output = tcl_test_support::run_script_fixture(&interpreter.path, SCRIPT, &[])
            .expect("channel boolean probe executes")
            .strict_text()
            .expect("caught channel boolean probe has no process error");
        assert_eq!(
            output,
            format!("{} {{0 0 0 0 0 0 1 1 1 1 1 1 1 1}}", version.patchlevel()),
            "{version:?}"
        );
    }
}

#[test]
fn channel_value_grammars_preserve_prefixes_lists_and_unknown_contexts() {
    use ChannelValueValidation::{Invalid, Unknown, Valid};
    let version = Some(TclVersion::V9_0);
    for value in ["f", "line", "n"] {
        assert_eq!(
            ChannelValueGrammar::Buffering.validate(value, version),
            Valid
        );
    }
    assert!(matches!(
        ChannelValueGrammar::Buffering.validate("sideways", version),
        Invalid(_)
    ));
    for value in ["-10", "0", "999999999"] {
        assert_eq!(ChannelValueGrammar::Integer.validate(value, version), Valid);
    }
    assert!(matches!(
        ChannelValueGrammar::Integer.validate("oops", version),
        Invalid(_)
    ));
    for value in ["crlf", "{auto} {crlf}", "platform", "{} lf"] {
        assert_eq!(
            ChannelValueGrammar::Translation.validate(value, version),
            Valid,
            "{value}"
        );
    }
    assert_eq!(
        ChannelValueGrammar::Translation.validate("bogus lf", version),
        Unknown
    );
    assert_eq!(
        ChannelValueGrammar::Translation.validate("bogus bogus", version),
        Unknown
    );
    assert_eq!(
        ChannelValueGrammar::Encoding.validate("utf-8", version),
        Valid
    );
    assert_eq!(
        ChannelValueGrammar::Encoding.validate("application-private", version),
        Unknown
    );
    assert_eq!(
        ChannelValueGrammar::Encoding.validate("utf-8", None),
        Unknown
    );
    assert_eq!(
        ChannelValueGrammar::EofChar.validate("abc", Some(TclVersion::V8_6)),
        Valid
    );
    assert!(matches!(
        ChannelValueGrammar::EofChar.validate("abc", version),
        Invalid(_)
    ));
    assert_eq!(
        ChannelValueGrammar::EofChar.validate("a {}", version),
        Valid
    );
}

#[test]
fn source_mode_grammars_keep_abbreviated_masks() {
    for modes in ["read write", "r w", "r r"] {
        assert_eq!(
            validate_create_modes(InvocationArguments::literals(&["create", modes, "handler"])),
            LiteralArgumentValidation::Valid
        );
        assert_eq!(
            validate_postevent_modes(InvocationArguments::literals(&[
                "postevent",
                "channel",
                modes
            ])),
            LiteralArgumentValidation::Valid
        );
    }
    assert!(matches!(
        validate_create_modes(InvocationArguments::literals(&[
            "create", "sideways", "handler"
        ])),
        LiteralArgumentValidation::Invalid(_)
    ));
    for word in [InvocationWord::Dynamic, InvocationWord::Expanded] {
        let words = [
            InvocationWord::Literal("create"),
            word,
            InvocationWord::Literal("handler"),
        ];
        assert!(matches!(
            validate_create_modes(InvocationArguments::structured(&words)),
            LiteralArgumentValidation::Abstain(_)
        ));
    }
}

#[test]
fn configuration_validation_keeps_effective_offsets_and_driver_uncertainty() {
    let dialect = InvocationDialect::for_version(TclVersion::V9_0);
    let bare =
        InvocationArguments::literals(&["channel", "-buffering", "bad"]).with_dialect(dialect);
    let ensemble = InvocationArguments::literals(&["configure", "channel", "-buffering", "bad"])
        .with_dialect(dialect);
    for (result, index) in [
        (validate_fconfigure_configuration(bare), 2),
        (validate_chan_configuration(ensemble), 3),
    ] {
        let LiteralArgumentValidation::Invalid(issue) = result else {
            panic!("expected a bad value");
        };
        assert_eq!(issue.argument_index, index);
    }
    let custom =
        InvocationArguments::literals(&["channel", "-stat", "custom-setter"]).with_dialect(dialect);
    assert!(matches!(
        validate_fconfigure_configuration(custom),
        LiteralArgumentValidation::Abstain(_)
    ));
    let words = [
        InvocationWord::Literal("channel"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("bad"),
    ];
    assert_eq!(
        validate_fconfigure_configuration(
            InvocationArguments::structured(&words).with_dialect(dialect)
        ),
        LiteralArgumentValidation::Abstain(LiteralValidationDecline::NonLiteralArgument)
    );
}

#[test]
fn unselected_list_reader_refusal_is_not_an_invalid_channel_value() {
    use ChannelValueValidation::{Invalid, Unknown, Valid};
    assert_eq!(
        ChannelValueGrammar::Translation.validate(r"\x6c\x66", None),
        Unknown
    );
    assert_eq!(
        ChannelValueGrammar::Translation.validate("{", None),
        Unknown
    );
    assert_eq!(
        ChannelValueGrammar::EofChar.validate(r"\x61", None),
        Unknown
    );
    let create = ["create", r"\x72", "handler"];
    let postevent = ["postevent", "channel", r"\x72"];
    let unknown = LiteralArgumentValidation::Abstain(LiteralValidationDecline::UnselectedGrammar);
    assert_eq!(
        validate_create_modes(InvocationArguments::literals(&create)),
        unknown
    );
    assert_eq!(
        validate_postevent_modes(InvocationArguments::literals(&postevent)),
        unknown
    );
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        assert_eq!(
            ChannelValueGrammar::Translation.validate(r"\x6c\x66", Some(version)),
            Valid
        );
        assert!(matches!(
            ChannelValueGrammar::Translation.validate("{", Some(version)),
            Invalid(_)
        ));
        assert_eq!(
            validate_create_modes(InvocationArguments::literals(&create).with_dialect(dialect)),
            LiteralArgumentValidation::Valid
        );
        assert_eq!(
            validate_postevent_modes(
                InvocationArguments::literals(&postevent).with_dialect(dialect)
            ),
            LiteralArgumentValidation::Valid
        );
        let malformed = ["create", "{", "handler"];
        assert!(matches!(
            validate_create_modes(InvocationArguments::literals(&malformed).with_dialect(dialect)),
            LiteralArgumentValidation::Invalid(_)
        ));
        if version < TclVersion::V9_0 {
            assert_eq!(
                ChannelValueGrammar::EofChar.validate(r"\x61", Some(version)),
                Valid
            );
        }
    }
}

#[test]
fn empty_channel_masks_preserve_create_release_and_postevent_constraints() {
    let create = ["create", "", "handler"];
    let postevent = ["postevent", "channel", ""];
    assert_eq!(
        validate_create_modes(InvocationArguments::literals(&create)),
        LiteralArgumentValidation::Abstain(LiteralValidationDecline::UnselectedGrammar)
    );
    assert!(matches!(
        validate_postevent_modes(InvocationArguments::literals(&postevent)),
        LiteralArgumentValidation::Invalid(_)
    ));
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        let create_result =
            validate_create_modes(InvocationArguments::literals(&create).with_dialect(dialect));
        let event_result = validate_postevent_modes(
            InvocationArguments::literals(&postevent).with_dialect(dialect),
        );
        if version == TclVersion::V8_4 {
            assert!(matches!(
                create_result,
                LiteralArgumentValidation::Abstain(_)
            ));
            assert!(matches!(
                event_result,
                LiteralArgumentValidation::Abstain(_)
            ));
        } else {
            assert_eq!(
                matches!(create_result, LiteralArgumentValidation::Valid),
                version >= TclVersion::V8_6
            );
            assert!(matches!(
                event_result,
                LiteralArgumentValidation::Invalid(_)
            ));
        }
    }
}

#[test]
fn directionless_translation_preserves_only_provable_value_errors() {
    use ChannelValueValidation::{Invalid, Unknown};
    for version in TclVersion::ALL {
        for value in ["bad", "bad bad", "bad lf"] {
            assert_eq!(
                ChannelValueGrammar::Translation.validate(value, Some(version)),
                Unknown
            );
            let words = ["configure", "channel", "-translation", value];
            assert!(matches!(
                validate_chan_configuration(
                    InvocationArguments::literals(&words)
                        .with_dialect(InvocationDialect::for_version(version))
                ),
                LiteralArgumentValidation::Abstain(_)
            ));
        }
        for value in ["", "bad bad bad", "{"] {
            assert!(matches!(
                ChannelValueGrammar::Translation.validate(value, Some(version)),
                Invalid(_)
            ));
        }
        for value in ["é", "é é", "\0", "x y z"] {
            assert!(matches!(
                ChannelValueGrammar::EofChar.validate(value, Some(version)),
                Invalid(_)
            ));
        }
    }
}

#[test]
fn channel_empty_masks_and_directionless_values_match_reference_tcl() {
    const SCRIPT: &str = r#"if {![llength [info commands chan]]} {
    puts {chan unavailable}
    exit
}
proc rc {method id args} {
    if {$method eq "initialize"} {return {initialize finalize watch read write}}
    if {$method eq "write"} {return [string length [lindex $args 0]]}
    return
}
set failed [catch {chan create {} rc} c]
puts [list empty-mode $failed]
if {!$failed} {close $c}
foreach mode {{} read write {read write}} {
    if {[catch {chan create $mode rc} c]} {continue}
    puts [list postevent-empty $mode [catch {chan postevent $c {}}]]
    set results {}
    foreach value {bad {bad bad} {bad bad bad} \{} {
        lappend results [catch {chan configure $c -translation $value}]
    }
    puts [list translation $mode $results]
    set results {}
    foreach value {bad é {é é} \u0000 {x y z} \{} {
        lappend results [catch {chan configure $c -eofchar $value}]
    }
    puts [list eofchar $mode $results]
    close $c
}
"#;
    for version in TclVersion::ALL {
        let Some(interpreter) = tcl_test_support::witness_tclsh(version) else {
            continue;
        };
        let output = tcl_test_support::run_script_fixture(&interpreter.path, SCRIPT, &[])
            .expect("channel value probe executes")
            .strict_text()
            .expect("caught channel value probe has no process error");
        if version == TclVersion::V8_4 {
            assert_eq!(output.trim(), "chan unavailable");
            continue;
        }
        let mut expected = vec![format!(
            "empty-mode {}",
            u8::from(version == TclVersion::V8_5)
        )];
        for mode in ["{}", "read", "write", "{read write}"] {
            if mode == "{}" && version == TclVersion::V8_5 {
                continue;
            }
            expected.push(format!("postevent-empty {mode} 1"));
            let translation = if mode == "{}" { "0 0 1 1" } else { "1 1 1 1" };
            expected.push(format!("translation {mode} {{{translation}}}"));
            let eofchar = if version < TclVersion::V9_0 {
                "0 1 1 1 1 1"
            } else {
                "1 1 1 1 1 0"
            };
            expected.push(format!("eofchar {mode} {{{eofchar}}}"));
        }
        assert_eq!(output, expected.join("\n"), "{version:?}");
    }
}
