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

import * as assert from "assert";
import { ConfigurationTarget } from "vscode";
import type { TextDocument } from "vscode";
import {
  DISMISS_LANGUAGE_MISMATCH,
  HealthContext,
  SemanticStatus,
  chooseEnableTarget,
  decideSemanticStatus,
  filterOwnedExtensions,
  resetHighlightingHealthSession,
  runHighlightingHealthChecks,
  tclLanguageIdForExtension,
} from "../highlightingHealth";
import {
  EXTENSION_LANGUAGE_IDS,
  FILENAME_LANGUAGE_IDS,
  isTclLanguage,
  tclLanguageIdForPath,
} from "../languageIds";

// A minimal TextDocument stand-in — the checks only read uri.scheme,
// languageId, and fileName.
function fakeDoc(fileName: string, languageId: string, scheme = "file"): TextDocument {
  return {
    fileName,
    languageId,
    uri: { scheme },
  } as unknown as TextDocument;
}

interface Recorder {
  ctx: HealthContext;
  messages: string[];
  dismissed: Set<string>;
  switched: Array<{ languageId: string }>;
  enabled: SemanticStatus["reason"][];
}

// Build a fake HealthContext that records interactions and answers the notice
// with `answer` (the action label the "user" clicks, or undefined to dismiss
// the toast without choosing).
function makeRecorder(opts: {
  answer?: string;
  owned?: Set<string>;
  semantic?: SemanticStatus;
  preDismissed?: string[];
}): Recorder {
  const dismissed = new Set<string>(opts.preDismissed ?? []);
  const messages: string[] = [];
  const switched: Array<{ languageId: string }> = [];
  const enabled: SemanticStatus["reason"][] = [];
  const ctx: HealthContext = {
    isDismissed: (key) => dismissed.has(key),
    dismiss: (key) => dismissed.add(key),
    ownedExtensions: opts.owned ?? new Set([".tcl", ".irul"]),
    resolveSemantic: () => opts.semantic ?? { effective: true, reason: "on" },
    notify: (message) => {
      messages.push(message);
      return Promise.resolve(opts.answer);
    },
    switchLanguage: (_document, languageId) => switched.push({ languageId }),
    enableSemantic: (reason) => enabled.push(reason),
  };
  return { ctx, messages, dismissed, switched, enabled };
}

suite("Highlighting Health — semantic-status decision", () => {
  test("feature toggle off wins over editor and theme", () => {
    assert.deepStrictEqual(decideSemanticStatus(false, true, true), {
      effective: false,
      reason: "featureOff",
    });
  });

  test("editor.semanticHighlighting.enabled=false reports editorOff", () => {
    assert.deepStrictEqual(decideSemanticStatus(null, false, true), {
      effective: false,
      reason: "editorOff",
    });
  });

  test("editor setting true forces on regardless of theme", () => {
    assert.deepStrictEqual(decideSemanticStatus(null, true, false), {
      effective: true,
      reason: "on",
    });
  });

  test("configuredByTheme with an unsupported theme reports themeUnsupported", () => {
    assert.deepStrictEqual(decideSemanticStatus(null, "configuredByTheme", false), {
      effective: false,
      reason: "themeUnsupported",
    });
  });

  test("configuredByTheme with unknown theme support does not warn", () => {
    assert.deepStrictEqual(decideSemanticStatus(null, "configuredByTheme", undefined), {
      effective: true,
      reason: "on",
    });
  });
});

suite("Highlighting Health — helpers", () => {
  test("owned-extension filter drops generics and lower-cases", () => {
    const owned = filterOwnedExtensions([".tcl", ".TK", ".itcl", ".irul", ".test", ".apl", ".exp"]);
    assert.ok(owned.has(".tcl") && owned.has(".tk") && owned.has(".itcl") && owned.has(".irul"));
    assert.ok(!owned.has(".test") && !owned.has(".apl") && !owned.has(".exp"));
  });

  test("maps file extensions to the most specific Tcl language id", () => {
    assert.strictEqual(tclLanguageIdForExtension(".tcl"), "tcl");
    assert.strictEqual(tclLanguageIdForExtension(".tm"), "tcl");
    assert.strictEqual(tclLanguageIdForExtension(".irul"), "tcl-irule");
    assert.strictEqual(tclLanguageIdForExtension(".irule"), "tcl-irule");
    assert.strictEqual(tclLanguageIdForExtension(".iapp"), "tcl-iapp");
    assert.strictEqual(tclLanguageIdForExtension(".iappimpl"), "tcl-iapp");
  });

  // Issue #1625: the hand-written switch this replaced knew 4 of the 25
  // registered extensions, so "Switch to Tcl" on a `.sdc` file offered plain
  // `tcl` — dropping the file's whole dialect on the way.
  test("covers every registered extension, not just the F5 ones", () => {
    assert.strictEqual(tclLanguageIdForExtension(".sdc"), "tcl-synopsys");
    assert.strictEqual(tclLanguageIdForExtension(".xdc"), "tcl-xilinx");
    assert.strictEqual(tclLanguageIdForExtension(".tmsh"), "tcl-tmsh");
    assert.strictEqual(tclLanguageIdForExtension(".irules"), "tcl-irule");
    assert.strictEqual(tclLanguageIdForExtension(".expect"), "tcl-expect");
    assert.strictEqual(tclLanguageIdForExtension(".tclspec"), "tclspec");
    // Case-folded, and an extension we do not own still falls back to `tcl`.
    assert.strictEqual(tclLanguageIdForExtension(".SDC"), "tcl-synopsys");
    assert.strictEqual(tclLanguageIdForExtension(".rs"), "tcl");
  });
});

suite("Language-id projection", () => {
  test("resolves a path by whole basename before extension", () => {
    // The BIG-IP config files are claimed by *name*: a bare `.conf` belongs
    // to every unrelated config file, so the extension tier must not see it.
    assert.strictEqual(tclLanguageIdForPath("/config/bigip.conf"), "tcl-bigip");
    assert.strictEqual(tclLanguageIdForPath("C:\\config\\BIGIP_BASE.CONF"), "tcl-bigip");
    assert.strictEqual(tclLanguageIdForPath("/etc/httpd.conf"), undefined);
    // The extension tier still answers for everything else.
    assert.strictEqual(tclLanguageIdForPath("/w/foo.irules"), "tcl-irule");
    assert.strictEqual(tclLanguageIdForPath("foo.tcl"), "tcl");
    assert.strictEqual(tclLanguageIdForPath("Makefile"), undefined);
  });

  test("a filename that names an Object.prototype member resolves to nothing", () => {
    // The lookup keys are filenames, so a plain `map[key]` answers for
    // `constructor` / `__proto__` / `toString` with something inherited — a
    // file called `constructor` would resolve to a *function* posing as a
    // language id, and be handed to `setTextDocumentLanguage`.
    for (const name of ["constructor", "__proto__", "toString", "hasOwnProperty"]) {
      assert.strictEqual(tclLanguageIdForPath(`/w/${name}`), undefined, name);
      assert.strictEqual(tclLanguageIdForExtension(`.${name}`), "tcl", name);
    }
  });

  test("every registered language id is one we recognise as Tcl", () => {
    // The two generated maps and the generated id set are projections of one
    // catalogue; a mapping that named an id outside `TCL_LANGUAGE_IDS` would
    // associate a file with a language our own client refuses to attach to.
    for (const id of [
      ...Object.values(EXTENSION_LANGUAGE_IDS),
      ...Object.values(FILENAME_LANGUAGE_IDS),
    ]) {
      assert.ok(isTclLanguage(id), `${id} is not a Tcl language id`);
    }
    assert.ok(Object.keys(EXTENSION_LANGUAGE_IDS).length >= 24);
  });
});

suite("Highlighting Health — enable-target selection", () => {
  const isFalse = (v: unknown) => v === false;
  const noScope = { hasFolder: false, hasWorkspace: false };

  test("overrides the narrowest scope that forces the value off, not Global", () => {
    const t = chooseEnableTarget({ workspaceValue: false, globalValue: false }, isFalse, {
      hasFolder: false,
      hasWorkspace: true,
    });
    assert.strictEqual(t.target, ConfigurationTarget.Workspace);
    assert.strictEqual(t.overrideInLanguage, false);
  });

  test("prefers a language-scoped off value over a resource value at the same scope", () => {
    const t = chooseEnableTarget(
      { workspaceFolderLanguageValue: false, workspaceValue: false },
      isFalse,
      { hasFolder: true, hasWorkspace: true },
    );
    assert.strictEqual(t.target, ConfigurationTarget.WorkspaceFolder);
    assert.strictEqual(t.overrideInLanguage, true);
  });

  test("falls back to the narrowest writable scope when nothing is explicitly off", () => {
    assert.strictEqual(
      chooseEnableTarget({}, isFalse, { hasFolder: true, hasWorkspace: true }).target,
      ConfigurationTarget.WorkspaceFolder,
    );
    assert.strictEqual(
      chooseEnableTarget(undefined, isFalse, { hasFolder: false, hasWorkspace: true }).target,
      ConfigurationTarget.Workspace,
    );
    assert.strictEqual(chooseEnableTarget({}, isFalse, noScope).target, ConfigurationTarget.Global);
  });

  test("treats configuredByTheme as off for the editor-setting predicate", () => {
    const isEditorOff = (v: unknown) => v === false || v === "configuredByTheme";
    const t = chooseEnableTarget({ workspaceValue: "configuredByTheme" }, isEditorOff, {
      hasFolder: false,
      hasWorkspace: true,
    });
    assert.strictEqual(t.target, ConfigurationTarget.Workspace);
  });
});

suite("Highlighting Health — language mismatch check", () => {
  setup(() => resetHighlightingHealthSession());

  test("offers to switch a mis-associated Tcl file and switches on accept", async () => {
    const r = makeRecorder({ answer: "Switch to Tcl" });
    await runHighlightingHealthChecks(fakeDoc("/w/mpfit.tcl", "plaintext"), r.ctx);
    assert.strictEqual(r.messages.length, 1);
    assert.match(r.messages[0], /not Tcl/);
    assert.deepStrictEqual(r.switched, [{ languageId: "tcl" }]);
  });

  test("records a permanent dismissal on 'Don't show again'", async () => {
    const r = makeRecorder({ answer: "Don't show again" });
    await runHighlightingHealthChecks(fakeDoc("/w/x.irul", "plaintext"), r.ctx);
    assert.ok(r.dismissed.has(DISMISS_LANGUAGE_MISMATCH));
    assert.strictEqual(r.switched.length, 0);
  });

  test("stays silent for a correctly-associated Tcl file", async () => {
    const r = makeRecorder({ answer: "Switch to Tcl" });
    await runHighlightingHealthChecks(fakeDoc("/w/ok.tcl", "tcl"), r.ctx);
    assert.strictEqual(r.messages.length, 0);
  });

  test("stays silent for a foreign extension we don't own", async () => {
    const r = makeRecorder({ answer: "Switch to Tcl" });
    await runHighlightingHealthChecks(fakeDoc("/w/notes.md", "markdown"), r.ctx);
    assert.strictEqual(r.messages.length, 0);
  });

  test("does not warn again once permanently dismissed", async () => {
    const r = makeRecorder({ answer: "Switch to Tcl", preDismissed: [DISMISS_LANGUAGE_MISMATCH] });
    await runHighlightingHealthChecks(fakeDoc("/w/mpfit.tcl", "plaintext"), r.ctx);
    assert.strictEqual(r.messages.length, 0);
  });
});

suite("Highlighting Health — semantic-off check", () => {
  setup(() => resetHighlightingHealthSession());

  test("warns and enables when semantic highlighting is off", async () => {
    const r = makeRecorder({
      answer: "Enable",
      semantic: { effective: false, reason: "editorOff" },
    });
    await runHighlightingHealthChecks(fakeDoc("/w/a.tcl", "tcl"), r.ctx);
    assert.strictEqual(r.messages.length, 1);
    assert.match(r.messages[0], /semanticHighlighting\.enabled is off/);
    assert.deepStrictEqual(r.enabled, ["editorOff"]);
  });

  test("themeUnsupported reason surfaces the theme wording", async () => {
    const r = makeRecorder({
      answer: undefined,
      semantic: { effective: false, reason: "themeUnsupported" },
    });
    await runHighlightingHealthChecks(fakeDoc("/w/a.tcl", "tcl"), r.ctx);
    assert.match(r.messages[0], /theme doesn't support semantic highlighting/);
    assert.strictEqual(r.enabled.length, 0);
  });

  test("stays silent when the overlay is effective", async () => {
    const r = makeRecorder({ answer: "Enable", semantic: { effective: true, reason: "on" } });
    await runHighlightingHealthChecks(fakeDoc("/w/a.tcl", "tcl"), r.ctx);
    assert.strictEqual(r.messages.length, 0);
  });
});
