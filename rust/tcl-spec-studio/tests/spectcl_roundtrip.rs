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

//! The gate for the `SpecTcl` renderer: draft → `.tclspec` → draft.
//!
//! [`render_spectcl`](tcl_spec_studio::render_spectcl) is the inverse of
//! [`spectcl::load_pack`](tcl_spec_studio::spectcl::load_pack), so the honest
//! test of it is the round trip, run over the *whole* command surface rather
//! than a handful of examples: every command in every browsable dialect is
//! drafted from the live registry, rendered to `SpecTcl`, loaded back, drafted
//! again, and the two drafts are compared **field by field** — the same
//! comparison `tests/spectcl_ports.rs` makes between a ported pack and the
//! shipped spec it was ported from.
//!
//! ## What "unequal" is allowed to mean
//!
//! Exactly the keys [`render_spectcl::GAPS`] names, and nothing else. Each is
//! one of three kinds, and each kind is a different claim:
//!
//! - [`GapKind::DraftOpaque`] — the DSL *has* a spelling and the loader reads
//!   it, but the draft model records the field as "set, expression not
//!   recovered" ([`draft::UNRENDERABLE_KEY`]), because the spec field is a
//!   function pointer or a reference to a named `&'static` descriptor. The
//!   loss is the draft's, not the DSL's.
//! - [`GapKind::LoaderGap`] — the draft holds the value and the design memo
//!   documents a spelling, but no loader reader exists yet.
//! - [`GapKind::Excluded`] — a pack may not author the field at all.
//!
//! **Native hooks are deliberately not in that list.** A hook field renders as
//! `field -native ID`, the loader installs its family's abstention, and
//! seeding records the reloaded field exactly as it recorded the shipped one —
//! so the eight function-pointer families round-trip *equal*, and the gate
//! checks that they do rather than excusing them.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde_json::Value;
use tcl_spec_studio::render_spectcl::{self, GapKind};
use tcl_spec_studio::{browsable_dialects, command_names, draft, load_command, schema, spectcl};

/// One field that did not survive the round trip.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Diff {
    key: String,
    rendered: String,
    shipped: String,
}

fn short(value: &Value) -> String {
    let text = value.to_string();
    if text.chars().count() > 200 {
        let head: String = text.chars().take(180).collect();
        format!("{head}… ({} bytes)", text.len())
    } else {
        text
    }
}

fn compare_keys(
    rendered: &Value,
    shipped: &Value,
    keys: impl Iterator<Item = &'static str>,
    prefix: &str,
    out: &mut Vec<Diff>,
) {
    for key in keys {
        let a = rendered.get(key).unwrap_or(&Value::Null);
        let b = shipped.get(key).unwrap_or(&Value::Null);
        if a != b {
            out.push(Diff {
                key: format!("{prefix}{key}"),
                rendered: short(a),
                shipped: short(b),
            });
        }
    }
}

/// The `__unrenderable` lists compared as sets, so the report names the keys
/// that moved rather than printing two arrays.
fn compare_unrenderable(rendered: &Value, shipped: &Value, prefix: &str, out: &mut Vec<Diff>) {
    let keys = |value: &Value| -> BTreeSet<String> {
        value
            .get(draft::UNRENDERABLE_KEY)
            .and_then(Value::as_array)
            .map(|keys| {
                keys.iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    let (a, b) = (keys(rendered), keys(shipped));
    for key in b.difference(&a) {
        out.push(Diff {
            key: format!("{prefix}{key}"),
            rendered: "not set".to_owned(),
            shipped: "set (unrecoverable)".to_owned(),
        });
    }
    for key in a.difference(&b) {
        out.push(Diff {
            key: format!("{prefix}{key}"),
            rendered: "set (unrecoverable)".to_owned(),
            shipped: "not set".to_owned(),
        });
    }
}

/// The bare field name behind a `subcommands[foo].bar` diff key.
fn field_of(key: &str) -> &str {
    key.rsplit('.').next().unwrap_or(key)
}

/// The object a diff key names — the draft itself, or the subcommand inside it.
fn at_level<'a>(key: &str, draft: &'a Value) -> Option<&'a Value> {
    let Some(rest) = key.strip_prefix("subcommands[") else {
        return Some(draft);
    };
    let name = rest.split(']').next()?;
    draft
        .get("subcommands")?
        .as_array()?
        .iter()
        .find(|sub| sub["name"].as_str() == Some(name))
}

/// Compare a rendered-and-reloaded draft against the one it was rendered from.
fn diffs(rendered: &Value, shipped: &Value) -> Vec<Diff> {
    let mut out = Vec::new();
    compare_keys(
        rendered,
        shipped,
        schema::COMMAND_FIELDS.iter().map(|field| field.key),
        "",
        &mut out,
    );
    compare_unrenderable(rendered, shipped, "", &mut out);

    // `subcommands` is compared as a whole above; when the two agree on the
    // set of names, compare each one's own fields so the report says which
    // subcommand and which key rather than dumping two large arrays.
    let names = |value: &Value| -> Vec<String> {
        value
            .get("subcommands")
            .and_then(Value::as_array)
            .map(|subs| {
                subs.iter()
                    .map(|sub| sub["name"].as_str().unwrap_or("").to_owned())
                    .collect()
            })
            .unwrap_or_default()
    };
    let (a, b) = (names(rendered), names(shipped));
    if a == b {
        out.retain(|diff| diff.key != "subcommands");
        for (i, name) in a.iter().enumerate() {
            let x = &rendered["subcommands"][i];
            let y = &shipped["subcommands"][i];
            let prefix = format!("subcommands[{name}].");
            compare_keys(
                x,
                y,
                schema::SUBCOMMAND_FIELDS.iter().map(|field| field.key),
                &prefix,
                &mut out,
            );
            compare_unrenderable(x, y, &prefix, &mut out);
        }
    }
    out
}

/// Whether a notice is the loader's *policy report* rather than a degradation.
///
/// Naming a lowering or codegen hook is reported by design — "this changes how
/// the compiler translates the command, not just what the editor knows about
/// it" — so a pack the renderer wrote faithfully still raises it. Every other
/// notice means a declaration was dropped, and the gate fails on those.
fn is_policy_report(message: &str) -> bool {
    message.contains("names a lowering hook") || message.contains("names a codegen hook")
}

/// What one command's round trip produced.
struct Trip {
    /// The draft the reloaded pack seeds.
    reloaded: Value,
    /// Loader notices that mean something was *dropped* — the policy reports
    /// are filtered out below.
    notices: Vec<String>,
    /// The rendered pack.
    text: String,
    /// Fields the renderer said up front it could not carry.
    losses: Vec<render_spectcl::Loss>,
}

/// Render `shipped`, load the pack back, and report what happened.
fn round_trip(shipped: &Value) -> Trip {
    let draft = shipped
        .as_object()
        .expect("a draft is a JSON object")
        .clone();
    let pack_name = draft
        .get(draft::SOURCE_DIALECT_KEY)
        .and_then(Value::as_str)
        .unwrap_or("pack")
        .to_owned();
    let (text, losses) = render_spectcl::render_pack_reporting(&[draft], &pack_name);
    let pack = spectcl::load_pack(&text);
    let name = shipped["name"].as_str().unwrap_or_default();
    // Naming a lowering or codegen hook is *reported* by design — "this
    // changes how the compiler translates the command" — so it is a policy
    // report about a pack the renderer wrote faithfully, not a degradation.
    // Every other notice means something was dropped, and the gate fails on it.
    let notices: Vec<String> = pack
        .notices
        .iter()
        .filter(|notice| !is_policy_report(&notice.message))
        .map(ToString::to_string)
        .collect();
    let reloaded = pack.command(name).map_or(Value::Null, |command| {
        Value::Object(draft::from_command_spec(command.spec))
    });
    Trip {
        reloaded,
        notices,
        text,
        losses,
    }
}

/// Whether an `arg_roles` difference is exactly the `CommandPrefix` roles the
/// DSL *implies*.
///
/// `arg N -appends {Exactly 2}` implies `-role CommandPrefix` — README says so,
/// and the loader does it — so a spec that declares a command-prefix position
/// with **no** role at that index cannot be said in `SpecTcl`: reloading always
/// finds the implied role. That is an expressiveness gap in the DSL rather
/// than a renderer bug, and it is the only shape of `arg_roles` difference the
/// gate tolerates.
fn only_implied_command_prefix_roles(rendered: &Value, shipped: &Value) -> bool {
    let prefixes: BTreeSet<u64> = rendered
        .get("command_prefixes")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry["index"].as_u64())
                .collect()
        })
        .unwrap_or_default();
    let roles = |value: &Value| -> Vec<Value> {
        value
            .get("arg_roles")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let shipped_roles = roles(shipped);
    let implied: Vec<Value> = roles(rendered)
        .into_iter()
        .filter(|entry| {
            !(entry["role"] == "CommandPrefix"
                && entry["index"]
                    .as_u64()
                    .is_some_and(|i| prefixes.contains(&i))
                && !shipped_roles
                    .iter()
                    .any(|other| other["index"] == entry["index"]))
        })
        .collect();
    implied == shipped_roles && !prefixes.is_empty()
}

/// Arity windows survive render → load → re-seed (issue #1627).
///
/// The whole-surface trip above cannot cover them: every shipped spec has
/// empty `arity_windows`, so the field is absent from every draft it visits
/// and a renderer that dropped the rows entirely would still pass. This
/// drives one command that actually has them.
#[test]
fn arity_windows_survive_the_round_trip() {
    use tcl_registry::arity::{Arity, ArityWindow};
    use tcl_registry::lifecycle::Lifecycle;

    const WINDOWS: &[ArityWindow] = &[
        ArityWindow {
            lifecycle: Lifecycle {
                introduced: Some("3.0"),
                deprecated: None,
                retired: Some("5.0"),
                deprecation_fix: None,
            },
            arity: Arity::exact(2),
        },
        ArityWindow {
            lifecycle: Lifecycle {
                introduced: Some("5.0"),
                deprecated: None,
                retired: None,
                deprecation_fix: None,
            },
            arity: Arity::new(1, 3),
        },
    ];

    let spec = tcl_registry::CommandSpec {
        name: "probe::grew",
        arity: Arity::exact(1),
        arity_windows: WINDOWS,
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let draft = Value::Object(draft::from_command_spec(&spec));
    let trip = round_trip(&draft);

    assert!(trip.notices.is_empty(), "{:?}\n{}", trip.notices, trip.text);
    assert!(
        trip.text.contains("arity 2 -introduced 3.0 -retired 5.0"),
        "the closed window is rendered with both releases:\n{}",
        trip.text
    );
    assert!(
        trip.text.contains("arity 1..3 -introduced 5.0"),
        "the open window renders no -retired:\n{}",
        trip.text
    );

    let windows = trip.reloaded["arity_windows"]
        .as_array()
        .expect("reloaded draft carries arity_windows");
    assert_eq!(windows.len(), 2, "{windows:?}\n{}", trip.text);
    assert_eq!(windows[0]["arity"]["max"], serde_json::json!(2));
    assert_eq!(
        windows[0]["lifecycle"]["introduced"],
        serde_json::json!("3.0")
    );
    assert_eq!(windows[0]["lifecycle"]["retired"], serde_json::json!("5.0"));
    assert_eq!(windows[1]["arity"]["min"], serde_json::json!(1));
    assert_eq!(windows[1]["arity"]["max"], serde_json::json!(3));
    assert_eq!(windows[1]["lifecycle"]["retired"], Value::Null);

    // The plain arity is untouched by the windows beside it.
    assert_eq!(trip.reloaded["arity"]["min"], serde_json::json!(1));
    assert_eq!(trip.reloaded["arity"]["max"], serde_json::json!(1));
}

/// **The gate.** Every command of every browsable dialect renders to `SpecTcl`,
/// loads back, and drafts equal — except on the documented [`GAPS`].
#[test]
#[allow(clippy::too_many_lines)]
fn every_command_in_every_dialect_round_trips_through_spectcl() {
    let mut report = String::new();
    let mut failures: Vec<String> = Vec::new();
    let mut total = 0usize;
    let mut clean = 0usize;
    // key → (commands affected, one example)
    let mut lossy: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let mut noticed: BTreeMap<String, (usize, String)> = BTreeMap::new();

    for (dialect, _) in browsable_dialects() {
        let mut dialect_total = 0usize;
        let mut dialect_clean = 0usize;
        for name in command_names(dialect) {
            let shipped = load_command(name, dialect)
                .unwrap_or_else(|| panic!("{name} is listed in {dialect} but does not load"));
            let trip = round_trip(&shipped);
            total += 1;
            dialect_total += 1;

            assert!(
                !trip.reloaded.is_null(),
                "{dialect}/{name}: the rendered pack does not declare the command:\n{}",
                trip.text
            );

            for notice in &trip.notices {
                let entry = noticed
                    .entry(notice_shape(notice))
                    .or_insert_with(|| (0, format!("{dialect}/{name}: {notice}")));
                entry.0 += 1;
            }

            let found = diffs(&trip.reloaded, &shipped);
            if found.is_empty() {
                clean += 1;
                dialect_clean += 1;
                continue;
            }
            let reported: BTreeSet<&str> =
                trip.losses.iter().map(|loss| loss.key.as_str()).collect();
            for diff in &found {
                let field = field_of(&diff.key);
                // Three ways a difference is legitimate: the field is in the
                // renderer's own gap register, the renderer said at render
                // time that it could not carry it, or it is the implied
                // command-prefix role.
                let documented = render_spectcl::gap(field)
                    .map(|gap| format!("{field} ({:?})", gap.kind))
                    .or_else(|| {
                        reported
                            .contains(field)
                            .then(|| format!("{field} (unwritable text)"))
                    })
                    .or_else(|| {
                        (field == "arg_roles"
                            && at_level(&diff.key, &trip.reloaded)
                                .zip(at_level(&diff.key, &shipped))
                                .is_some_and(|(a, b)| only_implied_command_prefix_roles(a, b)))
                        .then(|| "arg_roles (implied by -appends)".to_owned())
                    });
                match documented {
                    Some(bucket) => {
                        let entry = lossy
                            .entry(bucket)
                            .or_insert_with(|| (0, format!("{dialect}/{name}")));
                        entry.0 += 1;
                    }
                    None => {
                        failures.push(format!(
                            "{dialect}/{name}: {} \n        rendered: {}\n        shipped : {}",
                            diff.key, diff.rendered, diff.shipped
                        ));
                    }
                }
            }
        }
        let _ = writeln!(
            report,
            "{dialect:>10}: {dialect_clean:>4} / {dialect_total:>4} commands round-trip with no \
             difference at all ({:.1}%)",
            100.0 * f64::from(u32::try_from(dialect_clean).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(dialect_total.max(1)).unwrap_or(u32::MAX))
        );
    }

    let _ = writeln!(
        report,
        "\n     total: {clean} / {total} commands round-trip with no difference at all"
    );
    let _ = writeln!(report, "\nDocumented losses, by field:");
    for (key, (count, example)) in &lossy {
        let _ = writeln!(report, "  {count:>5}  {key}   e.g. {example}");
    }
    if !noticed.is_empty() {
        let _ = writeln!(report, "\nLoader notices raised by rendered packs:");
        for (shape, (count, example)) in &noticed {
            let _ = writeln!(report, "  {count:>5}  {shape}   e.g. {example}");
        }
    }
    let _ = writeln!(report, "\nGap register (render_spectcl::GAPS):");
    for gap in render_spectcl::GAPS {
        let _ = writeln!(
            report,
            "  {:<28} {:?}  {}",
            gap.key,
            gap.kind,
            match gap.kind {
                GapKind::DraftOpaque | GapKind::LoaderGap => gap.spelling,
                GapKind::Excluded => "excluded from what a pack may author",
            }
        );
    }

    println!("{report}");
    assert!(
        failures.is_empty(),
        "{} undocumented round-trip difference(s), {} shown:\n{}\n{report}",
        failures.len(),
        failures.len().min(40),
        failures
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        noticed.is_empty(),
        "a rendered pack must load without notices; got:\n{}\n{report}",
        noticed
            .iter()
            .map(|(shape, (count, example))| format!("  {count:>5}  {shape}   e.g. {example}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// A notice's shape — its message with the specific word stripped — so the
/// report groups thousands of notices into a handful of causes.
fn notice_shape(notice: &str) -> String {
    let message = notice.rsplit(": ").next().unwrap_or(notice);
    let mut out = String::new();
    let mut in_quote = false;
    for c in message.chars() {
        if c == '`' {
            in_quote = !in_quote;
            out.push('`');
            if in_quote {
                out.push('…');
            }
            continue;
        }
        if !in_quote {
            out.push(c);
        }
    }
    out
}

/// The eleven ported packs are the style exemplars, so rendering their own
/// drafts back out is the closest thing to a diff against a hand-written file.
///
/// Differences are expected and allowed — a renderer writes what a draft
/// holds, not what an author's comments and blank lines said — so this reports
/// them rather than asserting on them. What it *does* assert is that the
/// rendered text loads again and produces the same draft: the round-trip gate,
/// applied to the packs the syntax was designed from.
#[test]
fn the_eleven_port_fixtures_render_and_reload_as_themselves() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/design/spec-dsl-examples");
    let mut report = String::new();
    let mut failures: Vec<String> = Vec::new();

    let mut files: Vec<String> = std::fs::read_dir(&dir)
        .expect("the spec-dsl-examples directory")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".tclspec"))
        .collect();
    files.sort();
    assert_eq!(files.len(), 11, "the eleven ports: {files:?}");

    for file in &files {
        let source = std::fs::read_to_string(dir.join(file)).expect("a port");
        let hand = spectcl::load_pack(&source);
        let drafts: Vec<draft::Draft> = hand
            .commands
            .iter()
            .map(|command| draft::from_command_spec(command.spec))
            .collect();
        let rendered = render_spectcl::render_pack(&drafts, &hand.name);
        let reloaded = spectcl::load_pack(&rendered);

        let hand_lines = source.lines().count();
        let rendered_lines = rendered.lines().count();
        let _ = writeln!(
            report,
            "\n=== {file}: hand-written {hand_lines} lines, rendered {rendered_lines} lines, \
             {} notice(s) on reload",
            reloaded.notices.len()
        );
        for notice in reloaded
            .notices
            .iter()
            .filter(|notice| !is_policy_report(&notice.message))
        {
            failures.push(format!("{file}: reloading the rendered pack: {notice}"));
        }

        for command in &hand.commands {
            let name = command.spec.name;
            let before = Value::Object(draft::from_command_spec(command.spec));
            let after = reloaded.command(name).map_or_else(
                || panic!("{file}: the rendered pack lost `{name}`"),
                |c| Value::Object(draft::from_command_spec(c.spec)),
            );
            let found = diffs(&after, &before);
            let _ = writeln!(report, "  {name}: {} differing field(s)", found.len());
            for diff in &found {
                let field = field_of(&diff.key);
                let documented = render_spectcl::gap(field).is_some();
                let _ = writeln!(
                    report,
                    "    {} {}\n        rendered: {}\n        hand    : {}",
                    if documented { "[documented]" } else { "[FAIL]" },
                    diff.key,
                    diff.rendered,
                    diff.shipped
                );
                if !documented {
                    failures.push(format!("{file}/{name}: {}", diff.key));
                }
            }
            // The style check: where the renderer's output differs in *shape*
            // from the hand-written file, say so rather than pretend.
            let hand_statements = statement_words(&source, name);
            let rendered_statements = statement_words(&rendered, name);
            let only_hand: Vec<&String> = hand_statements
                .difference(&rendered_statements)
                .take(12)
                .collect();
            let only_rendered: Vec<&String> = rendered_statements
                .difference(&hand_statements)
                .take(12)
                .collect();
            if !only_hand.is_empty() || !only_rendered.is_empty() {
                let _ = writeln!(
                    report,
                    "    style: statements only in the hand-written file: {only_hand:?}\n    \
                     style: statements only in the rendered file:     {only_rendered:?}"
                );
            }
        }
    }

    println!("{report}");
    assert!(
        failures.is_empty(),
        "{} undocumented difference(s) rendering the ports:\n{}\n{report}",
        failures.len(),
        failures.join("\n")
    );
}

/// The set of property words used inside a pack's command blocks, for the
/// shape comparison. Deliberately crude: it is a report, not an assertion.
fn statement_words(source: &str, _command: &str) -> BTreeSet<String> {
    source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_whitespace().next())
        .filter(|word| {
            word.chars()
                .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
        })
        .map(ToOwned::to_owned)
        .collect()
}

/// A second-level subcommand that carries its own option table is written as a
/// **block**, and the block survives the round trip (issue #1610).
///
/// `namespace ensemble create` and `configure` are the case the field exists
/// for: two different C option tables (`ensembleCreateOptions` /
/// `ensembleConfigOptions`, `tclEnsemble.c`), so a pack that could only say
/// one of them would silently re-merge the bug the split removed.
#[test]
fn a_sub_subcommands_own_option_table_survives_the_round_trip() {
    let shipped = load_command("namespace", "tcl").expect("tcl has `namespace`");
    let trip = round_trip(&shipped);

    assert!(
        trip.text.contains("sub_subcommand create") && trip.text.contains("option -command"),
        "`create`'s own table must be written out:\n{}",
        trip.text
    );
    assert!(
        trip.text.contains("sub_subcommand configure") && trip.text.contains("option -namespace"),
        "`configure`'s own table must be written out:\n{}",
        trip.text
    );
    // `exists` declares none, so it stays a flag row: its statement never
    // opens a block, however the row happens to wrap.
    assert!(
        !trip
            .text
            .lines()
            .any(|l| l.trim_start().starts_with("sub_subcommand exists") && l.ends_with('{')),
        "an operation with no table of its own keeps the flag row:\n{}",
        trip.text
    );

    let reloaded = &trip.reloaded;
    let ensemble = reloaded["subcommands"]
        .as_array()
        .expect("subcommands is a list")
        .iter()
        .find(|sub| sub["name"] == "ensemble")
        .expect("`namespace ensemble` survives");
    let ops = ensemble["sub_subcommands"]
        .as_array()
        .expect("sub_subcommands is a list");
    let names_of = |op: &str| -> Vec<String> {
        ops.iter()
            .find(|row| row["name"] == op)
            .and_then(|row| row["options"].as_array())
            .map(|opts| {
                opts.iter()
                    .map(|o| o["name"].as_str().unwrap_or_default().to_owned())
                    .collect()
            })
            .unwrap_or_default()
    };
    assert_eq!(
        names_of("create"),
        vec![
            "-command",
            "-map",
            "-parameters",
            "-prefixes",
            "-subcommands",
            "-unknown"
        ],
    );
    assert_eq!(
        names_of("configure"),
        vec![
            "-map",
            "-namespace",
            "-parameters",
            "-prefixes",
            "-subcommands",
            "-unknown"
        ],
    );
    assert!(names_of("exists").is_empty());
}

/// An **empty** option block is a claim of its own and survives as one.
///
/// `sub_subcommand exists {}` says "this operation takes no options"; writing
/// no block at all says "this operation declares nothing, use the
/// subcommand's". If the DSL could only spell the second, the round trip would
/// quietly turn `namespace ensemble exists`'s empty table back into an
/// inheriting one and re-offer it the parent's union (issue #1610, Codex
/// review).
#[test]
fn an_empty_sub_subcommand_option_block_means_empty_not_inherit() {
    let shipped = load_command("namespace", "tcl").expect("tcl has `namespace`");
    let trip = round_trip(&shipped);

    assert!(
        trip.text
            .lines()
            .any(|l| l.trim_start().starts_with("sub_subcommand exists")
                && l.trim_end().ends_with("{}")),
        "`exists`'s empty table must be written as an empty block:\n{}",
        trip.text
    );

    let ops = trip.reloaded["subcommands"]
        .as_array()
        .expect("subcommands is a list")
        .iter()
        .find(|sub| sub["name"] == "ensemble")
        .expect("`namespace ensemble` survives")["sub_subcommands"]
        .as_array()
        .expect("sub_subcommands is a list")
        .clone();
    let options_of = |op: &str| -> Value {
        ops.iter()
            .find(|row| row["name"] == op)
            .map_or(Value::Null, |row| row["options"].clone())
    };
    assert_eq!(
        options_of("exists"),
        Value::Array(vec![]),
        "`exists` reloads as an empty table, not a null one"
    );
    assert!(
        options_of("create")
            .as_array()
            .is_some_and(|o| !o.is_empty()),
        "`create` still reloads with its own table"
    );
}
