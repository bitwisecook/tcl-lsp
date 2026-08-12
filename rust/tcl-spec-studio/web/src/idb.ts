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

// Live save, in the browser and nowhere else.
//
// `docs/design/spec-packs.md` asks for "every keystroke persists to browser
// storage (IndexedDB) … reload, crash, or restart resumes exactly". What is
// persisted is the same thing the studio treats as its model: the pack's
// `.tclspec` document, plus the little bit of place-keeping that makes a reload
// resume rather than restart (which command was open, which dialect).
//
// Storage is best effort by design. A page opened from `file://` has an opaque
// origin in some browsers and IndexedDB is simply unavailable there; a private
// window may refuse to persist. None of that is an error worth interrupting an
// author over — the studio keeps working, it just will not remember. So every
// call here resolves rather than rejects, and the caller is told whether
// persistence is live so it can say so in the UI.

/** What a reload restores. */
export interface Session {
  /** The pack's `.tclspec` document — the studio's one authoritative model. */
  source: string;
  /** The pack command the editor had open, if any. */
  open: string | null;
  /** The registry dialect the browser was pointed at. */
  dialect: string;
  /** The Test tab's sample code — as much a part of the work as the pack. */
  sample?: string;
}

const DB_NAME = "tcl-spec-studio";
const DB_VERSION = 1;
const STORE = "session";
/** One row: the studio edits one pack at a time in this slice. */
const KEY = "current";

let handle: Promise<IDBDatabase | null> | null = null;

/**
 * Open (and if needed create) the database, once.
 *
 * Resolves to `null` when this browser or origin has no usable IndexedDB,
 * which is a supported state rather than a failure.
 */
function db(): Promise<IDBDatabase | null> {
  handle ??= new Promise<IDBDatabase | null>((resolve) => {
    let request: IDBOpenDBRequest;
    try {
      request = indexedDB.open(DB_NAME, DB_VERSION);
    } catch {
      resolve(null);
      return;
    }
    request.onupgradeneeded = () => {
      const open = request.result;
      if (!open.objectStoreNames.contains(STORE)) open.createObjectStore(STORE);
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => resolve(null);
    request.onblocked = () => resolve(null);
  });
  return handle;
}

/** Whether persistence is available at all. */
export async function available(): Promise<boolean> {
  return (await db()) !== null;
}

/** Persist `session`, replacing whatever was there. Never throws. */
export async function save(session: Session): Promise<boolean> {
  const open = await db();
  if (!open) return false;
  return new Promise<boolean>((resolve) => {
    let tx: IDBTransaction;
    try {
      tx = open.transaction(STORE, "readwrite");
    } catch {
      resolve(false);
      return;
    }
    tx.objectStore(STORE).put(session, KEY);
    tx.oncomplete = () => resolve(true);
    tx.onerror = () => resolve(false);
    tx.onabort = () => resolve(false);
  });
}

/** The persisted session, or `null` when there is none to resume. */
export async function load(): Promise<Session | null> {
  const open = await db();
  if (!open) return null;
  return new Promise<Session | null>((resolve) => {
    let request: IDBRequest<unknown>;
    try {
      request = open.transaction(STORE, "readonly").objectStore(STORE).get(KEY);
    } catch {
      resolve(null);
      return;
    }
    request.onsuccess = () => {
      const value = request.result;
      // Anything that is not the shape we wrote is treated as absent: a
      // half-written or version-skewed row must never be able to break a boot.
      if (!value || typeof value !== "object") {
        resolve(null);
        return;
      }
      const row = value as Partial<Session>;
      if (typeof row.source !== "string") {
        resolve(null);
        return;
      }
      resolve({
        source: row.source,
        open: typeof row.open === "string" ? row.open : null,
        dialect: typeof row.dialect === "string" ? row.dialect : "tcl9.0",
        sample: typeof row.sample === "string" ? row.sample : undefined,
      });
    };
    request.onerror = () => resolve(null);
  });
}

/** Forget the persisted session. */
export async function clear(): Promise<void> {
  const open = await db();
  if (!open) return;
  await new Promise<void>((resolve) => {
    let tx: IDBTransaction;
    try {
      tx = open.transaction(STORE, "readwrite");
    } catch {
      resolve();
      return;
    }
    tx.objectStore(STORE).delete(KEY);
    tx.oncomplete = () => resolve();
    tx.onerror = () => resolve();
    tx.onabort = () => resolve();
  });
}
