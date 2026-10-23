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

/** Exact original Unicode scalar boundaries under the browser's UTF-8 codec.
 * This supplies presentation geometry, never source identity or edit authority. */
export function sourceByteBoundaries(source: string): ReadonlyMap<number, number> | null {
  const encoder = new TextEncoder();
  const boundaries = new Map<number, number>([[0, 0]]);
  let bytes = 0;
  let units = 0;
  for (const scalar of source) {
    const point = scalar.codePointAt(0)!;
    // TextEncoder replaces lone surrogates; that would change the source image.
    if (point >= 0xd800 && point <= 0xdfff) return null;
    bytes += encoder.encode(scalar).length;
    units += scalar.length;
    boundaries.set(bytes, units);
  }
  return boundaries;
}
