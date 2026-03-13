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
