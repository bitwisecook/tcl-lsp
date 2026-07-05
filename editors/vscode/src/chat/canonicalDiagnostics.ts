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
 * Typed re-exports of canonical diagnostic data.
 *
 * Source of truth: ai/shared/diagnostics.json (copied to src/chat/canonical/
 * at build time by the Makefile `copy-canonical` target).
 */

import diagnostics from "./canonical/diagnostics.json";

/** Flat lookup: diagnostic code → category key. */
export const CODE_TO_CATEGORY: Record<string, string> = (() => {
  const map: Record<string, string> = {};
  for (const cat of diagnostics.categories) {
    for (const code of cat.codes) {
      map[code] = cat.key;
    }
  }
  return map;
})();

/** Prefix-based fallback rules applied when a code has no explicit entry. */
export const CATEGORY_PREFIX_RULES: ReadonlyArray<{ prefix: string; category: string }> =
  diagnostics.prefix_rules;

/** Default category when no explicit or prefix match is found. */
export const DEFAULT_CATEGORY: string = diagnostics.default_category;

/** Ordered list of categories for display. */
export const CATEGORY_ORDER: ReadonlyArray<{ key: string; label: string }> =
  diagnostics.categories.map((c) => ({ key: c.key, label: c.label }));

/** Diagnostic codes that have auto-fix conversions. */
export const CONVERTIBLE_CODES = new Set(diagnostics.convertible_codes);

/** Human-readable descriptions of each auto-fix conversion. */
export const CONVERSION_MAP: Record<string, string> = diagnostics.conversion_map;

/** Regex to extract iRules event names from source code. */
export const IRULES_EVENT_PATTERN = new RegExp(diagnostics.irules_event_pattern, "gm");
