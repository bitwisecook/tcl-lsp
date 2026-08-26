//! Generate the iRule-test framework's registry-backed Tcl data assets.
//!
//! The framework is intentionally Tcl so it can run in a stock `tclsh` as
//! well as the in-process VM, but its event and command surfaces belong to
//! `tcl-registry`.  This module projects those owners into the two framework
//! assets that need them:
//!
//! - `rust/tcl-irule-test/tcl/_event_data.tcl` — event order, flow chains,
//!   and multiplicity sets from [`tcl_registry::events::EventRegistry`];
//! - `rust/tcl-irule-test/tcl/_mock_stubs.tcl` — generic fallback mock actions
//!   for command specs without a hand-written Tcl mock.
//!
//! Run `cargo xtask gen-irule-test-data` to update the checked-in files, or
//! pass `--check` to make stale generated data fail the Rust check gate.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result};
use tcl_registry::CommandRegistry;
use tcl_registry::events::{EventRegistry, FlowChain, OrderEntry};
use tcl_registry::model::ResolvedContext;

use crate::util::{DISABLED_SENTINEL, license_banner, repo_root};

const EVENT_DATA_PATH: &str = "rust/tcl-irule-test/tcl/_event_data.tcl";
const MOCK_STUBS_PATH: &str = "rust/tcl-irule-test/tcl/_mock_stubs.tcl";
const HAND_WRITTEN_MOCKS_PATH: &str = "rust/tcl-irule-test/tcl/command_mocks.tcl";
const GENERATED_BY: &str = "cargo xtask gen-irule-test-data";
/// The canonical id of the environment the iRules harness simulates — the
/// one dialect name this generator resolves through the ingress seam.
const IRULES_DIALECT: &str = "f5-irules";

/// A generic stub-table entry.  The key is exactly the tail returned by the
/// Tcl framework's `_mock_proc_name`; the values are the decision-log
/// category and action.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MockStubEntry {
    mock_name: String,
    category: String,
    action: String,
}

fn tcl_list(words: &[&str]) -> String {
    if words.is_empty() {
        "{}".to_owned()
    } else {
        format!("{{{}}}", words.join(" "))
    }
}

fn render_event_data(order: &[OrderEntry], chains: &[FlowChain], events: &EventRegistry) -> String {
    let mut out = license_banner("#");
    let _ = write!(
        out,
        "# _event_data.tcl -- AUTO-GENERATED from tcl-registry by `{GENERATED_BY}`\n#\n# DO NOT EDIT. Regenerate with:\n#   {GENERATED_BY}\n#\n# Source owner: tcl-registry::events::EventRegistry.\n\n"
    );
    out.push_str("namespace eval ::orch {\n\n");
    out.push_str("    # Canonical event firing order: {event_name profile_gates}.\n");
    out.push_str("    variable MASTER_ORDER {\n");
    for entry in order {
        let _ = writeln!(
            out,
            "        {{{:<40} {}}}",
            entry.event,
            tcl_list(entry.profile_gates)
        );
    }
    out.push_str("    }\n\n");
    out.push_str("    variable _event_index\n    array set _event_index {}\n");
    out.push_str("    variable _idx 0\n    foreach _entry $MASTER_ORDER {\n");
    out.push_str("        set _event_index([lindex $_entry 0]) $_idx\n        incr _idx\n    }\n");
    out.push_str("    unset _idx _entry\n\n");

    let mut once: Vec<_> = events
        .all_event_names()
        .into_iter()
        .filter(|event| events.is_once_per_connection(event))
        .collect();
    once.sort_unstable();
    out.push_str("    # Events that fire at most once per connection.\n");
    out.push_str("    variable ONCE_PER_CONNECTION {\n");
    for event in once {
        let _ = writeln!(out, "        {event}");
    }
    out.push_str("    }\n\n");

    let mut per_request: Vec<_> = events
        .all_event_names()
        .into_iter()
        .filter(|event| events.is_per_request(event))
        .collect();
    per_request.sort_unstable();
    out.push_str("    # Events that fire once per request/transaction.\n");
    out.push_str("    variable PER_REQUEST {\n");
    for event in per_request {
        let _ = writeln!(out, "        {event}");
    }
    out.push_str("    }\n\n");

    out.push_str("    # Pre-built registry-owned flow chains.\n");
    out.push_str("    variable FLOW_CHAINS\n    array set FLOW_CHAINS {}\n\n");
    let mut chains: Vec<_> = chains.iter().collect();
    chains.sort_unstable_by_key(|chain| chain.chain_id);
    for chain in chains {
        let _ = writeln!(out, "    set FLOW_CHAINS({}) {{", chain.chain_id);
        let _ = writeln!(out, "        profiles {}", tcl_list(chain.profiles));
        out.push_str("        steps {\n");
        for step in &chain.steps {
            let _ = writeln!(out, "            {{{} {}}}", step.event, step.phase);
        }
        out.push_str("        }\n    }\n\n");
    }
    out.push_str("}\n");
    out
}

fn mock_proc_name(command: &str) -> String {
    let command = command.strip_prefix("::").unwrap_or(command);
    if let Some((namespace, _)) = command.split_once("::") {
        let action = command.rsplit("::").next().unwrap_or(command);
        format!(
            "{}_{}",
            safe_mock_piece(namespace.to_ascii_lowercase()),
            safe_mock_piece(action)
        )
    } else {
        format!("cmd_{}", safe_mock_piece(command))
    }
}

fn safe_mock_piece(piece: impl AsRef<str>) -> String {
    piece
        .as_ref()
        .chars()
        .map(|ch| match ch {
            '-' | '.' => '_',
            _ => ch,
        })
        .collect()
}

fn handwritten_mock_names(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(|line| line.strip_prefix("    proc "))
        .filter_map(|rest| rest.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}

fn stub_entries(
    registry: &CommandRegistry,
    context: &ResolvedContext,
    handwritten: &BTreeSet<String>,
) -> Vec<MockStubEntry> {
    let mut entries = BTreeSet::new();
    for command in registry.command_names() {
        // The generation's store intentionally begins with the broad
        // default registry so package and cross-dialect descriptions remain
        // available to analysis.  The Tcl harness, however, may only
        // register commands callable in its F5 iRules environment.  Resolve
        // through the environment's own context — the replacement for
        // `ProfileQueries::resolve_command` (ledger row F1's assistance
        // half) — instead of treating registry membership as visibility
        // (notably excludes Tcllib, Tk, `file`, and `exec`).
        if context.resolve_spec(registry, command).is_none() {
            continue;
        }
        if command == DISABLED_SENTINEL || command.starts_with("::tcl::mathop::") {
            continue;
        }
        let mock_name = mock_proc_name(command);
        if handwritten.contains(&mock_name) {
            continue;
        }
        let (category, action) = if let Some((namespace, _)) = command
            .strip_prefix("::")
            .unwrap_or(command)
            .split_once("::")
        {
            (
                namespace.to_ascii_lowercase(),
                command
                    .strip_prefix("::")
                    .unwrap_or(command)
                    .rsplit("::")
                    .next()
                    .unwrap_or(command)
                    .to_owned(),
            )
        } else {
            ("toplevel".to_owned(), command.to_owned())
        };
        entries.insert(MockStubEntry {
            mock_name,
            category,
            action,
        });
    }
    entries.into_iter().collect()
}

fn render_mock_stubs(entries: &[MockStubEntry]) -> String {
    let mut out = license_banner("#");
    let _ = write!(
        out,
        "# _mock_stubs.tcl -- AUTO-GENERATED from tcl-registry by `{GENERATED_BY}`\n#\n# DO NOT EDIT. Regenerate with:\n#   {GENERATED_BY}\n#\n# The action table covers registry commands that do not have a hand-written\n# mock proc in command_mocks.tcl.\n\n"
    );
    out.push_str("namespace eval ::itest::cmd {\n\n");
    out.push_str("    # One generic fallback for registry-only commands.\n");
    out.push_str("    proc _stub {cat action args} {\n");
    out.push_str(
        "        ::itest::log_decision $cat $action $args\n        return \"\"\n    }\n\n",
    );
    out.push_str("    # Map: _mock_proc_name tail -> {decision category action}.\n");
    out.push_str("    variable _stub_actions\n    array set _stub_actions {\n");
    for entry in entries {
        let _ = writeln!(
            out,
            "    {} {{{} {}}}",
            entry.mock_name, entry.category, entry.action
        );
    }
    out.push_str("    }\n}\n\n");
    let _ = writeln!(out, "# Total stub actions generated: {}", entries.len());
    out
}

fn generated_files() -> Result<Vec<(&'static str, String)>> {
    let root = repo_root();
    let events = EventRegistry::build();
    // The harness simulates the F5 iRules environment, not every optional
    // package that `CommandRegistry::build_default` happens to preload.  The
    // resolved environment's registry generation is the same store
    // LiveSession uses (ledger row B10: this generator is the replacement
    // for the frozen `_registry_data.tcl` subtraction, so its own ingress
    // goes through the one seam).
    let registry = crate::environment::store_for_dialect(IRULES_DIALECT);
    let irules_context = crate::environment::context_for_dialect(IRULES_DIALECT);
    let mocks = fs::read_to_string(root.join(HAND_WRITTEN_MOCKS_PATH))
        .with_context(|| format!("reading {HAND_WRITTEN_MOCKS_PATH}"))?;
    Ok(vec![
        (
            EVENT_DATA_PATH,
            render_event_data(events.master_order(), events.flow_chains(), &events),
        ),
        (
            MOCK_STUBS_PATH,
            render_mock_stubs(&stub_entries(
                registry,
                irules_context,
                &handwritten_mock_names(&mocks),
            )),
        ),
    ])
}

/// Update or verify the two registry-backed iRule-test Tcl assets.
pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let mut drift = Vec::new();
    for (rel, content) in generated_files()? {
        let path = root.join(rel);
        if check {
            let current =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            if current != content {
                drift.push(rel);
            }
        } else {
            fs::write(&path, content).with_context(|| format!("writing {}", path.display()))?;
            eprintln!("wrote {rel}");
        }
    }
    if check && !drift.is_empty() {
        eprintln!(
            "{} generated iRule-test data file(s) are stale — run `{GENERATED_BY}`:",
            drift.len()
        );
        for rel in drift {
            eprintln!("  - {rel}");
        }
        return Ok(ExitCode::from(1));
    }
    if check {
        eprintln!("OK: generated iRule-test data is in sync with tcl-registry.");
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_renderer_changes_when_registry_owned_event_data_changes() {
        let events = EventRegistry::build();
        let original = render_event_data(events.master_order(), events.flow_chains(), &events);
        let changed = [OrderEntry {
            event: "REGISTRY_MUTATION",
            profile_gates: &["TEST"],
        }];
        let mutated = render_event_data(&changed, events.flow_chains(), &events);
        assert_ne!(original, mutated);
        assert!(mutated.contains("REGISTRY_MUTATION"));
    }

    #[test]
    fn mock_renderer_changes_when_registry_owned_stub_data_changes() {
        let original = render_mock_stubs(&[MockStubEntry {
            mock_name: "http_header".to_owned(),
            category: "http".to_owned(),
            action: "header".to_owned(),
        }]);
        let mutated = render_mock_stubs(&[MockStubEntry {
            mock_name: "http_header".to_owned(),
            category: "http".to_owned(),
            action: "uri".to_owned(),
        }]);
        assert_ne!(original, mutated);
        assert!(mutated.contains("{http uri}"));
    }

    #[test]
    fn mock_name_follows_tcl_registration_convention() {
        assert_eq!(mock_proc_name("HTTP::header"), "http_header");
        assert_eq!(mock_proc_name("traffic-group"), "cmd_traffic_group");
        assert_eq!(mock_proc_name("::tcl::path"), "tcl_path");
    }

    #[test]
    fn mock_stubs_only_project_commands_visible_to_f5_irules() {
        let registry = crate::environment::store_for_dialect(IRULES_DIALECT);
        let context = crate::environment::context_for_dialect(IRULES_DIALECT);
        let entries = stub_entries(registry, context, &BTreeSet::new());
        let names: BTreeSet<_> = entries
            .iter()
            .map(|entry| entry.mock_name.as_str())
            .collect();

        // These are in the broad registry but unavailable to the iRules
        // profile; adding one to a default package must not leak a stub.
        for unavailable in [
            "aes_Decrypt",
            "base64_decode",
            "fileutil_cat",
            "ttk_button",
            "cmd_file",
            "cmd_exec",
        ] {
            assert!(
                !names.contains(unavailable),
                "unavailable command leaked into the iRules stub table: {unavailable}"
            );
        }

        // Registry-only iRules commands remain available for generic stub
        // dispatch; hand-written mocks are filtered separately by the caller.
        assert!(names.contains("access_session"));
        assert!(names.contains("aaa_acct_result"));
    }

    #[test]
    fn committed_files_match_registry_generation() {
        let root = repo_root();
        for (rel, content) in generated_files().expect("generate iRule-test data") {
            let current =
                fs::read_to_string(root.join(rel)).expect("read committed generated file");
            assert_eq!(current, content, "{rel} is stale — run `{GENERATED_BY}`");
        }
    }
}
