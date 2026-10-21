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
import type { Diagnostic as LspDiagnostic } from "vscode-languageclient";
import {
  CurrentDiagnosticsSuppressedError,
  diagnosticConversionTopics,
  CurrentDiagnosticsUnavailableError,
  acquireSessionDialectOverride,
  captureAnalysisContext,
  invalidateAnalysisContext,
  isAnalysisContextCurrent,
  setCurrentDiagnosticsTransportProvider,
  setUnleasedSessionDialectOverride,
  waitForDiagnostics,
  waitForDiagnosticsReport,
  currentDiagnosticsReportMatches,
  currentDiagnosticsReportIsStillPublished,
  validateCurrentDiagnosticsPublication,
  resourceConfiguration,
  waitForProposalDiagnostics,
  proposalDiagnosticsReportMatches,
  type CurrentDiagnosticsTransport,
  type SessionDialectOverrideLease,
} from "../chat/diagnosticAccess";
import { activate, getDocUri } from "./helper";
import { DiagnosticSourceOperation, buildPromptContextBlock } from "../chat/contextPack";
import type { CommandContext } from "../chat/types";

interface Snapshot {
  uri: string;
  version: number;
  source: string;
}

function complete(snapshot: Snapshot, diagnostics: LspDiagnostic[]) {
  return {
    status: "complete",
    ...snapshot,
    dialectId: "tcl8.6",
    revision: "1",
    inputsEpoch: "1",
    resultId: "result-1",
    diagnostics,
  };
}

function diagnosticMessage(item: LspDiagnostic): string {
  return typeof item.message === "string" ? item.message : item.message.value;
}

function diagnostic(code: string): LspDiagnostic {
  return {
    range: {
      start: { line: 0, character: 0 },
      end: { line: 0, character: 1 },
    },
    severity: 1,
    code,
    message: code,
  };
}

const testSessionIdentity = {};

function transport(
  request: CurrentDiagnosticsTransport["request"],
  current: () => boolean = () => true,
  converted: string[] = [],
): CurrentDiagnosticsTransport {
  let published: unknown;
  return {
    sessionIdentity: testSessionIdentity,
    request: async (params) => {
      const response = await request(params);
      published = response;
      return response;
    },
    validatePublication: async (expected) => {
      const publication = published as Record<string, unknown> | undefined;
      const keys = [
        "uri",
        "version",
        "source",
        "dialectId",
        "revision",
        "inputsEpoch",
        "resultId",
      ] as const;
      return current() &&
        publication?.status === "complete" &&
        keys.every((key) => publication[key] === expected[key])
        ? { ...expected, status: "current" }
        : { status: "unavailable" };
    },
    isCurrent: current,
    convert: async (items) => {
      converted.push(...items.map((item) => String(item.code)));
      return items.map((item) => {
        const result = new vscode.Diagnostic(
          new vscode.Range(
            item.range.start.line,
            item.range.start.character,
            item.range.end.line,
            item.range.end.character,
          ),
          diagnosticMessage(item),
        );
        result.code = item.code;
        return result;
      });
    },
  };
}

suite("Chat current diagnostic publication adapter", () => {
  suiteSetup(async function () {
    this.timeout(60_000);
    await activate(getDocUri("simple.tcl"));
  });

  async function openFixture(content: string): Promise<vscode.TextDocument> {
    return vscode.workspace.openTextDocument({ language: "tcl", content });
  }

  test("an older dialect lease cannot clear a newer session override", async () => {
    const operations: Array<string | null> = [];
    const activeTransport = transport(async (request) => complete(request, []));
    activeTransport.setSessionDialectOverride = async (dialect) => {
      operations.push(dialect);
    };
    const provider = setCurrentDiagnosticsTransportProvider(() => activeTransport);
    try {
      const older = await acquireSessionDialectOverride("f5-irules");
      const newer = await acquireSessionDialectOverride("tcl");
      assert.strictEqual(isAnalysisContextCurrent(older.analysisContext), false);
      assert.strictEqual(await older.restore(), undefined);
      const writeContext = await newer.restore();
      assert.ok(writeContext);
      assert.strictEqual(isAnalysisContextCurrent(writeContext), true);
      assert.deepStrictEqual(operations, ["f5-irules", "tcl", null]);
    } finally {
      provider.dispose();
    }
  });

  test("an acknowledged stale acquisition clears only its own server override", async () => {
    const operations: Array<string | null> = [];
    let acknowledge: (() => void) | undefined;
    let entered: (() => void) | undefined;
    const enteredRequest = new Promise<void>((resolve) => {
      entered = resolve;
    });
    const activeTransport = transport(async (request) => complete(request, []));
    activeTransport.setSessionDialectOverride = async (dialect) => {
      operations.push(dialect);
      if (dialect !== null) {
        entered?.();
        await new Promise<void>((resolve) => {
          acknowledge = resolve;
        });
      }
    };
    const provider = setCurrentDiagnosticsTransportProvider(() => ({ ...activeTransport }));
    try {
      const acquisition = acquireSessionDialectOverride("f5-irules");
      await enteredRequest;
      assert.strictEqual(captureAnalysisContext(), undefined);
      invalidateAnalysisContext();
      acknowledge?.();
      await assert.rejects(
        acquisition,
        (error: unknown) =>
          error instanceof CurrentDiagnosticsUnavailableError && error.reason === "stale",
      );
      assert.deepStrictEqual(operations, ["f5-irules", null]);
    } finally {
      provider.dispose();
    }
  });

  test("a stale successor clears an acknowledged temporary predecessor", async () => {
    const operations: Array<string | null> = [];
    const activeTransport = transport(async (request) => complete(request, []));
    activeTransport.setSessionDialectOverride = async (dialect) => {
      operations.push(dialect);
    };
    const provider = setCurrentDiagnosticsTransportProvider(() => ({ ...activeTransport }));
    try {
      const predecessor = await acquireSessionDialectOverride("f5-irules");
      const successor = acquireSessionDialectOverride("tcl");
      invalidateAnalysisContext();
      await assert.rejects(
        successor,
        (error: unknown) =>
          error instanceof CurrentDiagnosticsUnavailableError && error.reason === "stale",
      );
      assert.strictEqual(await predecessor.restore(), undefined);
      assert.deepStrictEqual(operations, ["f5-irules", null]);
      assert.ok(captureAnalysisContext(), "cleanup of the predecessor ends the transition");
    } finally {
      provider.dispose();
    }
  });

  test("a queued ownerless clear survives context invalidation after a temporary write", async () => {
    const operations: Array<string | null> = [];
    let acknowledgeWrite: (() => void) | undefined;
    let enteredWrite: (() => void) | undefined;
    const writeEntered = new Promise<void>((resolve) => {
      enteredWrite = resolve;
    });
    const activeTransport = transport(async (request) => complete(request, []));
    activeTransport.setSessionDialectOverride = async (dialect) => {
      operations.push(dialect);
      if (dialect !== null) {
        enteredWrite?.();
        await new Promise<void>((resolve) => {
          acknowledgeWrite = resolve;
        });
      }
    };
    const provider = setCurrentDiagnosticsTransportProvider(() => ({ ...activeTransport }));
    try {
      const acquisition = acquireSessionDialectOverride("f5-irules");
      await writeEntered;
      const clear = setUnleasedSessionDialectOverride(null);
      invalidateAnalysisContext();
      acknowledgeWrite?.();
      await assert.rejects(
        acquisition,
        (error: unknown) =>
          error instanceof CurrentDiagnosticsUnavailableError && error.reason === "stale",
      );
      await clear;
      assert.deepStrictEqual(operations, ["f5-irules", null]);
      assert.ok(captureAnalysisContext(), "the acknowledged ownerless clear ends the transition");
    } finally {
      provider.dispose();
    }
  });

  test("context invalidation during lease cleanup clears the override without a write context", async () => {
    const operations: Array<string | null> = [];
    let acknowledgeClear: (() => void) | undefined;
    let enteredClear: (() => void) | undefined;
    const clearEntered = new Promise<void>((resolve) => {
      enteredClear = resolve;
    });
    const activeTransport = transport(async (request) => complete(request, []));
    activeTransport.setSessionDialectOverride = async (dialect) => {
      operations.push(dialect);
      if (dialect === null) {
        enteredClear?.();
        await new Promise<void>((resolve) => {
          acknowledgeClear = resolve;
        });
      }
    };
    const provider = setCurrentDiagnosticsTransportProvider(() => activeTransport);
    try {
      const lease = await acquireSessionDialectOverride("f5-irules");
      const restoration = lease.restore();
      await clearEntered;
      assert.strictEqual(captureAnalysisContext(), undefined);
      invalidateAnalysisContext();
      acknowledgeClear?.();
      assert.strictEqual(await restoration, undefined);
      assert.deepStrictEqual(operations, ["f5-irules", null]);
      assert.ok(captureAnalysisContext(), "acknowledged cleanup ends the transition");
      assert.ok(captureAnalysisContext(), "a completed cleanup ends the transition");
    } finally {
      provider.dispose();
    }
  });

  test("pending is retried and a completed empty publication remains empty", async () => {
    const document = await openFixture("set adapterPending 1");
    let calls = 0;
    const lease = setCurrentDiagnosticsTransportProvider(() =>
      transport(async (request) => {
        calls += 1;
        return calls === 1 ? { status: "unavailable", reason: "pending" } : complete(request, []);
      }),
    );
    try {
      const result = await waitForDiagnostics(document.uri, {
        timeout: 1_000,
        expectedSource: document.getText(),
      });
      assert.deepStrictEqual(result, []);
      assert.strictEqual(calls, 2, "pending must not be reported as a completed empty result");
    } finally {
      lease.dispose();
    }
  });

  test("a complete empty report remains distinct from display suppression", async () => {
    const document = await openFixture("set adapterCompleteEmpty 1");
    const lease = setCurrentDiagnosticsTransportProvider(() =>
      transport(async (request) => complete(request, [])),
    );
    try {
      const report = await waitForDiagnosticsReport(document.uri, {
        timeout: 1_000,
        expectedSource: document.getText(),
      });
      assert.strictEqual(report.response.status, "complete");
      assert.deepStrictEqual(report.diagnostics, []);
      assert.strictEqual(report.response.diagnostics.length, 0);
      assert.strictEqual(report.response.source, document.getText());
    } finally {
      lease.dispose();
    }
  });

  test("a source edit during the request fails as stale instead of rebasing", async () => {
    const document = await openFixture("set adapterStale 1");
    let calls = 0;
    const lease = setCurrentDiagnosticsTransportProvider(() =>
      transport(async (request) => {
        calls += 1;
        const edit = new vscode.WorkspaceEdit();
        edit.replace(
          document.uri,
          new vscode.Range(0, 0, 0, document.getText().length),
          "set adapterStale 2",
        );
        assert.strictEqual(await vscode.workspace.applyEdit(edit), true);
        return complete(request, []);
      }),
    );
    try {
      await assert.rejects(
        waitForDiagnostics(document.uri, {
          timeout: 1_000,
          expectedSource: "set adapterStale 1",
        }),
        (error: unknown) =>
          error instanceof CurrentDiagnosticsUnavailableError && error.reason === "stale",
      );
      assert.strictEqual(calls, 1, "the old caller snapshot must not be silently replaced");
    } finally {
      lease.dispose();
    }
  });

  test("a current report retains the producer dialect and exact report/session receipt", async () => {
    const document = await openFixture("set adapterReport 1");
    let requests = 0;
    const activeTransport = transport(async (request) => {
      requests += 1;
      return complete(request, [diagnostic("REPORT")]);
    });
    const lease = setCurrentDiagnosticsTransportProvider(() => activeTransport);
    try {
      const report = await waitForDiagnosticsReport(document.uri, {
        timeout: 1_000,
        expectedSource: document.getText(),
      });
      assert.strictEqual(report.response.dialectId, "tcl8.6");
      assert.strictEqual(report.response.revision, "1");
      assert.strictEqual(report.response.inputsEpoch, "1");
      assert.strictEqual(report.response.resultId, "result-1");
      assert.strictEqual(report.response.source, document.getText());
      assert.strictEqual(report.sessionIdentity, testSessionIdentity);
      assert.strictEqual(report.diagnostics.length, 1);
      const promptContext = await buildPromptContextBlock(
        { request: { references: [] } } as unknown as CommandContext,
        {
          document,
          code: document.getText(),
          includeDiagnostics: false,
          includeSymbolDefinitions: false,
          includeEventMetadata: false,
          completedDiagnosticsReport: report,
        },
      );
      assert.match(promptContext, /Completed resource analysis: tcl8\.6/);
      assert.strictEqual(
        requests,
        1,
        "the caller reuses the full completed report, not a fresh query",
      );
      assert.strictEqual(
        currentDiagnosticsReportMatches(report, {
          document,
          uri: document.uri,
          version: document.version,
          source: document.getText(),
        }),
        true,
      );
      invalidateAnalysisContext();
      assert.strictEqual(
        currentDiagnosticsReportMatches(report, {
          document,
          uri: document.uri,
          version: document.version,
          source: document.getText(),
        }),
        false,
      );
    } finally {
      lease.dispose();
    }
  });

  test("a resource setting change invalidates a report even when source and session stay current", async () => {
    const document = await openFixture("set adapterResourceContext 1");
    const folder = vscode.workspace.getWorkspaceFolder(document.uri);
    assert.ok(folder, "the resource setting control needs its actual workspace folder");
    const configuration = vscode.workspace.getConfiguration("tclLsp", document.uri);
    const priorDialect = configuration.get<string>("dialect");
    const lease = setCurrentDiagnosticsTransportProvider(() =>
      transport(async (request) => complete(request, [])),
    );
    try {
      const report = await waitForDiagnosticsReport(document.uri, {
        timeout: 1_000,
        expectedSource: document.getText(),
      });
      const originalResourceConfiguration = report.resourceConfiguration;
      assert.ok(originalResourceConfiguration);
      assert.strictEqual(
        await configuration.update("dialect", "tcl9.0", vscode.ConfigurationTarget.WorkspaceFolder),
        undefined,
      );
      assert.notStrictEqual(
        resourceConfiguration(document),
        originalResourceConfiguration,
        "the shared owner observes the actual per-resource configuration change",
      );
      assert.strictEqual(
        currentDiagnosticsReportMatches(report, {
          document,
          uri: document.uri,
          version: document.version,
          source: document.getText(),
        }),
        false,
        "the captured publisher dialect cannot be carried across a changed resource profile",
      );
    } finally {
      await configuration.update(
        "dialect",
        priorDialect,
        vscode.ConfigurationTarget.WorkspaceFolder,
      );
      lease.dispose();
    }
  });

  test("a resource setting change during publication refuses the pinned report", async () => {
    const document = await openFixture("set adapterResourcePending 1");
    const folder = vscode.workspace.getWorkspaceFolder(document.uri);
    assert.ok(folder, "the resource setting control needs its actual workspace folder");
    const configuration = vscode.workspace.getConfiguration("tclLsp", document.uri);
    const priorDialect = configuration.get<string>("dialect");
    let resolveReport!: () => void;
    const lease = setCurrentDiagnosticsTransportProvider(() =>
      transport(
        (request) =>
          new Promise((resolve) => (resolveReport = () => resolve(complete(request, [])))),
      ),
    );
    try {
      const pending = waitForDiagnosticsReport(document.uri, {
        timeout: 1_000,
        expectedSource: document.getText(),
      });
      await new Promise((resolve) => setTimeout(resolve, 0));
      await configuration.update("dialect", "tcl9.0", vscode.ConfigurationTarget.WorkspaceFolder);
      resolveReport();
      await assert.rejects(
        pending,
        (error: unknown) =>
          error instanceof CurrentDiagnosticsUnavailableError && error.reason === "stale",
      );
    } finally {
      await configuration.update(
        "dialect",
        priorDialect,
        vscode.ConfigurationTarget.WorkspaceFolder,
      );
      lease.dispose();
    }
  });

  test("a retained report refuses a newer publisher epoch for the same source and session", async () => {
    const document = await openFixture("set adapterPublisherEpoch 1");
    let requests = 0;
    const activeTransport = transport(async (request) => {
      requests += 1;
      return requests === 1
        ? complete(request, [])
        : {
            ...complete(request, []),
            revision: "2",
            inputsEpoch: "2",
            resultId: "result-2",
          };
    });
    const lease = setCurrentDiagnosticsTransportProvider(() => activeTransport);
    try {
      const report = await waitForDiagnosticsReport(document.uri, {
        timeout: 1_000,
        expectedSource: document.getText(),
      });
      assert.strictEqual(
        await currentDiagnosticsReportIsStillPublished(report, {
          document,
          uri: document.uri,
          version: document.version,
          source: document.getText(),
        }),
        false,
        "a newer complete publication cannot replace the report retained by the caller",
      );
      assert.strictEqual(requests, 2, "the exact original report is joined once to the ledger");
    } finally {
      lease.dispose();
    }
  });

  test("a current report refuses to recapture after its original session changes", async () => {
    const document = await openFixture("set adapterReportRestart 1");
    let resolveOld!: (value: unknown) => void;
    let current: CurrentDiagnosticsTransport | undefined;
    let newCalls = 0;
    const oldTransport: CurrentDiagnosticsTransport = {
      sessionIdentity: {},
      request: () => new Promise((resolve) => (resolveOld = resolve)),
      convert: async () => [],
      isCurrent: () => current === oldTransport,
    };
    const newTransport = transport(async (request) => {
      newCalls += 1;
      return complete(request, []);
    });
    current = oldTransport;
    const lease = setCurrentDiagnosticsTransportProvider(() => current);
    try {
      const pending = waitForDiagnosticsReport(document.uri, {
        timeout: 1_000,
        expectedSource: document.getText(),
      });
      await new Promise((resolve) => setTimeout(resolve, 0));
      current = newTransport;
      resolveOld(
        complete(
          { uri: document.uri.toString(), version: document.version, source: document.getText() },
          [],
        ),
      );
      await assert.rejects(
        pending,
        (error: unknown) =>
          error instanceof CurrentDiagnosticsUnavailableError && error.reason === "stale",
      );
      assert.strictEqual(
        newCalls,
        0,
        "a later session must not replace the original report session",
      );
    } finally {
      lease.dispose();
    }
  });

  test("a report request refuses a changed source instead of rebasing its pinned session", async () => {
    const original = "set adapterReportSource 1";
    const document = await openFixture(original);
    const originalVersion = document.version;
    let resolveOriginal!: (value: unknown) => void;
    let calls = 0;
    const activeTransport = transport(
      (request) => {
        calls += 1;
        return new Promise((resolve) => {
          resolveOriginal = () => resolve(complete(request, []));
        });
      },
      () => true,
    );
    const lease = setCurrentDiagnosticsTransportProvider(() => activeTransport);
    try {
      const pending = waitForDiagnosticsReport(document.uri, {
        timeout: 1_000,
        expectedSource: original,
      });
      await new Promise((resolve) => setTimeout(resolve, 0));
      const edit = new vscode.WorkspaceEdit();
      edit.replace(
        document.uri,
        new vscode.Range(0, 0, 0, original.length),
        "set adapterReportSource 2",
      );
      assert.strictEqual(await vscode.workspace.applyEdit(edit), true);
      resolveOriginal(
        complete({ uri: document.uri.toString(), version: originalVersion, source: original }, []),
      );
      await assert.rejects(
        pending,
        (error: unknown) =>
          error instanceof CurrentDiagnosticsUnavailableError && error.reason === "stale",
      );
      assert.strictEqual(calls, 1, "a changed report source must not trigger a new request");
    } finally {
      lease.dispose();
    }
  });

  test("a delivered disabled receipt remains typed suppression, not clean empty", async () => {
    const document = await openFixture("set adapterSuppressed 1");
    const lease = setCurrentDiagnosticsTransportProvider(() =>
      transport(async (request) => ({
        status: "suppressed",
        reason: "disabled",
        uri: request.uri,
        version: request.version,
        source: request.source,
        dialectId: "tcl8.6",
        revision: "2",
        inputsEpoch: "2",
        resultId: "result-disabled",
        diagnostics: [],
      })),
    );
    try {
      await assert.rejects(
        waitForDiagnostics(document.uri, {
          timeout: 1_000,
          expectedSource: document.getText(),
        }),
        (error: unknown) =>
          error instanceof CurrentDiagnosticsSuppressedError &&
          error.reason === "disabled" &&
          error.receipt.source === document.getText(),
      );
      await assert.rejects(
        waitForDiagnosticsReport(document.uri, {
          timeout: 1_000,
          expectedSource: document.getText(),
        }),
        (error: unknown) =>
          error instanceof CurrentDiagnosticsSuppressedError &&
          error.reason === "disabled" &&
          error.receipt.dialectId === "tcl8.6",
      );
    } finally {
      lease.dispose();
    }
  });

  test("conversion is bounded by the original diagnostic deadline", async () => {
    const document = await openFixture("set adapterDeadline 1");
    const lease = setCurrentDiagnosticsTransportProvider(() => ({
      sessionIdentity: testSessionIdentity,
      request: async (request) => complete(request, []),
      convert: async () => new Promise<vscode.Diagnostic[]>(() => {}),
      isCurrent: () => true,
    }));
    try {
      await assert.rejects(
        waitForDiagnostics(document.uri, {
          timeout: 25,
          expectedSource: document.getText(),
        }),
        (error: unknown) =>
          error instanceof CurrentDiagnosticsUnavailableError && error.reason === "timeout",
      );
    } finally {
      lease.dispose();
    }
  });

  test("a same-client stop/start generation rejects an ABA response", async () => {
    const document = await openFixture("set adapterSameClientRestart 1");
    let generation = 0;
    let calls = 0;
    let oldConvertCalls = 0;
    const lease = setCurrentDiagnosticsTransportProvider(() => {
      const capturedGeneration = generation;
      return {
        sessionIdentity: testSessionIdentity,
        request: async (request) => {
          calls += 1;
          if (calls === 1) {
            // Model stop/start on the same LanguageClient object: the
            // object identity is unchanged, but a state transition advances
            // its generation while the old request is in flight.
            generation += 1;
            return complete(request, [diagnostic("OLD")]);
          }
          return complete(request, [diagnostic("NEW")]);
        },
        convert: async (items) => {
          if (capturedGeneration === 0) oldConvertCalls += 1;
          return items.map(
            (item) => new vscode.Diagnostic(new vscode.Range(0, 0, 0, 1), diagnosticMessage(item)),
          );
        },
        isCurrent: () => capturedGeneration === generation,
      };
    });
    try {
      const result = await waitForDiagnostics(document.uri, {
        timeout: 1_000,
        expectedSource: document.getText(),
      });
      assert.strictEqual(result.length, 1);
      assert.strictEqual(result[0].message, "NEW");
      assert.strictEqual(oldConvertCalls, 0);
      assert.strictEqual(calls, 2);
    } finally {
      lease.dispose();
    }
  });

  test("a converter resolving after a blocked deadline remains a timeout", async () => {
    const document = await openFixture("set adapterLateConversion 1");
    const lease = setCurrentDiagnosticsTransportProvider(() => ({
      sessionIdentity: testSessionIdentity,
      request: async (request) => complete(request, []),
      convert: async () => {
        const finishAt = Date.now() + 40;
        while (Date.now() < finishAt) {
          // Block the event loop so the deadline timer cannot win the race.
        }
        return [];
      },
      isCurrent: () => true,
    }));
    try {
      await assert.rejects(
        waitForDiagnostics(document.uri, {
          timeout: 10,
          expectedSource: document.getText(),
        }),
        (error: unknown) =>
          error instanceof CurrentDiagnosticsUnavailableError && error.reason === "timeout",
      );
    } finally {
      lease.dispose();
    }
  });

  test("a server restart discards an old client's response and converter", async () => {
    const document = await openFixture("set adapterRestart 1");
    let current: CurrentDiagnosticsTransport | undefined;
    let oldConvertCalls = 0;
    const newTransport = transport(async (request) => complete(request, [diagnostic("NEW")]));
    const oldTransport: CurrentDiagnosticsTransport = {
      sessionIdentity: testSessionIdentity,
      request: async (request) => {
        current = newTransport;
        return complete(request, [diagnostic("OLD")]);
      },
      convert: async () => {
        oldConvertCalls += 1;
        return [];
      },
      isCurrent: () => current === oldTransport,
    };
    current = oldTransport;
    const lease = setCurrentDiagnosticsTransportProvider(() => current);
    try {
      const result = await waitForDiagnostics(document.uri, {
        timeout: 1_000,
        expectedSource: document.getText(),
      });
      assert.strictEqual(result.length, 1);
      assert.strictEqual(result[0].code, "NEW");
      assert.strictEqual(oldConvertCalls, 0, "conversion must stay with the client that answered");
    } finally {
      lease.dispose();
    }
  });

  test("proposal diagnostics bind complete results to one original source and proposal", async () => {
    const document = await openFixture("set adapterProposalOriginal 1");
    const source = document.getText();
    const proposal = "set adapterProposalReplacement 1";
    let seen: { uri: string; version: number; source: string; proposal: string } | undefined;
    const activeTransport = transport(async () =>
      complete(
        {
          uri: document.uri.toString(),
          version: document.version,
          source,
        },
        [diagnostic("PROPOSAL")],
      ),
    ) as CurrentDiagnosticsTransport & {
      requestProposal(params: {
        uri: string;
        version: number;
        source: string;
        proposal: string;
      }): Promise<unknown>;
    };
    activeTransport.requestProposal = async (request) => {
      seen = request;
      return {
        status: "complete",
        ...request,
        dialectId: "tcl8.6",
        revision: "Revision(4):17",
        inputsEpoch: "2",
        workspaceIndexGeneration: "3",
        resultId: "proposal-1",
        diagnostics: [diagnostic("PROPOSAL")],
      };
    };
    const lease = setCurrentDiagnosticsTransportProvider(() => activeTransport);
    try {
      const result = await waitForProposalDiagnostics(document.uri, proposal, {
        timeout: 1_000,
        expectedSource: source,
      });
      assert.strictEqual(result.diagnostics.length, 1);
      assert.strictEqual(result.diagnostics[0].code, "PROPOSAL");
      assert.deepStrictEqual(
        result.response,
        {
          status: "complete",
          uri: document.uri.toString(),
          version: document.version,
          source,
          proposal,
          dialectId: "tcl8.6",
          revision: "Revision(4):17",
          inputsEpoch: "2",
          workspaceIndexGeneration: "3",
          resultId: "proposal-1",
          diagnostics: [diagnostic("PROPOSAL")],
        },
        "the full producer response must remain attached to converted diagnostics",
      );
      assert.strictEqual(result.sessionIdentity, activeTransport.sessionIdentity);
      assert.strictEqual(result.document, document);
      assert.strictEqual(
        proposalDiagnosticsReportMatches(result, {
          document,
          uri: document.uri,
          version: document.version,
          source,
          proposal,
        }),
        true,
      );
      const replacementSession = setCurrentDiagnosticsTransportProvider(() => ({
        ...activeTransport,
        sessionIdentity: {},
        isCurrent: () => true,
      }));
      try {
        assert.strictEqual(
          proposalDiagnosticsReportMatches(result, {
            document,
            uri: document.uri,
            version: document.version,
            source,
            proposal,
          }),
          false,
          "a report from a prior client session must not be reused",
        );
      } finally {
        replacementSession.dispose();
      }
      assert.deepStrictEqual(seen, {
        uri: document.uri.toString(),
        version: document.version,
        source,
        proposal,
      });
      assert.strictEqual(
        document.getText(),
        source,
        "proposal analysis must not mutate the source document",
      );
    } finally {
      lease.dispose();
    }
  });

  test("proposal diagnostics require the server-selected resource dialect", async () => {
    const document = await openFixture("set adapterProposalMissingDialect 1");
    const source = document.getText();
    const activeTransport = transport(async () =>
      complete({ uri: document.uri.toString(), version: document.version, source }, []),
    ) as CurrentDiagnosticsTransport & {
      requestProposal(params: {
        uri: string;
        version: number;
        source: string;
        proposal: string;
      }): Promise<unknown>;
    };
    activeTransport.requestProposal = async (request) => ({
      status: "complete",
      ...request,
      revision: "Revision(4):17",
      inputsEpoch: "2",
      workspaceIndexGeneration: "3",
      resultId: "proposal-missing-dialect",
      diagnostics: [],
    });
    const lease = setCurrentDiagnosticsTransportProvider(() => activeTransport);
    try {
      await assert.rejects(
        waitForProposalDiagnostics(document.uri, "set candidate 1", {
          timeout: 1_000,
          expectedSource: source,
        }),
        (error: unknown) =>
          error instanceof CurrentDiagnosticsUnavailableError &&
          error.reason === "malformedResponse",
      );
    } finally {
      lease.dispose();
    }
  });

  test("proposal diagnostics reject a source edit instead of rebasing the request", async () => {
    const document = await openFixture("set adapterProposalStale 1");
    const source = document.getText();
    let answer: (() => void) | undefined;
    let entered: (() => void) | undefined;
    const requested = new Promise<void>((resolve) => {
      entered = resolve;
    });
    const version = document.version;
    const requests: { uri: string; version: number; source: string; proposal: string }[] = [];
    const activeTransport = transport(async () =>
      complete(
        {
          uri: document.uri.toString(),
          version: document.version,
          source,
        },
        [],
      ),
    ) as CurrentDiagnosticsTransport & {
      requestProposal(params: {
        uri: string;
        version: number;
        source: string;
        proposal: string;
      }): Promise<unknown>;
    };
    activeTransport.requestProposal = async (request) => {
      requests.push(request);
      entered?.();
      await new Promise<void>((resolve) => {
        answer = () => resolve();
      });
      return { status: "unavailable", reason: "stale" };
    };
    const lease = setCurrentDiagnosticsTransportProvider(() => activeTransport);
    try {
      const pending = waitForProposalDiagnostics(document.uri, "set candidate 1", {
        timeout: 1_000,
        expectedSource: source,
      });
      await requested;
      const edit = new vscode.WorkspaceEdit();
      edit.insert(document.uri, new vscode.Position(0, 0), "# changed\n");
      await vscode.workspace.applyEdit(edit);
      answer?.();
      await assert.rejects(
        pending,
        (error: unknown) =>
          error instanceof CurrentDiagnosticsUnavailableError && error.reason === "stale",
      );
      assert.deepStrictEqual(
        requests,
        [{ uri: document.uri.toString(), version, source, proposal: "set candidate 1" }],
        "the original request must not retry against a later source",
      );
    } finally {
      lease.dispose();
    }
  });

  test("a restart during conversion discards the old client's converted result", async () => {
    const document = await openFixture("set adapterConvertRestart 1");
    let current: CurrentDiagnosticsTransport | undefined;
    const newTransport = transport(async (request) => complete(request, [diagnostic("NEW")]));
    const oldTransport: CurrentDiagnosticsTransport = {
      sessionIdentity: testSessionIdentity,
      request: async (request) => complete(request, [diagnostic("OLD")]),
      convert: async (items) => {
        current = newTransport;
        return items.map(
          (item) => new vscode.Diagnostic(new vscode.Range(0, 0, 0, 1), diagnosticMessage(item)),
        );
      },
      isCurrent: () => current === oldTransport,
    };
    current = oldTransport;
    const lease = setCurrentDiagnosticsTransportProvider(() => current);
    try {
      const result = await waitForDiagnostics(document.uri, {
        timeout: 1_000,
        expectedSource: document.getText(),
      });
      assert.strictEqual(result.length, 1);
      assert.strictEqual(result[0].message, "NEW");
    } finally {
      lease.dispose();
    }
  });
  test("source-only prompt context retains the resource hint without claiming completed analysis", async () => {
    const document = await openFixture("set promptSourceOnly 1");
    const lease = setCurrentDiagnosticsTransportProvider(() =>
      transport(async (request) => complete(request, [])),
    );
    try {
      const block = await buildPromptContextBlock(
        { request: { references: [] } } as unknown as CommandContext,
        {
          document,
          code: document.getText(),
          includeDiagnostics: false,
          includeSymbolDefinitions: false,
          includeEventMetadata: false,
        },
      );
      assert.ok(block.includes("Captured resource dialect hint (not completed analysis):"));
      assert.ok(!block.includes("Completed resource analysis:"));
    } finally {
      lease.dispose();
    }
  });

  test("source-only prompt context rejects an analysis generation retired during packing", async () => {
    const document = await openFixture("set promptRetired 1");
    const lease = setCurrentDiagnosticsTransportProvider(() =>
      transport(async (request) => complete(request, [])),
    );
    let first = true;
    try {
      await assert.rejects(
        buildPromptContextBlock({ request: { references: [] } } as unknown as CommandContext, {
          document,
          code: document.getText(),
          includeDiagnostics: false,
          includeSymbolDefinitions: false,
          includeEventMetadata: false,
          isSourceCurrent: () => {
            if (first) {
              first = false;
              invalidateAnalysisContext();
            }
            return true;
          },
        }),
        /captured prompt source or analysis input changed/,
      );
    } finally {
      lease.dispose();
    }
  });
  test("prompt packing refuses a completed report replaced in the publisher ledger", async () => {
    const document = await openFixture("set promptLedger 1");
    const publisher = transport(async (request) => complete(request, []));
    const lease = setCurrentDiagnosticsTransportProvider(() => publisher);
    try {
      const report = await waitForDiagnosticsReport(document.uri, {
        timeout: 1000,
        expectedSource: document.getText(),
      });
      publisher.validatePublication = async () => ({ status: "unavailable" });
      await assert.rejects(
        buildPromptContextBlock({ request: { references: [] } } as unknown as CommandContext, {
          document,
          code: document.getText(),
          completedDiagnosticsReport: report,
          includeDiagnostics: false,
          includeSymbolDefinitions: false,
          includeEventMetadata: false,
        }),
        /report was retired while prompt context was packed/,
      );
    } finally {
      lease.dispose();
    }
  });
  test("a still-published report remains valid after its analysis query budget", async () => {
    const document = await openFixture("set promptLongLivedPublication 1");
    let requests = 0;
    const publisher = transport(async (request) => {
      requests += 1;
      return complete(request, []);
    });
    const lease = setCurrentDiagnosticsTransportProvider(() => publisher);
    try {
      const started = Date.now();
      const queryBudget = 250;
      const report = await waitForDiagnosticsReport(document.uri, {
        timeout: queryBudget,
        expectedSource: document.getText(),
      });
      await new Promise((resolve) =>
        setTimeout(resolve, Math.max(0, started + queryBudget + 10 - Date.now())),
      );
      assert.strictEqual(
        await validateCurrentDiagnosticsPublication(
          report,
          {
            document,
            uri: document.uri,
            version: document.version,
            source: document.getText(),
          },
          1000,
        ),
        true,
      );
      assert.strictEqual(
        requests,
        1,
        "publication validation does not restart the completed analysis query",
      );
    } finally {
      lease.dispose();
    }
  });
  test("canonical conversion metadata supplies proposal topics without a local code list", () => {
    const known = new vscode.Diagnostic(new vscode.Range(0, 0, 0, 1), "known");
    known.code = { value: "W100", target: vscode.Uri.parse("https://example.invalid/W100") };
    const unknown = new vscode.Diagnostic(new vscode.Range(0, 0, 0, 1), "unknown");
    unknown.code = "UNKNOWN_CONVERSION";
    const topics = diagnosticConversionTopics([known, unknown]);
    assert.strictEqual(topics.length, 1);
    assert.strictEqual(topics[0].diagnostic, known);
    assert.strictEqual(topics[0].description, "Unbraced expr -> braced expr");
  });

  test("source operation keeps one completed publication and refuses its retirement", async () => {
    const cancellation = new vscode.CancellationTokenSource();
    const document = await openFixture("set operationPublication 1");
    await vscode.window.showTextDocument(document);
    let requests = 0;
    const publisher = transport(async (request) => {
      requests += 1;
      return complete(request, []);
    });
    const lease = setCurrentDiagnosticsTransportProvider(() => publisher);
    try {
      const operation = await DiagnosticSourceOperation.resolve({
        request: { references: [] },
        token: cancellation.token,
      } as unknown as CommandContext);
      assert.ok(operation);
      const report = await operation.readReport(1000);
      assert.strictEqual(operation.promptOptions().completedDiagnosticsReport, report);
      await operation.assertPublished();
      assert.strictEqual(requests, 1);
      publisher.validatePublication = async () => ({ status: "unavailable" });
      await assert.rejects(operation.assertPublished(), /publication is no longer current/);
      assert.strictEqual(requests, 1, "retirement check never reissues the analysis report");
    } finally {
      lease.dispose();
      cancellation.dispose();
    }
  });

  test("source operation does not retag a retired analysis generation", async () => {
    const cancellation = new vscode.CancellationTokenSource();
    const document = await openFixture("set operationGeneration 1");
    await vscode.window.showTextDocument(document);
    const publisher = transport(async (request) => complete(request, []));
    const lease = setCurrentDiagnosticsTransportProvider(() => publisher);
    try {
      const operation = await DiagnosticSourceOperation.resolve({
        request: { references: [] },
        token: cancellation.token,
      } as unknown as CommandContext);
      assert.ok(operation);
      await operation.readReport(1000);
      invalidateAnalysisContext();
      assert.strictEqual(operation.isCurrent(), false);
      await assert.rejects(operation.assertPublished(), /publication is no longer current/);
    } finally {
      lease.dispose();
      cancellation.dispose();
    }
  });
  test("source operation adopts only its actual acknowledged override lease", async () => {
    const cancellation = new vscode.CancellationTokenSource();
    const document = await openFixture("set operationOwnedOverride 1");
    await vscode.window.showTextDocument(document);
    let acknowledgements = 0;
    const publisher = transport(async (request) => complete(request, []));
    publisher.setSessionDialectOverride = async () => {
      acknowledgements += 1;
    };
    const transportLease = setCurrentDiagnosticsTransportProvider(() => publisher);
    let override: SessionDialectOverrideLease | undefined;
    try {
      const operation = await DiagnosticSourceOperation.resolve({
        request: { references: [] },
        token: cancellation.token,
      } as unknown as CommandContext);
      assert.ok(operation);
      override = await acquireSessionDialectOverride("f5-irules");
      operation.adoptOverride(override);
      const report = await operation.readReport(1000);
      assert.strictEqual(acknowledgements, 1);
      assert.strictEqual(
        report.analysisContext.overrideGeneration,
        override.analysisContext.overrideGeneration,
      );
      await operation.assertPublished();
      assert.ok(await override.restore());
      override = undefined;
      assert.strictEqual(operation.sourceUnchanged(), true);
      assert.strictEqual(operation.isCurrent(), false, "restoration does not retag the old report");
    } finally {
      if (override) await override.restore();
      transportLease.dispose();
      cancellation.dispose();
    }
  });
});
