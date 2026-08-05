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
import {
  STICKY_SCROLL_DEFAULT_MODEL,
  StickyScrollHealthContext,
  StickyScrollInspection,
  StoredGlobalChoice,
  StoredRestoreChoice,
  decideStickyScrollAction,
  runStickyScrollHealthCheck,
} from "../stickyScrollHealth";

// Sticky Scroll Health -- decision logic
//
// Decision matrix (issue #1122 residuals):
//   1. Any language-level value (global/workspace/workspace-folder scoped)
//      -> the user made an explicit per-Tcl choice -> no action, ever.
//   2. Else a global value present (no language-level override) -> case B,
//      "overridden explicit user choice" -> offer to honour it.
//   3. Else a resolved value that isn't foldingProviderModel -> case A,
//      "dropped defaults" -> offer to restore it.
//   4. Else -> everything already resolves as expected -> no action.
// A stored one-time choice for cases 2/3 suppresses the prompt on repeat runs.

const NO_STORED: { globalChoice: StoredGlobalChoice; restoreChoice: StoredRestoreChoice } = {
  globalChoice: undefined,
  restoreChoice: undefined,
};

suite("Sticky Scroll Health -- decision", () => {
  test("a language-level global value means no action, regardless of anything else", () => {
    const insp: StickyScrollInspection = {
      globalLanguageValue: "outlineModel",
      globalValue: "indentation",
    };
    assert.deepStrictEqual(decideStickyScrollAction(insp, "indentation", NO_STORED), {
      kind: "languageOverridePresent",
    });
  });

  test("a language-level workspace value means no action", () => {
    const insp: StickyScrollInspection = { workspaceLanguageValue: "outlineModel" };
    assert.deepStrictEqual(decideStickyScrollAction(insp, "outlineModel", NO_STORED), {
      kind: "languageOverridePresent",
    });
  });

  test("a language-level workspace-folder value means no action", () => {
    const insp: StickyScrollInspection = { workspaceFolderLanguageValue: "outlineModel" };
    assert.deepStrictEqual(decideStickyScrollAction(insp, "outlineModel", NO_STORED), {
      kind: "languageOverridePresent",
    });
  });

  test("a global value with no language override offers to honour it", () => {
    const insp: StickyScrollInspection = { globalValue: "indentation" };
    assert.deepStrictEqual(decideStickyScrollAction(insp, STICKY_SCROLL_DEFAULT_MODEL, NO_STORED), {
      kind: "promptHonorGlobal",
      globalValue: "indentation",
    });
  });

  test("a global value honours the stored 'keepDefault' choice quietly", () => {
    const insp: StickyScrollInspection = { globalValue: "indentation" };
    const stored = { globalChoice: "keepDefault" as const, restoreChoice: undefined };
    assert.deepStrictEqual(decideStickyScrollAction(insp, STICKY_SCROLL_DEFAULT_MODEL, stored), {
      kind: "globalChoiceKept",
      globalValue: "indentation",
    });
  });

  test("a global value honours the stored 'useGlobal' choice quietly", () => {
    const insp: StickyScrollInspection = { globalValue: "indentation" };
    const stored = { globalChoice: "useGlobal" as const, restoreChoice: undefined };
    assert.deepStrictEqual(decideStickyScrollAction(insp, STICKY_SCROLL_DEFAULT_MODEL, stored), {
      kind: "globalChoiceHonoured",
      globalValue: "indentation",
    });
  });

  test("a dropped default (no global value, resolved != expected) offers to restore", () => {
    const insp: StickyScrollInspection = {};
    assert.deepStrictEqual(decideStickyScrollAction(insp, "outlineModel", NO_STORED), {
      kind: "promptRestore",
      resolvedValue: "outlineModel",
    });
  });

  test("a dropped default with no resolved value at all also offers to restore", () => {
    const insp: StickyScrollInspection = {};
    assert.deepStrictEqual(decideStickyScrollAction(insp, undefined, NO_STORED), {
      kind: "promptRestore",
      resolvedValue: undefined,
    });
  });

  test("a dropped default honours the stored 'ignore' choice quietly", () => {
    const insp: StickyScrollInspection = {};
    const stored = { globalChoice: undefined, restoreChoice: "ignore" as const };
    assert.deepStrictEqual(decideStickyScrollAction(insp, "outlineModel", stored), {
      kind: "restoreIgnored",
      resolvedValue: "outlineModel",
    });
  });

  test("a dropped default honours the stored 'restore' choice quietly", () => {
    const insp: StickyScrollInspection = {};
    const stored = { globalChoice: undefined, restoreChoice: "restore" as const };
    assert.deepStrictEqual(decideStickyScrollAction(insp, "outlineModel", stored), {
      kind: "restoreApplied",
      resolvedValue: "outlineModel",
    });
  });

  test("everything resolving as expected means no action", () => {
    const insp: StickyScrollInspection = {};
    assert.deepStrictEqual(decideStickyScrollAction(insp, STICKY_SCROLL_DEFAULT_MODEL, NO_STORED), {
      kind: "expected",
    });
  });

  test("a global value takes priority over a merely-dropped resolved value", () => {
    // Both a global value AND a non-default resolved value are present --
    // case B (global override) must win the branch, not case A.
    const insp: StickyScrollInspection = { globalValue: "indentation" };
    assert.deepStrictEqual(decideStickyScrollAction(insp, "outlineModel", NO_STORED), {
      kind: "promptHonorGlobal",
      globalValue: "indentation",
    });
  });
});

// Sticky Scroll Health -- side effects
//
// Exercises `runStickyScrollHealthCheck` against a fake context that records
// every collaborator interaction, so the notification/update dispatch can be
// verified without a real VS Code settings store.

interface Recorder {
  ctx: StickyScrollHealthContext;
  messages: Array<{ message: string; actions: string[] }>;
  logs: string[];
  applied: string[];
  globalChoice: StoredGlobalChoice;
  restoreChoice: StoredRestoreChoice;
}

function makeRecorder(opts: {
  insp: StickyScrollInspection;
  resolvedValue: string | undefined;
  answer?: string;
  initialGlobalChoice?: StoredGlobalChoice;
  initialRestoreChoice?: StoredRestoreChoice;
  applyShouldThrow?: boolean;
}): Recorder {
  const messages: Array<{ message: string; actions: string[] }> = [];
  const logs: string[] = [];
  const applied: string[] = [];
  const r: Recorder = {
    messages,
    logs,
    applied,
    globalChoice: opts.initialGlobalChoice,
    restoreChoice: opts.initialRestoreChoice,
    ctx: undefined as unknown as StickyScrollHealthContext,
  };
  r.ctx = {
    inspect: () => opts.insp,
    resolvedValue: () => opts.resolvedValue,
    storedGlobalChoice: () => r.globalChoice,
    storedRestoreChoice: () => r.restoreChoice,
    storeGlobalChoice: (choice) => {
      r.globalChoice = choice;
    },
    storeRestoreChoice: (choice) => {
      r.restoreChoice = choice;
    },
    notify: (message, ...actions) => {
      messages.push({ message, actions });
      return Promise.resolve(opts.answer);
    },
    applyValueToAllLanguages: async (value) => {
      if (opts.applyShouldThrow) {
        throw new Error("settings file is readonly");
      }
      applied.push(value);
    },
    log: (line) => logs.push(line),
  };
  return r;
}

suite("Sticky Scroll Health -- side effects", () => {
  test("language-level override: no notification, no log, no write", async () => {
    const r = makeRecorder({
      insp: { globalLanguageValue: "outlineModel" },
      resolvedValue: "outlineModel",
      answer: "Use my global setting",
    });
    await runStickyScrollHealthCheck(r.ctx);
    assert.strictEqual(r.messages.length, 0);
    assert.strictEqual(r.logs.length, 0);
    assert.strictEqual(r.applied.length, 0);
  });

  test("expected state: no notification, no log, no write", async () => {
    const r = makeRecorder({
      insp: {},
      resolvedValue: STICKY_SCROLL_DEFAULT_MODEL,
      answer: "Use my global setting",
    });
    await runStickyScrollHealthCheck(r.ctx);
    assert.strictEqual(r.messages.length, 0);
    assert.strictEqual(r.logs.length, 0);
    assert.strictEqual(r.applied.length, 0);
  });

  test("global override, 'Use my global setting': writes to all languages and stores the choice", async () => {
    const r = makeRecorder({
      insp: { globalValue: "indentation" },
      resolvedValue: STICKY_SCROLL_DEFAULT_MODEL,
      answer: "Use my global setting",
    });
    await runStickyScrollHealthCheck(r.ctx);
    assert.strictEqual(r.messages.length, 1);
    assert.deepStrictEqual(r.messages[0].actions, ["Use my global setting", "Keep Tcl default"]);
    assert.deepStrictEqual(r.applied, ["indentation"]);
    assert.strictEqual(r.globalChoice, "useGlobal");
    assert.strictEqual(r.logs.length, 1);
  });

  test("global override, 'Keep Tcl default': no write, stores the choice", async () => {
    const r = makeRecorder({
      insp: { globalValue: "indentation" },
      resolvedValue: STICKY_SCROLL_DEFAULT_MODEL,
      answer: "Keep Tcl default",
    });
    await runStickyScrollHealthCheck(r.ctx);
    assert.strictEqual(r.applied.length, 0);
    assert.strictEqual(r.globalChoice, "keepDefault");
    assert.strictEqual(r.logs.length, 1);
  });

  test("global override, dismissed (no answer): treated as keep, stores the choice", async () => {
    const r = makeRecorder({
      insp: { globalValue: "indentation" },
      resolvedValue: STICKY_SCROLL_DEFAULT_MODEL,
      answer: undefined,
    });
    await runStickyScrollHealthCheck(r.ctx);
    assert.strictEqual(r.applied.length, 0);
    assert.strictEqual(r.globalChoice, "keepDefault");
    assert.strictEqual(r.logs.length, 1);
  });

  test("global override with a stored 'keepDefault' choice: no re-prompt, still logs once", async () => {
    const r = makeRecorder({
      insp: { globalValue: "indentation" },
      resolvedValue: STICKY_SCROLL_DEFAULT_MODEL,
      answer: "Use my global setting", // would accept if (wrongly) prompted again
      initialGlobalChoice: "keepDefault",
    });
    await runStickyScrollHealthCheck(r.ctx);
    assert.strictEqual(r.messages.length, 0, "must not prompt again");
    assert.strictEqual(r.applied.length, 0);
    assert.strictEqual(r.logs.length, 1);
  });

  test("dropped default, 'Restore': writes the default to all languages and stores the choice", async () => {
    const r = makeRecorder({
      insp: {},
      resolvedValue: "outlineModel",
      answer: "Restore",
    });
    await runStickyScrollHealthCheck(r.ctx);
    assert.strictEqual(r.messages.length, 1);
    assert.deepStrictEqual(r.messages[0].actions, ["Restore", "Ignore"]);
    assert.deepStrictEqual(r.applied, [STICKY_SCROLL_DEFAULT_MODEL]);
    assert.strictEqual(r.restoreChoice, "restore");
    assert.strictEqual(r.logs.length, 1);
  });

  test("dropped default, 'Ignore': no write, stores the choice", async () => {
    const r = makeRecorder({
      insp: {},
      resolvedValue: "outlineModel",
      answer: "Ignore",
    });
    await runStickyScrollHealthCheck(r.ctx);
    assert.strictEqual(r.applied.length, 0);
    assert.strictEqual(r.restoreChoice, "ignore");
    assert.strictEqual(r.logs.length, 1);
  });

  test("dropped default with a stored 'ignore' choice: no re-prompt, still logs once", async () => {
    const r = makeRecorder({
      insp: {},
      resolvedValue: "outlineModel",
      answer: "Restore", // would accept if (wrongly) prompted again
      initialRestoreChoice: "ignore",
    });
    await runStickyScrollHealthCheck(r.ctx);
    assert.strictEqual(r.messages.length, 0, "must not prompt again");
    assert.strictEqual(r.applied.length, 0);
    assert.strictEqual(r.logs.length, 1);
  });

  test("a write failure (e.g. readonly settings file) is caught and logged, never thrown", async () => {
    const r = makeRecorder({
      insp: {},
      resolvedValue: "outlineModel",
      answer: "Restore",
      applyShouldThrow: true,
    });
    await assert.doesNotReject(runStickyScrollHealthCheck(r.ctx));
    assert.strictEqual(r.restoreChoice, "restore");
    assert.ok(
      r.logs.some((l) => l.includes("readonly") || l.includes("failed")),
      `expected a failure log line, got ${JSON.stringify(r.logs)}`,
    );
  });

  test("re-running after repair (language-level value now present) goes quiet", async () => {
    // Simulates the post-restore state: the write from a prior "Restore" (or
    // "Use my global setting") landed as a globalLanguageValue, so this run's
    // inspect() now reports a language-level override and step 2 fires.
    const r = makeRecorder({
      insp: { globalLanguageValue: STICKY_SCROLL_DEFAULT_MODEL },
      resolvedValue: STICKY_SCROLL_DEFAULT_MODEL,
      answer: "Restore",
    });
    await runStickyScrollHealthCheck(r.ctx);
    assert.strictEqual(r.messages.length, 0);
    assert.strictEqual(r.logs.length, 0);
    assert.strictEqual(r.applied.length, 0);
  });
});
