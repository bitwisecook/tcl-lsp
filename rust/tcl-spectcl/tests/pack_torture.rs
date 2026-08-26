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

//! Torture tests for `.tclspec` loading — the stability half of the pack
//! pipeline, at the altitude where a defect is a panic rather than a squiggle.
//!
//! `docs/design/spec-packs.md` makes two promises this file is the negative
//! proof of:
//!
//! * **"a spec must never be able to take the LSP down"** — no input, however
//!   malformed, may panic, hang, or overflow the stack inside the loader.
//!   `Backend::reload_spec_packs` catches a loader panic and keeps the
//!   previous pack set, but a caught panic is still a workspace whose packs
//!   silently stop updating, so the loader is held to never panicking at all.
//! * **"unknown property words … are dropped with a logged notice; the rest of
//!   the spec loads"** — degradation is per-declaration, so one bad row must
//!   not cost the file, and one bad file must not cost the pack set.
//!
//! The battery below walks the grammar from the byte level (empty file, BOM,
//! CRLF, NUL, non-UTF-8) up through statement structure (truncation, brace
//! imbalance, absurd sizes) to semantics (lifecycle ordering, vocabulary
//! versions, dialect names, duplicate declarations, extension ownership), and
//! finishes on scale and on adversarial spec *content*.
//!
//! Everything here runs against the real loader with no stubs. The editor-side
//! half — that none of this wedges the extension host — is
//! `editors/vscode/src/test/specPackTorture.test.ts`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tcl_compiler::analyser::Analyser;
use tcl_spectcl::discovery::{DiscoveryOptions, Origin, PackFile, Tier};
use tcl_spectcl::loader::load_pack;
use tcl_spectcl::pack::{self, PackSet};

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// A private on-disk workspace for one test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tcl-spectcl-torture-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A `PackFile` naming `path` in the workspace tier, as discovery would.
fn workspace_file(path: &Path) -> PackFile {
    PackFile {
        path: path.to_path_buf(),
        tier: Tier::Workspace,
        origin: Origin::DotDir,
    }
}

/// Load the given files as one pack set, with both non-workspace tiers pinned
/// away so a developer's own `~/.config/tcl-lsp/specs` cannot change an answer.
fn load_files(paths: &[PathBuf]) -> PackSet {
    let files: Vec<PackFile> = paths.iter().map(|p| workspace_file(p)).collect();
    pack::load(&files)
}

/// Discover and load `root` as a workspace, with the user and bundled tiers
/// pinned at directories that do not exist.
fn load_workspace(root: &Path) -> PackSet {
    let files = tcl_spectcl::discover(&DiscoveryOptions {
        workspace_roots: vec![root.to_path_buf()],
        user_dir: Some(root.join("no-user-tier")),
        bundled_dir: Some(root.join("no-bundled-tier")),
        ..DiscoveryOptions::default()
    });
    pack::load(&files)
}

/// Write `body` to `<root>/.tcl-lsp/<name>` and return the path.
fn write_pack(root: &Path, name: &str, body: &str) -> PathBuf {
    let dir = root.join(".tcl-lsp");
    std::fs::create_dir_all(&dir).expect("pack dir");
    let path = dir.join(name);
    std::fs::write(&path, body).expect("write pack");
    path
}

/// Every notice message in the set, joined — for `contains` assertions that do
/// not care which notice carried the phrase.
fn notice_text(set: &PackSet) -> String {
    set.notices
        .iter()
        .map(|n| {
            format!(
                "{}:{} [{}] {}",
                n.path.display(),
                n.line,
                n.context,
                n.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Run `body` on a thread with a small-ish stack, asserting it finishes within
/// `budget`.
///
/// Two hazards in one harness: a loader that recurses per brace level
/// overflows a stack, and a loader that backtracks per byte hangs. A plain
/// `#[test]` catches neither — an overflow aborts the whole process (so the
/// failure is unattributable) and a hang stalls CI until the job timeout.
/// Running on a spawned thread bounds the damage of the first and lets the
/// second be reported as a test failure naming the input.
fn within<T: Send + 'static>(
    label: &str,
    budget: Duration,
    body: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = std::thread::Builder::new()
        // Deliberately modest: the default 8 MiB main stack hides a
        // per-brace-level recursion that a worker thread — which is where
        // `reload_spec_packs` actually parses — would hit first.
        .stack_size(2 * 1024 * 1024)
        .name(format!("torture-{label}"))
        .spawn(move || {
            let value = body();
            // Send before the value is dropped, so a slow drop is timed too.
            tx.send(()).ok();
            value
        })
        .expect("spawn torture thread");
    match rx.recv_timeout(budget) {
        Ok(()) => handle.join().expect("torture body must not panic"),
        Err(why) => {
            panic!(
                "`{label}` did not finish within {budget:?} ({why}) — the loader hung or crashed"
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Axis 1a — the byte level
// ---------------------------------------------------------------------------

/// Every degenerate byte sequence a file can hold loads to *something* and
/// never panics.
///
/// The assertion is deliberately weak on content and absolute on liveness: for
/// most of these there is no useful pack to recover, and pinning the exact
/// notice text would make the test a change-detector. What must hold is that
/// `load_pack` returns.
#[test]
fn degenerate_byte_sequences_never_panic() {
    let cases: Vec<(&str, String)> = vec![
        ("empty", String::new()),
        ("only-whitespace", "   \n\t\n  ".to_owned()),
        ("single-open-brace", "{".to_owned()),
        ("single-close-brace", "}".to_owned()),
        ("only-braces", "{}{}{}".to_owned()),
        (
            "bom-then-speclib",
            "\u{feff}speclib demo 1.1 {\n}\n".to_owned(),
        ),
        (
            "crlf",
            "speclib demo 1.1 {\r\n  command a { arity 1 }\r\n}\r\n".to_owned(),
        ),
        (
            "lone-cr",
            "speclib demo 1.1 {\r  command a { arity 1 }\r}\r".to_owned(),
        ),
        (
            "nul-bytes",
            "speclib demo 1.1 {\0command a\0{ arity 1 }\n}".to_owned(),
        ),
        ("comment-only", "# nothing but a comment\n".to_owned()),
        (
            "unterminated-quote",
            "speclib demo 1.1 {\n  command \"a { arity 1 }\n}".to_owned(),
        ),
        (
            "unterminated-bracket",
            "speclib demo 1.1 {\n  command [a { arity 1 }\n}".to_owned(),
        ),
        (
            "backslash-eof",
            "speclib demo 1.1 {\n  command a { arity 1 }\n}\\".to_owned(),
        ),
        (
            "dollar-eof",
            "speclib demo 1.1 {\n  command a { arity 1 }\n}$".to_owned(),
        ),
        (
            "high-unicode",
            "speclib \u{1f4a9}\u{202e}demo 1.1 {\n  command \u{0301}\u{0301} { arity 1 }\n}"
                .to_owned(),
        ),
        (
            "rtl-override-in-name",
            "speclib demo 1.1 {\n  command a\u{202e}b { arity 1 }\n}".to_owned(),
        ),
    ];

    for (label, source) in cases {
        let owned = source.clone();
        let pack = within(label, Duration::from_secs(20), move || load_pack(&owned));
        // Nothing is asserted about *what* loaded — only that a `Pack` came
        // back and its own invariants hold.
        assert!(
            pack.commands.iter().all(|c| !c.spec.name.is_empty()),
            "`{label}` produced a command with an empty name"
        );
    }
}

/// The well-formed pack the truncation tests below cut down.
const TRUNCATION_SUBJECT: &str = "speclib demo 1.1 {\n\
     \x20 display_name {Demo}\n\
     \x20 file_extension dem -name {Demo files} -dialect tcl9.0\n\
     \x20 values level { value fast -detail {Quick.} }\n\
     \x20 command demo::run {\n\
     \x20   dialects tcl8.6+\n\
     \x20   arity 2\n\
     \x20   introduced_version 1.2\n\
     \x20   arg 0 -role VarWrite\n\
     \x20   arg 1 -role Body\n\
     \x20   option -verbose -detail {Chatter.}\n\
     \x20   subcommand now { arity 0 }\n\
     \x20   hover { summary {Runs the demo.} }\n\
     \x20 }\n\
     }\n";

/// A pack truncated at each of the loader's distinct parse states loads
/// without panicking or hanging.
///
/// Truncation is the shape a real editor produces constantly: a file saved
/// mid-keystroke, a partial write, a `git checkout` racing the watcher.
///
/// These six cuts are fixed and hand-picked, one per state the loader can be
/// interrupted in — the exhaustive every-byte sweep is
/// [`every_prefix_of_a_valid_pack_loads`], which is `#[ignore]`d into the
/// manual tier because a permuted-input loop is fuzz-shaped
/// (`AGENTS.md`, "Fuzzing is always manual"). Keeping the representative set
/// here is what that policy asks for: "deterministic fixed-input tests
/// covering the same code stay in CI".
#[test]
fn truncation_at_each_parse_state_loads() {
    let cases = [
        ("mid-keyword", "speclib demo 1.1 {\n  comm"),
        ("just past the speclib header", "speclib demo 1.1 {\n"),
        (
            "inside an open command body",
            "speclib demo 1.1 {\n  command demo::run {\n",
        ),
        (
            "mid-lifecycle, a flag with no value",
            "speclib demo 1.1 {\n  command demo::run {\n    arity 2\n    introduced_version",
        ),
        (
            "mid-string, inside a braced summary",
            "speclib demo 1.1 {\n  command demo::run {\n    hover { summary {Runs the",
        ),
        (
            "the last byte, one newline short of complete",
            &TRUNCATION_SUBJECT[..TRUNCATION_SUBJECT.len() - 1],
        ),
    ];

    for (label, source) in cases {
        let owned = source.to_owned();
        let pack = within(label, Duration::from_secs(20), move || load_pack(&owned));
        assert!(
            pack.commands.iter().all(|c| !c.spec.name.is_empty()),
            "`{label}` produced a nameless command"
        );
    }
}

/// The exhaustive companion to [`truncation_at_each_parse_state_loads`]: every
/// byte offset of the same pack, not just the six representative ones.
///
/// Deliberately in the manual tier. The body is a permuted-input loop over
/// generated inputs, which `AGENTS.md` ("Fuzzing is always manual") puts
/// behind `#[ignore]` "regardless of how fast it happens to be today" — the
/// six-case test above is the CI cover for the same code.
#[ignore = "permuted-input sweep over every truncation offset; fuzz-shaped, so \
            manual tier only — run explicitly with `cargo test -p tcl-spectcl \
            --test pack_torture -- --ignored`, or via `make test-exhaustive`. \
            The representative fixed cases run in CI as \
            truncation_at_each_parse_state_loads"]
#[test]
fn every_prefix_of_a_valid_pack_loads() {
    let full = TRUNCATION_SUBJECT;

    for cut in 0..=full.len() {
        if !full.is_char_boundary(cut) {
            continue;
        }
        let prefix = full[..cut].to_owned();
        let pack = within(
            &format!("prefix-{cut}"),
            Duration::from_secs(20),
            move || load_pack(&prefix),
        );
        assert!(
            pack.commands.iter().all(|c| !c.spec.name.is_empty()),
            "prefix of {cut} bytes produced a nameless command"
        );
    }
}

/// Deeply nested braces do not overflow the stack.
///
/// The loader reaches every level of a pack through the same `statements()`
/// door, and a braced word's contents are re-segmented by a recursive call. A
/// pack nesting braces thousands deep is a two-line file to write and, if the
/// recursion is unguarded, an unconditional process abort — the one failure
/// mode `reload_spec_packs`'s `spawn_blocking` catch cannot convert to a
/// notice, because a stack overflow is not an unwind.
#[test]
fn deep_brace_nesting_does_not_overflow_the_stack() {
    for depth in [64usize, 512, 4_096, 50_000] {
        let source = format!(
            "speclib demo 1.1 {{\n  command a {{ arity 1 hover {{ summary {}{}{} }} }}\n}}\n",
            "{".repeat(depth),
            "x",
            "}".repeat(depth),
        );
        let pack = within(
            &format!("nest-{depth}"),
            Duration::from_secs(30),
            move || load_pack(&source),
        );
        assert!(
            pack.commands.iter().all(|c| !c.spec.name.is_empty()),
            "depth {depth} produced a nameless command"
        );
    }
}

/// An unbalanced brace at each structural level degrades to a notice, and the
/// declarations that *are* balanced still load where the design says they can.
#[test]
fn unbalanced_braces_degrade_rather_than_crash() {
    let cases = [
        (
            "speclib-body-unclosed",
            "speclib demo 1.1 {\n  command a { arity 1 }\n",
        ),
        (
            "command-body-unclosed",
            "speclib demo 1.1 {\n  command a { arity 1\n}\n",
        ),
        (
            "extra-close",
            "speclib demo 1.1 {\n  command a { arity 1 }}\n}\n",
        ),
        (
            "hover-unclosed",
            "speclib demo 1.1 {\n  command a { arity 1 hover { summary {x} }\n}\n",
        ),
        (
            "second-command-recovers",
            "speclib demo 1.1 {\n  command a { arity 1\n  command b { arity 1 }\n}\n",
        ),
    ];
    for (label, source) in cases {
        let owned = source.to_owned();
        let pack = within(label, Duration::from_secs(20), move || load_pack(&owned));
        assert!(
            pack.commands.iter().all(|c| !c.spec.name.is_empty()),
            "`{label}` produced a nameless command"
        );
    }
}

/// Absurd sizes — a megabyte-long command name, ten thousand words in one
/// statement, a very long single line — load in bounded time.
#[test]
fn absurd_sizes_load_in_bounded_time() {
    let mega_name = "x".repeat(1_000_000);
    let cases: Vec<(&str, String)> = vec![
        (
            "megabyte-command-name",
            format!("speclib demo 1.1 {{\n  command {mega_name} {{ arity 1 }}\n}}\n"),
        ),
        (
            "megabyte-summary",
            format!(
                "speclib demo 1.1 {{\n  command a {{ arity 1 hover {{ summary {{{}}} }} }}\n}}\n",
                "y".repeat(1_000_000)
            ),
        ),
        (
            "ten-thousand-words",
            format!(
                "speclib demo 1.1 {{\n  command a {{ arity 1 {} }}\n}}\n",
                "-flag ".repeat(10_000)
            ),
        ),
        (
            "one-very-long-line",
            format!(
                "speclib demo 1.1 {{ {} }}\n",
                "command a { arity 1 } ".repeat(5_000)
            ),
        ),
    ];

    for (label, source) in cases {
        let started = Instant::now();
        let pack = within(label, Duration::from_mins(1), move || load_pack(&source));
        assert!(
            started.elapsed() < Duration::from_mins(1),
            "`{label}` took {:?}",
            started.elapsed()
        );
        assert!(
            pack.commands.iter().all(|c| !c.spec.name.is_empty()),
            "`{label}` produced a nameless command"
        );
    }
}

/// A file that is not valid UTF-8 is reported and does not stop its neighbours
/// from loading.
///
/// `read_sources` turns an unreadable file into a whole-file notice, which is
/// the contract; what this pins is the *containment* — the good pack beside it
/// still reaches the set.
#[test]
fn a_non_utf8_pack_file_is_a_notice_and_its_neighbour_still_loads() {
    let root = scratch("non-utf8");
    let dir = root.join(".tcl-lsp");
    std::fs::create_dir_all(&dir).expect("pack dir");

    let bad = dir.join("broken.tclspec");
    // A lone 0x80 continuation byte: valid Tcl shape, invalid UTF-8.
    let mut bytes = b"speclib broken 1.1 {\n  command b { arity 1 }\n}\n".to_vec();
    bytes[9] = 0x80;
    std::fs::write(&bad, &bytes).expect("write invalid utf-8");

    write_pack(
        &root,
        "good.tclspec",
        "speclib good 1.1 {\n  command good::cmd { arity 1 }\n}\n",
    );

    let set = load_workspace(&root);
    assert!(
        set.packs.iter().any(|p| p.name == "good"),
        "the readable pack must still load: {:?}",
        set.packs.iter().map(|p| &p.name).collect::<Vec<_>>()
    );
    assert!(
        notice_text(&set).contains("cannot read pack file"),
        "the unreadable file must be reported: {}",
        notice_text(&set)
    );

    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// Axis 1b — semantics
// ---------------------------------------------------------------------------

/// An invalid lifecycle ordering is a notice and drops that lifecycle, and the
/// command carrying it still loads — the "notice-only for packs" rule from
/// `docs/design/spec-packs.md`.
///
/// The well-ordered `demo::sane` beside it is the control: without it, a loader
/// that dropped *every* lifecycle would pass the ordering assertion for the
/// wrong reason.
#[test]
fn an_invalid_lifecycle_ordering_is_a_notice_and_the_command_survives() {
    let pack = load_pack(
        "speclib demo 1.1 {\n\
        \x20 command demo::backwards {\n\
        \x20   arity 1\n\
        \x20   introduced_version 2.0\n\
        \x20   retired_version 1.0\n\
        \x20 }\n\
        \x20 command demo::sane {\n\
        \x20   arity 1\n\
        \x20   introduced_version 1.0\n\
        \x20   retired_version 2.0\n\
        \x20 }\n\
        }\n",
    );

    let sane = pack
        .command("demo::sane")
        .expect("a well-ordered neighbour must be unaffected");
    assert_eq!(
        (sane.spec.lifecycle.introduced, sane.spec.lifecycle.retired),
        (Some("1.0"), Some("2.0")),
        "the control's own lifecycle must survive — otherwise the assertion \
         below passes for the wrong reason"
    );

    let backwards = pack
        .command("demo::backwards")
        .expect("an invalid ordering must not drop the command itself");
    assert_eq!(
        (
            backwards.spec.lifecycle.introduced,
            backwards.spec.lifecycle.retired
        ),
        (None, None),
        "an invalid ordering must fall back to UNSPECIFIED"
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("predates the introducing release")),
        "the ordering failure must be reported: {:#?}",
        pack.notices
    );
}

/// The same rule one level down — an option's own lifecycle — and the
/// *containment* rule beside it, which is reported but **kept**.
#[test]
fn lifecycle_ordering_and_containment_are_checked_at_every_level() {
    let pack = load_pack(
        "speclib demo 1.1 {\n\
        \x20 command demo::cmd {\n\
        \x20   arity 1\n\
        \x20   introduced_version 2.0\n\
        \x20   option -backwards -introduced 2.0 -retired 1.0\n\
        \x20   subcommand early {\n\
        \x20     arity 0\n\
        \x20     introduced_version 1.0\n\
        \x20   }\n\
        \x20 }\n\
        }\n",
    );

    let cmd = pack.command("demo::cmd").expect("the command must load");
    assert_eq!(
        cmd.spec.lifecycle.introduced,
        Some("2.0"),
        "the command's own well-ordered lifecycle is untouched"
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("`-backwards`")
                && n.message.contains("predates the introducing release")),
        "an option's invalid ordering must be reported: {:#?}",
        pack.notices
    );
    assert!(
        pack.notices.iter().any(|n| n.message.contains("`early`")
            && n.message.contains("reaches outside")
            && n.message.contains("kept as declared")),
        "a subcommand reaching outside its command's window is reported but \
         kept, unlike an ordering failure: {:#?}",
        pack.notices
    );
}

/// A `speclib` version this build does not know still loads what it can, and
/// says so once — for every spelling except an unsupported **major**, which
/// design §6.1 makes a load error rather than a notice.
#[test]
fn an_unknown_vocabulary_version_loads_what_it_can() {
    for declared in ["2.9", "banana", "1.1.1", "-1"] {
        let source = format!("speclib demo {declared} {{\n  command demo::cmd {{ arity 1 }}\n}}\n");
        let pack = load_pack(&source);
        assert!(
            pack.command("demo::cmd").is_some(),
            "vocabulary `{declared}` must not cost the commands: {:#?}",
            pack.notices
        );
        assert!(
            !pack.notices.is_empty(),
            "vocabulary `{declared}` must be reported"
        );
        assert_eq!(pack.load_error, None, "{declared}");
    }
}

/// A `speclib` **major** past this build fails the whole pack closed
/// (design §6.1): a new major may redefine words this loader thinks it
/// knows, so reading the ones it recognises would publish confident answers
/// derived from a vocabulary it does not speak.
#[test]
fn an_unsupported_speclib_major_fails_the_pack_closed() {
    for declared in ["3.0", "99.99"] {
        let source = format!("speclib demo {declared} {{\n  command demo::cmd {{ arity 1 }}\n}}\n");
        let pack = load_pack(&source);
        assert!(
            pack.commands.is_empty(),
            "vocabulary `{declared}` must load nothing: {:#?}",
            pack.notices
        );
        assert_eq!(
            pack.load_error,
            Some(tcl_spectcl::LoadError::UnsupportedMajor(
                declared.to_owned()
            )),
            "{declared}"
        );
        assert_eq!(pack.notices.len(), 1, "{:#?}", pack.notices);
        assert!(
            pack.notices[0].message.contains("nothing is loaded"),
            "{:#?}",
            pack.notices
        );
    }
}

/// Unknown words at every level are dropped with a notice, and their siblings
/// still load — the compatibility policy's central promise.
#[test]
fn unknown_words_at_every_level_are_dropped_with_a_notice() {
    let pack = load_pack(
        "speclib demo 1.1 {\n\
        \x20 no_such_pack_level_word {whatever}\n\
        \x20 command demo::cmd {\n\
        \x20   arity 1\n\
        \x20   no_such_command_word 3\n\
        \x20   arg 0 -role NoSuchRole\n\
        \x20   option -x -no-such-flag 1\n\
        \x20   subcommand sub { arity 0 no_such_sub_word 1 }\n\
        \x20   hover { summary {Kept.} no_such_hover_word {x} }\n\
        \x20 }\n\
        }\n",
    );

    let cmd = pack
        .command("demo::cmd")
        .expect("the command must survive every unknown word");
    assert_eq!(
        cmd.spec.arity.min, 1,
        "a known sibling word must still apply"
    );
    assert!(
        pack.notices.len() >= 5,
        "every unknown word gets its own notice, got {:#?}",
        pack.notices
    );
}

/// A `-dialect` naming no profile keeps the extension row without routing, and
/// a bogus `dialects` word does not take the command down.
#[test]
fn bogus_dialect_names_degrade_to_a_notice() {
    let pack = load_pack(
        "speclib demo 1.1 {\n\
        \x20 file_extension dem -dialect no-such-dialect-at-all\n\
        \x20 command demo::cmd { arity 1 dialects no-such-dialect-at-all }\n\
        }\n",
    );

    assert_eq!(
        pack.file_extensions.len(),
        1,
        "the row is kept without routing: {:?}",
        pack.file_extensions
    );
    assert!(
        pack.file_extensions[0].dialect.is_none(),
        "a typo must not intern a fake profile name"
    );
    assert!(
        pack.command("demo::cmd").is_some(),
        "a bogus `dialects` word must not drop the command"
    );
    assert!(
        notice_text_for(&pack).contains("no dialect profile"),
        "the bad routing must be reported: {:#?}",
        pack.notices
    );
}

/// Loader notices for one pack, joined.
fn notice_text_for(pack: &tcl_spectcl::Pack) -> String {
    pack.notices
        .iter()
        .map(|n| format!("{}:{} {}", n.context, n.line, n.message))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A command declared twice — in one file and across two files of the same
/// pack — is a notice with the first definition winning, never a silent
/// overwrite.
#[test]
fn duplicate_command_declarations_are_reported_and_the_first_wins() {
    let root = scratch("dupes");

    // Within one file.
    let single = write_pack(
        &root,
        "single.tclspec",
        "speclib dupes 1.1 {\n\
        \x20 command dupes::twice { arity 1 hover { summary {First.} } }\n\
        \x20 command dupes::twice { arity 9 hover { summary {Second.} } }\n\
        }\n",
    );
    let set = load_files(&[single]);
    let merged = set
        .packs
        .iter()
        .find(|p| p.name == "dupes")
        .expect("the pack");
    let twice = merged.command("dupes::twice").expect("one survivor");
    assert_eq!(
        twice.spec.arity.min, 1,
        "the first declaration wins, not the last"
    );
    assert_eq!(
        merged
            .commands
            .iter()
            .filter(|c| c.spec.name == "dupes::twice")
            .count(),
        1,
        "the duplicate must not be installed twice"
    );
    // The notice lands on the *ignored* declaration (line 3 here), not on the
    // file's first line, and does not send the reader to the path they already
    // have open (issue #1638).
    let in_file = set
        .notices
        .iter()
        .find(|n| n.message.contains("already defined"))
        .expect("the in-file duplicate must be reported");
    assert_eq!(
        in_file.line, 3,
        "the squiggle belongs on the duplicate the author can delete, got line {}",
        in_file.line
    );
    assert!(
        in_file.message.contains("on line 2 of this file"),
        "an in-file duplicate must name the winning line, not the path: {}",
        in_file.message
    );

    // Across two files of one pack.
    let a = write_pack(
        &root,
        "a-first.tclspec",
        "speclib dupes2 1.1 {\n  command dupes2::twice { arity 1 }\n}\n",
    );
    let b = write_pack(
        &root,
        "b-second.tclspec",
        "speclib dupes2 1.1 {\n  command dupes2::twice { arity 9 }\n}\n",
    );
    let set = load_files(&[a, b]);
    let merged = set
        .packs
        .iter()
        .find(|p| p.name == "dupes2")
        .expect("the merged pack");
    assert_eq!(
        merged
            .commands
            .iter()
            .filter(|c| c.spec.name == "dupes2::twice")
            .count(),
        1,
        "a cross-file duplicate must not be installed twice"
    );
    assert_eq!(
        merged
            .command("dupes2::twice")
            .expect("survivor")
            .spec
            .arity
            .min,
        1,
        "merge order is sorted path order, so `a-first` wins"
    );
    let cross_file = set
        .notices
        .iter()
        .find(|n| n.message.contains("already defined in pack"))
        .expect("the cross-file duplicate must be reported");
    assert!(
        cross_file.message.contains("a-first.tclspec:2"),
        "a cross-file duplicate names the winning file *and* line: {}",
        cross_file.message
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Two *different* packs claiming the same command name: both load, and
/// installation picks one deterministically rather than corrupting the
/// registry.
#[test]
fn two_packs_claiming_one_command_install_deterministically() {
    let root = scratch("cross-pack-claim");
    let a = write_pack(
        &root,
        "alpha.tclspec",
        "speclib alpha 1.1 {\n  command shared::cmd { arity 1 hover { summary {Alpha.} } }\n}\n",
    );
    let b = write_pack(
        &root,
        "beta.tclspec",
        "speclib beta 1.1 {\n  command shared::cmd { arity 4 hover { summary {Beta.} } }\n}\n",
    );
    let set = load_files(&[a, b]);
    assert_eq!(set.packs.len(), 2, "two distinct packs, both loaded");

    let installed = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.1", &set);
    let spec = installed
        .get("shared::cmd")
        .expect("the contested command must reach the registry exactly once");
    assert!(
        installed
            .command_names()
            .filter(|n| *n == "shared::cmd")
            .count()
            == 1,
        "a contested name must not be enumerable twice — completion would show a duplicate"
    );

    // Same inputs, same winner: repeat the whole install and compare.
    let again = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.1", &set);
    assert_eq!(
        spec.arity,
        again.get("shared::cmd").expect("still there").arity,
        "which pack wins a contested command must be deterministic"
    );

    // Deterministic is not enough on its own — the losing author has to be
    // told, on their own declaration, which is what issue #1637 was about.
    let notice = set
        .notices
        .iter()
        .find(|n| n.message.contains("is already declared by pack"))
        .expect("a cross-pack command collision must be reported");
    assert!(
        notice.path.ends_with("beta.tclspec") && notice.line == 2,
        "the notice belongs on the declaration that lost, got {}:{}",
        notice.path.display(),
        notice.line
    );
    assert!(
        notice.message.contains("`alpha`") && notice.message.contains("-override"),
        "and must name the winner and the way out: {}",
        notice.message
    );
    assert_eq!(spec.arity.min, 1, "`alpha` is the winner the notice names");

    let _ = std::fs::remove_dir_all(&root);
}

/// The `-override` direction of the same collision: the later pack wins, and
/// the notice moves to the declaration it displaced (#1637).
///
/// The notice must track the install, not a fixed idea of who wins — otherwise
/// it would tell the author the opposite of what the registry did.
#[test]
fn a_cross_pack_override_reports_on_the_declaration_it_replaces() {
    let root = scratch("cross-pack-override");
    let a = write_pack(
        &root,
        "alpha.tclspec",
        "speclib alpha 1.1 {\n  command shared::cmd { arity 1 }\n}\n",
    );
    let b = write_pack(
        &root,
        "beta.tclspec",
        "speclib beta 1.1 {\n  command shared::cmd -override { arity 4 }\n}\n",
    );
    let set = load_files(&[a, b]);

    let installed = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.1", &set);
    assert_eq!(
        installed.get("shared::cmd").expect("installed").arity.min,
        4,
        "`-override` makes the later pack win"
    );

    let notice = set
        .notices
        .iter()
        .find(|n| n.message.contains("with `-override`"))
        .expect("the replacement must be reported");
    assert!(
        notice.path.ends_with("alpha.tclspec"),
        "the notice belongs on the declaration that was replaced, got {}",
        notice.path.display()
    );
    assert!(
        notice.message.contains("`beta`"),
        "and must name who replaced it: {}",
        notice.message
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Two packs declaring the same name for **different vendor packages** are not
/// a collision, and must not be reported (#1637).
///
/// This is the false positive that matters: the six shipped EDA loadables all
/// declare `report_timing`, and `install_into`'s vendor gate means no profile
/// ever admits two of them. A collision check that ignored the gate would put
/// a dozen warnings on the packs tcl-lsp itself ships.
#[test]
fn different_vendor_packages_declaring_one_name_is_not_a_collision() {
    let root = scratch("vendor-gate");
    let a = write_pack(
        &root,
        "vendor-a.tclspec",
        "speclib vendora 1.1 {\n\
        \x20 command vendor::report_timing {\n\
        \x20   arity 1\n\
        \x20   required_package cadence-genus\n\
        \x20 }\n\
        }\n",
    );
    let b = write_pack(
        &root,
        "vendor-b.tclspec",
        "speclib vendorb 1.1 {\n\
        \x20 command vendor::report_timing {\n\
        \x20   arity 4\n\
        \x20   required_package synopsys\n\
        \x20 }\n\
        }\n",
    );
    let set = load_files(&[a, b]);

    assert!(
        !set.notices
            .iter()
            .any(|n| n.message.contains("is already declared by pack")),
        "two vendors that never share a profile do not collide: {:#?}",
        set.notices
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Two packs claiming the same `file_extension`: the owner invariant says one
/// owner per extension, so the set must resolve to exactly one.
#[test]
fn one_owner_per_file_extension_across_packs() {
    let root = scratch("extension-owners");
    let a = write_pack(
        &root,
        "alpha.tclspec",
        "speclib alpha 1.1 {\n\
        \x20 file_extension shr -name {Alpha's} -dialect tcl9.0\n\
        \x20 command alpha::cmd { arity 1 }\n\
        }\n",
    );
    let b = write_pack(
        &root,
        "beta.tclspec",
        "speclib beta 1.1 {\n\
        \x20 file_extension shr -name {Beta's} -dialect tcl8.6\n\
        \x20 command beta::cmd { arity 1 }\n\
        }\n",
    );
    let set = load_files(&[a, b]);

    let owners: Vec<(String, &'static str)> = set.extension_dialects();
    let shr: Vec<_> = owners.iter().filter(|(ext, _)| ext == "shr").collect();
    assert_eq!(
        shr.len(),
        1,
        "one extension has exactly one owner: {owners:?}"
    );
    assert_eq!(shr[0].1, "tcl9.0", "the first pack in name order owns it");

    // And the pack that lost the extension is told, on the row it declared
    // (issue #1637). This matters more than the command case: an extension
    // routed to the wrong dialect mis-lexes every file of that type.
    let notice = set
        .notices
        .iter()
        .find(|n| n.message.contains("is already claimed by pack"))
        .expect("the extension collision must be reported");
    assert!(
        notice.path.ends_with("beta.tclspec") && notice.line == 2,
        "the notice belongs on the losing `file_extension` row, got {}:{}",
        notice.path.display(),
        notice.line
    );
    assert!(
        notice.message.contains("`alpha`") && notice.message.contains("tcl9.0"),
        "and must name the owner and the routing that won: {}",
        notice.message
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A pack redefining a shipped builtin does not change it unless it says
/// `-override`, and either way the outcome is reported.
#[test]
fn a_pack_redefining_a_builtin_reports_and_does_not_silently_win() {
    let root = scratch("builtin-clash");
    let path = write_pack(
        &root,
        "clash.tclspec",
        "speclib clash 1.1 {\n\
        \x20 command lsort { arity 1 hover { summary {Not the real lsort.} } }\n\
        }\n",
    );
    let set = load_files(&[path]);

    let plain = tcl_registry::registry_for_dialect("tcl9.1");
    let with_packs = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.1", &set);
    let shipped = plain.get("lsort").expect("`lsort` is shipped");
    let after = with_packs.get("lsort").expect("`lsort` is still there");
    assert_eq!(
        shipped.arity, after.arity,
        "a pack without `-override` must not change a shipped command"
    );

    let collisions = pack::collision_notices(&set, &with_packs);
    assert!(
        collisions.iter().any(|n| n.message.contains("lsort")),
        "the collision must be reported: {collisions:#?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// Axis 2 — scale
// ---------------------------------------------------------------------------

/// A generated pack with thousands of commands loads, installs, and stays
/// queryable in bounded time.
///
/// The budget is deliberately loose — this is a stability gate, not a
/// benchmark, and it runs on whatever CI hardware is going. What it catches is
/// a quadratic: `merge_group`'s duplicate detection, `install`'s insertion, and
/// the registry's own name lookup all have to be sub-quadratic for a pack this
/// size to be usable at all.
#[test]
fn a_pack_with_thousands_of_commands_loads_and_installs() {
    const COMMANDS: usize = 4_000;

    let mut source = String::from("speclib bulk 1.1 {\n");
    for i in 0..COMMANDS {
        let _ = write!(
            source,
            "  command bulk::cmd{i} {{\n\
             \x20   arity 2\n\
             \x20   arg 0 -role Value\n\
             \x20   option -opt{i} -detail {{Option {i}.}}\n\
             \x20   subcommand sub{i} {{ arity 0 }}\n\
             \x20   hover {{ summary {{Command {i}.}} }}\n\
             \x20 }}\n"
        );
    }
    source.push_str("}\n");

    let root = scratch("bulk");
    let path = write_pack(&root, "bulk.tclspec", &source);

    let started = Instant::now();
    let set = within("bulk-load", Duration::from_mins(3), move || {
        load_files(&[path])
    });
    let load_time = started.elapsed();

    let merged = set
        .packs
        .iter()
        .find(|p| p.name == "bulk")
        .expect("the pack");
    assert_eq!(merged.commands.len(), COMMANDS, "every command must load");
    assert!(
        set.notices.is_empty(),
        "a well-formed bulk pack must produce no notices: {:#?}",
        &set.notices[..set.notices.len().min(5)]
    );

    let started = Instant::now();
    let installed = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.1", &set);
    let install_time = started.elapsed();

    for i in [0usize, COMMANDS / 2, COMMANDS - 1] {
        assert!(
            installed.get(&format!("bulk::cmd{i}")).is_some(),
            "`bulk::cmd{i}` did not reach the registry"
        );
    }
    let enumerated = installed
        .command_names()
        .filter(|n| n.starts_with("bulk::cmd"))
        .count();
    assert_eq!(
        enumerated, COMMANDS,
        "every bulk command must be enumerable, or completion silently loses them"
    );

    eprintln!("bulk: {COMMANDS} commands loaded in {load_time:?}, installed in {install_time:?}");
    assert!(
        load_time < Duration::from_mins(2),
        "loading {COMMANDS} commands took {load_time:?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Many packs at once — one file each — discover, merge, and install without a
/// blowup in either time or notices.
#[test]
fn many_packs_at_once_load_together() {
    const PACKS: usize = 200;

    let root = scratch("many-packs");
    for i in 0..PACKS {
        write_pack(
            &root,
            &format!("pack{i:03}.tclspec"),
            &format!("speclib many{i} 1.1 {{\n  command many{i}::cmd {{ arity 1 }}\n}}\n"),
        );
    }

    let started = Instant::now();
    let root_for_load = root.clone();
    let set = within("many-packs", Duration::from_mins(3), move || {
        load_workspace(&root_for_load)
    });
    let elapsed = started.elapsed();

    assert_eq!(set.packs.len(), PACKS, "every pack must be discovered");
    assert!(
        set.notices.is_empty(),
        "well-formed packs produce no notices: {:#?}",
        &set.notices[..set.notices.len().min(5)]
    );
    eprintln!("{PACKS} packs loaded in {elapsed:?}");

    let installed = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.1", &set);
    for i in [0usize, PACKS / 2, PACKS - 1] {
        assert!(
            installed.get(&format!("many{i}::cmd")).is_some(),
            "`many{i}::cmd` did not reach the registry"
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// One malformed pack among many good ones costs only itself.
///
/// This is the containment property that matters most in a real workspace: a
/// user editing one `.tclspec` must not lose the commands every *other* pack
/// declares while their file is mid-keystroke.
#[test]
fn one_broken_pack_does_not_cost_the_others() {
    let root = scratch("broken-neighbour");
    for i in 0..20 {
        write_pack(
            &root,
            &format!("good{i:02}.tclspec"),
            &format!("speclib good{i} 1.1 {{\n  command good{i}::cmd {{ arity 1 }}\n}}\n"),
        );
    }
    write_pack(&root, "broken.tclspec", "speclib broken 1.1 {\n  command ");
    write_pack(&root, "empty.tclspec", "");
    write_pack(&root, "brace.tclspec", "{");

    let set = load_workspace(&root);
    for i in 0..20 {
        assert!(
            set.packs.iter().any(|p| p.name == format!("good{i}")),
            "`good{i}` was lost to a broken neighbour: {:?}",
            set.packs.iter().map(|p| &p.name).collect::<Vec<_>>()
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// Axis 6 — adversarial content
// ---------------------------------------------------------------------------

/// Injection shapes inside spec *strings* are data, not code.
///
/// A `.tclspec` is Tcl, and its `hover` bodies are braced words — so
/// `[exec …]`, `${…}` and friends inside one must reach the registry as the
/// literal bytes the author wrote. If any of them were ever substituted, a
/// pack would be a remote-code-execution vector on open.
#[test]
fn injection_shapes_in_spec_strings_stay_literal() {
    // The sentinel lives in this test's own scratch directory, never at a
    // fixed path: a shared `/tmp` name collides between concurrent runs and
    // between checkouts, and — worse — a single genuine failure would leave
    // the file behind and make every later run fail for a reason that has
    // nothing to do with the code under test. Removed before the load so the
    // assertion cannot inherit a stale file, and again afterwards so a failure
    // here does not poison the next run.
    let root = scratch("injection");
    let sentinel = root.join("pwned");
    let _ = std::fs::remove_file(&sentinel);

    let hostile_summary = format!(
        r"[exec /bin/sh -c {{touch {}}}] ${{env(HOME)}} $::argv \x41 %s %n",
        sentinel.display()
    );
    let source = format!(
        "speclib evil 1.1 {{\n\
        \x20 command evil::cmd {{\n\
        \x20   arity 1\n\
        \x20   hover {{ summary {{{hostile_summary}}} }}\n\
        \x20 }}\n\
        }}\n"
    );
    let pack = load_pack(&source);
    let cmd = pack.command("evil::cmd").expect("the command must load");
    let summary = cmd.spec.hover.map(|h| h.summary).unwrap_or_default();
    assert!(
        summary.contains("[exec") && summary.contains("${env(HOME)}"),
        "the injection shapes must survive verbatim as data, got: {summary:?}"
    );
    let executed = sentinel.exists();
    let _ = std::fs::remove_file(&sentinel);
    let _ = std::fs::remove_dir_all(&root);
    assert!(!executed, "loading a pack executed its `hover` body");
}

/// Path traversal in a path-like field is carried as text and never resolved
/// by the loader.
#[test]
fn path_traversal_in_a_path_field_is_not_resolved() {
    let pack = load_pack(
        "speclib evil 1.1 {\n\
        \x20 command evil::cmd {\n\
        \x20   arity 1\n\
        \x20   hover { source {../../../../../../etc/passwd} }\n\
        \x20 }\n\
        }\n",
    );
    let cmd = pack.command("evil::cmd").expect("the command must load");
    // Whatever the field carries, the loader must not have opened anything —
    // proven by the load having succeeded with no read error and the text
    // arriving unchanged.
    let source_field = cmd.spec.hover.map(|h| h.source).unwrap_or_default();
    assert!(
        source_field.contains("etc/passwd"),
        "the field is data; got {source_field:?}"
    );
}

/// A pack whose *own* file name and pack name are adversarial still routes to
/// notices keyed by the real path.
#[test]
fn adversarial_pack_names_do_not_confuse_notice_routing() {
    let root = scratch("evil-names");
    let path = write_pack(
        &root,
        "..evil.tclspec",
        "speclib ../../../etc/passwd 1.1 {\n  command ok { arity 1 }\n}\n",
    );
    let set = load_files(std::slice::from_ref(&path));
    for notice in &set.notices {
        assert_eq!(
            notice.path, path,
            "every notice must be keyed by the file it came from"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// Axis 4 — the version gate, at the loader/analyser altitude
// ---------------------------------------------------------------------------

/// Analyse `source` with `packs` overlaid, returning the version-gate codes and
/// messages only.
fn version_gate_diags(packs: &PackSet, dialect: &str, source: &str) -> Vec<(String, String)> {
    // Building the overlay registry is what puts the pack's specs where
    // `with_pack_overlay` can find them — the same order `reload_spec_packs`
    // uses, and the reason a test that skipped it would see a plain registry
    // and no gate at all.
    let _registry = tcl_spectcl::install::registry_for_dialect_with_packs(dialect, packs);
    Analyser::new()
        .with_pack_overlay(packs.key)
        .analyse(source, dialect)
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W135" | "W136" | "W139" | "W144"))
        .map(|d| (d.code.to_string(), d.message.clone()))
        .collect()
}

/// A pack's own lifecycle stamps drive W135 / W139 / W144 against the
/// document's `package require`, and a document that pins a satisfying version
/// stays silent.
///
/// This is the pack half of the version gate: the same three codes a shipped
/// spec produces, sourced from a `.tclspec` the user wrote. The controls
/// matter as much as the positives — a gate that fired unconditionally would
/// satisfy every positive assertion here.
#[test]
fn pack_lifecycle_stamps_drive_the_version_gate() {
    let root = scratch("version-gate");
    let path = write_pack(
        &root,
        "gated.tclspec",
        "speclib gated 1.1 {\n\
        \x20 command gated::future {\n\
        \x20   arity 1\n\
        \x20   required_package gatedlib\n\
        \x20   introduced_version 2.0\n\
        \x20 }\n\
        \x20 command gated::oldish {\n\
        \x20   arity 1\n\
        \x20   required_package gatedlib\n\
        \x20   introduced_version 1.0\n\
        \x20   deprecated_version 1.5\n\
        \x20 }\n\
        \x20 command gated::gone {\n\
        \x20   arity 1\n\
        \x20   required_package gatedlib\n\
        \x20   introduced_version 1.0\n\
        \x20   retired_version 1.5\n\
        \x20 }\n\
        \x20 command gated::always {\n\
        \x20   arity 1\n\
        \x20   required_package gatedlib\n\
        \x20 }\n\
        }\n",
    );
    let set = load_files(&[path]);
    assert!(
        set.notices.is_empty(),
        "the gated pack must load cleanly: {:#?}",
        set.notices
    );

    // Floor 1.0: `future` is not there yet, `gone` is already gone, `oldish`
    // is not yet deprecated, `always` is ungated.
    let at_1_0 = version_gate_diags(
        &set,
        "tcl9.0",
        "package require gatedlib 1.0\n\
         gated::future x\n\
         gated::oldish x\n\
         gated::gone x\n\
         gated::always x\n",
    );
    assert!(
        at_1_0
            .iter()
            .any(|(code, msg)| code == "W135" && msg.contains("gated::future")),
        "a command introduced at 2.0 must be W135 against a 1.0 pin: {at_1_0:?}"
    );
    assert!(
        !at_1_0.iter().any(|(_, msg)| msg.contains("gated::always")),
        "an ungated command must never draw a version diagnostic: {at_1_0:?}"
    );
    assert!(
        !at_1_0
            .iter()
            .any(|(code, msg)| code == "W144" && msg.contains("gated::oldish")),
        "1.0 is before the 1.5 deprecation — no W144 yet: {at_1_0:?}"
    );

    // Floor 1.5: `oldish` is now deprecated and `gone` retired; `future` is
    // still not there.
    let at_1_5 = version_gate_diags(
        &set,
        "tcl9.0",
        "package require gatedlib 1.5\n\
         gated::future x\n\
         gated::oldish x\n\
         gated::gone x\n\
         gated::always x\n",
    );
    assert!(
        at_1_5
            .iter()
            .any(|(code, msg)| code == "W144" && msg.contains("gated::oldish")),
        "a command deprecated at 1.5 must be W144 against a 1.5 pin: {at_1_5:?}"
    );
    assert!(
        at_1_5
            .iter()
            .any(|(code, msg)| code == "W139" && msg.contains("gated::gone")),
        "a command retired at 1.5 must be W139 against a 1.5 pin — the \
         retiring release is exclusive: {at_1_5:?}"
    );
    assert!(
        !at_1_5.iter().any(|(_, msg)| msg.contains("gated::always")),
        "an ungated command stays silent at every floor: {at_1_5:?}"
    );

    // Floor 2.0: everything the pack still has is silent.
    let at_2_0 = version_gate_diags(
        &set,
        "tcl9.0",
        "package require gatedlib 2.0\n\
         gated::future x\n\
         gated::oldish x\n\
         gated::always x\n",
    );
    assert!(
        !at_2_0
            .iter()
            .any(|(code, msg)| code == "W135" && msg.contains("gated::future")),
        "2.0 satisfies the 2.0 introduction — W135 must clear: {at_2_0:?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Editing a pack's lifecycle stamp changes the verdict — the same reload the
/// editor performs, at this altitude.
///
/// The pack set's content key is what the overlay registry is cached under, so
/// this is also the regression guard for a stale-overlay bug: if the second
/// load reused the first key, the second analysis would answer with the first
/// pack's stamps.
#[test]
fn editing_a_lifecycle_stamp_changes_the_verdict_on_reload() {
    let root = scratch("gate-reload");
    let source = "package require churnlib 1.0\nchurn::cmd x\n";

    let path = write_pack(
        &root,
        "churn.tclspec",
        "speclib churn 1.1 {\n\
        \x20 command churn::cmd {\n\
        \x20   arity 1\n\
        \x20   required_package churnlib\n\
        \x20   introduced_version 2.0\n\
        \x20 }\n\
        }\n",
    );
    let before = load_files(std::slice::from_ref(&path));
    let gated = version_gate_diags(&before, "tcl9.0", source);
    assert!(
        gated
            .iter()
            .any(|(code, msg)| code == "W135" && msg.contains("churn::cmd")),
        "the 2.0 introduction must gate a 1.0 pin: {gated:?}"
    );

    // The author fixes the stamp.
    std::fs::write(
        &path,
        "speclib churn 1.1 {\n\
        \x20 command churn::cmd {\n\
        \x20   arity 1\n\
        \x20   required_package churnlib\n\
        \x20   introduced_version 1.0\n\
        \x20 }\n\
        }\n",
    )
    .expect("rewrite pack");
    let after = load_files(&[path]);
    assert_ne!(
        before.key, after.key,
        "an edited pack must produce a different content key, or every \
         consumer keyed on it serves the pre-edit answer"
    );
    let cleared = version_gate_diags(&after, "tcl9.0", source);
    assert!(
        !cleared
            .iter()
            .any(|(code, msg)| code == "W135" && msg.contains("churn::cmd")),
        "the edited stamp must clear the gate: {cleared:?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// Regressions for the notice gaps this sweep found (#1634, #1635, #1637, #1638)
// ---------------------------------------------------------------------------

/// A `.tclspec` saved with a UTF-8 BOM loads normally (#1635).
///
/// The mark is a file prologue, exactly as Tcl 9's `source` treats it — so the
/// `speclib` word is `speclib`, not `\u{feff}speclib`, and the pack is not
/// lost to an editor that defaults to "UTF-8 with BOM".
#[test]
fn a_bom_prefixed_pack_loads_like_any_other() {
    let pack = load_pack("\u{feff}speclib demo 1.1 {\n  command demo::a { arity 1 }\n}\n");

    assert_eq!(
        pack.name, "demo",
        "the BOM must not hide the `speclib` word"
    );
    assert_eq!(pack.dsl_version, "1.1");
    assert!(pack.command("demo::a").is_some());
    assert!(
        pack.notices.is_empty(),
        "a BOM is not a defect to report: {:#?}",
        pack.notices
    );
}

/// `speclib_version_span` must skip a leading BOM exactly as the loader does.
///
/// This is the hook `tcl spec upgrade` rewrites through, and it is a *separate*
/// entry point with its own `LexerConfig` — the pack-loading test above does
/// not exercise it. If the two disagreed about whether a leading mark is a
/// prologue, the upgrade would compute a byte range against a different
/// tokenisation than the one `load_pack` used and rewrite the wrong span.
/// (Caught by mutation testing during the #1641 review: flipping this entry's
/// disposition alone left every existing test green.)
#[test]
fn the_version_span_skips_a_leading_bom_like_the_loader() {
    let source = "\u{feff}speclib demo 1.1 {\n  command demo::a { arity 1 }\n}\n";

    let (range, version) =
        tcl_spectcl::speclib_version_span(source).expect("a BOM must not hide the version word");

    assert_eq!(version, "1.1");
    assert_eq!(
        &source[range.clone()],
        "1.1",
        "the span must be the version word's own bytes, so a rewrite lands on \
         it and not on the BOM-shifted text beside it"
    );
}

/// …but a BOM *inside* a block is ordinary data (#1635).
///
/// The half of the fix that is easy to get wrong: flipping the disposition for
/// every segmentation, rather than only the file entry, would silently edit
/// pack content — a `hover` summary an author deliberately began with U+FEFF
/// would reach the registry a character short.
#[test]
fn a_bom_inside_a_block_is_content_not_a_prologue() {
    let pack = load_pack(
        "speclib demo 1.1 {\n\
        \x20 command demo::a {\n\
        \x20   arity 1\n\
        \x20   hover { summary {\u{feff}leading mark} }\n\
        \x20 }\n\
        }\n",
    );
    let summary = pack
        .command("demo::a")
        .expect("the command must load")
        .spec
        .hover
        .map(|h| h.summary)
        .unwrap_or_default();
    assert!(
        summary.starts_with('\u{feff}'),
        "a mark inside a block is the author's character, got {summary:?}"
    );
}

/// A BOM'd pack survives the compiled-pack cache round trip (#1635).
///
/// The fix bumped the on-disk `FORMAT`, so a cold load, a warm load, and a load
/// with the cache thrown away mid-flight must all produce the same pack — both
/// for the file's leading mark and for a mark inside a block, which are read
/// under opposite dispositions.
///
/// # What this does *not* claim
///
/// It does not pin the BOM disposition's presence in the memo key. That is
/// deliberate: a memo is installed per file (`load_pack_cached` seeds one
/// `load_pack` call from that file's own entry), and within a file the
/// file-entry text and any block's text can never be equal, so the two
/// dispositions cannot meet in one memo and no test can force them to. The key
/// still carries the disposition because a key that omits something affecting
/// the answer is wrong on its face — but an assertion here would pass whether
/// or not it were there, and a test that cannot fail is worse than none.
#[test]
fn a_bom_prefixed_pack_survives_the_cache_round_trip() {
    let root = scratch("bom-cache");
    let cache = root.join("cache");
    tcl_spectcl::cache::redirect_for_test(Some((cache.clone(), false)));

    let path = write_pack(
        &root,
        "bom.tclspec",
        "\u{feff}speclib bomdemo 1.1 {\n\
        \x20 command bomdemo::a {\n\
        \x20   arity 1\n\
        \x20   hover { summary {\u{feff}kept} }\n\
        \x20 }\n\
        }\n",
    );

    let shape = |set: &PackSet| {
        let pack = set
            .packs
            .iter()
            .find(|p| p.name == "bomdemo")
            .expect("the BOM'd pack must load");
        let summary = pack
            .command("bomdemo::a")
            .expect("bomdemo::a")
            .spec
            .hover
            .map(|h| h.summary)
            .unwrap_or_default();
        (pack.name.clone(), pack.commands.len(), summary.to_owned())
    };

    let cold = load_files(std::slice::from_ref(&path));
    assert!(
        std::fs::read_dir(&cache).is_ok(),
        "a cold load populated the cache directory"
    );
    let warm = load_files(std::slice::from_ref(&path));
    assert_eq!(shape(&cold), shape(&warm), "a warm load is the same load");

    // And with the cache thrown away, still the same — the disposability
    // contract, with a BOM in play.
    let _ = std::fs::remove_dir_all(&cache);
    assert_eq!(
        shape(&cold),
        shape(&load_files(std::slice::from_ref(&path)))
    );

    let (name, commands, summary) = shape(&cold);
    assert_eq!(name, "bomdemo", "the file's leading mark is a prologue");
    assert_eq!(commands, 1);
    assert_eq!(summary, "\u{feff}kept", "the mark inside a block is data");

    tcl_spectcl::cache::redirect_for_test(None);
    let _ = std::fs::remove_dir_all(&root);
}

/// A `command` with no body block is named in a notice (#1634).
///
/// The shape is the brace-on-the-next-line mistake, which is valid Tcl and so
/// reaches the loader as two statements. Before the fix the command vanished
/// with nothing naming it.
#[test]
fn a_command_with_no_body_is_named_in_a_notice() {
    let pack = load_pack("speclib demo 1.1 {\n  command demo::bar\n  {\n    arity 1\n  }\n}\n");

    assert!(pack.command("demo::bar").is_none(), "it cannot load");
    let naming = pack
        .notices
        .iter()
        .find(|n| n.message.contains("demo::bar"))
        .expect("some notice must name the dropped command");
    assert_eq!(naming.line, 2, "the notice belongs on the declaration");
    assert!(
        naming.message.contains("no `{ … }` body block"),
        "and must say why: {}",
        naming.message
    );
}

/// The orphaned block left over by that mistake gets a readable one-line
/// notice, not its own body quoted back (#1634).
#[test]
fn an_orphaned_block_notice_is_one_readable_line() {
    let pack = load_pack("speclib demo 1.1 {\n  command demo::bar\n  {\n    arity 1\n  }\n}\n");

    let orphan = pack
        .notices
        .iter()
        .find(|n| n.message.contains("with no preceding declaration"))
        .expect("the orphaned block must be reported");
    assert!(
        !orphan.message.contains('\n'),
        "a diagnostic message must stay one line, got {:?}",
        orphan.message
    );
    assert!(
        !orphan.message.contains("arity 1"),
        "the block's own body must not be quoted into the message, got {:?}",
        orphan.message
    );
}

/// An unknown *property* that is long or spans lines is elided, not quoted
/// whole (#1634).
///
/// The orphaned-block test above returns before the quoting code, so it does
/// not pin it — flipping `unknown_property` back to a raw `word_text(0)` left
/// that test green (found by mutation testing during the #1641 review). This
/// is the case that catches it: a word that is neither empty nor short, on the
/// non-block path where `quotable` is the only thing keeping the message to
/// one bounded line.
#[test]
fn a_long_unknown_property_is_elided_rather_than_quoted_whole() {
    let pack = load_pack(
        "speclib demo 1.1 {\n\
        \x20 command demo::a {\n\
        \x20   arity 1\n\
        \x20   \"a very long unknown property name that runs well past the limit\n\
        \x20    and then keeps going onto a second physical line\"\n\
        \x20 }\n\
        }\n",
    );

    let notice = pack
        .notices
        .iter()
        .find(|n| n.message.contains("unknown"))
        .unwrap_or_else(|| panic!("the unknown property must be reported: {:#?}", pack.notices));
    assert!(
        !notice.message.contains('\n'),
        "a diagnostic message must stay one physical line, got {:?}",
        notice.message
    );
    assert!(
        notice.message.contains('…'),
        "and must show it was elided rather than silently truncated, got {:?}",
        notice.message
    );
    assert!(
        !notice.message.contains("second physical line"),
        "the tail must not reach the message, got {:?}",
        notice.message
    );
}

/// A second `speclib` block in one file is reported, with what it costs (#1634).
#[test]
fn a_second_speclib_block_is_reported_with_its_command_count() {
    let pack = load_pack(
        "speclib one 1.1 {\n\
        \x20 command one::a { arity 1 }\n\
        }\n\
        speclib two 1.1 {\n\
        \x20 command two::b { arity 1 }\n\
        \x20 command two::c { arity 1 }\n\
        }\n",
    );

    assert_eq!(
        pack.name, "one",
        "the first pack is still the one that loads"
    );
    assert!(pack.command("one::a").is_some());
    let notice = pack
        .notices
        .iter()
        .find(|n| n.message.contains("second `speclib` block"))
        .expect("the discarded block must be reported");
    assert!(
        notice.message.contains("`two`") && notice.message.contains("2 command(s)"),
        "naming the block and its cost is the actionable part: {}",
        notice.message
    );
}

/// A command name carrying whitespace is **valid Tcl** and must load.
///
/// #1638 originally dropped these, reasoning that a command word cannot carry
/// whitespace. It can: a Tcl command name is an arbitrary string key in the
/// namespace command table, and a braced or quoted call site invokes it.
/// Confirmed against tclsh 8.6 and 9.0 — `proc {evil name} {} {…}` is created,
/// listed by `info commands`, and invoked as `{evil name}`; the same holds for
/// a tab, a newline, and for a name that is only a space. Our own pipeline
/// resolves such a call site too, since a braced command word lowers to
/// `WordExpr::BracedLiteral` whose text is the brace-stripped content. The
/// drop removed valid specs; this pins the reversal.
#[test]
fn a_whitespace_bearing_command_name_loads_and_is_installable() {
    for (source, expected) in [
        (
            "speclib demo 1.1 {\n command {evil name} { arity 1 }\n}\n",
            "evil name",
        ),
        ("speclib demo 1.1 {\n command { } { arity 1 }\n}\n", " "),
        (
            "speclib demo 1.1 {\n command {tab\tname} { arity 1 }\n}\n",
            "tab\tname",
        ),
    ] {
        let pack = load_pack(source);
        assert!(
            pack.commands.iter().any(|c| c.spec.name == expected),
            "a whitespace-bearing name is valid Tcl and must load; wanted \
             {expected:?}, got {:?}",
            pack.commands
                .iter()
                .map(|c| c.spec.name)
                .collect::<Vec<_>>()
        );
        assert!(
            !pack
                .notices
                .iter()
                .any(|n| n.message.contains("whitespace")),
            "and must not be warned about: {:#?}",
            pack.notices
        );
    }
}

/// Three packs claim one name; two of them are live in the same profile.
///
/// The collision that matters is between the *second* and the *third*, and a
/// standing-claim map holding one entry per name cannot see it: the third
/// claim is compared only against the first, found vendor-disjoint, and waved
/// through. Found reviewing #1637 — the fix keeps every standing claim, so a
/// new one settles against the first claim it could actually share a registry
/// with.
#[test]
fn a_collision_behind_a_vendor_disjoint_claim_is_still_reported() {
    let root = scratch("three-way-claim");
    let a = write_pack(
        &root,
        "alpha.tclspec",
        "speclib alpha 1.1 {\n\
        \x20 command vendor::report_timing {\n\
        \x20   arity 1\n\
        \x20   required_package synopsys\n\
        \x20 }\n\
        }\n",
    );
    let b = write_pack(
        &root,
        "bravo.tclspec",
        "speclib bravo 1.1 {\n\
        \x20 command vendor::report_timing {\n\
        \x20   arity 2\n\
        \x20   required_package cadence-genus\n\
        \x20 }\n\
        }\n",
    );
    let c = write_pack(
        &root,
        "charlie.tclspec",
        "speclib charlie 1.1 {\n\
        \x20 command vendor::report_timing {\n\
        \x20   arity 3\n\
        \x20   required_package cadence-genus\n\
        \x20 }\n\
        }\n",
    );
    let set = load_files(&[a, b, c]);

    let notice = set
        .notices
        .iter()
        .find(|n| n.message.contains("is already declared by pack"))
        .unwrap_or_else(|| {
            panic!(
                "two packs live in one profile collide even behind a \
                 vendor-disjoint claim: {:#?}",
                set.notices
            )
        });
    assert!(
        notice.path.ends_with("charlie.tclspec"),
        "the notice belongs on the claim that lost, got {}",
        notice.path.display()
    );
    assert!(
        notice.message.contains("`bravo`"),
        "and must name the claim it actually collided with — `bravo`, not the \
         vendor-disjoint `alpha`: {}",
        notice.message
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A logical pack spanning two files: an extension collision is attributed to
/// the file that declared the row, not to the pack's first file.
///
/// The same class already fixed for commands via `PackCommand::file`. With the
/// row's line published against the wrong document the squiggle lands on
/// whatever sits at that line of a file the author was not editing.
#[test]
fn an_extension_collision_names_the_file_that_declared_the_row() {
    let root = scratch("extension-attribution");
    let alpha = write_pack(
        &root,
        "alpha.tclspec",
        "speclib alpha 1.1 {\n\
        \x20 file_extension shr -name {Alpha's} -dialect tcl9.0\n\
        }\n",
    );
    // One logical pack, two files. The extension row sits several lines into
    // the second, so a notice published against the first file would point at
    // a line that file does not even have.
    let beta_one = write_pack(
        &root,
        "beta-one.tclspec",
        "speclib beta 1.1 {\n\
        \x20 command beta::cmd { arity 1 }\n\
        }\n",
    );
    let beta_two = write_pack(
        &root,
        "beta-two.tclspec",
        "speclib beta 1.1 {\n\
        \x20 command beta::other { arity 1 }\n\
        \x20 command beta::third { arity 1 }\n\
        \x20 file_extension shr -name {Beta's} -dialect tcl8.6\n\
        }\n",
    );
    let set = load_files(&[alpha, beta_one, beta_two]);

    let notice = set
        .notices
        .iter()
        .find(|n| n.message.contains("is already claimed by pack"))
        .expect("the extension collision must still be reported");
    assert!(
        notice.path.ends_with("beta-two.tclspec"),
        "the row was declared in beta-two.tclspec, so the notice belongs there, \
         got {}",
        notice.path.display()
    );
    assert_eq!(
        notice.line, 4,
        "on the `file_extension` row's own line in that file"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// `speclib` with no version word is refused rather than taking the body as
/// the pack name (#1638).
///
/// The name is the merge key and the `name` field every editor is handed in
/// `spec_packs_loaded`, so a multi-line blob there is not a cosmetic problem.
#[test]
fn a_speclib_without_a_version_does_not_become_a_pack_name() {
    let pack = load_pack("speclib {\n command x { arity 1 }\n}\n");

    assert_eq!(pack.name, "", "a braced word is not a pack name");
    assert!(pack.commands.is_empty());
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("needs a name and a vocabulary version")),
        "and the author is told the shape: {:#?}",
        pack.notices
    );
}
