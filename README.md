<p align="center">
  <!-- PNG, not SVG: the VS Code Marketplace (vsce) rejects SVG and picky
       <picture>/srcset image sources in READMEs. Rendered from
       docs/tcl-lsp-logo.svg by `make logo`. -->
  <img src="docs/Tcl LSP Logo-8bit-512.png" alt="Tcl LSP" width="128">
</p>

# tcl-lsp

A language server for Tcl with multi-editor support.

<p align="center">
  <img src="docs/screenshots/tcl-lsp-demo.gif" alt="tcl-lsp in action" width="820">
</p>

The server is written in Python using [pygls](https://github.com/openlawlibrary/pygls)
and communicates over stdio, making it compatible with any LSP client.

## Editor support

> **Installation guides:**
> [INSTALL-editors.md](INSTALL-editors.md) — step-by-step setup for
> VS Code, Neovim, Zed, Emacs, Helix, Sublime Text, and JetBrains on
> macOS (Homebrew), Linux (Debian/Ubuntu, RHEL/CentOS, Fedora), and
> Windows.
> [INSTALL-cli.md](INSTALL-cli.md) — the `tcl` and `f5` CLIs,
> including a one-line `curl | sh` installer.

### All editors

| Editor | Type | Setup | Unique extras |
|--------|------|-------|---------------|
| [VS Code](editors/vscode/) | Full extension (.vsix) | Install `.vsix` from Releases | Compiler explorer panel, Tk preview, `@irule`/`@tcl`/`@tk` Copilot chat, 25+ commands |
| [Neovim](editors/neovim/) | Config snippet (Lua) | Copy `tcl_lsp.lua` to `~/.config/nvim/server/` | Zero-plugin on 0.11+; also supports nvim-lspconfig |
| [Zed](editors/zed/) | Full extension (TOML + Rust) | Install from Zed extension registry | 16 built-in snippets, MCP context server, `/tcl-doc` and `/irule-event` slash commands |
| [Emacs](editors/emacs/) | Config snippet (Elisp) | Add to `init.el` for eglot or lsp-mode | Works with built-in eglot (Emacs 29+) |
| [Helix](editors/helix/) | Config snippet (TOML) | Add to `~/.config/helix/languages.toml` | Minimal pure-TOML setup |
| [Sublime Text](editors/sublime-text/) | Full package (.sublime-package) | Package Control or manual install | Works standalone (syntax + snippets) without LSP; enhanced with LSP package |
| [JetBrains](editors/jetbrains/) | Full plugin (.zip) | Settings > Plugins > Install from Disk | Compiler explorer tool window, settings UI panel, IntelliJ IDEA 2024.1+ |

All editors connect to the same Python LSP server over stdio.  The server can
be invoked from source (`uv run python -m server`) or as a standalone zipapp
(`python3 tcl-lsp-server.pyz`).

**Also documented in [INSTALL-editors.md](INSTALL-editors.md):**

- *VS Code-compatible editors* (load the same `.vsix` unchanged) —
  Cursor, Windsurf, VSCodium, code-server / Coder, GitHub Codespaces,
  Gitpod, and Eclipse Theia.
- *Other LSP-capable editors* (point a generic LSP client at the
  `.pyz`) — Vim (vim-lsp or coc.nvim), Kate, Kakoune, Notepad++, Geany,
  Lite XL, micro, CudaText, JupyterLab, Doom Emacs, and Spacemacs.

**File types recognised:** `.tcl`, `.tk`, `.itcl`, `.tm`, `.irul`, `.irule`,
`.iapp`, `.iappimpl`, `.impl`, `.apl`, `.exp`, plus shebang detection for
`#!/usr/bin/tclsh`, `#!/usr/bin/wish`, and `#!/usr/bin/expect`.
Files named `presentation` (no extension) are auto-detected as APL.
Per-file `# tcl-dialect:` comment directives pin a specific dialect.

### VS Code

The full-featured extension, distributed as a `.vsix`, bundles the LSP server
and provides the richest integration.

**25+ commands** including: Restart Server, Select Dialect, Apply Safe Quick
Fixes, Apply All Optimisations, Open in Tcl Compiler Explorer, Open Tk Preview,
Format Document, Minify Document, Insert iRule Event Skeleton, Scaffold Tcl
Package Starter, Insert `package require`, Run Runtime Validation, Translate
iRule to F5 XC, Extract iRule from Config, Escape/Unescape Selection, Base64
Encode/Decode Selection.

**Keyboard shortcuts:** Ctrl+Alt+O (optimise), Ctrl+Alt+M (minify),
Ctrl+Alt+E (compiler explorer).

**Status bar:** shows the active dialect (clickable to change) and the
extension version.

Install: see [INSTALL-editors.md](INSTALL-editors.md#vs-code).

### Neovim

Zero-plugin setup on Neovim 0.11+ using the native LSP client.  Also works
with nvim-lspconfig (0.8+) or a manual `FileType` autocommand.

```lua
-- ~/.config/nvim/server/tcl_lsp.lua  (Neovim 0.11+)
return {
  cmd = { "python3", "/path/to/tcl-lsp-server.pyz" },
  filetypes = { "tcl" },
  settings = {
    tclLsp = {
      dialect = "tcl8.6",
      formatting = { indentSize = 4, maxLineLength = 120 },
    },
  },
}

-- init.lua
vim.filetype.add({ extension = { tcl = "tcl", irul = "tcl", irule = "tcl" } })
vim.lsp.enable("tcl_lsp")
```

### Zed

A full Zed extension that auto-downloads the server zipapp from GitHub Releases
on first use and auto-discovers Python 3.10+ on your PATH.

Includes 16 built-in snippets (`tcl-proc`, `tcl-namespace`, `tcl-if`,
`irule-http-request`, `irule-collect-release`, etc.), an MCP context server
exposing all 44 analysis tools, and slash commands (`/tcl-doc`, `/irule-event`,
`/tcl-validate`).

Install: see [INSTALL-editors.md](INSTALL-editors.md#zed).

### Emacs

Works with the built-in **eglot** client (Emacs 29+) or **lsp-mode**.

```elisp
;; eglot (Emacs 29+)
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               '(tcl-mode . ("python3" "/path/to/tcl-lsp-server.pyz"))))
(add-hook 'tcl-mode-hook #'eglot-ensure)

;; Settings
(setq-default eglot-workspace-configuration
              '(:tclLsp (:dialect "tcl8.6"
                         :formatting (:indentSize 4 :maxLineLength 120))))
```

### Helix

Minimal TOML configuration in `~/.config/helix/languages.toml`.

```toml
[language-server.tcl-lsp]
command = "python3"
args = ["/path/to/tcl-lsp-server.pyz"]

[language-server.tcl-lsp.config.tclLsp]
dialect = "tcl8.6"

[[language]]
name = "tcl"
scope = "source.tcl"
file-types = ["tcl", "tk", "itcl", "tm", "irul", "irule", "iapp"]
language-servers = ["tcl-lsp"]
```

### Sublime Text

A full Sublime Text package (`.sublime-package`) that works in two modes:
standalone (syntax highlighting + 16 snippets + static completions) and
enhanced (full LSP features when the LSP package is installed).

Auto-discovers the bundled `.pyz` server from the package archive.

Install: see [INSTALL-editors.md](INSTALL-editors.md#sublime-text).

**Commands:** Select Dialect, Restart Language Server, Format Document, Minify
Document, Apply Safe Quick Fixes, Apply All Optimisations.

### JetBrains

A full IntelliJ Platform plugin (`.zip`) for IntelliJ IDEA 2024.1+ and other
JetBrains IDEs.  Includes a dedicated settings panel (Settings > Tools > Tcl
Language Server) with toggles for every feature, diagnostic code, and
formatting option.

Features a **Compiler Explorer tool window** with JCEF browser for inspecting
IR, CFG, SSA, and optimiser output directly inside the IDE.

Install: see [INSTALL-editors.md](INSTALL-editors.md#jetbrains).
Build from source: `make build-editor-jetbrains`.

## Features

### Async tiered diagnostics

Fast syntax feedback fires immediately on every keystroke; deeper semantic,
optimiser, and security analysis runs in the background and merges results as
each tier completes.

```tcl
# Tier 1 (instant): syntax errors — missing brace caught on parse
proc broken {x {
    puts $x
}

# Tier 2 (background): semantic — arity mismatch flagged after analysis
string length "a" "b"   ;# E003: too many arguments
```

### Semantic highlighting

Variables, procs, keywords, and strings are classified using SSA-informed type
information, giving richer highlighting than a TextMate grammar alone.  The
server provides 44 token types beyond the standard LSP set, including
sub-token highlighting inside strings.  Tokens are cached per top-level chunk
so only dirty regions are recomputed after an edit, and the server supports
`textDocument/semanticTokens/full/delta` for bandwidth-efficient incremental
updates.

```tcl
namespace eval app {
    variable count 0            ;# 'count' highlighted as variable
    proc handle {request} {     ;# 'handle' highlighted as function
        incr count              ;# 'incr' highlighted as keyword
        puts "req: $request"    ;# '$request' highlighted as variable inside string
    }
}
```

In addition to standard token types (keyword, function, variable, string,
comment, number, operator, parameter, namespace), the server provides
domain-specific token types:

| Category | Token types | Example |
|----------|-------------|---------|
| **Regexp** | `regexpGroup`, `regexpCharClass`, `regexpQuantifier`, `regexpAnchor`, `regexpEscape`, `regexpBackref`, `regexpAlternation` | `regexp {(\d+)\s+(\w+)} $line` — each part gets distinct highlighting |
| **Format strings** | `formatPercent`, `formatSpec`, `formatFlag`, `formatWidth` | `format "%- 10.2f" $val` — `%`, `-`, `10.2`, and `f` each highlighted |
| **Binary format** | `binarySpec`, `binaryCount`, `binaryFlag` | `binary scan $data su3 x y z` — `s`, `u`, and `3` each highlighted |
| **Clock format** | `clockPercent`, `clockSpec`, `clockModifier` | `clock format $t -format "%Y-%m-%d"` — `%`, `Y`, `m`, `d` each highlighted |
| **Escape sequences** | `escape` | `puts "line1\nline2\t${var}"` — `\n`, `\t` highlighted inside strings |
| **BIG-IP config** | `object`, `ipAddress`, `port`, `partition`, `pool`, `monitor`, `profile`, `vlan`, `fqdn`, `routeDomain`, `encrypted`, `interface` | BIG-IP `.conf` files get object-aware highlighting |

### Diagnostics

Arity errors, unknown subcommands, best-practice violations, and security
issues are reported with precise ranges.  Diagnostics can be suppressed
inline, per-file, per-project, per-editor, or globally — see
[Suppressing diagnostics](#suppressing-diagnostics).

```tcl
string frobulate $x          ;# W001: unknown subcommand 'frobulate'
set y [expr $a + $b]         ;# W100: unbraced expr (double-substitution risk)
eval $user_input             ;# W101: eval with substituted arguments (injection risk)
catch { error "oops" }       ;# W302: catch without a result variable
```

### Completions

Context-aware completions for commands, subcommands, variables, proc names
(workspace-wide), switch arms, and `package require` names.

```tcl
string len|              ;# offers: length, last, ...
set name "world"
puts $na|                ;# offers: $name
dict |                   ;# offers: create, get, set, exists, ...
```

### Hover

Hovering on a command, proc call, variable, or operator shows its signature,
doc comment, and type information.  Multi-line docstrings are supported, and
`@param`, `@return`, and `@brief` tags are parsed and displayed as structured
markdown.  Docstrings can appear above the proc or inside the proc body.

```tcl
# @brief Greet a person by name.
# @param name - Who to greet
# @return The greeting string
proc greet {name} {
    return "Hello, $name!"
}

greet "Alice"     ;# hover on 'greet' shows signature + formatted @param/@return docs
```

### Go to definition

Jump to the definition of a proc or variable — works across files in the
workspace.

```tcl
proc helper {} { return 42 }
set x [helper]       ;# Ctrl+Click on 'helper' → jumps to proc definition above
puts $x              ;# Ctrl+Click on '$x' → jumps to the set statement
```

### Find references

Locate every usage of a proc or variable, including inside nested braced
script bodies such as `if`, `foreach`, and `namespace eval`.

```tcl
proc add {a b} { expr {$a + $b} }
set sum [add 1 2]       ;# ← reference to 'add'
puts [add 3 4]           ;# ← reference to 'add'
# "Find all references" on 'add' highlights all three locations
```

### Call hierarchy

Inspect incoming callers and outgoing callees for any procedure.

```tcl
proc validate {input} { return [string is integer $input] }
proc process {data}   { if {[validate $data]} { store $data } }
proc main {}          { process "42" }

# Incoming calls to 'validate': process
# Outgoing calls from 'process': validate, store
```

### Rename symbol

Safely rename a proc or variable across all scopes in the file.

```tcl
proc greeting {name} {
    puts "Hi, $name"
}
greeting "World"
# Rename 'greeting' → 'salute' updates the proc definition and all call sites
```

### Signature help

As you type arguments, the server shows the expected parameter list with the
active parameter highlighted.

```tcl
proc connect {host port {timeout 30}} { ... }
connect "db.local" |
#                  ↑ signature help shows: connect (host port ?timeout?)
#                    with 'port' highlighted as the active parameter
```

### Inlay hints

Inline annotations show inferred types, format-string specifier meanings, and
parameter names.

```tcl
set count 42                          ;# inlay: ': int'
set msg [format "%s has %d items" $name $count]
#                 ↑ '%s → string'  ↑ '%d → integer'
```

### Document symbols

A structured outline of the current file — procs, namespaces, variables — for
quick navigation (Ctrl+Shift+O / Cmd+Shift+O).

```tcl
namespace eval app {
    variable config {}         ;# symbol: app::config (variable)
    proc init {} { ... }       ;# symbol: app::init (function)
    proc run {} { init }       ;# symbol: app::run (function)
}
# Outline: app (namespace) → config, init, run
```

### Workspace symbols

Search for procs and variables across all open files in the workspace
(Ctrl+T / Cmd+T).

```tcl
# File: utils.tcl
proc ::utils::parse_csv {data} { ... }

# File: main.tcl
# Type "parse_csv" in workspace symbol search → jumps to utils.tcl
```

### Folding ranges

Collapse proc bodies, control-flow blocks, multi-line comments, and namespace
bodies.

```tcl
# ── Header comment ──          ← foldable
# Author: ...
proc calculate {x} {            ← foldable
    if {$x > 0} {               ← foldable
        return [expr {$x * 2}]
    }
}
```

### Selection ranges

Smart expand/shrink selection by syntactic structure
(Alt+Shift+→ / Alt+Shift+←).

```tcl
proc greet {name} {
    puts "Hello $name"
}
# Cursor on 'name' inside puts → expand: "$name" → "Hello $name" → puts command → proc body → proc → file
```

### Document links

`source` paths and `package require` names become clickable links that
navigate to the resolved file or package.

```tcl
package require http        ;# click → opens http package source
source lib/utils.tcl        ;# click → opens lib/utils.tcl
```

### Formatting

Full-document and range formatting with 25 configurable options.  Defaults
follow the F5 iRules Style Guide.  Supports full-document
(`textDocument/formatting`) and range (`textDocument/rangeFormatting`)
requests.

```tcl
# Before:
proc messy { x  }  {
if {$x>0}{return $x }
   set y   [expr $x+1]  ;  puts $y }

# After (formatted):
proc messy {x} {
    if {$x > 0} {
        return $x
    }
    set y [expr $x + 1]
    puts $y
}
```

Capabilities include indentation (spaces or tabs, configurable size),
brace placement (K&R), expression bracing enforcement, variable
bracing (`$var` → `${var}`), line-length wrapping, semicolon splitting,
single-line body expansion, blank-line normalisation between procs and
blocks, comment alignment, trailing whitespace trimming, and line-ending
normalisation (LF/CRLF/CR).

```tcl
# Expression bracing enforcement (enforceBracedExpr = true):
if {$x > 0} { ... }       ;# ✓ braced
if $x>0 { ... }           ;# → rewritten to: if {$x > 0} { ... }

# Variable bracing (enforceBracedVariables = true):
puts $name                 ;# → rewritten to: puts ${name}
```

### Code actions

Quick-fix actions are offered for diagnostics that have automated repairs.
Refactoring actions are available on selected code.

```tcl
expr $a + $b         ;# W100 → quick-fix: wrap in braces → expr {$a + $b}
catch { error "x" }  ;# W302 → quick-fix: add result variable → catch { error "x" } result
set x [expr {$x+1}]  ;# O114 → quick-fix: use incr idiom → incr x
```

**Extract to proc** — select one or more lines, trigger code actions
(`Ctrl+.`), and choose *Extract selection into proc*. The selected code
moves into a new `proc` with detected variable parameters; the original
lines are replaced with a call. The editor places the cursor on the new
proc name so you can rename it immediately.

### Snippets

Bundled code templates for Tcl structures and iRules event skeletons with
secure defaults, collect/release pairs, and common patterns.

```tcl
# Type 'proc' + Tab:
proc name {args} {
    # body
}

# Type 'when' + Tab (iRules):
when HTTP_REQUEST {
    # handler
}
```

### Dialect profiles

Switch between Tcl 8.4/8.5/8.6/9.0, F5 iRules, F5 iApps, and EDA tooling
profiles.  Tk, tcllib, and stdlib commands activate automatically when their
`package require` appears. F5 iRules metadata follows BIG-IP command/event
source data, including profile aliases used by newer namespaces and events,
shared TLS helper profiles such as `PERSIST`, and protocol namespace layer
metadata that stays aligned with the enabling profile stack.

```tcl
# With dialect = tcl8.6:
try {
    open $path r         ;# ✓ known in 8.6
} on error {msg} {
    puts $msg
}

# With dialect = tcl8.5:
try { ... }              ;# W002: command disabled in active dialect (try requires 8.6)
```

### TclOO support

Full TclOO class hierarchy analysis with method resolution order (MRO),
class definition tracking, and object-aware introspection.

```tcl
oo::class create Animal {
    variable name
    constructor {n} { set name $n }
    method speak {} { return "$name says ..." }
}
oo::class create Dog {
    superclass Animal
    method speak {} { return "[my name] says woof!" }
}
# Hover on 'Dog' shows class hierarchy: Dog -> Animal -> oo::object
# Go-to-definition on 'speak' jumps to the method body
# Type hierarchy shows Dog as a subtype of Animal
```

Features include class definition and method hover, go-to-definition for
methods and constructors, type hierarchy (supertypes and subtypes), MRO
computation matching C Tcl's algorithm, mixin and filter chain support,
private variable and method visibility (TIP 500), and property/configurable
support (TIP 558).  The VM executes TclOO code with 85% native test
conformance against the Tcl 9.0.3 oo.test suite.

### Compiler pipeline

The server lowers source to an intermediate representation, builds a
control-flow graph, converts to SSA form, and runs type inference — all used
to power deeper diagnostics and the optimiser.

```tcl
proc fibonacci {n} {
    set a 0; set b 1
    for {set i 0} {$i < $n} {incr i} {
        set t $b
        set b [expr {$a + $b}]
        set a $t
    }
    return $a
}
# IR → CFG → SSA → SCCP → liveness → type inference → bytecode
```

The WASM code generator uses a per-proc **var-escape analysis** to decide
which Tcl variables can stay in fast WASM locals and which must spill to
the runtime frame so `uplevel`, `upvar`, `eval`, and dynamic `set $name`
can see them by name. Procs that provably never let a variable escape pay
zero frame-sync overhead on interpreter fallbacks. See the
[design doc](docs/design/compiler/var-escape-analysis.md) and the
[KCS note](docs/kcs/features/kcs-feature-var-escape-analysis.md) for the
rules and the interprocedural propagation of callee `upvar` sources.

### Static optimiser

Twenty-plus optimisation passes detect constant propagation, dead code,
redundant computations, loop-invariant hoisting, strength reduction, and
idiomatic rewrites — each offered as a quick-fix code action.

```tcl
# O102 — constant expression folding:
set a 1
set b [expr {$a + 2}]   ;# → suggestion: replace with 'set b 3'

# O114 — incr idiom recognition:
set x [expr {$x + 1}]   ;# → suggestion: replace with 'incr x'

# O105 — constant var-ref propagation / redundant computation (GVN/CSE):
set a [expr {$x + $y}]
set b [expr {$x + $y}]  ;# → suggestion: replace with 'set b $a'

# O106 — loop-invariant code motion (LICM):
for {set i 0} {$i < $n} {incr i} {
    set len [string length $fixed]   ;# → suggestion: hoist above the loop
    lappend result $len
}
```

### Shimmer detection

Tracks each variable's Tcl internal representation through the SSA type
lattice.  When a command forces a type conversion ("shimmer"), the
performance cost is reported — especially inside loops.

```tcl
# S100 — single shimmer (info):
set x "hello"
set n [llength $x]       ;# 'x' shimmers from STRING → LIST

# S101 — shimmer inside loop (warning):
set s "42"
for {set i 0} {$i < 1000} {incr i} {
    set v [expr {$s + $i}]   ;# 's' shimmers STRING → INT on every iteration
}

# S102 — type thunking (warning):
for {set i 0} {$i < 100} {incr i} {
    set n [llength $data]     ;# 'data' shimmers STRING → LIST
    set data "updated $n"     ;# 'data' back to STRING — oscillates each iteration
}
```

S100–S102 are *performance* warnings.  **S110** is a *correctness* warning for
byte-array corruption: binary data (a `binary format` result or an iRules
`*::payload` byte array) that is forced through character-string semantics and
then written back as bytes silently re-encodes every byte `≥ 0x80`.  This is the
canonical iRules payload-rewrite bug ([F5 KB K22406348](https://my.f5.com/manage/s/article/K22406348)).

```tcl
# S110 — byte-array corruption (warning):
when HTTP_REQUEST_DATA {
    set body [HTTP::payload]
    set body "$body INJECTED"          ;# byte array decoded to a character string
    HTTP::payload replace 0 100 $body  ;# ✗ written back: UTF-8 re-encodes high bytes
}

# Fix — re-binarify before writing back (or avoid the string detour):
when HTTP_REQUEST_DATA {
    set body [HTTP::payload]
    set body "$body INJECTED"
    binary scan $body c* -             ;# forces a byte-array intrep
    HTTP::payload replace 0 100 $body  ;# ✓ written byte-for-byte
}

# Plain Tcl — string case folding mangles a byte array directly:
set ba [binary format c* {128 195 255}]
set up [string toupper $ba]            ;# ✗ S110: 0xFF → U+0178 corrupts the bytes
```

### Taint analysis

Colour-aware data provenance tracking follows untrusted I/O through
assignments, interpolation, and phi nodes to dangerous sinks.  Commands that
produce fixed-type results (e.g. `string length`) act as sanitisers.

```tcl
# T100 — tainted data in dangerous sink:
set input [gets stdin]
eval $input                  ;# ✗ tainted data flows into eval

# T102 — tainted data in option position:
set pat [HTTP::uri]
regexp $pat $string          ;# ✗ tainted pattern without '--' terminator
regexp -- $pat $string       ;# ✓ safe: '--' prevents option injection

# IRULE1007 — collect without release (side-aware):
when HTTP_REQUEST {
    HTTP::collect 1048576    ;# ✗ missing matching HTTP::release on client side
}
```

### Semantic graph queries

Call graph, symbol graph, and data-flow graph are exposed for AI agent
consumption — enabling automated code review, impact analysis, and
refactoring assistance.

```tcl
proc validate {input} { string is integer $input }
proc store {data}     { puts $data }
proc process {x}      { if {[validate $x]} { store $x } }

# Call graph query: "who calls validate?" → process
# Symbol graph query: "variables in process" → x
# Data-flow query: "trace $x" → parameter → validate → store
```

### Compiler explorer (VS Code panel)

An interactive webview panel (Ctrl+Alt+E / Cmd+Alt+E) that visualises the
compiler's intermediate representation, control-flow graph, SSA form,
optimiser output, Tcl bytecode, and WebAssembly disassembly for the active
editor.  The **WASM** tab renders each instruction with its originating Tcl
source range (click an instruction to place the source cursor inside the
expression, substituted command, or post-`;` sub-command it compiled from),
resolved call targets (click `call 42 ; ::greet` to jump to both the
callee's disassembly and its definition), resolved branch targets (click
`br 0 ; loop_header foreach` to jump to the matching `loop` open), a
labelled `block` / `loop` / `if` for each Tcl construct (`foreach`,
`while`, `for`, `if`, `catch body`, `switch arm`), a source-line comment
above every instruction group, and orthogonal control-flow arrows in the
left gutter.

The IR, CFG, SSA, bytecode, and WASM tabs each carry an **optimiser lens**
(`off` / `on` / `diff`).  The `diff` mode compares the relevant node — IR
statement, CFG block, or bytecode instruction — rather than raw text, so
byte offsets, source ranges, sequence indices, and tree-connector glyphs
that merely shift when the optimiser adds or removes a node are ignored.
A single rewrite then shows as a single localised change instead of every
following line being flagged.  The `tcl-explorer` CLI and TUI render the
same offset-free diff via `--opt diff`.

```
┌─────────────────────────────────────────────────┐
│  IR  │  CFG  │  SSA  │  Optimiser  │  Bytecode  │
├─────────────────────────────────────────────────┤
│  proc fibonacci {n}                             │
│    ENTRY:                                       │
│      %0 = param n                               │
│      %1 = const 0        ;  set a 0             │
│      %2 = const 1        ;  set b 1             │
│    LOOP:                                        │
│      %3 = phi [%1, ENTRY] [%6, BODY]            │
│      ...                                        │
└─────────────────────────────────────────────────┘
```

![Compiler explorer — IR](docs/screenshots/10-compiler-explorer.png)

![Compiler explorer — CFG](docs/screenshots/11-compiler-cfg.png)

![Compiler explorer — SSA](docs/screenshots/12-compiler-ssa.png)

![Compiler explorer — Optimiser](docs/screenshots/13-compiler-optimiser.png)

### Tk preview (VS Code panel)

A live preview panel that extracts the widget hierarchy from Tk source and
renders a visual layout — updates in real time as you edit.

```tcl
package require Tk
ttk::frame .f
ttk::label .f.lbl -text "Name:"
ttk::entry .f.ent -textvariable name
ttk::button .f.btn -text "OK" -command { puts $name }
grid .f.lbl .f.ent .f.btn -padx 5 -pady 5
pack .f
# Preview panel shows the grid layout with label, entry, and button
```

### BIG-IP configuration support

Open a BIG-IP `.conf` or `.scf` file to get syntax highlighting, object
navigation, and iRule extraction.

```
# BIG-IP config file (bigip.conf)
ltm virtual /Common/my_vs {
    destination /Common/10.0.0.1:443
    pool /Common/my_pool
    rules {
        /Common/my_irule        ← right-click → "Open iRule in Editor"
    }
}
# "Extract All iRules to Files..." exports every iRule to separate .tcl files
```

**`f5` CLI tool with a `cleanup` verb** — find every object the
configuration defines but no virtual server (or wide-IP) references,
and emit a `tmsh delete` script in reverse-topological order so each
delete runs only after the objects that reference its target have
already been removed.  iRule bodies are scanned too (`pool …`,
`SSL::profile …`, `class match …`, `persist …`, `snatpool …`,
`virtual …`, `node …`, `LSN::pool …`, `STATS::*`, `ifile …`,
`HTTP::respond ifile …`, plus every other iRule command that names a
BIG-IP object).  Constant-string variables are tracked through `set
var /Common/foo; pool $var` linear copy-propagation, so refs written
through local bindings are caught.

```
f5 cleanup samples/bigip/bigip.conf
f5 cleanup --keep /Common/critical_pool bigip.conf
f5 cleanup --json bigip.conf > report.json
```

**`f5 grep` verb** — find every BIG-IP object related to a given
object name (or regex, or CIDR) by walking the same
forward-and-reverse reference graph the cleanup analysis uses.  By
default the BFS traverses both directions, so a single command
surfaces the seed's full neighbourhood: forward edges (objects the
seed depends on) and reverse edges (objects that depend on the seed).

`--cidr` switches the seed selector from "match the object's full
path" to "match an IP address or CIDR mentioned anywhere inside the
object — header, body, or iRule script".  Multiple networks may be
passed at once as a comma- or whitespace-separated list, and an
object qualifies when any IP/CIDR token in its text overlaps any
requested network.  This catches addresses buried deep inside iRule
bodies (`if { [IP::addr [IP::client_addr] equals "10.0.0.5"] }`,
`class match … "10.0.0.0/8"`, …) that a plain path grep can't reach.

```
f5 grep /Common/web_pool bigip.conf
f5 grep --direction reverse /Common/web1 bigip.conf
f5 grep --regex '^/Common/(web|api)_pool$' bigip.conf
f5 grep --json --max-depth 2 web_pool bigip.conf
f5 grep --cidr 10.0.0.0/8 bigip.conf
f5 grep --cidr '10.0.0.0/8, 192.168.0.0/16' bigip.conf
f5 grep --no-recurse --cidr 10.0.0.0/8 bigip.conf
```

The related-object BFS is on by default; pass `--no-recurse` to
skip it and return only the objects that directly match the
pattern (`-r` / `--recurse` toggle it explicitly back on).  This
applies to every match mode: substring, `--regex`, and `--cidr`.

**`f5 irule` verb group** — iRules-specific analysis with
`event-order` and `event-info` sub-actions, defaulting to the
`f5-irules` dialect:

```sh
f5 irule event-order samples/irules/policy.irule
f5 irule event-info HTTP_REQUEST --json
```

`f5` is a separate CLI from `tcl`.  The full verb list (today):

| Group | Verbs |
| --- | --- |
| Acquisition | `fetch`, `extract` (UCS → SCF) |
| Analysis | `stats`, `graph`, `explain`, `diff`, `grep`, `cleanup`, `validate` |
| Transformation | `rename`, `redact`, `unredact`, `encrypt-secrets`, `decrypt-secrets`, `pcap-remap`, `split`, `merge`, `convert`, `tmsh` |
| Round-trip | `pull`, `push` |
| iRules | `irule event-order`, `irule event-info`, `irule lint`, `irule trace`, `irule extract` |
| Misc | `completion` |

Highlights of the newer verbs:

- **`f5 fetch`** — pull SCF/UCS from a live BIG-IP via iControl REST or
  SSH (system `ssh`/`scp`).  Credentials resolve from CLI flags, env
  vars, an XDG `hosts.toml`, or interactive prompt.
- **Encrypted UCS** — archives saved with `tmsh save sys ucs <name>
  passphrase <pass>` are GnuPG symmetric (AES-128) OpenPGP messages (F5
  KB K5437).  Every verb that reads a `.ucs` — `extract`,
  `convert ucs2scf`, `query`, `grep`, `cleanup`, `diff`, `irule …` —
  decrypts them transparently and entirely **in memory**; the decrypted archive
  (which holds SSL private keys) never touches disk.  The passphrase is
  read from `$F5_UCS_PASSPHRASE` or a secure terminal prompt; `extract`
  and `convert` also accept `--passphrase-env VAR` / `--no-passphrase-prompt`.
  Decryption shells out to `gpg`/`gpg2` when present (exactly what BIG-IP
  uses) and otherwise falls back to a bundled, dependency-free
  pure-Python OpenPGP decryptor, so it works even in the zipapp on a host
  with no GnuPG installed.

  ```sh
  export F5_UCS_PASSPHRASE='…'        # or be prompted on a TTY
  f5 extract encrypted.ucs -o prod.scf
  f5 query '.ltm.virtual[].name' encrypted.ucs
  ```
- **`f5 explain {virtual|pool} <name>`** — print the resolved profile
  chain, iRule chain, persistence, SNAT, default pool, and members for
  one object: the operator's "what actually happens to this VIP?"
  question, answered in one command.
- **`f5 diff old.scf new.scf`** — semantic, object-aware diff that
  ignores property ordering and iRule whitespace.  Each side may be an
  SCF / `bigip.conf` stanza dump *or* a tmsh command script
  (`tmsh create` / `tmsh modify` lines, as emitted by `f5 tmsh` or
  pasted from a BIG-IP shell), and the two formats may be mixed.  Every
  config-producing verb (`extract`, `pull`, `grep`, `split`, `merge`,
  `rename`, `redact`, `unredact`) also takes `--format scf|tmsh` so the
  same artefact can be replayed either way.
- **`f5 redact` + `f5 unredact`** — strip secrets and remap public IPs
  while preserving CIDR relationships (a /24 of real IPs lands in a /24
  of redacted IPs).  A sidecar map file makes the redaction reversible
  *and stable across runs* — re-running `redact` with the same map
  reuses every prior assignment, so iterative work with F5 support
  stays consistent.  `unredact` walks the map in reverse over any text,
  including support emails and log snippets.
- **`f5 encrypt-secrets` + `f5 decrypt-secrets`** — encrypt or decrypt the
  credential-bearing values in a `bigip.conf` / SCF (passphrase, password,
  secret, shared-secret, auth-password, privacy-password)
  using the unit master key — the base64 key `f5mku -K` prints on the
  device.  `encrypt-secrets` wraps clear-text values in the
  `$M$<salt>$<base64>` envelope BIG-IP stores; `decrypt-secrets` recovers the
  clear text.  Both leave values already in the target form untouched, so
  they are idempotent.  The key is supplied via `--f5mku KEY`,
  `--f5mku-file FILE`, or `$F5MKU`, otherwise it is read from a secure
  `F5 MKU Key:` terminal prompt (suppress with `--no-key-prompt`); the
  AES-ECB transform runs on the bundled pure-Python cipher, so it works
  in the zipapp with no `cryptography` dependency.

  ```sh
  f5mku -K > key.txt                                       # on the device
  f5 decrypt-secrets bigip.conf --f5mku-file key.txt       # reveal secrets
  F5MKU="$(cat key.txt)" f5 encrypt-secrets clear.conf -o sealed.conf
  ```
- **`f5 pcap-remap`** — apply the same map to a PCAP capture: rewrites
  IPv4/IPv6 src/dst, recomputes IP and TCP/UDP/ICMP checksums, and
  *parses* the F5 Ethernet trailer (legacy + DPT formats; `tcpdump -i
  0.0:nnnp`) to rewrite peer IPs at schema-known offsets.  Schema
  ported from Wireshark's `packet-f5ethtrailer.c`; `--schema FILE`
  layers in fleet-specific extensions; `--on-unknown=error|preserve|sweep`
  picks the policy when a TLV has no registered layout.
- **`f5 tmsh`** — emit `tmsh create` (or `--modify`) commands for every
  object in a config, in dependency order so the script can be pasted
  into a BIG-IP shell unchanged.
- **`f5 query` (alias `f5 q`)** — small jq-flavoured DSL for inspecting
  and rewriting BIG-IP configs.  Built-in **renderer plugins** turn
  query output into a Mermaid diagram, an ASCII Gantt timeline of
  monitor up/down transitions, or a Unicode line-art block diagram —
  no sidecar Python scripts required.  Run
  `f5 q --help-renderers` for the catalogue:

  ```sh
  # ASCII Gantt of pool-member up/down events from a BIG-IP log
  f5 q --render gantt '
      f5log_load("ltm.log")[]
      | select(.module == "01340011" or .module == "01340012")
      | tsv(.timestamp,
            (sub(.message, "^.*member ", "") | sub(., " monitor.*$", "")),
            (if .module == "01340011" then "DOWN" else "UP" end))
  ' bigip.conf

  # Mermaid diagram of every web virtual server and its references
  f5 q --render mermaid '.ltm.virtual["~/web_"]' bigip.conf
  ```

**Use the query engine from Python** — the same engine is importable
as `f5q` so external scripts can drive queries, build them up
progressively, render results through plugins or inline callables,
and ship reusable extensions via one-line decorators:

```python
from f5q import q, load, renderer, builtin, input_format

# One-liner: q() takes (expression, *inputs).
for name in q(".ltm.virtual[] | .name", "bigip.conf"):
    print(name)

# Progressive — chain queries on top of each other (typed wrapper, immutable).
filtered = q(".ltm.virtual[]", "bigip.conf").q(".[] | select(.pool != null)").q(".[] | .name")

# Render via a registered plugin OR an inline callable.
filtered.render("ascii-blocks")
filtered.render(lambda values, **opts: ", ".join(map(str, values)) + "\n")

# Coerce to plain JSON-friendly Python.
data = filtered.out()  # [{"kind": ..., "fields": {...}}, ...]

# Pre-stage once, query many times. Custom file formats? Pass an inline parser.
corpus = load("ltm.conf", "gtm.conf")
routes = load("routes.xml", parser=my_xml_parser)


# Ship a custom renderer the f5 CLI can dispatch via --render NAME.
@renderer("md-table", summary="Markdown table of results.", accepts="any")
def _render(values, **opts):
    return "| name |\n| ---- |\n" + "\n".join(f"| {v} |" for v in values)


# Ship a custom DSL function the query language can call.
@builtin("uppercase", summary="ASCII uppercase.", min_args=1, max_args=1)
def _u(s):
    return str(s).upper()


# Ship a custom side-input format `--input KIND NAME=PATH` can load.
@input_format("yaml", summary="YAML side-input.")
def _parse_yaml(source, *, uri, options=()):
    import yaml

    return yaml.safe_load(source)
```

**Auto-discovered plugins** — drop any of the above into
`$XDG_CONFIG_HOME/dialects/f5/query/plugins/*.py` (default
`~/.config/dialects/f5/query/plugins/*.py`) and the engine picks them up on the
first registry access, no import dance required.  Broken plugins
warn to stderr and are skipped; `f5 q --help-plugins` shows what
loaded.

**Documentation**:

- **Python API reference** — autodoc-generated, every public
  symbol with full signature, docstring, and `[source]` links.
  Build locally with `make docs-html` (output at
  `docs/sphinx/_build/html/index.html`); the same Sphinx tree
  builds on Read the Docs via [`.readthedocs.yaml`](.readthedocs.yaml).
- [KCS: how-to — script against `f5 query` from Python](docs/kcs/kcs-howto-script-against-f5-query-from-python.md)
  — task-oriented walkthrough.
- [KCS: feature — `f5 query` plugins](docs/kcs/features/kcs-feature-f5-query-renderers.md)
  — built-in plugin catalogue and CLI flag reference.
- [Design — `f5 query` plugin contract](docs/design/f5-query-renderer-contract.md)
  — formal contracts, registration lifecycle, error mapping.

**Install the `f5` CLI** — the released artefact is a single-file
zipapp (`f5-<version>.pyz`) that needs only Python 3.10+ on the host.
See [INSTALL-cli.md](INSTALL-cli.md) for the one-line `curl | sh`
installer, manual install steps for macOS/Debian/Ubuntu/RHEL/CentOS/
Fedora, shell completion setup, and source-build instructions.

In VS Code, run the command palette entry **Tcl: Generate BIG-IP
Cleanup Script** while a `bigip.conf` is open; the script and its JSON
metadata report open side-by-side.  See
[KCS: feature — BIG-IP Config Cleanup](docs/kcs/features/kcs-feature-bigip-cleanup.md)
for the full options reference.

### APL (iApp Presentation Language)

Open `.apl` files or files named `presentation` to get semantic highlighting
for the iApp Application Presentation Language.  APL-specific tokens include
section/table/row keywords, field types (`string`, `choice`, `password`, ...),
attributes (`default`, `display`, `required`, `validator`), `define` blocks,
`optional` conditionals, `#include`/`#inline` directives, and validator names.
Embedded Tcl inside `[...]` brackets (e.g. `[tmsh::get_config ...]`) receives
full Tcl semantic highlighting.

```
# iApp APL presentation file
section basic {
    string addr default "0.0.0.0" required validator "IpAddress"
    choice protocol display "medium" default "tcp" {
        "TCP" => "tcp",
        "UDP" => "udp"
    }
    yesno use_snat default "yes"
}
text {
    basic "Basic Configuration"
    basic.addr "Virtual Server IP Address"
}
```

**Cross-file integration:** When a `presentation` (APL) file and an
`implementation` (iApp Tcl) file are in the same directory, the server
cross-validates them:

- **IAPP7001**: Implementation references a variable (`$::section__field`) not
  defined in the presentation
- **IAPP7002**: Presentation field is never referenced in the implementation
- **IAPP7003**: `#include` file not found

The `#include` directive is resolved relative to the APL file's directory,
with recursive resolution and circular-include protection.

### tmsh commands

The `f5-iapps` dialect includes 30+ `tmsh::` namespace commands
(`tmsh::create`, `tmsh::modify`, `tmsh::get_config`, `tmsh::get_field_value`,
etc.) and 4 `script::` commands (`script::run`, `script::init`, etc.) with
hover documentation and arity validation.

### iRules-to-XC migration

Translate F5 BIG-IP iRules to F5 Distributed Cloud configuration, with both
Terraform HCL and JSON API output plus a coverage report.

```tcl
# Source iRule:
when HTTP_REQUEST {
    if { [HTTP::uri] starts_with "/api" } {
        pool api_pool
    } else {
        HTTP::redirect "https://[HTTP::host]/api[HTTP::uri]"
    }
}

# "Translate iRule to F5 XC" produces:
# - Terraform HCL with routes, origin pools, and redirect rules
# - JSON API payload for direct XC API calls
# - Coverage report showing translated vs. untranslatable constructs
```

### iRule Event Orchestrator (test framework)

Generate and run deterministic tests for F5 iRules.  The framework simulates
BIG-IP's event lifecycle, pool selection, data groups, and multi-TMM CMP
behaviour in a standard `tclsh`.

```tcl
::orch::configure_tests \
    -profiles {TCP HTTP} \
    -irule { when HTTP_REQUEST { pool web_pool } } \
    -setup { ::orch::add_pool web_pool {{10.0.0.1:80}} }

::orch::test "routing-1.0" "basic request goes to web_pool" -body {
    ::orch::run_http_request -host "example.com" -uri "/"
    ::orch::assert_that pool_selected equals "web_pool"
}

exit [::orch::done]
```

The `generate-test` CLI command and `generate_irule_test` MCP tool analyse an
iRule's control-flow graph to produce test cases automatically.  For iRules
with CMP-sensitive patterns (`static::` writes in hot events, `table` shared
state), multi-TMM scenarios using fakeCMP distribution are included.

### Runtime validation

Optionally run the active file through a real `tclsh` (or an iRules stub
adapter) on save to catch issues that static analysis alone cannot detect.

```tcl
# With tclLsp.runtimeValidation.enabled = true:
proc test {} {
    package require NoSuchPackage   ;# runtime error: can't find package
}
# The server invokes tclsh in syntax-check mode and merges runtime
# errors into the diagnostics panel alongside static analysis results
```

### Text encoding tools

Editor commands for common encoding operations, available from the right-click
context menu or the command palette.

```
Escape Selection          →  converts special chars to Tcl backslash sequences
Unescape Selection        →  reverses backslash sequences to literal chars
Base64 Encode Selection   →  encodes selected text as base64
Base64 Decode Selection   →  decodes base64 back to text
Copy File as Base64       →  copies entire file content as base64 to clipboard
Copy File as Gzip+Base64  →  compresses then base64-encodes file to clipboard
```

### Package scaffolding

Generate a complete Tcl package project layout with a single command.

```
"Tcl: Scaffold Tcl Package Starter" creates:

  mypackage/
    pkgIndex.tcl          Package index
    mypackage.tcl         Package source with namespace and public API
    tests/
      all.tcl             Test runner
      mypackage.test      tcltest skeleton
    .github/
      workflows/ci.yml    GitHub Actions CI workflow
    README.md             Package README
```

## AI integrations

### Chat participants (VS Code + GitHub Copilot)

Three chat participants integrate with GitHub Copilot to provide
domain-specific AI assistance backed by the LSP's static analysis.

#### `@irule` — iRules assistant

| Command | Description |
|---------|-------------|
| `/create` | Generate a new iRule from a natural-language description |
| `/explain` | Explain what an iRule does, including data flow and security |
| `/fix` | Iteratively fix all LSP diagnostics in the current iRule |
| `/validate` | Run full LSP validation and show a categorised report |
| `/review` | Deep security and safety review (injection, DoS, races) |
| `/find-legacy` | Find and modernise legacy patterns (unbraced expr, matchclass, etc.) |
| `/optimise` | Apply optimiser suggestions with explanations |
| `/scaffold` | Generate an iRule skeleton from selected events |
| `/datagroup` | Suggest data-group extraction for inline lookups |
| `/diff` | Explain differences between two iRule versions |
| `/event` | Show which commands are valid in a given event |
| `/migrate` | Convert nginx/Apache/HAProxy config to an iRule |
| `/diagram` | Generate a Mermaid flowchart of the iRule's logic flow |
| `/xc` | Translate the iRule to F5 Distributed Cloud configuration |

```
User:   @irule /create rate limiter that allows 100 requests per minute per client IP
Copilot: generates a complete iRule with HTTP_REQUEST handler, table-based
         counting, and HTTP::respond 429 — validated against the LSP
```

![AI — create iRule](docs/screenshots/26-ai-create.png)

![AI — explain iRule](docs/screenshots/27-ai-explain.png)

![AI — diagram iRule](docs/screenshots/28-ai-diagram.png)

#### `@tcl` — Tcl assistant

| Command | Description |
|---------|-------------|
| `/create` | Generate Tcl code from a description |
| `/explain` | Explain what Tcl code does |
| `/fix` | Iteratively fix all LSP diagnostics |
| `/validate` | Run full LSP validation and show a report |
| `/optimise` | Apply optimiser suggestions with explanations |

```
User:   @tcl /explain what does the fibonacci proc do?
Copilot: walks through the loop, variable assignments, and return value
```

#### `@tk` — Tk GUI assistant

| Command | Description |
|---------|-------------|
| `/create` | Generate a Tk GUI from a description |
| `/explain` | Explain the widget hierarchy and layout |
| `/preview` | Open the Tk Preview pane for the current file |

```
User:   @tk /create a simple calculator with number buttons and a display
Copilot: generates Tk code with grid layout, button callbacks, and display label
```

### Claude Code skills

Twenty purpose-built skills for Claude Code (CLI) that combine LSP static
analysis with AI reasoning.  Each skill invokes the `tcl-lsp-ai` analyser,
iterates on diagnostics, and produces clean output.

| Skill | Description |
|-------|-------------|
| `irule-create` | Generate a new iRule from a description, validate until clean |
| `irule-explain` | Explain an iRule's logic, data flow, and security posture |
| `irule-fix` | Iteratively fix all diagnostics (analyse → fix → re-analyse) |
| `irule-validate` | Categorised validation report (errors, security, style, optimiser) |
| `irule-review` | Deep security audit: injection, DoS, races, information leakage |
| `irule-convert` | Modernise legacy patterns to current best practices |
| `irule-optimise` | Apply optimiser suggestions with safety explanations |
| `irule-scaffold` | Generate event skeleton with log gating and placeholders |
| `irule-datagroup` | Suggest data-group extraction for inline lookups |
| `irule-diff` | Explain semantic differences between two iRule versions |
| `irule-event` | Look up event/command validity from the registry |
| `irule-migrate` | Convert nginx/Apache/HAProxy config to an iRule |
| `irule-diagram` | Generate a Mermaid flowchart from compiler IR |
| `irule-xc` | Translate to F5 XC with Terraform and JSON output |
| `tcl-create` | Generate Tcl code from a description, validate until clean |
| `tcl-explain` | Explain Tcl code with analysis context |
| `tcl-fix` | Iteratively fix all Tcl diagnostics |
| `tcl-validate` | Categorised Tcl validation report |
| `tcl-optimise` | Apply Tcl optimiser suggestions |
| `tk-create` | Generate Tk GUI code with proper widget hierarchy |

```sh
# Example: fix all issues in an iRule
claude /irule-fix my_irule.tcl

# Example: security review
claude /irule-review production_rule.tcl

# Example: generate a Mermaid diagram
claude /irule-diagram complex_rule.tcl
```

### MCP server (Claude Desktop / AI agents)

A Model Context Protocol server that exposes tcl-lsp analysis as 27 tools for
any MCP-compatible client (Claude Desktop, custom agents, etc.).

| Tool | Description |
|------|-------------|
| `analyze` | Full analysis: diagnostics, symbols, events, and metadata |
| `validate` | Categorised validation report |
| `review` | Security-focused diagnostic report |
| `find-legacy` | Detect legacy patterns eligible for modernisation |
| `optimize` | Optimisation suggestions with rewritten source |
| `hover` | Hover information at a position |
| `complete` | Completions at a position |
| `goto_definition` | Find definition of a symbol |
| `find_references` | Find all references to a symbol |
| `symbols` | Document symbol hierarchy |
| `code_actions` | Quick fixes for a source range |
| `format_source` | Format Tcl/iRules source code |
| `rename` | Rename a symbol throughout the document |
| `event_info` | iRules event metadata and valid commands |
| `command_info` | Command metadata and valid events |
| `event_order` | Events in canonical firing order |
| `call_graph` | Build proc call graph with roots and leaves |
| `symbol_graph` | Build scope/definition/reference graph |
| `dataflow_graph` | Build taint and side-effect graph |
| `diagram` | Extract control-flow diagram data from IR |
| `xc_translate` | Translate iRule to XC configuration |
| `tk_layout` | Extract Tk widget tree as JSON |
| `generate_irule_test` | Generate iRule test script with CFG paths and multi-TMM detection |
| `irule_cfg_paths` | Extract CFG control-flow paths for test planning |
| `fakecmp_which_tmm` | Look up which TMM a connection tuple maps to |
| `fakecmp_suggest_sources` | Find client addr/port combos that hit each TMM |
| `set_dialect` | Set active Tcl dialect for the session |

```json
// Claude Desktop — claude_desktop_config.json
{
  "mcpServers": {
    "tcl-lsp": {
      "command": "./tcl-lsp-mcp-server.pyz"
    }
  }
}
```

## Packaging & environments

`tcl pkg` is a deterministic Tcl package manager using Go-style Minimum
Version Selection and a content-addressable SHA-256 cache.  `tcl venv` creates
isolated virtual environments that pin a specific tclsh version.

```sh
# Quick start
tcl venv create .venv            # create a virtual environment
source .venv/bin/activate        # activate it
tcl pkg init                     # create tclpkg.tcl manifest
tcl pkg add json 1.0             # add a dependency
tcl pkg install                  # resolve, fetch, and lock
tcl pkg tree                     # show dependency tree
tcl pkg verify                   # check integrity hashes
```

The manifest is a native Tcl file (`tclpkg.tcl`) evaluated in a sandboxed
interpreter.  The lockfile (`tclpkg.lock`) is canonical JSON — two runs against
the same manifest produce byte-identical output (aside from the
`generated` timestamp, which `--frozen` preserves).

```tcl
# tclpkg.tcl — example manifest
package     myapp
version     1.0.0
license     MIT
tcl         >=8.6

require json    1.3.5
require http    2.9.8
dev-require tcltest 2.5.5
```

The LSP server auto-detects `tclpkg.tcl` projects and venv `lib/` directories,
and offers an "Install via tclpkg" quick-fix on missing-package diagnostics.

See [docs/kcs/kcs-tclpkg-overview.md](docs/kcs/kcs-tclpkg-overview.md) for the
full architecture and contracts.

## CLI tools

All CLI tools are distributed as self-contained Python zipapps (`.pyz`) — no
`pip install` required.

### Unified Tcl tool zipapp (`tcl`)

A single verb-based CLI that aggregates common local workflows:

- `opt` / `optimise` — optimise combined input source and emit rewritten Tcl
- `diag` — run diagnostics across files/directories/packages
- `lint` — run lint diagnostics across files/directories/packages
- `validate` — error-level validation checks
- `format` — format source using canonical Tcl style rules
- `symbols` — emit symbol definitions for the resolved source
- `diagram` — extract control-flow diagram data from compiler IR
- `callgraph` — build procedure call graph data
- `symbolgraph` — build symbol relationship graph data
- `dataflow` — build taint/effect data-flow graph data
- `command-info` — look up command registry metadata
- `find-legacy` — detect legacy modernisation patterns (detection only)
- `dis` — bytecode disassembly
- `compwasm` — compile input to a WASM binary
- `highlight` — emit syntax-highlighted source (`ansi` or `html`)
- `diff` — compare two sources across AST/IR/CFG compiler representations
- `explore` — run compiler-explorer views (`ir`, `cfg`, `ssa`, `opt`, `asm`, `wasm`, ...)
- `help` — search bundled KCS feature docs from the SQLite help index
- `pkg` — package management: `init`, `add`, `remove`, `install`, `list`, `tree`, `verify`, `info`, `search`, `update`, `sync`, `outdated`, `why`, `vendor`, `run`
- `venv` — virtual environments: `create`, `delete`, `info`, `activate`, `deactivate`, `list`, `update`, `run`

```sh
# Optimise everything under src/ into one output script
python tcl.pyz opt src/ -o build/optimised.tcl

# Run diagnostics across a directory and a Tcl package
python tcl.pyz diag src/ mypkg --package-path ./vendor/tcl

# Run lint diagnostics (same checks as `diag`)
python tcl.pyz lint src/ mypkg --package-path ./vendor/tcl

# Validate syntax/error diagnostics
python tcl.pyz validate src/

# Validate as JSON
python tcl.pyz validate src/ --json

# Format source text
python tcl.pyz format script.tcl -o formatted.tcl

# Minify source (strip comments, collapse whitespace, join commands)
python tcl.pyz minify script.tcl -o minified.tcl

# Aggressive minify (optimise + static substring folding via SCCP + name compaction)
python tcl.pyz minify --aggressive script.tcl -o minified.tcl --symbol-map map.txt

# Symbol/graph/find-legacy analysis verbs
python tcl.pyz symbols script.tcl --json
python tcl.pyz diagram script.tcl --json
python tcl.pyz callgraph script.tcl --json
python tcl.pyz symbolgraph script.tcl --json
python tcl.pyz dataflow script.tcl --json
python tcl.pyz command-info HTTP::uri --dialect f5-irules --json
python tcl.pyz find-legacy rule.irule --json

# iRules-specific lookups live on the f5 CLI:
python f5.pyz irule event-order rule.irule --json
python f5.pyz irule event-info HTTP_REQUEST --json

# Emit bytecode disassembly
python tcl.pyz dis script.tcl

# Compile to WASM binary (+ optional WAT sidecar)
python tcl.pyz compwasm script.tcl -o out.wasm --wat-output out.wat

# Emit ANSI-highlighted output (or --format html)
python tcl.pyz highlight script.tcl --force-colour

# Diff two iRules using compiler structure layers
python tcl.pyz diff old.irule new.irule --show ast,ir,cfg

# Use compiler explorer views from the same zipapp
python tcl.pyz explore script.tcl --show ir,cfg,opt

# Search KCS help docs (optionally scoped by dialect)
python tcl.pyz help taint analysis --dialect f5-irules

# Show help for the help command itself
python tcl.pyz help --help

# Emit help search results as JSON
python tcl.pyz help taint --json
```

For iRules input, pass `--dialect f5-irules` explicitly:

```sh
tcl.pyz lint rules/ --dialect f5-irules
```

iRules-specific verbs (`event-order`, `event-info`) live on the separate
`f5` CLI under the `irule` verb group — see the F5 BIG-IP CLI section.

For source builds, run `make kcs-db` before packaging zipapps so `tcl.pyz help`
can query the bundled KCS SQLite database.

**Install the `tcl` CLI** — the released artefact is a single-file
zipapp (`tcl-<version>.pyz`) that needs only Python 3.10+ on the host.
See [INSTALL-cli.md](INSTALL-cli.md) for the one-line `curl | sh`
installer, manual install steps for macOS/Debian/Ubuntu/RHEL/CentOS/
Fedora, source builds, and shell completion (`bash`, `zsh`, `fish`)
that covers every verb, dialect, optimiser profile, and source-path
glob (`*.tcl`, `*.tk`, `*.itcl`, `*.tm`, `*.irul`, `*.irule`,
`*.iapp`, `*.iappimpl`).

![Unified Tcl verb CLI](docs/screenshots/30-tcl-verb-cli.png)

### Compiler explorer (CLI)

Console tool for inspecting the compiler pipeline: IR, CFG, SSA, optimiser
rewrites, shimmer warnings, taint analysis, and bytecode.

```sh
# Full exploration of a Tcl file
uv run python -m tooling.explorer script.tcl

# Focus on optimiser rewrites only
uv run python -m tooling.explorer script.tcl --show opt

# Inline source with optimised output
uv run python -m tooling.explorer --source 'set a 1; set b [expr {$a + 2}]' --show-optimised-source

# Show only IR and CFG
uv run python -m tooling.explorer script.tcl --show ir,cfg

# iRules dialect with flow analysis
uv run python -m tooling.explorer irule.tcl --dialect bigip --show irules
```

Available views: `ir`, `cfg`, `ssa`, `interproc`, `types`, `opt`, `gvn`,
`shimmer`, `taint`, `irules`, `callouts`, `asm`, `wasm`.  Groups: `all`,
`compiler`, `optimiser`.

### AI analysis tool (CLI)

Standalone static analyser for use with AI agents and CI pipelines.

```sh
# Full context pack (diagnostics + symbols + events) as JSON
uv run python -m ai.claude.tcl_ai context script.tcl

# Categorised validation report
uv run python -m ai.claude.tcl_ai validate script.tcl

# Security-focused review
uv run python -m ai.claude.tcl_ai review irule.tcl

# Optimisation suggestions with rewritten source
uv run python -m ai.claude.tcl_ai optimize script.tcl

# Build call graph
uv run python -m ai.claude.tcl_ai call-graph script.tcl

# Look up iRules event metadata
uv run python -m ai.claude.tcl_ai event-info HTTP_REQUEST

# Extract Tk widget tree
uv run python -m ai.claude.tcl_ai tk-layout gui.tcl

# Generate iRule test script (Event Orchestrator framework)
uv run python -m ai.claude.tcl_ai generate-test irule.tcl

# Extract CFG paths for test planning
uv run python -m ai.claude.tcl_ai cfg-paths irule.tcl
```

### Tcl-to-WASM compiler

Compile Tcl scripts to WebAssembly (WAT text or binary WASM format).

```sh
# Compile to human-readable WAT
uv run python -m tooling.wasm.main script.tcl --format wat

# Compile to WASM binary with optimisations
uv run python -m tooling.wasm.main script.tcl -O --format wasm -o out.wasm

# Compare optimised vs. unoptimised output
uv run python -m tooling.wasm.main --source 'set x [expr {1+2}]' --format both
```

### Compiler explorer (web GUI)

A standalone web UI for the compiler explorer, available in two variants:
offline (bundles Pyodide) and CDN (loads Pyodide from jsDelivr).

```sh
# Standalone (offline, ~100 MB)
./tcl-lsp-explorer-gui.pyz --port 8080

# CDN variant (lightweight, requires internet)
./tcl-lsp-explorer-gui-cdn.pyz --port 8080
```

### Tcl VM

A bytecode interpreter that compiles and executes Tcl scripts using the
compiler pipeline, with an interactive REPL and disassembly mode.  Supports
TclOO classes (constructors, destructors, methods, mixins, filters, private
variables), namespaces, coroutine-free control flow, and 85% conformance
against Tcl 9.0.3 native test suites.

```sh
# Execute a script
uv run python -m tooling.vm script.tcl arg1 arg2

# Interactive REPL
uv run python -m tooling.vm

# Inline evaluation
uv run python -m tooling.vm -e 'puts [expr {6 * 7}]'

# Show bytecode disassembly without executing
uv run python -m tooling.vm --disassemble script.tcl
```

### Tcl debugger

An interactive debugger that can single-step through Tcl scripts with
breakpoints, variable inspection, and call stack visualisation.  Three
backends are available:

| Backend | Description |
|---------|-------------|
| `vm` | The project's own bytecode VM (default) |
| `tclsh` | External `tclsh` subprocess |
| `tkinter` | Python's built-in `tkinter.Tcl()` interpreter |

```sh
# Debug a script (uses VM backend by default)
uv run python -m debugger script.tcl

# Force a specific backend
uv run python -m debugger --backend vm script.tcl

# Read from stdin
echo 'puts hello' | uv run python -m debugger -
```

Debugger commands: `run`, `step`/`s`, `next`/`n`, `finish`, `continue`/`c`,
`break <line>`/`b`, `delete <id>`/`d`, `vars`, `print <var>`/`p`, `stack`,
`list`/`l`, `quit`/`q`.

## Screenshots

### Diagnostics & quick fixes

![Diagnostics overview](docs/screenshots/01-diagnostics-overview.png)

![Quick fix](docs/screenshots/04-quickfix.png)

### Hover & completions

![Hover](docs/screenshots/02-hover-proc.png)

![Completions](docs/screenshots/03-completions.png)

### Security taint analysis

![Taint analysis](docs/screenshots/05-security-taint.png)

### Semantic highlighting

![Semantic highlighting](docs/screenshots/09-semantic-highlighting.png)

## Dialect support

The server ships a registry of command signatures, argument roles, and
validation rules keyed by dialect.  Switching the dialect profile changes
which commands are known, which are deprecated, and which event/layer
constraints apply.

### Automatic dialect detection

The dialect is selected automatically using the following priority chain
(highest to lowest):

1. **Editor language ID** -- opening a file as `tcl-irule`, `tcl8.4`, etc.
   selects the matching dialect immediately.
2. **File extension** -- `.irul`/`.irule` → `f5-irules`,
   `.iapp`/`.iappimpl`/`.impl` → `f5-iapps`, `.exp` → `expect`.
3. **Comment directive** -- a `# tcl-dialect: <dialect>` comment in the
   first 5 lines of a file pins the dialect for that file:

   ```tcl
   # tcl-dialect: tcl8.4
   set x 1
   ```
4. **Shebang** -- `#!/usr/bin/env tclsh8.5` selects `tcl8.5`;
   `#!/usr/bin/expect` selects `expect`.
5. **User setting** -- the `tclLsp.dialect` configuration value acts as the
   default for files that have no per-file hint.
6. **Hardcoded fallback** -- `tcl8.6` when nothing else matches.

Per-file hints (directive, shebang, extension) always take priority over
the global setting, so different files in the same workspace can target
different Tcl versions without manual switching.

| Dialect | Description |
|---------|-------------|
| `tcl8.4` | Tcl 8.4 core commands |
| `tcl8.5` | Tcl 8.5 core commands (adds `{*}`, `lassign`, `dict`, etc.) |
| `tcl8.6` | Tcl 8.6 core commands (adds `try`/`finally`, `tailcall`, coroutines) -- **default** |
| `tcl9.0` | Tcl 9.0 core commands (adds `lpop`, zipfs, updated `encoding`) |
| `tcl9.1` | Tcl 9.1 core commands (superset of 9.0; adds the `unicode` and `timer` ensembles and `subst`'s positive `-backslashes`/`-commands`/`-variables` options) |
| `f5-irules` | F5 BIG-IP iRules: HTTP/SSL/DNS/LB namespaces, event-validity checks, taint analysis, `static::` scoping rules |
| `f5-iapps` | F5 iApps template commands |
| `f5-bigip` | F5 BIG-IP configuration (`bigip.conf`) commands |
| `synopsys-eda-tcl` | Synopsys EDA commands (Design Compiler, PrimeTime, ICC2, Formality) |
| `cadence-eda-tcl` | Cadence EDA commands (Genus, Innovus, Tempus, Xcelium) |
| `xilinx-eda-tcl` | Xilinx/AMD EDA commands (Vivado, Vitis) |
| `intel-quartus-eda-tcl` | Intel Quartus Prime commands |
| `mentor-eda-tcl` | Mentor/Siemens EDA commands (ModelSim, Questa, Calibre) |
| `expect` | Expect: `spawn`, `expect`, `send`, `interact` and related commands for automating interactive programs |

**Tk**, **tcllib**, and **Tcl stdlib** commands are automatically recognised
when the corresponding `package require` appears in the file.  No manual
toggle is needed — the registry activates the relevant command definitions
per-document.

### Dialect command stubs

For commands that the LSP does not know about (custom extensions, vendor
tools, internal frameworks), you can declare stubs so the LSP understands
their signatures.  Two mechanisms are supported:

**External stub files** (`<name>.tcl.stubs`):

```
# synopsys.tcl.stubs
stub foreach_in_collection {varName:var collection body:body} -loop
stub get_cells {?-hierarchical? ?-filter? pattern:pattern} -pure
stub sizeof_collection {collection} -pure
stub expr-func sizeof 1
```

**Inline stubs** (in any `.tcl` file, using markers):

```tcl
# tcl-lsp: stubs-begin
# tcl-lsp: stub foreach_in_collection {varName:var collection body:body} -loop
# tcl-lsp: stub get_cells {pattern:pattern} -pure
# tcl-lsp: stub expr-func sizeof 1
# tcl-lsp: stub expr-op contains 2
# tcl-lsp: stubs-end
```

Multiple stubs blocks per file are supported.  Argument roles include
`body`, `expr`, `var`, `var_read`, `name`, `pattern`, `channel`, and
`value` (default).  Flags include `-barrier`, `-loop`, `-pure`,
`-mutator`, `-unsafe`, and `-scope_alias`.

Expression stubs declare custom math functions (`expr-func`) and infix
operators (`expr-op`) with optional arity.

See [KCS: Dialect stubs](docs/kcs/kcs-dialect-stubs.md) for full syntax.

### Command alias resolution

When `interp alias {} name {} target ?args?` creates a command alias in the
current interpreter, the LSP automatically inherits the target command's
argument semantics.  This means expression arguments, body arguments, variable
names, and patterns are all correctly analysed through the alias:

```tcl
interp alias {} = {} expr
proc calculate {x y} {
    set result [= {$x + $y}]   ;# $x and $y recognised as reads — no W214
    return $result
}
```

Alias information is also used by LSP features: **hover** shows the target
command's documentation, **completion** offers aliases as candidates,
**go-to-definition** follows aliases to the target proc, and
**signature help** shows the target's parameter hints.

See [KCS: Command alias resolution](docs/kcs/kcs-command-alias-resolution.md)
for details.

### Proc argument trait inference

The analyser automatically infers how each proc parameter is used inside
the proc body, producing structured trait annotations:

| Trait | Detected pattern |
|-------|-----------------|
| `EVAL` | `eval $param`, `uplevel 1 $param` |
| `BODY` | `foreach item $list $param` |
| `VAR_WRITE` | `upvar 1 $param local; set local 42` |
| `VAR_READ` | `upvar 1 $param local; return $local` |
| `EXPR` | `if {$param} {...}` |
| `LOOP_LIST` | `foreach item $param {...}` |

Two analysis tiers: a fast shallow pass (synchronous, top-level commands)
and a deep pass (asynchronous, recursive descent into nested bodies).
Traits feed optimisation, shimmer analysis, taint propagation, and
diagnostics.

See [KCS: Proc arg traits](docs/kcs/kcs-proc-arg-traits.md) for details.

## Authoring workflows (VS Code commands)

- `Tcl: Insert Tcl Template Snippet` -- quick-pick and insert any bundled Tcl/iRules snippet template.
- `Tcl: Insert iRule Event Skeleton` -- scaffold selected iRules events into a new Tcl buffer.
- `Tcl: Scaffold Tcl Package Starter` -- generate package layout, tests, CI workflow, and README.
- `Tcl: Insert package require` -- suggest and insert `package require` lines based on symbol usage.
- `Tcl: Apply Safe Quick Fixes` -- apply all non-overlapping safe quick fixes in one pass.
- `Tcl: Run Runtime Validation` -- run dialect-aware runtime checks on demand.

## Code formatting

The formatter supports full-document and range formatting via the standard LSP
`textDocument/formatting` and `textDocument/rangeFormatting` requests.  Defaults
follow the [F5 iRules Style Guide](https://community.f5.com/kb/technicalarticles/irules-style-guide/305921).

![Formatting side-by-side (before left, after right)](docs/screenshots/07-formatting-after.png)

Capabilities include:

- **Indentation** -- configurable size, spaces or tabs, with separate continuation indent
- **Brace placement** -- K&R (end of line) style
- **Expression bracing** -- optionally enforce `expr {$x + 1}` instead of `expr $x + 1`
- **Variable bracing** -- optionally rewrite `$var` as `${var}`
- **Line length** -- hard limit and soft goal; long lines are wrapped at continuation points
- **Semicolons** -- convert `;`-separated commands to individual lines
- **Body expansion** -- optionally expand single-line `if`/`foreach`/etc. bodies to multi-line
- **Blank lines** -- normalise spacing between procs, between control-flow blocks, and cap consecutive blank lines
- **Comments** -- ensure space after `#`, align inline comments to a consistent column
- **Whitespace** -- trim trailing whitespace, ensure final newline, normalise line endings (LF/CRLF/CR)
- **Docstrings** -- configurable style (preceding or body-internal), doxygen or plain tag format, optional decoration borders

The formatter also recognises multi-line docstrings with `@param`, `@return`,
and `@brief` tags (doxygen-style) and displays them as structured hover
information.  Body-internal docstrings (comment blocks at the start of a proc
body) are supported as a fallback when no preceding comment exists.

All options are exposed through `tclLsp.formatting.*` settings (see
[Configuration](#formatter-settings) below).

## Suppressing diagnostics

Diagnostics can be suppressed at five different scopes.  Smaller scope is
always better — turning a code off globally hides real problems in future
projects.

| Scope | How |
|-------|-----|
| One command | `# noqa: CODE` on the line before the command |
| One file | `# tcl-lsp: disable=CODE,CODE` near the top of the file |
| One project | `[diagnostics]\ndisabled = CODE` in `.tcl-lsp.ini` at the workspace root |
| One editor | `tclLsp.diagnostics.CODE: false` in editor settings |
| Everywhere | `[diagnostics]\ndisabled = CODE` in the [global config file](#configuration-file) |

**Inline** — put on the line *before* the command:

```tcl
# noqa: W100
expr $x + 1

# noqa: *
eval $user_input
```

**Top-of-file** — before the first non-comment line:

```tcl
#!/usr/bin/env tclsh
# tcl-lsp: disable=W100,O111
```

**Project config** — `.tcl-lsp.ini` at the workspace root (commit with source):

```ini
[diagnostics]
disabled = W111, IRULE1005

[optimiser]
disabled = O109
```

For the complete reference, see
[`docs/kcs/kcs-howto-suppress-diagnostics.md`](docs/kcs/kcs-howto-suppress-diagnostics.md).

## Diagnostic codes

### Errors

| Code | Description | Quick-fix |
|------|-------------|-----------|
| E001 | Missing required subcommand | |
| E002 | Too few arguments | |
| E003 | Too many arguments | |
| E100 | Unmatched `]` -- missing opening `[` | Insert `[` |
| E101 | Missing `{` after `switch` -- body cases follow without braces | |
| E102 | Unmatched `}` -- missing opening `{` | Remove stray `}` |
| E103 | Missing `}` -- a nested body consumed this closing brace | |
| E200 | Parse error -- internal representation cannot be determined | |

### Warnings -- Style & Best Practice

![Style warnings](docs/screenshots/08-style-warnings.png)

| Code | Description | Quick-fix |
|------|-------------|-----------|
| W001 | Unknown subcommand | |
| W002 | Command is disabled in active dialect profile | |
| W003 | Expression operator not available in the active dialect | |
| W004 | Command option not available in the active dialect | |
| W100 | Unbraced `expr`/`if`/`while`/`for` expression (double substitution risk) | Wrap in braces |
| W104 | `append` with space-separated values (use `lappend` for lists) | |
| W105 | Unbraced code block or missing `variable` declaration in `namespace eval` | Wrap in braces |
| W106 | Dangerous unbraced `switch` body | |
| W108 | Non-ASCII characters in token content (smart quotes, non-breaking spaces) | Replace with ASCII |
| W110 | `==`/`!=` on strings in `expr` (use `eq`/`ne`) | Replace operator |
| W111 | Line exceeds configured maximum length | |
| W112 | Trailing whitespace | Remove whitespace |
| W113 | Procedure shadows a built-in command | |
| W114 | Redundant nested `[expr]` -- already in expression context | |
| W115 | Backslash-newline in comment silently swallows the next line | Convert to per-line comments |
| W116 | Stub command shadows a built-in command | |
| W117 | Stub expression definition shadows a built-in function or operator | |
| W118 | Inconsistent line endings | |
| W120 | Package-gated command used without `package require` | Insert `package require` |
| W121 | Subnet mask has non-contiguous bits | Replace with nearest valid mask |
| W122 | Mistyped IPv4 address (octet > 255 or leading zero) | |
| W123 | Unknown command — not found in registry, user procs, or `unknown` handler (opt-in) | Replace with suggestion |
| W124 | Invalid IP address literal | |
| W125 | Orphaned control-flow keyword used as a standalone command | |
| W126 | Non-channel value in channel argument position | |
| W127 | Value not in the command's allowed set (e.g. `HTTP::version "2.0"`) | Use one of the listed values |
| W200 | Binary format modifier requires newer Tcl | |
| W201 | Manual path concatenation — uses rendered value properties and taint suppression (use `file join`) | Rewrite as `[file join]` |
| W230 | Constant list index out of range -- `lindex`/`lrange`/`lreplace` silently return empty or clamp | |
| W231 | Constant list index out of range -- `lset` raises a runtime error | |
| W232 | Constant string index out of range -- `string index`/`range`/`replace`/`insert` silently no-op | |
| W240 | Loop condition is constant false -- body never executes | |
| W241 | Loop is provably infinite -- constant-true condition with no `break`/`return` | |

### Warnings -- Variables

| Code | Description | Quick-fix |
|------|-------------|-----------|
| H300 | Possible paste error -- repeated assignment to same variable with same value | |
| W210 | Variable read before set (with case-mismatch suggestion when applicable; `info exists`/`array exists` are existence tests, not reads, so they are excluded and instead fold to a constant branch where provable) | |
| W211 | Variable set but never used (with case-mismatch suggestion when applicable) | |
| W212 | Variable substitution where name expected (`set $x`, `incr $x`, `info exists $x`, etc.) | |
| W213 | `unset` on variable that may not exist -- use `unset -nocomplain` | |
| W214 | Unused proc parameter -- argument declared but never read in the body | |
| W215 | Variable name unreachable via `$`-substitution (creatable, but no `$`-form can read it) | |
| W216 | Broken brace-form array element reference (`${arr}(x)` parses as scalar + literal) | |
| W220 | Dead store -- variable set but overwritten before use (with case-mismatch suggestion when applicable) | |

### Warnings -- Security

| Code | Description | Quick-fix |
|------|-------------|-----------|
| W101 | `eval` with substituted arguments (code injection risk) | |
| W102 | `subst` with a variable argument (template injection risk) | |
| W103 | `open` with pipeline or variable argument (command injection risk) | |
| W300 | `source` with a variable path (code execution risk) | |
| W301 | `uplevel` with unbraced or multi-arg script (injection risk) | |
| W303 | `regexp` with nested quantifiers (ReDoS risk) | |
| W304 | Missing `--` on option-bearing commands before positional input | Insert `--` |
| W306 | Substitution in literal-expected argument position | |
| W307 | Non-literal command name (variable or command substitution as command) | |
| W308 | `subst` without `-nocommands` | |
| W309 | `eval`/`uplevel` with `subst` -- double substitution risk | |
| W310 | Hardcoded credentials (API keys, tokens, passwords) | |
| W311 | Unsafe channel encoding mismatch (`-encoding binary` with `-translation`) | |
| W312 | `interp eval`/`interp invokehidden` with dynamic script (injection risk) | |
| W313 | Destructive `file` operations (`delete`/`rename`/`mkdir`) with variable path | |

### Warnings -- Packages

| Code | Description | Quick-fix |
|------|-------------|-----------|
| W130 | `tclpkg.tcl` requires a package not in `tclpkg.lock` | Run `tcl pkg install` |
| W131 | `tclpkg.lock` is out of sync with `tclpkg.tcl` | Run `tcl pkg install` |
| W132 | `tclpkg.lock` integrity mismatch -- CAS hash differs from lockfile | |
| W133 | `tclpkg.tcl` directive not permitted in safe mode | |
| W134 | Package resolved but no `pkgIndex.tcl` found -- `package require` will fail at runtime | |

### Hints

| Code | Description | Quick-fix |
|------|-------------|-----------|
| W242 | Loop termination cannot be proven -- counter not provably modified by the body or step | |
| W302 | `catch` without a result variable (silently swallows errors) | Add result variable |

### Shimmer detection

The shimmer analyser tracks each variable's Tcl internal representation
("intrep") through the SSA type lattice.  When a command expects a different
intrep than the variable currently holds, Tcl must destroy and recreate the
representation -- a "shimmer".  This is normally invisible but can be a
significant performance cost in loops.

| Code | Severity | Description |
|------|----------|-------------|
| S100 | Info | Single shimmer outside a loop |
| S101 | Warning | Shimmer inside a loop body (per-iteration cost) |
| S102 | Warning | Variable oscillates between two types across loop iterations (type thunking) |

### Taint analysis

The taint analyser tracks data provenance through the SSA graph using a
colour-aware lattice.  Values originating from I/O commands (network reads,
file reads, process execution) are tagged as tainted.  Taint propagates
through assignments, string interpolation, and phi nodes.  Commands that
produce fixed-type results (e.g. `string length`, `llength`) act as
sanitisers.

Taint colours carry value properties (e.g. `PATH_NORMALISED` for values
normalised via `file normalize`, `PATH_JOINED` for values assembled via
`file join`).  At join points, colours are intersected so only properties
shared by all paths survive -- this suppresses false positives.

The **Rendered Value Properties** pass (`compiler/rendered_properties.py`)
runs before taint propagation and computes per-SSA-value string content
properties after Tcl backslash substitution.  This enables precise detection
of path separators (resolving escape sequences like `\x2f` to `/` before
checking) and is used by the W201 path concatenation diagnostic.

| Code | Severity | Description | Quick-fix |
|------|----------|-------------|-----------|
| T100 | Warning | Tainted data flows into a dangerous code-execution sink | |
| T101 | Warning | Tainted data flows into an output command | |
| T102 | Warning | Tainted data in option position without `--` terminator | Insert `--` |
| T103 | Warning | Tainted data in `regexp`/`regsub` pattern (regex injection / ReDoS risk) | Wrap with `[regex::quote]` |
| T104 | Warning | Tainted data in network address argument (SSRF risk) | |
| T105 | Warning | Tainted data in `interp eval` script argument (cross-interpreter injection) | |
| T106 | Info | Double-encoding -- value already carries encoding colour | Remove redundant encoder |
### iRules codes

These diagnostics fire only in the `f5-irules` dialect.

#### Event validity & flow

| Code | Severity | Description | Quick-fix |
|------|----------|-------------|-----------|
| IRULE1001 | Warning/Hint | Command invalid or ineffective in this iRules event | |
| IRULE1002 | Warning | Unknown iRules event name | |
| IRULE1003 | Warning | Deprecated iRules event | |
| IRULE1004 | Hint | `when` block missing explicit `priority` | |
| IRULE1005 | Warning | `*_DATA` event handler without matching `*::collect` call | Bootstrap `collect` |
| IRULE1006 | Warning | `*::payload` access without matching `*::collect` call | Bootstrap `collect` |
| IRULE1007 | Error | `*::collect` without matching `*::release` on the same connection side | |
| IRULE1008 | Error | `*::release` without matching `*::collect` on the same connection side | |
| IRULE1201 | Warning | HTTP command used after `HTTP::respond`/`HTTP::redirect` | |
| IRULE1202 | Warning | Multiple `HTTP::respond`/`HTTP::redirect` on different branches | |

#### Deprecated & unsafe commands

| Code | Severity | Description | Quick-fix |
|------|----------|-------------|-----------|
| IRULE2001 | Warning | Deprecated `matchclass` -- use `class match` | Auto-replace |
| IRULE2002 | Warning | Deprecated iRules command | |
| IRULE2003 | Error | Unsafe iRules command (context escalation risk) | |

#### Taint & security

| Code | Severity | Description | Quick-fix |
|------|----------|-------------|-----------|
| IRULE3001 | Warning | Tainted data in HTTP response body (XSS risk) | Wrap with `[HTML::encode]` |
| IRULE3002 | Warning | Tainted data in HTTP header or cookie value (header injection) | Wrap with `[URI::encode]` |
| IRULE3003 | Warning | Tainted data in `log` command (log injection) | |
| IRULE3004 | Warning | Tainted data in `HTTP::redirect` URL (open redirect risk) | |
| IRULE3101 | Warning | `HTTP::uri`/`HTTP::path` set to value not provably starting with `/` | |
| IRULE3102 | Warning | `HTTP::path`/`HTTP::uri`/`HTTP::query` getter used without `-normalized` | |
| IRULE3103 | Info | `*::uri` used where `*::path` or `*::query` suffices (`split`, `starts_with`, `contains`, `string match`, etc.) | |

#### Scoping & state

| Code | Severity | Description |
|------|----------|-------------|
| IRULE4001 | Warning | Write to `static::` variable outside `RULE_INIT` (race condition) |
| IRULE4002 | Hint | Generic `static::` variable name — collision likely across iRules |
| IRULE4003 | Hint | Variable scoping concern across events |
| IRULE4004 | Info | Constant `set` in per-request event could be hoisted to per-connection |
| IRULE4005 | Warning | Potential race — `static::` variable written outside `RULE_INIT` and read in another event |

#### Performance & control flow

| Code | Severity | Description | Quick-fix |
|------|----------|-------------|-----------|
| IRULE2101 | Hint | Heavy `regexp` in a high-frequency event | |
| IRULE5001 | Hint | Ungated `log` in a high-frequency event | |
| IRULE5002 | Warning | `drop`/`reject`/`discard` without `event disable all` or `return` | Add `event disable all` + `return` |
| IRULE5003 | Hint | Loop condition `$var != 0` can miss zero if decremented past it | |
| IRULE5004 | Warning | `DNS::return` without `return` | Add `return` |
| IRULE5005 | Error | Direct proc invocation without `call` in iRules | Prefix with `call` |
| IRULE5006 | Warning | Top-level-only command used inside a nested body | |
| IRULE5007 | Warning | Event-context command used at top level outside a `when` block | |

## Optimiser codes

The optimiser operates on the SSA/CFG intermediate representation and suggests
source-level rewrites.  All optimiser diagnostics appear at **Information**
severity and include a quick-fix code action with the suggested replacement.

Five named profiles control which passes run.  Individual codes can be
overridden via `tclLsp.optimiser.*` settings.

| Code | Category | Description | readability | standard | full |
|------|----------|-------------|:-----------:|:--------:|:----:|
| O100 | constant_folding | Propagate constant variables into expressions and command arguments. |  | ✓ | ✓ |
| O101 | constant_folding | Fold constant integer expressions. |  | ✓ | ✓ |
| O102 | constant_folding | Fold constant `[expr {...}]` command substitutions. |  | ✓ | ✓ |
| O103 | constant_folding | Fold static procedure calls using interprocedural summaries. |  | ✓ | ✓ |
| O104 | pattern | Fold static string build chains into a single assignment. |  | ✓ | ✓ |
| O105 | constant_folding | Propagate constants into variable references and detect redundant computations (GVN/CSE). |  | ✓ | ✓ |
| O106 | code_motion | Hoist loop-invariant computations. |  |  | ✓ |
| O107 | dce | Eliminate unreachable dead code. |  |  | ✓ |
| O108 | dce | Eliminate transitively dead code. |  |  | ✓ |
| O109 | dce | Eliminate dead stores. |  |  | ✓ |
| O110 | constant_folding | Canonicalise expressions (InstCombine). |  | ✓ | ✓ |
| O111 | readability | Brace expression performance hints (paired with W100). | ✓ | ✓ | ✓ |
| O112 | dce | Eliminate constant-condition compound statements. |  |  | ✓ |
| O113 | constant_folding | Strength-reduce expressions (`x**2` → `x*x`, `x%8` → `x&7`). |  | ✓ | ✓ |
| O114 | readability | Recognise `incr` idiom (`set x [expr {$x + N}]` → `incr x N`). | ✓ | ✓ | ✓ |
| O115 | readability | Remove redundant nested `[expr {...}]` in expression context. | ✓ | ✓ | ✓ |
| O116 | constant_folding | Fold constant `[list a b c]` to literal value. |  | ✓ | ✓ |
| O117 | readability | Simplify `[string length $s] == 0` → `$s eq ""`. | ✓ | ✓ | ✓ |
| O118 | constant_folding | Fold constant `[lindex {a b c} 1]` to element. |  | ✓ | ✓ |
| O119 | pattern | Pack consecutive `set` literals into `lassign`/`foreach`. |  | ✓ | ✓ |
| O120 | readability | Prefer `eq`/`ne` over `==`/`!=` for string comparisons. | ✓ | ✓ | ✓ |
| O121 | recursion | Rewrite self-recursive tail calls to `tailcall`. |  |  | ✓ |
| O122 | recursion | Convert fully tail-recursive proc to iterative `while` loop. |  |  | ✓ |
| O123 | recursion | Detect non-tail recursion eligible for accumulator introduction (hint only). |  |  | ✓ |
| O124 | dce | Comment out unused procs in iRules (not called from any event). |  |  | ✓ |
| O125 | code_motion | Sink side-effect-free assignments into the deepest decision block (`if`/`switch`) that uses them. |  |  | ✓ |
| O126 | dce | Remove unused variable assignments — eliminate `set` statements for variables that are never read. |  |  | ✓ |
| O127 | code_motion | Inline single-use variable assignment — eliminate redundant variable load by folding `set` into the use site. |  |  | ✓ |
| O128 | readability | Rewrite `[expr {[llength $L] - N}]` / `[expr {[string length $s] - N}]` to `end-(N-1)` when used as an index argument to `lindex` (first index), `lrange`, `lreplace`, `string index`, `string range`, or `string replace` with a matching container reference. | ✓ | ✓ | ✓ |

**Profiles:** `off` disables all passes. `readability`, `standard`, and `full` enable
progressively more passes (single-pass). `aggressive` = `full` with multi-pass
to fixpoint (up to 5 iterations). The default editor profile is `readability`;
explicit actions (CLI, chat, MCP) default to `full`.

## Prerequisites

- Python 3.10+
- [uv](https://docs.astral.sh/uv/) (Python package manager)
- Node.js 24+ with npm (pinned to v12 via `packageManager`; run `corepack enable npm`)
- VS Code 1.93+

## Quick start

```sh
# Clone and enter the repo
git clone <repo-url>
cd tcl-lsp

# Run tests
make test

# Build the .vsix
make build-editor-vsix

# Install in VS Code
code --install-extension tcl-lsp-vscode-0.1.0.vsix
```

## Build targets

Run `make help` to see all targets:

| Target | Description |
|--------|-------------|
| `make ci-fast` | **Full CI gate** — lint + Python tests + extension tests + smoke tests |
| `make build-editor-vsix` | Build the .vsix (tests must pass first) |
| `make install` | Build and install the .vsix into VS Code |
| `make package-vsix` | Package VSIX (skip lint/test, for CI) |
| `make test` | Run all tests (Python + VS Code extension) |
| `make test-py` | Run the Python test suite only |
| `make test-ext` | Run VS Code extension integration tests |
| `make lint` | Run all lint and style checks |
| `make lint-py` | Lint Python code with Ruff |
| `make typecheck-py` | Type-check Python code with ty |
| `make lint-ts` | Lint/format-check TypeScript extension code |
| `make format-py` | Format and auto-fix Python code with Ruff |
| `make npm-env` | Install/update npm dependencies |
| `make compile` | Compile the TypeScript extension |
| `make zipapps` | Build all zipapps (Tcl, explorer-cli, explorer-gui, explorer-gui-cdn, LSP, AI, MCP, WASM) |
| `make zipapp-tcl` | Build the unified Tcl tools zipapp |
| `make zipapp-explorer-cli` | Build the compiler-explorer CLI zipapp |
| `make zipapp-explorer-gui` | Build the standalone explorer GUI zipapp (bundles Pyodide) |
| `make zipapp-explorer-gui-cdn` | Build the CDN explorer GUI zipapp (loads Pyodide from CDN) |
| `make zipapp-lsp` | Build the LSP server zipapp |
| `make zipapp-ai` | Build the AI analysis zipapp |
| `make zipapp-mcp` | Build the MCP server zipapp |
| `make zipapp-wasm` | Build the WASM compiler zipapp |
| `make claude-skills` | Build Claude Code skills release zip |
| `make build-editor-jetbrains` | Build the JetBrains plugin (.zip) | <!-- editors:JetBrains -->
| `make build-editor-sublime` | Build the Sublime Text package (.sublime-package) | <!-- editors:Sublime Text -->
| `make build-editor-zed` | Build the Zed extension (.tar.gz WASM artifact) | <!-- editors:Zed -->
| `make screenshot` | Alias of `make screenshots` |
| `make screenshots` | Capture extension screenshots and build demo GIF (macOS) |
| `make release` | Build all release artifacts (parity with tagged CI release jobs) |
| `make release-tag` | Bump version, annotated-tag, and push (`V=x.y.z`) |
| `make clean` | Remove build artifacts |
| `make distclean` | Remove build artifacts and `node_modules` |

Artifact version strings are derived from `git describe` (with `v` stripped).
If Git metadata is unavailable, builds fall back to `dev` (and semver-constrained
manifest fields use `0.0.0-dev`).

`make build-editor-vsix` is the main entry point.  It runs the test suite first and will
not package a .vsix if any test fails.  Packaging uses an isolated staging
directory under `build/vsix-stage/`, and the output file lands under
`build/` as `tcl-lsp-<version>.vsix`.

On macOS, `make screenshots` prefers a small Swift window-probe helper when
`swiftc` is available, so captures use deterministic
`screencapture -o -l <window-id>`.  If Swift is unavailable, it falls back to
AppleScript-based probing.
By default, `make screenshots` auto-installs missing screenshot tools with
Homebrew (`pngquant`, `oxipng`, `gifsicle`, and `imagemagick` when needed).
To disable auto-install, run:
`TCL_LSP_SCREENSHOT_AUTO_BREW=0 make screenshots`.
By default, screenshot runs are isolated:
- downloaded VS Code `stable` via `@vscode/test-electron`
- isolated user data (`~/.tcl-lsp-screenshots/user-data`)
- isolated extensions dir (`~/.tcl-lsp-screenshots/extensions`)
- allowlisted external extensions only (`github.copilot-chat`)

Useful overrides:
- Reuse your normal VS Code user data: `TCL_LSP_SCREENSHOT_REUSE_CODE_USER_DATA=1 make screenshots`
- Use local app bundle instead of downloaded VS Code:
  `TCL_LSP_SCREENSHOT_USE_SYSTEM_VSCODE=1 TCL_LSP_SCREENSHOT_FORCE_DOWNLOADED_VSCODE=0 make screenshots`
- Change allowed external extensions (comma-separated extension IDs):
  `TCL_LSP_SCREENSHOT_ALLOWED_EXTENSIONS=github.copilot-chat make screenshots`

### Dependency audit policy

- Production dependency audits are enforced with `npm audit --omit=dev`.
- Dev-only audit findings are accepted and do not block releases in this repository.

## Project layout

```
tcl-lsp/
  Makefile                Build system
  pyproject.toml          Python project metadata (hatchling)
  server/                    Python LSP server
    __main__.py           Entry point (python -m server)
    server.py             pygls server, handler wiring
    async_diagnostics.py  Background diagnostic scheduler (tiered publishing)
    analysis/
      analyser.py         Single-pass semantic analyser
      checks.py           Best-practice and security checks (W-series)
      irules_checks.py    iRules-specific best-practice checks (IRULE-series)
      semantic_model.py   Data model (scopes, procs, diagnostics)
      semantic_graph.py   Call/symbol/data-flow graph queries
    bigip/
      parser.py           BIG-IP configuration file parser
      model.py            BIG-IP configuration data model
      rule_extract.py     iRule extraction from BIG-IP configs
      validator.py        Configuration validation
      diagnostics.py      BIG-IP-specific diagnostics
    commands/
      registry/
        models.py         CommandSpec dataclass (arity, roles, dialect flags)
        command_registry.py CommandRegistry class (query methods)
        runtime.py        Registry runtime (dialects, roles, body/expr index helpers)
        signatures.py     Argument signature helpers
        namespace_registry.py Namespace registry (event/command metadata facade)
        namespace_data.py    Canonical event/command data tables
        namespace_models.py  Namespace model dataclasses
        operators.py      Operator definitions and hover data
        taint_hints.py    Per-command taint source/sink hints
        type_hints.py     Per-command return type hints
        tcl/              One file per Tcl command (@register decorator)
        irules/           F5 iRules command definitions
        iapps/            F5 iApps template command definitions
        tk/               Tk widget command definitions
        tcllib/           tcllib package command definitions
        stdlib/           Tcl standard library command definitions
    common/
      dialect.py          Active dialect state
      naming.py           Name normalisation helpers
      ranges.py           Range/position utilities
    packages/
      resolver.py         Tcl package require resolution
    compiler/
      lowering.py         Tcl source -> IR lowering
      ir.py               IR node definitions
      cfg.py              Control flow graph construction
      ssa.py              Static single assignment form
      core_analyses.py    SCCP, liveness, type inference, dead store detection
      compilation_unit.py Compile pipeline orchestration and caching
      compiler_checks.py  IR-to-diagnostics (arity, subcommands)
      optimiser.py        Source rewrite passes (O100–O128)
      gvn.py              GVN/CSE/PRE/LICM redundant computation detection (O105–O106)
      interprocedural.py  Call graph, function purity/side-effect summaries
      taint.py            Data taint analysis (T100–T106, IRULE3xxx)
      shimmer.py          Tcl object representation analysis (S100–S102)
      irules_flow.py      iRules control-flow checks (IRULE1xxx/4004/5xxx)
      codegen.py          Tcl VM bytecode assembly backend
      static_loops.py     Conservative static evaluation for for-loops
      tcl_expr_eval.py    Tcl expression evaluator (constant folding)
      expr_ast.py         Expression AST parser
      expr_types.py       Expression type inference
      effects.py          Command side-effect classification
      connection_scope.py iRules connection-scope variable tracking
      types.py            Type lattice definitions
      token_helpers.py    Shared token-stream utilities
      eval_helpers.py     Evaluation helper constants
    diagram/
      extract.py          iRule event-flow diagram extraction
    features/
      code_actions.py     Quick-fix code actions
      completion.py       Completions
      definition.py       Go to definition
      diagnostics.py      Diagnostic aggregation (internal -> LSP)
      formatting.py       LSP formatting handlers
      hover.py            Hover information
      inlay_hints.py      Inlay hint provider (inferred types, format strings)
      references.py       Find references
      rename.py           Rename symbol
      call_hierarchy.py   Call hierarchy (incoming/outgoing calls)
      document_symbols.py Document symbol hierarchy
      document_links.py   Document link provider
      folding.py          Folding range provider
      selection_range.py  Selection range provider
      signature_help.py   Signature help provider
      workspace_symbols.py Workspace symbol search
      semantic_tokens.py  Semantic token provider
      snippet_templates.py Tcl/iRules snippet templates
      symbol_resolution.py Shared word/variable/scope resolution helpers
    parsing/
      lexer.py            Tcl lexer with position tracking
      tokens.py           Token and position types
      command_segmenter.py Command segmentation from token stream
      token_scanning.py   Shared token-stream scanning helpers
      recovery.py         Centralised error recovery via virtual tokens
      expr_lexer.py       Expression sub-lexer
      expr_parser.py      Expression sub-parser
      subst_nocommands.py Compile-time `[subst -nocommands]` evaluator
    tk/
      detection.py        Tk widget auto-detection
      diagnostics.py      Tk-specific diagnostics
      extract.py          Tk widget hierarchy extraction
    workspace/
      document_state.py   Per-file analysis cache (dialect-gated profile scanning)
      workspace_index.py  Cross-file proc index (O(1) tail lookup, usage caching)
      scanner.py          Background workspace file scanner
    xc/
      translator.py       iRules-to-XC migration translator
      mapping.py          iRules → XC command mapping table
      xc_model.py         XC output data model
      terraform.py        Terraform HCL generation
      json_api.py         JSON API for XC translation
      diagnostics.py      Migration diagnostics
  tooling/explorer/               Compiler explorer (CLI + web GUI)
    cli.py                CLI interface
    pipeline.py           Compilation pipeline wrapper
    serialise.py          Output serialisation (IR, CFG, SSA, optimiser)
    formatters.py         Display formatters
    static/               Web GUI assets (Pyodide)
  ai/                     AI integrations
    claude/
      skills/             Claude Code skills (20 CLI commands)
    mcp/
      tcl_mcp_server.py   MCP server for Claude Desktop integration
    prompts/              System prompts for Tcl/iRules/Tk
    shared/               Shared diagnostics manifest and utilities
  tests/                  pytest test suite
  editors/
    vscode/               VS Code extension client (.vsix)
      package.json        Extension manifest
      tsconfig.json       TypeScript config
      src/extension.ts    Extension entry point
      language-configuration.json
      syntaxes/tcl.tmLanguage.json
    neovim/               Neovim LSP config (Lua) <!-- editors:Neovim -->
    zed/                  Zed extension (TOML + Rust WASM) <!-- editors:Zed -->
    emacs/                Emacs eglot / lsp-mode config <!-- editors:Emacs -->
    helix/                Helix languages.toml config <!-- editors:Helix -->
    sublime-text/         Sublime Text package (syntax, LSP, snippets) <!-- editors:Sublime Text -->
    jetbrains/            JetBrains plugin (Gradle/Kotlin) <!-- editors:JetBrains -->
```

## Development

See `CONTRIBUTING.md` for coding-style and packaging rules.

### Running the server standalone

The server communicates over stdio.  To launch it directly:

```sh
uv run python -m server
```

This is useful for debugging or for use with any LSP client.
See `editors/` for per-editor setup instructions.

### Running tests

```sh
# Via make (sets up the venv automatically)
make test

# Or directly with uv
uv run --extra dev pytest tests/ -v

# Run a specific test file
uv run --extra dev pytest tests/test_checks.py -v

# Run tests matching a pattern
uv run --extra dev pytest tests/ -k "unbraced_expr"

# Lint Python code
make lint-py

# Type-check Python code
make typecheck-py

# Auto-fix and format Python code
make format-py
```

### Compiler and optimiser explorer

Use `tcl_compiler_explorer.py` to inspect how source is lowered and optimised:

```sh
# Full compiler + optimiser exploration
uv run python tcl_compiler_explorer.py samples/for_screenshots/22-optimiser-before.tcl

# Focus on optimiser rewrites only
uv run python tcl_compiler_explorer.py samples/for_screenshots/22-optimiser-before.tcl --focus optimiser

# Inline source with explicit optimised output
uv run python tcl_compiler_explorer.py --source 'set a 1; set b [expr {$a + 2}]' --show-optimised-source
```

The explorer renders:
- lowered IR and per-procedure bodies
- CFG pre-SSA and post-SSA (with use/def and inferred constants)
- interprocedural summaries
- optimiser rewrites
- source callouts with caret markers and `+-->` arrows for salient spans

### Developing the extension client

```sh
# Install npm deps
make npm-env

# Watch mode (recompiles on save)
cd editors/vscode && npm run watch
```

To test the extension in VS Code, open `editors/vscode/` in VS Code and press
**F5** to launch the Extension Development Host.

### Developing the server

During development you can point the extension at your working copy instead
of the bundled server.  Set `tclLsp.serverPath` in your VS Code settings:

```json
{
  "tclLsp.serverPath": "/path/to/tcl-lsp"
}
```

The extension will use `uv run` from that directory, so changes to the Python
source take effect on the next editor reload.

### Adding a new diagnostic check

1. Add a check function to the appropriate submodule in `analyser/checks/`
   (e.g. `_security.py`, `_style.py`, `_domain.py`, `_syntax.py`) following
   the existing pattern -- each check receives the command name, argument
   texts, argument tokens, all tokens, and the source string.
2. Register it in the `ALL_CHECKS` list in `analyser/checks/_orchestrator.py`.
3. If the check can be auto-fixed, include a `CodeFix` in the diagnostic's
   `fixes` tuple.
4. Add tests to `tests/test_checks.py`.
5. Run `make test` to verify.

### Adding a new formatter option

1. Add the field to `FormatterConfig` in `tooling/formatter/config.py`.
2. Handle it in `tooling/formatter/engine.py`.
3. Add `to_dict`/`from_dict` support if the field uses a non-primitive type.
4. Add tests to `tests/test_formatter.py`.
5. Import the formatter through its public API (`tooling.formatter`), then run
   `make test` to verify.

## Configuration

Server/runtime settings are available through the `tclLsp.*` namespace.

### Dialect settings

| Setting | Default | Description |
|---------|---------|-------------|
| `dialect` | `tcl8.6` | Default dialect for files without a shebang or `# tcl-dialect:` comment directive. Per-file hints take priority. |
| `extraCommands` | `[]` | Extra command names treated as known varargs commands |
| `libraryPaths` | `[]` | Additional directories to scan for Tcl packages and libraries |

### Formatter settings

Formatter options are available through `tclLsp.formatting.*` (defaults based
on the F5 iRules Style Guide):

| Setting | Default | Description |
|---------|---------|-------------|
| `indentSize` | `4` | Spaces per indent level |
| `indentStyle` | `spaces` | `spaces` or `tabs` |
| `continuationIndent` | `4` | Extra indentation for continuation lines |
| `braceStyle` | `k_and_r` | `k_and_r` |
| `spaceBetweenBraces` | `true` | Space between consecutive braces (`} {` vs `}{`) |
| `enforceBracedVariables` | `false` | Rewrite `$var` as `${var}` |
| `enforceBracedExpr` | `false` | Require braced expressions |
| `maxLineLength` | `120` | Hard line length limit |
| `goalLineLength` | `100` | Soft target for line length |
| `expandSingleLineBodies` | `false` | Force multi-line bodies |
| `minBodyCommandsForExpansion` | `2` | Minimum commands in body before expansion |
| `spaceAfterCommentHash` | `true` | Space between `#` and comment text |
| `trimTrailingWhitespace` | `true` | Remove trailing whitespace |
| `alignCommentsToCode` | `true` | Align inline comments to a consistent column |
| `replaceSemicolonsWithNewlines` | `true` | Convert `;` to newlines |
| `blankLinesBetweenProcs` | `1` | Blank lines separating proc definitions |
| `blankLinesBetweenBlocks` | `1` | Blank lines between control flow blocks |
| `maxConsecutiveBlankLines` | `2` | Maximum consecutive blank lines allowed |
| `lineEnding` | `lf` | Line ending style (`lf`, `crlf`, `cr`) |
| `ensureFinalNewline` | `true` | Ensure file ends with a newline |

### Shimmer detection settings

| Setting | Default | Description |
|---------|---------|-------------|
| `shimmer.enabled` | `true` | Enable shimmer detection (S-series diagnostics) |

### Optimiser settings

Optimiser settings are under `tclLsp.optimiser.*`:

| Setting | Default | Description |
|---------|---------|-------------|
| `enabled` | `true` | Enable optimiser suggestions as diagnostics |
| `profile` | `readability` | Named profile: `off`, `readability`, `standard`, `full`, `aggressive` |
| `O100`–`O128` | `null` | Per-code override (`true`/`false` = force on/off; `null` = inherit from profile) |

See the [Optimiser codes](#optimiser-codes) table for which codes each profile enables.

### Diagnostic settings

All diagnostic codes can be toggled individually via
`tclLsp.diagnostics.<CODE>: true/false`. The main series are:

| Series | Codes | Area |
|--------|-------|------|
| **E** | E100–E999 | Errors |
| **W** | W100–W299 | General warnings |
| **W** | W300–W309 | Security warnings |
| **S** | S100–S102 | Shimmer detection |
| **T** | T100–T102 | Taint analysis |
| **H** | H100+ | Hints |
| **IRULE** | IRULE1001–IRULE5005 | iRules-specific diagnostics |

### Configuration File

Settings can be stored in INI files.  Two files are read:

- **Global** — user-wide defaults, platform-native location:

  | Platform | Default path |
  |----------|-------------|
  | **Linux / BSD / WSL2** | `~/.config/tcl-lsp/config.ini` |
  | **macOS** | `~/Library/Application Support/tcl-lsp/config.ini` |
  | **Windows** | `%APPDATA%\tcl-lsp\config.ini` |
  | **MSYS2 / Cygwin** | `~/.config/tcl-lsp/config.ini` |

  Setting `$XDG_CONFIG_HOME` overrides the default on every platform.

- **Project** — `.tcl-lsp.ini` at the workspace root, committed with the
  source so every contributor picks up the same rules automatically.

**Precedence** (applied in order — later entries override earlier):

1. Built-in defaults
2. Global config file
3. Editor settings (VS Code `settings.json`, Neovim `lspconfig`, etc.)
4. Project config file (`.tcl-lsp.ini` — highest server-level priority)

Both files use the same INI schema:

```ini
[diagnostics]
disabled = W111, T100

[optimiser]
disabled = O109

[shimmer]
enabled = true

[features]
inlayTypeHints = false
inlayParameterHints = false

[formatting]
indent_size = 2
```

See [`docs/design/contracts/xdg-config.md`](docs/design/contracts/xdg-config.md) for the
full reference, including how settings interact with each editor.

### Export Settings

In **VS Code**, run the command **"Tcl: Export Settings to Config File"**
from the command palette. For other editors, send the
`tcl-lsp.exportConfig` request via `workspace/executeCommand`.

Only non-default values are written, keeping the generated config file
minimal.  This lets you configure in one editor and have the same
defaults apply everywhere.

### Example

```json
{
  "tclLsp.dialect": "f5-irules",
  "tclLsp.extraCommands": ["myCompany::command"]
}
```

## Acknowledgements

This project was inspired by:

- [Picol](https://github.com/antirez/picol) by Salvatore Sanfilippo (antirez) -- a minimal Tcl interpreter in C that demonstrates the elegance of the Tcl parsing model
- [iRuleScan](https://github.com/simonkowallik/irulescan) by Simon Kowallik -- a security scanner for F5 iRules
- [tclint-vscode](https://github.com/nmoroze/tclint-vscode) by Noah Moroze -- a Tcl linter with VS Code integration

## AI

This project used AI very heavily.

- The core parser, lexer, IR, CFG were largely hand created with input on AI about
  structure, and lots of AI code review.
- The command registry was seeded by hand then filled out with AI.
- The vscode extension, compiler explorer, editor integrations, CI/CD, build pipelines
  VM, and compiler to Tcl bytecode were all entirely vibe coded.
- The Claude skills, AI integrations were vibe coded with hand work on the prompts
  .. they need more of that.
- The vast bulk of tests were AI written, AI ported from sources like Tcl, but all 
  largely directed by me in their creation. If I'd been doing that by hand you'd
  see 3 tests and they'd all be "make install worked for me, good luck"
- Claude Opus 4.6, Gemini 3.1 Pro and OpenAI GPT-5.3-Codex were all used to review
  the code, critise it, rewrite and reorganise it.

## License

This project is licensed under the [GNU Affero General Public License v3.0](LICENSE)
(AGPL-3.0-or-later).

You are free to use this tool as-is. If you modify the code or incorporate
portions of it into another project, the AGPL requires that the complete
source of the derivative work is made available under the same license.

**Upstream contributions strongly preferred.** If you improve or extend this
project, please submit your changes back as a pull request rather than
maintaining a private fork. See [CONTRIBUTING.md](CONTRIBUTING.md) for details.
