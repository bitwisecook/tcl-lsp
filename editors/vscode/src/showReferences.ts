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

import * as vscode from "vscode";

export interface JsonPosition {
  line: number;
  character: number;
}

export interface JsonLocation {
  uri: string;
  range: {
    start: JsonPosition;
    end: JsonPosition;
  };
}

// Convert the JSON-RPC shapes that the LSP server serialises into the
// ``vscode.Uri`` / ``vscode.Position`` / ``vscode.Location`` instances that
// the built-in ``editor.action.showReferences`` command validates against.
export function convertShowReferencesArgs(
  uriString: string,
  position: JsonPosition,
  locations: ReadonlyArray<JsonLocation>,
): {
  uri: vscode.Uri;
  position: vscode.Position;
  locations: vscode.Location[];
} {
  return {
    uri: vscode.Uri.parse(uriString),
    position: new vscode.Position(position.line, position.character),
    locations: locations.map(
      (loc) =>
        new vscode.Location(
          vscode.Uri.parse(loc.uri),
          new vscode.Range(
            loc.range.start.line,
            loc.range.start.character,
            loc.range.end.line,
            loc.range.end.character,
          ),
        ),
    ),
  };
}
