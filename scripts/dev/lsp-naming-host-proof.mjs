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

// Shared by the existing browser-worker and WASI protocol harnesses.
import assert from "node:assert/strict";

export const HOST_NAMING_SOURCE = 'proc host_old {value} { return $value }\nset host_marker "🧪 café"; proc host_caller {} {return [host_old 7]}\n';
export const OPAQUE_PUTS_NAMING_SOURCE = 'proc host_old {value} { return $value }\nputs "🧪 café"; puts [host_old 7]\n';

function offsetAt(source, position) {
    const lines = source.split("\n");
    const { line, character } = position;
    assert(Number.isInteger(line) && line >= 0 && line < lines.length);
    assert(Number.isInteger(character) && character >= 0 && character <= lines[line].length);
    const before = lines[line].charCodeAt(character - 1);
    const after = lines[line].charCodeAt(character);
    assert(!(before >= 0xd800 && before <= 0xdbff && after >= 0xdc00 && after <= 0xdfff),
        "a protocol position must not split a UTF-16 surrogate pair");
    return lines.slice(0, line).reduce((total, text) => total + text.length + 1, 0) + character;
}

function locations(value) {
    if (value == null) return [];
    return (Array.isArray(value) ? value : [value]).map((entry) => ({
        uri: entry.uri ?? entry.targetUri,
        range: entry.range ?? entry.targetSelectionRange ?? entry.targetRange,
    }));
}

function assertTargets(definition, references, uri, source, name) {
    const declaration = { start: { line: 0, character: 5 }, end: { line: 0, character: 5 + name.length } };
    const call = source.split("\n")[1].indexOf(name);
    assert(call >= 0);
    assert.deepEqual(locations(definition), [{ uri, range: declaration }]);
    const actual = locations(references);
    const expected = [{ uri, range: declaration }, { uri, range: {
        start: { line: 1, character: call }, end: { line: 1, character: call + name.length },
    } }];
    const ordered = (items) => items.map((item) => JSON.stringify(item)).sort();
    assert.deepEqual(ordered(actual), ordered(expected), "references include exactly the selected declaration and call");
}

function applyCapturedEdit(edit, uri, source) {
    assert(edit && typeof edit === "object", "rename must return a complete WorkspaceEdit");
    const edits = [];
    for (const [target, changes] of Object.entries(edit.changes ?? {})) {
        assert.equal(target, uri, "every edited URI belongs to this captured fixture");
        assert(Array.isArray(changes));
        edits.push(...changes);
    }
    for (const change of edit.documentChanges ?? []) {
        assert(change.textDocument && Array.isArray(change.edits), "fixture rename must contain text edits, not resource operations");
        assert.equal(change.textDocument.uri, uri);
        assert(change.textDocument.version == null || change.textDocument.version === 1,
            "versioned edits match the captured source revision");
        edits.push(...change.edits);
    }
    assert.equal(edits.length, 2, "rename edits both the declaration and call");
    const spans = edits.map((change) => {
        const start = offsetAt(source, change.range.start);
        const end = offsetAt(source, change.range.end);
        assert(start < end);
        assert.equal(source.slice(start, end), "host_old");
        assert.equal(change.newText, "host_new");
        return { start, end, text: change.newText };
    }).sort((left, right) => right.start - left.start);
    let boundary = source.length;
    let result = source;
    for (const span of spans) {
        assert(span.end <= boundary, "edit ranges must not overlap or repeat");
        result = result.slice(0, span.start) + span.text + result.slice(span.end);
        boundary = span.start;
    }
    assert.equal(result, source.replaceAll("host_old", "host_new"));
    return result;
}

/** Run once after the host's ordinary startup/scan readiness has completed. */
export async function runNamingHostProof(session, uri, check, publish = () => {}, budgetMs = 30_000) {
    async function request(method, params, deadline) {
        const remaining = deadline - Date.now();
        assert(remaining > 0, `${method} exceeded the host reply budget`);
        let timer;
        try {
            const response = await Promise.race([
                session.request(method, params, remaining),
                new Promise((_, reject) => { timer = setTimeout(() => reject(new Error(`${method} timed out`)), remaining); }),
            ]);
            assert(response && response.error === undefined, `${method} failed: ${JSON.stringify(response)}`);
            assert(Date.now() < deadline, `${method} exceeded the host reply budget`);
            return response.result;
        } finally {
            clearTimeout(timer);
        }
    }
    async function completed(source, version) {
        const deadline = Date.now() + budgetMs;
        for (;;) {
            // This command reads the existing publisher owner and never schedules
            // analysis. Retry only its explicit pending/stale state, never a naming
            // answer, diagnostic count, or expected target.
            const report = await request("workspace/executeCommand", {
                command: "tcl-lsp.currentDiagnostics", arguments: [{ uri, version, source }],
            }, deadline);
            if (report?.status === "complete") {
                assert.equal(report.uri, uri);
                assert.equal(report.version, version);
                assert.equal(report.source, source);
                for (const field of ["revision", "inputsEpoch"]) assert(typeof report[field] === "string" && /^\d+$/.test(report[field]));
                assert.equal(typeof report.resultId, "string");
                assert(report.resultId.length > 0);
                assert(Array.isArray(report.diagnostics));
                return;
            }
            assert(report?.status === "unavailable" && ["pending", "stale"].includes(report.reason), `unavailable diagnostic publication: ${JSON.stringify(report)}`);
            await new Promise((resolve) => setTimeout(resolve, 25));
        }
    }
    async function targets(source, name) {
        const position = { line: 1, character: source.split("\n")[1].indexOf(name) + 2 };
        const params = { textDocument: { uri }, position };
        const definition = await request("textDocument/definition", params, Date.now() + budgetMs);
        const references = await request("textDocument/references", { ...params, context: { includeDeclaration: true } }, Date.now() + budgetMs);
        assertTargets(definition, references, uri, source, name);
        return position;
    }
    await publish(uri, HOST_NAMING_SOURCE);
    session.notify("textDocument/didOpen", {
        textDocument: { uri, languageId: "tcl", version: 1, text: HOST_NAMING_SOURCE },
    });
    await completed(HOST_NAMING_SOURCE, 1);
    const position = await targets(HOST_NAMING_SOURCE, "host_old");
    const edit = await request("textDocument/rename", {
        textDocument: { uri }, position, newName: "host_new",
    }, Date.now() + budgetMs);
    const applied = applyCapturedEdit(edit, uri, HOST_NAMING_SOURCE);
    await publish(uri, applied);
    session.notify("textDocument/didChange", {
        textDocument: { uri, version: 2 }, contentChanges: [{ text: applied }],
    });
    await completed(applied, 2);
    await targets(applied, "host_new");
    check("protocol rename applies every UTF-16 edit and the first new-revision targets agree", true);
    async function opaquePutsSourceAbstains() {
        await publish(uri, OPAQUE_PUTS_NAMING_SOURCE);
        session.notify("textDocument/didChange", {
            textDocument: { uri, version: 3 }, contentChanges: [{ text: OPAQUE_PUTS_NAMING_SOURCE }],
        });
        await completed(OPAQUE_PUTS_NAMING_SOURCE, 3);
        const position = { line: 1, character: OPAQUE_PUTS_NAMING_SOURCE.split("\n")[1].indexOf("host_old") + 2 };
        const params = { textDocument: { uri }, position };
        const definition = await request("textDocument/definition", params, Date.now() + budgetMs);
        const references = await request("textDocument/references", { ...params, context: { includeDeclaration: true } }, Date.now() + budgetMs);
        assert.deepEqual(locations(definition), [], "earlier opaque puts prevents a unique call definition");
        assert.deepEqual(locations(references), [], "earlier opaque puts prevents a unique call reference owner");
        let timer;
        try {
            const deadline = Date.now() + budgetMs;
            const response = await Promise.race([
                session.request("textDocument/rename", {
                    textDocument: { uri }, position: { line: 0, character: 7 }, newName: "host_new",
                }, budgetMs),
                new Promise((_, reject) => { timer = setTimeout(() => reject(new Error("opaque puts rename refusal timed out")), budgetMs); }),
            ]);
            assert(Date.now() < deadline, "opaque puts rename refusal exceeded the host reply budget");
            assert.equal(response?.error?.code, -32602);
            assert.equal(response.error.data?.code, "unknown-completed-command-world");
            assert.equal(response.error.data?.reason, "a completed original command world is unavailable for collision coverage");
            assert(response.result == null, "opaque puts never receives an edit grant");
        } finally {
            clearTimeout(timer);
        }
        check("opaque puts retains uncertainty and abstains from navigation and rename", true);
    }
    await opaquePutsSourceAbstains();
}
