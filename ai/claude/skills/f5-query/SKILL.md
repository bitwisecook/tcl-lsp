---
name: f5-query
description: "Translate a natural-language question about an F5 BIG-IP configuration into the jq-flavoured ``f5 query`` DSL and run it. Use when the user asks anything like 'find all virtuals in /Common with default pool', 'rename pool X to Y across this config', 'which iRules reference data-group Z', 'list profiles attached to vs1', 'find unreferenced objects', or any other read-only or mutating query against a ``bigip.conf`` / ``.scf`` file. The DSL is jq-shaped on purpose so jq idioms transfer; the divergences live in :mod:`docs/design/f5-query-dsl.md` and :mod:`docs/design/f5-query-dsl-builtins.md`."
allowed-tools: Bash, Read
---

# F5 BIG-IP Query (natural-language → DSL)

The ``f5 query`` verb embeds a small jq-flavoured language for
querying and editing parsed BIG-IP configurations.  Your job: turn
the user's plain-English question into a query expression and run
it against the file(s) they point at.

## Workflow

1. **Confirm the input.**  The user names a file (or several).  If
   they didn't, ask which.  ``.conf`` and ``.scf`` are both fine;
   ``.ucs`` archives need ``f5 ucs-extract`` first.
2. **Translate the question.**  Use the recipe table below as a
   starting point; the full grammar lives in
   ``docs/design/f5-query-dsl.md`` (run ``f5 query --help-dsl`` to
   print a summary), and every builtin's signature + example is in
   ``docs/design/f5-query-dsl-builtins.md`` (or
   ``f5 query --help-builtins``).
3. **Run the query.**  Read-only by default — the diff preview is
   your friend.  Mutating queries (anything containing ``=``,
   ``+=``, ``-=``, ``|=``, or a builtin like ``rename`` /
   ``rename_partition``) print a unified diff; pass ``--write`` to
   send the rewritten config to stdout, or ``--in-place`` to
   overwrite the source file.
4. **Read the output and answer.**  ``--raw`` for one-value-per-
   line scripting output, ``--json`` for structured tooling output,
   ``--paths-only`` for full-paths only, default mode renders SCF
   stanzas when the result is a stream of objects.

## DSL primer (jq-flavoured)

The mental model is jq:

- ``.`` is the current value; ``.foo.bar`` reads nested fields.
- ``.X[]`` is a stream-generator (every entry of the X container).
- ``|`` pipes one stage's output into the next.
- ``[ ... ]`` is the array constructor — collects a stream into
  one list for aggregators (``[.X[].name] | sort | first``).
- Bare builtin names (``length`` / ``sort`` / ``unique`` / …)
  operate on the current value.
- ``select(predicate)`` filters; ``map(body)`` transforms.
- ``count``, ``unique``, ``sort``, ``any``, ``all``, ``contains``,
  ``startswith`` / ``endswith``, ``match`` (boolean — jq's
  ``test()`` not its ``match()``), ``str``.

The important divergences from jq:

| Concern | jq | f5 query |
|---|---|---|
| Function args | ``;`` separated | ``,`` separated |
| Stream concat ``,`` | yes | not present — use ``[ ... ]`` lists or ``;`` statements |
| ``test()`` | rich match objects | ``match()`` is boolean (the DSL has no rich-match equivalent — use ``sub`` / ``gsub`` for capture groups) |
| Truthiness | only ``false`` and ``null`` are falsey | also empty string / list / stream / PathRef / numeric 0 |
| Object literals ``{...}`` | yes | not in v1 — use ``--json`` output |
| String interpolation | yes | not in v1 — concat with ``+`` (auto-coerces scalars) |
| Comma operator inside ``[...]`` | yes | not in v1 — see error message at parse time |

## Recipes (natural language → query)

### Read-only

| User asks | Query |
|---|---|
| "list every virtual server" | ``.ltm.virtual[].name`` |
| "names of pools in /Common" | ``.ltm.pool[] \| select(partition(."full-path") == "Common") \| .name`` |
| "which virtuals use pool X" | ``.ltm.virtual[] \| select(.pool == "/Common/X") \| .name`` |
| "show vs1's full config" | ``.ltm.virtual["/Common/vs1"]`` |
| "iRule names attached to vs1" | ``.ltm.virtual["/Common/vs1"].rules[]`` |
| "pools without monitors" | ``.ltm.pool[] \| select(.monitor == "") \| .name`` |
| "find unreferenced pools" | use ``f5 cleanup`` instead — purpose-built for orphan detection |
| "virtuals listening on port 443" | ``.ltm.virtual[] \| select(endswith(.destination, ":443")) \| .name`` |
| "any iRule references the missing pool /Common/X?" | ``any(.ltm.rule[].refs.pools[] \| (. == "/Common/X"))`` |
| "report each pool's member count" | ``.ltm.pool[] \| .name + ": " + count(.members)`` |
| "every persistence profile inheriting from cookie" | ``.ltm.persistence[] \| select(."defaults-from" == "/Common/cookie") \| .name`` |
| "data-groups whose body contains 'foo'" | ``.ltm["data-group"][] \| select(contains(.records, "foo")) \| .name`` |
| "every kind of object in the config" | ``[.[]] \| count`` (note: ``.[]`` walks every kind across every module) |

### Mutating (always preview the diff first!)

| User asks | Query |
|---|---|
| "rename pool old to new" | ``rename("/Common/old", "/Common/new")`` |
| "move every object from /Common to /Tenant_A" | ``rename_partition("Common", "Tenant_A")`` |
| "set vs1's description" | ``.ltm.virtual["/Common/vs1"].description = "production HTTPS"`` |
| "attach iRule X to every VS in /Common" | ``.ltm.virtual[] \| select(partition(."full-path") == "Common") \| .rules += "/Common/X"`` |
| "remove iRule Y from vs1" | ``.ltm.virtual["/Common/vs1"].rules -= "/Common/Y"`` |
| "rename pool to match its destination" | not safe to script — use ``rename`` per VS, the destination is a compound value |

### CLI flags worth knowing

- ``--raw`` — one scalar per line (for shell pipelines).
- ``--paths-only`` — full-paths only (for ``xargs``).
- ``--json`` — structured output; multi-file emits one envelope
  ``[{"uri": ..., "values": [...]}, ...]`` (not adjacent arrays).
- ``--scf`` — render every value as an SCF stanza.
- ``--format scf|tmsh`` — output format for the rewritten config.
  ``--format tmsh`` produces a ``tmsh modify`` script suitable for
  piping to a remote device.  **Refused with ``--in-place``** —
  in-place writes always preserve the source format.
- ``--in-place`` — overwrite the input file.  Reads strictly UTF-8
  (refuses undecodable bytes rather than silently substituting
  U+FFFD).
- ``--write`` — print the rewritten config to stdout.
- ``--from-file FILE`` — read the query from a file (multi-line OK).

### Help layers

The CLI has self-discovering help — read these first when the
recipe table doesn't cover the user's question:

```bash
f5 query --help            # CLI flags
f5 query --help-dsl        # grammar reference
f5 query --help-builtins   # every builtin's signature + example
f5 query --help-builtins NAME  # one named builtin in detail
f5 query --help-examples   # worked-example cookbook
```

## Common gotchas

1. **``map(predicate)`` is many-to-many.**  ``map`` runs ``body``
   on each item and uses the same flattening as the pipe; a body
   that yields ``select(false)`` contributes zero outputs.
   ``map(select(...) | .field)`` is the canonical filter+transform
   idiom.
2. **No ``,`` inside ``[ ... ]``.**  ``[a, b]`` raises a clear
   parse error pointing at the comma.  Use one pipeline expression
   inside the brackets (``[stream | sort]``).
3. **``--in-place --format tmsh`` is refused.**  The dry-run diff
   is always SCF↔SCF, so writing tmsh to the source file would
   silently change its format.  Use ``--write`` for tmsh output.
4. **Strings are SCF-encoded on write.**  Field-edit values get
   double-quoted when they contain whitespace / quotes / commas;
   newlines / braces are *refused* with an ``EditError`` because
   they'd break the brace-balanced format.
5. **Multi-file output**: scalar / paths / SCF modes get
   ``# === <uri> ===`` banners; ``--json`` gets the single
   top-level envelope.

## When to defer to another tool

- **Orphan analysis** → ``f5 cleanup`` (walks the full ref graph
  in reverse-topological order; purpose-built for unreferenced
  objects).
- **Detailed lint findings** → ``f5 lint`` (every BIG-IP rule:
  empty pools, virtuals without pools, deprecated iRule commands).
- **Deep-link / reference graph** → ``f5 grep`` (full transitive
  reference discovery; the same engine LSP go-to-definition uses).
- **Whole-config diff** → ``f5 diff`` (round-trip-aware diff that
  ignores formatting differences).
- **Ad-hoc Tcl analysis** → ``f5 trace`` / ``f5 explain``.

## Output etiquette

- Always show the user the **exact query** you ran so they can
  re-execute it / tweak / save it.
- For mutating queries, **always run the dry-run first** and show
  the diff before suggesting ``--in-place``.
- For long results, **summarise** with counts + a representative
  sample rather than dumping the full list.
- When the query produces no values, say so explicitly and
  suggest the next step (e.g. partition filter mismatch — try
  ``select(partition(."full-path") == "Common")``).
