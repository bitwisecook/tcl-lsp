// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

import * as vscode from "vscode";
import {
  applyDocumentTransform,
  captureDocumentTransform,
  isDocumentTransformCurrent,
  requestDocumentTransform,
} from "./documentTransform";

/** Catalogue values are explicit authoring choices, never existing symbol identities. */
export async function insertPackageRequirement(
  editor: vscode.TextEditor,
  catalogue: () => Promise<unknown>,
): Promise<void> {
  const snapshot = captureDocumentTransform(editor.document);
  if (!snapshot) return;
  let response: unknown;
  try {
    response = await catalogue();
  } catch {
    return;
  }
  if (!isDocumentTransformCurrent(snapshot) || !response || typeof response !== "object") return;
  const values = (response as { packages?: unknown }).packages;
  if (!Array.isArray(values) || !values.every((value) => typeof value === "string")) return;
  if (values.length === 0) {
    vscode.window.showInformationMessage(
      "No known packages found. Configure tclLsp.libraryPaths to enable package discovery.",
    );
    return;
  }
  const choices = values.map((packageValue: string) => ({ label: packageValue, packageValue }));
  const picked = await vscode.window.showQuickPick(choices, {
    placeHolder: "Select package to require",
    title: "Insert package require",
  });
  if (!picked || !choices.includes(picked) || !isDocumentTransformCurrent(snapshot)) return;
  const result = await requestDocumentTransform<{ source: string; alreadyRequired: boolean }>(
    snapshot,
    "tcl-lsp.insertPackageRequire",
    [picked.packageValue],
  );
  if (!result || !isDocumentTransformCurrent(snapshot)) return;
  // Even unchanged results consume their original server admission before success.
  const applied = await applyDocumentTransform(snapshot, result);
  vscode.window.showInformationMessage(
    applied
      ? result.alreadyRequired
        ? `'package require ${picked.packageValue}' already exists.`
        : `Inserted 'package require ${picked.packageValue}'.`
      : "Failed to insert package require.",
  );
}
