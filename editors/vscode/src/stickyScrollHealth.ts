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

import { ConfigurationTarget, ExtensionContext, OutputChannel, window, workspace } from "vscode";
import { TCL_LANGUAGE_IDS } from "./languageIds";

// Sticky-scroll default-model health check (issue #1122 residuals).
//
// Our `configurationDefaults` block sets `editor.stickyScroll.defaultModel` to
// `foldingProviderModel` for every contributed Tcl language id (see
// `./languageIds` for why those ids are undotted). Two things can defeat that
// default, both states of the same setting and mutually exclusive in practice:
//
//   A. Dropped defaults. A third-party extension shipping its own dotted
//      `[lang.id]` `configurationDefaults` block throws while VS Code builds the
//      default-configuration value tree, aborting the rest of that block --
//      including ours -- and silently reverting Tcl to `outlineModel`.
//   B. Overridden explicit user choice. VS Code precedence makes an extension's
//      `[tcl]` language default beat a user's explicit GLOBAL
//      `editor.stickyScroll.defaultModel`, even though the user chose it on
//      purpose. A language-scoped user choice still wins outright -- only the
//      global case needs help.
//
// Both cases are surfaced (at most once each, ever) via a one-time information
// message offering a one-click fix, persisted in `context.globalState` so we
// never nag twice.

/** The value our `configurationDefaults` block sets for every Tcl language id. */
export const STICKY_SCROLL_DEFAULT_MODEL = "foldingProviderModel";

const STICKY_SCROLL_SECTION = "stickyScroll.defaultModel";

// `getConfiguration(section, { languageId })` needs *a* Tcl language id to
// resolve language-scoped values against; any one of `TCL_LANGUAGE_IDS` would
// do since our `configurationDefaults` and any user language-level override
// are per-id, but `"tcl"` (the base language) is the natural sentinel to read
// through.
const SENTINEL_LANGUAGE_ID = "tcl";

export const STICKY_SCROLL_GLOBAL_CHOICE_KEY = "tclLsp.stickyScrollModelChoice.v1";
export const STICKY_SCROLL_RESTORE_CHOICE_KEY = "tclLsp.stickyScrollRestoreChoice.v1";

const USE_GLOBAL_ACTION = "Use my global setting";
const KEEP_DEFAULT_ACTION = "Keep Tcl default";
const RESTORE_ACTION = "Restore";
const IGNORE_ACTION = "Ignore";

/** The subset of `WorkspaceConfiguration.inspect()` scope values this check consults. */
export interface StickyScrollInspection {
  globalLanguageValue?: string;
  workspaceLanguageValue?: string;
  workspaceFolderLanguageValue?: string;
  globalValue?: string;
}

export type StoredGlobalChoice = "useGlobal" | "keepDefault" | undefined;
export type StoredRestoreChoice = "restore" | "ignore" | undefined;

export interface StickyScrollStoredState {
  globalChoice: StoredGlobalChoice;
  restoreChoice: StoredRestoreChoice;
}

export type StickyScrollDecision =
  | { kind: "languageOverridePresent" }
  | { kind: "expected" }
  | { kind: "promptHonorGlobal"; globalValue: string }
  | { kind: "globalChoiceKept"; globalValue: string }
  | { kind: "globalChoiceHonoured"; globalValue: string }
  | { kind: "promptRestore"; resolvedValue: string | undefined }
  | { kind: "restoreIgnored"; resolvedValue: string | undefined }
  | { kind: "restoreApplied"; resolvedValue: string | undefined };

/**
 * Pure decision for what (if anything) the health check should do, given the
 * `inspect()` result, the resolved effective value, and any previously stored
 * one-time choices. Exported for unit testing.
 *
 * Decision order mirrors precedence:
 *   1. Any language-level value (global, workspace, or workspace-folder scoped)
 *      means the user already made a per-Tcl choice -- do nothing.
 *   2. A global value with no language-level override is case B -- offer to
 *      honour it, or report the previously stored choice.
 *   3. A resolved value that isn't our expected default (and no global value)
 *      is case A -- offer to restore it, or report the previously stored choice.
 *   4. Otherwise everything already resolves as expected.
 */
export function decideStickyScrollAction(
  insp: StickyScrollInspection,
  resolvedValue: string | undefined,
  stored: StickyScrollStoredState,
): StickyScrollDecision {
  if (
    insp.globalLanguageValue !== undefined ||
    insp.workspaceLanguageValue !== undefined ||
    insp.workspaceFolderLanguageValue !== undefined
  ) {
    return { kind: "languageOverridePresent" };
  }

  if (insp.globalValue !== undefined) {
    if (stored.globalChoice === "keepDefault") {
      return { kind: "globalChoiceKept", globalValue: insp.globalValue };
    }
    if (stored.globalChoice === "useGlobal") {
      return { kind: "globalChoiceHonoured", globalValue: insp.globalValue };
    }
    return { kind: "promptHonorGlobal", globalValue: insp.globalValue };
  }

  if (resolvedValue !== STICKY_SCROLL_DEFAULT_MODEL) {
    if (stored.restoreChoice === "ignore") {
      return { kind: "restoreIgnored", resolvedValue };
    }
    if (stored.restoreChoice === "restore") {
      return { kind: "restoreApplied", resolvedValue };
    }
    return { kind: "promptRestore", resolvedValue };
  }

  return { kind: "expected" };
}

/**
 * The side-effecting collaborators the check depends on. Production builds
 * this from the VS Code API and the extension context; tests inject fakes so
 * the dispatch logic can be exercised without real notifications or global
 * settings mutations.
 */
export interface StickyScrollHealthContext {
  inspect(): StickyScrollInspection;
  resolvedValue(): string | undefined;
  storedGlobalChoice(): StoredGlobalChoice;
  storedRestoreChoice(): StoredRestoreChoice;
  storeGlobalChoice(choice: "useGlobal" | "keepDefault"): void;
  storeRestoreChoice(choice: "restore" | "ignore"): void;
  notify(message: string, ...actions: string[]): Thenable<string | undefined>;
  /** Write `value` into every Tcl language id's user-level block (Global scope). */
  applyValueToAllLanguages(value: string): Promise<void>;
  log(line: string): void;
}

/** Run the health check's decision + dispatch against the given collaborators. */
export async function runStickyScrollHealthCheck(ctx: StickyScrollHealthContext): Promise<void> {
  const decision = decideStickyScrollAction(ctx.inspect(), ctx.resolvedValue(), {
    globalChoice: ctx.storedGlobalChoice(),
    restoreChoice: ctx.storedRestoreChoice(),
  });

  switch (decision.kind) {
    case "languageOverridePresent":
    case "expected":
      // Everything is already as expected (or the user made an explicit
      // per-Tcl choice) -- stay quiet, no log noise.
      return;

    case "promptHonorGlobal": {
      const choice = await ctx.notify(
        `You have set editor.stickyScroll.defaultModel to '${decision.globalValue}' globally, but the ` +
          `Tcl LSP extension defaults Tcl files to '${STICKY_SCROLL_DEFAULT_MODEL}', which takes ` +
          "precedence over your global setting for Tcl languages. Use your global setting for Tcl too?",
        USE_GLOBAL_ACTION,
        KEEP_DEFAULT_ACTION,
      );
      if (choice === USE_GLOBAL_ACTION) {
        ctx.storeGlobalChoice("useGlobal");
        try {
          await ctx.applyValueToAllLanguages(decision.globalValue);
          ctx.log(
            `Tcl LSP: sticky scroll -- global editor.stickyScroll.defaultModel is ` +
              `'${decision.globalValue}'; user chose to use it for Tcl too, wrote it to every Tcl ` +
              "language id.",
          );
        } catch (err) {
          ctx.log(
            `Tcl LSP: sticky scroll -- failed to write the user's global ` +
              `editor.stickyScroll.defaultModel ('${decision.globalValue}') to the Tcl language ` +
              `ids: ${String(err)}`,
          );
        }
      } else {
        ctx.storeGlobalChoice("keepDefault");
        ctx.log(
          `Tcl LSP: sticky scroll -- global editor.stickyScroll.defaultModel is ` +
            `'${decision.globalValue}'; user chose to keep the Tcl default ('` +
            `${STICKY_SCROLL_DEFAULT_MODEL}'${choice === undefined ? ", dismissed" : ""}).`,
        );
      }
      return;
    }

    case "globalChoiceKept":
      ctx.log(
        `Tcl LSP: sticky scroll -- global editor.stickyScroll.defaultModel is ` +
          `'${decision.globalValue}'; user previously chose to keep the Tcl default, not asking again.`,
      );
      return;

    case "globalChoiceHonoured":
      ctx.log(
        `Tcl LSP: sticky scroll -- global editor.stickyScroll.defaultModel is ` +
          `'${decision.globalValue}'; user previously chose to honour it for Tcl too.`,
      );
      return;

    case "promptRestore": {
      const choice = await ctx.notify(
        "The Tcl LSP extension's sticky-scroll default for Tcl files appears to have been dropped " +
          "(likely another installed extension shipping an invalid language-scoped configuration " +
          "defaults block). Restore it?",
        RESTORE_ACTION,
        IGNORE_ACTION,
      );
      if (choice === RESTORE_ACTION) {
        ctx.storeRestoreChoice("restore");
        try {
          await ctx.applyValueToAllLanguages(STICKY_SCROLL_DEFAULT_MODEL);
          ctx.log(
            `Tcl LSP: sticky scroll -- editor.stickyScroll.defaultModel for 'tcl' had resolved to ` +
              `'${String(decision.resolvedValue)}', not '${STICKY_SCROLL_DEFAULT_MODEL}'; restored it ` +
              "for every Tcl language id.",
          );
        } catch (err) {
          ctx.log(
            `Tcl LSP: sticky scroll -- failed to restore '${STICKY_SCROLL_DEFAULT_MODEL}' to the Tcl ` +
              `language ids: ${String(err)}`,
          );
        }
      } else {
        ctx.storeRestoreChoice("ignore");
        ctx.log(
          `Tcl LSP: sticky scroll -- editor.stickyScroll.defaultModel for 'tcl' resolved to ` +
            `'${String(decision.resolvedValue)}', not the expected '${STICKY_SCROLL_DEFAULT_MODEL}' -- ` +
            "our configurationDefaults override may have been dropped (e.g. another extension's " +
            "dotted [lang.id] configurationDefaults block aborting VS Code's defaults processing). " +
            `User chose to ignore it${choice === undefined ? " (dismissed)" : ""}, not asking again.`,
        );
      }
      return;
    }

    case "restoreIgnored":
      ctx.log(
        `Tcl LSP: sticky scroll -- editor.stickyScroll.defaultModel for 'tcl' is still ` +
          `'${String(decision.resolvedValue)}', not '${STICKY_SCROLL_DEFAULT_MODEL}'; user previously ` +
          "chose to ignore this, not asking again.",
      );
      return;

    case "restoreApplied":
      ctx.log(
        `Tcl LSP: sticky scroll -- editor.stickyScroll.defaultModel for 'tcl' is still ` +
          `'${String(decision.resolvedValue)}' though the user previously chose to restore ` +
          `'${STICKY_SCROLL_DEFAULT_MODEL}'.`,
      );
      return;
  }
}

/** Write `value` into every Tcl language id's user-level `"[<id>]"` block (Global scope). */
async function applyStickyScrollValueToAllLanguages(value: string): Promise<void> {
  for (const languageId of TCL_LANGUAGE_IDS) {
    // The final `true` is `overrideInLanguage`: it makes the languageId-scoped
    // `getConfiguration` + `update` write into the user-level `"[<id>]"` block
    // rather than the unscoped Global setting.
    await workspace
      .getConfiguration("editor", { languageId })
      .update(STICKY_SCROLL_SECTION, value, ConfigurationTarget.Global, true);
  }
}

function buildProductionStickyScrollContext(
  context: ExtensionContext,
  ch: OutputChannel,
): StickyScrollHealthContext {
  return {
    inspect: () =>
      workspace
        .getConfiguration("editor", { languageId: SENTINEL_LANGUAGE_ID })
        .inspect<string>(STICKY_SCROLL_SECTION) ?? {},
    resolvedValue: () =>
      workspace
        .getConfiguration("editor", { languageId: SENTINEL_LANGUAGE_ID })
        .get<string>(STICKY_SCROLL_SECTION),
    storedGlobalChoice: () =>
      context.globalState.get<StoredGlobalChoice>(STICKY_SCROLL_GLOBAL_CHOICE_KEY),
    storedRestoreChoice: () =>
      context.globalState.get<StoredRestoreChoice>(STICKY_SCROLL_RESTORE_CHOICE_KEY),
    storeGlobalChoice: (choice) =>
      void context.globalState.update(STICKY_SCROLL_GLOBAL_CHOICE_KEY, choice),
    storeRestoreChoice: (choice) =>
      void context.globalState.update(STICKY_SCROLL_RESTORE_CHOICE_KEY, choice),
    notify: (message, ...actions) => window.showInformationMessage(message, ...actions),
    applyValueToAllLanguages: applyStickyScrollValueToAllLanguages,
    log: (line) => ch.appendLine(line),
  };
}

/**
 * One-time health check + repair for `editor.stickyScroll.defaultModel`,
 * called once after `client.start()`. Detects and offers a one-click fix for
 * two states of the same setting (issue #1122 residuals):
 *
 *   - Dropped defaults: our `configurationDefaults` override never landed
 *     (e.g. a third-party extension's dotted `[lang.id]` block aborted VS
 *     Code's defaults processing) -- offers to Restore.
 *   - Overridden explicit choice: the user set a GLOBAL
 *     `editor.stickyScroll.defaultModel`, which our language-scoped default
 *     beats even though a language-scoped user choice would have won --
 *     offers to honour the user's global choice for Tcl too.
 *
 * A user who already made an explicit per-Tcl (language-scoped) choice is
 * left alone entirely -- that always wins and needs no prompt.
 */
export async function ensureStickyScrollDefaultModel(
  context: ExtensionContext,
  ch: OutputChannel,
): Promise<void> {
  await runStickyScrollHealthCheck(buildProductionStickyScrollContext(context, ch));
}
