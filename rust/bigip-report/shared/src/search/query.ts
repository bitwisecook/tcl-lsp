// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Parse the global-search box into an optional scope prefix + free-text terms.
// Supported scopes:
//   t<n>:<terms>          — restrict to devices in tier <n>   (e.g. `t0:vs_web`)
//   <partial_devname>:<terms> — restrict to devices whose name partially
//                           matches <partial_devname>          (e.g. `edge:pool`)
// Anything else is treated as free text (the colon is literal), and IP/CIDR
// queries are handled by the caller *before* this runs, so `10.0.0.1:8443`
// never reaches the scope parser.

import { subsequenceMatch } from "./matcher";

export interface Scope {
  tier: number | null;
  dev: string | null;
}

export interface ParsedQuery {
  scope: Scope;
  terms: string;
  /** True when a `t<n>:` or `<devname>:` prefix was recognised. */
  hadScope: boolean;
}

const EMPTY_SCOPE: Scope = { tier: null, dev: null };

/** Does `prefix` plausibly name one of `deviceNames` (substring or in-order
 *  subsequence, case-insensitive)? Used to tell a `devname:` scope from a
 *  literal colon. */
export function matchesDeviceName(prefix: string, deviceNames: string[]): boolean {
  const p = prefix.toLowerCase();
  if (!p) return false;
  return deviceNames.some((n) => {
    const nl = n.toLowerCase();
    return nl.includes(p) || subsequenceMatch(nl, p);
  });
}

export function parseQuery(raw: string, deviceNames: string[]): ParsedQuery {
  const s = raw.trim();
  if (!s) return { scope: { ...EMPTY_SCOPE }, terms: "", hadScope: false };

  // t<n>: — tier scope wins over a same-shaped device name.
  const tierM = /^t(\d+):(.*)$/i.exec(s);
  if (tierM) {
    return {
      scope: { tier: parseInt(tierM[1], 10), dev: null },
      terms: tierM[2].trim(),
      hadScope: true,
    };
  }

  // <partial_devname>: — only when the prefix actually matches a device.
  const devM = /^([^:\s]+):(.*)$/.exec(s);
  if (devM && matchesDeviceName(devM[1], deviceNames)) {
    return {
      scope: { tier: null, dev: devM[1] },
      terms: devM[2].trim(),
      hadScope: true,
    };
  }

  return { scope: { ...EMPTY_SCOPE }, terms: s, hadScope: false };
}

/** Does a device (by name + tier) satisfy the parsed scope? */
export function deviceInScope(
  scope: Scope,
  deviceName: string,
  deviceTier: number | null,
): boolean {
  if (scope.tier !== null && deviceTier !== scope.tier) return false;
  if (scope.dev !== null && !matchesDeviceName(scope.dev, [deviceName])) return false;
  return true;
}
