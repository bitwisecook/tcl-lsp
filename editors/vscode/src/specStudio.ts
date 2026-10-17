// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

import * as fs from "fs";
import { randomBytes } from "crypto";
import * as vscode from "vscode";
import { State, type LanguageClient } from "vscode-languageclient/node";

type Surface = "dsl" | "sample" | "rust" | "stub";

interface StudioMessage {
  type: string;
  surface?: Surface;
  text?: string;
  dialect?: string;
  sourceRevision?: number;
  webGeneration?: number;
  expectedSourceRevision?: number;
  basedOnDslRevision?: number;
  accepted?: boolean;
  retryable?: boolean;
  publicationPending?: boolean;
  publicationAttempt?: number;
  bootstrap?: boolean;
}

const filenames: Record<Surface, string> = {
  dsl: "active.tclspec",
  sample: "test.tcl",
  rust: "generated.rs",
  stub: "generated-stub.tcl",
};

let current: SpecStudioSession | undefined;

const studioIgnore = "*\n";

interface StudioSourceLease {
  issuer: number[];
  token: number;
}

function parseStudioSourceLease(value: unknown): StudioSourceLease | undefined {
  if (!value || typeof value !== "object") return undefined;
  const result = value as { accepted?: unknown; lease?: unknown };
  if (result.accepted !== true || !result.lease || typeof result.lease !== "object")
    return undefined;
  const lease = result.lease as { issuer?: unknown; token?: unknown };
  if (
    !Array.isArray(lease.issuer) ||
    lease.issuer.length !== 16 ||
    !lease.issuer.every((byte) => Number.isInteger(byte) && byte >= 0 && byte <= 255) ||
    !Number.isInteger(lease.token) ||
    (lease.token as number) <= 0 ||
    (lease.token as number) > 0xffff_ffff
  )
    return undefined;
  return { issuer: lease.issuer as number[], token: lease.token as number };
}

/** Ensure generated projections stay ignored without claiming ownership of old sessions. */
export async function prepareSpecStudioStorage(): Promise<void> {
  for (const folder of vscode.workspace.workspaceFolders ?? []) {
    const root = vscode.Uri.joinPath(folder.uri, ".tcl-lsp", ".spec-studio");
    await vscode.workspace.fs.createDirectory(root);
    await vscode.workspace.fs.writeFile(
      vscode.Uri.joinPath(root, ".gitignore"),
      Buffer.from(studioIgnore, "utf8"),
    );
  }
}

function injectNativeBridge(html: string, nativeModule: string): string {
  const source = JSON.stringify(nativeModule).replace(/</g, "\\u003c");
  const bridge = `<script>const vscode=acquireVsCodeApi();window.__tclSpecStudioHost={postMessage:(message)=>vscode.postMessage(message)};window.__tclSpecStudioNativeModuleUrl=URL.createObjectURL(new Blob([${source}],{type:'text/javascript'}));</script>`;
  return html
    .replace("script-src 'self'", "script-src 'self' blob:")
    .replace("<head>", `<head>${bridge}`);
}

class SpecStudioSession implements vscode.Disposable {
  private readonly panel: vscode.WebviewPanel;
  private readonly root: vscode.Uri;
  private readonly sessionDir: vscode.Uri;
  private readonly documents = new Map<Surface, vscode.Uri>();
  private readonly sourceRevisions = new Map<Surface, number>();
  private readonly writeGenerations = new Map<Surface, number>();
  private readonly pageWriteTexts = new Map<Surface, string>();
  private readonly lastPageWriteGenerations = new Map<Surface, number>();
  private readonly surfaceWriteQueues = new Map<Surface, Promise<void>>();
  private sourceLease: StudioSourceLease | undefined;
  private sourceLeaseRevision = 0;
  private dslPublicationAttempt = 0;
  private sourceLeaseGeneration = 0;
  private sourceLeaseQueue: Promise<void> = Promise.resolve();
  private sampleDialectGeneration = 0;
  private sampleDialectQueue: Promise<void> = Promise.resolve();
  private pendingDslGeneration: number | undefined;
  private readonly subscriptions: vscode.Disposable[] = [];
  private readonly saveTimers = new Map<string, ReturnType<typeof setTimeout>>();
  private readonly dslPublicationInFlight = new Set<number>();
  private disposed = false;

  constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly client: LanguageClient,
    workspaceRoot: vscode.Uri,
  ) {
    this.root = vscode.Uri.joinPath(context.extensionUri, "spec-studio");
    // Each panel owns a distinct document URI. A disposal-time dialect clear
    // or directory removal from an older panel cannot affect a replacement.
    this.sessionDir = vscode.Uri.joinPath(
      workspaceRoot,
      ".tcl-lsp",
      ".spec-studio",
      "vscode",
      `session-${randomBytes(16).toString("hex")}`,
    );
    this.panel = vscode.window.createWebviewPanel(
      "tclSpecStudio",
      "Tcl Spec Studio",
      { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
      {
        enableScripts: true,
        retainContextWhenHidden: true,
        localResourceRoots: [this.root],
      },
    );

    for (const surface of Object.keys(filenames) as Surface[]) {
      this.documents.set(surface, vscode.Uri.joinPath(this.sessionDir, filenames[surface]));
      this.sourceRevisions.set(surface, 0);
      this.writeGenerations.set(surface, 0);
      this.lastPageWriteGenerations.set(surface, 0);
    }
    this.subscriptions.push(
      this.panel.webview.onDidReceiveMessage((message: StudioMessage) => this.receive(message)),
      vscode.workspace.onDidChangeTextDocument((event) => this.documentChanged(event.document)),
      vscode.workspace.onDidSaveTextDocument((document) => this.documentSaved(document)),
      this.client.onDidChangeState((event) => {
        if (this.disposed) return;
        if (event.newState === State.Running) void this.reacquirePackLease();
        else this.markDslPublicationPending();
      }),
      this.panel.onDidDispose(() => this.dispose()),
    );
  }

  async start(): Promise<void> {
    const index = vscode.Uri.joinPath(this.root, "index.html");
    let html: string;
    try {
      html = (await vscode.workspace.fs.readFile(index)).toString();
      const nativeModule = (
        await vscode.workspace.fs.readFile(
          vscode.Uri.joinPath(this.root, "assets", "native-editor-host.js"),
        )
      ).toString();
      html = injectNativeBridge(html, nativeModule);
    } catch {
      void vscode.window.showErrorMessage(
        "Tcl Spec Studio assets are missing from this extension build. Rebuild with `make spec-studio-wasm`.",
      );
      this.dispose();
      return;
    }
    await vscode.workspace.fs.createDirectory(this.sessionDir);
    await this.reacquirePackLease();
    if (this.disposed) return;
    this.panel.webview.html = html;
  }

  private enqueueLeaseOperation(operation: () => Promise<void>): Promise<void> {
    const next = this.sourceLeaseQueue.catch(() => undefined).then(operation);
    this.sourceLeaseQueue = next.catch(() => undefined);
    return next;
  }

  private markDslPublicationPending(
    text = this.readSurfaceText("dsl"),
    revision = this.sourceRevisions.get("dsl")!,
  ): number {
    const attempt = ++this.dslPublicationAttempt;
    this.pendingDslGeneration = -1;
    if (text !== undefined) {
      this.sendSurfaceUpdate("dsl", text, revision, undefined, undefined, undefined, true, attempt);
    }
    return attempt;
  }

  private async reacquirePackLease(): Promise<void> {
    const generation = ++this.sourceLeaseGeneration;
    const currentRevision = this.sourceRevisions.get("dsl")!;
    const currentText = this.readSurfaceText("dsl");
    const publicationAttempt = this.markDslPublicationPending(currentText, currentRevision);
    await this.enqueueLeaseOperation(async () => {
      if (this.disposed || generation !== this.sourceLeaseGeneration) return;
      const uri = this.documents.get("dsl")!;
      try {
        const lease = await this.acquirePackLeaseNow(uri);
        if (!lease) return;
        if (this.disposed || generation !== this.sourceLeaseGeneration) {
          await this.retireLease(lease, uri);
          return;
        }
        this.sourceLease = lease;
        this.sourceLeaseRevision = 0;
        const published = await this.publishSavedPackSource(uri, lease, generation, currentText);
        const liveText = this.readSurfaceText("dsl");
        if (
          published &&
          currentText !== undefined &&
          liveText === currentText &&
          this.sourceRevisions.get("dsl") === currentRevision &&
          this.pendingDslGeneration === -1 &&
          publicationAttempt === this.dslPublicationAttempt &&
          generation === this.sourceLeaseGeneration &&
          !this.disposed
        ) {
          this.pendingDslGeneration = undefined;
          this.sendSurfaceUpdate(
            "dsl",
            currentText,
            currentRevision,
            undefined,
            true,
            false,
            false,
            publicationAttempt,
          );
        }
      } catch (error) {
        console.error("[spec-studio] could not acquire its active source lease", error);
      }
    });
  }

  private async acquirePackLeaseNow(uri: vscode.Uri): Promise<StudioSourceLease | undefined> {
    const response = await this.client.sendRequest<unknown>("workspace/executeCommand", {
      command: "tcl-lsp.studioSource",
      arguments: [{ operation: "acquire", uri: uri.toString() }],
    });
    return parseStudioSourceLease(response);
  }

  private async publishSavedPackSource(
    uri: vscode.Uri,
    lease: StudioSourceLease,
    generation: number,
    expectedText?: string,
  ): Promise<boolean> {
    try {
      const bytes = await vscode.workspace.fs.readFile(uri);
      const source = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
      if (expectedText !== undefined && source !== expectedText) return false;
      if (this.disposed || generation !== this.sourceLeaseGeneration || this.sourceLease !== lease)
        return false;
      const revision = this.sourceLeaseRevision + 1;
      if (revision > 0xffff_ffff) return false;
      const response = await this.client.sendRequest<unknown>("workspace/executeCommand", {
        command: "tcl-lsp.studioSource",
        arguments: [
          {
            operation: "publish",
            uri: uri.toString(),
            lease,
            revision,
            source,
          },
        ],
      });
      const accepted =
        !!response &&
        typeof response === "object" &&
        (response as { accepted?: unknown }).accepted === true;
      if (
        !accepted ||
        this.disposed ||
        generation !== this.sourceLeaseGeneration ||
        this.sourceLease !== lease
      ) {
        return false;
      }
      this.sourceLeaseRevision = revision;
      return true;
    } catch (error) {
      console.error("[spec-studio] could not publish its saved active source", error);
      return false;
    }
  }

  private async publishPackSource(expectedText?: string): Promise<boolean> {
    let published = false;
    await this.enqueueLeaseOperation(async () => {
      let lease = this.sourceLease;
      const generation = this.sourceLeaseGeneration;
      if (this.disposed) return;
      const uri = this.documents.get("dsl")!;
      if (!lease) {
        try {
          lease = await this.acquirePackLeaseNow(uri);
        } catch (error) {
          console.error("[spec-studio] could not reacquire its active source lease", error);
          return;
        }
        if (!lease) return;
        if (this.disposed || generation !== this.sourceLeaseGeneration) {
          await this.retireLease(lease, uri);
          return;
        }
        this.sourceLease = lease;
        this.sourceLeaseRevision = 0;
      }
      published = await this.publishSavedPackSource(uri, lease, generation, expectedText);
      if (!published && this.sourceLease === lease && !this.disposed) {
        await this.retireLease(lease, uri);
      }
    });
    return published;
  }

  private async retireLease(lease: StudioSourceLease, uri: vscode.Uri): Promise<void> {
    try {
      const response = await this.client.sendRequest<unknown>("workspace/executeCommand", {
        command: "tcl-lsp.studioSource",
        arguments: [{ operation: "retire", uri: uri.toString(), lease }],
      });
      if (
        response &&
        typeof response === "object" &&
        (response as { accepted?: unknown }).accepted === true &&
        this.sourceLease === lease
      ) {
        this.sourceLease = undefined;
      }
    } catch (error) {
      console.error("[spec-studio] could not retire its active source lease", error);
    }
  }

  reveal(): void {
    this.panel.reveal(vscode.ViewColumn.Beside, true);
  }

  private async receive(message: StudioMessage): Promise<void> {
    if (message.type === "openSurface" && message.surface) {
      await this.openSurface(message.surface);
      return;
    }
    if (message.type === "surfaceUpdate" && message.surface && typeof message.text === "string") {
      if (
        (message.surface === "dsl" || message.surface === "sample") &&
        (typeof message.webGeneration !== "number" ||
          !Number.isSafeInteger(message.webGeneration) ||
          message.webGeneration < 0)
      )
        return;
      if (
        message.expectedSourceRevision !== undefined &&
        (typeof message.expectedSourceRevision !== "number" ||
          !Number.isSafeInteger(message.expectedSourceRevision) ||
          message.expectedSourceRevision < 0)
      )
        return;
      if (
        (message.surface === "rust" || message.surface === "stub") &&
        (typeof message.basedOnDslRevision !== "number" ||
          !Number.isSafeInteger(message.basedOnDslRevision) ||
          message.basedOnDslRevision < 0)
      )
        return;
      const webGeneration = Number.isSafeInteger(message.webGeneration)
        ? (message.webGeneration as number)
        : undefined;
      const expected = Number.isSafeInteger(message.expectedSourceRevision)
        ? (message.expectedSourceRevision as number)
        : null;
      const basedOnDsl = Number.isSafeInteger(message.basedOnDslRevision)
        ? (message.basedOnDslRevision as number)
        : null;
      await this.enqueueSurfaceWrite(
        message.surface,
        message.text,
        expected,
        webGeneration,
        basedOnDsl,
        message.bootstrap === true,
      );
      return;
    }
    if (message.type === "dialectUpdate") {
      await this.applySampleDialect(message.dialect);
    }
  }

  /**
   * Pin the sample surface to the studio's selected dialect.
   *
   * The sample is always materialised as `test.tcl`, so without this the
   * server resolves it as generic Tcl however the studio's selector is set,
   * and a pack whose commands only exist in another dialect shows no
   * highlighting, completion or hover in the very buffer the studio exists to
   * give feedback on. The per-document override is the seam that does not
   * disturb any other open buffer, unlike the two session-global dialect
   * commands.
   */
  private applySampleDialect(dialect: string | undefined, cleanup = false): Promise<void> {
    const uri = this.documents.get("sample")!;
    const generation = ++this.sampleDialectGeneration;
    const next = this.sampleDialectQueue
      .catch(() => undefined)
      .then(async () => {
        if (generation !== this.sampleDialectGeneration && !cleanup) return;
        if (this.disposed && !cleanup) return;
        try {
          await this.client.sendRequest("workspace/executeCommand", {
            command: "tcl-lsp.setDocumentDialectOverride",
            arguments: [uri.toString(), dialect ?? null],
          });
        } catch (error) {
          console.error("[spec-studio] could not set the sample document dialect", error);
        }
      });
    this.sampleDialectQueue = next;
    return next;
  }

  private async writeSurface(
    surface: Surface,
    text: string,
    expectedSourceRevision: number | null,
    webGeneration: number | undefined,
    basedOnDslRevision: number | null,
    bootstrap: boolean,
  ): Promise<void> {
    if (this.disposed) return;
    const uri = this.documents.get(surface)!;
    const bytes = new TextEncoder().encode(text);
    if (new TextDecoder("utf-8", { fatal: true }).decode(bytes) !== text) {
      if (surface === "dsl" || surface === "sample") {
        const current = this.readSurfaceText(surface);
        if (current !== undefined) {
          this.sendSurfaceUpdate(
            surface,
            current,
            this.sourceRevisions.get(surface)!,
            webGeneration,
            false,
            false,
          );
        }
      }
      return;
    }
    if (bootstrap && (surface === "dsl" || surface === "sample")) {
      if (this.hostSourceExists(surface)) {
        const current = this.readSurfaceText(surface);
        if (current !== undefined)
          await this.publishBootstrapSource(surface, current, webGeneration);
        return;
      }
      if ((this.sourceRevisions.get(surface) ?? 0) !== 0) return;
    }
    if (surface === "dsl" || surface === "sample") {
      const revision = this.sourceRevisions.get(surface)!;
      if ((expectedSourceRevision ?? 0) !== revision) {
        const current = this.readSurfaceText(surface);
        if (current !== undefined) {
          const retryable = this.lastPageWriteGenerations.get(surface) !== 0;
          this.sendSurfaceUpdate(surface, current, revision, webGeneration, false, retryable);
        }
        return;
      }
    } else if (
      this.pendingDslGeneration !== undefined ||
      basedOnDslRevision !== this.sourceRevisions.get("dsl")
    ) {
      return;
    }
    const sourceBefore =
      surface === "dsl" || surface === "sample" ? this.readSurfaceText(surface) : undefined;
    const revisionBefore =
      surface === "dsl" || surface === "sample" ? this.sourceRevisions.get(surface)! : undefined;
    const generation = (this.writeGenerations.get(surface) ?? 0) + 1;
    this.writeGenerations.set(surface, generation);
    if (surface === "dsl") {
      this.pendingDslGeneration = webGeneration ?? -1;
      for (const output of ["rust", "stub"] as const) {
        this.writeGenerations.set(output, (this.writeGenerations.get(output) ?? 0) + 1);
      }
    }
    try {
      await vscode.workspace.fs.createDirectory(this.sessionDir);
    } catch {
      this.refuseSurfaceWrite(surface, webGeneration, sourceBefore, revisionBefore);
      return;
    }
    if (
      this.disposed ||
      this.writeGenerations.get(surface) !== generation ||
      this.sourceRevisions.get(surface) !== (expectedSourceRevision ?? 0)
    )
      return;
    if (
      bootstrap &&
      (surface === "dsl" || surface === "sample") &&
      this.hostSourceExists(surface)
    ) {
      const current = this.readSurfaceText(surface);
      if (current !== undefined) await this.publishBootstrapSource(surface, current, webGeneration);
      return;
    }
    const open = vscode.workspace.textDocuments.find(
      (document) => document.uri.toString() === uri.toString(),
    );
    if (open && open.getText() !== text) {
      this.pageWriteTexts.set(surface, text);
      const edit = new vscode.WorkspaceEdit();
      edit.replace(
        uri,
        new vscode.Range(open.positionAt(0), open.positionAt(open.getText().length)),
        text,
      );
      let applied = false;
      try {
        applied = await vscode.workspace.applyEdit(edit);
      } catch {
        this.refuseSurfaceWrite(surface, webGeneration, sourceBefore, revisionBefore);
        return;
      }
      if (
        !applied ||
        this.writeGenerations.get(surface) !== generation ||
        open.isClosed ||
        open.getText() !== text
      ) {
        this.refuseSurfaceWrite(surface, webGeneration, sourceBefore, revisionBefore);
        return;
      }
      try {
        const saved = await open.save();
        if (!saved) {
          this.refuseSurfaceWrite(surface, webGeneration, sourceBefore, revisionBefore);
          return;
        }
      } catch {
        this.refuseSurfaceWrite(surface, webGeneration, sourceBefore, revisionBefore);
        return;
      }
      this.pageWriteTexts.delete(surface);
    } else if (!open) {
      if (this.disposed || this.writeGenerations.get(surface) !== generation) return;
      try {
        await vscode.workspace.fs.writeFile(uri, Buffer.from(bytes));
      } catch {
        this.refuseSurfaceWrite(surface, webGeneration, sourceBefore, revisionBefore);
        return;
      }
    }
    if (this.disposed || this.writeGenerations.get(surface) !== generation) return;
    if (surface === "dsl" || surface === "sample") {
      const revision = this.sourceRevisions.get(surface)! + 1;
      this.sourceRevisions.set(surface, revision);
      this.lastPageWriteGenerations.set(surface, webGeneration ?? 0);
      const publicationAttempt = surface === "dsl" ? ++this.dslPublicationAttempt : undefined;
      if (surface === "dsl" && !(await this.publishPackSource(text))) {
        this.refuseSurfaceWrite(surface, webGeneration, sourceBefore, revisionBefore);
        return;
      }
      if (
        this.disposed ||
        this.writeGenerations.get(surface) !== generation ||
        this.sourceRevisions.get(surface) !== revision
      )
        return;
      if (surface === "dsl" && this.pendingDslGeneration === (webGeneration ?? -1)) {
        this.pendingDslGeneration = undefined;
      }
      this.sendSurfaceUpdate(
        surface,
        text,
        revision,
        webGeneration,
        true,
        false,
        surface === "dsl" ? false : undefined,
        publicationAttempt,
      );
    }
  }

  private enqueueSurfaceWrite(
    surface: Surface,
    text: string,
    expectedSourceRevision: number | null,
    webGeneration: number | undefined,
    basedOnDslRevision: number | null,
    bootstrap = false,
  ): Promise<void> {
    const previous = this.surfaceWriteQueues.get(surface) ?? Promise.resolve();
    const next = previous
      .catch(() => undefined)
      .then(() =>
        this.writeSurface(
          surface,
          text,
          expectedSourceRevision,
          webGeneration,
          basedOnDslRevision,
          bootstrap,
        ),
      );
    this.surfaceWriteQueues.set(surface, next);
    return next.finally(() => {
      if (this.surfaceWriteQueues.get(surface) === next) this.surfaceWriteQueues.delete(surface);
    });
  }

  private hostSourceExists(surface: Surface): boolean {
    const uri = this.documents.get(surface)!;
    return (
      fs.existsSync(uri.fsPath) ||
      vscode.workspace.textDocuments.some((document) => document.uri.toString() === uri.toString())
    );
  }

  private async publishBootstrapSource(
    surface: "dsl" | "sample",
    text: string,
    webGeneration: number | undefined,
  ): Promise<void> {
    const source = this.readSurfaceText(surface);
    if (source === undefined || source !== text) return;
    const sourceRevision = this.sourceRevisions.get(surface)!;
    const sourceWriteGeneration = this.writeGenerations.get(surface) ?? 0;
    const sourceAttempt = surface === "dsl" ? this.dslPublicationAttempt : undefined;
    const sourcePendingGeneration = surface === "dsl" ? this.pendingDslGeneration : undefined;
    const leaseGeneration = surface === "dsl" ? this.sourceLeaseGeneration : undefined;
    const lease = surface === "dsl" ? this.sourceLease : undefined;
    if (surface === "dsl" && !(await this.publishPackSource(source))) return;
    if (
      this.disposed ||
      this.readSurfaceText(surface) !== source ||
      this.sourceRevisions.get(surface) !== sourceRevision ||
      (this.writeGenerations.get(surface) ?? 0) !== sourceWriteGeneration ||
      (surface === "dsl" &&
        (this.dslPublicationAttempt !== sourceAttempt ||
          this.pendingDslGeneration !== sourcePendingGeneration ||
          this.sourceLeaseGeneration !== leaseGeneration ||
          (lease !== undefined && this.sourceLease !== lease) ||
          this.sourceLease === undefined))
    )
      return;
    const revision = sourceRevision + 1;
    this.sourceRevisions.set(surface, revision);
    this.lastPageWriteGenerations.set(surface, 0);
    const publicationAttempt = surface === "dsl" ? ++this.dslPublicationAttempt : undefined;
    if (surface === "dsl") {
      this.pendingDslGeneration = undefined;
      for (const output of ["rust", "stub"] as const) {
        this.writeGenerations.set(output, (this.writeGenerations.get(output) ?? 0) + 1);
      }
    }
    this.sendSurfaceUpdate(
      surface,
      source,
      revision,
      webGeneration,
      false,
      false,
      surface === "dsl" ? false : undefined,
      publicationAttempt,
    );
  }

  private readSurfaceText(surface: Surface): string | undefined {
    const uri = this.documents.get(surface)!;
    const open = vscode.workspace.textDocuments.find(
      (document) => document.uri.toString() === uri.toString(),
    );
    if (open) return open.getText();
    try {
      return new TextDecoder("utf-8", { fatal: true }).decode(fs.readFileSync(uri.fsPath));
    } catch {
      return undefined;
    }
  }

  private refuseSurfaceWrite(
    surface: Surface,
    webGeneration: number | undefined,
    sourceBefore: string | undefined,
    revisionBefore: number | undefined,
  ): void {
    this.pageWriteTexts.delete(surface);
    this.lastPageWriteGenerations.set(surface, 0);
    if (surface !== "dsl" && surface !== "sample") return;
    const current = this.readSurfaceText(surface);
    if (current === undefined) return;
    let revision = this.sourceRevisions.get(surface)!;
    if (revision === revisionBefore && current !== sourceBefore) {
      revision += 1;
      this.sourceRevisions.set(surface, revision);
    }
    const publicationAttempt = surface === "dsl" ? ++this.dslPublicationAttempt : undefined;
    this.sendSurfaceUpdate(
      surface,
      current,
      revision,
      webGeneration,
      false,
      false,
      surface === "dsl" ? true : undefined,
      publicationAttempt,
    );
  }

  private sendSurfaceUpdate(
    surface: Surface,
    text: string,
    sourceRevision: number,
    webGeneration?: number,
    accepted?: boolean,
    retryable?: boolean,
    publicationPending?: boolean,
    publicationAttempt?: number,
  ): void {
    void this.panel.webview.postMessage({
      type: "tclSpecStudioHostUpdate",
      surface,
      text,
      sourceRevision,
      webGeneration,
      accepted,
      retryable,
      publicationPending,
      publicationAttempt,
    });
  }

  private async openSurface(surface: Surface): Promise<void> {
    const uri = this.documents.get(surface)!;
    if (!fs.existsSync(uri.fsPath)) {
      await this.enqueueSurfaceWrite(
        surface,
        "",
        this.sourceRevisions.get(surface)!,
        undefined,
        this.sourceRevisions.get("dsl")!,
      );
    }
    const document = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(document, {
      viewColumn: vscode.ViewColumn.One,
      preserveFocus: false,
    });
  }

  private documentChanged(document: vscode.TextDocument): void {
    const entry = [...this.documents.entries()].find(
      ([, uri]) => uri.toString() === document.uri.toString(),
    );
    if (!entry || this.disposed) return;
    const [surface] = entry;
    if (surface === "rust" || surface === "stub") return;
    if (this.pageWriteTexts.get(surface) === document.getText()) return;
    const revision = this.sourceRevisions.get(surface)! + 1;
    this.sourceRevisions.set(surface, revision);
    this.lastPageWriteGenerations.set(surface, 0);
    const publicationAttempt = surface === "dsl" ? ++this.dslPublicationAttempt : undefined;
    if (surface === "dsl") {
      this.pendingDslGeneration = -1;
      for (const output of ["rust", "stub"] as const) {
        this.writeGenerations.set(output, (this.writeGenerations.get(output) ?? 0) + 1);
      }
    }
    this.writeGenerations.set(surface, (this.writeGenerations.get(surface) ?? 0) + 1);
    this.sendSurfaceUpdate(
      surface,
      document.getText(),
      revision,
      undefined,
      undefined,
      undefined,
      surface === "dsl" ? true : undefined,
      publicationAttempt,
    );
    const key = document.uri.toString();
    const previous = this.saveTimers.get(key);
    if (previous) clearTimeout(previous);
    this.saveTimers.set(
      key,
      setTimeout(() => {
        this.saveTimers.delete(key);
        const expectedRevision = this.sourceRevisions.get(surface)!;
        const expectedText = document.getText();
        void document.save().then(async (saved) => {
          if (
            saved &&
            surface === "dsl" &&
            !this.disposed &&
            this.sourceRevisions.get(surface) === expectedRevision &&
            document.getText() === expectedText
          ) {
            await this.publishCurrentDsl(
              document,
              expectedText,
              expectedRevision,
              publicationAttempt,
            );
          }
        });
      }, 120),
    );
  }

  private documentSaved(document: vscode.TextDocument): void {
    const uri = this.documents.get("dsl")!;
    if (
      this.disposed ||
      document.uri.toString() !== uri.toString() ||
      this.pendingDslGeneration !== -1
    )
      return;
    const revision = this.sourceRevisions.get("dsl")!;
    const attempt = this.dslPublicationAttempt;
    void this.publishCurrentDsl(document, document.getText(), revision, attempt);
  }

  private async publishCurrentDsl(
    document: vscode.TextDocument,
    text: string,
    revision: number,
    attempt: number | undefined,
  ): Promise<void> {
    if (attempt === undefined || this.dslPublicationInFlight.has(attempt)) return;
    this.dslPublicationInFlight.add(attempt);
    try {
      if (
        this.disposed ||
        this.pendingDslGeneration !== -1 ||
        this.sourceRevisions.get("dsl") !== revision ||
        this.dslPublicationAttempt !== attempt ||
        document.getText() !== text
      )
        return;
      const published = await this.publishPackSource(text);
      if (
        published &&
        !this.disposed &&
        this.pendingDslGeneration === -1 &&
        this.sourceRevisions.get("dsl") === revision &&
        this.dslPublicationAttempt === attempt &&
        document.getText() === text
      ) {
        this.pendingDslGeneration = undefined;
        this.sendSurfaceUpdate("dsl", text, revision, undefined, true, false, false, attempt);
      }
    } finally {
      this.dslPublicationInFlight.delete(attempt);
    }
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.sourceLeaseGeneration += 1;
    for (const surface of Object.keys(filenames) as Surface[]) {
      this.writeGenerations.set(surface, (this.writeGenerations.get(surface) ?? 0) + 1);
    }
    for (const timer of this.saveTimers.values()) clearTimeout(timer);
    for (const subscription of this.subscriptions.splice(0)) subscription.dispose();
    void (async () => {
      // Clear the pin before considering files. A random session path alone
      // does not prove that a user has not opened or changed its contents.
      await this.applySampleDialect(undefined, true);
      // VS Code's file-provider API has no conditional delete tied to a live
      // byte receipt, so keep these random session files rather than race a
      // user edit or open.
      // The active source owner is retired independently of file presence.
      const lease = this.sourceLease;
      const uri = this.documents.get("dsl")!;
      if (lease) {
        await this.enqueueLeaseOperation(async () => {
          for (let attempt = 0; attempt < 3 && this.sourceLease === lease; attempt += 1) {
            await this.retireLease(lease, uri);
            if (this.sourceLease !== lease) break;
            await new Promise((resolve) => setTimeout(resolve, 100));
          }
        });
      }
    })();
    if (current === this) current = undefined;
  }
}

export async function openSpecStudio(
  context: vscode.ExtensionContext,
  client: LanguageClient,
): Promise<void> {
  if (current) {
    current.reveal();
    return;
  }
  const folder = vscode.workspace.workspaceFolders?.[0];
  if (!folder) {
    void vscode.window.showErrorMessage(
      "Tcl Spec Studio needs an open workspace for its temporary pack override.",
    );
    return;
  }
  current = new SpecStudioSession(context, client, folder.uri);
  context.subscriptions.push(current);
  await current.start();
}

export const specStudioTest = { injectNativeBridge };
