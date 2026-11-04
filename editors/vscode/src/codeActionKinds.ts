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

const GENERATE_DOCSTRING_KIND = "source.generateDocstring";

interface TypedCodeAction {
  readonly title: string;
  readonly kind?: { readonly value: string };
  readonly disabled?: unknown;
}

/** Find an enabled docstring action by its protocol kind, independent of its display title. */
export function findGenerateDocstringAction<T extends TypedCodeAction>(
  actions: readonly T[] | undefined,
): T | undefined {
  return actions?.find(
    (action) => action.kind?.value === GENERATE_DOCSTRING_KIND && action.disabled === undefined,
  );
}
