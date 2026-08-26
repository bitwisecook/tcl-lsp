# KCS: How do I use tcl-lsp in the browser, on vscode.dev or github.dev?

> **Audience:** User
> **Type:** How-To

## Applies to

VS Code

## Question

How do I use tcl-lsp in the browser, on vscode.dev or github.dev?

## Before you start

- A browser. Nothing else — there is no binary to download and no Python,
  and the extension works on a machine where you cannot install software.
- A GitHub repository with Tcl in it, or any folder you can open in
  <https://vscode.dev>.

## Answer

1. Open the editor in the browser: press **.** (full stop) on a GitHub
   repository page, or go to <https://vscode.dev> and use **Open Folder**.
2. Open the **Extensions** view in the activity bar.
3. Search for `tcl-lsp` and install **Tcl/Tk, iRules, EDA-Tools, Expect
   LSP/MCP**. The Marketplace only offers it here because the extension
   declares a browser entry point, so an offered install is one that runs.
4. Open a `.tcl` file (or `.irule`, `.tclspec`, `bigip.conf`, or any other
   registered Tcl-family file).

The language server is the same analyser as on the desktop, compiled to
WebAssembly and running in a background thread inside the page. Everything
stays in the browser — the module ships inside the extension, and the extension
makes no network request of its own.

## How to tell it worked

The status bar shows **tcl-lsp v… (web)** while a Tcl file is focused, and the
file gets semantic colouring and diagnostics. For detail, open **View > Output**
and choose **Tcl Language Server**: it reports the worker it started, how many
workspace files it read, and anything it skipped.

## What is different from the desktop

**Desktop-only commands.** Runtime validation (it runs `tclsh`), the compiler
explorer, the spec studio, the Tk preview, "Copy file as base64", the package
scaffolder, and the `@irule` / `@tcl` / `@tk` chat participants need a process
or a filesystem. Running one in the browser tells you which, instead of failing
obscurely.

**Cross-file results on a GitHub repository.** The browser server has no
filesystem, so the extension reads the workspace itself and hands the files
over. That hand-off currently only carries files on the `file:` scheme, and
github.dev serves a repository on a virtual one — so each file you *open* is
analysed in full, while answers that depend on files you have **not** opened (a
proc defined in a sibling, `package require` resolution, workspace symbols) are
not available there. The output channel says so at startup. Open the sibling
file and its definitions resolve.

**A budget on what is read.** A large workspace is read up to
`tclLsp.web.workspaceSync.maxFiles` (2000 files),
`tclLsp.web.workspaceSync.maxTotalBytes` (32 MiB), and
`tclLsp.web.workspaceSync.maxFileBytes` (2 MiB per file). Nothing is dropped
quietly: every skipped file is named in the **Tcl Language Server** output
channel, with the setting that would include it. Raise the setting, then run
**Tcl: Restart Server**.

**Workspace trust.** Analysis needs no trust — the server never executes the
Tcl it reads. The settings that choose which program the *desktop* extension
launches (`tclLsp.serverPath`, `tclLsp.rustServerPath`,
`tclLsp.runtimeValidation.tclshPath`) and where it reads from
(`tclLsp.libraryPaths`, `tclLsp.specPacks`, `tclLsp.packageManager.*`) are
ignored until you trust the workspace.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [Editor installation](../../INSTALL-editors.md)
- [The closed-file source store](../design/contracts/lsp-source-store.md) — how
  the browser server is given files at all.
