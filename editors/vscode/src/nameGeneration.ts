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

import namespaceCatalog from "./generated/scaffoldNamespacePolicy.json";

export interface GeneratedScaffoldNamespace {
  spelling: string;
  lossy: boolean;
}

/** Construct new scaffold names; the shared policy permits sanitisation collisions. */
export function packageScaffoldNamespace(packageName: string): GeneratedScaffoldNamespace {
  const policy = namespaceCatalog.policy;
  const accepts = (ranges: number[][], unit: number) =>
    ranges.some(([first, last]) => first <= unit && unit <= last);
  const parts: string[] = [];
  let lossy = false;
  for (const part of packageName.split(policy.separator)) {
    if (!part) {
      lossy = true;
      continue;
    }
    let cleaned = "";
    for (let index = 0; index < part.length; index++) {
      const unit = part.charCodeAt(index);
      cleaned += accepts(policy.followingUnitRanges, unit) ? part[index] : policy.replacement;
    }
    if (!accepts(policy.firstUnitRanges, cleaned.charCodeAt(0))) {
      cleaned = policy.identifierPrefix + cleaned;
    }
    lossy ||= cleaned !== part;
    parts.push(cleaned);
  }
  if (parts.length === 0) {
    parts.push(policy.fallback);
    lossy = true;
  }
  return { spelling: policy.root + parts.join(policy.separator), lossy };
}
