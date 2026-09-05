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

// The open-command tabs: which of the pack's commands are open at once, and
// which one the form is currently a projection of.
//
// `docs/design/spec-packs.md` asks for many commands to one deliverable, and
// that is how a library is actually written — comparing two specs, copying an
// option table from one command to another, checking a subcommand against its
// sibling. A studio with one editing slot makes every one of those a round
// trip through the browser list.
//
// Tabs are *views*, not stores. Nothing in this file holds a draft: the
// `.tclspec` document remains the one model, and a tab is a command's name
// plus enough place-keeping to bring its form back the way it was left. So
// everything here is a pure function of a tab list, and the DOM half in
// `studio.ts` is a rendering of a decision made here.

/** Where a command was opened from — the same distinction the visit stack makes. */
export type TabSource = "pack" | "registry";

/**
 * How many commands stay open at once.
 *
 * Twelve is two working sets: the cluster an author moves between — a command
 * and its subcommands, or a handful of siblings being compared — plus the few
 * shipped commands opened alongside to copy an option table out of. Past that
 * the strip stops being something a person can read at a glance, and a tab you
 * cannot see is only a leak. Closing one costs nothing but a click to reopen:
 * every edit already lives in the document, never in the tab.
 */
export const MAX_OPEN_TABS = 12;

/** One open command. */
export interface OpenTab {
  name: string;
  where: TabSource;
  /** The form groups the author had open, so the view returns as it was left. */
  groups: string[];
  /** How far the page was scrolled when the tab last lost focus, in pixels. */
  scroll: number;
  /**
   * Whether this command has been edited while open.
   *
   * Not "unsaved" — there is no such state, every keystroke is already in the
   * document. It marks the tab as somewhere the author is working, which is
   * what makes it a worse thing to evict than one merely read.
   */
  edited: boolean;
  /** When the tab last had focus, on [`TabState.clock`]. */
  used: number;
}

/** The open tabs, and which of them the form is showing. */
export interface TabState {
  tabs: OpenTab[];
  /** The focused tab, or -1 when the form holds a draft no tab names. */
  active: number;
  /** Hands out `used` stamps, so "least recently used" is a total order. */
  clock: number;
}

/** What a tab is worth persisting: its command, and how to rebuild its view. */
export interface StoredTab {
  name: string;
  where: TabSource;
  groups: string[];
  scroll: number;
}

/** How a tab's view was left — everything but the command it is a view of. */
export interface TabView {
  groups: string[];
  scroll: number;
}

export function emptyTabs(): TabState {
  return { tabs: [], active: -1, clock: 0 };
}

/** Where `name` sits among the open tabs, or -1 when it is not open. */
export function tabIndex(state: TabState, name: string): number {
  return state.tabs.findIndex((tab) => tab.name === name);
}

/** The focused tab, or `null` when the form holds a draft no tab names. */
export function activeTab(state: TabState): OpenTab | null {
  return state.tabs[state.active] ?? null;
}

/** Give the tab at `index` focus, stamping it as the most recently used. */
export function focusTab(state: TabState, index: number): TabState {
  if (!state.tabs[index]) return state;
  const clock = state.clock + 1;
  return {
    tabs: state.tabs.map((tab, at) => (at === index ? { ...tab, used: clock } : tab)),
    active: index,
    clock,
  };
}

/**
 * Open `name`, or focus it where it is already open.
 *
 * Re-opening never disturbs the tab's remembered view — the whole reason to
 * come back to a tab is to find it as you left it.
 */
export function openTab(
  state: TabState,
  name: string,
  where: TabSource,
  cap: number = MAX_OPEN_TABS,
): { state: TabState; evicted: OpenTab | null } {
  const existing = tabIndex(state, name);
  if (existing >= 0) return { state: focusTab(state, existing), evicted: null };

  const clock = state.clock + 1;
  const opened: TabState = {
    tabs: [...state.tabs, { name, where, groups: [], scroll: 0, edited: false, used: clock }],
    active: state.tabs.length,
    clock,
  };
  const victim = evictionTarget(opened, cap);
  if (victim < 0) return { state: opened, evicted: null };
  const evicted = opened.tabs[victim] ?? null;
  return { state: closeTabAt(opened, victim).state, evicted };
}

/**
 * Which tab an over-cap list should give up, or -1 when it is within the cap.
 *
 * The least recently used *clean* tab: one the author has only read is the
 * cheapest thing in the list to lose. When every other tab has been edited the
 * least recently used of those goes instead — the cap is the point, and an
 * evicted tab costs a click, not an edit, because the document holds the work.
 * The focused tab is never a candidate: closing what you are looking at is not
 * housekeeping.
 */
export function evictionTarget(state: TabState, cap: number = MAX_OPEN_TABS): number {
  if (state.tabs.length <= cap) return -1;
  const oldest = (candidates: number[]): number =>
    candidates.reduce(
      (best, at) =>
        best < 0 || (state.tabs[at]?.used ?? 0) < (state.tabs[best]?.used ?? 0) ? at : best,
      -1,
    );
  const open = state.tabs.map((_, at) => at).filter((at) => at !== state.active);
  const clean = open.filter((at) => !state.tabs[at]?.edited);
  return clean.length ? oldest(clean) : oldest(open);
}

/**
 * Close the tab at `index`.
 *
 * `focus` is the tab that should be shown now, which is `null` when the list
 * is empty or when the closed tab was not the one being shown. Closing is not
 * a navigation — the caller opens `focus` without recording a visit — so the
 * neighbour is chosen spatially, the way a browser does it: the tab to the
 * right, or the one to the left when there is nothing to the right.
 */
export function closeTabAt(
  state: TabState,
  index: number,
): { state: TabState; focus: OpenTab | null } {
  if (!state.tabs[index]) return { state, focus: null };
  const tabs = state.tabs.filter((_, at) => at !== index);
  if (index !== state.active) {
    return {
      state: { ...state, tabs, active: state.active > index ? state.active - 1 : state.active },
      focus: null,
    };
  }
  const at = Math.min(index, tabs.length - 1);
  const focus = tabs[at] ?? null;
  if (!focus) return { state: { ...state, tabs, active: -1 }, focus: null };
  const clock = state.clock + 1;
  return {
    state: {
      tabs: tabs.map((tab, i) => (i === at ? { ...tab, used: clock } : tab)),
      active: at,
      clock,
    },
    focus,
  };
}

/** The same, by command name. */
export function closeTab(
  state: TabState,
  name: string,
): { state: TabState; focus: OpenTab | null } {
  return closeTabAt(state, tabIndex(state, name));
}

/** The tab Ctrl+Tab (`delta` 1) and Ctrl+Shift+Tab (-1) move to, or -1. */
export function cycleIndex(state: TabState, delta: number): number {
  const count = state.tabs.length;
  if (count === 0) return -1;
  const from = state.active < 0 ? (delta > 0 ? -1 : 0) : state.active;
  return (((from + delta) % count) + count) % count;
}

/**
 * Follow a rename.
 *
 * Renaming a command is an ordinary edit — the declaration keeps its place in
 * the document — so the tab keeps its place in the strip and its view with it.
 */
export function renameTab(state: TabState, from: string, to: string): TabState {
  const at = tabIndex(state, from);
  if (at < 0 || from === to) return state;
  // A rename onto a name that is already open would leave two tabs over one
  // declaration; the older one is what the document no longer has.
  const collision = tabIndex(state, to);
  const renamed = state.tabs.map((tab, i) => (i === at ? { ...tab, name: to } : tab));
  if (collision < 0) return { ...state, tabs: renamed };
  return closeTabAt({ ...state, tabs: renamed }, collision).state;
}

/** Mark `name` as somewhere the author is working, so eviction passes it over. */
export function markEdited(state: TabState, name: string): TabState {
  const at = tabIndex(state, name);
  if (at < 0 || state.tabs[at]?.edited) return state;
  return {
    ...state,
    tabs: state.tabs.map((tab, i) => (i === at ? { ...tab, edited: true } : tab)),
  };
}

/** Record how a tab's form was arranged, so focusing it again restores that. */
export function rememberView(state: TabState, index: number, view: TabView): TabState {
  if (!state.tabs[index]) return state;
  return {
    ...state,
    tabs: state.tabs.map((tab, at) =>
      at === index ? { ...tab, groups: [...view.groups], scroll: view.scroll } : tab,
    ),
  };
}

/**
 * Drop every tab the document no longer declares.
 *
 * A tab is a view of a declaration; deleting the declaration in the DSL pane
 * has to take the view with it. `focus` is the tab to show when the one that
 * was showing has gone.
 */
export function retainTabs(
  state: TabState,
  names: ReadonlySet<string>,
): { state: TabState; focus: OpenTab | null } {
  let next = state;
  let focus: OpenTab | null = null;
  for (let at = next.tabs.length - 1; at >= 0; at -= 1) {
    const tab = next.tabs[at];
    if (!tab || names.has(tab.name)) continue;
    const closed = closeTabAt(next, at);
    next = closed.state;
    focus = closed.focus ?? focus;
  }
  return { state: next, focus };
}

/* Persistence ------------------------------------------------------------ */

/** What goes into the session record: the commands, in the strip's order. */
export function storedTabs(state: TabState): StoredTab[] {
  return state.tabs.map((tab) => ({
    name: tab.name,
    where: tab.where,
    groups: [...tab.groups],
    scroll: tab.scroll,
  }));
}

/**
 * Read the tabs out of a stored session, keeping only well-formed rows.
 *
 * A record written before tabs existed carries none, which restores as "no
 * tabs" rather than as a failure to restore — the same defensive reading
 * `idb.ts` already gives `expanded` and `dockOpen`.
 */
export function readStoredTabs(value: unknown): StoredTab[] {
  if (!Array.isArray(value)) return [];
  const rows: StoredTab[] = [];
  for (const raw of value) {
    if (!raw || typeof raw !== "object") continue;
    const row = raw as Partial<StoredTab>;
    if (typeof row.name !== "string" || !row.name) continue;
    rows.push({
      name: row.name,
      where: row.where === "registry" ? "registry" : "pack",
      groups: Array.isArray(row.groups)
        ? row.groups.filter((group): group is string => typeof group === "string")
        : [],
      scroll: typeof row.scroll === "number" && Number.isFinite(row.scroll) ? row.scroll : 0,
    });
  }
  return rows;
}

/**
 * Rebuild a tab list from a stored session.
 *
 * Duplicates and anything over the cap are dropped here rather than at the
 * first eviction, so a restored session is a list the studio could have
 * produced itself. `active` names the tab that was showing; a record that
 * names none, or names one the list no longer has, restores as the first tab.
 */
export function restoreTabs(
  rows: StoredTab[],
  active: string | null,
  cap: number = MAX_OPEN_TABS,
): TabState {
  const seen = new Set<string>();
  const kept: StoredTab[] = [];
  for (const row of rows) {
    if (seen.has(row.name) || kept.length >= cap) continue;
    seen.add(row.name);
    kept.push(row);
  }
  if (!kept.length) return emptyTabs();
  const at = active ? kept.findIndex((row) => row.name === active) : -1;
  // Restored tabs are stamped oldest-first in strip order, so the first
  // eviction after a reload falls on the left of the strip rather than on
  // whichever row happened to be read back first; focusing the restored tab
  // then lifts it clear of that, as opening it in the first place would have.
  return focusTab(
    {
      tabs: kept.map((row, i) => ({ ...row, groups: [...row.groups], edited: false, used: i })),
      active: -1,
      clock: kept.length,
    },
    at < 0 ? 0 : at,
  );
}
