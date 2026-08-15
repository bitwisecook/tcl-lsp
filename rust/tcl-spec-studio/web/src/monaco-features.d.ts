// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

// Monaco publishes its feature entry points as JavaScript-only side-effect
// modules. Their package export is valid, but there is intentionally no API
// surface for TypeScript to describe.
declare module "monaco-editor/features/*/register.js";
