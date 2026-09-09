#!/usr/bin/env node
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
// SPDX-License-Identifier: AGPL-3.0-or-later

// JSON.parse accepts duplicate object keys and silently keeps the last value.
// Reject them before parsing so checked-in manifests cannot hide an operative
// value behind a duplicate key.
export function parseJsonNoDuplicateKeys(text, label) {
  const stack = [];
  let index = 0;

  while (index < text.length) {
    const character = text[index];
    if (/\s/.test(character)) {
      index++;
      continue;
    }
    if (character === '"') {
      const start = index++;
      while (index < text.length) {
        if (text[index] === "\\") {
          index += 2;
        } else if (text[index++] === '"') {
          break;
        }
      }
      const object = stack.at(-1);
      if (object?.kind === "object" && object.expectKey) {
        const key = JSON.parse(text.slice(start, index));
        if (object.keys.has(key)) {
          throw new Error(`${label} contains duplicate object key: ${key}`);
        }
        object.keys.add(key);
        object.expectKey = false;
      }
      continue;
    }
    if (character === "{") {
      stack.push({ kind: "object", keys: new Set(), expectKey: true });
    } else if (character === "[") {
      stack.push({ kind: "array" });
    } else if (character === "}" || character === "]") {
      stack.pop();
    } else if (character === ",") {
      const object = stack.at(-1);
      if (object?.kind === "object") object.expectKey = true;
    }
    index++;
  }

  return JSON.parse(text);
}
