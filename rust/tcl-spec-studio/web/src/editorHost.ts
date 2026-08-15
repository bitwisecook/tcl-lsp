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

// The contract between the studio controller and its text surface.
//
// `studio.ts` is deliberately Monaco-free: it knows only this interface, and
// gets an implementation either from the lazily-loaded `monacoHost` chunk (the
// real editor, driven by the real language server) or from the textarea it has
// always had. Neither side imports the other, which is what makes the fallback
// a swap of one object rather than a branch through every call site.

/** How a surface reports progress and degradation to the page. */
export type Report = (message: string, kind?: "ok" | "err") => void;

/** One text surface the host is asked to take over. */
export interface SurfaceSpec {
  /** The element the editor is mounted into — it is emptied first. */
  container: HTMLElement;
  /** The textarea the surface replaces, kept as the fallback and as state. */
  textarea: HTMLTextAreaElement;
  /** Called on every settled edit, with the surface's full text. */
  onChange: (text: string) => void;
}

/** One read-only rendered-code surface. */
export interface OutputSurfaceSpec {
  /** The element Monaco mounts into. */
  container: HTMLElement;
  /** The fallback `<pre>` whose current text seeds the model. */
  fallback: HTMLElement;
}

/** Everything the host needs to stand all four editors up. */
export interface EditorHostOptions {
  /** Content-keyed URL for the language server worker. */
  workerUrl: string;
  /** Content-keyed URL for Monaco's emitted stylesheet. */
  stylesheetUrl: string;
  /** The VS Code extension's authoritative Tcl TextMate grammar. */
  grammarUrl: string;
  /** Oniguruma WASM used to execute that grammar in the browser. */
  onigurumaUrl: string;
  /** The `.tclspec` pack document. Always opened as `spectcl`. */
  dsl: SurfaceSpec;
  /** The Tcl sample, opened under whichever dialect the studio has selected. */
  sample: SurfaceSpec;
  /** Rendered Rust source, always a Rust model. */
  rust: OutputSurfaceSpec;
  /** Rendered Tcl stub, opened as Tcl 9 + SpecTcl. */
  stub: OutputSurfaceSpec;
  /** The studio's current dialect, used as the sample's LSP language id. */
  dialect: string;
  /** Where boot progress and any degradation is announced. */
  report: Report;
}

/** The four mounted code editors. */
export interface EditorHost {
  /** Replace the pack document's text without re-entering `onChange`. */
  setDslText(text: string): void;
  /** Replace the sample's text without re-entering `onChange`. */
  setSampleText(text: string): void;
  /** Replace the rendered Rust model. */
  setRustText(text: string): void;
  /** Replace the rendered Tcl-stub model. */
  setStubText(text: string): void;
  /** Re-open the sample under a new dialect (its LSP language id). */
  setDialect(dialect: string): void;
  /** Re-measure after the containing tab became visible. */
  layout(): void;
  /** Whether the language server answered `initialize`. */
  readonly lspReady: boolean;
}

/** The lazily-imported chunk's shape — the one export `studio.ts` calls. */
export interface MonacoHostModule {
  /** `git describe` embedded in this lazy bundle. */
  buildVersion: string;
  mountEditors(options: EditorHostOptions): Promise<EditorHost>;
}
