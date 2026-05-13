# F5 query DSL — builtin function reference

> **Audience:** Developer / Maintainer
> **Type:** Reference

**This page is generated from the builtin registry in
[`core/bigip/query/builtins.py`](../../core/bigip/query/builtins.py).
Edit that registry, not this file.**  The generator lives at
[`scripts/dev/gen_query_builtins_doc.py`](../../scripts/dev/gen_query_builtins_doc.py);
a CI check asserts the on-disk file is up to date.

This is the **canonical per-function reference** for every builtin the
`f5 query` DSL exposes.  For grammar, value-model, edit-pipeline, and
architectural context see [`f5-query-dsl.md`](f5-query-dsl.md); for
the user-facing feature overview and worked-example KCS notes start
from
[`../kcs/features/kcs-feature-bigip-query.md`](../kcs/features/kcs-feature-bigip-query.md).

The same per-function reference is available offline through the
verb's own help action — ``f5 query --help-builtins NAME`` prints
exactly the same content for one builtin.

## Categories


- **[stream](#stream)** — Sequence-shaped operations: filter (`select`), transform (`map`), aggregate (`any` / `all` / `count` / `unique` / `sort`), and the object-introspection helpers (`keys` / `values` / `first` / `last`).
  - [`all`](#all), [`any`](#any), [`count`](#count), [`first`](#first), [`keys`](#keys), [`last`](#last), [`map`](#map), [`select`](#select), [`sort`](#sort), [`unique`](#unique), [`values`](#values)
- **[string](#string)** — String predicates and rewrites: substring / prefix / suffix tests, regex `match` / `sub` / `gsub`, plain `split` / `join`, casing.
  - [`contains`](#contains), [`downcase`](#downcase), [`endswith`](#endswith), [`gsub`](#gsub), [`join`](#join), [`match`](#match), [`split`](#split), [`startswith`](#startswith), [`sub`](#sub), [`upcase`](#upcase)
- **[path](#path)** — BIG-IP full-path string helpers — extract the partition or basename, swap a partition prefix.  These are *string* transforms; they don't move objects.  For object renames, reach for the **rename** category.
  - [`basename`](#basename), [`partition`](#partition), [`with_partition`](#with_partition)
- **[rename](#rename)** — Cascading rename operations — `rename` for one object, `rename_partition` for every object in a partition.  Both route through the same token-bounded engine `f5 rename` uses, so references inside iRule bodies and compound values (destination addresses, pool-member identifiers) are rewritten consistently.
  - [`rename`](#rename), [`rename_partition`](#rename_partition)
- **[net](#net)** — IP-address arithmetic and route-domain helpers.  The `ip(net, src)` rebase is the workhorse of bulk readdressing; `with_route_domain` sets / replaces / strips the `%rd` suffix.
  - [`host`](#host), [`in_cidr`](#in_cidr), [`ip`](#ip), [`net`](#net), [`port`](#port), [`route_domain`](#route_domain), [`with_route_domain`](#with_route_domain)
- **[graph](#graph)** — Forward / reverse references across the same edge model `f5 grep` walks.  One hop deep; multi-hop walks belong in `f5 grep` for now.
  - [`referenced_by`](#referenced_by), [`refs`](#refs)
- **[value](#value)** — Type / identity introspection: `kind` (TMSH kind), `path` (full-path), `length`, `defined`, `type`.
  - [`defined`](#defined), [`kind`](#kind), [`length`](#length), [`path`](#path), [`str`](#str), [`type`](#type)

## stream

Sequence-shaped operations: filter (`select`), transform (`map`), aggregate (`any` / `all` / `count` / `unique` / `sort`), and the object-introspection helpers (`keys` / `values` / `first` / `last`).

### `all`

True when every item of a list or stream is truthy.

**Signatures**

- `all(value: list | stream) -> boolean`

**Details**

Tests whether **every** item in a list or stream is truthy.
Short-circuits on the first falsy item.  An empty input returns
``true`` (vacuous truth — there's no falsy item to find).

Common pattern: validate an invariant across the config —
``all(.ltm.virtual[].pool | startswith(., "/Common/"))``
is "are all default pools in Common?".  Pipe iterates the stream
of pools, ``startswith`` runs per item, ``all`` collapses.

Related: ``any``, ``select``, ``map``.

**Examples**

```
all(.ltm.virtual[].pool | startswith(., "/Common/"))
all(.ltm.virtual[].pool | . != "")          # every VS has a default pool?
```

### `any`

True when at least one item of a list or stream is truthy.

**Signatures**

- `any(value: list | stream) -> boolean`

**Details**

Tests whether **any** item in a list or stream is truthy.
Truthy means non-null, non-empty-string, non-empty-collection,
and non-zero — same conventions as ``select``.

Used most often with a per-item predicate piped through a
stream:
``any(.pool.members[].address | in_cidr(., "10.0.0.0/8"))``
is "does any member's address lie in 10/8?".  The pipe iterates
the stream of addresses (each becomes ``.``), produces a stream
of booleans, and ``any`` collapses it.

Note on ``map``: piping a stream into ``map(predicate)`` invokes
``map`` once **per item** — each call returns a single-element
list ``[predicate(item)]``.  ``any`` flattens one level of
list-of-lists so ``any(stream | map(predicate))`` Just Works,
but the predicate form (``any(stream | predicate)``) is the
idiomatic shape.

Short-circuits — stops at the first truthy item.

Related: ``all``, ``select``, ``map``.

**Examples**

```
any(.pool.members[].address | in_cidr(., "10.0.0.0/8"))
.ltm.virtual[] | select(any(.pool.members[].address | in_cidr(., "10.0.0.0/8"))) | .name
```

### `count`

Count the items in a list or stream.

**Signatures**

- `count(value: list | stream) -> integer`

**Details**

Alias for ``length`` restricted to lists and streams.  Reads
naturally in filter prose: ``select(.rules | count > 0)``.

Related: ``length``.

**Examples**

```
[.ltm.virtual[]] | count                 # number of VSes
.ltm.virtual[] | select(.rules | count > 0) | .name
```

### `first`

Return the first item of a list or stream, or null when empty.

**Signatures**

- `first(value: list | stream) -> any`

**Details**

Returns the first element of a list or stream.  Returns ``null``
(not an error) when the input is empty, so it's safe to apply to
fields that may have no entries (``first(.rules)`` on a VS with
no attached iRules returns null).

Useful in combination with sorting / unique-ing to pick the
"smallest" or "first by name" entry.

Related: ``last``, ``count``, ``length``, ``sort``.

**Examples**

```
first(.rules)
[.ltm.virtual[].name] | sort | first     # alphabetical first VS
```

### `keys`

Return the field names of an object as a sorted list.

**Signatures**

- `keys(value: object) -> list[string]`

**Details**

Returns the field-name keys of an :class:`ObjectRef` (or a plain
``dict``) as a sorted list.  Useful for introspecting unfamiliar
object kinds or for projecting "which fields does each kind
expose?".

Returns the keys, not the values — pair with ``values`` (or just
index back through the object) to fetch the values too.

Raises ``BuiltinError`` for non-object inputs.

Related: ``values``, ``length``, ``type``.

**Examples**

```
keys(.ltm.virtual.web_vs)                # all field names of one VS
[.ltm.virtual[]] | first | keys          # discover the VS field set
```

### `last`

Return the last item of a list or stream, or null when empty.

**Signatures**

- `last(value: list | stream) -> any`

**Details**

Returns the last element of a list or stream.  Returns ``null``
when the input is empty.  Idiomatic for splitting "address:port"
style destinations: ``split(.destination, ":") | last``.

Related: ``first``, ``count``, ``sort``.

**Examples**

```
last(.rules)
split(.destination, ":") | last(.)        # port portion
```

### `map`

Apply the body to every item, returning the list of results.

**Signatures**

- `map(body) -> list`

**Details**

**Special form.**  ``map`` is the transform primitive — for each
item of the *input* (which must be a list / stream), it
evaluates *body* with ``.`` re-bound to that item and collects
the results into a list.

Output cardinality matches jq's ``map(f) == [.[] | f]`` rule:
each *body* invocation flattens through the same machinery the
pipe uses, so a body that produces

- one value contributes one element (the common case);
- a stream contributes every stream item;
- the ``select`` drop sentinel contributes zero elements
  (``map(select(predicate))`` is the canonical filter idiom).

So ``map`` is many-to-many in general, not strictly one-to-one.

The body can be any expression: a field projection, a builtin
call, a multi-stage pipeline, an arithmetic expression — `.` is
the current item throughout.

Common patterns:

- **Project a field of a list**: ``.rules | map(basename(.))`` —
  ``.rules`` is a list, the pipe passes it whole, ``map``
  iterates it.
- **Filter + transform**: ``map(select(.address) | .name)`` —
  drops items whose ``address`` is falsey, projects ``.name``
  on the survivors.  Zero outputs per dropped item.
- **Predicate over a stream** (don't use ``map`` for this): pipe
  the stream through the predicate instead —
  ``.pool.members[].address | in_cidr(., "10.0.0.0/8")``
  yields a stream of booleans suitable for ``any`` / ``all``.
- **Compose with sort + unique on a stream**: wrap with a list
  literal first so subsequent stages see one list:
  ``[.ltm.virtual[].name | partition(.)] | unique | sort``.

Related: ``select``, ``any``, ``all``, ``unique``, ``sort``.

**Examples**

```
.rules | map(basename(.))
[.ltm.virtual[].name | partition(.)] | unique | sort
any(.pool.members[].address | in_cidr(., "10.0.0.0/8"))
```

### `select`

Drop the current value unless the body evaluates to a truthy result.

**Signatures**

- `select(body) -> any | drop`

**Details**

**Special form.**  ``select`` is the filter primitive — for each
input value, it evaluates *body* against that value (with ``.``
re-bound to the current item) and emits the current value
unchanged when the result is truthy, dropping it otherwise.

Truthy values: non-null, non-empty-string, non-empty-list /
-stream, non-zero numbers, true booleans, non-empty path-refs.

Typical use is inside a pipeline that streams objects:
``.ltm.virtual[] | select(.pool != "")`` keeps only VSes with a
default pool.  Chain multiple ``select(...)`` to AND predicates
together; use ``or`` inside one body to OR.

Unlike most builtins, *body* may be any expression (not just a
value) — it's the unevaluated AST and is re-evaluated per item.
That makes ``select`` the source of every conditional flow in
the DSL: filter, partition, branch.

Related: ``map`` (transform every item instead of filtering),
``any``, ``all``, ``not``.

**Examples**

```
.ltm.virtual[] | select(.pool != "") | .name
.ltm.virtual[] | select(startswith(.name, "vs_prod_"))
.ltm.virtual[] | select(in_cidr(.destination, "10.0.0.0/8"))
.ltm.virtual[] | select(.rules | count > 0 and .rules | count < 5)
```

### `sort`

Return a sorted list.  Strings sort lexicographically; numbers numerically.

**Signatures**

- `sort(value: list | stream) -> list`

**Details**

Sorts the items of a list or stream and returns a list.
:class:`PathRef` items sort on their ``full_path``.  Heterogeneous
types in the same list raise — sort what comes back from a
projection (always one type) rather than mixed object/scalar
streams.

Stable (Python's ``sorted`` is).  Use the list-literal collection
idiom ``[.X[].name] | sort`` to gather a stream from ``[]`` before
sorting — bare ``... | sort`` after a stream would sort each item
individually, not the stream as a whole.

Related: ``unique``, ``first``, ``last``.

**Examples**

```
[.ltm.virtual[].name] | sort
[.ltm.virtual[].pool] | unique | sort    # sorted distinct pools
```

### `unique`

Return the unique items of a list, preserving first-seen order.

**Signatures**

- `unique(value: list | stream) -> list`

**Details**

De-duplicates a list or stream while preserving the original
order of first occurrence.  :class:`PathRef` items are compared
on their ``full_path``, so a stream that pulls the same pool
reference from many VSes collapses to one entry.

Unhashable items (rare — usually nested lists) fall back to a
linear scan, so worst-case is O(n^2); for the typical case of
strings, integers, and path-refs it's O(n).

Pairs nicely with ``sort`` for stable de-duplicated output:
``[.ltm.virtual[].pool] | unique | sort``.

Related: ``sort``, ``count``, ``map``.

**Examples**

```
[.ltm.virtual[].pool] | unique           # every distinct default pool
[.ltm.virtual[].name | partition(.)] | unique  # used partitions
```

### `values`

Return the field values of an object as a list.

**Signatures**

- `values(value: object) -> list`

**Details**

Returns the values of an :class:`ObjectRef`'s fields, ordered by
sorted field name.  Pairs with ``keys`` for matched
``(name, value)`` traversal.

The returned list mixes types — most BIG-IP objects carry a mix
of strings, path-refs, and nested lists — so subsequent
operations should be type-aware (``select(. != "")`` etc.).

Raises ``BuiltinError`` for non-object inputs.

Related: ``keys``, ``length``.

**Examples**

```
values(.ltm.virtual.web_vs)
.ltm.virtual.web_vs | values | map(type)   # type signature of one VS
```

## string

String predicates and rewrites: substring / prefix / suffix tests, regex `match` / `sub` / `gsub`, plain `split` / `join`, casing.

### `contains`

Test whether a string contains a substring, or a list contains a value.

**Signatures**

- `contains(value: string, needle: string) -> boolean`
- `contains(value: list, needle: any) -> boolean`

**Details**

Overloaded by the type of *value*:

- When *value* is a **string** (or :class:`PathRef`), tests
  substring membership: ``contains(.destination, ":443")``.
- When *value* is a **list / stream** (such as ``.rules`` —
  a list of :class:`PathRef`), tests element membership.
  :class:`PathRef` items and string needles are compared on
  their ``full_path`` so ``contains(.rules,
  "/Common/log_rule")`` works against the streamed list of
  path-refs.

Raises ``BuiltinError`` if *value* is neither a string nor a
list-like value.

Related: ``startswith``, ``endswith``, ``match``, ``any`` /
``all`` (for more general predicates over a stream).

**Examples**

```
contains(.destination, ":443")
contains(.rules, "/Common/log_rule")
.ltm.virtual[] | select(contains(.rules, "/Common/log_rule")) | .name
```

### `downcase`

Lowercase a string.

**Signatures**

- `downcase(value: string) -> string`

**Details**

Returns *value* with every ASCII letter converted to lowercase.
Accepts :class:`PathRef`.

Related: ``upcase``.

**Examples**

```
downcase(.name)
downcase("VS_PROD_WEB")                  # -> "vs_prod_web"
```

### `endswith`

Test whether a string ends with a suffix.

**Signatures**

- `endswith(value: string, suffix: string) -> boolean`

**Details**

Returns ``true`` when *value* ends with *suffix*.  Accepts
:class:`PathRef` for either argument; compared via ``full_path``.

Related: ``startswith``, ``contains``, ``match``.

**Examples**

```
endswith(.name, "_pool")
.ltm.virtual[] | select(endswith(.destination, ":443"))
```

### `gsub`

Replace every regex match in a string.

**Signatures**

- `gsub(value: string, pattern: string, replacement: string) -> string`

**Details**

Like ``sub`` but replaces **every** occurrence of *pattern* in
*value*.  Useful for blanket string rewrites inside iRule bodies
or data-group values.

For object full-path renames, prefer ``rename`` or
``rename_partition`` over a raw ``gsub`` — those route through a
token-bounded engine that won't touch substring collisions or
short-name references in unsafe contexts.

Related: ``sub``, ``match``, ``rename``, ``rename_partition``.

**Examples**

```
gsub(.body, "/Common/old_", "/Common/new_")
.ltm.virtual[].destination |= gsub(., "%5", "%7")  # bulk RD change
```

### `join`

Join a list of strings with a separator.

**Signatures**

- `join(values: list, separator: string) -> string`

**Details**

Joins a list (or stream) of strings into one string, separated
by *separator*.  :class:`PathRef` items are coerced to their
``full_path``, so ``join(.rules, ", ")`` works on the streamed
list of attached iRule references.

Useful for ad-hoc reports: ``.ltm.virtual[] | "\(.name): \(join
(.rules, ", "))"`` (when string interpolation lands) or
``join(map(.name, .ltm.virtual[]), "\n")`` to flatten a stream
of names.

Related: ``split``, ``map``, ``sort``.

**Examples**

```
join(.rules, ", ")
join(sort([.ltm.virtual[].name]), ", ")
```

### `match`

Regex-match a string; returns true when the pattern matches anywhere.

**Signatures**

- `match(value: string, pattern: string) -> boolean`

**Details**

Tests whether *pattern* (a Python regex) matches anywhere in
*value* (semantically ``re.search``, not ``re.match``).  Use
``^`` / ``$`` to anchor.

**jq users note.**  This DSL's ``match`` is a *boolean
predicate* — it corresponds to jq's ``test(pattern)`` builtin,
not jq's ``match(pattern)`` (which returns rich match objects
with capture groups, byte offsets, and named groups).  This DSL
has no equivalent of jq's match-object output; if you need
capture groups, use ``sub`` / ``gsub`` with a replacement
template instead.

An invalid regex raises ``BuiltinError`` with the underlying
``re.error`` reason — the pattern comes from the query author,
so a typo should fail loudly.

**Trust boundary.** ``match`` / ``sub`` / ``gsub`` and the
``[~"pattern"]`` regex subscript route their patterns through a
central guard that caps pattern length and refuses obvious
catastrophic-backtracking shapes (``(a+)+`` etc.).  Local CLI
use is trusted (the query author is the operator); the same
guard makes it safe to expose the DSL through MCP / chat /
editor command surfaces where the pattern can come from
untrusted input.  See ``_safe_regex_compile`` for the exact
shape filter.

For pure prefix/suffix or substring tests, prefer ``startswith``
/ ``endswith`` / ``contains`` — they're cheaper and read better.

Note: the **regex subscript** form ``.ltm.virtual["~pattern"]``
is a separate, more efficient mechanism for filtering keys
inside a container — reach for ``match`` when you need to test
a *value* against a pattern, and for the subscript when you're
selecting *keys*.

Related: ``sub``, ``gsub``, ``startswith``, ``endswith``,
``contains``.

**Examples**

```
match(.name, "^vs_prod_.*")
.ltm.virtual[] | select(match(.destination, ":(80|443)$")) | .name
```

### `split`

Split a string on a separator.  Returns a list.

**Signatures**

- `split(value: string, separator: string) -> list[string]`

**Details**

Splits *value* on every occurrence of *separator*, returning a
Python list of substrings.  The separator is not a regex — use
a literal string.

Common pattern: project a single string field, split it, and
extract a component.  ``.ltm.virtual[].destination | split(., ":")
| last(.)`` projects the port part of every destination.

Related: ``join`` (the inverse), ``sub`` / ``gsub`` (for regex
rewrites).

**Examples**

```
split(.destination, ":")
split(.destination, ":") | last(.)        # port portion
```

### `startswith`

Test whether a string starts with a prefix.

**Signatures**

- `startswith(value: string, prefix: string) -> boolean`

**Details**

Returns ``true`` when *value* begins with *prefix*.  Accepts
:class:`PathRef` for either argument (compared via the
``full_path``), so ``startswith(.pool, "/Common/")`` works even
though ``.pool`` is a path-ref, not a plain string.

Use ``match`` when you need pattern-based matching.

Related: ``endswith``, ``contains``, ``match``.

**Examples**

```
startswith(.name, "vs_prod_")
.ltm.virtual[] | select(startswith(.name, "vs_dev_")) | .name
```

### `sub`

Replace the first regex match in a string.

**Signatures**

- `sub(value: string, pattern: string, replacement: string) -> string`

**Details**

Replaces the **first** occurrence of *pattern* in *value* with
*replacement* and returns the new string.  *pattern* is a Python
regex; *replacement* may use ``\1`` / ``\g<name>`` backrefs.

Use ``gsub`` to replace every match instead.  An invalid pattern
raises ``BuiltinError``.

Pairs naturally with ``|=`` to rewrite a property in place:
``.ltm.virtual[].name |= sub(., "^vs_dev_", "vs_qa_")``.  When
the LHS is a stream of identity-field paths, each match is
rewritten through ``rename_object`` — references update along
with the headers.

Related: ``gsub``, ``match``, ``rename`` (for full-path
identity renames the engine already understands).

**Examples**

```
sub(.name, "^vs_dev_", "vs_qa_")
.ltm.virtual[].destination |= sub(., ":443$", ":8443")
```

### `upcase`

Uppercase a string.

**Signatures**

- `upcase(value: string) -> string`

**Details**

Returns *value* with every ASCII letter converted to uppercase.
Accepts :class:`PathRef`; the result is a plain string (the path
is normalised).  Use locale-aware casing helpers in Python if
you need them — this wrapper just calls ``str.upper``.

Related: ``downcase``.

**Examples**

```
upcase(.name)
upcase("vs_prod_web")                    # -> "VS_PROD_WEB"
```

## path

BIG-IP full-path string helpers — extract the partition or basename, swap a partition prefix.  These are *string* transforms; they don't move objects.  For object renames, reach for the **rename** category.

### `basename`

Return the last segment of a full-path (``/Common/foo`` → ``foo``).

**Signatures**

- `basename(path: string) -> string`

**Details**

Returns everything after the last ``/`` in a path string.  For a
bare name (no slashes) the input is returned unchanged.

Pairs naturally with ``|=`` to strip the partition prefix from
every reference in one statement:
``.ltm.virtual[].pool |= basename(.)``.

Related: ``partition`` (the inverse — partition segment),
``with_partition`` (replace partition, preserve basename).

**Examples**

```
basename("/Common/web_pool")             # -> "web_pool"
basename("/Tenant_A/api_pool")           # -> "api_pool"
basename("relative_name")                # -> "relative_name"
```

### `partition`

Return the partition name of a full-path (``/Common/foo`` → ``Common``).

**Signatures**

- `partition(path: string) -> string`

**Details**

Extracts the partition segment of a BIG-IP full-path — the bit
between the first and second ``/``.  An input that does not begin
with ``/`` (a relative reference, or a bare name) returns the
empty string.

Useful for group-by aggregates: ``[.ltm.virtual[].name |
partition(.)] | unique | sort`` enumerates every partition
that owns at least one virtual server.

Related: ``basename`` (the inverse — last segment),
``with_partition`` (replace the partition), ``rename_partition``
(move every object in a partition).

**Examples**

```
partition("/Common/web_pool")            # -> "Common"
partition("/Tenant_A/web_pool")          # -> "Tenant_A"
partition("relative_name")               # -> ""
```

### `with_partition`

Replace the partition of a full-path, preserving the basename.

**Signatures**

- `with_partition(path: string, partition: string) -> string`

**Details**

Returns ``/<partition>/<basename(path)>``.  This is a *string*
transform — by itself it just builds a new path string, it does
NOT move the underlying object.  Pair it with ``|=`` on an
identity field to actually move objects (the ``|=`` routes
through ``rename_object``):
``.ltm.pool["~^/Common/"] | .name |= with_partition(., "Tenant_A")``.

For a whole-partition migration (every object, not just pools),
reach for ``rename_partition`` — the cascade rewrites compound
values like destination addresses and pool-member names too,
which ``with_partition`` alone can't reach because they aren't
standalone object identifiers.

Raises ``BuiltinError`` when the new partition is empty.

Related: ``partition``, ``basename``, ``rename``,
``rename_partition``.

**Examples**

```
with_partition("/Common/web_pool", "Tenant_A")  # -> "/Tenant_A/web_pool"
.ltm.pool["~^/Common/"] | .name |= with_partition(., "Tenant_A")
```

## rename

Cascading rename operations — `rename` for one object, `rename_partition` for every object in a partition.  Both route through the same token-bounded engine `f5 rename` uses, so references inside iRule bodies and compound values (destination addresses, pool-member identifiers) are rewritten consistently.

### `rename`

Rename a BIG-IP object full-path and update every reference to it.  Routes through the same engine ``f5 rename`` uses (token-bounded regex substitution across the whole source, covering iRule body references and pool-member identifiers).

**Signatures**

- `rename(old: string, new: string) -> integer`

**Details**

Schedules a token-bounded source rewrite that replaces every
occurrence of *old* with *new*.  The substitution is the same one
``rename_object`` performs:

- The match is **token-bounded**, so renaming ``/Common/foo`` does
  not touch ``/Common/foobar`` or ``/Common/foo_extra``.
- **References inside iRule bodies** are rewritten too —
  ``pool /Common/foo``, ``persist add ... /Common/foo``,
  ``class match ... /Common/foo``, and so on.  Short-name
  references (``foo`` instead of ``/Common/foo``) are *not*
  rewritten; they're unsafe to handle by regex.
- **Pool-member identifiers** that embed the renamed name are
  rewritten (``/Common/foo:80`` → ``/Common/new:80``).

Unlike the DSL form ``.<kind>["/Common/old"].name = "/Common/new"``
(which raises when the LHS resolves to nothing), ``rename()`` is
**tolerant**: a zero-occurrence outcome yields a no-op
``AppliedSource`` with no rename report.  The ``f5 rename`` CLI
detects the no-op and surfaces it as ``warning: no occurrences
of <old> found`` with exit code 1 — matching its historical
behaviour.

Pre-flight checks: empty old/new names raise ``BuiltinError``;
``old == new`` is a no-op that returns 0 without scheduling an
edit.

Common patterns:

- ``f5 rename /Common/old /Common/new bigip.conf`` is exactly
  ``f5 query 'rename("/Common/old", "/Common/new")' bigip.conf``.
- Chain with a property edit using ``;`` so the second statement
  sees the renamed object:
  ``rename("/Common/old", "/Common/new") ;
  .ltm.pool["/Common/new"].monitor = "/Common/tcp"``.

Related: ``rename_partition`` (whole-partition cascade), the DSL
form ``.<kind>[X].name = Y`` (strict variant — errors when X
doesn't exist).

**Examples**

```
rename("/Common/old_pool", "/Common/new_pool")
rename("/Common/log_rule", "/Common/audit_rule")
rename("/Common/old", "/Common/new") ; .ltm.pool["/Common/new"].monitor = "/Common/tcp"
```

### `rename_partition`

Rename a BIG-IP partition by rewriting every textual occurrence of the ``/<old>/`` prefix across the whole source.  Token-bounded and covers object headers, references in config properties, destination address prefixes, pool-member identifiers, iRule body literals, and the ``auth partition`` stanza header.

**Signatures**

- `rename_partition(old: string, new: string) -> integer`

**Details**

A whole-partition migration: every textual ``/<old>/`` occurrence
in the source becomes ``/<new>/`` in one atomic rewrite.  The
pattern is token-bounded the same way ``rename`` is, so:

- Neighbouring identifiers like ``/<old>Ext/...`` are not touched.
- The trailing lookahead requires the next character to be the
  start of an identifier or address, so bare standalone
  occurrences of the partition name (which appear as property
  values in some kinds of objects) are not rewritten.

Crucially, this covers **compound values** that ``rename`` cannot:

- Destination addresses: ``destination /Common/10.10.0.5%5:443``
  — the prefix part of an address isn't a standalone object
  identifier, so ``rename`` won't touch it.  ``rename_partition``
  will.
- Pool-member identifiers: ``/Common/n1%5:80``.
- Bare ``/Common/`` mentions inside iRule body literals.

The ``auth partition Common { ... }`` stanza header is also
renamed when present — both halves of the migration land in one
statement.

Route domains, ports, and the bits inside compound values that
don't reference the partition (the host address, the port
number) are preserved exactly.

Pre-flight checks: empty names raise ``BuiltinError``; old
names containing ``/`` raise ``BuiltinError`` (pass bare
partition names, not paths); names not matching
``[A-Za-z0-9_.-]+`` raise.  ``old == new`` is a no-op.

The applier rejects mixing ``rename_partition`` with field edits
in the *same* statement — the prefix rewrite shifts byte offsets
and field-slot ranges captured at projection time would target
the wrong span.  Split them with ``;`` and the runner applies
each statement against the post-rewrite source.

Returns the count of textual matches the cascade will land on
(computed against the source as the builtin runs, before any
edits apply).

Related: ``rename`` (single-object), ``with_partition`` (string
transform, doesn't migrate references).

**Examples**

```
rename_partition("Common", "Tenant_A")
rename_partition("staging", "prod")
```

## net

IP-address arithmetic and route-domain helpers.  The `ip(net, src)` rebase is the workhorse of bulk readdressing; `with_route_domain` sets / replaces / strips the `%rd` suffix.

### `host`

Return just the address half of a BIG-IP destination, stripping any partition prefix, route domain, and ``:port`` suffix.

**Signatures**

- `host(value: string) -> string`

**Details**

BIG-IP destinations are spelt
``[/Partition/]address[%route-domain][:port]``.  ``host`` extracts
just the address — the partition prefix, route-domain suffix,
and port are all dropped:

- ``host("/Common/10.0.0.1%5:80")`` returns ``"10.0.0.1"``.
- Use ``route_domain``, ``port``, and ``partition`` to recover
  the parts ``host`` strips.

Falls back to returning the input verbatim when the string does
not parse as a destination, so it's safe to apply to fields that
might already be bare addresses.

Related: ``ip`` (one-arg form does the same normalisation),
``port``, ``route_domain``, ``partition``.

**Examples**

```
host(.destination)
host("/Common/192.168.1.1:80")           # -> "192.168.1.1"
host("/Common/10.0.0.1%5:443")           # -> "10.0.0.1"
```

### `in_cidr`

Test whether an address (or destination) lies within a CIDR network.  Partition prefixes and ``:port`` suffixes on the address are ignored.

**Signatures**

- `in_cidr(addr: string, network: string) -> boolean`

**Details**

Strips any partition prefix and ``:port`` suffix from *addr*,
parses what's left as an IP, and tests for membership in
*network*.  An unparseable address returns ``false`` (not an
error) so the helper is safe to use as a stream filter without
pre-validation.  An unparseable *network* raises
``BuiltinError`` — the network is supplied by the query author,
so a typo there should fail loudly.

Address-family mismatches return ``false``: an IPv4 host in an
IPv6 network is just "not in the network", not an error.

The route-domain portion of *addr* (``%5``) is ignored for the
membership test — RDs don't take part in the prefix arithmetic.

Related: ``ip``, ``net``, ``host``, ``route_domain``.

**Examples**

```
in_cidr("10.0.0.5", "10.0.0.0/8")              # -> true
in_cidr("/Common/10.0.0.5:80", "10.0.0.0/8")   # -> true
.ltm.virtual[] | select(in_cidr(.destination, "10.0.0.0/8")) | .name
```

### `ip`

Construct an IP-address string from a single string argument, or from a network + a source address whose host bits should be preserved (the readdressing helper).

**Signatures**

- `ip(addr: string) -> string`
- `ip(network: string, source: string) -> string`

**Details**

The one-argument form normalises a destination string to its bare
address: ``ip("/Common/192.168.1.1:80")`` returns ``"192.168.1.1"``,
stripping the partition prefix, the route domain, and the port.
Use the dedicated helpers (``partition``, ``port``,
``route_domain``) to recover those parts.

The two-argument form is the **readdressing helper** and is what
most query-driven migrations use.  It takes the host bits of
*source* and joins them to *network*'s prefix, producing a new
address in *network* that occupies the same host position as the
original.  Crucially, the partition prefix, route domain, and
port on *source* are **preserved**:

- ``ip("192.168.9.0/24", "/Common/10.10.0.5%5:443")`` returns
  ``"/Common/192.168.9.5%5:443"``.
- The host portion of ``10.10.0.5`` in ``/24`` is ``.5``; the
  result lands ``.5`` into the new network.

Address-family mismatch raises ``BuiltinError`` (an IPv4 host
cannot land in an IPv6 network).  An unparseable network or
source address likewise raises with the offending token in the
message.

Pair with ``|=`` to readdress every VS in one statement:
``.ltm.virtual[] | .destination |= ip("192.168.9.0/24", .)``.

Related: ``net``, ``host``, ``port``, ``route_domain``,
``with_route_domain``, ``in_cidr``.

**Examples**

```
ip("10.0.0.1")
ip("/Common/10.10.0.5%5:443")           # -> "10.10.0.5"
ip("192.168.9.0/24", .destination)      # rebase, keep host bits
ip("192.168.9.0/24", "10.10.0.5%5:443") # -> "192.168.9.5%5:443"
```

### `net`

Return the network portion of an IP/CIDR string as ``addr/prefix``.

**Signatures**

- `net(value: string) -> string`

**Details**

Parses *value* as a network (``addr/prefix``) and returns its
canonical form.  Host bits in the input are masked off, so
``net("192.168.9.42/24")`` returns ``"192.168.9.0/24"``.

Useful as a normaliser when you want every VS in the same /24 to
report the same network string: ``.ltm.virtual[] | .destination
| host(.) + "/24" | net(.)``.

Unparseable input raises ``BuiltinError``.  IPv4 and IPv6 networks
are both accepted; the prefix is required.

Related: ``ip``, ``in_cidr``, ``host``.

**Examples**

```
net("192.168.9.0/24")           # -> "192.168.9.0/24"
net("192.168.9.42/24")          # -> "192.168.9.0/24"
net("2001:db8::42/64")          # -> "2001:db8::/64"
```

### `port`

Return the port half of a BIG-IP destination as an integer, or ``null`` if no port is present.

**Signatures**

- `port(value: string) -> integer | null`

**Details**

Extracts the ``:port`` suffix from a destination string and
returns it as an integer.  Returns ``null`` (not ``0``) when no
port is present, so ``port(.destination) | defined(.)`` is the
natural way to filter VSes that explicitly target a port.

Partition prefix and route domain on the input are ignored.  A
malformed port (non-numeric) returns ``null`` rather than
raising — the destination simply doesn't have a recognisable
port.

Related: ``host``, ``ip``, ``route_domain``.

**Examples**

```
port(.destination)
port("192.168.1.1:80")                   # -> 80
port("/Common/10.0.0.1%5:443")           # -> 443
port("/Common/10.0.0.1")                 # -> null
```

### `route_domain`

Return the route-domain number of a destination / address string (``10.0.0.1%5:80`` -> ``5``), or null when none is present.

**Signatures**

- `route_domain(value: string) -> string | null`

**Details**

Extracts the ``%<rd>`` portion of a BIG-IP destination.  Returns
the route domain as a string (not an integer) because RDs may be
spelled with leading zeros or non-numeric tokens in some
configs; cast to int when you need to compare numerically.

Returns ``null`` when no route domain is present, so ``select(
route_domain(.destination) | defined(.))`` filters VSes that
explicitly bind to a non-default route domain.

Related: ``with_route_domain`` (set / replace / strip),
``host``, ``port``.

**Examples**

```
route_domain(.destination)
route_domain("10.0.0.1%5:80")            # -> "5"
route_domain("/Common/10.0.0.1:80")      # -> null
```

### `with_route_domain`

Set, replace, or strip the route-domain on a destination / address.  Pass an empty string (or null) as the second argument to strip the route-domain entirely.  Partition prefix and port are preserved.

**Signatures**

- `with_route_domain(value: string, rd: string | integer | null) -> string`

**Details**

Edits the route-domain portion of a destination string in place.
Accepts an integer (``with_route_domain(.dest, 7)``), a string
(``with_route_domain(.dest, "7")``), or null/empty-string to
strip the route domain entirely.

Partition prefix and port survive the edit unchanged — this
helper only touches the ``%<rd>`` segment.

Booleans are rejected (``with_route_domain(.dest, true)`` raises
``BuiltinError``) so accidental coercions don't produce
nonsense addresses.

Common pattern: ``.ltm.virtual[] | .destination |=
with_route_domain(., 7)`` rebinds every VS to RD 7 in one
statement.

Related: ``route_domain`` (read), ``ip`` (rebase, preserves RD),
``host``, ``port``.

**Examples**

```
with_route_domain(.destination, 5)
with_route_domain("/Common/10.0.0.1:80", 7)        # -> "/Common/10.0.0.1%7:80"
with_route_domain("/Common/10.0.0.1%5:80", "")     # -> "/Common/10.0.0.1:80"
.ltm.virtual[] | .destination |= with_route_domain(., 7)
```

## graph

Forward / reverse references across the same edge model `f5 grep` walks.  One hop deep; multi-hop walks belong in `f5 grep` for now.

### `referenced_by`

List the full-paths of every object that references the given object (reverse edges in the ``f5 grep`` graph).

**Signatures**

- `referenced_by(value: object) -> list[string]`

**Details**

The inverse of ``refs`` — walks one hop backwards in the
reference graph and lists the objects that depend on the seed.
Empty list means the object is an orphan (nothing in the config
references it).

Useful for orphan / cleanup queries:
``.ltm.pool[] | select(referenced_by(.) | count == 0) | .name``
lists every pool that no virtual / iRule / data-group attaches to.

Like ``refs``, the object must have been loaded from a real
config (has a ``config_uri``).

Related: ``refs`` (forward direction), ``count``, ``select``.

**Examples**

```
referenced_by(.ltm.pool.web_pool)
.ltm.pool[] | select(referenced_by(.) | count == 0) | .name  # orphan pools
```

### `refs`

List the full-paths of every object the given object references (forward edges in the same graph ``f5 grep`` walks).

**Signatures**

- `refs(value: object) -> list[string]`

**Details**

Walks the same reference graph the ``f5 grep`` verb uses, one
hop forward from the given object.  Returns a list of full-path
strings, deduplicated and excluding the seed itself.

Forward edges include every kind of reference ``grep`` knows
about: a VS's pool / iRules / profiles / persist / SNAT-pool,
a pool's monitor and member nodes, a rule's pool / persist /
data-group references extracted from its body, and so on.

Requires the object to have been loaded from a real config —
hand-built :class:`ObjectRef` values without a ``config_uri``
raise.

Currently always one hop deep; multi-hop walks belong in
``f5 grep`` (which produces a structured report) until the DSL
grows a ``depth`` argument.

Related: ``referenced_by`` (reverse direction), ``kind``,
``path``.

**Examples**

```
refs(.ltm.virtual.web_vs)
.ltm.virtual.web_vs | refs(.) | sort   # all dependencies, sorted
.ltm.virtual.web_vs | refs(.) | count  # dependency count
```

## value

Type / identity introspection: `kind` (TMSH kind), `path` (full-path), `length`, `defined`, `type`.

### `defined`

True when the argument is not null and not the empty string.

**Signatures**

- `defined(value: any) -> boolean`

**Details**

Returns ``true`` for values that are "set" — anything that is
not ``null``, not the empty string ``""``, and not an empty
:class:`PathRef`.

Distinct from a general truthiness check: ``defined`` returns
``true`` for ``false``, ``0``, and an empty list — those are
*defined* but falsy.  Pair with ``select`` to keep only objects
that have a particular field populated:
``.ltm.virtual[] | select(defined(.snatpool))``.

Related: ``not``, ``select``.

**Examples**

```
select(defined(.pool))
.ltm.virtual[] | select(defined(.snatpool)) | .name
```

### `kind`

Return the TMSH kind of an object (``ltm virtual``, ``ltm pool``, …).

**Signatures**

- `kind(value: object) -> string`

**Details**

Returns the TMSH module+type string an :class:`ObjectRef` belongs
to.  For a :class:`PathRef` returns the ``expected_kind`` (which
is the kind the surrounding field declared, e.g. ``"ltm pool"``
for ``.ltm.virtual[].pool``).

Useful for grouping or for filtering across kinds:
``[.ltm.pool[] | kind(.)] | unique`` returns the single-element
list ``["ltm pool"]``, and ``[.ltm.virtual[] | refs(.)[]]``
surfaces every dependency path; ``kind`` distinguishes them
downstream.

Related: ``path``, ``type``, ``refs``.

**Examples**

```
kind(.ltm.virtual.web_vs)                  # -> 'ltm virtual'
[.ltm.virtual[] | refs(.)[]] | unique | sort
```

### `length`

Length of a string, list, stream, or object's field map.

**Signatures**

- `length(value: any) -> integer`

**Details**

Returns the size of *value*:

- **string** / :class:`PathRef`: character count of the string.
- **list** / **stream**: number of items.
- **object**: number of TMSH fields (uncommonly used; mostly for
  introspection of unknown kinds).
- **null**: returns 0.

Raises ``BuiltinError`` for any other type (numbers, booleans).

Pairs naturally with comparisons for predicates: ``select(.rules
| length > 0)`` keeps every VS that has at least one attached
iRule.

Related: ``count`` (alias for list/stream only).

**Examples**

```
length(.rules)
.rules | length
.ltm.virtual[] | select(.rules | length > 0) | .name
```

### `path`

Return the BIG-IP full-path of an object or path-ref.

**Signatures**

- `path(value: object | path-ref) -> string`

**Details**

Returns the ``full_path`` of an :class:`ObjectRef` or
:class:`PathRef` as a plain string.  This is the same as reading
``."full-path"`` from an ObjectRef but reads more naturally in
pipelines.

Useful when you have a stream of mixed objects and want to print
a flat list of paths:
``.ltm.virtual.web_vs | refs(.) | map(path(.))`` (refs returns a
list, pipe passes it whole, ``map`` iterates it).

Raises ``BuiltinError`` for scalars (use the value directly when
it's already a string).

Related: ``kind``, ``partition``, ``basename``.

**Examples**

```
path(.ltm.virtual.web_vs)                # -> '/Common/web_vs'
[.ltm.virtual[] | path(.)]               # collect every VS full-path
```

### `str`

Convert any scalar to its string form.

**Signatures**

- `str(value: any) -> string`

**Details**

Coerces a scalar value to its string representation.  Useful for
building report-style output where a number or boolean needs to
appear next to text:
``.ltm.pool[] | .name + ": " + str(count(.members)) + " members"``.

The ``+`` operator also auto-coerces scalars when one side is
already a string, so ``str()`` is typically only needed when
both sides are non-strings (e.g. building a key out of two
numbers).

Rendering:

- **string** / :class:`PathRef`: returned as-is (PathRef → full-path).
- **integers** and **floats**: their decimal form.
- **booleans**: ``"true"`` / ``"false"``.
- **null**: ``"null"``.

Raises ``BuiltinError`` for objects, lists, and streams — those
have no single-line canonical form and the user should pick
explicit fields instead.

Related: ``+`` (string concat coerces scalars), ``length``,
``basename``.

**Examples**

```
.ltm.pool[] | .name + ": " + str(count(.members))
str(42)
```

### `type`

Name of the value's runtime type (``string``, ``object``, ``stream``, ...).

**Signatures**

- `type(value: any) -> string`

**Details**

Returns the DSL-level type name for *value*.  Possible values:

- ``"null"``, ``"bool"``, ``"int"``, ``"float"``, ``"string"``
- ``"path-ref"``, ``"object"``, ``"stream"``, ``"list"``

Useful for introspection and for writing queries that branch on
type (rare — most queries know the type from context).  Mainly
surfaces in debugging.

Related: ``kind`` (TMSH kind, more useful for BIG-IP objects),
``defined``.

**Examples**

```
type(.pool)                            # -> 'path-ref'
type(.destination)                     # -> 'string'
type(.rules)                           # -> 'list'
```

---

*Generated by `scripts/dev/gen_query_builtins_doc.py`.*
