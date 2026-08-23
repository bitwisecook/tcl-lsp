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

// One editor per `schema::FieldKind`.
//
// Each editor takes the field's current JSON value and a `set` callback, and
// returns the DOM for it. Nothing here knows any *field* — only kinds — which
// is what lets a new `CommandSpec` field appear in the form with no change to
// this file.

import { clone, el, type Child } from "./dom.js";
import type { Catalogues, FieldKind, Json, Variant } from "./types.js";

/**
 * Callback an editor uses to replace the field's whole value.
 *
 * `structural` says whether the change alters the editor's *shape* — a row
 * added or removed, a discriminant switched, a toggle revealing further
 * controls. It defaults to `true`, which is the safe answer: the field is
 * rebuilt. A composite editor passes `false` for a plain text or number edit
 * inside a row, so the input the user is typing into survives the keystroke.
 */
export type Setter = (next: Json, structural?: boolean) => void;

/** Builds the editor DOM for one field kind. */
export type Editor = (kind: FieldKind, value: Json, set: Setter) => HTMLElement;

/**
 * Kinds whose editor changes *shape* when its value changes — a row added or
 * removed, a toggle revealing further controls. Those must be rebuilt after a
 * change; a plain text or number input must not be, or the caret jumps to the
 * end mid-word.
 */
export const STRUCTURAL_KINDS = new Set([
  "textList",
  "indexList",
  "optIndexList",
  "flagSet",
  "roleMap",
  "presentationMap",
  "prefixMap",
  "argTypeMap",
  "argValueMap",
  "options",
  "forms",
  "sideEffects",
  "setterConstraints",
  "manufacturerMethods",
  "subSubCommands",
  "subCommands",
  "hover",
  "objectClass",
]);

/** Everything an editor needs from the surrounding app. */
export interface EditorContext {
  catalogues: Catalogues;
  /** Builds the inline documentation control for one catalogue item. */
  variantHelp: (variant: Variant) => { button: HTMLButtonElement; panel: HTMLElement };
  /** A fresh subcommand draft, for the subcommand list's add button. */
  newSubcommand: () => Record<string, Json>;
  /** Renders a nested subcommand form into `container`. */
  buildSubcommandForm: (
    container: HTMLElement,
    draft: Record<string, Json>,
    onChange: () => void,
  ) => void;
}

/* Value coercions. A draft arrives from `JSON.parse`, so every read has to
   narrow before use rather than trusting the declared shape. */

export function asRecord(value: Json): Record<string, Json> {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, Json>)
    : {};
}

function asArray(value: Json): Json[] {
  return Array.isArray(value) ? value : [];
}

export function asString(value: Json | undefined): string {
  return typeof value === "string" ? value : "";
}

function asNumber(value: Json | undefined): number | null {
  return typeof value === "number" ? value : null;
}

function asBool(value: Json | undefined): boolean {
  return value === true;
}

function asStringList(value: Json): string[] {
  return asArray(value).filter((item): item is string => typeof item === "string");
}

/* Primitive controls. */

function textInput(
  value: string,
  onChange: (text: string) => void,
  opts?: { placeholder?: string; size?: number },
): HTMLInputElement {
  const input = el("input", { type: "text", placeholder: opts?.placeholder ?? "" });
  if (opts?.size) input.size = opts.size;
  input.value = value;
  input.addEventListener("input", () => onChange(input.value));
  return input;
}

function numberInput(
  value: number | null,
  onChange: (n: number | null) => void,
  opts?: { min?: number; placeholder?: string },
): HTMLInputElement {
  const input = el("input", {
    type: "number",
    min: opts?.min ?? 0,
    placeholder: opts?.placeholder ?? "",
  });
  input.style.width = "6rem";
  input.value = value === null ? "" : String(value);
  input.addEventListener("input", () => onChange(input.value === "" ? null : Number(input.value)));
  return input;
}

function checkbox(
  value: boolean,
  onChange: (on: boolean) => void,
  label: string,
): HTMLLabelElement {
  const box = el("input", { type: "checkbox" });
  box.checked = value;
  box.addEventListener("change", () => onChange(box.checked));
  return el("label", { class: "checkrow" }, [box, label]);
}

function labelled(text: string, control: Child): HTMLLabelElement {
  return el("label", {}, [text, control]);
}

function removeButton(onRemove: () => void): HTMLButtonElement {
  return el("button", {
    type: "button",
    class: "rm",
    title: "remove",
    "aria-label": "remove",
    text: "✕",
    onclick: onRemove,
  });
}

/**
 * A repeating list of rows. `make` builds one row given the item, an `update`
 * that replaces it and a `remove` that drops it; `blank` supplies a fresh item
 * for the add button.
 */
function rowList<T extends Json>(
  items: T[],
  blank: () => T,
  make: (
    item: T,
    index: number,
    update: (next: T, structural?: boolean) => void,
    remove: () => void,
  ) => HTMLElement,
  onChange: (next: T[], structural?: boolean) => void,
  addLabel: string,
): HTMLElement {
  const list = items.slice();
  const wrap = el("div", { class: "rows" });
  const commit = (structural = true): void => onChange(list.slice(), structural);

  list.forEach((item, i) => {
    wrap.appendChild(
      make(
        item,
        i,
        (next, structural) => {
          list[i] = next;
          commit(structural);
        },
        () => {
          list.splice(i, 1);
          commit();
        },
      ),
    );
  });

  wrap.appendChild(
    el("div", {}, [
      el("button", {
        type: "button",
        class: "ghost",
        text: `+ ${addLabel}`,
        onclick: () => {
          list.push(blank());
          commit();
        },
      }),
    ]),
  );
  return wrap;
}

/** Build the set of editors bound to `ctx`. */
export function makeEditors(ctx: EditorContext): Record<string, Editor> {
  const catalogueFor = (kind: FieldKind): Variant[] =>
    (kind.catalogue ? ctx.catalogues[kind.catalogue] : undefined) ?? [];

  /**
   * A `<select>` over a catalogue. `optional` adds an "unset" entry; a current
   * value that is not in the catalogue (a payload-carrying variant such as
   * `Rebinarifies { value_arg: 1 }`) is added so nothing is lost.
   */
  function catalogueSelect(
    kind: FieldKind,
    value: string | null,
    onChange: (next: string | null) => void,
  ): HTMLSelectElement {
    const options: HTMLOptionElement[] = [];
    if (kind.optional) options.push(el("option", { value: "", text: "— unset —" }));
    let seen = false;
    for (const entry of catalogueFor(kind)) {
      if (entry.key === value) seen = true;
      options.push(el("option", { value: entry.key, title: entry.doc, text: entry.key }));
    }
    if (value !== null && value !== "" && !seen) {
      options.push(el("option", { value, text: `${value} (from the spec)` }));
    }
    const select = el("select", {}, options);
    select.value = value ?? "";
    select.addEventListener("change", () => onChange(select.value === "" ? null : select.value));
    return select;
  }

  /** Searchable, documented groups over a bitflag catalogue. */
  function flagChips(kind: FieldKind, value: Json, onChange: Setter): HTMLElement {
    const declared = value !== null;
    const bits = declared ? asStringList(value) : [];
    const wrap = el("div", {});

    if (kind.optional) {
      wrap.appendChild(checkbox(declared, (on) => onChange(on ? bits : null), "declared"));
    }

    const entries = catalogueFor(kind);
    const order = entries.map((entry) => entry.key);
    const groups = new Map<string, Variant[]>();
    for (const entry of entries) {
      const group = entry.group ?? "Other";
      groups.set(group, [...(groups.get(group) ?? []), entry]);
    }

    const list = el("div", { class: "toggle-groups" });
    const search = el("input", {
      type: "search",
      class: "toggle-search",
      placeholder: `Filter ${entries.length} traits…`,
      "aria-label": "Filter traits",
    });
    search.disabled = Boolean(kind.optional) && !declared;

    const rows: Array<{ node: HTMLElement; group: HTMLElement; hay: string }> = [];
    let groupIndex = 0;
    for (const [groupName, groupEntries] of groups) {
      const count = el("span", { class: "n" });
      const body = el("div", { class: "toggle-list" });
      const details = el("details", { class: "toggle-group" }, [
        el("summary", {}, [document.createTextNode(groupName), count]),
        body,
      ]);
      if (groupIndex++ === 0 || groupEntries.some((entry) => bits.includes(entry.key))) {
        details.open = true;
      }

      const updateCount = (): void => {
        const selected = groupEntries.filter((entry) => bits.includes(entry.key)).length;
        count.textContent = selected
          ? `${selected} of ${groupEntries.length} selected`
          : `${groupEntries.length}`;
      };
      for (const entry of groupEntries) {
        const box = el("input", { type: "checkbox" });
        box.checked = bits.includes(entry.key);
        box.disabled = Boolean(kind.optional) && !declared;
        const help = ctx.variantHelp(entry);
        const choice = el("label", { class: "trait-choice" }, [
          box,
          el("code", { text: entry.key }),
        ]);
        const copy = el("div", { class: "trait-copy" }, [
          el("div", { class: "trait-title" }, [choice, help.button]),
          el("span", { class: "doc", text: entry.doc }),
        ]);
        const row = el("div", { class: "trait-toggle" }, [copy, help.panel]);
        box.addEventListener("change", () => {
          const at = bits.indexOf(entry.key);
          if (box.checked && at < 0) bits.push(entry.key);
          else if (!box.checked && at >= 0) bits.splice(at, 1);
          bits.sort((a, b) => order.indexOf(a) - order.indexOf(b));
          updateCount();
          onChange(bits.slice(), false);
        });
        body.appendChild(row);
        rows.push({
          node: row,
          group: details,
          hay: `${groupName} ${entry.key} ${entry.doc}`.toLowerCase(),
        });
      }
      updateCount();
      list.appendChild(details);
    }
    search.addEventListener("input", () => {
      const tokens = search.value.toLowerCase().trim().split(/\s+/).filter(Boolean);
      for (const { node, hay } of rows) node.hidden = !tokens.every((token) => hay.includes(token));
      for (const details of list.querySelectorAll<HTMLElement>(".toggle-group")) {
        details.hidden = !rows.some(({ node, group }) => group === details && !node.hidden);
        if (tokens.length && !details.hidden) (details as HTMLDetailsElement).open = true;
      }
    });
    wrap.append(search, list);
    return wrap;
  }

  /** The `ArgValue` list shared by `arg_values` and an option's value set. */
  function argValueList(values: Json[], onChange: (next: Json[]) => void): HTMLElement {
    return rowList<Json>(
      values,
      () => ({ value: "", detail: "", min_tcl: null, code: null }),
      (item, _i, update, remove) => {
        const row = asRecord(item);
        const patch = (next: Record<string, Json>): void => {
          update({ ...clone(row), ...next });
        };
        const minTcl = el("select", {}, [
          el("option", { value: "", text: "— any —" }),
          el("option", { value: "V8_4", text: "8.4" }),
          el("option", { value: "V8_5", text: "8.5" }),
          el("option", { value: "V8_6", text: "8.6" }),
          el("option", { value: "V9_0", text: "9.0" }),
          el("option", { value: "V9_1", text: "9.1" }),
        ]);
        minTcl.value = asString(row.min_tcl);
        minTcl.addEventListener("change", () =>
          patch({ min_tcl: minTcl.value === "" ? null : minTcl.value }),
        );
        return el("div", { class: "row" }, [
          labelled(
            "value",
            textInput(asString(row.value), (t) => patch({ value: t }), { size: 12 }),
          ),
          labelled(
            "detail",
            textInput(asString(row.detail), (t) => patch({ detail: t })),
          ),
          labelled("min Tcl", minTcl),
          labelled(
            "code",
            numberInput(asNumber(row.code), (n) => patch({ code: n }), {
              min: -2147483648,
              placeholder: "none",
            }),
          ),
          removeButton(remove),
        ]);
      },
      onChange,
      "value",
    );
  }

  /** One `OptionSpec` row — flag or value-taking, with its value set. */
  function optionRow(
    item: Json,
    _index: number,
    update: (next: Json, structural?: boolean) => void,
    remove: () => void,
  ): HTMLElement {
    const opt = asRecord(item);
    // `structural: false` for a plain text edit — the row keeps its DOM, so
    // the input being typed into is not torn down mid-word. Anything that
    // shows or hides a control (toggling "takes a value", switching the arity
    // kind) leaves it defaulted so the row rebuilds.
    const patch = (next: Record<string, Json>, structural = true): void => {
      update({ ...clone(opt), ...next }, structural);
    };
    const takesValue = opt.value !== null && opt.value !== undefined;

    const rows: Child[] = [
      el("div", { class: "ctl" }, [
        labelled(
          "flag",
          textInput(asString(opt.name), (t) => patch({ name: t }, false), { size: 14 }),
        ),
        checkbox(
          takesValue,
          (on) =>
            patch({
              value: on
                ? {
                    arity: { kind: "One" },
                    role: "Value",
                    also_role: null,
                    body_kind: "Plain",
                    values: [],
                    closed: false,
                    integer: null,
                    hint: "",
                    appended_arity: { kind: "Unknown" },
                  }
                : null,
            }),
          "takes a value",
        ),
        removeButton(remove),
      ]),
      textInput(asString(opt.detail), (t) => patch({ detail: t }, false), {
        placeholder: "what the option does",
      }),
      labelled(
        "deprecation fix hook",
        textInput(
          asString(opt.deprecation_fix),
          (text) => patch({ deprecation_fix: text === "" ? null : text }, false),
          {
            placeholder: "Some(DeprecationFixHook::…)",
            size: 44,
          },
        ),
      ),
    ];

    if (takesValue) {
      const value = asRecord(opt.value);
      const arity = asRecord(value.arity);
      const patchValue = (next: Record<string, Json>, structural = true): void => {
        patch({ value: { ...clone(value), ...next } }, structural);
      };
      const words = el("select", {}, [
        el("option", { value: "One", text: "one" }),
        el("option", { value: "Fixed", text: "fixed count" }),
        el("option", { value: "Hook", text: "computed (hook)" }),
      ]);
      const arityKind = asString(arity.kind);
      words.value = arityKind === "Fixed" || arityKind === "Hook" ? arityKind : "One";
      words.addEventListener("change", () => {
        if (words.value === "Fixed") {
          patchValue({ arity: { kind: "Fixed", n: asNumber(arity.n) ?? 2 } });
        } else if (words.value === "Hook") {
          // Preserve any expression already typed when toggling back and forth.
          patchValue({ arity: { kind: "Hook", hook: arity.hook ?? null } });
        } else {
          patchValue({ arity: { kind: "One" } });
        }
      });

      rows.push(
        el("div", { class: "ctl" }, [
          labelled(
            "role",
            catalogueSelect({ tag: "enum", catalogue: "argRole" }, asString(value.role), (r) =>
              patchValue({ role: r }),
            ),
          ),
          labelled("words", words),
          arityKind === "Fixed"
            ? labelled(
                "n",
                numberInput(asNumber(arity.n), (n) =>
                  patchValue({ arity: { kind: "Fixed", n: n ?? 1 } }),
                ),
              )
            : null,
          // A function pointer the studio cannot recover by seeding — the
          // author types the expression and it renders like any other arity.
          arityKind === "Hook"
            ? labelled(
                "hook fn",
                textInput(
                  asString(arity.hook),
                  (t) =>
                    patchValue(
                      { arity: { kind: "Hook", hook: t.trim() === "" ? null : t.trim() } },
                      false,
                    ),
                  { size: 16, placeholder: "errorstack_value" },
                ),
              )
            : null,
          labelled(
            "hint",
            textInput(asString(value.hint), (t) => patchValue({ hint: t }, false), { size: 10 }),
          ),
          checkbox(asBool(value.closed), (on) => patchValue({ closed: on }), "closed set"),
        ]),
      );
      rows.push(
        el("div", {}, [
          el("span", { class: "hint", text: "allowed values" }),
          argValueList(asArray(value.values), (values) => patchValue({ values })),
        ]),
      );
    }

    return el("div", { class: "row wide" }, rows);
  }

  const editors: Record<string, Editor> = {
    bool: (_kind, value, set) => checkbox(asBool(value), set, "enabled"),

    optBool: (_kind, value, set) => {
      const select = el("select", {}, [
        el("option", { value: "", text: "— unset —" }),
        el("option", { value: "true", text: "true" }),
        el("option", { value: "false", text: "false" }),
      ]);
      select.value = value === null ? "" : String(value);
      select.addEventListener("change", () =>
        set(select.value === "" ? null : select.value === "true"),
      );
      return select;
    },

    count: (_kind, value, set) => numberInput(asNumber(value) ?? 0, (n) => set(n ?? 0)),

    optIndex: (_kind, value, set) =>
      el("div", { class: "ctl" }, [
        numberInput(asNumber(value), set, { placeholder: "unset" }),
        el("span", {
          class: "hint",
          text: "0-based, after the command name — blank for unset",
        }),
      ]),

    text: (_kind, value, set) => textInput(asString(value), set),

    optText: (_kind, value, set) =>
      textInput(asString(value), (text) => set(text === "" ? null : text), {
        placeholder: "unset",
      }),

    prose: (_kind, value, set) => {
      const area = el("textarea", { rows: 4 });
      area.value = asString(value);
      area.addEventListener("input", () => set(area.value));
      return area;
    },

    textList: (_kind, value, set) =>
      rowList<Json>(
        asArray(value),
        () => "",
        (item, _i, update, remove) =>
          el("div", { class: "row" }, [
            textInput(asString(item), (t) => update(t)),
            removeButton(remove),
          ]),
        set,
        "entry",
      ),

    indexList: (_kind, value, set) =>
      rowList<Json>(
        asArray(value),
        () => 0,
        (item, _i, update, remove) =>
          el("div", { class: "row" }, [
            labelled(
              "index",
              numberInput(asNumber(item), (n) => update(n ?? 0)),
            ),
            removeButton(remove),
          ]),
        set,
        "index",
      ),

    optIndexList: (kind, value, set) => {
      const declared = value !== null;
      const wrap = el("div", {});
      wrap.appendChild(checkbox(declared, (on) => set(on ? [] : null), "declared"));
      if (declared) wrap.appendChild(editors.indexList(kind, value, set));
      return wrap;
    },

    enum: (kind, value, set) =>
      catalogueSelect(kind, typeof value === "string" ? value : null, set),

    flagSet: (kind, value, set) => flagChips(kind, value, set),

    arity: (_kind, value, set) => {
      const v = asRecord(value);
      const update = (patch: Record<string, Json>): void => {
        set({
          min: v.min ?? 0,
          max: v.max ?? null,
          step: v.step ?? 0,
          also_exact: v.also_exact ?? null,
          ...patch,
        });
      };
      return el("div", { class: "ctl" }, [
        labelled(
          "min",
          numberInput(asNumber(v.min) ?? 0, (n) => update({ min: n ?? 0 })),
        ),
        labelled(
          "max",
          numberInput(asNumber(v.max), (n) => update({ max: n }), { placeholder: "∞" }),
        ),
        labelled(
          "step",
          numberInput(asNumber(v.step) ?? 0, (n) => update({ step: n ?? 0 })),
        ),
        labelled(
          "also exactly",
          numberInput(asNumber(v.also_exact), (n) => update({ also_exact: n }), {
            placeholder: "none",
          }),
        ),
        el("span", {
          class: "hint",
          text: "blank max = unbounded; step and also-exact are for clause-shaped commands",
        }),
      ]);
    },

    roleMap: (_kind, value, set) =>
      rowList<Json>(
        asArray(value),
        () => ({ index: 0, role: "Value" }),
        (item, _i, update, remove) => {
          const row = asRecord(item);
          return el("div", { class: "row" }, [
            labelled(
              "index",
              numberInput(asNumber(row.index) ?? 0, (n) =>
                update({ index: n ?? 0, role: row.role ?? "Value" }),
              ),
            ),
            labelled(
              "role",
              catalogueSelect({ tag: "enum", catalogue: "argRole" }, asString(row.role), (role) =>
                update({ index: row.index ?? 0, role }),
              ),
            ),
            removeButton(remove),
          ]);
        },
        set,
        "role",
      ),

    presentationMap: (_kind, value, set) =>
      rowList<Json>(
        asArray(value),
        () => ({ index: 0, presentation: "BlockScript" }),
        (item, _i, update, remove) => {
          const row = asRecord(item);
          return el("div", { class: "row" }, [
            labelled(
              "index",
              numberInput(asNumber(row.index) ?? 0, (n) =>
                update({ index: n ?? 0, presentation: row.presentation ?? "BlockScript" }),
              ),
            ),
            labelled(
              "presentation",
              catalogueSelect(
                { tag: "enum", catalogue: "argPresentation" },
                asString(row.presentation),
                (presentation) => update({ index: row.index ?? 0, presentation }),
              ),
            ),
            removeButton(remove),
          ]);
        },
        set,
        "presentation",
      ),

    prefixMap: (_kind, value, set) =>
      rowList<Json>(
        asArray(value),
        () => ({ index: 0, arity: { kind: "Unknown" } }),
        (item, _i, update, remove) => {
          const row = asRecord(item);
          const arity = asRecord(row.arity);
          const kindName = asString(arity.kind) || "Unknown";
          const nInput = numberInput(asNumber(arity.n), (n) =>
            update({ index: row.index ?? 0, arity: { kind: kindName, n: n ?? 0 } }),
          );
          nInput.disabled = kindName === "Unknown";
          return el("div", { class: "row" }, [
            labelled(
              "index",
              numberInput(asNumber(row.index) ?? 0, (n) =>
                update({ index: n ?? 0, arity: row.arity ?? { kind: "Unknown" } }),
              ),
            ),
            labelled(
              "appended",
              catalogueSelect({ tag: "enum", catalogue: "appendedArity" }, kindName, (k) =>
                update({
                  index: row.index ?? 0,
                  arity:
                    k === "Unknown" || k === null
                      ? { kind: "Unknown" }
                      : { kind: k, n: asNumber(arity.n) ?? 0 },
                }),
              ),
            ),
            labelled("n", nInput),
            removeButton(remove),
          ]);
        },
        set,
        "command prefix",
      ),

    argTypeMap: (_kind, value, set) =>
      rowList<Json>(
        asArray(value),
        () => ({ index: 0, expected: null, shimmers: false, transparent_from: [] }),
        (item, _i, update, remove) => {
          const row = asRecord(item);
          const patch = (next: Record<string, Json>): void => {
            update({ ...clone(row), ...next });
          };
          return el("div", { class: "row wide" }, [
            el("div", { class: "ctl" }, [
              labelled(
                "index",
                numberInput(asNumber(row.index) ?? 0, (n) => patch({ index: n ?? 0 })),
              ),
              labelled(
                "expected",
                catalogueSelect(
                  { tag: "enum", catalogue: "tclType", optional: true },
                  typeof row.expected === "string" ? row.expected : null,
                  (t) => patch({ expected: t }),
                ),
              ),
              checkbox(asBool(row.shimmers), (on) => patch({ shimmers: on }), "shimmers"),
              removeButton(remove),
            ]),
            el("div", {}, [
              el("span", { class: "hint", text: "transparent from" }),
              flagChips(
                { tag: "flagSet", catalogue: "tclType", optional: false },
                row.transparent_from ?? [],
                (types) => patch({ transparent_from: types ?? [] }),
              ),
            ]),
          ]);
        },
        set,
        "type hint",
      ),

    argValueMap: (_kind, value, set) =>
      rowList<Json>(
        asArray(value),
        () => ({ index: 0, values: [] }),
        (item, _i, update, remove) => {
          const row = asRecord(item);
          return el("div", { class: "row wide" }, [
            el("div", { class: "ctl" }, [
              labelled(
                "index",
                numberInput(asNumber(row.index) ?? 0, (n) =>
                  update({ index: n ?? 0, values: row.values ?? [] }),
                ),
              ),
              removeButton(remove),
            ]),
            argValueList(asArray(row.values), (values) =>
              update({ index: row.index ?? 0, values }),
            ),
          ]);
        },
        set,
        "argument",
      ),

    options: (_kind, value, set) =>
      rowList<Json>(
        asArray(value),
        () => ({
          name: "-flag",
          detail: "",
          dialects: null,
          aliases: [],
          introduced_version: null,
          deprecated_version: null,
          retired_version: null,
          deprecation_fix: null,
          min_abbrev: null,
          value: null,
        }),
        optionRow,
        set,
        "option",
      ),

    forms: (_kind, value, set) =>
      rowList<Json>(
        asArray(value),
        () => ({ kind: "Default", synopsis: "", dialects: null }),
        (item, _i, update, remove) => {
          const row = asRecord(item);
          const synopsis = textInput(
            asString(row.synopsis),
            (text) => update({ ...clone(row), synopsis: text }),
            { placeholder: "cmd arg ?option?" },
          );
          synopsis.style.flex = "1 1 18rem";
          return el("div", { class: "row" }, [
            labelled(
              "kind",
              catalogueSelect({ tag: "enum", catalogue: "formKind" }, asString(row.kind), (k) =>
                update({ ...clone(row), kind: k }),
              ),
            ),
            synopsis,
            removeButton(remove),
          ]);
        },
        set,
        "form",
      ),

    sideEffects: (_kind, value, set) =>
      rowList<Json>(
        asArray(value),
        () => ({
          target: "Unknown",
          reads: false,
          writes: false,
          connection_side: "None",
          dialects: null,
        }),
        (item, _i, update, remove) => {
          const row = asRecord(item);
          const patch = (next: Record<string, Json>): void => {
            update({ ...clone(row), ...next });
          };
          return el("div", { class: "row" }, [
            labelled(
              "target",
              catalogueSelect(
                { tag: "enum", catalogue: "sideEffectTarget" },
                asString(row.target),
                (t) => patch({ target: t }),
              ),
            ),
            checkbox(asBool(row.reads), (on) => patch({ reads: on }), "reads"),
            checkbox(asBool(row.writes), (on) => patch({ writes: on }), "writes"),
            labelled(
              "side",
              catalogueSelect(
                { tag: "enum", catalogue: "connectionSide" },
                asString(row.connection_side),
                (s) => patch({ connection_side: s }),
              ),
            ),
            removeButton(remove),
          ]);
        },
        set,
        "side effect",
      ),

    setterConstraints: (_kind, value, set) =>
      rowList<Json>(
        asArray(value),
        () => ({ arg_index: 0, required_prefix: "/", code: "IRULE3101", message: "" }),
        (item, _i, update, remove) => {
          const row = asRecord(item);
          const patch = (next: Record<string, Json>): void => {
            update({ ...clone(row), ...next });
          };
          return el("div", { class: "row wide" }, [
            el("div", { class: "ctl" }, [
              labelled(
                "index",
                numberInput(asNumber(row.arg_index) ?? 0, (n) => patch({ arg_index: n ?? 0 })),
              ),
              labelled(
                "required prefix",
                textInput(asString(row.required_prefix), (t) => patch({ required_prefix: t }), {
                  size: 8,
                }),
              ),
              labelled(
                "code",
                textInput(asString(row.code), (t) => patch({ code: t }), { size: 10 }),
              ),
              removeButton(remove),
            ]),
            textInput(asString(row.message), (t) => patch({ message: t }), {
              placeholder: "explanation shown with the diagnostic",
            }),
          ]);
        },
        set,
        "constraint",
      ),

    manufacturerMethods: (_kind, value, set) =>
      rowList<Json>(
        asArray(value),
        () => ({
          keyword: "create",
          visibility: "Exported",
          names_instance_at: null,
          definition_body_at: null,
          constructor_args_from: 1,
        }),
        (item, _i, update, remove) => {
          const row = asRecord(item);
          const patch = (next: Record<string, Json>): void => {
            update({ ...clone(row), ...next }, false);
          };
          const visibility = el("select", {}, [
            el("option", { value: "Exported", text: "Exported" }),
            el("option", { value: "Unexported", text: "Unexported" }),
          ]);
          visibility.value = asString(row.visibility) || "Exported";
          visibility.addEventListener("change", () => patch({ visibility: visibility.value }));
          return el("div", { class: "row wide" }, [
            labelled(
              "keyword",
              textInput(asString(row.keyword), (text) => patch({ keyword: text }), { size: 16 }),
            ),
            labelled("visibility", visibility),
            labelled(
              "instance name at",
              numberInput(asNumber(row.names_instance_at), (n) => patch({ names_instance_at: n })),
            ),
            labelled(
              "definition body at",
              numberInput(asNumber(row.definition_body_at), (n) =>
                patch({ definition_body_at: n }),
              ),
            ),
            labelled(
              "constructor args from",
              numberInput(asNumber(row.constructor_args_from) ?? 0, (n) =>
                patch({ constructor_args_from: n ?? 0 }),
              ),
            ),
            removeButton(remove),
          ]);
        },
        set,
        "manufacturer method",
      ),

    subSubCommands: (_kind, value, set) =>
      rowList<Json>(
        asArray(value),
        () => ({ name: "", detail: "", synopsis: "", dialects: null }),
        (item, _i, update, remove) => {
          const row = asRecord(item);
          const patch = (next: Record<string, Json>): void => {
            update({ ...clone(row), ...next });
          };
          return el("div", { class: "row" }, [
            labelled(
              "name",
              textInput(asString(row.name), (t) => patch({ name: t }), { size: 12 }),
            ),
            labelled(
              "detail",
              textInput(asString(row.detail), (t) => patch({ detail: t })),
            ),
            labelled(
              "synopsis",
              textInput(asString(row.synopsis), (t) => patch({ synopsis: t })),
            ),
            removeButton(remove),
          ]);
        },
        set,
        "operation",
      ),

    hover: (_kind, value, set) => {
      const declared = value !== null;
      const wrap = el("div", {});
      wrap.appendChild(
        checkbox(
          declared,
          (on) =>
            set(
              on
                ? {
                    summary: "",
                    synopsis: [],
                    snippet: "",
                    source: "",
                    examples: "",
                    return_value: "",
                  }
                : null,
            ),
          "declared",
        ),
      );
      if (!declared) return wrap;

      const hover = asRecord(value);
      const patch = (next: Record<string, Json>): void => {
        set({ ...clone(hover), ...next });
      };
      const area = (
        label: string,
        key: string,
        rows: number,
        placeholder?: string,
      ): HTMLElement => {
        const box = el("textarea", { rows, placeholder: placeholder ?? "" });
        box.value = asString(hover[key]);
        box.addEventListener("input", () => patch({ [key]: box.value }));
        return el("div", { class: "field" }, [
          el("div", { class: "lbl" }, [el("span", { class: "name", text: label })]),
          box,
        ]);
      };

      wrap.appendChild(area("Summary", "summary", 2, "One sentence — what the command does."));
      wrap.appendChild(
        el("div", { class: "field" }, [
          el("div", { class: "lbl" }, [el("span", { class: "name", text: "Synopsis lines" })]),
          editors.textList({ tag: "textList" }, hover.synopsis ?? [], (lines) =>
            patch({ synopsis: lines }),
          ),
        ]),
      );
      wrap.appendChild(area("Prose", "snippet", 6, "The full hover body."));
      wrap.appendChild(area("Examples", "examples", 4, "Runnable Tcl, one idea per block."));
      wrap.appendChild(area("Return value", "return_value", 2, "What the command evaluates to."));
      wrap.appendChild(
        area("Source", "source", 1, "Where this came from — e.g. Tcl man page lappend.n"),
      );
      return wrap;
    },

    subCommands: (_kind, value, set) => {
      const list = asArray(value);
      const wrap = el("div", {});

      list.forEach((sub, i) => {
        const draft = asRecord(sub);
        const body = el("div", { class: "body" });
        const details = el("details", { class: "group" }, [
          el("summary", {}, [
            document.createTextNode(asString(draft.name) || "(unnamed)"),
            el("span", { class: "n", text: "subcommand" }),
          ]),
          body,
        ]);
        body.appendChild(
          el("div", { style: "display:flex;justify-content:flex-end" }, [
            el("button", {
              type: "button",
              class: "ghost",
              text: "Remove this subcommand",
              onclick: () => {
                const next = list.slice();
                next.splice(i, 1);
                set(next);
              },
            }),
          ]),
        );
        ctx.buildSubcommandForm(body, draft, () => set(list.slice()));
        wrap.appendChild(details);
      });

      wrap.appendChild(
        el("button", {
          type: "button",
          class: "ghost",
          text: "+ subcommand",
          onclick: () => {
            const fresh = ctx.newSubcommand();
            fresh.name = "new";
            set([...list, fresh]);
          },
        }),
      );
      return wrap;
    },

    objectClass: (_kind, value, set) => {
      const declared = value !== null;
      const wrap = el("div", {});
      wrap.appendChild(
        checkbox(
          declared,
          (on) =>
            set(
              on
                ? {
                    class_name: "",
                    instance_methods: [],
                    superclasses: [],
                    allow_unknown_methods: false,
                  }
                : null,
            ),
          "declared",
        ),
      );
      if (!declared) return wrap;

      const cls = asRecord(value);
      const patch = (next: Record<string, Json>): void => {
        set({ ...clone(cls), ...next });
      };
      const field = (label: string, hint: string, control: Child): HTMLElement =>
        el("div", { class: "field" }, [
          el("div", { class: "lbl" }, [el("span", { class: "name", text: label })]),
          control,
          el("span", { class: "hint", text: hint }),
        ]);

      wrap.appendChild(
        field(
          "Class name",
          "Fully qualified, and for a TclOO class equal to this factory command's own name — that is how the registry resolves it.",
          textInput(asString(cls.class_name), (text) => patch({ class_name: text }), {
            placeholder: "::ticklecharts::chart",
          }),
        ),
      );
      wrap.appendChild(
        field(
          "Superclasses",
          "Direct superclass names, each itself a class command. A method not found here is resolved through these.",
          editors.textList({ tag: "textList" }, asStringList(cls.superclasses), (names) =>
            patch({ superclasses: names }),
          ),
        ),
      );
      wrap.appendChild(
        checkbox(
          asBool(cls.allow_unknown_methods),
          (on) => patch({ allow_unknown_methods: on }),
          "accept unrecognised instance methods (the class has an `unknown` handler or builds methods at runtime)",
        ),
      );

      // Instance methods are `&[SubCommand]`, so they take the subcommand form
      // unchanged — the same shape the DSL reuses for its `method` rows.
      const methods = asArray(cls.instance_methods);
      methods.forEach((method, i) => {
        const draft = asRecord(method);
        const body = el("div", { class: "body" });
        const details = el("details", { class: "group" }, [
          el("summary", {}, [
            document.createTextNode(asString(draft.name) || "(unnamed)"),
            el("span", { class: "n", text: "method" }),
          ]),
          body,
        ]);
        body.appendChild(
          el("div", { style: "display:flex;justify-content:flex-end" }, [
            el("button", {
              type: "button",
              class: "ghost",
              text: "Remove this method",
              onclick: () => {
                const next = methods.slice();
                next.splice(i, 1);
                patch({ instance_methods: next });
              },
            }),
          ]),
        );
        ctx.buildSubcommandForm(body, draft, () => patch({ instance_methods: methods.slice() }));
        wrap.appendChild(details);
      });
      wrap.appendChild(
        el("button", {
          type: "button",
          class: "ghost",
          text: "+ method",
          onclick: () => {
            const fresh = ctx.newSubcommand();
            fresh.name = "new";
            patch({ instance_methods: [...methods, fresh] });
          },
        }),
      );
      return wrap;
    },

    rustExpr: (kind, value, set) =>
      el("div", {}, [
        textInput(asString(value), (text) => set(text.trim() === "" ? null : text), {
          placeholder: kind.hint ?? "a Rust expression",
        }),
        el("span", {
          class: "hint",
          text:
            "The studio cannot model this as data — type the Rust expression to emit, e.g. " +
            `${kind.hint ?? "…"}.`,
        }),
      ]),
  };

  return editors;
}
