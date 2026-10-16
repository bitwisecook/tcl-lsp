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

/**
 * The universal VSIX's last rung: run `tcl-lsp-server-wasi.wasm` under VS
 * Code's WASM WASI host instead of a native `tcl-lsp-server`.
 *
 * Reached only when `resolveRustServer` found no native binary for this
 * platform and none in a dev checkout — i.e. an architecture none of the seven
 * cross-compiled triples covers.  `extension.ts` imports this module *lazily*
 * for exactly that reason: every platform that has a binary evaluates none of
 * it, and pays nothing for `@vscode/wasm-wasi` / `@vscode/wasm-wasi-lsp`.
 *
 * The host contract the module expects — preopen the workspace, drain stdout,
 * a monotonic clock, and the `shutdown`/`exit` codes — is Part 4 of
 * `docs/design/rust/lsp-runtime-and-transports.md`.  This module's job is to
 * satisfy it with the WASM WASI Core extension's API:
 *
 * - **Preopens.** Each process creates one actual RootFileSystem from explicit
 *   `vscodeFileSystem` mounts. Provider round trips authenticate the selected
 *   roots, and the same filesystem is passed to the process. Unique per-session
 *   mount paths are unrelated to folder display names. Stable client converter
 *   callbacks reborrow only the current process's retained typed URI roster.
 * - **Spec packs.** The native rung finds its `.tclspec` loadables in a
 *   `specs/` directory beside the executable.  A guest has no executable
 *   path, so the staged directory is mounted read-only and named explicitly
 *   through `TCL_LSP_SPEC_PACK_DIR`, which `tcl_spectcl::discovery::bundled_dir`
 *   honours ahead of the beside-the-executable probe.
 * - **Draining.** `startServer` pumps stdout; stderr (where the server writes
 *   its diagnostics — stdout carries the protocol) is drained into the output
 *   channel here, so a chatty session cannot block the single-threaded guest
 *   inside `write`.
 *
 * The WASM WASI Core extension is a **soft** dependency: see
 * `WASM_WASI_CORE_EXTENSION_ID` in `./serverResolution` for why it can never be
 * `extensionDependencies`.
 */

import * as vscode from "vscode";
import { Wasm, type MountPointDescriptor, type WasmProcess } from "@vscode/wasm-wasi/v1";
import { createStdioOptions, startServer } from "@vscode/wasm-wasi-lsp";
import type { MessageTransports } from "vscode-languageclient";
import type { ServerOptions } from "vscode-languageclient/node";
import {
  WASI_SPECS_RELATIVE_PATH,
  WASM_WASI_CORE_EXTENSION_ID,
  wasiRuntimeAction,
  allocatedWasiUriMapping,
} from "./serverResolution";

/**
 * `globalState` key holding the user's "don't ask again" answer to the
 * runtime-install prompt.
 */
export const WASI_PROMPT_DISMISSED_KEY = "tclLsp.wasiRuntime.installPromptDismissed";

/** Where the spec-pack mount lands inside the guest. */
const GUEST_SPEC_PACK_DIR = "/tcl-lsp/specs";

/** `tcl_spectcl::discovery::BUNDLED_DIR_ENV` — keep the two spellings in step. */
const SPEC_PACK_DIR_ENV = "TCL_LSP_SPEC_PACK_DIR";

/** The `LanguageClientOptions.uriConverters` shape, as the client declares it. */
interface UriConverters {
  code2Protocol: (value: vscode.Uri) => string;
  protocol2Code: (value: string) => vscode.Uri;
}

/** What `startWasiServer` hands back for `activate` to build a client from. */
export interface WasiServerSetup {
  readonly serverOptions: ServerOptions;
  readonly uriConverters: UriConverters | undefined;
}

/**
 * Make sure the WASM WASI host extension is installed, prompting once if it is
 * not, and return whether the WASI rung can proceed.
 */
async function ensureWasiRuntime(
  context: vscode.ExtensionContext,
  channel: vscode.OutputChannel,
): Promise<boolean> {
  const installed = () => vscode.extensions.getExtension(WASM_WASI_CORE_EXTENSION_ID) !== undefined;
  const dismissed = context.globalState.get<boolean>(WASI_PROMPT_DISMISSED_KEY, false);
  const action = wasiRuntimeAction(installed(), dismissed);
  if (action === "start") {
    return true;
  }
  if (action === "declined") {
    channel.appendLine(
      `The WebAssembly language server needs '${WASM_WASI_CORE_EXTENSION_ID}', which is not ` +
        "installed. The install prompt was dismissed previously; run the command " +
        "'Extensions: Install Extension' to add it, then reload the window.",
    );
    return false;
  }

  const install = "Install";
  const notNow = "Not now";
  const never = "Don't ask again";
  const answer = await vscode.window.showInformationMessage(
    "Tcl LSP: no native language server binary ships for this platform, but the extension " +
      "bundles a WebAssembly one. Running it needs the 'WASM WASI Core' extension " +
      `(${WASM_WASI_CORE_EXTENSION_ID}).`,
    install,
    notNow,
    never,
  );
  if (answer === never) {
    await context.globalState.update(WASI_PROMPT_DISMISSED_KEY, true);
    channel.appendLine(
      `Install prompt for '${WASM_WASI_CORE_EXTENSION_ID}' dismissed for good; no language ` +
        "server will start on this platform until it is installed by hand.",
    );
    return false;
  }
  if (answer !== install) {
    // "Not now", or the notification dismissed.  Nothing is persisted, so the
    // prompt returns on the next window.
    channel.appendLine(
      `'${WASM_WASI_CORE_EXTENSION_ID}' not installed; the WebAssembly language server did not start.`,
    );
    return false;
  }

  try {
    await vscode.commands.executeCommand(
      "workbench.extensions.installExtension",
      WASM_WASI_CORE_EXTENSION_ID,
    );
  } catch (err) {
    vscode.window.showErrorMessage(
      `Tcl LSP: could not install ${WASM_WASI_CORE_EXTENSION_ID}: ${String(err)}`,
    );
    return false;
  }
  if (!installed()) {
    // The install succeeded but the extension host has not picked the new
    // extension up yet — activating it in this session would fail, so say so
    // rather than starting a client that can never connect.
    vscode.window.showInformationMessage(
      `Tcl LSP: ${WASM_WASI_CORE_EXTENSION_ID} was installed. Reload the window to start the ` +
        "WebAssembly language server.",
    );
    return false;
  }
  return true;
}

/**
 * Prepare the WASI rung.  Returns `undefined` when the host runtime is absent
 * and the user declined to install it — the caller has already been told why,
 * so it should simply stop activating the language client.
 */
export async function startWasiServer(
  context: vscode.ExtensionContext,
  modulePath: string,
  channel: vscode.OutputChannel,
): Promise<WasiServerSetup | undefined> {
  if (!(await ensureWasiRuntime(context, channel))) {
    return undefined;
  }

  let wasm: Wasm;
  try {
    wasm = await Wasm.load();
  } catch (err) {
    vscode.window.showErrorMessage(
      `Tcl LSP: the WebAssembly language server could not start — ${WASM_WASI_CORE_EXTENSION_ID} ` +
        `did not activate: ${String(err)}`,
    );
    return undefined;
  }
  // `versions.extension` only: the API-version field is spelled `apt` in
  // @vscode/wasm-wasi@1.0.2's `.d.ts` and `api` in the object the extension
  // actually returns, so logging the typed name prints "undefined".
  channel.appendLine(`WASM WASI Core ${wasm.versions.extension} will host ${modulePath}`);

  // Compiled once and reused: `tclLsp.restartServer` stops the client and
  // starts it again, which re-invokes the ServerOptions factory below, and
  // re-compiling ~19 MiB of WebAssembly on every restart is pure waste.  A
  // failed compile is dropped rather than cached, so a restart can retry it.
  let compiled: Promise<WebAssembly.Module> | undefined;
  const compileModule = async (): Promise<WebAssembly.Module> => {
    compiled ??= wasm.compile(vscode.Uri.file(modulePath));
    try {
      return await compiled;
    } catch (err) {
      compiled = undefined;
      throw err;
    }
  };

  // The converter functions are stable, but their receipt is installed only
  // for the actual process/filesystem generation that has finished admission.
  let generation = 0;
  let workspaceEpoch = 0;
  let active: ReturnType<typeof allocatedWasiUriMapping> | undefined;
  context.subscriptions.push(
    vscode.workspace.onDidChangeWorkspaceFolders(() => {
      workspaceEpoch += 1;
      active = undefined;
    }),
  );
  const uriConverters: UriConverters = {
    code2Protocol(value) {
      if (!active) throw new Error("WASI mount session is unavailable");
      return active.toGuest(value);
    },
    protocol2Code(value) {
      if (!active) throw new Error("WASI mount session is unavailable");
      return active.toEditor(value) as vscode.Uri;
    },
  };

  const serverOptions: ServerOptions = async (): Promise<MessageTransports> => {
    active = undefined;
    const selectedGeneration = ++generation;
    const selectedEpoch = workspaceEpoch;
    const folders = [...(vscode.workspace.workspaceFolders ?? [])];
    const allocations = folders.map((folder, index) => ({
      editor: folder.uri,
      guestPath: `/tcl-lsp/workspaces/${selectedGeneration}/${index}`,
    }));
    let live = true;
    const current = (): boolean =>
      live &&
      generation === selectedGeneration &&
      workspaceEpoch === selectedEpoch &&
      folders.length === (vscode.workspace.workspaceFolders ?? []).length &&
      folders.every((folder, index) => vscode.workspace.workspaceFolders?.[index] === folder);
    const revoke = (): void => {
      live = false;
      if (generation === selectedGeneration) active = undefined;
    };
    let process: WasmProcess | undefined;
    try {
      const module = await compileModule();
      if (!current()) throw new Error("Workspace changed while preparing WASI mounts");
      const mountPoints: MountPointDescriptor[] = [
        ...allocations.map((mount): MountPointDescriptor => ({
          kind: "vscodeFileSystem",
          uri: mount.editor,
          mountPoint: mount.guestPath,
        })),
        {
          kind: "extensionLocation",
          extension: context,
          path: WASI_SPECS_RELATIVE_PATH,
          mountPoint: GUEST_SPEC_PACK_DIR,
        },
      ];
      const fileSystem = await wasm.createRootFileSystem(mountPoints);
      if (!current()) throw new Error("Workspace changed while allocating WASI mounts");
      for (const mount of allocations) {
        const path = await fileSystem.toWasm(mount.editor);
        if (!current() || path !== mount.guestPath) throw new Error("WASI provider mount mismatch");
        const uri = await fileSystem.toVSCode(mount.guestPath);
        if (!current() || !uri || uri.toString() !== mount.editor.toString()) {
          throw new Error("WASI provider reverse mount mismatch");
        }
      }
      const mapping = allocatedWasiUriMapping(
        allocations,
        (value) => vscode.Uri.parse(value, true),
        current,
      );
      process = await wasm.createProcess("tcl-lsp-server-wasi", module, {
        stdio: createStdioOptions(),
        rootFileSystem: fileSystem,
        env: { [SPEC_PACK_DIR_ENV]: GUEST_SPEC_PACK_DIR },
      });
      if (!current()) throw new Error("Workspace changed while creating WASI process");
      const actualProcess = process;
      drainStderr(actualProcess, channel);
      const trackedProcess: WasmProcess = {
        stdin: actualProcess.stdin,
        stdout: actualProcess.stdout,
        stderr: actualProcess.stderr,
        run: () => actualProcess.run().finally(revoke),
        terminate: () => {
          revoke();
          return actualProcess.terminate();
        },
      };
      const transports = await startServer(trackedProcess);
      if (!current()) throw new Error("WASI process ended before mount publication");
      const write = transports.writer.write.bind(transports.writer);
      transports.writer.write = (message) => {
        try {
          if ("method" in message && (message.method === "shutdown" || message.method === "exit"))
            revoke();
          return write(message).catch((error) => {
            revoke();
            throw error;
          });
        } catch (error) {
          revoke();
          throw error;
        }
      };
      transports.reader.onError(revoke);
      transports.reader.onClose(revoke);
      transports.writer.onError(revoke);
      transports.writer.onClose(revoke);
      const disposeReader = transports.reader.dispose.bind(transports.reader);
      transports.reader.dispose = () => {
        revoke();
        disposeReader();
      };
      const disposeWriter = transports.writer.dispose.bind(transports.writer);
      transports.writer.dispose = () => {
        revoke();
        disposeWriter();
      };
      active = mapping;
      return transports;
    } catch (error) {
      revoke();
      if (process) await process.terminate();
      throw error;
    }
  };

  return { serverOptions, uriConverters };
}

/**
 * Contract point 2, the stderr half.  The guest is single-threaded and writes
 * its diagnostics to stderr (stdout carries the protocol), so an undrained
 * stream would eventually block it inside `write`.  Buffered to line
 * boundaries because a chunk is not a line.
 */
function drainStderr(process: WasmProcess, channel: vscode.OutputChannel): void {
  const decoder = new TextDecoder("utf-8");
  let pending = "";
  process.stderr?.onData((data) => {
    pending += decoder.decode(data, { stream: true });
    let newline = pending.indexOf("\n");
    while (newline >= 0) {
      const line = pending.slice(0, newline).trimEnd();
      pending = pending.slice(newline + 1);
      if (line) {
        channel.appendLine(`[wasi] ${line}`);
      }
      newline = pending.indexOf("\n");
    }
  });
}
