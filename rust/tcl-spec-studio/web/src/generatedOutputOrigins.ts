// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

export type GeneratedSurface = "rust" | "stub";

/** Retains the DSL image that produced each independently selected output pane. */
export class GeneratedOutputOrigins {
  private readonly origins: Record<GeneratedSurface, string | null> = { rust: null, stub: null };

  set(surface: GeneratedSurface, sourceBasis: string | null): void {
    this.origins[surface] = sourceBasis;
  }

  get(surface: GeneratedSurface): string | null {
    return this.origins[surface];
  }
}
