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
import {
  describeXcSource,
  requestPartitionRename,
  RuleInfo,
  RuleScratchRegistry,
  ServerCommandSender,
  sourcePositionArgument,
  translateXcDocument,
} from "../bigipSourceActions";
import { workspaceEditFromLsp } from "../clientCore";

interface SentCommand {
  command: string;
  args: unknown[];
}

function recordingSender(answer: (sent: SentCommand) => unknown): {
  send: ServerCommandSender;
  sent: SentCommand[];
} {
  const sent: SentCommand[] = [];
  const send: ServerCommandSender = async (command, args) => {
    const entry = { command, args };
    sent.push(entry);
    return answer(entry);
  };
  return { send, sent };
}

const SNAPSHOT = {
  uri: "file:///rules/app.irul",
  version: 7,
  source: "when HTTP_REQUEST {\n    pool p\n}\n",
};

function rule(body: string, documentVersion: number | null = 3): RuleInfo {
  return {
    name: "r",
    fullPath: "/Common/r",
    header: "ltm rule /Common/r",
    body,
    bodyStartOffset: 21,
    bodyEndOffset: 21 + body.length,
    uri: "file:///bigip.conf",
    documentVersion,
  };
}

suite("BIG-IP source actions", () => {
  test("XC document translation names the document it describes", async () => {
    const { send, sent } = recordingSender(() => ({
      coverage_pct: 100,
      items: [],
      source: { purpose: "document", uri: SNAPSHOT.uri, version: 7, dialect: "f5-irules" },
    }));
    const outcome = await translateXcDocument(send, SNAPSHOT, "both");
    assert.deepStrictEqual(
      sent.map((entry) => entry.command),
      ["tcl-lsp.xcTranslateDocument"],
    );
    assert.deepStrictEqual(sent[0].args, [SNAPSHOT, "both"]);
    assert.strictEqual(outcome.kind, "translated");
    if (outcome.kind !== "translated") return;
    assert.deepStrictEqual(outcome.source, {
      purpose: "document",
      uri: SNAPSHOT.uri,
      version: 7,
      dialect: "f5-irules",
    });
    assert.match(describeXcSource(outcome.source), /open document \(version 7, f5-irules\)/);
  });

  test("a stale XC snapshot is refused, never reparsed standalone", async () => {
    const { send, sent } = recordingSender(() => ({ unavailable: "stale" }));
    const outcome = await translateXcDocument(send, SNAPSHOT, "both");
    assert.deepStrictEqual(outcome, { kind: "stale" });
    assert.strictEqual(sent.length, 1, "no standalone fallback for a stale snapshot");
  });

  test("a non-iRules document is translated as standalone text and says so", async () => {
    const { send, sent } = recordingSender((entry) =>
      entry.command === "tcl-lsp.xcTranslateDocument"
        ? { unavailable: "notIrules" }
        : { coverage_pct: 50, items: [] },
    );
    const outcome = await translateXcDocument(send, SNAPSHOT, "console");
    assert.deepStrictEqual(sent[1], {
      command: "tcl-lsp.xcTranslate",
      args: [SNAPSHOT.source, "console"],
    });
    assert.strictEqual(outcome.kind, "translated");
    if (outcome.kind !== "translated") return;
    assert.deepStrictEqual(outcome.source, { purpose: "standalone", reason: "notIrules" });
    assert.match(describeXcSource(outcome.source), /standalone iRule source/);
  });

  test("an XC answer about another document is not accepted", async () => {
    const { send } = recordingSender(() => ({
      items: [],
      source: { purpose: "document", uri: SNAPSHOT.uri, version: 6, dialect: "f5-irules" },
    }));
    const outcome = await translateXcDocument(send, SNAPSHOT, "both");
    assert.strictEqual(outcome.kind, "failed");
  });

  test("rule write-back sends the server's receipt and keeps the returned one", async () => {
    const written = rule("    pool q\n", null);
    const { send, sent } = recordingSender(() => ({ applied: true, rule: written }));
    const registry = new RuleScratchRegistry(() => send);
    registry.track("untitled:Untitled-1", rule("    pool p\n"));

    const first = await registry.writeBack("untitled:Untitled-1", "    pool q\n");
    assert.deepStrictEqual(first, { kind: "written", rule: written });
    assert.deepStrictEqual(sent[0], {
      command: "tcl-lsp.writeRuleBack",
      args: [rule("    pool p\n"), "    pool q\n"],
    });
    assert.deepStrictEqual(registry.receipt("untitled:Untitled-1"), written);

    // Saving the body the server already holds sends nothing.
    assert.deepStrictEqual(await registry.writeBack("untitled:Untitled-1", "    pool q\n"), {
      kind: "unchanged",
    });
    assert.strictEqual(sent.length, 1);
    assert.strictEqual(registry.writeBack("untitled:Other", "x"), undefined);
  });

  test("concurrent rule saves are serialised on the latest receipt", async () => {
    let release: (() => void) | undefined;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    const receipts: unknown[] = [];
    const send: ServerCommandSender = async (_command, args) => {
      receipts.push(args[0]);
      if (receipts.length === 1) await gate;
      return { applied: true, rule: rule(args[1] as string, null) };
    };
    const registry = new RuleScratchRegistry(() => send);
    registry.track("untitled:Untitled-2", rule("a\n"));
    const first = registry.writeBack("untitled:Untitled-2", "b\n");
    const second = registry.writeBack("untitled:Untitled-2", "c\n");
    await Promise.resolve();
    assert.strictEqual(receipts.length, 1, "the second save waits for the first answer");
    release?.();
    await first;
    await second;
    assert.deepStrictEqual(receipts, [rule("a\n"), rule("b\n", null)]);
  });

  test("a refused rule write-back keeps the original receipt and reports why", async () => {
    const { send } = recordingSender(() => ({ applied: false, reason: "stale" }));
    const registry = new RuleScratchRegistry(() => send);
    registry.track("untitled:Untitled-3", rule("a\n"));
    assert.deepStrictEqual(await registry.writeBack("untitled:Untitled-3", "b\n"), {
      kind: "refused",
      reason: "stale",
    });
    assert.deepStrictEqual(registry.receipt("untitled:Untitled-3"), rule("a\n"));
    const unavailable = new RuleScratchRegistry(() => undefined);
    unavailable.track("untitled:Untitled-4", rule("a\n"));
    assert.deepStrictEqual(await unavailable.writeBack("untitled:Untitled-4", "b\n"), {
      kind: "refused",
      reason: "noServer",
    });
  });

  test("cursor positions travel as versioned LSP positions", () => {
    const document = { version: 12 } as vscode.TextDocument;
    assert.deepStrictEqual(sourcePositionArgument(document, new vscode.Position(3, 5)), {
      position: { line: 3, character: 5 },
      version: 12,
    });
  });

  test("a versioned server edit converts only at its own document version", () => {
    const edit = {
      documentChanges: [
        {
          textDocument: { uri: "file:///bigip.conf", version: 4 },
          edits: [
            {
              range: { start: { line: 0, character: 0 }, end: { line: 0, character: 3 } },
              newText: "new",
            },
          ],
        },
      ],
    };
    const current = workspaceEditFromLsp(edit, () => 4);
    assert.ok(current, "the current version converts");
    assert.strictEqual(current.size, 1);
    assert.strictEqual(
      workspaceEditFromLsp(edit, () => 5),
      undefined,
    );
    assert.strictEqual(
      workspaceEditFromLsp(edit, () => undefined),
      undefined,
    );
    const unversioned = workspaceEditFromLsp({
      changes: {
        "file:///bigip.conf": [
          {
            range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
            newText: "x",
          },
        ],
      },
    });
    assert.strictEqual(unversioned?.size, 1);
  });

  test("a partition rename names its document version and refuses a stale edit", async () => {
    const { send, sent } = recordingSender(() => ({
      success: true,
      edit: {
        documentChanges: [
          {
            textDocument: { uri: "file:///bigip.conf", version: 2 },
            edits: [
              {
                range: { start: { line: 0, character: 0 }, end: { line: 1, character: 0 } },
                newText: "auth partition B { }\n",
              },
            ],
          },
        ],
      },
    }));
    const stale = await requestPartitionRename(send, "file:///bigip.conf", "A", "B", 2, () => 3);
    assert.deepStrictEqual(sent[0].args, ["file:///bigip.conf", "A", "B", 2]);
    assert.strictEqual(typeof stale, "string");
    const current = await requestPartitionRename(send, "file:///bigip.conf", "A", "B", 2, () => 2);
    assert.ok(current instanceof vscode.WorkspaceEdit);
  });
});
