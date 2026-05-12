# F5 query DSL

> **Audience:** Developer / Maintainer
> **Type:** Architecture

The `f5 query` verb embeds a small jq-flavoured language for
navigating and rewriting BIG-IP configuration.  It complements the
existing verbs:

- `f5 grep` answers "which objects are related to X?".
- `f5 rename` answers "swap object name X for Y everywhere".
- `f5 query` answers "for every object matching this filter, project /
  set / append this field".

This document is the canonical reference for the grammar, value model,
builtin library, and edit-application pipeline.  The user-facing
quick-start lives in
[`docs/kcs/features/kcs-feature-bigip-query.md`](../kcs/features/kcs-feature-bigip-query.md).

## Module map

```
core/bigip/query/
  __init__.py        # public API: run_query, parse_query, format_*
  errors.py          # QueryError hierarchy (lex / parse / eval / edit / builtin)
  values.py          # ObjectRef, PathRef, Stream, Root, FieldSlot
  source_map.py      # offset → (line, column) lookup
  lexer.py           # hand-rolled tokeniser
  ast.py             # frozen-dataclass AST node types
  parser.py          # recursive-descent parser
  projection.py      # BigipConfig → navigable Container tree
  graph.py           # refs / referenced_by — forwards to core.bigip.grep
  builtins.py        # @_register'd function library
  evaluator.py       # walks the AST, collects edits, returns values
  edit_plan.py       # routes identity writes through rename_object,
                     # detects conflicts, applies bottom-up
  output.py          # auto / scf / raw / paths / json renderers
  runner.py          # high-level orchestration used by the CLI verb
  grammar.py         # plain-text grammar for --help-dsl
  examples.py        # worked-example cookbook for --help-examples
explorer/verbs/f5/query.py
                     # argparse plumbing + custom help actions
```

## Grammar

The DSL is a small pipeline language.  Each statement is a pipeline of
stages joined by `|`; statements are separated by `;`.

```
program       := pipeline (';' pipeline)*
pipeline      := pipe_stage ('|' pipe_stage)*
pipe_stage    := or_expr (ASSIGN_OP pipe_stage)?
ASSIGN_OP     := '=' | '|=' | '+=' | '-='
or_expr       := and_expr ('or' and_expr)*
and_expr      := not_expr ('and' not_expr)*
not_expr      := 'not' not_expr | cmp_expr
cmp_expr      := add_expr (CMP add_expr)?
CMP           := '==' | '!=' | '<' | '<=' | '>' | '>='
add_expr      := mul_expr (('+' | '-') mul_expr)*
mul_expr      := unary    (('*' | '/') unary)*
unary         := '-' unary | postfix
postfix       := primary path_tail
primary       := literal | call | path | list_literal | '(' pipeline ')'
list_literal  := '[' pipeline? ']'
path          := '.' | '.' field path_tail | '.' subscript path_tail
path_tail     := ('.' field | subscript)*
field         := IDENT | STRING
subscript     := '[' (pipeline | /* empty stream */) ']'
                 /* A STRING subscript whose contents start with "~"
                    is the regex-subscript form ["~pattern"]. */
call          := IDENT ('(' (pipeline (',' pipeline)*)? ')')?
                 /* parens optional for single-arg builtins —
                    bare `length` is sugar for `length(.)` */
literal       := NUMBER | STRING | 'true' | 'false' | 'null'
```

### Pipe semantics

The pipe operator iterates **streams**, not plain lists.  `[]` and
stream-returning builtins produce streams; lists (such as the value of
`.rules`) pass through `|` as a single value.  Two consequences:

- `.rules | length` returns the length of the rules list (the list
  flows whole into `length`).
- `.X[] | length` returns the length **of each X** (the stream
  iterates `length` per item).

To fold a stream into a list — the input most aggregators want — wrap
it with a list literal:

```
[.ltm.virtual[]] | length              # number of VSes
[.ltm.virtual[].name] | sort | first   # alphabetical first VS name
[.ltm.virtual[] | refs(.)[]] | unique  # union of every VS's deps
```

This matches jq's `[.foo[]] | length` idiom exactly.

### Compatibility with jq

The core idioms are deliberately jq-compatible — anyone reading the
docs with jq experience should be able to use `f5 query` without
surprise.  Shared semantics:

- `.X[]` is a stream-generator; `|` iterates it.
- `[ ... ]` is the array constructor — collects a stream into a list
  for aggregators (`[.X[].name] | sort | first`).
- Bare builtin names (`length`, `sort`, `unique`, …) operate on `.`.
- Plain lists pass through `|` whole; iterate with `.[]` to flow each
  item, or pass directly to a list-aware builtin like `map`.
- `length` of a string is its character count; of a list/stream, its
  item count; of an object, its field count.

Deliberate divergences:

| Concern | jq | f5 query |
|---|---|---|
| Function arguments | `;` separated | `,` separated |
| Stream concat `,` | yes | not present — use lists or `;` statements |
| Regex test | `test("pat")` builtin | `["~pat"]` subscript form |
| Identifier hyphens | quoted only | bareword (`source-address-translation`) |
| Object literals `{...}` | yes | not present in v1 — use `--json` output |
| String interpolation `"\(.x)"` | yes | not present in v1 |
| Assignment + pipe | `path \|= f` binds tight via custom precedence | `\|=` is a pipe-stage trailing operator, so `a \| b \|= c` parses as `a \| (b \|= c)` |

The assignment-precedence divergence is the most consequential.  In
jq, `a \| b \|= c` is `(a \| b) \|= c` — the LHS is a "path expression
that may span pipes".  We chose the simpler reading because the
readdressing pattern (`.ltm.virtual[] \| .destination \|= ip(net, .)`)
is the most common mutating query, and it reads naturally as "for
each VS, update its destination".

## Value model

The runtime is mostly plain Python values (`str`, `int`, `float`,
`bool`, `None`, `list`).  Three wrapper types carry extra information:

### `ObjectRef`

A projected BIG-IP object.  Holds:

- `kind` — TMSH module+type (`"ltm virtual"`, `"ltm pool"`, …).
- `full_path` — `/Common/foo`.
- `fields` — dict of TMSH-spelt field names to projected values
  (strings, `PathRef`s, lists, sub-objects).
- `field_slots` — dict of field name → byte range of the value in the
  source.  Single-line property values land in this map; sub-blocks
  do not, which makes them non-writable in v1.
- `stanza_slot` — byte range of the whole stanza (header + body), used
  by `--scf` output and as a fallback for identity-rename verification.

### `PathRef`

A string reference to another object by full-path.  String-like in
every scalar context (string predicates compare on `.full_path`); the
evaluator transparently dereferences `PathRef` on field-access so
`.ltm.virtual[].pool.members[].address` walks VS → pool → member in
one chain.

### `Stream`

A flat sequence produced by `[]` or stream-returning builtins.
Distinct from `list` so the output formatter knows when to emit one
value per line versus a single JSON array.

## Tree shape

The DSL exposes the parsed `BigipConfig` as a nested mapping, with one
top-level child per recognised module:

```
.ltm
  .virtual["/Common/web_vs"].destination
                            .pool
                            .rules[]
                            .profiles[]
                            .persist[]
                            .policies[]
                            .snatpool
                            .source-address-translation
  .pool["/Common/web_pool"].members[].address
                                     .port
                                     .monitor
                           .monitor
                           .load-balancing-mode
  .node["/Common/n1"].address
  .rule["/Common/r1"].body
                     .refs.pools[]
                          .persists[]
                          .data-groups[]
  .profile     .monitor     .persistence
  .snatpool    .policy      .data-group
.net
  .route["/Common/default_gw"].network
                              .gw
                              .pool          (path-ref → ltm pool)
  .vlan["/Common/external"].tag
                           .interfaces[]
  .self["/Common/198.51.100.5"].address
                               .vlan         (path-ref → net vlan)
                               .traffic-group
                               .allow-service[]
  .route-domain["/Common/0"].id
                            .vlans[]         (path-refs → net vlan)
  .port-list["/Common/web_ports"].ports[]
  .interface["1.1"].media-fixed
  .dns-resolver["/Common/r1"].route-domain   (path-ref → net route-domain)
                              .forward-zones[]
  .tunnels-tunnel["/Common/t1"].profile      (path-ref → ltm profile)
                               .local-address
                               .remote-address
                               .description
  .stp["/Common/cist"].interfaces[]
.sys
  .dns[].name-servers[]                    (singleton; one entry)
  .ntp[].servers[]                         (singleton; one entry)
  .snmp[].agent-addresses[]                (singleton; one entry)
        .communities[]
  .global-settings[].hostname              (singleton; one entry)
  .provision["ltm"].level
  .folder["/Common"].traffic-group
  .file-ssl-cert["/Common/f5.crt"].source-path
                                  .cache-path
  .file-ssl-key["/Common/f5.key"].source-path
                                 .passphrase
  .management-route["/Common/default"].gateway
                                      .network
```

Singletons (``sys.dns``, ``sys.ntp``, ``sys.snmp``,
``sys.global-settings``) have no full-path — they're stored under
the empty-string key, so ``.sys.dns[]`` streams the one entry and
``.sys.dns[""]`` is an exact lookup.

```
.security
  .firewall-port-list["/Common/web"].ports[]
  .firewall-rule-list["/Common/rl1"].rules[]
  .firewall-config-entity-id["/Common/id1"].entity-id
  .ip-intelligence-policy["/Common/ip-intel"].name
  .protocol-inspection-compliance-map["/Common/m1"].insp-id
                                                   .key-type
                                                   .value-type
  .protocol-inspection-compliance-objects["/Common/o1"].insp-id
                                                       .type
  .device-id-attribute["/Common/att01"].id
.apm
  .access-policy["/Common/p1"].start-item       (path-ref → policy-item)
                              .default-ending  (path-ref → policy-item)
                              .items[]         (path-refs → policy-item)
  .policy-item["/Common/i1"].caption
                            .color
                            .item-type
                            .agents[]          (path-refs → policy-agent)
  .policy-agent["/Common/a1"].agent-type
                             .customization-group
  .customization-source["/Common/cs"].name
  .oauth-db-instance["/Common/oauthdb"].description
  .ssh-security-config["/Common/cfg"].ciphers[]
                                     .hmacs[]
                                     .kex-methods[]
  .default-report[].report-name              (singleton; one entry)
                   .user
```

PathRefs auto-deref from ``access-policy`` into the referenced
``policy-item`` and from ``policy-item.agents[]`` into the matching
``policy-agent`` — chained queries like
``.apm.access-policy[].start-item.caption`` walk
``access-policy → policy-item → caption`` in one step.

PathRefs cross module boundaries: `.net.self[].vlan.tag` walks
`net self → net vlan → tag` in one chain, and
`.ltm.virtual[].pool.members[].address` walks the existing
`virtual → pool → member → address` chain — same auto-deref engine.

Unmodelled kinds (`apm.*`, `security.*`, `sys.*`, `cm.*`, `pem.*`, …)
still parse — every stanza lands in `cfg.generic_objects` with full
byte ranges — and are reached by source-level operations
(`rename_partition` cascades, `--scf` selection through grep / a real
SCF concatenation), but they're not navigable from the DSL in v1.
Follow-on rounds will add typed projection for the high-value modules.

The full per-kind field map lives in `_KIND_FIELD_MAPS` in
`projection.py`; that table is the single source of truth for which
field names are valid and how they map to dataclass attributes.

## Addressing

Four ways to land on a value inside a container:

| Form | Semantics |
|---|---|
| `.ltm.virtual[]` | Stream every entry. |
| `.ltm.virtual["/Common/web_vs"]` | Exact full-path subscript. |
| `.ltm.virtual["~pattern"]` | Regex subscript — every key whose full-path matches the pattern. |
| `.ltm.virtual.web_vs` | Partition shorthand — bare name resolves to `/Common/web_vs` when unambiguous.  Raises if the same basename exists in two partitions. |

## Operators and precedence

From highest to lowest:

1. Unary `-`, `not`
2. `*`, `/`
3. `+`, `-`
4. `==`, `!=`, `<`, `<=`, `>`, `>=`
5. `and`
6. `or`
7. `|` (pipe)
8. `=`, `|=`, `+=`, `-=` (trailing on a pipe-stage)
9. `;` (statement separator)

## Assignment semantics

- `path = expr` — set the target field to `expr`.  `expr` is evaluated
  against the *outer* input, not against the path's current value.
- `path |= expr` — set the target to `path | expr`.  `.` inside `expr`
  is rebound to the path's current value, which makes
  `.destination |= sub(., ":443", ":8443")` read naturally.
- `path += expr` — numeric add, string concat, or list append.
- `path -= expr` — numeric sub, or remove-by-equality from a list.

### Identity-field writes

Assigning to `.name` or `."full-path"` auto-routes through
`core.bigip.rewrite.rename_object`, which rewrites the object's header
and every reference to it (configuration properties, iRule body
command arguments, pool-member addresses).  A line of the form

```
renamed /Common/old -> /Common/new (N occurrence(s))
```

is printed to stderr so the multi-stanza rewrite is visible.  No
threshold or confirmation prompt — the change is opt-in by being part
of the query, and the dry-run unified diff (the default) shows the
full impact before `--write` / `--in-place` is added.

### Edit plan

The evaluator collects every `Assignment` node's resolved target into
an `EditPlan`.  When evaluation finishes, the planner:

1. Applies any **prefix-cascade rewrites** queued by builtins like
   `rename_partition` (token-bounded regex prefix substitution against
   the whole source).  Verifies the result still parses.
2. Splits the remaining ops into identity writes and field writes.
3. Routes identity writes through `rename_object`, threading the
   evolving source between successive renames.  `|=` on an identity
   field is admitted — the evaluator has already computed the new
   value, the planner just hands it to `rename_object` like any
   other rename.
4. Slots field writes by byte range; rejects edits without a
   `field_slot` (compound sub-block values are not writable in v1).
5. Sorts field-write slots by offset, checks for overlaps, raises
   `EditError` on conflict.
6. Splices the new text in a single forward pass.

Mixing prefix-cascade rewrites and field edits inside a *single*
statement is rejected — a prefix rewrite shifts byte offsets, and
the field-slot ranges captured at projection time would target the
wrong span after the rewrite.  Split them with `;` and the planner
will apply each statement against the post-rewrite source.

The output is one `AppliedSource` per touched URI, surfacing the new
text, the `RenameReport` list (including synthetic reports for
cascade rewrites), and the field-edit count.  The CLI verb chooses
between unified diff, stdout, and in-place write based on flags.

## Builtins

Builtin functions are registered through the `@_register(...)`
decorator in `builtins.py`.  Each registration captures the
signature(s), summary, category, examples, arity bounds, and whether
it is a special form (evaluator-driven, like `select` / `map`).

The same registry feeds:

- runtime dispatch in `evaluator._eval_call`;
- the `--help-builtins` action in `explorer/verbs/f5/query.py`;
- the test that asserts every builtin has at least one example and
  one signature.

To add a builtin, drop a new `@_register(...)` decorator on a Python
function in `builtins.py`.  No other plumbing is required — the help
text and dispatch table both pick it up automatically.

### Categories

- **net** — `ip`, `net`, `host`, `port`, `in_cidr`, `route_domain`,
  `with_route_domain`
- **path** — `partition`, `basename`, `with_partition`
- **rename**
  - `rename(old, new)` — token-bounded single-object rename across
    the whole source (header + every reference, including
    references inside iRule body command arguments).  Same engine
    the `f5 rename` verb shells out to; tolerant of zero-match.
  - `rename_partition(old, new)` — cascading prefix rewrite that
    moves every object in a partition, including references in
    compound values like destination addresses, pool-member names,
    and iRule body literals; also renames the `auth partition`
    stanza header itself when present.
- **string** — `startswith`, `endswith`, `contains`, `match`, `sub`,
  `gsub`, `split`, `join`, `upcase`, `downcase`
- **stream** — `keys`, `values`, `first`, `last`, `count`, `unique`,
  `sort`, `any`, `all`, `select` (special form), `map` (special form)
- **value** — `length`, `kind`, `path`, `defined`, `type`
- **graph** — `refs`, `referenced_by` (forwards to `core.bigip.grep`)

### Rename verb integration

`f5 rename old new file.conf` is a thin shell over the query engine:
the verb constructs a `rename(OLD, NEW)` expression and runs it
through `run_query`.  Routing both verbs through one engine keeps the
rename logic in one place — improvements to `rename_object`, the
iRule body walk, and the diff renderer are inherited automatically.
The CLI surface (positional `old new path`, `--write` / `--in-place` /
unified-diff default, stderr "renamed X -> Y (N occurrence(s))"
summary, exit 1 on zero-match warning) is preserved exactly.

The `EditOp.strict` flag (default `True`) distinguishes
DSL-driven identity assignments (`.x["/Common/y"].name = "z"`,
strict — raises on zero-match because the user named the object
explicitly) from search-and-replace style renames driven by the
`rename()` builtin (non-strict — yields a no-op AppliedSource so
the CLI can surface the no-match with a warning + exit code 1).

### Partition and route-domain transforms

Two flavours of partition operation cover the common workflows:

1. **Whole-partition migration** — `rename_partition("Common",
   "Tenant_A")`.  Emits a `PrefixRewrite` op that the planner applies
   as a token-bounded regex substitution on the entire source.
   This catches every occurrence of `/Common/`, including the
   structural prefix on destination addresses
   (`destination /Common/10.10.0.5:443`) and pool-member identifiers
   (`/Common/n1:80`).  It also rewrites the bare `auth partition
   Common` header.

2. **Selective rename** — `.ltm.pool["~^/Common/"] | .name |=
   with_partition(., "Tenant_A")`.  Routes each match through
   `rename_object` (token-bounded full-path replacement).  Moves
   only the chosen kind; other objects in `/Common/` stay put, and
   the rename is restricted to standalone object identifiers — it
   does **not** touch compound values that happen to share the
   `/Common/` prefix.

Pick whole-partition migration when the partition itself is moving;
pick selective rename when you want one kind to move and the others
to stay.

Route domains attach to addresses with a `%<n>` suffix.  Three
builtins cover them:

- `route_domain(value)` — returns the route-domain part of a
  destination string, or `null` when none is present.
- `with_route_domain(value, rd)` — sets, replaces, or strips
  (`""`/`null`) the route domain on a destination string.
- `ip(network, source)` — when rebasing into a new network, the
  route domain on `source` is preserved by default along with the
  partition prefix and port.  Use `with_route_domain` afterwards to
  change it.

All address builtins share a single tokeniser
(`_split_destination` in `builtins.py`) that returns
`(partition, address, route_domain, port)`.  Adding new partition or
RD-aware operations means dispatching that tuple and re-joining via
`_rebuild_destination` — no ad-hoc string slicing.

## iRule sub-tree (v1)

`.ltm.rule["/Common/r1"]` exposes:

- `.body` — the raw Tcl source.
- `.refs.pools[]` / `.refs.persists[]` / `.refs.data-groups[]` — the
  list of object references extracted by
  `core.bigip.irules_refs.extract_irules_object_references`.  These
  are the same edges `f5 grep` walks, so the two verbs always agree on
  what an iRule "uses".

Writes inside an iRule body are restricted to those reference slots
in v1, and they happen via the same `rename_object` text engine so
the rewrite covers every `pool foo` / `persist add ... foo` / `class
match ... foo` occurrence.  A general command-argument editor (range
each arg, allow `.commands[].args[0] |= ...`) is deferred to v2 once
the iRule parser exports byte-level ranges for every token.

## Output modes

- `auto` (default) — SCF stanzas when every value is a writable
  object; one value per line when every value is a scalar; JSON
  otherwise.
- `scf` — every value rendered as an SCF stanza (with header), or
  coerced through `str()` when no stanza slot is available.
- `raw` — one scalar per line, no quoting.
- `paths` — print the full-path of each object or path-ref.
- `json` — `json.dumps([...], indent=2)`, with objects serialised as
  `{"kind", "full-path", "fields"}` maps.

## Multi-file behaviour

Each input file is its own root.  In `auto` / `scf` / `raw` / `paths`
output modes the renderer emits a `# === <uri> ===` header before
each file's values when more than one file was supplied.  Edits are
applied per file; identity renames do not propagate across files —
both `f5 rename` and `f5 query` operate on one source at a time.  To
perform a rename across a fan-out of partition files, either
concatenate them first with `f5 merge` (or `cat`) and run the rename
once over the combined SCF, or run `f5 rename` separately against
each file (a shell loop is enough).

## Exit codes

- `0` — query produced at least one value or applied at least one edit.
- `1` — read-only query produced no values.
- `2` — parse / lex / eval / edit error, or a CLI argument problem.

## Help layers

The CLI verb exposes four kinds of help:

- `f5 query --help` — argparse summary plus example block.
- `f5 query --help-dsl` — grammar reference (this document, abridged).
- `f5 query --help-builtins [NAME]` — function catalogue (generated
  from the registry).
- `f5 query --help-examples` — cookbook of common one-liners
  (generated from `examples.py`).

The same content powers the KCS feature note and the design doc, so
end-users get one consistent surface whether they read the terminal,
the rendered docs, or the source.
