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
3. **Errors** (if any): for each error list:
   - File path and line number
   - Error message
   - Brief context (1-2 lines)
4. **Warnings** (if any): list only actionable warnings, not standard
   informational messages or deprecation noise
5. **Key output**: any version numbers, package sizes, or notable
   messages the user would want to know

Do NOT include:
- The raw command output
- Dependency resolution / download progress
- Compilation progress for individual files
- npm/pip install chatter
- Successful step confirmations (only note the overall result)

Keep your entire response under 200 words for a passing build, or under
500 words for a failing build. Use markdown formatting.
~~~

$ARGUMENTS
