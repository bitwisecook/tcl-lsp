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
  - [`contains`](#contains), [`csv`](#csv), [`downcase`](#downcase), [`endswith`](#endswith), [`gsub`](#gsub), [`index`](#index), [`join`](#join), [`match`](#match), [`split`](#split), [`startswith`](#startswith), [`sub`](#sub), [`tsv`](#tsv), [`upcase`](#upcase)
- **[path](#path)** — BIG-IP full-path string helpers — extract the partition or basename, swap a partition prefix.  These are *string* transforms; they don't move objects.  For object renames, reach for the **rename** category.
  - [`basename`](#basename), [`partition`](#partition), [`with_partition`](#with_partition)
- **[rename](#rename)** — Cascading rename operations — `rename` for one object, `rename_partition` for every object in a partition.  Both route through the same token-bounded engine `f5 rename` uses, so references inside iRule bodies and compound values (destination addresses, pool-member identifiers) are rewritten consistently.
  - [`rename`](#rename), [`rename_folder`](#rename_folder), [`rename_partition`](#rename_partition), [`rename_prefix`](#rename_prefix)
- **[net](#net)** — IP-address arithmetic and route-domain helpers.  The `ip(net, src)` rebase is the workhorse of bulk readdressing; `with_route_domain` sets / replaces / strips the `%rd` suffix.
  - [`broadcast_address`](#broadcast_address), [`can_see`](#can_see), [`collapse_cidrs`](#collapse_cidrs), [`dns`](#dns), [`first_host`](#first_host), [`folder`](#folder), [`host`](#host), [`host_count`](#host_count), [`http_body`](#http_body), [`http_body_json`](#http_body_json), [`http_client_error`](#http_client_error), [`http_header`](#http_header), [`http_headers`](#http_headers), [`http_ok`](#http_ok), [`http_redirect`](#http_redirect), [`http_server_error`](#http_server_error), [`http_status`](#http_status), [`in_cidr`](#in_cidr), [`in_folder`](#in_folder), [`in_partition`](#in_partition), [`ip`](#ip), [`ip_range_contains`](#ip_range_contains), [`ip_range_count`](#ip_range_count), [`ip_range_supernet`](#ip_range_supernet), [`ip_range_to_cidrs`](#ip_range_to_cidrs), [`is_documentation`](#is_documentation), [`is_fqdn`](#is_fqdn), [`is_ipv4`](#is_ipv4), [`is_ipv6`](#is_ipv6), [`is_link_local`](#is_link_local), [`is_loopback`](#is_loopback), [`is_multicast`](#is_multicast), [`is_private`](#is_private), [`is_public`](#is_public), [`is_reserved`](#is_reserved), [`is_unspecified`](#is_unspecified), [`is_wildcard_port`](#is_wildcard_port), [`last_host`](#last_host), [`net`](#net), [`network_address`](#network_address), [`overlaps`](#overlaps), [`ping`](#ping), [`port`](#port), [`port_set_contains`](#port_set_contains), [`port_set_count`](#port_set_count), [`port_set_overlaps`](#port_set_overlaps), [`portping`](#portping), [`prefix_length`](#prefix_length), [`rev_dns`](#rev_dns), [`route_domain`](#route_domain), [`socket_get`](#socket_get), [`subnet_of`](#subnet_of), [`supernet_of`](#supernet_of), [`tls_handshake`](#tls_handshake), [`traceroute`](#traceroute), [`url_get`](#url_get), [`url_head`](#url_head), [`url_options`](#url_options), [`url_post`](#url_post), [`with_folder`](#with_folder), [`with_host`](#with_host), [`with_name`](#with_name), [`with_port`](#with_port), [`with_route_domain`](#with_route_domain), [`x509_parse`](#x509_parse)
- **[graph](#graph)** — Forward / reverse references across the same edge model `f5 grep` walks.  One hop deep; multi-hop walks belong in `f5 grep` for now.
  - [`check_partition_visibility`](#check_partition_visibility), [`referenced_by`](#referenced_by), [`references_to`](#references_to), [`refs`](#refs)
- **[value](#value)** — Type / identity introspection: `kind` (TMSH kind), `path` (full-path), `length`, `defined`, `type`.
  - [`defined`](#defined), [`json_load`](#json_load), [`json_parse`](#json_parse), [`kind`](#kind), [`length`](#length), [`path`](#path), [`source_file`](#source_file), [`str`](#str), [`type`](#type)

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

### `csv`

Join arguments with commas, quoting cells when necessary.

**Signatures**

- `csv(*cells: any) -> string`
- `csv(a, b, c, ...) -> string`

**Details**

RFC 4180-style CSV row builder.  Each argument is coerced to its
scalar string form and emitted as a CSV field; cells containing
``,``, ``"``, ``\n``, or ``\r`` are wrapped in double quotes
with embedded quotes doubled (``"`` → ``""``).  Empty cells emit
as an empty field (``,``), not as ``""``.

Broadcasts the same way as ``tsv``: when any argument is a
:class:`Stream`, ``csv`` produces one row per element with scalar
arguments replicated.

Pair with ``--raw`` for clean piping into CSV consumers:
``f5 query --raw 'csv(.name, .destination)' bigip.conf | head``.

Related: ``tsv``, ``join``.

**Examples**

```
.ltm.virtual[] | csv(.name, .destination, .pool)
.ltm.pool[].members[] | csv(.name, .address)
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

### `index`

Position of a needle inside a string or list (jq-compatible).

**Signatures**

- `index(value: string, needle: string) -> integer | null`
- `index(value: list, needle: any) -> integer | null`

**Details**

Mirrors jq's ``index`` builtin.  Returns the zero-based offset of
the first occurrence of *needle* inside *value*, or ``null`` when
*needle* is not present.

For strings, ``index`` does substring search.  For lists / streams
/ :class:`BigipList` values, ``index`` matches element-wise on
``full_path`` when items are :class:`PathRef`, otherwise on
equality.

Common predicate idiom (paralleling jq):
``.ltm.virtual[] | select(.name | index(":443"))`` — keeps every
virtual whose name contains the substring.

Related: ``contains`` (boolean variant), ``startswith`` /
``endswith``.

**Examples**

```
.ltm.virtual[] | select(.name | index(":443"))
[.profiles[].name] | index("http")     # 0..n-1 or null
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

### `tsv`

Join arguments with tabs for tab-separated row output.

**Signatures**

- `tsv(*cells: any) -> string`
- `tsv(a, b, c, ...) -> string`

**Details**

Each argument is coerced to its scalar string form (``PathRef`` →
full-path, ``null`` → empty, bool → ``true`` / ``false``,
numbers → their decimal form) and joined with ``\t``.  Embedded
tabs, newlines, and carriage returns inside cell values are
replaced with spaces so the resulting line stays one TSV row;
pre-quote cells explicitly if you need to retain whitespace.

Designed to compose with stream broadcast: when any argument is a
:class:`Stream`, ``tsv`` broadcasts element-wise so
``tsv(.name, .destination, .pool)`` produces one row per virtual
server, and ``tsv(.name, .pool.members[].address)`` produces one
row per pool member with the VS name replicated across each row
(same semantics every other scalar builtin uses).

Pair with ``--raw`` to print without surrounding quoting:
``f5 query --raw 'tsv(.name, .destination)' bigip.conf``.

Related: ``csv`` (comma-separated, quote-aware), ``join`` (join
one list with a separator), string concat with ``+``.

**Examples**

```
.ltm.virtual[] | tsv(.name, .destination, .pool)
.ltm.pool[].members[] | tsv(.name, .address, port(.name))
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

### `rename_folder`

Move every object from one folder path to another.

**Signatures**

- `rename_folder(old: string, new: string) -> integer`

**Details**

The folder-level sibling of :func:`rename_partition`.  ``old``
and ``new`` are folder paths (``/Common/iApps/Tenant.app`` /
``/Tenant_A/iApps/Tenant.app``) — every reference whose path
starts with ``<old>/`` is rewritten to start with ``<new>/``.

Cascades into every place a TMSH path appears in the source:

- object stanza headers (``ltm pool /Common/iApps/old.app/p1``);
- reference properties (``pool /Common/iApps/old.app/p1``);
- destinations that embed the folder
  (``destination /Common/iApps/old.app/10.0.0.1:80``);
- iRule body literals.

Uses the same token-bounded prefix-cascade machinery
``rename_partition`` uses — so an unrelated path
``/Common/iApps/old.app.bak/p1`` doesn't accidentally match.

Pre-flight checks: both arguments must be parseable folder
paths (``/<partition>[/<segment>...]``); empty names raise
``BuiltinError``.  ``old == new`` is a no-op.

Returns the count of textual matches the cascade landed on.

Related: ``rename_partition`` (partition-level),
``rename`` (single-object), ``with_folder`` (string transform,
doesn't migrate references), ``folder`` (extract folder).

**Examples**

```
rename_folder("/Common/iApps/old.app", "/Common/iApps/new.app")
rename_folder("/Common/iApps/Tenant.app", "/Tenant_A/iApps/Tenant.app")
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
rename_partition("Tenant_A", "Tenant_B")
rename_partition("staging", "prod")
```

### `rename_prefix`

Rewrite every object whose full-path starts with *old* to start with *new*.

**Signatures**

- `rename_prefix(old: string, new: string) -> integer`

**Details**

A general-purpose sibling of :func:`rename_partition` and
:func:`rename_folder`: where those are scoped to partition or
folder boundaries, ``rename_prefix`` operates on arbitrary
full-path prefixes.  Useful for moving a *family* of related
objects together when their identifying convention is a leaf-
name prefix that doesn't align with a partition or folder
boundary, e.g. moving every ``/Common/app3_*`` object to
``/Tenant_A/app3_*``:

::

    rename_prefix("/Common/app3_", "/Tenant_A/app3_")

Every full-path occurrence beginning with ``<old>`` is rewritten
to begin with ``<new>``, cascading through:

- object stanza headers (``ltm pool /Common/app3_p1``);
- reference properties (``pool /Common/app3_p1``);
- destinations that embed the prefix
  (``destination /Common/app3_vip:443``);
- iRule body literals.

Token-bounded so an unrelated path that *contains* the prefix
later in the string (``/Common/old/app3_x``) doesn't accidentally
match — the rewrite only fires when the prefix starts on a
path-segment boundary.

Pre-flight checks: both arguments must be non-empty.  ``old ==
new`` is a no-op.  Mixing with field edits inside the same
statement is rejected (byte offsets shift); split with ``;``.

Returns the count of textual matches the cascade landed on.

Related: ``rename_partition`` (partition-level cascade),
``rename_folder`` (folder-level cascade), ``rename`` (single
object + every reference).

**Examples**

```
rename_prefix("/Common/app3_", "/Tenant_A/app3_")
rename_prefix("/Common/legacy-", "/Tenant_B/legacy-")
```

## net

IP-address arithmetic and route-domain helpers.  The `ip(net, src)` rebase is the workhorse of bulk readdressing; `with_route_domain` sets / replaces / strips the `%rd` suffix.

### `broadcast_address`

Return the broadcast address of a CIDR (last address in range).

**Signatures**

- `broadcast_address(value: string) -> string | null`

**Details**

For IPv4 the broadcast is the ``.255`` (or whatever the
prefix gives); for IPv6 there is no true broadcast, but
``ipaddress`` exposes the last address in the range and we
surface it here for symmetry.  Returns ``null`` for
unparseable input.

Related: ``network_address``, ``last_host``, ``host_count``.

**Examples**

```
broadcast_address("10.0.0.0/24")             # -> "10.0.0.255"
.net.route[] | {net: network_address(.network), bcast: broadcast_address(.network)}
```

### `can_see`

True when *referrer_path*'s partition may reference *target_path*'s partition.

**Signatures**

- `can_see(referrer_path: string, target_path: string) -> boolean`

**Details**

F5 partition visibility is **directional**:

- Objects in any partition may reference objects in ``/Common``
  (one-way visibility).
- Objects in ``/Common`` may **not** reference objects in any
  tenant partition.
- Cross-tenant references (``/Tenant_A/...`` ↔ ``/Tenant_B/...``)
  are **not** allowed.
- Same partition is always visible to itself.

Use this predicate to validate that a proposed rename or
cross-config reference is legal *before* applying it.  Example:
"find every iRule that references a pool whose partition the
rule itself can't see" (uses a let-binding to carry the rule's
full path into the per-reference stream — the DSL has no jq
``..`` parent operator):

``.ltm.rule[] as $r | $r.refs.pools[] | select(not can_see($r."full-path", .))``

Related: ``partition``, ``in_partition``,
``check_partition_visibility``.

**Examples**

```
can_see("/Tenant_A/vs1", "/Common/web_pool")  # true — Tenant_A can see /Common
can_see("/Common/vs1", "/Tenant_A/web_pool")  # false — /Common cannot see /Tenant_A
can_see("/Tenant_A/vs1", "/Tenant_B/web_pool")  # false — cross-tenant
```

### `collapse_cidrs`

Merge a list of CIDRs into the minimal set of ranges.

**Signatures**

- `collapse_cidrs(values: list[string]) -> list[string]`

**Details**

Wraps :func:`ipaddress.collapse_addresses`.  Adjacent or
subsumed CIDRs in *values* are merged so the result is the
smallest set of non-overlapping ranges that covers the same
address space.  Mixed IPv4 / IPv6 lists are split and each
family collapsed independently.

Useful for normalising address-list and firewall-rule
address-list payloads before diffing:
``collapse_cidrs([.security.firewall."address-list"[].addresses[]])``.

Related: ``supernet_of`` (one CIDR covering everything),
``subnet_of``.

**Examples**

```
collapse_cidrs(["10.0.0.0/24", "10.0.1.0/24"])    # -> ["10.0.0.0/23"]
collapse_cidrs(["10.0.0.0/8", "10.1.0.0/16"])     # -> ["10.0.0.0/8"]
```

### `dns`

Resolve a hostname to its IP addresses (A + AAAA records).

**Signatures**

- `dns(name: string) -> list[string]`

**Details**

Performs a forward DNS lookup of *name* via the system
resolver (``socket.getaddrinfo``).  Returns the sorted list
of unique IP addresses or an empty list when resolution
fails.

Results are memoised for the lifetime of the Python process
so repeated lookups inside one query don't hammer DNS.
Lookups are time-bounded by the resolver's default timeout
(typically 5s).

Pair with ``rev_dns`` for round-trip checks
(``dns("host.example.com") | map(rev_dns(.))``).

**Examples**

```
dns("one.one.one.one")                          # -> ["1.1.1.1", "1.0.0.1"]
.ltm.node[].address | {addr: ., rev: rev_dns(.)}
```

### `first_host`

Return the lowest usable host address inside a CIDR.

**Signatures**

- `first_host(value: string) -> string | null`

**Details**

For prefix lengths that yield a network and broadcast
address (IPv4 ``/30`` or shorter, IPv6 anything), this is
``network + 1`` — the first address assignable to a host.
For point-to-point ``/31`` and host ``/32`` IPv4 networks
where ``ipaddress.hosts()`` is empty, falls back to the
network address itself (the only / lowest address in the
range).  Returns ``null`` for unparseable input.

Related: ``last_host``, ``host_count``, ``network_address``.

**Examples**

```
first_host("10.0.0.0/24")                    # -> "10.0.0.1"
.ltm.pool[].members[].address | first_host(. + "/24")
```

### `folder`

Return the folder portion of a TMSH path (``/Common/Application_X``).

**Signatures**

- `folder(value: string) -> string`

**Details**

Extracts the folder path from a full BIG-IP object path.  Bare
partition (``/Common/pool``) → ``"/Common"`` (just the
partition root); nested-folder (``/Common/iApps/Tenant.app/p``)
→ ``"/Common/iApps/Tenant.app"``.  Returns ``""`` for non-path
input.

Sibling to :func:`partition` (which returns just the partition
name without the slash).

Related: ``partition``, ``basename``, ``with_partition``,
``with_folder``.

**Examples**

```
folder(."full-path")
.ltm.virtual[] | select(folder(."full-path") == "/Common/iApps/Tenant.app") | .name
```

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

### `host_count`

Count of host addresses inside a CIDR.

**Signatures**

- `host_count(value: string) -> integer | null`

**Details**

Returns the number of host-assignable addresses in *value*.
``host_count("10.0.0.0/24")`` → ``254`` (256 − network −
broadcast).  ``/31`` returns 2 and ``/32`` returns 1, matching
operational reality on point-to-point and host networks.
Returns ``null`` for unparseable input.

Related: ``first_host``, ``last_host``, ``prefix_length``.

**Examples**

```
host_count("10.0.0.0/24")                    # -> 254
host_count("10.0.0.0/31")                    # -> 2
```

### `http_body`

Response body as a string.

**Signatures**

- `http_body(response: object) -> string`

**Details**

Accessor for the response's ``body`` field.  Always a string;
binary payloads round-trip with U+FFFD replacement.

**Examples**

```
url_get("https://example.com/") | http_body(.)
```

### `http_body_json`

Parse the response body as JSON.

**Signatures**

- `http_body_json(response: object) -> any`

**Details**

Convenience wrapper around ``json_parse(.body)`` that adds a
light content-type sanity check: if the response declares a
``content-type`` and it doesn't include ``json``, the
builtin still parses but raises ``BuiltinError`` if the body
isn't valid JSON.  When ``content-type`` is missing it
silently attempts the parse.

Use this when an API returns JSON and you want to traverse
the parsed value without spelling out a ``json_parse(.body)``
chain every time.

**Examples**

```
url_get("https://api/v1") | http_body_json(.) | .items
.urls[] | url_get(.) | http_body_json(.).version
```

### `http_client_error`

True when the response status is 4xx.

**Signatures**

- `http_client_error(response: object) -> boolean`

**Details**

Range predicate for the 400-499 client-error class.

**Examples**

```
.urls[] | url_get(.) | select(http_client_error(.))
```

### `http_header`

Return one header value by name (case-insensitive).

**Signatures**

- `http_header(response: object, name: string) -> string | null`

**Details**

Looks *name* up in the response's headers; the match is
case-insensitive (``Content-Type`` finds ``content-type``).
Returns ``null`` when the header isn't present.

Note: HTTP allows multiple headers with the same name to
repeat (e.g. ``Set-Cookie``).  The underlying urllib path
collapses repeats into a single comma-separated string,
matching the wire-format convention.

**Examples**

```
url_get("https://example.com/") | http_header(., "content-type")
.urls[] | url_head(.) | http_header(., "server")
```

### `http_headers`

Return the response's headers as a dict (keys lowercased).

**Signatures**

- `http_headers(response: object) -> object`

**Details**

The underlying ``url_*`` builtins already store headers
with lowercase keys so a query can do case-insensitive
lookups directly.  This helper is the typed accessor: use
``http_header(resp, "name")`` for one value, or
``http_headers(resp)`` when you want the whole map.

**Examples**

```
url_get("https://example.com/") | http_headers(.) | keys
```

### `http_ok`

True when the response status is 2xx.

**Signatures**

- `http_ok(response: object) -> boolean`

**Details**

Range predicate for the 200-299 success class.  Useful as
the head of audit pipelines:
``.urls[] | url_get(.) | select(http_ok(.))``.

Related: ``http_redirect``, ``http_client_error``,
``http_server_error``.

**Examples**

```
.urls[] | url_get(.) | select(http_ok(.))
```

### `http_redirect`

True when the response status is 3xx.

**Signatures**

- `http_redirect(response: object) -> boolean`

**Details**

Range predicate for the 300-399 redirect class.
Pair with ``http_header(., "location")`` to extract the
Location target.

**Examples**

```
.urls[] | url_head(.) | select(http_redirect(.))
```

### `http_server_error`

True when the response status is 5xx.

**Signatures**

- `http_server_error(response: object) -> boolean`

**Details**

Range predicate for the 500-599 server-error class.  When
diffing an audit run, surfacing these reliably gives an
operator the right signal — server errors typically need
a different escalation path from 4xx client misuse.

**Examples**

```
.urls[] | url_get(.) | select(http_server_error(.))
```

### `http_status`

Status code from an HTTP response dict.

**Signatures**

- `http_status(response: object) -> integer | null`

**Details**

Accessor for the ``status`` field of an ``url_get``-style
response.  Returns ``null`` when the request failed before
the server responded (DNS error, connect timeout, etc.).

Equivalent to ``response.status`` — provided for parity with
the other ``http_*`` helpers and so audits can spell their
intent symmetrically.

**Examples**

```
url_get("https://example.com/") | http_status(.)
.urls[] | url_head(.) | {url: ., status: http_status(.)}
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

### `in_folder`

True when *path* lives at-or-below *folder*.

**Signatures**

- `in_folder(path: string, folder: string) -> boolean`

**Details**

Matches paths whose folder prefix equals *folder* OR has
*folder* as an ancestor.  ``in_folder(
"/Common/iApps/Tenant.app/pool_1", "/Common/iApps")`` →
``true``; ``in_folder("/Common/web_pool",
"/Common/iApps")`` → ``false``.

Symbolic alternative to ``startswith(folder(.), "/Common/iApps")``
— does the right thing on folder boundaries (won't match
``/Common/iApps_bak/...``).

Related: ``folder``, ``in_partition``, ``startswith``.

**Examples**

```
.ltm.pool[] | select(in_folder(."full-path", "/Common/iApps")) | .name
```

### `in_partition`

True when *path* belongs to *partition*.

**Signatures**

- `in_partition(path: string, partition: string) -> boolean`

**Details**

Accepts both spellings of the partition argument: bare
(``"Common"``) and slash-prefixed (``"/Common"``).  Returns
``false`` for inputs that aren't TMSH paths.

Symbolic alternative to ``partition(.) == "Common"`` — reads
better in filters and avoids the bare-name vs path-shape
pitfall.

Related: ``partition``, ``in_folder``.

**Examples**

```
.ltm.pool[] | select(in_partition(."full-path", "Common")) | .name
in_partition("/Common/web_pool", "Common")
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

### `ip_range_contains`

True when *addr* lies inside the ``first-last`` range.

**Signatures**

- `ip_range_contains(range: string, addr: string) -> boolean`

**Details**

Inclusive membership check.  Mixed-family inputs (v4 range,
v6 address or vice versa) always return ``false`` rather
than raising — different families never overlap.

Related: ``in_cidr`` (CIDR equivalent), ``ip_range_to_cidrs``.

**Examples**

```
ip_range_contains("192.168.9.77-192.168.9.83", "192.168.9.80")  # -> true
ip_range_contains("10.0.0.0-10.0.0.255", "10.0.1.1")            # -> false
```

### `ip_range_count`

Count of addresses in an IP range (inclusive).

**Signatures**

- `ip_range_count(range: string) -> integer | null`

**Details**

``"10.0.0.5-10.0.0.9"`` → 5 (five addresses inclusive).
Returns ``null`` for unparseable input.

Related: ``ip_range_to_cidrs``, ``ip_range_contains``.

**Examples**

```
ip_range_count("192.168.9.77-192.168.9.83")    # -> 7
ip_range_count("10.0.0.1")                     # -> 1
```

### `ip_range_supernet`

Smallest single CIDR that covers an IP range.

**Signatures**

- `ip_range_supernet(range: string) -> string | null`

**Details**

The minimum-prefix CIDR containing both endpoints of *range*.
May include addresses outside the original ``[first, last]``
span — that's the inherent cost of summarising a free-form
range as a single CIDR.

Pair with :func:`ip_range_to_cidrs` (exact decomposition)
when you need precision instead of a single bounding network.

**Examples**

```
ip_range_supernet("192.168.9.77-192.168.9.83")  # -> "192.168.9.64/27"
```

### `ip_range_to_cidrs`

Decompose ``first-last`` IP range into the minimum CIDR set.

**Signatures**

- `ip_range_to_cidrs(range: string) -> list[string]`

**Details**

Parses *range* (``"192.168.9.77-192.168.9.83"``) and returns
the smallest list of CIDRs that exactly covers the range.
Useful for converting free-form ranges into firewall
``address-list`` entries.

Returns ``null`` for unparseable input.  Single-address
inputs return a one-element list of the ``/32`` (or
``/128``).

Related: ``ip_range_supernet``, ``ip_range_count``,
``ip_range_contains``.

**Examples**

```
ip_range_to_cidrs("192.168.9.77-192.168.9.83")  # -> 4 /29.. /30 etc.
ip_range_to_cidrs("10.0.0.1-10.0.0.255")
```

### `is_documentation`

True when *value* is in a documentation-example range (RFC 5737 / RFC 3849).

**Signatures**

- `is_documentation(value: string) -> boolean`

**Details**

IPv4 ``192.0.2.0/24``, ``198.51.100.0/24``, ``203.0.113.0/24``,
and IPv6 ``2001:db8::/32``.  Catching these in production
configs is almost always a lab-template leak.

Related: ``is_public``, ``is_private``, ``is_reserved``.

**Examples**

```
.ltm.virtual[] | select(is_documentation(.destination)) | .name
```

### `is_fqdn`

True when *value*'s host portion is an FQDN (not an IP).

**Signatures**

- `is_fqdn(value: string) -> boolean`

**Details**

Distinguishes FQDN pool members (``/Common/host.example.com:443``)
from IP-based ones.  Returns ``false`` for IPv4 / IPv6 / empty
/ unparseable input.

Useful for branching when a pool has a mix of IP and FQDN
members — typically the FQDN form needs DNS-resolution checks
while IP-form members get straight reachability checks.

Related: ``is_ipv4``, ``is_ipv6``.

**Examples**

```
is_fqdn(.address)
.ltm.pool[] | .members[] | select(is_fqdn(.address)) | .name
```

### `is_ipv4`

True when *value* parses as an IPv4 address.

**Signatures**

- `is_ipv4(value: string) -> boolean`

**Details**

Accepts a bare IPv4 (``10.0.0.1``) or a destination string
(``/Common/10.0.0.1:80`` — the host portion is extracted).
Returns ``false`` for IPv6, FQDN, or unparseable input.

Pairs with :func:`is_ipv6` to branch on address family without
pattern-matching the string.

Related: ``is_ipv6``, ``is_fqdn``, ``is_private``,
``is_loopback``, ``is_unspecified``.

**Examples**

```
is_ipv4(.destination)
.ltm.virtual[] | select(is_ipv4(.destination)) | .name
```

### `is_ipv6`

True when *value* parses as an IPv6 address.

**Signatures**

- `is_ipv6(value: string) -> boolean`

**Details**

Accepts every documented F5 spelling — bare (``2001:db8::1``),
bracketed (``[2001:db8::1]``), with ``.``-port
(``[2001:db8::1].80`` / ``2001:db8::1.80``), with ``:``-port
(``[2001:db8::1]:80``), partition-prefixed, folder-nested.
Returns ``false`` for IPv4 / FQDN / unparseable input.

Related: ``is_ipv4``, ``is_fqdn``.

**Examples**

```
is_ipv6(.destination)
.ltm.virtual[] | select(is_ipv6(.destination)) | .name
```

### `is_link_local`

True when *value* is link-local (``169.254.0.0/16`` IPv4 / ``fe80::/10`` IPv6).

**Signatures**

- `is_link_local(value: string) -> boolean`

**Details**

RFC 3927 (IPv4) / RFC 4291 (IPv6) link-local — addresses that
are only valid on the directly attached segment.  Useful when
auditing for accidentally-leaked auto-configured addresses.

Related: ``is_multicast``, ``is_loopback``, ``is_private``.

**Examples**

```
.ltm.node[] | select(is_link_local(.address)) | .name
```

### `is_loopback`

True when *value* is a loopback address (``127.0.0.0/8`` / ``::1``).

**Signatures**

- `is_loopback(value: string) -> boolean`

**Details**

Returns ``false`` for non-loopback IPs, FQDNs, and unparseable
input.

Related: ``is_private``, ``is_unspecified``.

**Examples**

```
.ltm.virtual[] | select(is_loopback(.destination)) | .name
```

### `is_multicast`

True when *value* is a multicast IP (``224.0.0.0/4`` / ``ff00::/8``).

**Signatures**

- `is_multicast(value: string) -> boolean`

**Details**

Classifies through Python's ``ipaddress``: IPv4 ``224.0.0.0/4``
and IPv6 ``ff00::/8``.  Returns ``false`` for FQDNs, unicast
IPs, and unparseable input.

Related: ``is_link_local``, ``is_reserved``, ``is_public``.

**Examples**

```
.ltm.virtual[] | select(is_multicast(.destination)) | .name
```

### `is_private`

True when *value* is an RFC-1918 / RFC-4193 private IP.

**Signatures**

- `is_private(value: string) -> boolean`

**Details**

Classifies through Python's ``ipaddress`` stdlib —
``10.0.0.0/8``, ``172.16.0.0/12``, ``192.168.0.0/16`` for IPv4;
``fc00::/7`` for IPv6 ULAs; plus a handful of other "non-global"
ranges per the IANA registries.

Returns ``false`` for FQDN, public IPs, and unparseable input.

Related: ``is_loopback``, ``is_unspecified``, ``in_cidr``.

**Examples**

```
is_private(.destination)
.ltm.virtual[] | select(is_private(.destination)) | .name
```

### `is_public`

True when *value* is globally routable on the public internet.

**Signatures**

- `is_public(value: string) -> boolean`

**Details**

Returns ``true`` only when *value* is **not** in any of the
reserved / private / loopback / link-local / multicast /
unspecified ranges — i.e. an address you might legitimately
see on the public internet.  Backed by
:pyattr:`ipaddress.IPv4Address.is_global`.

Use to audit "what's actually exposed?" without spelling out
every negation:
``.ltm.virtual[] | select(is_public(.destination)) | .name``.

Related: ``is_private`` (the complement), ``is_reserved``,
``is_documentation``.

**Examples**

```
.ltm.virtual[] | select(is_public(.destination)) | .name
```

### `is_reserved`

True when *value* is in an IANA-reserved range (no current use).

**Signatures**

- `is_reserved(value: string) -> boolean`

**Details**

Reserved means "IANA has set aside the range, no current
allocation" — distinct from ``is_private`` (carved out for
intra-network use).  IPv4 ``240.0.0.0/4`` and various IPv6
blocks fall here.

Related: ``is_public``, ``is_private``.

**Examples**

```
.ltm.virtual[] | select(is_reserved(.destination)) | .name
```

### `is_unspecified`

True when *value* is the unspecified-host wildcard (``0.0.0.0`` / ``::``).

**Signatures**

- `is_unspecified(value: string) -> boolean`

**Details**

F5 uses ``0.0.0.0`` / ``::`` as the listen-on-any host wildcard
on virtual servers.  This predicate filters those out cleanly:

``.ltm.virtual[] | select(is_unspecified(.destination)) | .name``

Returns ``false`` for any non-wildcard IP, FQDN, or
unparseable input.

Related: ``is_wildcard_port`` (the partner for the port half),
``is_loopback``, ``is_private``.

**Examples**

```
.ltm.virtual[] | select(is_unspecified(.destination)) | .name
```

### `is_wildcard_port`

True when *value*'s port portion is the wildcard (``any`` / ``*`` / ``0``).

**Signatures**

- `is_wildcard_port(value: string) -> boolean`

**Details**

F5 virtual-server destinations carrying port wildcards
(``/Common/0.0.0.0:any`` / ``/Common/10.0.0.1:0``) match every
incoming port; surface them with this predicate rather than
matching a string suffix.

Related: ``port``, ``is_unspecified`` (the host half).

**Examples**

```
.ltm.virtual[] | select(is_wildcard_port(.destination)) | .name
```

### `last_host`

Return the highest usable host address inside a CIDR.

**Signatures**

- `last_host(value: string) -> string | null`

**Details**

The mirror of :func:`first_host`.  For ``/30`` and shorter
IPv4 networks this is one below the broadcast; for ``/31``
and ``/32`` it is the network address itself.  Returns
``null`` for unparseable input.

Related: ``first_host``, ``broadcast_address``, ``host_count``.

**Examples**

```
last_host("10.0.0.0/24")                     # -> "10.0.0.254"
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

### `network_address`

Return the network (``.0``) address of a CIDR.

**Signatures**

- `network_address(value: string) -> string | null`

**Details**

Strips the host bits off *value* and returns the canonical
network address.  ``network_address("10.0.0.5/24")`` →
``"10.0.0.0"``.  Returns ``null`` for unparseable input.

Related: ``broadcast_address``, ``first_host``, ``last_host``,
``prefix_length``.

**Examples**

```
network_address("10.0.0.5/24")               # -> "10.0.0.0"
.net.self[] | network_address(.address)
```

### `overlaps`

True when two networks overlap (share at least one address).

**Signatures**

- `overlaps(net1: string, net2: string) -> boolean`

**Details**

Useful for finding self-IP / route-domain conflicts.  The DSL
doesn't ship a pairwise-combinations primitive yet, so the
natural pattern uses a let-binding to cross the stream against
itself: ``[.net.self[]] as $all | .net.self[] as $a | $all[]
| select(. != $a) | select(overlaps($a.address, .address))
| $a.name + " ↔ " + .name``.

IPv4 ↔ IPv6 comparison returns ``false``.

Related: ``subnet_of``, ``in_cidr``.

**Examples**

```
overlaps("10.0.0.0/24", "10.0.0.0/16")              # -> true
overlaps("10.0.0.0/24", "10.1.0.0/24")              # -> false
```

### `ping`

ICMP echo to *ip*.  Requires --enable-probes.

**Signatures**

- `ping(ip: string) -> object`

**Details**

Subprocess invocation of the system ``ping`` command.
Returns ``{ok: bool, rtt_ms: float | null, error: string | null}``.
Gated by ``--enable-probes`` — without the flag, raises
``BuiltinError`` so an offline query never hits the network
by accident.

Related: ``portping`` (TCP/UDP), ``traceroute``, ``dns``.

**Examples**

```
ping("10.0.0.1")
.ltm.node[] | {addr: .address, reachable: (ping(.address).ok)}
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

### `port_set_contains`

True when *port* lies inside the comma-separated *spec*.

**Signatures**

- `port_set_contains(spec: string, port: integer) -> boolean`

**Details**

*spec* is a F5 firewall-rule port spec like ``"80-82,8081"``.
Returns ``true`` when *port* falls inside any segment.  Use
this to audit rules:
``.security.firewall.rule-list[].rules[] | select(port_set_contains(.port, 443))``.

Related: ``port_set_count``, ``port_set_overlaps``,
``in_cidr`` (the address-side counterpart).

**Examples**

```
port_set_contains("80-82,8081", 81)            # -> true
port_set_contains("80,443", 8080)              # -> false
```

### `port_set_count`

Total number of ports across every segment of *spec*.

**Signatures**

- `port_set_count(spec: string) -> integer | null`

**Details**

Counts how many distinct ports a comma-separated port spec
covers.  ``"80-82,8081"`` → 4.  ``"any"`` → 65536.  Returns
``null`` for unparseable input.

Related: ``port_set_contains``, ``port_set_overlaps``.

**Examples**

```
port_set_count("80-82,8081")                   # -> 4
port_set_count("any")                          # -> 65536
```

### `port_set_overlaps`

True when two port specs share at least one port.

**Signatures**

- `port_set_overlaps(a: string, b: string) -> boolean`

**Details**

Pair-wise overlap check between two comma-separated port
specs.  Useful when comparing firewall-rule port windows to
spot accidental coverage gaps or duplications.

Related: ``port_set_contains``, ``port_set_count``.

**Examples**

```
port_set_overlaps("80-82,443", "82-100")       # -> true
port_set_overlaps("80-82,443", "100-200")      # -> false
```

### `portping`

TCP/UDP probe to *(ip, port)*.  Requires --enable-probes.

**Signatures**

- `portping(ip: string, port: integer[, protocol: string]) -> object`

**Details**

TCP-connect (default) or UDP send-receive timing.  Returns
``{ok, rtt_ms, error}``.  UDP is best-effort — no reply does
not imply unreachable.  Pass ``protocol="udp"`` to switch.

**Examples**

```
portping("10.0.0.1", 443)
.ltm.virtual[] | {name: .name, vip_up: portping(host(.destination), port(.destination)).ok}
```

### `prefix_length`

Return the CIDR prefix length of a network string.

**Signatures**

- `prefix_length(value: string) -> integer | null`

**Details**

Accepts both integer CIDR (``10.0.0.0/24``) and dotted-quad
netmask (``10.0.0.0/255.255.255.0``) — both render the same
prefix length.  Returns ``null`` for inputs that aren't
networks.

Pairs with :func:`subnet_of` for CIDR algebra.

Related: ``in_cidr``, ``subnet_of``.

**Examples**

```
prefix_length(.net.self[].address)
.net.self[] | select(prefix_length(.address) >= 24) | .name
```

### `rev_dns`

Reverse-resolve an IP address to its PTR hostname.

**Signatures**

- `rev_dns(ip: string) -> list[string]`

**Details**

Performs a reverse DNS lookup (``socket.gethostbyaddr``).
Returns the canonical hostname plus any aliases, or an
empty list on failure.  Memoised per process; bounded by the
resolver's default timeout.

Related: ``dns`` (forward).

**Examples**

```
rev_dns("1.1.1.1")                              # -> ["one.one.one.one"]
.ltm.node[].address | rev_dns(.)
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

### `socket_get`

TCP connect + read banner.  Requires --enable-probes.

**Signatures**

- `socket_get(host: string, port: integer[, send: string]) -> string`

**Details**

Opens a TCP socket to *(host, port)*, optionally sends *send*,
reads up to 4096 bytes, and returns the response as UTF-8
(replacement on non-text bytes).  Useful for protocol-banner
fingerprinting — SSH versions, SMTP greetings, etc.

**Examples**

```
socket_get("ssh.example.com", 22)
socket_get("smtp.example.com", 25)
```

### `subnet_of`

True when *subnet* lies entirely inside *supernet*.

**Signatures**

- `subnet_of(subnet: string, supernet: string) -> boolean`

**Details**

Wraps ``ipaddress.IPv4Network.subnet_of`` /
``IPv6Network.subnet_of``.  Both arguments must be networks
(CIDR or dotted-quad netmask form).  IPv4 ↔ IPv6 comparison
returns ``false`` rather than raising — different families are
never subsets.

Related: ``in_cidr`` (single-host membership), ``prefix_length``.

**Examples**

```
subnet_of("10.1.0.0/16", "10.0.0.0/8")               # -> true
subnet_of(.net.self[].address, "10.0.0.0/8")
```

### `supernet_of`

Return the smallest single CIDR that covers every address or network in *values*.

**Signatures**

- `supernet_of(values: list[string]) -> string | null`

**Details**

Finds the minimal supernet that contains every input.
Plain IPs are treated as ``/32`` (IPv4) or ``/128`` (IPv6).
Mixed-family inputs raise — IPv4 and IPv6 are never in the
same supernet.  Returns ``null`` when *values* is empty.

Pairs with ``collapse_cidrs`` for the two natural CIDR-
algebra operations: "merge what's already adjacent" versus
"what's the bounding CIDR".

Related: ``collapse_cidrs``, ``subnet_of``, ``in_cidr``.

**Examples**

```
supernet_of(["10.0.0.1", "10.0.1.1"])             # -> "10.0.0.0/23"
supernet_of(["10.0.0.0/24", "10.0.1.0/24"])       # -> "10.0.0.0/23"
.ltm.pool[].members[].address | [.] | supernet_of(.)
```

### `tls_handshake`

Open a TLS connection and inspect what the peer offered.

**Signatures**

- `tls_handshake(host: string, port: integer[, sni: string]) -> object`

**Details**

Performs a full TLS handshake against *(host, port)* (with
SNI defaulting to *host*) and returns the negotiated
protocol, cipher suite, ALPN selection, peer certificate
dict, and verify status against the system trust store.

Requires ``--enable-probes``.

**Examples**

```
tls_handshake("example.com", 443) | .protocol
tls_handshake("example.com", 443) | .peer_cert.subject
```

### `traceroute`

Hop-by-hop path probe to *ip*.  Requires --enable-probes.

**Signatures**

- `traceroute(ip: string) -> list[object]`

**Details**

Subprocess invocation of ``traceroute``.  Returns one record
per hop: ``{hop: int, ip: string | null, rtt_ms: float | null}``.
Hops the router didn't answer for show up with ``ip=null``.

**Examples**

```
traceroute("8.8.8.8") | last(.) | .ip
```

### `url_get`

HTTP GET request.  Requires --enable-probes.

**Signatures**

- `url_get(url: string[, headers: object]) -> object`

**Details**

Issues an HTTP ``GET`` to *url* via urllib.
Returns ``{status: int | null, headers: object, body: string,
error: string | null}``.  Default timeout 5s.

Optional second argument is a dict of request headers.

Related: ``url_get``, ``url_head``, ``url_post``,
``url_options``.

**Examples**

```
url_get("https://example.com/")
url_get("https://api.example/v1", {"Authorization": "Bearer X"})
```

### `url_head`

HTTP HEAD request.  Requires --enable-probes.

**Signatures**

- `url_head(url: string[, headers: object]) -> object`

**Details**

Issues an HTTP ``HEAD`` to *url* via urllib.
Returns ``{status: int | null, headers: object, body: string,
error: string | null}``.  Default timeout 5s.

Optional second argument is a dict of request headers.

Related: ``url_get``, ``url_head``, ``url_post``,
``url_options``.

**Examples**

```
url_head("https://example.com/")
url_head("https://api.example/v1", {"Authorization": "Bearer X"})
```

### `url_options`

HTTP OPTIONS request.  Requires --enable-probes.

**Signatures**

- `url_options(url: string[, headers: object]) -> object`

**Details**

Issues an HTTP ``OPTIONS`` to *url* via urllib.
Returns ``{status: int | null, headers: object, body: string,
error: string | null}``.  Default timeout 5s.

Optional second argument is a dict of request headers.

Related: ``url_get``, ``url_head``, ``url_post``,
``url_options``.

**Examples**

```
url_options("https://example.com/")
url_options("https://api.example/v1", {"Authorization": "Bearer X"})
```

### `url_post`

HTTP POST request.  Requires --enable-probes.

**Signatures**

- `url_post(url: string[, headers: object]) -> object`

**Details**

Issues an HTTP ``POST`` to *url* via urllib.
Returns ``{status: int | null, headers: object, body: string,
error: string | null}``.  Default timeout 5s.

Optional second argument is a dict of request headers.

Related: ``url_get``, ``url_head``, ``url_post``,
``url_options``.

**Examples**

```
url_post("https://example.com/")
url_post("https://api.example/v1", {"Authorization": "Bearer X"})
```

### `with_folder`

Return *path* with its folder portion replaced by *folder*.

**Signatures**

- `with_folder(path: string, folder: string) -> string`

**Details**

Replaces every segment from the leading slash up to (but not
including) the leaf name.  ``folder`` may be a single
partition (``/Common``) or a nested folder
(``/Common/Application_X``); the leaf is kept exactly.

Related: ``folder``, ``with_partition``, ``basename``.

**Examples**

```
with_folder("/Common/web_pool", "/Tenant_A")              # -> "/Tenant_A/web_pool"
with_folder("/Common/iApps/old.app/pool_1", "/Common/iApps/new.app")  # -> "/Common/iApps/new.app/pool_1"
```

### `with_host`

Return *dest* with its host replaced by *host*.

**Signatures**

- `with_host(dest: string, host: string) -> string`

**Details**

Preserves the partition, folder, route-domain, port, and IPv6
bracket form; replaces only the address.  ``host`` may be an
IPv4, IPv6, or FQDN string.

Inverse of :func:`host`; pairs with :func:`with_port` for full
destination editing.

Related: ``host``, ``with_port``, ``with_partition``.

**Examples**

```
with_host(.destination, "10.0.0.2")
with_host("/Common/10.0.0.1:80", "host.example.com")   # -> "/Common/host.example.com:80"
```

### `with_name`

Return *path* with its leaf name replaced by *name*.

**Signatures**

- `with_name(path: string, name: string) -> string`

**Details**

Preserves the partition + every folder segment; replaces only
the final segment (the object's bare name).  Useful for
relocating an object inside its existing folder context:
``with_name("/Common/iApps/Tenant.app/old_pool", "new_pool")``
→ ``"/Common/iApps/Tenant.app/new_pool"``.

Both spellings are accepted as the *path* argument:

- **Full path** (``"/Common/old_pool"``): the partition + folder
  segments are preserved and only the leaf is replaced.
- **Bare leaf** (``"old_pool"`` — what ``.name`` projects): no
  partition / folder context to preserve, so the result is just
  the new leaf name.  This makes ``.name |= with_name(., "X")``
  work the same way ``."full-path" |= with_name(., "X")`` does.

Related: ``basename`` (extract leaf), ``with_partition``,
``with_folder``.

**Examples**

```
with_name("/Common/old_pool", "new_pool")
with_name(."full-path", "renamed")
with_name(.name, "renamed")
```

### `with_port`

Return *dest* with its port replaced by *port*.

**Signatures**

- `with_port(dest: string, port: integer | string) -> string`

**Details**

Preserves every other component of the destination — partition,
folder, address, route-domain, IPv6 brackets, and the
``.``-vs-``:`` port separator.  ``port`` can be an integer
(``443``), the wildcard string (``"any"`` / ``"*"`` / ``"0"``),
or the empty string to strip the port entirely.

Inverse of :func:`port`; pairs with ``with_partition`` /
``with_route_domain`` for full destination editing.

Related: ``port``, ``with_host``, ``with_partition``,
``with_route_domain``.

**Examples**

```
with_port(.destination, 8443)
with_port("/Common/10.0.0.1:80", "any")              # -> "/Common/10.0.0.1:any"
.ltm.virtual[] | .destination |= with_port(., 443)
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

### `x509_parse`

Parse a PEM-encoded X.509 certificate.

**Signatures**

- `x509_parse(pem: string) -> object`

**Details**

Returns a dict of fields: subject, issuer, not_before,
not_after, serial, fingerprint_sha256, sans, key_alg,
key_size, sig_alg, version, public_key_pem.  Uses
:mod:`cryptography` when available; falls back to stdlib
:mod:`ssl` (a subset of fields) when not.

Does NOT need ``--enable-probes`` — it operates on locally-
held PEM text.  Pair with ``url_get`` or ``json_load`` to
feed it certificate data.

Related: ``tls_handshake`` (negotiated chain), ``json_load``.

**Examples**

```
x509_parse(json_load("/etc/ssl/cert.pem"))
tls_handshake("example.com", 443).peer_cert
```

## graph

Forward / reverse references across the same edge model `f5 grep` walks.  One hop deep; multi-hop walks belong in `f5 grep` for now.

### `check_partition_visibility`

Return every reference in this config that violates F5 partition visibility rules.

**Signatures**

- `check_partition_visibility() -> list`

**Details**

Walks the parsed config and surfaces every reference whose
*referrer* partition can't see the *target* partition under the
F5 partition-visibility rules (see :func:`can_see`).  Returns a
list of ``"<referrer> -> <target>"`` strings — empty list when
every reference is legal.

Used to validate a config before applying a partition-level
refactor, or to audit a config that was hand-edited and may
have grown invalid cross-partition refs over time.

Related: ``can_see``, ``references_to``, ``rename_partition``.

**Examples**

```
check_partition_visibility()
count(check_partition_visibility())  # 0 → config is partition-clean
```

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

### `references_to`

Return every object in this config that references *path*.

**Signatures**

- `references_to(path: string) -> list`

**Details**

Walks the parsed BIG-IP config for the current document and
returns every object whose body contains a token-bounded
reference to *path*.  Routes through the same engine
``f5 grep`` uses, so the search picks up references in:

- property values (``pool /Common/p``);
- compound values (destination prefixes,
  pool-member partition prefixes, profile attachment lists);
- iRule body command arguments (``pool $member`` /
  ``class match …`` / ``persist …``).

Multi-file workspaces: only the current document's graph is
walked, mirroring the per-file semantics of mutating queries.

Related: ``refs``, ``referenced_by`` (object-relative graph
forms — pass an object value, get its forward / reverse
edges).

**Examples**

```
references_to("/Common/web_pool")
count(references_to("/Common/log_irule"))
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

### `json_load`

Read a file from disk and parse it as JSON.

**Signatures**

- `json_load(path: string) -> any`

**Details**

Reads *path* from the local filesystem and returns the parsed
JSON value.  Use this to mix external data (CMDB exports,
vlan-to-tenant maps, signed-cert manifests) into a query:

.. code-block:: text

   json_load("/etc/inventory/servers.json") as $inv
     | .ltm.node[]
     | {name: .name, owner: $inv[.address]}

Tilde expansion is honoured (``json_load("~/data/x.json")``).
Raises :class:`BuiltinError` for missing files or invalid
JSON — failures are explicit rather than producing ``null``.

**Examples**

```
json_load("/etc/inventory/servers.json")
.ltm.node[].address as $a | json_load("data.json")[$a]
```

### `json_parse`

Parse a JSON string into its native value.

**Signatures**

- `json_parse(text: string) -> any`

**Details**

Counterpart to :func:`json_load` for in-memory strings.
Useful for parsing the ``body`` of an HTTP response or any
other JSON-bearing text the query already has in hand:

.. code-block:: text

   url_get("https://api.example/v1/inventory")
     | json_parse(.body)
     | .servers

Raises :class:`BuiltinError` on invalid JSON.

**Examples**

```
json_parse("[1, 2, 3]")                          # -> [1, 2, 3]
url_get("https://api/v1") | json_parse(.body)
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

### `source_file`

Return the source file URI of the current object.

**Signatures**

- `source_file(value: object) -> string | null`

**Details**

Resolves the source URI of the BIG-IP object passed in (the file
a ``ltm pool`` / ``ltm virtual`` / ... stanza was parsed from).
Most useful in ``--merge`` mode, where a single query streams
objects from several inputs and the consumer wants to label each
by origin: ``.ltm.virtual[] | {name: .name, src: source_file}``.

Returns ``null`` for synthetic / non-object values.  The result is
the source URI as stored on the underlying :class:`ObjectRef`
(typically a ``file:///`` URL); pair with ``basename`` for a
short filename.

Related: ``--merge`` mode, ``$name`` for explicit per-source
binding.

**Examples**

```
.ltm.virtual[] | {name: .name, src: source_file}
.ltm.pool[] | {name: .name, file: basename(source_file)}
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
