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

/**
 * The `Tcl: Select Dialect` quick pick, shared by the node and browser entries:
 * one heading per kind ("Dialects", "Tcl + packages"), each dialect showing its
 * display name, its canonical name and its one-line description.
 */

import { QuickPickItem, QuickPickItemKind, window } from "vscode";
import { DialectChoice, describeDialect, groupDialectChoices } from "./dialectChoices";

/** A dialect row, or a group heading (no `value`). */
export interface DialectPickItem extends QuickPickItem {
  value?: string;
}

export function dialectPickItems(choices: readonly DialectChoice[]): DialectPickItem[] {
  const items: DialectPickItem[] = [];
  for (const group of groupDialectChoices(choices)) {
    items.push({ label: group.title, kind: QuickPickItemKind.Separator });
    for (const choice of group.choices) {
      items.push({
        label: choice.label,
        description: choice.name,
        detail: choice.description,
        value: choice.name,
      });
    }
  }
  return items;
}

/** Ask which dialect to use; resolves to the canonical name, or `undefined` if dismissed. */
export async function pickDialect(
  choices: readonly DialectChoice[],
  current: string,
): Promise<string | undefined> {
  const items = dialectPickItems(choices);
  const picked = await new Promise<DialectPickItem | undefined>((resolve) => {
    const quickPick = window.createQuickPick<DialectPickItem>();
    quickPick.title = "Select Tcl Dialect";
    quickPick.placeholder = `Current dialect: ${describeDialect(current, choices).label}`;
    quickPick.items = items;
    const currentItem = items.find((item) => item.value === current);
    if (currentItem) {
      quickPick.activeItems = [currentItem];
    }

    let done = false;
    const finish = (value: DialectPickItem | undefined) => {
      if (done) {
        return;
      }
      done = true;
      disposeAccept.dispose();
      disposeHide.dispose();
      quickPick.dispose();
      resolve(value);
    };

    const disposeAccept = quickPick.onDidAccept(() => {
      const [selected] =
        quickPick.selectedItems.length > 0 ? quickPick.selectedItems : quickPick.activeItems;
      finish(selected);
    });
    const disposeHide = quickPick.onDidHide(() => finish(undefined));
    quickPick.show();
  });
  return picked?.value;
}
