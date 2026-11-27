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
use crate::CommandRegistry;
use tcl_dialect::model::{Family, SurfaceQuery};

#[test]
fn channel_protocols_preserve_required_methods_and_optional_signatures() {
    let reflected = ChannelCallbackProtocol::Reflected;
    let required: Vec<_> = reflected
        .methods_for(TclVersion::V8_6)
        .filter(|method| method.requirement == MethodRequirement::Always)
        .map(|method| method.name)
        .collect();
    assert_eq!(required, ["initialize", "finalize", "watch"]);
    assert!(
        !reflected
            .methods_for(TclVersion::V8_6)
            .any(|method| method.name == "truncate")
    );
    assert!(
        reflected
            .methods_for(TclVersion::V9_0)
            .any(|method| method.name == "truncate")
    );
    let seek = reflected
        .methods()
        .iter()
        .find(|method| method.name == "seek")
        .unwrap();
    assert_eq!(seek.appended_words, ["seek", "channel", "offset", "base"]);
    let cget = reflected
        .methods()
        .iter()
        .find(|method| method.name == "cget")
        .unwrap();
    assert_eq!(cget.requires, ["cgetall"]);
    let transform = ChannelCallbackProtocol::Transform;
    assert!(transform.needs_accessible_direction());
    assert!(!reflected.needs_accessible_direction());
    assert!(transform.methods_for(TclVersion::V8_5).next().is_none());
    assert!(
        transform
            .methods_for(TclVersion::V8_6)
            .any(|method| method.name == "limit?")
    );
    assert_eq!(
        reflected
            .appended_arity()
            .exact_counts()
            .unwrap()
            .collect::<Vec<_>>(),
        [2, 3]
    );
    assert_eq!(
        transform
            .appended_arity()
            .exact_counts()
            .unwrap()
            .collect::<Vec<_>>(),
        [2, 3]
    );
    assert_eq!(
        ChannelCallbackProtocol::Copy
            .appended_arity()
            .exact_counts()
            .unwrap()
            .collect::<Vec<_>>(),
        [1, 2]
    );
}

#[test]
fn callback_protocols_follow_selected_prefix_operands_and_offsets() {
    let registry = CommandRegistry::build_default();
    let surface = Some(SurfaceQuery::core(Family::Tcl, "9.0"));
    for (name, arguments, index, protocol) in [
        (
            "chan",
            vec!["create", "read", "handler"],
            2,
            ChannelCallbackProtocol::Reflected,
        ),
        (
            "chan",
            vec!["push", "channel", "handler"],
            2,
            ChannelCallbackProtocol::Transform,
        ),
        (
            "socket",
            vec!["-server", "handler", "0"],
            1,
            ChannelCallbackProtocol::SocketServer,
        ),
        (
            "fcopy",
            vec!["in", "out", "-command", "handler"],
            3,
            ChannelCallbackProtocol::Copy,
        ),
        (
            "chan",
            vec!["copy", "in", "out", "-command", "handler"],
            4,
            ChannelCallbackProtocol::Copy,
        ),
    ] {
        let invocation = registry
            .resolve_invocation(name, &arguments, surface)
            .unwrap();
        assert_eq!(
            invocation.authored_source_callback_protocol(index),
            Some(protocol),
            "{name} {arguments:?}"
        );
        assert_eq!(invocation.authored_source_callback_protocol(0), None);
        let prefixes = invocation
            .authored_source_command_prefix_arguments()
            .unwrap();
        assert!(prefixes.contains(&(index, protocol.appended_arity())));
    }
}

fn oracle_fixture(interpreter: &std::path::Path, script: &str) -> String {
    tcl_test_support::run_script_fixture(
        interpreter,
        &format!("cd $::oracle_fixture_root\n{script}"),
        &[],
    )
    .expect("channel fixture executes")
    .strict_text()
    .expect("caught channel fixture has no process error")
}

fn log_line(output: &str) -> &str {
    output
        .lines()
        .find_map(|line| line.strip_prefix("log: "))
        .expect("channel fixture emits its protocol log")
}

fn check_reflected_calls(log: &str, separator: Option<char>, version: TclVersion) {
    let dialect = tcl_native_model::InvocationDialect::for_version(version);
    let calls: Vec<_> = separator.map_or_else(
        || {
            dialect
                .read_list(log)
                .unwrap()
                .into_iter()
                .map(std::borrow::Cow::into_owned)
                .collect()
        },
        |separator| log.split(separator).map(ToOwned::to_owned).collect(),
    );
    for call in calls {
        let words = dialect.read_list(&call).unwrap();
        let method = ChannelCallbackProtocol::Reflected
            .methods_for(version)
            .find(|method| method.name == words[0])
            .expect("observed method is in the protocol");
        // The retained fixtures omit the channel id from their log.
        assert_eq!(
            usize::from(method.appended_count()),
            words.len() + 1,
            "{call}"
        );
    }
}

#[test]
fn channel_protocol_c06_c09_orders_and_word_counts_match_reference_tcl() {
    const READ: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/work/name-handling/handover/oo-dispatch-probe/chan/c06-refchan-protocol.tcl"
    ));
    const FULL: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/work/name-handling/handover/oo-dispatch-probe/chan/c07-refchan-full.tcl"
    ));
    const ERRORS: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/work/name-handling/handover/oo-dispatch-probe/chan/c08-refchan-errors.tcl"
    ));
    const TRANSFORM_PROBE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/work/name-handling/handover/oo-dispatch-probe/chan/c09-transform.tcl"
    ));
    for version in TclVersion::ALL {
        let Some(interpreter) = tcl_test_support::witness_tclsh(version) else {
            continue;
        };
        let read = oracle_fixture(&interpreter.path, READ);
        if version == TclVersion::V8_4 {
            assert!(read.contains("create: ERR invalid command name \"chan\""));
            continue;
        }
        assert_eq!(log_line(&read), "{initialize read} {read 4096} finalize");
        check_reflected_calls(log_line(&read), None, version);
        let full = oracle_fixture(&interpreter.path, FULL);
        if version < TclVersion::V9_0 {
            assert!(full.contains("returned bad method \"truncate\""));
            assert_eq!(log_line(&full), "initialize {read write}");
        } else {
            assert_eq!(
                log_line(&full),
                "initialize {read write}|seek 0 current|seek 10 start|seek 0 current|write abc|cget -foo|configure -foo bar|blocking 0|truncate 5|cgetall|finalize"
            );
            check_reflected_calls(log_line(&full), Some('|'), version);
        }
        let errors = oracle_fixture(&interpreter.path, ERRORS);
        assert!(errors.contains("lacks a \"read\" method"));
        assert!(errors.contains("does not support all required methods"));
        if version >= TclVersion::V8_6 {
            let transform = oracle_fixture(&interpreter.path, TRANSFORM_PROBE);
            assert_eq!(
                log_line(&transform),
                "initialize clear write drain flush finalize initialize read drain flush"
            );
            for method in log_line(&transform).split(' ') {
                assert!(
                    ChannelCallbackProtocol::Transform
                        .methods_for(version)
                        .any(|row| row.name == method)
                );
            }
        }
    }
}

#[test]
fn protocol_descriptions_list_the_release_methods_and_widest_call() {
    let reflected = ChannelCallbackProtocol::Reflected;
    let card = reflected.describe(TclVersion::V8_6);
    assert!(
        card.contains("- `initialize channel mode` (required)"),
        "{card}"
    );
    assert!(
        card.contains("- `read channel count` (required to read)"),
        "{card}"
    );
    assert!(!card.contains("truncate"), "{card}");
    assert!(
        reflected
            .describe(TclVersion::V9_0)
            .contains("- `truncate channel length`")
    );
    assert!(
        ChannelCallbackProtocol::SocketServer
            .describe(TclVersion::V8_6)
            .contains("`handler channel address port`")
    );
    for (protocol, widest) in [
        (ChannelCallbackProtocol::Reflected, 4),
        (ChannelCallbackProtocol::Transform, 3),
        (ChannelCallbackProtocol::SocketServer, 3),
        (ChannelCallbackProtocol::Copy, 2),
    ] {
        assert_eq!(protocol.max_appended_count(), widest, "{}", protocol.name());
    }
    assert!(reflected.dispatches_methods());
    assert!(!ChannelCallbackProtocol::SocketServer.dispatches_methods());
}

#[test]
fn invocation_facts_name_the_protocol_of_each_prefix_operand() {
    let registry = CommandRegistry::build_default();
    let surface = Some(SurfaceQuery::core(Family::Tcl, "9.0"));
    let protocol = |name: &str, arguments: &[&str], index: usize| {
        registry
            .resolve_invocation(name, arguments, surface)
            .unwrap()
            .facts()
            .callback_protocol(index)
    };
    assert_eq!(
        protocol("chan", &["create", "read", "h"], 2),
        Some(ChannelCallbackProtocol::Reflected)
    );
    assert_eq!(
        protocol("chan", &["push", "ch", "h"], 2),
        Some(ChannelCallbackProtocol::Transform)
    );
    assert_eq!(
        protocol("socket", &["-server", "h", "0"], 1),
        Some(ChannelCallbackProtocol::SocketServer)
    );
    assert_eq!(protocol("chan", &["configure", "ch", "-blocking"], 2), None);
}
