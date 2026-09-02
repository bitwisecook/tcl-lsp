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

import { readdir, readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { runInThisContext } from "node:vm";

const here = dirname(fileURLToPath(import.meta.url));
const dist = join(here, "..", "dist");
// The shipped loadables, at the one path both the server's `include_str!`
// fallback and the VSIX staging read them from.  The browser host primes these
// exact files into the mount, so the scenario below has to use the real ones.
const SHIPPED_SPECS = join(here, "..", "..", "..", "specs");

const SPEC_URI = "file:///w/demo.tclspec";
const SSLIC_URI = "file:///w/demo.sslictcl";
const TCL_URI = "file:///w/demo.tcl";

// The second session's workspace, served entirely from the store. Nothing here
// exists on any disk — the point is that a host that hands the worker bytes
// gets a real multi-file session: the folder scan indexes the siblings, the
// package database is built from the upserted index files, and `.tclspec`
// packs the host registers load as the bundled tier.
const WS_ROOT = "file:///ws";
// The third session's root: no files at all, because the packs are the subject.
const PRIMED_ROOT = "file:///primed";
const WS_MAIN_URI = "file:///ws/main.tcl";
const WS_HELPERS_URI = "file:///ws/lib/helpers.tcl";

const WS_HELPERS_SOURCE = `proc greet_helper {who} {
    return "hi $who"
}
`;

// Deliberately under `tmp/`, which the workspace *file* scan skips
// (`is_skipped_scan_dir`) and the *package database* tree walk does not. A
// command defined here can therefore only be resolved through the package
// database built from `tclIndex`, so finding it proves that path ran over the
// store rather than the file scan having swept the proc up by accident.
const WS_AUTOLOAD_INDEX_URI = "file:///ws/tmp/mypkg/tclIndex";
const WS_AUTOLOAD_IMPL_URI = "file:///ws/tmp/mypkg/impl.tcl";
const WS_AUTOLOAD_INDEX = `# Tcl autoload index file, version 2.0
set auto_index(mypkg_autoloaded) [list source [file join $dir impl.tcl]]
`;
const WS_AUTOLOAD_IMPL = `proc mypkg_autoloaded {} {
    return 42
}
`;

const WS_MAIN_SOURCE = `source lib/helpers.tcl

proc run {} {
    puts [greet_helper world]
    puts [mypkg_autoloaded]
}
`;

// A host-supplied pack, registered under the virtual mount rather than by URI.
const HOST_PACK_NAME = "vendor.tclspec";
const HOST_PACK_SOURCE = `speclib hostvendor 1.0 {

command hostvendor_place {
    dialects tcl8.6

    form Default {hostvendor_place ?-cell name?}

    hover {
        synopsis {hostvendor_place ?-cell name?}
    }
}

}
`;

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

// A minimal SslicTcl declaration document: the mandatory header, one block,
// and one unrecognised top-level word the open-world rule preserves as
// `SSLIC1101`.
const SSLIC_SOURCE = `sslictcl 1
endpoint /Common/www {
    hostname www.example.test
}
site-owner {web platform}
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

/** The client capabilities both sessions declare. */
function clientCapabilities() {
    return {
        general: { positionEncodings: ["utf-16"] },
        workspace: { configuration: true, symbol: {} },
        textDocument: {
            synchronization: { dynamicRegistration: false },
            hover: { contentFormat: ["markdown", "plaintext"] },
            completion: { completionItem: { snippetSupport: false } },
            definition: { linkSupport: false },
            semanticTokens: {
                requests: { full: true },
                tokenTypes: [],
                tokenModifiers: [],
                formats: ["relative"],
            },
            formatting: { dynamicRegistration: false },
            publishDiagnostics: {},
        },
    };
}

/** Every URI a `textDocument/definition` answer names, whatever shape it took. */
function definitionUris(result) {
    if (!result) return [];
    const list = Array.isArray(result) ? result : [result];
    return list.map((entry) => entry.uri ?? entry.targetUri).filter(Boolean);
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

/*
 * A whole workspace served from the store.
 *
 * Everything the server would normally read off disk — the sibling a `source`
 * edge points at, the files the folder scan indexes, the `tclIndex` the package
 * database is built from, the `.tclspec` the bundled tier loads — is upserted
 * before `initialize`, because `initialized` is what loads the packs and runs
 * the scan. A pass here means the whole-workspace paths read the store, not
 * just the single-document ones.
 */
async function workspaceSession(wasmBindgen) {
    const session = new Session(wasmBindgen.LspWorker);

    session.worker.vfs_upsert(WS_HELPERS_URI, WS_HELPERS_SOURCE);
    session.worker.vfs_upsert(WS_MAIN_URI, WS_MAIN_SOURCE);
    session.worker.vfs_upsert(WS_AUTOLOAD_INDEX_URI, WS_AUTOLOAD_INDEX);
    session.worker.vfs_upsert(WS_AUTOLOAD_IMPL_URI, WS_AUTOLOAD_IMPL);
    check(
        "vfs_upsert_spec_pack accepts a name inside the mount",
        session.worker.vfs_upsert_spec_pack(HOST_PACK_NAME, HOST_PACK_SOURCE) === true,
    );
    // A pack name that leaves the mount must not be able to shadow a store
    // path — here, the very file the scan is about to index.
    const escapes = [
        "/ws/main.tcl",
        "../../../ws/main.tcl",
        "nested/../../escape.tclspec",
    ];
    check(
        "vfs_upsert_spec_pack refuses a name that escapes the mount",
        escapes.every((name) => session.worker.vfs_upsert_spec_pack(name, "speclib bad 1 {}\n") === false),
    );
    check("the store holds the workspace before initialize", session.worker.vfs_len() === 5);

    await session.request("initialize", {
        processId: null,
        rootUri: WS_ROOT,
        workspaceFolders: [{ uri: WS_ROOT, name: "ws" }],
        capabilities: clientCapabilities(),
    });
    session.notify("initialized", {});

    // The scan's own completion line reports how many files it analysed, so it
    // is both the readiness signal and the assertion.
    const scanLine = await until(() =>
        session.logs.find((line) => line.includes("workspace_folders_scan")),
    );
    const scanned = Number(/files=(\d+)/.exec(scanLine ?? "")?.[1] ?? -1);
    check(
        "the workspace scan indexed the store's files",
        scanned >= 2,
        scanLine ?? "no scan log line",
    );

    // The host's pack loads on top of the shipped loadables rather than
    // replacing them — the mount is additive, a real `specs/` directory is not.
    const packLine = session.logs.find((line) => line.startsWith("SpecTcl:"));
    const packCount = Number(/(\d+) pack/.exec(packLine ?? "")?.[1] ?? -1);
    check(
        "the host's .tclspec pack loaded as the bundled tier",
        packCount >= 9,
        packLine ?? "no SpecTcl log line",
    );

    session.notify("textDocument/didOpen", {
        textDocument: {
            uri: WS_MAIN_URI,
            languageId: "tcl",
            version: 1,
            text: WS_MAIN_SOURCE,
        },
    });
    await settle(300);

    // A `source`d sibling that only the store has.
    const helperLine = WS_MAIN_SOURCE.split("\n").findIndex((l) => l.includes("greet_helper"));
    const helperDef = await session.request("textDocument/definition", {
        textDocument: { uri: WS_MAIN_URI },
        position: { line: helperLine, character: 14 },
    });
    const helperUris = definitionUris(helperDef.result);
    check(
        "go-to-definition reaches a `source`d sibling from the store",
        helperUris.some((u) => u.endsWith("/lib/helpers.tcl")),
        JSON.stringify(helperUris),
    );

    // The same proc, found through the scan's index rather than the source edge.
    const symbols = await session.request("workspace/symbol", { query: "greet_helper" });
    const symbolList = Array.isArray(symbols.result) ? symbols.result : [];
    check(
        "workspace/symbol finds a scanned closed file's proc",
        symbolList.some((s) => s.name.includes("greet_helper")),
        `${symbolList.length} symbols`,
    );

    // Only the package database can answer this one: its file lives under
    // `tmp/`, which the file scan skips.
    const autoLine = WS_MAIN_SOURCE.split("\n").findIndex((l) =>
        l.includes("mypkg_autoloaded"),
    );
    const autoDef = await session.request("textDocument/definition", {
        textDocument: { uri: WS_MAIN_URI },
        position: { line: autoLine, character: 14 },
    });
    const autoUris = definitionUris(autoDef.result);
    check(
        "go-to-definition resolves a tclIndex auto-loaded command",
        autoUris.some((u) => u.endsWith("/tmp/mypkg/impl.tcl")),
        JSON.stringify(autoUris),
    );

    const shutdown = await session.request("shutdown", null);
    check("the workspace session shuts down", shutdown.error === undefined);
    return session.logs;
}

/*
 * The browser host's real startup composition: the SHIPPED packs, mounted.
 *
 * `editors/vscode/src/extensionBrowser.ts` primes `dist/web/specs/*.tclspec`
 * into the virtual mount before `initialize`, so every bundled-tier file the
 * worker discovers already carries a shipped name.  The embedded fallback is
 * keyed by file name for exactly this reason (`bundled::load_discovered_in`):
 * key it on the *directory* alone and each pack loads twice under one name —
 * eight extra megabyte-scale parses on the worker's single thread and ~1,489
 * duplicate-command warnings per session.
 *
 * `workspaceSession` above deliberately mounts a pack under a name nothing
 * ships (`vendor.tclspec`), which is why it never saw this.  Both shapes are
 * host reality, so both are covered.
 */
async function primedShippedPacksSession(wasmBindgen) {
    const session = new Session(wasmBindgen.LspWorker);

    const names = (await readdir(SHIPPED_SPECS)).filter((name) => name.endsWith(".tclspec")).sort();
    check(
        "the shipped spec packs are on disk to prime",
        names.length === 8,
        `${names.length} packs`,
    );
    for (const name of names) {
        const text = await readFile(join(SHIPPED_SPECS, name), "utf8");
        session.worker.vfs_upsert_spec_pack(name, text);
    }

    await session.request("initialize", {
        processId: null,
        rootUri: PRIMED_ROOT,
        workspaceFolders: [{ uri: PRIMED_ROOT, name: "primed" }],
        capabilities: clientCapabilities(),
    });
    session.notify("initialized", {});

    // ~2 MB of pack text parsed on one worker thread, with no compiled-pack
    // cache in the browser — give it room rather than flaking on a slow runner.
    const packLine = await until(() => session.logs.find((line) => line.startsWith("SpecTcl:")), {
        tries: 600,
        every: 50,
    });
    const packCount = Number(/(\d+) pack/.exec(packLine ?? "")?.[1] ?? -1);
    const noticeCount = Number(/(\d+) notice/.exec(packLine ?? "")?.[1] ?? -1);
    check(
        "priming the shipped packs loads each of them exactly once",
        packCount === 8,
        packLine ?? "no SpecTcl log line",
    );
    check(
        "priming the shipped packs reports no notices",
        noticeCount === 0,
        packLine ?? "no SpecTcl log line",
    );

    const shutdown = await session.request("shutdown", null);
    check("the primed-packs session shuts down", shutdown.error === undefined);
    return session.logs;
}

async function main() {
    const wasmBindgen = await loadModule();
    const session = new Session(wasmBindgen.LspWorker);

    const initialize = await session.request("initialize", {
        processId: null,
        rootUri: null,
        capabilities: clientCapabilities(),
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

    // ---- the .sslictcl document ------------------------------------------
    // The sibling declarative dialect. Its `SSLIC1xxx` diagnostics come from
    // `tcl_sslictcl`, whose transitive crypto dependencies are the reason this
    // check exists here at all: the browser server has to keep building and
    // running with them linked in.
    session.notify("textDocument/didOpen", {
        textDocument: {
            uri: SSLIC_URI,
            languageId: "sslictcl",
            version: 1,
            text: SSLIC_SOURCE,
        },
    });
    await settle(250);

    const sslicHover = await session.request("textDocument/hover", {
        textDocument: { uri: SSLIC_URI },
        position: { line: 1, character: 2 },
    });
    const sslicHoverText = describeHover(sslicHover.result);
    check(
        "hover on `endpoint` returns the SslicTcl pack's own text (.sslictcl)",
        sslicHoverText.includes("Declare a TLS endpoint"),
        JSON.stringify(sslicHoverText.slice(0, 70)),
    );

    const sslicDiags = await until(() => {
        const diags = session.diagnostics.get(SSLIC_URI);
        return diags && diags.some((d) => String(d.code ?? "").startsWith("SSLIC")) ? diags : null;
    });
    check(
        "the SslicTcl loader's notice is published (.sslictcl)",
        Array.isArray(sslicDiags) &&
            sslicDiags.some((d) => String(d.code ?? "") === "SSLIC1101"),
        `${sslicDiags?.map((d) => d.code).join(", ") ?? "none"}`,
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

    const workspaceLogs = await workspaceSession(wasmBindgen);
    const primedLogs = await primedShippedPacksSession(wasmBindgen);

    console.log("");
    console.log(`${results.length - failures}/${results.length} checks passed`);
    if (session.logs.length > 0) {
        console.log(`server log lines: ${session.logs.length}`);
        for (const line of session.logs.slice(0, 5)) console.log(`   | ${line}`);
    }
    // The workspace session's log is where the scan and pack-load evidence
    // lives, so print it in full when something failed.
    if (failures > 0) {
        console.log(`workspace session log lines: ${workspaceLogs.length}`);
        for (const line of workspaceLogs) console.log(`   > ${line}`);
        console.log(`primed-packs session log lines: ${primedLogs.length}`);
        for (const line of primedLogs) console.log(`   > ${line}`);
    }
    process.exit(failures === 0 ? 0 : 1);
}

main().catch((err) => {
    console.error("e2e harness failed:", err);
    process.exit(1);
});
