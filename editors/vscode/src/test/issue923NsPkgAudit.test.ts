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

import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, setTestContent, pollUntil, waitForDiagnostics } from "./helper";

// Issue #923 differential audit — the namespaces and packages/autoindex/source
// findings, at the editor tier. Every case below was verified against real
// tclsh (8.6.16 and 9.0.4) before it was written; the oracle transcript is
// quoted on each test. The tier matters because everything else about these
// findings is pinned inside the server: a regression in the client-visible
// wiring (positions, provider registration, the shapes VS Code asks for)
// would not show up anywhere else.

suite("Issue #923 audit: namespaces and packages", () => {
  const docUri = getDocUri("issue923NsPkgAudit.tcl");
  const libUri = getDocUri("issue923NsPkgAuditLib.tcl");

  async function withContent<T>(content: string, body: () => Promise<T>): Promise<T> {
    const doc = await activate(docUri);
    const editor = await vscode.window.showTextDocument(doc);
    const original = doc.getText();
    try {
      await setTestContent(editor, content);
      return await body();
    } finally {
      await setTestContent(editor, original);
    }
  }

  function codeOf(d: vscode.Diagnostic): string {
    return typeof d.code === "object" && d.code !== null ? String(d.code.value) : String(d.code);
  }

  function hoverText(hovers: vscode.Hover[]): string {
    return hovers
      .flatMap((h) => h.contents)
      .map((c) => (typeof c === "string" ? c : (c as vscode.MarkdownString).value))
      .join("\n");
  }

  async function hoverAt(position: vscode.Position): Promise<string> {
    const hovers = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeHoverProvider", docUri, position),
      (r) => Array.isArray(r) && r.length > 0,
      { timeout: 15_000, label: `hover at ${position.line}:${position.character}` },
    )) as vscode.Hover[];
    return hoverText(hovers);
  }

  // idx 85 — `namespace ensemble create -map` inside a proc declared with a
  // fully-qualified name at top level. Oracle (tclsh 8.6.16 / 9.0.4): the
  // script prints `shown`, so `::app::widget show` really dispatches to
  // `::app::widget::Show`. Go-to-definition answered; find-references did not.
  test("idx 85: an ensemble dispatch site is found from the declaration and from itself", async () => {
    const src =
      "namespace eval ::i923w::widget {\n" + // 0
      "    variable state 0\n" + // 1
      "}\n" + // 2
      "proc ::i923w::widget::Setup {} {\n" + // 3
      "    namespace ensemble create -map {\n" + // 4
      "        show ::i923w::widget::Show\n" + // 5
      "    }\n" + // 6
      "}\n" + // 7
      'proc ::i923w::widget::Show {} { puts "shown" }\n' + // 8
      "::i923w::widget::Setup\n" + // 9
      "puts [::i923w::widget show]\n"; // 10
    await withContent(src, async () => {
      const lineOf = (locs: vscode.Location[]): number[] =>
        locs.map((l) => l.range.start.line).sort((a, b) => a - b);

      // `Show` in its declaration on line 8 starts at column 22.
      const fromDecl = (await pollUntil(
        () =>
          vscode.commands.executeCommand(
            "vscode.executeReferenceProvider",
            docUri,
            new vscode.Position(8, 23),
          ),
        (r) => Array.isArray(r) && (r as vscode.Location[]).some((l) => l.range.start.line === 10),
        { timeout: 15_000, label: "references from the ensemble target declaration" },
      )) as vscode.Location[];
      assert.ok(
        lineOf(fromDecl).includes(10),
        `the \`[::i923w::widget show]\` dispatch must be a reference: ${JSON.stringify(lineOf(fromDecl))}`,
      );

      // `show` in the dispatch on line 10 starts at column 22.
      const fromCall = (await vscode.commands.executeCommand(
        "vscode.executeReferenceProvider",
        docUri,
        new vscode.Position(10, 23),
      )) as vscode.Location[];
      assert.deepStrictEqual(
        lineOf(fromCall),
        lineOf(fromDecl),
        "both query directions must return the same reference set",
      );
    });
  });

  // idx 3 / idx 4 — tcllib's `textutil::adjust` submodule reached through a
  // wildcard import. Oracle (tclsh 8.6.16 / 9.0.4 with tcllib 2.0):
  // `namespace origin adjust` -> `::textutil::adjust::adjust`.
  test("idx 3/4: a wildcard-imported textutil submodule command hovers qualified", async () => {
    const src =
      "package require textutil::adjust\n" +
      "namespace import textutil::adjust::*\n" +
      'set wrapped [adjust "a long help string" -length 20]\n' +
      'set final [indent $wrapped "  "]\n';
    await withContent(src, async () => {
      const adjust = await hoverAt(new vscode.Position(2, 14));
      assert.ok(
        adjust.includes("textutil::adjust::adjust"),
        `bare \`adjust\` must hover as the three-segment submodule command: ${adjust}`,
      );
      const indent = await hoverAt(new vscode.Position(3, 12));
      assert.ok(
        indent.includes("textutil::adjust::indent"),
        `bare \`indent\` must hover as \`textutil::adjust::indent\`: ${indent}`,
      );
    });
  });

  // idx 102 — hover on the `{file}` *parameter declaration* must not be
  // misattributed to the builtin `file` command. Oracle (tclsh 8.6.16 /
  // 9.0.4): the parameter deterministically drives which file is sourced.
  test("idx 102: a parameter named after a builtin hovers as a variable", async () => {
    const src =
      "proc ::i923tk::SourceLibFile {file} {\n" +
      "    namespace eval :: [list source [file join $::i923tk_library $file.tcl]]\n" +
      "}\n";
    await withContent(src, async () => {
      // The `file` of the parameter list `{file}` on line 0 starts at col 29.
      const decl = await hoverAt(new vscode.Position(0, 30));
      assert.ok(decl.includes("Variable"), `the parameter declaration must be a variable: ${decl}`);
      assert.ok(
        !decl.includes("file dirname") && !decl.includes("file exists"),
        `it must not be misattributed to the builtin \`file\` command: ${decl}`,
      );
    });
  });

  // idx 43 — `ticklecharts`' foreach-installed procs. Oracle (tclsh 8.6.16 /
  // 9.0.4): `new elist {1 2 3}` prints `elist {1 2 3}`, so each literal loop
  // element really becomes a callable command.
  test("idx 43: foreach-installed procs are outlined by their real names", async () => {
    const src =
      "namespace eval i923tc {\n" +
      "    namespace ensemble create -command ::i923new -subcommands {elist elist.n}\n" +
      "}\n" +
      "foreach ptype {elist elist.n} {\n" +
      "    proc i923tc::${ptype} {args} [string map [list %P% $ptype] {\n" +
      '        return "%P% $args"\n' +
      "    }]\n" +
      "}\n";
    await withContent(src, async () => {
      const symbols = (await pollUntil(
        () => vscode.commands.executeCommand("vscode.executeDocumentSymbolProvider", docUri),
        (r) => Array.isArray(r) && r.length > 0,
        { timeout: 15_000, label: "document symbols for the foreach-installed procs" },
      )) as vscode.DocumentSymbol[];
      const names: string[] = [];
      const walk = (items: vscode.DocumentSymbol[]): void => {
        for (const s of items) {
          names.push(s.name);
          walk(s.children ?? []);
        }
      };
      walk(symbols);
      const dump = JSON.stringify(names);
      for (const want of ["elist", "elist.n"]) {
        assert.ok(names.includes(want), `the outline must list \`${want}\`: ${dump}`);
      }
      assert.ok(
        !names.some((n) => n.includes("${ptype}")),
        `the outline must not show the unsubstituted placeholder: ${dump}`,
      );
    });
  });

  // idx 64 — an unmatched literal `{` inside a double-quoted string does not
  // stop substitution. Oracle (tclsh 8.6.16 / 9.0.4): `set ns "ctx"; puts
  // "{ $ns"` prints `{ ctx`, so the variable really is read and the `set`
  // before it is not a dead store.
  test("idx 64: an unmatched brace in a quoted string does not make the set a dead store", async () => {
    // The trailing unbraced `expr` carries a substitution, so it yields W100 —
    // the established way this suite proves analysis ran before asserting the
    // *absence* of a code. It uses its own variables so the read under test is
    // still only the one beside the unmatched brace.
    const src =
      'set i923ns "ctx"\n' +
      'puts "{ $i923ns"\n' +
      "set i923a 1\n" +
      "set i923y [expr $i923a + 1]\n" +
      "puts $i923y\n";
    await withContent(src, async () => {
      const diags = await waitForDiagnostics(docUri, {
        timeout: 20_000,
        predicate: (d) => d.some((x) => codeOf(x) === "W100"),
      });
      const dump = JSON.stringify(diags.map((d) => [d.range.start.line, codeOf(d)]));
      const onSet = diags.filter(
        (d) => d.range.start.line === 0 && ["W211", "W220"].includes(codeOf(d)),
      );
      assert.deepStrictEqual(
        onSet.map(codeOf),
        [],
        `\`$i923ns\` is read after the unmatched \`{\`, so no dead-store hint may fire on its \`set\`: ${dump}`,
      );
    });
  });

  // idx 41 — a computed `source` path must resolve to its real target, and
  // must never be percent-encoded raw into a bogus `file://` URI. Oracle
  // (tclsh 8.6.16 / 9.0.4): `set dir [file dirname [info script]]; source
  // [file join $dir x.tcl]` loads the sibling beside the sourcing file.
  test("idx 41: a computed source path resolves, an unfoldable one produces no link", async () => {
    const src =
      "set i923dir [file dirname [info script]]\n" +
      "source [file join $i923dir issue923NsPkgAuditLib.tcl]\n" +
      "source [file join $i923undeclared .. missing.tcl]\n";
    await withContent(src, async () => {
      const links = (await pollUntil(
        () => vscode.commands.executeCommand("vscode.executeLinkProvider", docUri),
        (r) => Array.isArray(r),
        { timeout: 15_000, label: "document links" },
      )) as vscode.DocumentLink[];
      const dump = JSON.stringify(links.map((l) => [l.range.start.line, l.target?.toString()]));
      const resolved = links.find((l) => l.range.start.line === 1);
      assert.ok(resolved?.target, `the computed sibling path must produce a link: ${dump}`);
      assert.ok(
        resolved.target.fsPath.endsWith("issue923NsPkgAuditLib.tcl"),
        `it must resolve to the real sibling file: ${dump}`,
      );
      assert.ok(
        !links.some((l) => l.range.start.line === 2),
        `an unfoldable computed path must produce no link at all: ${dump}`,
      );
      for (const l of links) {
        const t = l.target?.toString() ?? "";
        assert.ok(
          !t.includes("%5B") && !t.includes("$") && !t.includes("["),
          `no link may carry unevaluated substitution text: ${dump}`,
        );
      }
    });
  });

  // idx 65 / 75 — a namespace variable whose declaration lives in a sibling
  // document that never references this one. Oracle (tclsh 8.6.16 / 9.0.4,
  // sourcing both): `puts $::i923audit::colors::i923auditPalette` prints
  // `red green blue`, so the two files really populate one namespace.
  test("idx 65/75: a cross-document namespace variable resolves both ways", async () => {
    // Open the declaring document so the server holds both.
    await activate(libUri);
    const src = "puts $::i923audit::colors::i923auditPalette\n";
    await withContent(src, async () => {
      const defs = (await pollUntil(
        () =>
          vscode.commands.executeCommand(
            "vscode.executeDefinitionProvider",
            docUri,
            new vscode.Position(0, 36),
          ),
        (r) => Array.isArray(r) && (r as vscode.Location[]).length > 0,
        { timeout: 20_000, label: "cross-document namespace variable definition" },
      )) as vscode.Location[];
      assert.ok(
        defs.some((d) => d.uri.fsPath.endsWith("issue923NsPkgAuditLib.tcl")),
        `the sibling document's \`variable\` declaration must be the target: ${JSON.stringify(
          defs.map((d) => [d.uri.fsPath, d.range.start.line]),
        )}`,
      );

      const refs = (await pollUntil(
        () =>
          vscode.commands.executeCommand(
            "vscode.executeReferenceProvider",
            libUri,
            new vscode.Position(5, 15),
          ),
        (r) =>
          Array.isArray(r) &&
          (r as vscode.Location[]).some((l) => l.uri.fsPath.endsWith("issue923NsPkgAudit.tcl")),
        { timeout: 20_000, label: "cross-document namespace variable references" },
      )) as vscode.Location[];
      assert.ok(
        refs.some((l) => l.uri.fsPath.endsWith("issue923NsPkgAudit.tcl")),
        `find-references from the declaration must reach the consumer document: ${JSON.stringify(
          refs.map((l) => [l.uri.fsPath, l.range.start.line]),
        )}`,
      );
    });
  });

  // idx 44 — `${ns}::setdef`, where `ns` comes from `[namespace qualifiers
  // [self class]]`. Oracle (tclsh 8.6.16 / 9.0.4): the constructor really
  // dispatches to `::i923tc::setdef`, printing `id {-default hello}`.
  test("idx 44: a folded ${ns}:: head navigates to its proc", async () => {
    const src =
      "namespace eval i923tc {}\n" +
      "proc i923tc::setdef {d key args} { return 1 }\n" +
      "oo::class create i923tc::dataset {\n" +
      "    variable options\n" +
      "    constructor {value} {\n" +
      "        set ns [namespace qualifiers [self class]]\n" +
      "        ${ns}::setdef options id -default $value\n" +
      "    }\n" +
      "}\n";
    await withContent(src, async () => {
      const defs = (await pollUntil(
        () =>
          vscode.commands.executeCommand(
            "vscode.executeDefinitionProvider",
            docUri,
            new vscode.Position(6, 17),
          ),
        (r) => Array.isArray(r) && (r as vscode.Location[]).length > 0,
        { timeout: 15_000, label: "definition through a folded ${ns}:: head" },
      )) as vscode.Location[];
      assert.ok(
        defs.some((d) => d.range.start.line === 1),
        `the \`\${ns}::setdef\` head must reach the proc declaration: ${JSON.stringify(
          defs.map((d) => d.range.start.line),
        )}`,
      );
    });
  });
});
