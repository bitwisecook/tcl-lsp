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

/*
 * Drive the wasm language server through a scripted LSP session under node.
 *
 * This is the proof that the *real* server runs in a browser worker: nothing
 * here stubs a handler or shortcuts a provider. It speaks the wire protocol
 * `worker.js` speaks — one JSON-RPC message per call, as a string — so a pass
 * here means a page wired to `monaco-languageclient` sees the same answers.
 *
 * Run with `make lsp-server-wasm-test`, or `node test/e2e.mjs` against an
 * already-built `dist/`.
 */

import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { runInThisContext } from "node:vm";

const here = dirname(fileURLToPath(import.meta.url));
const dist = join(here, "..", "dist");

const SPEC_URI = "file:///w/demo.tclspec";
const TCL_URI = "file:///w/demo.tcl";

// A small, valid SpecTcl pack. `hover`'s `synopsis` is the property word the
// hover assertion aims at, and the pack shape is the one `specs/*.tclspec`
// uses, so the dialect route under test is the production one.
//
// (`summary` on the line above it has no `CommandSpec` in the pack DSL
// registry — W123 reports it as an unknown command and it has no hover. That
// is a pre-existing registry gap, identical in the native server, not
// something the browser build introduces.)
const SPEC_SOURCE = `speclib demo 1.0 {

command demo_place {
    dialects tcl8.6

    form Default {demo_place ?-cell name? ?-count n?}

    hover {
        summary {Place a demonstration cell.}
        synopsis {demo_place ?-cell name? ?-count n?}
    }
}

}
`;

// Deliberately broken: an unterminated brace after a complete command, so the
// analyser has something well-formed to report *about*.
const SPEC_SOURCE_BROKEN = `speclib demo 1.0 {

command demo_place {
    dialects tcl8.6
    form Default {demo_place ?-cell name?
`;

const TCL_SOURCE = `proc greet {name} {
    set message "hello $name"
    puts $message
    return $message
}

greet world
`;

let failures = 0;
const results = [];

function check(label, ok, detail) {
    results.push({ label, ok, detail });
    if (!ok) failures += 1;
    const mark = ok ? "ok  " : "FAIL";
    console.log(`${mark} ${label}${detail ? ` — ${detail}` : ""}`);
}

/** Load the no-modules glue and instantiate the module. */
async function loadModule() {
    const gluePath = join(dist, "tcl_lsp_server_wasm.js");
    const wasmPath = join(dist, "tcl_lsp_server_wasm_bg.wasm");
    const glue = await readFile(gluePath, "utf8");
    // The glue is a script that ends in `let wasm_bindgen = …`, which a plain
    // eval would scope away. Wrapping it in a function and returning the
    // binding is how a non-browser host reaches it.
    const factory = runInThisContext(`(function () { ${glue}\n return wasm_bindgen; })`, {
        filename: gluePath,
    });
    const wasmBindgen = factory();
    await wasmBindgen({ module_or_path: await readFile(wasmPath) });
    return wasmBindgen;
}

/** A minimal LSP client over the worker's string protocol. */
class Session {
    constructor(LspWorker) {
        this.nextId = 1;
        this.pending = new Map();
        this.diagnostics = new Map();
        this.logs = [];
        this.worker = new LspWorker((text) => this.receive(text));
    }

    receive(text) {
        const message = JSON.parse(text);
        if (message.method !== undefined) {
            this.handleServerMessage(message);
            return;
        }
        const resolve = this.pending.get(message.id);
        if (resolve) {
            this.pending.delete(message.id);
            resolve(message);
        }
    }

    handleServerMessage(message) {
        if (message.method === "textDocument/publishDiagnostics") {
            this.diagnostics.set(message.params.uri, message.params.diagnostics);
        } else if (message.method === "window/logMessage") {
            this.logs.push(message.params.message);
        }
        if (message.id === undefined) return;
        // A server-initiated request. Answering promptly matters: the server
        // awaits `workspace/configuration` during `initialized`, and a client
        // that never answers makes it sit on its timeout.
        let result = null;
        if (message.method === "workspace/configuration") {
            result = message.params.items.map(() => ({}));
        }
        this.post({ jsonrpc: "2.0", id: message.id, result });
    }

    post(message) {
        this.worker.send(JSON.stringify(message));
    }

    notify(method, params) {
        this.post({ jsonrpc: "2.0", method, params });
    }

    request(method, params) {
        const id = this.nextId++;
        const reply = new Promise((resolve) => this.pending.set(id, resolve));
        this.post({ jsonrpc: "2.0", id, method, params });
        return reply;
    }
}

/** Let the worker's queued futures and timers run. */
function settle(ms = 60) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

/** Poll until `probe` returns a truthy value, or give up. */
async function until(probe, { tries = 120, every = 50 } = {}) {
    for (let attempt = 0; attempt < tries; attempt += 1) {
        const value = probe();
        if (value) return value;
        await settle(every);
    }
    return null;
}

function describeHover(hover) {
    const contents = hover?.contents;
    if (contents === undefined || contents === null) return "";
    if (typeof contents === "string") return contents;
    if (typeof contents.value === "string") return contents.value;
    if (Array.isArray(contents)) {
        return contents.map((c) => (typeof c === "string" ? c : c.value ?? "")).join(" ");
    }
    return JSON.stringify(contents);
}

async function main() {
    const wasmBindgen = await loadModule();
    const session = new Session(wasmBindgen.LspWorker);

    const initialize = await session.request("initialize", {
        processId: null,
        rootUri: null,
        capabilities: {
            general: { positionEncodings: ["utf-16"] },
            workspace: { configuration: true },
            textDocument: {
                synchronization: { dynamicRegistration: false },
                hover: { contentFormat: ["markdown", "plaintext"] },
                completion: { completionItem: { snippetSupport: false } },
                semanticTokens: {
                    requests: { full: true },
                    tokenTypes: [],
                    tokenModifiers: [],
                    formats: ["relative"],
                },
                formatting: { dynamicRegistration: false },
                publishDiagnostics: {},
            },
        },
    });

    const caps = initialize.result?.capabilities;
    check("initialize returns capabilities", Boolean(caps));
    check(
        "semanticTokensProvider advertised",
        Boolean(caps?.semanticTokensProvider),
        `hoverProvider=${caps?.hoverProvider} completionProvider=${Boolean(
            caps?.completionProvider,
        )}`,
    );
    check(
        "typeHierarchyProvider shim applied",
        caps?.typeHierarchyProvider === true,
        "the initialize-response shim from tcl_lsp_server::service",
    );

    session.notify("initialized", {});
    await settle(150);

    // ---- the .tclspec document -------------------------------------------
    session.notify("textDocument/didOpen", {
        textDocument: {
            uri: SPEC_URI,
            languageId: "tclspec",
            version: 1,
            text: SPEC_SOURCE,
        },
    });
    await settle(200);

    const tokens = await session.request("textDocument/semanticTokens/full", {
        textDocument: { uri: SPEC_URI },
    });
    const data = tokens.result?.data ?? [];
    check(
        "semanticTokens/full returns tokens (.tclspec)",
        data.length > 0 && data.length % 5 === 0,
        `${data.length / 5} tokens`,
    );

    // `synopsis` inside the hover block — a pack DSL property the registry
    // knows, so a real registry-backed hover has to come back.
    const synopsisLine = SPEC_SOURCE.split("\n").findIndex((l) => l.includes("synopsis"));
    const hover = await session.request("textDocument/hover", {
        textDocument: { uri: SPEC_URI },
        position: { line: synopsisLine, character: 10 },
    });
    const hoverText = describeHover(hover.result);
    check(
        "hover on a pack property returns contents",
        hoverText.length > 0,
        JSON.stringify(hoverText.slice(0, 70)),
    );

    const completion = await session.request("textDocument/completion", {
        textDocument: { uri: SPEC_URI },
        position: { line: 3, character: 4 },
        context: { triggerKind: 1 },
    });
    const items = Array.isArray(completion.result)
        ? completion.result
        : (completion.result?.items ?? []);
    check("completion returns items (.tclspec)", items.length > 0, `${items.length} items`);

    const formatting = await session.request("textDocument/formatting", {
        textDocument: { uri: SPEC_URI },
        options: { tabSize: 4, insertSpaces: true },
    });
    check(
        "formatting answers with an edit list",
        Array.isArray(formatting.result) || formatting.result === null,
        `${Array.isArray(formatting.result) ? formatting.result.length : 0} edits`,
    );

    const clean = await until(() => session.diagnostics.get(SPEC_URI));
    check(
        "publishDiagnostics arrived for the clean document",
        Array.isArray(clean),
        `${clean?.length ?? "none"} diagnostics`,
    );

    // Pull diagnostics are deliberately not advertised (push is the only
    // channel by default), so a MethodNotFound here is the correct answer —
    // assert the server *answers* rather than hangs.
    const pull = await session.request("textDocument/diagnostic", {
        textDocument: { uri: SPEC_URI },
    });
    check(
        "textDocument/diagnostic answers (pull is unadvertised by design)",
        pull.result !== undefined || pull.error !== undefined,
        pull.error ? `error ${pull.error.code}` : "result",
    );

    // ---- break it, and expect the analyser to say so ---------------------
    session.diagnostics.delete(SPEC_URI);
    session.notify("textDocument/didChange", {
        textDocument: { uri: SPEC_URI, version: 2 },
        contentChanges: [{ text: SPEC_SOURCE_BROKEN }],
    });
    const broken = await until(() => {
        const diags = session.diagnostics.get(SPEC_URI);
        return diags && diags.length > 0 ? diags : null;
    });
    check(
        "a broken document produces at least one diagnostic",
        Array.isArray(broken) && broken.length > 0,
        broken ? `${broken.length}: ${broken[0].code ?? ""} ${broken[0].message}` : "none",
    );

    // ---- a plain .tcl document -------------------------------------------
    session.notify("textDocument/didOpen", {
        textDocument: {
            uri: TCL_URI,
            languageId: "tcl",
            version: 1,
            text: TCL_SOURCE,
        },
    });
    await settle(250);

    const tclTokens = await session.request("textDocument/semanticTokens/full", {
        textDocument: { uri: TCL_URI },
    });
    const tclData = tclTokens.result?.data ?? [];
    check(
        "semanticTokens/full returns tokens (.tcl)",
        tclData.length > 0 && tclData.length % 5 === 0,
        `${tclData.length / 5} tokens`,
    );

    const tclHover = await session.request("textDocument/hover", {
        textDocument: { uri: TCL_URI },
        position: { line: 2, character: 5 },
    });
    const tclHoverText = describeHover(tclHover.result);
    check(
        "hover on `puts` returns contents (.tcl)",
        tclHoverText.length > 0,
        JSON.stringify(tclHoverText.slice(0, 70)),
    );

    const tclDiags = await until(() => session.diagnostics.get(TCL_URI));
    check(
        "publishDiagnostics arrived for the .tcl document",
        Array.isArray(tclDiags),
        `${tclDiags?.length ?? "none"} diagnostics`,
    );

    // ---- the closed-file store -------------------------------------------
    session.worker.vfs_upsert("file:///w/lib.tcl", "proc helper {} { return 1 }\n");
    check("vfs_upsert stores a closed file", session.worker.vfs_len() === 1);
    check("vfs_delete forgets it", session.worker.vfs_delete("file:///w/lib.tcl") === true);

    const shutdown = await session.request("shutdown", null);
    check("shutdown answers", shutdown.error === undefined);

    console.log("");
    console.log(`${results.length - failures}/${results.length} checks passed`);
    if (session.logs.length > 0) {
        console.log(`server log lines: ${session.logs.length}`);
        for (const line of session.logs.slice(0, 5)) console.log(`   | ${line}`);
    }
    process.exit(failures === 0 ? 0 : 1);
}

main().catch((err) => {
    console.error("e2e harness failed:", err);
    process.exit(1);
});
