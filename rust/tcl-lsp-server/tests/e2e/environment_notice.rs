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

//! The tool-environment notice, end to end over LSP.
//!
//! Opening a document that resolves to a bundled tool shell (`.xdc` is
//! Vivado's) makes the server tell the client, once per environment per
//! session, that the shell is a Tcl release plus library packages. Each test
//! drives a real server with a fake client whose capabilities, replies and
//! `XDG_*_HOME` directories it controls.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::common::{Lsp, scaled_timeout, unique_uri};

const ASK: &str = "window/showMessageRequest";
const SHOW_DOCUMENT: &str = "window/showDocument";
const SHOW_MESSAGE: &str = "window/showMessage";
const LEARN_MORE_URL: &str = "https://github.com/bitwisecook/tcl-lsp/blob/rust/docs/kcs/features/kcs-feature-tool-environments.md";
const VIVADO_TEXT: &str = "Xilinx Vivado (xilinx-eda-tcl) is Tcl 8.5 plus the vivado, sdc and \
     upf packages. Tool support is a set of library packages on a Tcl release, not a separate \
     dialect; your selection keeps working as before.";

/// How long a test waits for a message that must arrive.
const WAIT: Duration = Duration::from_secs(20);
/// How long a test waits before concluding a message is not coming.
const QUIET: Duration = Duration::from_millis(500);

static SCRATCH: AtomicU64 = AtomicU64::new(0);

/// A client that can render message actions and open a URL.
fn full_client() -> Value {
    json!({ "window": {
        "showMessage": { "messageActionItem": { "additionalPropertiesSupport": true } },
        "showDocument": { "support": true },
    }})
}

/// A client that can render message actions but not open a URL.
fn actions_only_client() -> Value {
    json!({ "window": {
        "showMessage": { "messageActionItem": { "additionalPropertiesSupport": true } },
    }})
}

/// A directory of the test's own, so state outlives one server.
fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-notice-{}-{name}-{}",
        std::process::id(),
        SCRATCH.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("create scratch directory");
    dir
}

/// Start a server whose editor answers `workspace/configuration` with `reply`,
/// waiting until `settle` is what the server reports as applied. `config_ini`
/// is written to the user config directory before the server first reads it.
fn start(
    reply: &Value,
    settle: &Value,
    capabilities: &Value,
    env: &[(&str, &str)],
    config_ini: Option<&str>,
) -> Lsp {
    let mut lsp = Lsp::spawn_with_env(reply.clone(), env);
    lsp.set_client_capabilities(capabilities.clone());
    if let Some(ini) = config_ini {
        let dir = lsp.xdg_root().join("config").join("tcl-lsp");
        std::fs::create_dir_all(&dir).expect("create the config directory");
        std::fs::write(dir.join("config.ini"), ini).expect("write config.ini");
    }
    lsp.initialize();
    lsp.settle_config(settle);
    lsp
}

/// A default-configured server with the given client capabilities.
fn start_default(capabilities: &Value) -> Lsp {
    let reply = json!({ "features": { "linkedEditingRange": true } });
    start(&reply, &reply, capabilities, &[], None)
}

/// Open a Vivado document (`.xdc`) under the plain `tcl` language id every
/// editor sends, and wait for its diagnostics.
fn open_vivado(lsp: &mut Lsp) -> String {
    let uri = unique_uri("xdc");
    lsp.open_ready(&uri, "create_clock -period 10 [get_ports clk]\n");
    uri
}

/// Every `window/showMessageRequest` the server has sent so far.
fn asks(lsp: &Lsp) -> Vec<Value> {
    requests(lsp, ASK)
}

fn requests(lsp: &Lsp, method: &str) -> Vec<Value> {
    lsp.server_requests()
        .into_iter()
        .filter(|request| request["method"] == method)
        .collect()
}

/// Every `window/showMessage` notification the server has sent so far.
fn plain_messages(lsp: &Lsp) -> Vec<Value> {
    lsp.notifications()
        .into_iter()
        .filter(|note| note["method"] == SHOW_MESSAGE)
        .collect()
}

fn message_of(message: &Value) -> &str {
    message["params"]["message"].as_str().unwrap_or_default()
}

fn action_titles(ask: &Value) -> Vec<&str> {
    ask["params"]["actions"]
        .as_array()
        .map(|actions| {
            actions
                .iter()
                .filter_map(|action| action["title"].as_str())
                .collect()
        })
        .unwrap_or_default()
}

/// The server's applied `tclLsp.notifications.environmentKind`.
fn notice_enabled(lsp: &mut Lsp) -> bool {
    lsp.effective_config("")["notifications_environment_kind"] == json!(true)
}

/// Wait until the server reports the setting as `want`.
fn wait_for_setting(lsp: &mut Lsp, want: bool) {
    let deadline = Instant::now() + scaled_timeout(WAIT);
    while notice_enabled(lsp) != want {
        assert!(
            Instant::now() < deadline,
            "notifications.environmentKind never became {want}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// `notices.ini` under `state`, once it lists `needle`.
fn saved_notices(state: &Path, needle: &str) -> String {
    let file = state.join("tcl-lsp").join("notices.ini");
    let deadline = Instant::now() + scaled_timeout(WAIT);
    loop {
        if let Ok(content) = std::fs::read_to_string(&file)
            && content.contains(needle)
        {
            return content;
        }
        assert!(
            Instant::now() < deadline,
            "{} never listed {needle}",
            file.display()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn effective_config_reports_the_setting_on_by_default() {
    let mut lsp = start_default(&json!({}));
    let cfg = lsp.effective_config("");
    assert_eq!(cfg["notifications_environment_kind"], json!(true), "{cfg}");
}

#[test]
fn opening_a_vivado_document_asks_once_with_two_actions() {
    let mut lsp = start_default(&full_client());
    open_vivado(&mut lsp);
    let ask = lsp.await_server_request(ASK, WAIT, 0);
    assert_eq!(ask["params"]["type"], json!(3), "an Info message: {ask}");
    assert_eq!(message_of(&ask), VIVADO_TEXT);
    assert_eq!(action_titles(&ask), ["Learn more", "Don't show again"]);
    assert_eq!(asks(&lsp).len(), 1, "exactly one request");
    assert!(
        plain_messages(&lsp).is_empty(),
        "the actions client is not also sent a plain message"
    );
}

#[test]
fn dont_show_again_is_saved_and_only_that_environment_stays_quiet() {
    let state = scratch_dir("dismiss");
    let state_env = [("XDG_STATE_HOME", state.to_str().expect("UTF-8 path"))];
    let reply = json!({ "features": { "linkedEditingRange": true } });
    {
        let mut lsp = start(&reply, &reply, &full_client(), &state_env, None);
        lsp.choose_message_action(Some("Don't show again"));
        open_vivado(&mut lsp);
        lsp.await_server_request(ASK, WAIT, 0);
        let saved = saved_notices(&state, "xilinx-eda-tcl");
        assert!(saved.contains("[dismissed]"), "{saved}");
        assert!(
            saved.contains("environment-kind = xilinx-eda-tcl\n"),
            "{saved}"
        );
    }

    // A fresh session reads that state: Vivado stays quiet, Synopsys still asks.
    let mut lsp = start(&reply, &reply, &full_client(), &state_env, None);
    open_vivado(&mut lsp);
    let synopsys = unique_uri("tcl");
    lsp.open_ready_lang(&synopsys, "create_clock -period 10 clk\n", "tcl-synopsys");
    let ask = lsp.await_server_request(ASK, WAIT, 0);
    assert!(message_of(&ask).contains("(synopsys-eda-tcl)"), "{ask}");
    let all = asks(&lsp);
    assert_eq!(all.len(), 1, "Vivado was dismissed: {all:?}");
    let saved = saved_notices(&state, "xilinx-eda-tcl");
    assert!(!saved.contains("synopsys-eda-tcl"), "{saved}");
    let _ = std::fs::remove_dir_all(state);
}

#[test]
fn an_unreadable_state_file_is_one_warning_and_the_notice_still_asks() {
    let state = scratch_dir("unreadable");
    let file = state.join("tcl-lsp").join("notices.ini");
    std::fs::create_dir_all(file.parent().expect("a parent directory")).expect("create state");
    std::fs::write(&file, [0xff, 0xfe, 0x00, 0x9f]).expect("write a file that is not UTF-8");
    let state_env = [("XDG_STATE_HOME", state.to_str().expect("UTF-8 path"))];
    let reply = json!({ "features": { "linkedEditingRange": true } });

    let mut lsp = start(&reply, &reply, &full_client(), &state_env, None);
    let warning = lsp.await_log(&["could not read", "notices.ini"], WAIT, 0);
    assert!(warning.contains("may be shown again"), "{warning}");
    open_vivado(&mut lsp);
    lsp.await_server_request(ASK, WAIT, 0);
    let warnings = lsp
        .notifications()
        .into_iter()
        .filter(|note| {
            note["method"] == "window/logMessage" && message_of(note).contains("could not read")
        })
        .count();
    assert_eq!(warnings, 1, "at most one warning");
    let _ = std::fs::remove_dir_all(state);
}

#[test]
fn a_second_open_in_the_same_session_sends_nothing_more() {
    let mut lsp = start_default(&full_client());
    let first = open_vivado(&mut lsp);
    lsp.await_server_request(ASK, WAIT, 0);
    open_vivado(&mut lsp);
    lsp.close_document(&first);
    open_vivado(&mut lsp);
    assert_eq!(asks(&lsp).len(), 1);
    assert!(plain_messages(&lsp).is_empty());
}

#[test]
fn a_client_without_action_items_gets_a_plain_message() {
    for capabilities in [
        json!({}),
        json!({ "window": { "showMessage": {} } }),
        json!({ "window": { "showDocument": { "support": true } } }),
    ] {
        let mut lsp = start_default(&capabilities);
        open_vivado(&mut lsp);
        let note = lsp.await_notification(SHOW_MESSAGE, WAIT);
        assert_eq!(note["params"]["type"], json!(3), "{capabilities}: {note}");
        assert_eq!(message_of(&note), VIVADO_TEXT, "{capabilities}");
        assert!(asks(&lsp).is_empty(), "{capabilities}");
        assert_eq!(plain_messages(&lsp).len(), 1, "{capabilities}");
    }
}

#[test]
fn the_editor_setting_switches_the_notice_off_and_back_on_live() {
    let off = json!({ "notifications": { "environmentKind": false } });
    let mut lsp = start(&off, &off, &full_client(), &[], None);
    open_vivado(&mut lsp);
    assert!(
        lsp.try_await_server_request(ASK, QUIET, 0).is_none(),
        "the setting is off"
    );
    assert!(plain_messages(&lsp).is_empty());

    // Turning it on applies to the next document, with no restart, and the
    // session's one showing was not spent while it was off.
    lsp.apply_configuration(json!({ "notifications": { "environmentKind": true } }));
    wait_for_setting(&mut lsp, true);
    open_vivado(&mut lsp);
    let ask = lsp.await_server_request(ASK, WAIT, 0);
    assert_eq!(message_of(&ask), VIVADO_TEXT);
}

/// A document restored at start-up opens while the first configuration pull is
/// still in flight; the setting it carries must be applied before the notice
/// decides.
#[test]
fn a_document_opened_before_the_first_pull_lands_still_honours_the_setting() {
    let off = json!({ "notifications": { "environmentKind": false } });
    let mut lsp = Lsp::spawn_with_env(off.clone(), &[]);
    lsp.set_client_capabilities(full_client());
    lsp.set_configuration_reply_delay(Duration::from_millis(300));
    lsp.initialize();
    lsp.open_document(
        &unique_uri("xdc"),
        "create_clock -period 10 [get_ports clk]\n",
    );
    assert!(
        lsp.try_await_server_request(ASK, Duration::from_millis(900), 0)
            .is_none(),
        "the notice went out before the setting was applied"
    );
    lsp.settle_config(&off);
    assert!(asks(&lsp).is_empty());
    assert!(plain_messages(&lsp).is_empty());
}

#[test]
fn a_flat_configuration_push_switches_the_notice_off() {
    let mut lsp = start_default(&full_client());
    lsp.notify(
        "workspace/didChangeConfiguration",
        json!({ "settings": { "tclLsp.notifications.environmentKind": false } }),
    );
    wait_for_setting(&mut lsp, false);
    open_vivado(&mut lsp);
    assert!(lsp.try_await_server_request(ASK, QUIET, 0).is_none());
}

#[test]
fn the_config_file_switches_the_notice_off_for_an_editor_that_sends_nothing() {
    let off = json!({ "notifications": { "environmentKind": false } });
    let mut lsp = start(
        &json!({}),
        &off,
        &full_client(),
        &[],
        Some("[notifications]\nenvironment_kind = false\n"),
    );
    let cfg = lsp.effective_config("");
    assert_eq!(cfg["notifications_environment_kind"], json!(false), "{cfg}");
    open_vivado(&mut lsp);
    assert!(
        lsp.try_await_server_request(ASK, QUIET, 0).is_none(),
        "config.ini turned it off"
    );
    assert!(plain_messages(&lsp).is_empty());
}

#[test]
fn an_editor_that_sends_the_setting_outranks_the_config_file() {
    let off = json!({ "notifications": { "environmentKind": false } });
    let mut lsp = start(
        &off,
        &off,
        &full_client(),
        &[],
        Some("[notifications]\nenvironment_kind = false\n"),
    );
    lsp.apply_configuration(json!({ "notifications": { "environmentKind": true } }));
    wait_for_setting(&mut lsp, true);
    open_vivado(&mut lsp);
    lsp.await_server_request(ASK, WAIT, 0);
}

#[test]
fn a_tk_document_gets_no_notice() {
    let mut lsp = start_default(&full_client());
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "# tcl-dialect: tk\npack [button .b -text hi]\n");
    let cfg = lsp.effective_config(&uri);
    assert_eq!(cfg["dialect_id"], json!("tk"), "{cfg}");
    assert_eq!(cfg["dialect_kind"], json!("packages"), "{cfg}");
    assert_eq!(cfg["dialect_provenance"], json!("built-in"), "{cfg}");

    // The first request the server ever makes is Vivado's, not Tk's.
    open_vivado(&mut lsp);
    let ask = lsp.await_server_request(ASK, WAIT, 0);
    assert_eq!(message_of(&ask), VIVADO_TEXT);
    assert_eq!(asks(&lsp).len(), 1);
}

#[test]
fn learn_more_opens_the_note_when_the_client_can_show_a_document() {
    let mut lsp = start_default(&full_client());
    lsp.choose_message_action(Some("Learn more"));
    open_vivado(&mut lsp);
    let show = lsp.await_server_request(SHOW_DOCUMENT, WAIT, 0);
    assert_eq!(show["params"]["uri"], json!(LEARN_MORE_URL), "{show}");
    assert_eq!(show["params"]["external"], json!(true), "{show}");
}

#[test]
fn learn_more_puts_the_url_on_the_log_when_the_client_cannot_show_a_document() {
    let mut lsp = start_default(&actions_only_client());
    lsp.choose_message_action(Some("Learn more"));
    open_vivado(&mut lsp);
    let logged = lsp.await_log(&[LEARN_MORE_URL], WAIT, 0);
    assert!(logged.contains("tool environments"), "{logged}");
    assert!(
        requests(&lsp, SHOW_DOCUMENT).is_empty(),
        "no document is requested from a client that did not advertise it"
    );
}
