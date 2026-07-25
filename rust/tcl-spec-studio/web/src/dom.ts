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

// Small DOM helpers shared by the studio's form and panes.

export type Child = Node | string | null | undefined;

export interface Attrs {
  class?: string;
  text?: string;
  [name: string]: string | number | boolean | EventListener | undefined;
}

/** Look up an element by id, failing loudly rather than returning null. */
export function byId<T extends HTMLElement = HTMLElement>(id: string): T {
  const node = document.getElementById(id);
  if (!node) throw new Error(`missing element #${id}`);
  return node as T;
}

/**
 * Build an element. `class` and `text` are handled specially; keys starting
 * `on` become listeners; `true` becomes a bare attribute and `false`/`undefined`
 * is skipped.
 */
export function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs?: Attrs,
  kids?: Child[],
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (attrs) {
    for (const [key, value] of Object.entries(attrs)) {
      if (value === undefined || value === false) continue;
      if (key === "class") node.className = String(value);
      else if (key === "text") node.textContent = String(value);
      else if (key.startsWith("on")) node.addEventListener(key.slice(2), value as EventListener);
      else if (value === true) node.setAttribute(key, "");
      else node.setAttribute(key, String(value));
    }
  }
  for (const kid of kids ?? []) {
    if (kid === null || kid === undefined) continue;
    node.appendChild(typeof kid === "string" ? document.createTextNode(kid) : kid);
  }
  return node;
}

/** Remove every child of `node`. */
export function clear(node: Node): void {
  while (node.firstChild) node.removeChild(node.firstChild);
}

/** Set a status line's text and severity. */
export function setStatus(id: string, message: string, kind?: "ok" | "err"): void {
  const node = byId(id);
  node.className = "status" + (kind ? ` ${kind}` : "");
  node.textContent = message;
}

/** Structural equality over JSON-shaped values. */
export function deepEqual(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** A structural copy of a JSON-shaped value. */
export function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

/** Copy `text` to the clipboard, reporting into the status line `statusId`. */
export function copyText(text: string, statusId: string): void {
  const done = (): void => {
    setStatus(statusId, "copied", "ok");
    window.setTimeout(() => setStatus(statusId, ""), 1600);
  };
  const fallback = (): void => {
    // `execCommand` is deprecated but is the only path that works from a
    // file:// page in some browsers — and this page is designed to be saved
    // and opened from disk.
    const area = document.createElement("textarea");
    area.value = text;
    area.style.position = "fixed";
    area.style.opacity = "0";
    document.body.appendChild(area);
    area.select();
    try {
      document.execCommand("copy");
      done();
    } catch {
      setStatus(statusId, "could not copy — select the text and copy manually", "err");
    }
    document.body.removeChild(area);
  };
  if (navigator.clipboard?.writeText) {
    navigator.clipboard.writeText(text).then(done, fallback);
  } else {
    fallback();
  }
}

/** Save `text` to the user's machine as `name`'s basename. */
export function download(name: string, text: string): void {
  const blob = new Blob([text], { type: "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = el("a", { href: url, download: name.split("/").pop() ?? name });
  document.body.appendChild(anchor);
  anchor.click();
  document.body.removeChild(anchor);
  window.setTimeout(() => URL.revokeObjectURL(url), 1000);
}
