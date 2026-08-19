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

// SpecTcl pack torture, through the extension host.
//
// `rust/tcl-spectcl/tests/pack_torture.rs` is the loader-altitude half of this:
// it proves no `.tclspec`, however malformed, can panic or hang the parser. It
// cannot prove the thing a user actually cares about, which is that none of it
// takes *the editor* with it — the loader runs on a worker inside a live
// server, its result is published into a per-profile registry every open
// document is analysed against, and a `.tclspec` is itself a watched file that
// reloads the whole set on every save.
//
// So this suite drives the pack through the surface it really lives on: a
// scratch `.tclspec` inside the workspace, named by `tclLsp.specPacks`, edited
// / broken / fixed / deleted / renamed while a document using its commands is
// open. After every hostile write it asks the server three questions it must
// still be able to answer, so a wedge is attributed to the write that caused it
// rather than to whichever later test happened to time out first.
//
// The pack lives under `specPackTortureScratch/` — a plain directory, not
// `.tcl-lsp/`, so nothing here is discovered by convention and the suite has
// exclusive control over when the pack exists. That also keeps every other
// suite's registry untouched: no test outside this file ever sees
// `torturepack`.

import * as assert from "assert";
import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";

import {
  activate,
  bounded,
  getDocUri,
  getServerLogSize,
  pollUntil,
  serverTransportWedged,
  waitForEffectiveConfig,
  waitForServerLog,
} from "./helper";

/** The scratch directory the suite owns; created in `suiteSetup`, removed in
 *  `suiteTeardown`, and git-ignored so a crashed run leaves no tracked drift. */
const SCRATCH = path.resolve(__dirname, "../../testFixture/specPackTortureScratch");
/** The one pack file the churn tests rewrite. */
const PACK = path.join(SCRATCH, "torture.tclspec");
/** The pack name every assertion keys on — the bundled tier is in
 *  `spec_packs_loaded` too, so "any pack at all" is true from the first
 *  reload and useless as a barrier. */
const PACK_NAME = "torturepack";
/** A phrase that appears in the pack's hover and nowhere else in the repo. */
const HOVER_MARKER = "TORTURE-PACK-HOVER-MARKER";

const docUri = getDocUri("specPackTorture.tcl");
/** A document no test drives — the liveness probe's own fixture. Answering a
 *  hover here means the server's document pipeline is draining generally, not
 *  merely that the consumer's queue happens to be alive. */
const probeUri = getDocUri("livenessProbe.tcl");

/**
 * A well-formed pack, stamped with `generation` in its hover summary.
 *
 * The stamp is what makes "which write is in effect" observable. Without it
 * every well-formed generation is byte-identical in the fields a test can
 * read, so a convergence assertion would be satisfied by *any* of them — a
 * stale publish two generations old would pass. `gen=<generation>` reaches the
 * hover, so a test can name the exact write it expects to win.
 */
function goodPack(generation: string): string {
  return `speclib ${PACK_NAME} 1.1 {

command ::torturepack::apply {
    dialects tcl8.6+
    arity 2
    required_package torturelib

    arg 0 -role VarWrite
    arg 1 -role Body

    form Default {::torturepack::apply resultVar script}

    hover {
        summary {${HOVER_MARKER} gen=${generation} — evaluate a script collecting into a caller variable.}
        synopsis {::torturepack::apply resultVar script}
    }
}

}
`;
}

/** The baseline well-formed pack every hostile write is measured against. */
const GOOD_PACK = goodPack("base");

/** Opt-in for the manual-tier edit storm below.
 *
 *  Matches the Rust corpus sweep's `SPECTCL_CORPUS_TMP` convention: any
 *  non-empty value other than `0` counts, so `SPECTCL_EXT_EDIT_STORM=1` reads
 *  the way it is documented without `=0` silently meaning the opposite. */
const EDIT_STORM_ENV = "SPECTCL_EXT_EDIT_STORM";
function editStormOptedIn(): boolean {
  const value = process.env[EDIT_STORM_ENV];
  return value !== undefined && value !== "" && value !== "0";
}

/**
 * The hostile battery, each entry a complete file body.
 *
 * Every one of these is something a real editor can put on disk: a save
 * mid-keystroke, a Windows editor's BOM, a CRLF checkout, a bad paste, a
 * find-and-replace that ate a brace. The contract is not that any of them
 * loads — most cannot — but that the server survives each one and is still
 * answering afterwards.
 */
const HOSTILE: ReadonlyArray<{ label: string; body: string }> = [
  { label: "empty file", body: "" },
  { label: "a single open brace", body: "{" },
  { label: "a single close brace", body: "}" },
  { label: "no speclib wrapper", body: "command ::torturepack::apply { arity 2 }\n" },
  {
    label: "truncated mid-statement",
    body: `speclib ${PACK_NAME} 1.1 {\n  command ::torturepack::apply {\n    arity 2\n    arg 0 -role`,
  },
  {
    label: "unbalanced speclib body",
    body: `speclib ${PACK_NAME} 1.1 {\n  command ::torturepack::apply { arity 2 }\n`,
  },
  {
    label: "unbalanced command body",
    body: `speclib ${PACK_NAME} 1.1 {\n  command ::torturepack::apply { arity 2\n}\n`,
  },
  {
    label: "a UTF-8 BOM before speclib",
    body: `﻿speclib ${PACK_NAME} 1.1 {\n  command ::torturepack::apply { arity 2 }\n}\n`,
  },
  {
    label: "CRLF line endings",
    body: `speclib ${PACK_NAME} 1.1 {\r\n  command ::torturepack::apply { arity 2 }\r\n}\r\n`,
  },
  {
    label: "NUL bytes inside the body",
    body: `speclib ${PACK_NAME} 1.1 {\0  command ::torturepack::apply\0{ arity 2 }\n}\n`,
  },
  {
    label: "an unknown vocabulary version",
    body: `speclib ${PACK_NAME} 99.99 {\n  command ::torturepack::apply { arity 2 }\n}\n`,
  },
  {
    label: "unknown words at every level",
    body:
      `speclib ${PACK_NAME} 1.1 {\n  no_such_pack_word {x}\n` +
      `  command ::torturepack::apply {\n    arity 2\n    no_such_command_word 3\n` +
      `    arg 0 -role NoSuchRole\n    hover { summary {${HOVER_MARKER}} no_such_hover_word {x} }\n  }\n}\n`,
  },
  {
    label: "an invalid lifecycle ordering",
    body:
      `speclib ${PACK_NAME} 1.1 {\n  command ::torturepack::apply {\n    arity 2\n` +
      `    required_package torturelib\n    introduced_version 9.0\n    retired_version 1.0\n  }\n}\n`,
  },
  {
    label: "a command name a megabyte long",
    body: `speclib ${PACK_NAME} 1.1 {\n  command ${"x".repeat(1_000_000)} { arity 1 }\n}\n`,
  },
  {
    label: "braces nested five thousand deep",
    body:
      `speclib ${PACK_NAME} 1.1 {\n  command ::torturepack::apply {\n    arity 2\n` +
      `    hover { summary {${"{".repeat(5_000)}x${"}".repeat(5_000)}} }\n  }\n}\n`,
  },
  {
    label: "Tcl injection shapes inside a hover summary",
    body:
      `speclib ${PACK_NAME} 1.1 {\n  command ::torturepack::apply {\n    arity 2\n` +
      `    hover { summary {[exec /bin/sh -c {touch ${path.join(SCRATCH, "pwned")}}] ` +
      `\${env(HOME)} $::argv %s %n} }\n  }\n}\n`,
  },
];

/** Write `body` to the pack file and wait for the server to finish the reload
 *  it triggers.
 *
 *  The barrier is the server's own `SpecTcl: N pack(s), …` line, which
 *  `reload_spec_packs` logs after publishing — a **positive** marker, so it
 *  cannot be satisfied vacuously the way "poll until the command is gone"
 *  could be by a reload that has not started yet. It is logged only when the
 *  pack set's content key actually moved, which is why every entry in the
 *  battery is a distinct byte sequence. */
async function writePack(body: string, label: string): Promise<void> {
  const since = getServerLogSize();
  fs.writeFileSync(PACK, body, "utf8");
  await waitForServerLog((line) => line.includes("SpecTcl:"), {
    since,
    timeout: 40_000,
    label: `the pack reload triggered by writing ${label}`,
  });
}

/** Remove the pack file and wait for the reload that follows. */
async function removePack(label: string): Promise<void> {
  const since = getServerLogSize();
  fs.rmSync(PACK, { force: true });
  await waitForServerLog((line) => line.includes("SpecTcl:"), {
    since,
    timeout: 40_000,
    label: `the pack reload triggered by ${label}`,
  });
}

/**
 * The three questions a live server must still answer, asked after each
 * hostile write.
 *
 * Deliberately the same three the harness's own liveness probe asks (helper.ts,
 * issue #1294): a document-free config pull proves the transport is alive, a
 * hover on an undriven document proves the document pipeline is draining, and a
 * hover on the consumer proves *this* document's queue is not wedged. Asking
 * them here, immediately, is what makes a wedge attributable to the pack that
 * caused it — without this the first symptom would be some later test's
 * timeout, which is exactly the unattributable shape #1600 recorded.
 */
async function assertServerAlive(label: string): Promise<void> {
  assert.ok(
    !serverTransportWedged(),
    `the server transport was already confirmed wedged before checking ${label}`,
  );
  const cfg = await bounded(
    vscode.commands.executeCommand("tcl-lsp.getEffectiveConfig", docUri.toString()),
    `a document-free getEffectiveConfig after ${label}`,
    { timeout: 20_000 },
  );
  assert.ok(cfg, `getEffectiveConfig returned nothing after ${label}`);
  await bounded(
    vscode.commands.executeCommand(
      "vscode.executeHoverProvider",
      probeUri,
      new vscode.Position(0, 0),
    ),
    `a hover on an undriven document after ${label}`,
    { timeout: 20_000 },
  );
  await bounded(
    vscode.commands.executeCommand(
      "vscode.executeHoverProvider",
      docUri,
      new vscode.Position(0, 0),
    ),
    `a hover on the pack's consumer after ${label}`,
    { timeout: 20_000 },
  );
}

/** The text of every hover at `position` in the consumer document. */
async function hoverTextAt(position: vscode.Position): Promise<string> {
  const hovers = (await vscode.commands.executeCommand(
    "vscode.executeHoverProvider",
    docUri,
    position,
  )) as vscode.Hover[] | undefined;
  return (hovers ?? [])
    .flatMap((hover) => hover.contents)
    .map((content) => (typeof content === "string" ? content : content.value))
    .join("\n");
}

/** Position of the `::torturepack::apply` command word in the fixture. */
const COMMAND_POSITION = new vscode.Position(12, 2);

/** Wait until the server reports `torturepack` loaded with at least one
 *  command — the positive barrier for "the good pack is in effect". */
async function awaitPackLoaded(label: string): Promise<void> {
  await waitForEffectiveConfig(
    docUri,
    (cfg) => cfg.spec_packs_loaded.some((pack) => pack.name === PACK_NAME && pack.commands > 0),
    { timeout: 40_000, label },
  );
}

/** Wait until the pack's hover marker is visible — the barrier that proves the
 *  reload reached the *analysis*, not merely the pack set. */
async function awaitHoverMarker(label: string): Promise<void> {
  await pollUntil(
    () => hoverTextAt(COMMAND_POSITION),
    (text) => text.includes(HOVER_MARKER),
    {
      timeout: 40_000,
      label,
    },
  );
}

/**
 * Wait until the hover shows exactly `generation`, then assert no other
 * generation's stamp is present.
 *
 * The equality is the point. `awaitHoverMarker` only proves *some* well-formed
 * generation is in effect, which any of a burst's writes would satisfy — this
 * names the one that must have won.
 */
async function awaitHoverGeneration(generation: string, label: string): Promise<void> {
  const stamp = `gen=${generation}`;
  const text = await pollUntil(
    () => hoverTextAt(COMMAND_POSITION),
    (t) => t.includes(stamp),
    {
      timeout: 40_000,
      label,
    },
  );
  const others = [...text.matchAll(/gen=([A-Za-z0-9-]+)/g)].map((m) => m[1]);
  assert.deepStrictEqual(
    [...new Set(others)],
    [generation],
    `the hover must show only generation ${generation}, got ${JSON.stringify(others)}`,
  );
}

suite("SpecTcl pack torture through the extension host", () => {
  let originalSetting: string[] | undefined;

  // One barrier for the whole suite, in `suiteSetup` rather than per test.
  //
  // The pattern is #1622's: a per-test wait cannot serve here because the
  // marker each wait keys on is emitted per *reload*, and the suite reuses one
  // consumer document throughout — re-activating an already-open, unedited
  // document starts no new analysis and so produces no new marker. Getting the
  // extension activated, the document open and drained, and the good pack in
  // effect once, up front, is what lets every test below start from a known
  // state.
  suiteSetup(async function () {
    this.timeout(180_000);
    fs.mkdirSync(SCRATCH, { recursive: true });
    fs.writeFileSync(PACK, GOOD_PACK, "utf8");

    const config = vscode.workspace.getConfiguration("tclLsp", docUri);
    originalSetting = config.inspect<string[]>("specPacks")?.workspaceValue;
    // A directory, not a file: discovery's `collect_path` scans it, so the
    // add / delete / rename tests can move files around underneath a setting
    // that never changes.
    await config.update("specPacks", [SCRATCH], vscode.ConfigurationTarget.Workspace);

    await activate(docUri);
    await awaitPackLoaded("the torture pack to load at suite start");
    await awaitHoverMarker("the torture pack's hover to reach the consumer");
  });

  suiteTeardown(async function () {
    this.timeout(60_000);
    const config = vscode.workspace.getConfiguration("tclLsp", docUri);
    try {
      await config.update("specPacks", originalSetting, vscode.ConfigurationTarget.Workspace);
    } catch {
      // Best effort — the fixture-settings snapshot in index.ts restores the
      // committed bytes regardless.
    }
    fs.rmSync(SCRATCH, { recursive: true, force: true });
  });

  test("the whole hostile battery leaves the server answering", async function () {
    // Each entry costs a full workspace pack reload plus three liveness
    // questions, so the backstop is sized for the battery rather than for one
    // write. Every wait inside is individually bounded and names itself.
    this.timeout(HOSTILE.length * 60_000 + 120_000);

    for (const { label, body } of HOSTILE) {
      await writePack(body, label);
      await assertServerAlive(label);
    }

    // Surviving is half the contract. The other half is that the server is
    // still *useful*: put the good pack back and the command must resolve
    // again, which a server that had quietly stopped reloading could not do.
    await writePack(GOOD_PACK, "the good pack, restored after the battery");
    await awaitPackLoaded("the pack to reload after the hostile battery");
    await awaitHoverMarker("the pack's hover to return after the hostile battery");

    assert.ok(
      !fs.existsSync(path.join(SCRATCH, "pwned")),
      "loading a pack executed the Tcl inside its `hover` summary",
    );
  });

  test("a pack broken and then fixed loses and regains its command", async function () {
    this.timeout(180_000);

    await writePack(
      `speclib ${PACK_NAME} 1.1 {\n  command ::torturepack::apply { arity 2\n`,
      "a pack truncated mid-body",
    );
    // Absence, read off a converged state: `awaitPackLoaded` cannot be the
    // barrier here (the pack is gone from the report), so the barrier is the
    // reload marker `writePack` already waited on, and the assertion is that
    // the *hover* — which is what the user sees — has lost the marker.
    await pollUntil(
      () => hoverTextAt(COMMAND_POSITION),
      (text) => !text.includes(HOVER_MARKER),
      { timeout: 40_000, label: "the pack's hover to disappear once the pack is broken" },
    );
    await assertServerAlive("a pack truncated mid-body");

    await writePack(GOOD_PACK, "the repaired pack");
    await awaitPackLoaded("the repaired pack to load");
    await awaitHoverMarker("the repaired pack's hover to come back");
  });

  test("a second write immediately after a first wins", async function () {
    // The minimal stale-publish shape, and the one that belongs in CI: two
    // writes with no wait between them, so the first reload is still reading
    // the disk when the second lands. A reload takes its sequence number
    // *before* it reads (`PublishedPackSet::seq`), so the earlier snapshot must
    // never publish over the later one however long it then spends parsing.
    //
    // The generation stamps are what make this a real assertion: both writes
    // are well-formed, so without them "some good pack is loaded" would be
    // true of the loser too. `awaitHoverGeneration` demands `gen=second` and
    // no other stamp — a stale publish leaves `gen=first` and fails.
    this.timeout(180_000);

    const since = getServerLogSize();
    fs.writeFileSync(PACK, goodPack("first"), "utf8");
    fs.writeFileSync(PACK, goodPack("second"), "utf8");

    await waitForServerLog((line) => line.includes("SpecTcl:"), {
      since,
      timeout: 40_000,
      label: "a reload from the two back-to-back writes",
    });
    await assertServerAlive("two back-to-back pack writes");

    await awaitPackLoaded("the pack set to converge on the second write");
    await awaitHoverGeneration("second", "the hover to converge on the second write");

    // Leave the suite's baseline in place for whatever runs next.
    await writePack(GOOD_PACK, "the baseline pack, restored");
    await awaitHoverGeneration("base", "the baseline hover to return");
  });

  // Manual tier. The body is a generated edit storm, which `AGENTS.md`
  // ("Fuzzing is always manual") keeps out of the automatic tiers "regardless
  // of how fast it happens to be today" — the deterministic two-write test
  // above is the CI cover for the same window. Opt in the way the Rust corpus
  // sweep does (`SPECTCL_CORPUS_TMP`), with an env var:
  //
  //     SPECTCL_EXT_EDIT_STORM=1 make test-ext
  test("a twenty-write edit storm still converges on the last write", async function () {
    if (!editStormOptedIn()) {
      this.skip();
    }
    this.timeout(300_000);

    const since = getServerLogSize();
    let last = "";
    for (let i = 0; i < 20; i++) {
      // Alternate broken and well-formed so consecutive writes cannot coalesce
      // into one content key, and stamp every well-formed one so the winner is
      // identifiable rather than merely well-formed.
      if (i % 2 === 0) {
        fs.writeFileSync(
          PACK,
          `speclib ${PACK_NAME} 1.1 {\n  command ::torturepack::apply { arity ${i}\n`,
          "utf8",
        );
      } else {
        last = `storm-${i}`;
        fs.writeFileSync(PACK, goodPack(last), "utf8");
      }
    }
    // End on a well-formed generation whose stamp nothing else in the burst
    // used, so convergence is checkable.
    last = "storm-final";
    fs.writeFileSync(PACK, goodPack(last), "utf8");

    await waitForServerLog((line) => line.includes("SpecTcl:"), {
      since,
      timeout: 40_000,
      label: "at least one reload from the edit storm",
    });
    await assertServerAlive("a twenty-write edit storm");

    await awaitPackLoaded("the pack set to converge after the storm");
    await awaitHoverGeneration(last, "the hover to converge on the storm's final write");

    await writePack(GOOD_PACK, "the baseline pack, restored after the storm");
    await awaitHoverGeneration("base", "the baseline hover to return after the storm");
  });

  test("deleting the pack retires its command, and re-creating it brings it back", async function () {
    this.timeout(180_000);

    await removePack("deleting the pack file");
    await pollUntil(
      () => hoverTextAt(COMMAND_POSITION),
      (text) => !text.includes(HOVER_MARKER),
      { timeout: 40_000, label: "the pack's hover to retire once the file is deleted" },
    );
    await assertServerAlive("deleting the pack file");

    await writePack(GOOD_PACK, "re-creating the pack file");
    await awaitPackLoaded("the re-created pack to load");
    await awaitHoverMarker("the re-created pack's hover");
  });

  test("renaming the pack file keeps the pack loaded under its new path", async function () {
    this.timeout(180_000);

    const renamed = path.join(SCRATCH, "renamed.tclspec");
    const since = getServerLogSize();
    fs.renameSync(PACK, renamed);
    await waitForServerLog((line) => line.includes("SpecTcl:"), {
      since,
      timeout: 40_000,
      label: "the reload triggered by renaming the pack file",
    });
    await assertServerAlive("renaming the pack file");

    // The pack is a logical unit, not a file: its name comes from `speclib`,
    // so a rename must be invisible to everything downstream.
    await awaitPackLoaded("the renamed pack to still be loaded");
    await awaitHoverMarker("the renamed pack's hover to survive the rename");
    const cfg = await waitForEffectiveConfig(
      docUri,
      (c) => c.spec_packs_loaded.some((p) => p.name === PACK_NAME),
      { timeout: 40_000, label: "the renamed pack in the effective config" },
    );
    const loaded = cfg.spec_packs_loaded.find((p) => p.name === PACK_NAME);
    assert.ok(
      loaded?.files.some((f) => f.endsWith("renamed.tclspec")),
      `the pack must be reported at its new path, got ${JSON.stringify(loaded?.files)}`,
    );

    // Restore the suite's invariant for whatever runs next.
    const back = getServerLogSize();
    fs.renameSync(renamed, PACK);
    await waitForServerLog((line) => line.includes("SpecTcl:"), {
      since: back,
      timeout: 40_000,
      label: "the reload restoring the original pack path",
    });
    await awaitPackLoaded("the pack at its original path");
  });

  test("a pack with thousands of commands loads and the editor stays responsive", async function () {
    this.timeout(300_000);

    const commands = 2_000;
    let body = `speclib ${PACK_NAME} 1.1 {\n`;
    // The consumer's own command has to stay in the pack, or the restore at the
    // end of this test is the only thing proving the reload happened.
    body += GOOD_PACK.split("\n").slice(1, -2).join("\n");
    for (let i = 0; i < commands; i++) {
      body +=
        `\ncommand ::torturepack::bulk${i} {\n` +
        `    arity 2\n` +
        `    arg 0 -role Value\n` +
        `    option -opt${i} -detail {Option ${i}.}\n` +
        `    subcommand sub${i} { arity 0 }\n` +
        `    hover { summary {Bulk command ${i}.} }\n` +
        `}\n`;
    }
    body += "\n}\n";

    const started = Date.now();
    await writePack(body, `a pack declaring ${commands} commands`);
    const elapsed = Date.now() - started;

    await assertServerAlive(`a pack declaring ${commands} commands`);
    await waitForEffectiveConfig(
      docUri,
      (cfg) => cfg.spec_packs_loaded.some((p) => p.name === PACK_NAME && p.commands > commands),
      { timeout: 60_000, label: `all ${commands} bulk commands to be reported loaded` },
    );
    // The pack's original command must still work — a bulk load that dropped
    // it would still satisfy the count above.
    await awaitHoverMarker("the hover to survive a bulk pack load");

    console.log(`[specPackTorture] ${commands} commands loaded in ${elapsed}ms`);

    await writePack(GOOD_PACK, "the small pack, restored after the bulk load");
    await awaitPackLoaded("the small pack to load after the bulk one");
  });

  test("load notices from a broken pack reach the Problems panel and clear when fixed", async function () {
    this.timeout(180_000);

    // An unknown property is the cleanest notice to assert on: the design
    // promises it is dropped *with a notice*, and unlike a truncation it leaves
    // the rest of the pack loading, so the pack is still reported and the
    // notice is unambiguously about the one bad row.
    await writePack(
      GOOD_PACK.replace("    arity 2\n", "    arity 2\n    definitely_not_a_spectcl_word 42\n"),
      "a pack with one unknown property",
    );

    const packUri = vscode.Uri.file(PACK);
    const diagnostics = await pollUntil(
      () => vscode.languages.getDiagnostics(packUri),
      (diags) => diags.length > 0,
      { timeout: 40_000, label: "a load notice on the pack file" },
    );
    assert.ok(
      diagnostics.some((d) => d.message.includes("definitely_not_a_spectcl_word")),
      `the notice must name the offending word, got: ${JSON.stringify(
        diagnostics.map((d) => d.message),
      )}`,
    );
    // The rest of the pack still loaded — degradation is per-declaration.
    await awaitPackLoaded("the pack to load despite the unknown property");
    await awaitHoverMarker("the hover to survive an unknown property beside it");

    await writePack(GOOD_PACK, "the pack with the unknown property removed");
    await pollUntil(
      () => vscode.languages.getDiagnostics(packUri),
      (diags) => !diags.some((d) => d.message.includes("definitely_not_a_spectcl_word")),
      { timeout: 40_000, label: "the load notice to clear once the pack is fixed" },
    );
  });
});
