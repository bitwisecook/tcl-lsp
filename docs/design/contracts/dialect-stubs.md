# KCS: Dialect command stubs

## Purpose

Dialect stubs let users declare command signatures for unknown dialect
extensions so the LSP provides completion, diagnostics, and semantic
understanding without a full registry entry.  This is essential for
EDA tools (Synopsys, Cadence, Xilinx), custom frameworks, and any Tcl
extension that adds commands the LSP does not know about.

## Delivery mechanisms

### External stub files

`<dialect>.tcl.stubs` files contain stub definitions, one per line.
No `#` prefix is needed.  Comments start with `#`.

```
# synopsys.tcl.stubs
stub foreach_in_collection {varName:var collection body:body} -loop
stub get_cells {?-hierarchical? ?-filter? pattern:pattern} -pure
stub sizeof_collection {collection} -pure
stub expr-func sizeof 1
```

### Inline stubs

Stub blocks are bracketed by `# tcl-lsp: stubs-begin` and
`# tcl-lsp: stubs-end` markers.  Multiple blocks per file are supported.
Stubs outside a block are ignored.

```tcl
# tcl-lsp: stubs-begin
# tcl-lsp: stub foreach_in_collection {varName:var collection body:body} -loop
# tcl-lsp: stub get_cells {pattern:pattern} -pure
# tcl-lsp: stub expr-func sizeof 1
# tcl-lsp: stub expr-op contains 2
# tcl-lsp: stubs-end
```

## Command stub syntax

```
stub <command-name> {arg1:role arg2 ?optArg:role?} ?flags...?
```

### Argument roles

| Role | Meaning |
|------|---------|
| `body` | Tcl script body (recursively analysed) |
| `expr` | Expression (expr sub-language) |
| `var` | Variable name written by the command |
| `var_read` | Variable name read without modification |
| `name` | Symbolic name (proc, namespace, design name) |
| `pattern` | Pattern or regex |
| `channel` | Channel identifier |
| `value` | Generic value (default when no role given) |

### Optional arguments

Wrap in `?...?` to mark as optional: `?-filter?`, `?count:value?`.

### Flags

| Flag | CommandSpec equivalent |
|------|----------------------|
| `-barrier` | `creates_dynamic_barrier=True` |
| `-loop` | `has_loop_body=True` |
| `-pure` | `pure=True` |
| `-mutator` | `mutator=True` (on forms) |
| `-unsafe` | `unsafe=True` |
| `-scope_alias` | `creates_scope_alias=True` |

## Expression stubs

Custom math functions and infix operators for dialects that extend expr:

```
stub expr-func <name> ?arity?    # default arity 1
stub expr-op <name> ?arity?      # default arity 2
```

Examples:
```
stub expr-func sizeof 1
stub expr-func clamp 3
stub expr-op contains 2
stub expr-op starts_with 2
```

## Data model

```python
@dataclass(frozen=True, slots=True)
class StubArgDef:
    name: str
    role: str = "value"
    optional: bool = False


@dataclass(frozen=True, slots=True)
class StubCommandDef:
    name: str
    args: tuple[StubArgDef, ...]
    range: Range
    barrier: bool = False
    loop: bool = False
    pure: bool = False
    mutator: bool = False
    unsafe: bool = False
    scope_alias: bool = False


@dataclass(frozen=True, slots=True)
class StubExprDef:
    name: str
    kind: str  # "function" or "operator"
    arity: int = 1
    pure: bool = True
    range: Range
```

Stubs are stored on `AnalysisResult`:
- `stub_commands: list[StubCommandDef]`
- `stub_expr_defs: list[StubExprDef]`

## AI-assisted stub generation

Users can describe a command or provide a man page and have AI translate
it to a stub.  This is supported via:

1. **MCP tool** — `generate_dialect_stub` accepts a command description
   or man page excerpt and returns a properly formatted stub line.
2. **Editor skill** — `/generate-stub` slash command in Claude Code.
3. **Inline assist** — when the LSP detects an unknown command, it can
   offer a code action to generate a stub from usage context.

## Parsing

`scan_source_for_stubs(source)` performs a line-based pre-scan of the
source text before lexing.  It finds stubs-begin/end markers and
parses all stub definitions between them.

`parse_stubs_file(path)` reads a `.tcl.stubs` file and returns both
command stubs and expression stubs.

## Files

| File | Purpose |
|------|---------|
| `compiler/registry/stub_comments.py` | Stub parser (inline + file) |
| `analyser/semantic_model.py` | `StubCommandDef`, `StubExprDef`, `StubArgDef` |
| `analyser/_analyser/__init__.py` | Pre-scan integration |
| `samples/synopsys.tcl.stubs` | Example external stubs file |
| `samples/dialect_stubs_inline_example.tcl` | Example inline stubs |
| `tests/test_stub_comments.py` | Unit tests |
