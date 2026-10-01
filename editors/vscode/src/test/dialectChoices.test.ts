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

// Pure tests for the dialect picker's data: parsing the server's
// `tcl-lsp.listDialects` answer and grouping it by kind. `dialectChoices.ts`
// imports nothing from `vscode`, so this file needs no extension host.
import * as assert from "assert";
import { DialectChoice, groupDialectChoices, parseListedDialects } from "../dialectChoices";

function listed(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    name: "jim",
    display_name: "Jim Tcl",
    short_name: "Jim",
    kind: "language",
    description: "Jim Tcl",
    ...overrides,
  };
}

function choice(name: string, kind: DialectChoice["kind"]): DialectChoice {
  return { name, label: name, shortLabel: name, kind, description: name };
}

suite("Dialect choices", () => {
  test("a well-formed listDialects answer becomes choices in the order given", () => {
    const parsed = parseListedDialects([
      listed(),
      listed({
        name: "xilinx-eda-tcl",
        display_name: "Xilinx Vivado",
        short_name: "Vivado",
        kind: "packages",
        description: "Xilinx Vivado — Tcl 8.5 + vivado, sdc, upf",
      }),
    ]);
    assert.deepStrictEqual(parsed, [
      {
        name: "jim",
        label: "Jim Tcl",
        shortLabel: "Jim",
        kind: "language",
        description: "Jim Tcl",
      },
      {
        name: "xilinx-eda-tcl",
        label: "Xilinx Vivado",
        shortLabel: "Vivado",
        kind: "packages",
        description: "Xilinx Vivado — Tcl 8.5 + vivado, sdc, upf",
      },
    ]);
  });

  test("one partial entry rejects the whole answer rather than showing a partial picker", () => {
    for (const partial of [
      listed({ name: undefined }),
      listed({ display_name: "" }),
      listed({ short_name: 7 }),
      listed({ description: undefined }),
      listed({ kind: "dialect" }),
      listed({ kind: undefined }),
      null,
    ]) {
      assert.strictEqual(
        parseListedDialects([listed({ name: "tcl8.6" }), partial]),
        undefined,
        JSON.stringify(partial),
      );
    }
  });

  test("an answer that is not a non-empty list is rejected", () => {
    for (const answer of [undefined, null, "jim", {}, { name: "jim" }, []]) {
      assert.strictEqual(parseListedDialects(answer), undefined, JSON.stringify(answer));
    }
  });

  test("choices are grouped by kind, languages first, whatever order they arrive in", () => {
    const groups = groupDialectChoices([
      choice("tk", "packages"),
      choice("jim", "language"),
      choice("xilinx-eda-tcl", "packages"),
      choice("tcl8.6", "language"),
    ]);
    assert.deepStrictEqual(
      groups.map((group) => [group.title, group.choices.map((entry) => entry.name)]),
      [
        ["Dialects", ["jim", "tcl8.6"]],
        ["Tcl + packages", ["tk", "xilinx-eda-tcl"]],
      ],
    );
  });

  test("a kind with no choices gets no heading", () => {
    assert.deepStrictEqual(
      groupDialectChoices([choice("jim", "language")]).map((group) => group.title),
      ["Dialects"],
    );
    assert.deepStrictEqual(
      groupDialectChoices([choice("tk", "packages")]).map((group) => group.title),
      ["Tcl + packages"],
    );
    assert.deepStrictEqual(groupDialectChoices([]), []);
  });
});
