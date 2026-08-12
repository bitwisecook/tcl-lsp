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
[`docs/kcs/features/kcs-feature-bigip-query.md`](../../kcs/features/kcs-feature-bigip-query.md).

## Module map

```
rust/tcl-bigip-query/src/
  lib.rs              # crate root — re-exports the public API
  errors.rs           # QueryError hierarchy (lex / parse / eval / edit / builtin)
  value.rs            # runtime value model: ObjectRef, PathRef, Stream, scalars
  lexer.rs            # hand-rolled tokeniser
  ast.rs              # AST node types
  parser.rs           # recursive-descent parser
  projection.rs       # BigipConfig → navigable Container tree
  eval.rs             # walks the AST, collects edits, returns values
  builtins/           # plain + stream builtin function library (mod.rs + submodules)
  special.rs          # special-form builtins (select / map / paths / getpath / …)
  probes.rs           # network-probe + X.509 builtins (refs / referenced_by
                       # forward into tcl-bigip's grep/graph support)
  edit_plan.rs         # routes identity writes through rewrite::rename_object,
                       # detects conflicts, applies bottom-up
  rewrite.rs          # token-bounded rename engine used by edit_plan and the
                       # rename* builtins
  output.rs           # auto / scf / raw / paths / json renderers
  runner.rs           # high-level orchestration used by the CLI verb
  grammar.rs          # plain-text grammar for --help-dsl
  manual.rs           # combined --help-manual surface (grammar + builtins + examples)
  examples.rs         # worked-example cookbook for --help-examples
  architecture.rs     # multi-device architecture / tier detection
  inputs.rs           # side-input parsers (--input-json / -jsonl / -csv / -f5log)
rust/f5-cli/src/commands/query.rs
                     # clap plumbing + help actions for the `f5 query` verb
```

## Grammar

The DSL is a small pipeline language.  Each statement is a pipeline of
stages joined by `|`; statements are separated by `;`.

```
program       := pipeline (';' pipeline)*
pipeline      := comma_expr ('|' comma_expr)*
comma_expr    := pipe_stage (',' pipe_stage)*
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
primary       := literal | call | path | list_literal | if_expr
              | '(' pipeline ')'
if_expr       := 'if' pipeline 'then' pipeline
                 ('elif' pipeline 'then' pipeline)*
                 ('else' pipeline)? 'end'
list_literal  := '[' pipeline? ']'
path          := '.' | '.' field path_tail | '.' subscript path_tail
path_tail     := ('.' field '?'? | subscript '?'?)*
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
- `a, b` concatenates streams, and `[a, b, c]` collects the resulting
  stream into a list.
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
| Stream concat `,` | yes | yes — except inside function arguments and object entries, where comma remains the separator; wrap a comma stream in parens when it is a single argument or value |
| Conditional `if … then … elif … else … end` | yes | yes — with the broader `f5 query` truthiness described below |
| Regex test | `test("pat")` builtin | `["~pat"]` subscript form, **and** `match()` builtin (jq's `match()` returns match objects; ours returns boolean — equivalent to jq's `test()`) |
| Identifier hyphens | quoted only | bareword (`source-address-translation`) |
| Object literals `{...}` | yes | yes — `{name, dest: .destination}` bareword keys desugar to `key: .key`; stream-valued fields broadcast element-wise into one row per item |
| `expr as $x \| body` | yes | yes — streams iterate (one body call per item), plain lists (from an explicit `[...]` collector) bind once.  Right-associative so `.a[] as $x \| .b \| $x.c + ...` keeps `$x` bound across subsequent pipe stages |
| `$name` variable | yes (let-binding only) | also names each loaded source — `$ltm`, `$gtm`, ... — for cross-config queries.  Auto-named from filename stem; `--name N=PATH` overrides |
| String interpolation `"\(.x)"` | yes | not present in v1 |
| Optional path suffix `?` | yes | yes for path steps (`.foo?`, `.items[]?`, `.[expr]?`); `try-catch` is still absent |
| `//` / `try-catch` / `reduce` / `foreach` | yes | not present — practical query language, not a jq subset. (`paths` / `leaf_paths` / `getpath` / `setpath` / `del` / `delpaths` / `to_entries` / `from_entries` **are** implemented — see the `value` category in [`builtins.md`](builtins.md).) |
| Truthiness (`select`, `and`, `or`, `if`) | only `false` and `null` are falsey | also: empty string, empty list/stream, empty `PathRef`, numeric `0`, `null` (broader falsey set; closer to "empty / zero / absent" than jq's strict definition) |
| Assignment + pipe | `path \|= f` binds tight via custom precedence | `\|=` is a pipe-stage trailing operator, so `a \| b \|= c` parses as `a \| (b \|= c)` |
| `--format` flag | not applicable | `--format scf` (default) emits the rewritten config; `--format tmsh` emits a `tmsh modify` script suitable for piping to a live device.  Refused with `--in-place` (would silently overwrite SCF with a different file format) |
| Multi-file `--json` | not applicable | one envelope `[{"uri": ..., "values": [...]}, ...]`, not adjacent arrays |

The assignment-precedence divergence is the most consequential.  In
jq, `a \| b \|= c` is `(a \| b) \|= c` — the LHS is a "path expression
that may span pipes".  We chose the simpler reading because the
readdressing pattern (`.ltm.virtual[] \| .destination \|= ip(net, .)`)
is the most common mutating query, and it reads naturally as "for
each VS, update its destination".

## Value model

The runtime value is the [`Value`](../../../rust/tcl-bigip-query/src/value.rs)
enum: the plain scalar/container variants (`Str`, `Int`, `Float`, `Bool`,
`Null`, `List`, `Object`) plus three wrapper variants that carry extra
information:

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
  .snatpool    .data-group
  .policy["/AS3/app/p1"].strategy
                        .controls[]
                        .requires[]
                        .rules[].name
                                .ordinal
                                .conditions[].operand
                                              .selector
                                              .operator
                                              .values[]
                                              .negate
                                .actions[].target
                                           .verb
                                           .pool   (path-ref → ltm pool)
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

```
.cm
  .cert["/Common/dtca.crt"].cache-path
                           .checksum
                           .revision
  .key["/Common/dtca.key"].cache-path
                          .checksum
  .device["/Common/host1"].hostname
                          .management-ip
                          .version
                          .cert            (path-ref → cm cert)
                          .key             (path-ref → cm key)
  .device-group["/Common/dg1"].auto-sync
                              .devices[]   (path-refs → cm device)
  .traffic-group["/Common/tg1"].unit-id
  .trust-domain["/Common/Root"].ca-cert    (path-ref → cm cert)
                               .ca-cert-bundle
                               .ca-key     (path-ref → cm key)
                               .ca-devices[] (path-refs → cm device)
                               .trust-group (path-ref → cm device-group)
                               .guid
                               .status
```

PathRefs from ``cm device.cert`` / ``cm device.key`` and from every
``cm trust-domain`` reference auto-deref into the target object —
``.cm.trust-domain[].trust-group.devices`` walks
``trust-domain → device-group → devices[]`` end-to-end.

```
.gtm
  .datacenter["/Common/dc1"].contact
                            .location
  .server["/Common/s1"].datacenter         (path-ref → gtm datacenter)
                       .monitor
                       .product
                       .addresses[]
                       .virtual-servers[]
  .pool["/AS3/app/p1"].record-type         (a / aaaa / cname / mx / …)
                      .members[]
                      .monitor
                      .load-balancing-mode
                      .ttl
  .wideip["/AS3/app/w1"].record-type
                        .pools[]           (path-refs → gtm pool)
                        .aliases[]
                        .pool-lb-mode
                        .last-resort-pool  (path-ref → gtm pool)
  .prober-pool["/Common/pp"].members[]     (path-refs → gtm server)
  .region["/Common/r1"].region-members[]
  .rule["/AS3/app/r1"].body
```

``gtm pool a|aaaa|cname|mx|srv|naptr`` and ``gtm wideip <type>`` are
merged into a single container each, with ``record-type`` carrying
the DNS record kind.  PathRefs from ``wideip.pools[]`` /
``last-resort-pool`` deref into the unified ``gtm pool`` so
``.gtm.wideip[].pools[].ttl`` walks the full chain.

PathRefs cross module boundaries: `.net.self[].vlan.tag` walks
`net self → net vlan → tag` in one chain, and
`.ltm.virtual[].pool.members[].address` walks the existing
`virtual → pool → member → address` chain — same auto-deref engine.

`apm.*`, `security.*`, `sys.*`, `cm.*`, `pem.*`, `auth.*`, `vcmp.*`,
`cli.*`, `api-protection.*`, `asm.*`, `ilx.*`, `wom.*`, and
`analytics.*` are all projected and navigable — `_MODULE_KINDS` in
`projection.py` enumerates the supported kinds per module.  Any TMSH
stanza the parser sees but no typed projection covers still lands in
`cfg.generic_objects` with full byte ranges and is reachable via
source-level operations (`rename_partition` cascades, `--scf` selection
through grep / a real SCF concatenation), but is not navigable from
the DSL.

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
`dialects.f5.bigip.rewrite.rename_object`, which rewrites the object's header
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
- the `--help-builtins` action in `tooling/f5/verbs/query.py`;
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
- **value** — `length`, `kind`, `path`, `defined`, `type`, `str`
- **graph** — `refs`, `referenced_by` (forwards to `dialects.f5.bigip.grep`)

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
  `dialects.f5.bigip.irules_refs.extract_irules_object_references`.  These
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
- `scf` — every value rendered as an SCF stanza (with header).  When
  no stanza slot is available, scalars route through the same
  `_scalar_str` formatter the `raw` mode uses (one canonical scalar
  formatter, not bare `str()` — bools render as `true` / `false`,
  `None` as `null`, `PathRef` as its full-path); object / list /
  stream values that aren't scalars are refused with a clear error
  asking for `--paths-only` / `--json` instead.
- `raw` — one scalar per line, no quoting.  Same scalar formatter as
  `scf`.
- `paths` — print the full-path of each object or path-ref.
- `json` — `json.dumps([...], indent=2)`, with objects serialised as
  `{"kind", "full-path", "fields"}` maps.

## CLI flags that affect output

- `--format scf` (default for the rewritten config) emits the source
  with edits applied in-place, preserving comments / whitespace /
  field order.
- `--format tmsh` emits a `tmsh modify` script suitable for
  `tmsh load /sys config from-terminal merge` or piping to a remote
  device.  Refused when combined with `--in-place`: the dry-run diff
  is always SCF↔SCF, so writing tmsh to the source file would
  silently change the file's format.

## Multi-file behaviour

Each input file is its own root.  In `auto` / `scf` / `raw` / `paths`
output modes the renderer emits a `# === <uri> ===` header before
each file's values when more than one file was supplied.

In `--json` mode, multi-file invocations emit a single top-level
envelope rather than adjacent per-file arrays (which would not be
valid JSON):

```json
[
  {"uri": "file:///a.conf", "values": [...]},
  {"uri": "file:///b.conf", "values": [...]}
]
```

Single-file `--json` invocations stay flat — just the values array,
no envelope — so the simple case keeps the expected shape.

Edits route back to the source the rewritten object came from.  In
the default per-file iteration mode the runner walks each loaded
config in turn as the primary input and applies edits to that
source.  Cross-file behaviour comes in two shapes:

- **`$name` variable** — load several configs and address a specific
  one by its filename-stem name (or an explicit `--name N=PATH`).
  `$ltm.ltm.virtual[].destination = "..."` writes to the source
  bound under `$ltm`, regardless of which source was iterating.
- **`--merge`** — every loaded source becomes one logical namespace.
  `.ltm.virtual[]` returns virtuals from every input and `refs` /
  `referenced_by` walk references across files; edits still route
  back to the originating source.  Refuses to merge when two
  sources define the same `(kind, full-path)` — namespace or
  redact the inputs first.

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
