---
name: build
description: >
  Run build targets and report results. Delegates to a Sonnet agent so
  only errors, warnings, and build status enter the main context —
  keeping Opus token usage low on routine builds.
allowed-tools: Agent
---

# Run Build

Run a build target and return a concise summary. The heavy lifting (running
the command and parsing its output) is always delegated to a **Sonnet agent**
so that verbose build output never enters the main Opus context.

## How to execute

1. Parse `$ARGUMENTS` to determine the make target. Use the mapping below;
   default to `compile` when no argument is given.
2. Spawn a **single** Agent with `model: "sonnet"` using the prompt template
   below. Do **not** run the build command yourself.
3. Relay the agent's summary to the user verbatim — do not embellish or
   re-summarise.

## Argument → make target mapping

| Argument | Make target | Notes |
|---|---|---|
| *(none)* | `make compile` | TypeScript extension compilation (default) |
| `compile` | `make compile` | TypeScript extension |
| `vsix` | `make vsix` | Full VS Code extension package |
| `rust` | `make rust-build` | Rust wheel (maturin + install) |
| `zipapps` | `make zipapps` | All zipapps |
| `zipapp-tcl` | `make zipapp-tcl` | Tcl tools zipapp |
| `zipapp-cli` | `make zipapp-cli` | CLI compiler explorer zipapp |
| `zipapp-lsp` | `make zipapp-lsp` | LSP server zipapp |
| `zipapp-ai` | `make zipapp-ai` | AI analysis zipapp |
| `zipapp-mcp` | `make zipapp-mcp` | MCP server zipapp |
| `zipapp-wasm` | `make zipapp-wasm` | WASM compiler zipapp |
| `jetbrains` | `make jetbrains` | JetBrains plugin (.zip) |
| `sublime` | `make sublime` | Sublime Text package |
| `zed` | `make zed` | Zed extension archive |
| `release` | `make release` | All release artifacts |
| `format` | `make format` | Auto-format Python, TypeScript, Rust |
| `format-py` | `make format-py` | Auto-format Python only |
| `format-ts` | `make format-ts` | Auto-format TypeScript only |
| `codegen` | `make codegen` | Regenerate all generated files |
| `gen-editor-settings` | `make gen-editor-settings` | Regenerate editor settings |
| `kcs-db` | `make kcs-db` | Build KCS help database |
| `skills` | `make claude-skills` | Claude skills release zip |
| `smoke` | `make smoke-zipapps` | Build and smoke-test all zipapps |
| `npm` | `make npm-env` | Install/update npm dependencies |

## Sonnet agent prompt template

Use this prompt, substituting `{COMMAND}` with the resolved command:

~~~
You are a build-output analyst. Run the following command and produce a
concise summary of the results. Your working directory is the project root.

Command:
```bash
{COMMAND}
```

Run the command with a 10 minute timeout. Then analyse the output and
report ONLY the following:

1. **Result**: SUCCESS or FAILURE (one word)
2. **Artefacts** (if any were produced): list file paths and sizes
3. **Errors** (if any): reproduce each error **verbatim** — do not
   paraphrase or shorten error messages, compiler diagnostics, or
   tracebacks. Include:
   - File path and line number exactly as printed
   - The full error message exactly as printed
   - The full traceback or compiler diagnostic chain (trim only
     build-system boilerplate frames, keep everything from the
     first project frame onward)
4. **Warnings** (if any): list only actionable warnings, not standard
   informational messages or deprecation noise. Reproduce warning
   text verbatim.
5. **Key output**: any version numbers, artefact sizes, or notable
   messages the user would want to know

CRITICAL: All error messages, compiler diagnostics, and tracebacks
must be copied character-for-character from the command output. Do
not summarise, paraphrase, or truncate them — the caller needs exact
text to locate and fix the issue.

Do NOT include:
- Successful compilation progress for individual files
- Dependency resolution / download progress
- npm/pip install chatter
- Successful step confirmations (only note the overall result)

Keep your entire response under 200 words for a passing build. For a
failing build there is no word limit — completeness of error context
is more important than brevity. Use markdown formatting.
~~~

$ARGUMENTS
