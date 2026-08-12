# Rendered value properties — string content analysis over SSA

How the compiler determines the string content properties of an SSA value —
path separators, escape sequences, and interpolation — after Tcl backslash
substitution, and how to add a property for a downstream consumer.

The Rendered Value Properties pass computes per-SSA-value properties of the
rendered (post-backslash-subst) string content.  `FunctionUnit::build` runs it
**after SCCP and type propagation** (so dead blocks and edges are known) and
**before taint propagation** (so the taint engine can colour values from the
result without re-lexing).

Source: `rust/tcl-compiler/src/rendered_properties.rs` —
`propagate_rendered_props` is the entry point.

Consumers:

- `rust/tcl-compiler/src/taint.rs` — `colour_from_rendered` stamps
  `TaintColour::PATH_PREFIXED` and `TaintColour::NON_DASH_PREFIXED` onto a
  value whose `must` bits carry `STARTS_WITH_SLASH`.
- `rust/tcl-compiler/src/path_concat.rs` — `find_path_concat_warnings` (W201).

### Lattice design

The analysis uses a **reduced product** of two property groups with different
join semantics:

```rust
pub struct RenderedValueProps {
    pub may: RenderedProperties,  // union at phi joins        (overapproximate)
    pub must: RenderedProperties, // intersection at phi joins (underapproximate)
}
```

- **may** properties: if *any* incoming edge has the property, the merged
  value has it.  Used for "does this value possibly contain X?"
- **must** properties: only survives if *all* incoming edges agree.  Used
  for "does this value always start with X?"

`RenderedValueProps::bottom()` is empty `may` with every `must` bit set;
`top()` is the reverse (`MAY_MASK` set, no `must` bit).  `Default` is
`bottom()`, so a key absent from the result map reads as bottom.

This follows the standard abstract interpretation design of a reduced product
of simple boolean domains (cf. Costantini, Ferrara & Cortesi 2011).

### Property flags

`RenderedProperties` is a `bitflags` set over `u32`.  All three groups share
the one bitmask; which field a bit is stored in decides how it joins.

| Flag | Group | Meaning |
|------|-------|---------|
| `HAS_FORWARD_SLASH` | may | Rendered literal text contains `/` |
| `HAS_BACKSLASH` | may | Rendered literal text contains `\` (path separator) |
| `HAS_CRLF` | may | Rendered literal text contains `\r` or `\n` |
| `HAS_INTERPOLATION` | may | Value contains `$var` or `[cmd]` |
| `HAS_DOUBLE_ESCAPE` | may | Rendered text still contains a backslash escape |
| `HAS_NULL` | may | Rendered text contains a null byte |
| `HAS_LITERAL_SPACE` | may | Rendered text has a top-level literal space or tab — prose, a protocol line, or display text, not a single path token |
| `WAS_UNESCAPED` | may (provenance) | Value passed through an unescape command |
| `DOUBLE_UNESCAPED` | may (provenance) | Value was already `WAS_UNESCAPED` then unescaped again |
| `FULLY_NORMALISED` | may (provenance) | Value fully canonical — no residual encoding (e.g. `-normalized`) |
| `STARTS_WITH_SLASH` | must | First rendered literal char is `/` |
| `STARTS_WITH_DASH` | must | First rendered literal char is `-` |

Three masks name the groups: `RenderedProperties::MAY_MASK` (the seven
content bits — the conservative top for an unknown value),
`RenderedProperties::PROVENANCE_MASK` (the three provenance bits), and
`RenderedProperties::MUST_MASK` (the two starts-with bits).  The provenance
bits live in the `may` field, so they union at joins, but they sit
deliberately **outside** `MAY_MASK`: an unknown value is not assumed to have
been unescaped.

### Escape rendering

`scan_value_text` slices each backslash escape out with `escape_byte_len`
(which mirrors `tcl_lexer::backslash_subst`'s per-escape consumption rules)
and renders it through `tcl_lexer::backslash_subst`, then reads the property
bits off what the decoder actually produced.  This means:

- `\n`, `\t`, `\r` → resolved to actual control characters (not path seps)
- `\x2f`, `\057` → resolved to `/` (detected as `HAS_FORWARD_SLASH`)
- `\\` → resolved to single `\` (detected as `HAS_BACKSLASH`)
- `\x61` → resolved to `a` (not a path separator)

`has_double_escape` sets `HAS_DOUBLE_ESCAPE` when the *rendered* text still
holds a backslash followed by a recognised escape character.

### SSA copy propagation

For pure variable references (`set x $y`), `evaluate_value` propagates
properties from the source SSA value via the `uses` map on `SsaStatement`.
If `y` has `HAS_FORWARD_SLASH`, `x` inherits it after the copy.  A use at
version 0 — an enclosing scope, or a name not yet defined on this path — is
modelled as `top` rather than bottom, so the merge stays sound.

### Unescape provenance tracking

The `WAS_UNESCAPED` and `DOUBLE_UNESCAPED` bits are set only by commands the
registry classifies as unescapers, and otherwise travel through copies and
phi joins.  Generic unknown commands do NOT get these bits.

Detection works at two levels, both keyed off `is_unescape_command`:

1. **`evaluate_value`**: when the assigned word is a pure command
   substitution calling an unescape command, sets `WAS_UNESCAPED` on the
   result and escalates to `DOUBLE_UNESCAPED` when an SSA input already
   carries `WAS_UNESCAPED` (and not `FULLY_NORMALISED`).
2. **`evaluate_call`**: the same rule for a `Statement::Call` that defines a
   value, over `collect_use_may`'s union of the statement's SSA inputs.

Copy propagation (`set b $a`) inherits `WAS_UNESCAPED` from the source,
so a chain like `set a [subst $x]; set b $a; set c [subst $b]` tags `c`
with `DOUBLE_UNESCAPED`.

Which commands unescape is **registry data**, not a list in the pass:
`is_unescape_command` asks for `Traits::IS_UNESCAPE` on the command's
top-level `CommandSpec`.  The commands carrying it today:

- `subst` — Tcl backslash/variable/command substitution
- `URI::decode` — percent-decoding
- `decode_uri` — legacy alias for `URI::decode`
- `b64decode` — base64 decoding

`encoding convertfrom` is *not* among them.  It declares `is_unescape: true`
on its own `SubCommand`, but no consumer reads that field, and
`is_unescape_command` looks only at the dispatcher's traits — so a value
decoded by `encoding convertfrom` carries no unescape provenance.

Normalised getters (`FULLY_NORMALISED`) are registry data too:
`is_normalised_getter` requires `Traits::UNNORMALISED_HTTP_GETTER` on the
spec **and** a literal `-normalized` among the arguments.  The getters
carrying that trait are `HTTP::uri`, `HTTP::path`, and `HTTP::query`; called
with `-normalized` they return fully URI-decoded values with no residual
encoding.  Still tainted (HTTP input) but encoding-safe.

`FULLY_NORMALISED` suppresses `DOUBLE_UNESCAPED` escalation: decoding a
fully normalised value is harmless since there is nothing left to decode.

### Pipeline placement

```text
SCCP  →  Type propagation  →  Rendered properties  →  Taint propagation
                                      ↓
                              stored on FunctionUnit.rendered_props
                                      ↓
                       consumed by colour_from_rendered (taint colours)
                       and find_path_concat_warnings (W201)
```

`propagate_rendered_props` walks blocks in `cfg_order(cfg)` to a fixpoint,
skipping blocks SCCP proved unreachable and phi predecessors on
non-executable edges.  The result is a `HashMap<ValueKey, RenderedValueProps>`
keyed by `(Symbol, ssa_version)`.

### Transfer functions

| IR statement | Transfer |
|---------|----------|
| `Statement::AssignConst` | `evaluate_const` — direct string inspection via `analyse_literal` |
| `Statement::AssignValue` (pure var ref) | Copy from the source SSA value |
| `Statement::AssignValue` (pure command sub) | `HAS_INTERPOLATION` plus registry refinements (`RETURNS_PATH`, `IS_UNESCAPE`, normalised getter) — deliberately *not* top |
| `Statement::AssignValue` (other) | `scan_value_text` — scan the literal/interpolation pattern, render ESC tokens |
| `Statement::AssignExpr`, `Statement::Incr` | Numeric result → empty `may`, empty `must` |
| `Statement::Call` with defs | `evaluate_call` — normalised getter, unescaper, or `RETURNS_PATH`; otherwise top |
| `Statement::Barrier` | Every def set to `top` — handled in the walk loop, before `evaluate_def` |
| anything else | top |

### W201 integration

`find_path_concat_warnings` in `path_concat.rs` uses rendered properties for
detection and taint colours for suppression.

**Structural filters** applied before the property test — the statement must
be a `Statement::AssignValue` whose value is neither a pure `$var` alias nor a
pure `[cmd …]` substitution, and must not contain `://`, `<`, or `>` (URLs and
markup are not filesystem paths).

**Detection** (both required):

1. `HAS_FORWARD_SLASH` or `HAS_BACKSLASH` in the `may` properties
2. `HAS_INTERPOLATION` in the `may` properties

**Suppression**:

- `HAS_LITERAL_SPACE` in the `may` properties — a literal space or tab marks
  prose or a protocol line, not a path being built.
- The `PATH_NORMALISED` taint colour on the SSA value.  This arm is inert:
  the taint engine does not put `PATH_NORMALISED` on `[file normalize]`
  results, so it never fires.
- Forward-scan: the next assignment to the same variable in the same block
  is `[file normalize $var]`.  This is the only suppression path that fires
  end to end.

### Adding a new property

1. Add the flag to the `RenderedProperties` bitflags in
   `rendered_properties.rs`
2. Add it to `MAY_MASK`, `PROVENANCE_MASK`, or `MUST_MASK` depending on join
   semantics
3. Set it in `analyse_literal` / `scan_value_text` (literal evidence) and/or
   `evaluate_value` / `evaluate_call` (registry evidence)
4. Add unit tests in `rust/tcl-compiler/src/rendered_properties.rs`
5. Consume it in the downstream pass

## Resolution

Refer to this document when modifying the rendered properties pass, adding
new properties, or debugging W201 false positives/negatives.  The lattice
join in `rendered_join()` handles phi nodes automatically.
