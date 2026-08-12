# F5 query DSL — builtin function reference

> **Audience:** Developer / Maintainer
> **Type:** Reference

**This page mirrors the builtin registry in
`rust/tcl-bigip-query/src/builtins/`.  Edit that registry, not this file.**
The same registry backs `f5 query --help-manual`, so the built binary is
the authority when the two disagree.

This is the **canonical per-function reference** for every builtin the
`f5 query` DSL exposes.  For grammar, value-model, edit-pipeline, and
architectural context see [`dsl.md`](dsl.md) and [`manual.md`](manual.md);
for the user-facing feature overview and worked-example KCS notes start
from
[`../../kcs/features/kcs-feature-bigip-query.md`](../../kcs/features/kcs-feature-bigip-query.md).

The same per-function reference is available offline through the
verb's own help action — ``f5 query --help-builtins NAME`` prints
exactly the same content for one builtin.

## Categories


- **[stream](#stream)** — Sequence-shaped operations: filter (`select`), transform (`map`), aggregate (`any` / `all` / `count` / `min` / `max` / `add` / `unique` / `dupes` / `sort` / `reverse`), generators (`range`), grouping (`group_by` / `unique_by` / `sort_by` / `min_by` / `max_by`), set membership (`IN` / `INDEX` / `inside` / `combinations`), flow control (`empty` / `error` / `not`), and the object-introspection helpers (`keys` / `values` / `first` / `last` / `nth` / `limit` / `flatten`).
  - [`IN`](#IN), [`INDEX`](#INDEX), [`add`](#add), [`all`](#all), [`any`](#any), [`combinations`](#combinations), [`count`](#count), [`debug`](#debug), [`dupes`](#dupes), [`empty`](#empty), [`error`](#error), [`first`](#first), [`flatten`](#flatten), [`group_by`](#group_by), [`halt`](#halt), [`halt_error`](#halt_error), [`inside`](#inside), [`keys`](#keys), [`keys_unsorted`](#keys_unsorted), [`last`](#last), [`limit`](#limit), [`map`](#map), [`map_values`](#map_values), [`max`](#max), [`max_by`](#max_by), [`max_min`](#max_min), [`min`](#min), [`min_by`](#min_by), [`min_max`](#min_max), [`not`](#not), [`nth`](#nth), [`range`](#range), [`reverse`](#reverse), [`select`](#select), [`sort`](#sort), [`sort_by`](#sort_by), [`stderr`](#stderr), [`unique`](#unique), [`unique_by`](#unique_by), [`values`](#values)
- **[string](#string)** — String predicates and rewrites: substring / prefix / suffix tests, regex `match` / `test` / `sub` / `gsub` / `scan` / `capture` / `splits` (all flag-aware), plain `split` / `join`, casing, trims (`ltrimstr` / `rtrimstr`), conversions (`tonumber` / `tostring` / `tojson` / `fromjson` / `explode` / `implode` / `ascii` / `utf8bytelength`), and jq-style encodings (`uri` / `base64` / `base64d` / `html` / `sh`).
  - [`ascii`](#ascii), [`ascii_downcase`](#ascii_downcase), [`ascii_upcase`](#ascii_upcase), [`base64`](#base64), [`base64d`](#base64d), [`capture`](#capture), [`contains`](#contains), [`csv`](#csv), [`downcase`](#downcase), [`endswith`](#endswith), [`explode`](#explode), [`fromjson`](#fromjson), [`gsub`](#gsub), [`html`](#html), [`implode`](#implode), [`index`](#index), [`join`](#join), [`ltrimstr`](#ltrimstr), [`match`](#match), [`rtrimstr`](#rtrimstr), [`scan`](#scan), [`sh`](#sh), [`split`](#split), [`splits`](#splits), [`startswith`](#startswith), [`sub`](#sub), [`test`](#test), [`tojson`](#tojson), [`tonumber`](#tonumber), [`tostring`](#tostring), [`tsv`](#tsv), [`upcase`](#upcase), [`uri`](#uri), [`utf8bytelength`](#utf8bytelength)
- **[math](#math)** — Numeric helpers matching jq's C-math surface: rounding (`floor` / `ceil` / `round` / `trunc` / `rint`), magnitude / sign (`abs` / `fabs` / `copysign` / `fdim`), powers / roots (`sqrt` / `cbrt` / `pow` / `exp` / `exp2` / `exp10`), logarithms (`log` / `log2` / `log10` / `logb`), trigonometry (`sin` / `cos` / `tan` / `asin` / `acos` / `atan` / `atan2`), hyperbolics (`sinh` / `cosh` / `tanh` / `asinh` / `acosh` / `atanh`), special functions (`gamma` / `tgamma` / `lgamma`, Bessel `j0` / `j1` / `y0` / `y1`), bit-level decomposition (`frexp` / `ldexp` / `modf` / `significand`), and IEEE-754 sentinels (`nan` / `infinite` / `isnan` / `isinfinite` / `isnormal`).
  - [`abs`](#abs), [`acos`](#acos), [`acosh`](#acosh), [`asin`](#asin), [`asinh`](#asinh), [`atan`](#atan), [`atan2`](#atan2), [`atanh`](#atanh), [`cbrt`](#cbrt), [`ceil`](#ceil), [`copysign`](#copysign), [`cos`](#cos), [`cosh`](#cosh), [`drem`](#drem), [`exp`](#exp), [`exp10`](#exp10), [`exp2`](#exp2), [`expm1`](#expm1), [`fabs`](#fabs), [`fdim`](#fdim), [`floor`](#floor), [`fma`](#fma), [`fmax`](#fmax), [`fmin`](#fmin), [`fmod`](#fmod), [`frexp`](#frexp), [`gamma`](#gamma), [`hypot`](#hypot), [`infinite`](#infinite), [`isinfinite`](#isinfinite), [`isnan`](#isnan), [`isnormal`](#isnormal), [`j0`](#j0), [`j1`](#j1), [`jn`](#jn), [`ldexp`](#ldexp), [`lgamma`](#lgamma), [`lgamma_r`](#lgamma_r), [`log`](#log), [`log10`](#log10), [`log1p`](#log1p), [`log2`](#log2), [`logb`](#logb), [`modf`](#modf), [`nan`](#nan), [`nearbyint`](#nearbyint), [`pow`](#pow), [`pow10`](#pow10), [`remainder`](#remainder), [`rint`](#rint), [`round`](#round), [`significand`](#significand), [`sin`](#sin), [`sinh`](#sinh), [`sqrt`](#sqrt), [`tan`](#tan), [`tanh`](#tanh), [`tgamma`](#tgamma), [`trunc`](#trunc), [`y0`](#y0), [`y1`](#y1), [`yn`](#yn)
- **[time](#time)** — Time and date helpers matching jq's surface: epoch reads (`now`), ISO-8601 conversions (`todate` / `todateiso8601` / `fromdate` / `fromdateiso8601` / `date`), broken-down time (`gmtime` / `localtime` / `mktime`), formatting / parsing (`strftime` / `strptime`), and epoch arithmetic (`dateadd` / `datesub`).
  - [`date`](#date), [`dateadd`](#dateadd), [`datesub`](#datesub), [`fromdate`](#fromdate), [`fromdateiso8601`](#fromdateiso8601), [`gmtime`](#gmtime), [`localtime`](#localtime), [`mktime`](#mktime), [`now`](#now), [`strftime`](#strftime), [`strptime`](#strptime), [`todate`](#todate), [`todateiso8601`](#todateiso8601)
- **[path](#path)** — BIG-IP full-path string helpers — extract the partition or basename, swap a partition prefix.  These are *string* transforms; they don't move objects.  For object renames, reach for the **rename** category.
  - [`basename`](#basename), [`partition`](#partition), [`with_partition`](#with_partition)
- **[rename](#rename)** — Cascading rename operations — `rename` for one object, `rename_partition` for every object in a partition.  Both route through the same token-bounded engine `f5 rename` uses, so references inside iRule bodies and compound values (destination addresses, pool-member identifiers) are rewritten consistently.
  - [`rename`](#rename), [`rename_folder`](#rename_folder), [`rename_partition`](#rename_partition), [`rename_prefix`](#rename_prefix)
- **[net](#net)** — IP-address arithmetic and route-domain helpers.  The `ip(net, src)` rebase is the workhorse of bulk readdressing; `with_route_domain` sets / replaces / strips the `%rd` suffix.
  - [`broadcast_address`](#broadcast_address), [`can_see`](#can_see), [`collapse_cidrs`](#collapse_cidrs), [`dns`](#dns), [`first_host`](#first_host), [`folder`](#folder), [`host`](#host), [`host_count`](#host_count), [`http_body`](#http_body), [`http_body_json`](#http_body_json), [`http_client_error`](#http_client_error), [`http_header`](#http_header), [`http_headers`](#http_headers), [`http_ok`](#http_ok), [`http_redirect`](#http_redirect), [`http_server_error`](#http_server_error), [`http_status`](#http_status), [`in_cidr`](#in_cidr), [`in_folder`](#in_folder), [`in_partition`](#in_partition), [`ip`](#ip), [`ip_range_contains`](#ip_range_contains), [`ip_range_count`](#ip_range_count), [`ip_range_supernet`](#ip_range_supernet), [`ip_range_to_cidrs`](#ip_range_to_cidrs), [`ip_translate`](#ip_translate), [`is_documentation`](#is_documentation), [`is_fqdn`](#is_fqdn), [`is_ipv4`](#is_ipv4), [`is_ipv6`](#is_ipv6), [`is_link_local`](#is_link_local), [`is_loopback`](#is_loopback), [`is_multicast`](#is_multicast), [`is_private`](#is_private), [`is_public`](#is_public), [`is_reserved`](#is_reserved), [`is_unspecified`](#is_unspecified), [`is_wildcard_port`](#is_wildcard_port), [`last_host`](#last_host), [`net`](#net), [`network_address`](#network_address), [`overlaps`](#overlaps), [`ping`](#ping), [`port`](#port), [`port_set_contains`](#port_set_contains), [`port_set_count`](#port_set_count), [`port_set_overlaps`](#port_set_overlaps), [`portping`](#portping), [`prefix_length`](#prefix_length), [`rev_dns`](#rev_dns), [`route_domain`](#route_domain), [`socket_get`](#socket_get), [`subnet_of`](#subnet_of), [`supernet_of`](#supernet_of), [`tls_handshake`](#tls_handshake), [`traceroute`](#traceroute), [`ucs_cert`](#ucs_cert), [`url_get`](#url_get), [`url_head`](#url_head), [`url_options`](#url_options), [`url_post`](#url_post), [`with_folder`](#with_folder), [`with_host`](#with_host), [`with_name`](#with_name), [`with_port`](#with_port), [`with_route_domain`](#with_route_domain), [`x509_eq`](#x509_eq), [`x509_from_config`](#x509_from_config), [`x509_parse`](#x509_parse)
- **[graph](#graph)** — Forward / reverse references across the same edge model `f5 grep` walks.  One hop deep; multi-hop walks belong in `f5 grep` for now.
  - [`check_partition_visibility`](#check_partition_visibility), [`referenced_by`](#referenced_by), [`references_to`](#references_to), [`refs`](#refs)
- **[value](#value)** — Type / identity introspection (`kind`, `path`, `length`, `defined`, `type`), object-shape conversions (`to_entries` / `from_entries` / `with_entries` / `has` / `in`), and jq-style tree manipulation (`paths` / `leaf_paths` / `getpath` / `setpath` / `del` / `delpaths` / `walk` / `recurse` / `until` / `repeat`).
  - [`cert_load`](#cert_load), [`csv_load`](#csv_load), [`defined`](#defined), [`del`](#del), [`delpaths`](#delpaths), [`env`](#env), [`f5log_load`](#f5log_load), [`from_entries`](#from_entries), [`getpath`](#getpath), [`has`](#has), [`in`](#in), [`json_load`](#json_load), [`json_parse`](#json_parse), [`jsonl_load`](#jsonl_load), [`kind`](#kind), [`leaf_paths`](#leaf_paths), [`length`](#length), [`path`](#path), [`paths`](#paths), [`pick`](#pick), [`recurse`](#recurse), [`recurse_down`](#recurse_down), [`repeat`](#repeat), [`setpath`](#setpath), [`source_file`](#source_file), [`str`](#str), [`to_entries`](#to_entries), [`type`](#type), [`until`](#until), [`walk`](#walk), [`with_entries`](#with_entries)
- **[bigip](#bigip)** — BIG-IP-specific helpers backed by the profile registry: order a virtual's profiles into traffic order (`profile_order`), and recover the TMOS default field values an SCF omits, keyed by BIG-IP version (`profile_defaults` / `profile_default`).
  - [`profile_default`](#profile_default), [`profile_defaults`](#profile_defaults), [`profile_order`](#profile_order)

## bigip

BIG-IP-specific helpers backed by the shared profile registry
(`tcl_registry::profiles` / `tcl_registry::profile_defaults`).

### `profile_order`

- `profile_order(refs) -> list`

Sort a list of profile references (names or full paths, e.g. a virtual's
`.profiles[].name`) into **traffic order** — transport (TCP/UDP/FASTL4) nearest
the wire, then TLS, then the application profile, with security / acceleration
facets on top. Types are inferred from well-known profile names.

```
.ltm.virtual["/Common/web_vs"].profiles[].name | [ . ] | profile_order(.)
```

### `profile_defaults`

- `profile_defaults(type) -> object`
- `profile_defaults(type, version) -> object`

The TMOS default field values of an unmodified `ltm profile <type>`, as an
object of `field → default`. An SCF (and `tmsh list … one-line`) omits any field
left at its default — those live in the read-only `/config/profile_base.conf` —
so this recovers a base profile's effective configuration. `type` is a profile
type name (`tcp`, `http`, `clientssl`, …; case- and separator-insensitive). The
optional `version` is a BIG-IP version string (`"15.1"`, `"16.1.3.2"`); omitted
resolves the current default. Unknown types yield an empty object.

```
profile_defaults("tcp")                      # every default field of ltm profile tcp
profile_defaults("clientssl", "13.1.0.8")    # as they were on 13.1.0.8
profile_defaults("json", "21.1")             # JSON profile limits introduced in 21.x
```

### `profile_default`

- `profile_default(type, field) -> string|null`
- `profile_default(type, field, version) -> string|null`

The TMOS default value of a single `field` of `ltm profile <type>`, or `null`
when the type/field is unknown. Defaults are keyed by BIG-IP **version range**,
because they drift across releases — e.g. the base `clientssl` `options` gained
`no-tlsv1.3` (TLS 1.3 disabled by default) in 14.0:

```
profile_default("clientssl", "options", "13.1")   # "dont-insert-empty-fragments"
profile_default("clientssl", "options", "16.1")   # "dont-insert-empty-fragments no-tlsv1.3"
profile_default("clientssl", "ciphers", "20.1")   # "DEFAULT"
profile_default("clientssl", "ciphers", "21.1")   # "none"
profile_default("clientssl", "cipher-group", "21.1") # "/Common/f5-default"
profile_default("tcp", "idle-timeout")             # "300"
```

BIG-IP 21.1 changes the canonical Client SSL and Server SSL profiles to use
`/Common/f5-default` and disables TLS 1.0 and TLS 1.1. The earlier settings
remain available on BIG-IP as the `clientssl-legacy` and `serverssl-legacy`
profiles; the resolver keeps the canonical defaults separated at the 21.1
version boundary.

## stream

Sequence-shaped operations: filter (`select`), transform (`map`), aggregate (`any` / `all` / `count` / `min` / `max` / `add` / `unique` / `dupes` / `sort` / `reverse`), generators (`range`), grouping (`group_by` / `unique_by` / `sort_by` / `min_by` / `max_by`), set membership (`IN` / `INDEX` / `inside` / `combinations`), flow control (`empty` / `error` / `not`), and the object-introspection helpers (`keys` / `values` / `first` / `last` / `nth` / `limit` / `flatten`).

### `IN`

True when the current value equals any of the candidate arguments.

**Signatures**

- `IN(...candidates) -> boolean`

**Details**

**Special form.**  Matches jq's ``IN(s)``: returns ``true`` when
the current value compares equal to **any** of the values in the
argument stream.

Our DSL accepts the candidates as ordinary function arguments —
``IN("a", "b", "c")`` — because comma inside a call is the arg
separator, not jq's stream-concat operator.  Each argument is
evaluated against the current input; stream-valued arguments
expand element-wise so the jq pattern
``.ltm.virtual[].name | IN([candidates] | .[])`` works the same
way.

Short-circuits on the first match.

Related: ``INDEX``, ``contains``, ``in``, ``any``.

**Examples**

```
.name | IN("a", "b", "c")
.ltm.virtual[].name | select(IN("web_vs", "api_vs"))
```

### `INDEX`

Build an object keyed by *idx_expr* evaluated against each item.

**Signatures**

- `INDEX(idx_expr) -> object`
- `INDEX(source, idx_expr) -> object`

**Details**

**Special form.**  Matches jq's ``INDEX``: with one argument,
indexes the current list / stream by the value of *idx_expr*
per item.  With two arguments, indexes the stream produced by
*source* the same way.

Duplicate keys: last write wins (jq parity).  Keys are coerced
to strings.

Related: ``group_by``, ``unique_by``, ``to_entries``.

**Examples**

```
[.ltm.virtual[]] | INDEX(.name)
INDEX(.ltm.virtual[]; .name)             # jq's two-arg form
```

### `add`

Combine the items of a list by ``+`` — sum numbers, concatenate strings/lists, merge objects.

**Signatures**

- `add(value: list | stream) -> any`

**Details**

Adds the elements of a list / stream together using the same
semantics as the ``+`` operator (matches jq's ``add``):

- **numbers**: arithmetic sum.
- **strings**: concatenation, in order.
- **lists**: concatenation (single level).
- **objects**: shallow merge with later entries overwriting
  earlier ones.

An **empty** input returns ``null`` (jq parity).  Items must be
homogeneous; mixing types raises ``BuiltinError`` — coerce with
``str`` / ``tonumber`` / ``map`` first if needed.

Related: ``flatten``, ``join`` (string-only with a separator),
``map``.

**Examples**

```
[1, 2, 3] | add                          # -> 6
["foo", "bar"] | add                     # -> "foobar"
[.ltm.virtual[].rules] | add             # flat list of every rule attachment
[.ltm.virtual[].pool.members[]] | add | length
```

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

### `combinations`

Cartesian product of a list of lists.

**Signatures**

- `combinations() -> stream[list]`
- `combinations(n: integer) -> stream[list]`

**Details**

**Special form.**  Matches jq's ``combinations``: with no
argument, returns every combination drawn from a list of lists —
one element from each sub-list.  With an integer ``n``, returns
every n-length combination of the current list's elements
(repeats allowed).

The result is a stream of lists.  For empty input or empty
sub-lists, the stream is empty.

Operates on the current input — call via pipe
(``[X] | combinations``) or directly (``combinations``).

Related: ``map``, ``range``, ``flatten``.

**Examples**

```
[[1, 2], [3, 4]] | combinations          # -> [1,3], [1,4], [2,3], [2,4]
[1, 2, 3] | combinations(2)              # -> [1,1], [1,2], ..., [3,3]
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

### `debug`

Pass-through that logs the current value to stderr.

**Signatures**

- `debug() -> any`
- `debug(label: string) -> any`

**Details**

**Special form.**  Matches jq's ``debug``: returns the current
value unchanged but writes a debug line to stderr
(``["DEBUG:", value]`` in jq's one-arg form, ``["DEBUG:", label,
value]`` with a label).  Useful for tracing complex pipelines
without adding extra pipeline stages.

Related: ``stderr``, ``error``.

**Examples**

```
.ltm.virtual[] | debug | .name
.ltm.virtual[] | debug("vs") | .name
```

### `dupes`

Return the duplicated items of a list — values that appear more than once, sorted.

**Signatures**

- `dupes(value: list | stream) -> list`

**Details**

Returns the items that appear **more than once** in *value*, with
each duplicate represented once and the result sorted by the same
ordering ``unique`` and ``sort`` use.

The complement of ``unique``: where ``unique`` collapses a list to
its distinct values, ``dupes`` keeps only the values whose count is
at least two.  Useful for triage queries — finding shared pool
names, repeated rule attachments, duplicated VIPs across configs.

Empty input returns an empty list.  :class:`PathRef` items are
compared on their ``full_path``, mixed types fall into jq's
cross-type ordering.

Related: ``unique``, ``unique_by``, ``group_by``, ``sort``.

**Examples**

```
[.ltm.virtual[].pool] | dupes            # pools attached to more than one VS
[.ltm.virtual[].destination | host] | dupes  # IPs reused across VSes
```

### `empty`

Emit no values — the zero element of jq's stream algebra.

**Signatures**

- `empty() -> stream`

**Details**

Matches jq's ``empty``: produces a stream with zero items.  Used
inside ``if ... then ... else empty end`` to silently drop a
branch, or inside ``map(...)`` to filter without ``select``.

Related: ``select`` (drops one item), ``error`` (raises instead).

**Examples**

```
if .pool then .name else empty end
map(if .x > 0 then .x else empty end)
```

### `error`

Raise a query error with a custom message.

**Signatures**

- `error() -> never`
- `error(msg: string) -> never`

**Details**

Matches jq's ``error``: aborts the query with an error.  With no
argument, raises with the current value's string form; with a
message, raises with that message.

Useful for fail-fast validation inside ``if`` / ``select``
pipelines: ``if .destination == null then error("VS has no dest")
else . end``.

Related: ``select``, ``empty``.

**Examples**

```
if .pool == null then error("no pool") else . end
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

### `flatten`

Flatten a nested list by one level, or by *depth* when specified.

**Signatures**

- `flatten() -> list`
- `flatten(depth: integer) -> list`

**Details**

**Special form.**  Matches jq's ``flatten``: with no depth
argument, flattens nested lists by exactly **one** level.  With a
*depth* argument, flattens that many levels deep.  ``flatten(0)``
is the identity.

The value is always the current input — call as
``[X] | flatten`` or ``[X] | flatten(2)``.

Non-list elements pass through unchanged at each level; this lets
you flatten a mixed stream of "string or list of strings" without
error.

A negative depth raises ``BuiltinError`` — jq's behaviour is
"flatten infinitely" for negative depths, but in this DSL that
pattern is almost always a typo, so we reject it explicitly.

Related: ``add`` (concatenates one level), ``map``.

**Examples**

```
[[1, 2], [3, 4]] | flatten               # -> [1, 2, 3, 4]
[[1, [2, 3]], [4]] | flatten             # -> [1, [2, 3], 4]
[[1, [2, 3]], [4]] | flatten(2)          # -> [1, 2, 3, 4]
[.ltm.virtual[].rules] | flatten | unique
```

### `group_by`

Group items by the value of *body*; returns a list of groups sorted by key.

**Signatures**

- `group_by(body) -> list[list]`

**Details**

**Special form.**  Matches jq's ``group_by``: for each input item,
evaluates *body* with ``.`` re-bound, then partitions the input
into groups of items sharing the same key value.  The outer list
is sorted by the group keys (jq's cross-type ordering); within
each group, items preserve their input order.

Pair with ``map(length)`` for a histogram, or ``map(first)`` for
one representative per group (cheaper than ``unique_by`` when you
also want the group counts).

Related: ``sort_by``, ``unique_by``, ``map``, ``count``.

**Examples**

```
[.ltm.virtual[]] | group_by(partition(.name))
[.ltm.virtual[]] | group_by(.pool) | map(length)  # VS count per pool
```

### `halt`

Halt the query silently — no further output.

**Signatures**

- `halt() -> never`

**Details**

Matches jq's ``halt``: terminates query evaluation without
emitting an error message.  In this DSL, ``halt`` raises a
distinct ``BuiltinError`` flagged so the runner exits with status
0 (versus ``halt_error`` which exits non-zero).

Useful for "I've found what I wanted, stop early" pipelines.

Related: ``halt_error``, ``error``, ``empty``.

**Examples**

```
.ltm.virtual[] | select(.name == "web_vs") | halt
```

### `halt_error`

Halt the query with an error message and exit code.

**Signatures**

- `halt_error() -> never`
- `halt_error(exit_code: integer) -> never`

**Details**

Matches jq's ``halt_error``: terminates evaluation and signals
a non-zero exit.  With an optional integer argument, jq sets
that exit code; this DSL preserves the code on the error so the
CLI can map it to a process exit.

Related: ``halt``, ``error``.

**Examples**

```
.ltm.virtual[] | select(.pool == null) | halt_error(5)
```

### `inside`

Inverse of ``contains`` — current is *inside* the given container.

**Signatures**

- `inside(needle: any, container: any) -> boolean`

**Details**

Matches jq's ``inside``: ``a | inside(b)`` is equivalent to
``b | contains(a)``.  String substring test, list element test,
dict submap test.  Use the implicit-receiver form for the
natural reading: ``"bar" | inside("foobar")``.

Related: ``contains``, ``has``, ``in``.

**Examples**

```
"bar" | inside("foobar")                 # -> true
"/Common/log_rule" | inside(.rules)      # rule in attached set?
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

### `keys_unsorted`

Field names of an object in insertion order (jq's ``keys_unsorted``).

**Signatures**

- `keys_unsorted(value: object) -> list[string]`

**Details**

Matches jq's ``keys_unsorted``: like ``keys`` but does **not**
sort the result.  Returns the field names in the order they were
first seen on the object.  For lists / streams, returns indices
``0..n-1`` (matching jq).

Use ``keys`` for the sorted form.

Related: ``keys``, ``values``, ``to_entries``.

**Examples**

```
{b: 1, a: 2} | keys_unsorted
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

### `limit`

Take the first *n* items of a list or stream.

**Signatures**

- `limit(value: list | stream, n: integer) -> list`

**Details**

Returns the first *n* items.  When the input has fewer than *n*
items, returns the input unchanged; when *n* is zero or negative,
returns an empty list.

Convenience for "give me a preview of the result" or "cap a
potentially-large stream" — paginate by combining with ``range``
and slicing.

Note: jq's ``limit(n; gen)`` is a two-argument special form that
takes a generator expression.  This DSL's ``limit`` is the value
form — pipe a stream / list into it, the same way ``count`` /
``sort`` work.

Related: ``first``, ``nth``, ``range``.

**Examples**

```
[.ltm.virtual[].name] | sort | limit(., 5)  # first five names alphabetically
.ltm.virtual[] | limit(3)                # first three VSes
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
  ``[.ltm.virtual[].name | partition(.)] | unique`` (``unique``
  already returns sorted output, jq parity).

Related: ``select``, ``any``, ``all``, ``unique``, ``sort``.

**Examples**

```
.rules | map(basename(.))
[.ltm.virtual[].name | partition(.)] | unique
any(.pool.members[].address | in_cidr(., "10.0.0.0/8"))
```

### `map_values`

Apply *body* to every value of an object / array, keeping shape.

**Signatures**

- `map_values(body) -> any`

**Details**

**Special form.**  Matches jq's ``map_values``: equivalent to
``with_entries(.value |= body)`` for objects and ``map(body)``
for arrays.  Preserves the input's shape — an object stays an
object with the same keys, an array stays an array.

Returning the ``select`` drop sentinel removes the value's slot.

Related: ``map``, ``with_entries``, ``select``.

**Examples**

```
{a: 1, b: 2} | map_values(. * 10)        # -> {a: 10, b: 20}
[1, 2, 3] | map_values(. * 10)            # -> [10, 20, 30]
```

### `max`

Largest element of a list or stream, or null when empty.

**Signatures**

- `max(value: list | stream) -> any`

**Details**

Returns the maximum element using jq's cross-type ordering.
:class:`PathRef` collates by ``full_path``.  Empty input returns
``null`` — matches jq.

For "largest by a derived key" use ``max_by(f)``.

Related: ``min``, ``max_by``, ``sort``, ``last``.

**Examples**

```
[1, 5, 2, 8, 3] | max                    # -> 8
[.ltm.virtual[].name] | max              # alphabetically last VS name
```

### `max_by`

Item whose *body* value is largest under jq's cross-type ordering.

**Signatures**

- `max_by(body) -> any`

**Details**

**Special form.**  Like ``min_by`` but picks the largest.  Empty
input returns ``null``.

Related: ``max``, ``min_by``, ``sort_by``, ``last``.

**Examples**

```
[.ltm.pool[]] | max_by(.members | length)  # biggest pool
[.ltm.virtual[]] | max_by(.name)            # alphabetically last VS
```

### `max_min`

Return ``[max, min]`` of a list, keyed by *body*.

**Signatures**

- `max_min() -> list`
- `max_min(body) -> list`

**Details**

**Special form.**  Matches jq 1.7's ``max_min``: like ``min_max``
but in the opposite order — ``[maximum, minimum]``.

Related: ``min_max``, ``min``, ``max``, ``min_by``, ``max_by``.

**Examples**

```
[5, 2, 8, 1] | max_min
```

### `min`

Smallest element of a list or stream, or null when empty.

**Signatures**

- `min(value: list | stream) -> any`

**Details**

Returns the minimum element using jq's cross-type ordering
(``null < false < true < numbers < strings < arrays < objects``).
:class:`PathRef` collates by ``full_path``.

Empty input returns ``null`` — matches jq.  For "smallest by a
derived key" use ``min_by(f)``.

Related: ``max``, ``min_by``, ``sort``, ``first``.

**Examples**

```
[1, 5, 2, 8, 3] | min                    # -> 1
[.ltm.virtual[].name] | min              # alphabetically first VS name
```

### `min_by`

Item whose *body* value is smallest under jq's cross-type ordering.

**Signatures**

- `min_by(body) -> any`

**Details**

**Special form.**  Matches jq's ``min_by``: for each input item,
evaluates *body* with ``.`` re-bound, and returns the item whose
derived key is the smallest under jq's cross-type ordering.

On ties, returns the first such item (Python's ``min`` is stable).
Empty input returns ``null``.

Related: ``min``, ``max_by``, ``sort_by``, ``first``.

**Examples**

```
[.ltm.pool[]] | min_by(.members | length)  # smallest pool
[.ltm.virtual[]] | min_by(.name)            # alphabetically first VS
```

### `min_max`

Return ``[min, max]`` of a list, keyed by *body*.

**Signatures**

- `min_max() -> list`
- `min_max(body) -> list`

**Details**

**Special form.**  Matches jq 1.7's ``min_max``: returns a
two-element list ``[minimum, maximum]``.  Without *body*, items
compare under jq's cross-type ordering; with *body*, each item's
key is the result of *body* applied to it (like ``min_by`` /
``max_by``).

Empty input returns ``[null, null]`` (jq parity).

Related: ``min``, ``max``, ``min_by``, ``max_by``, ``max_min``.

**Examples**

```
[5, 2, 8, 1] | min_max
[.ltm.pool[]] | min_max(.members | length)
```

### `not`

Invert the truthiness of the input — jq's postfix ``not``.

**Signatures**

- `not() -> boolean`

**Details**

Matches jq's ``not``: returns ``true`` when the current value is
falsy and ``false`` otherwise.  Truthiness follows the DSL's
usual rules (null / false / empty string / empty list are falsy).

The DSL also exposes ``not`` as a unary prefix operator — both
forms exist for jq snippet compatibility.

Related: ``select``, ``any``, ``all``.

**Examples**

```
.pool | not                             # true when pool is unset
.ltm.virtual[] | select(.snatpool | not)
```

### `nth`

The n-th element of a list or stream (0-indexed), or null when out of range.

**Signatures**

- `nth(value: list | stream, n: integer) -> any`

**Details**

Returns the *n*-th element (0-indexed) of a list or stream.  Out-
of-range indices return ``null`` rather than raising — matches the
"safe access" feel of ``first`` / ``last``.

Accepts the jq-style implicit receiver: ``stream | nth(2)`` is the
same as ``nth(stream, 2)``.  Negative indices count from the end
(``nth(stream, -1)`` is the last item) — a convenience over jq,
which doesn't accept negatives.

Related: ``first``, ``last``, ``limit``, ``range``.

**Examples**

```
[.ltm.virtual[].name] | nth(0)           # first VS name
.ltm.virtual.web_vs.rules | nth(0)        # first attached rule
```

### `range`

Generate a stream of integers — jq's range(); 1, 2, or 3 args.

**Signatures**

- `range(upto: integer) -> stream[integer]`
- `range(from: integer, upto: integer) -> stream[integer]`
- `range(from: integer, upto: integer, step: integer) -> stream[integer]`

**Details**

Matches jq's ``range`` exactly.  Emits a stream of integers:

- One arg ``upto`` → ``0, 1, 2, … upto-1``.
- Two args ``from, upto`` → ``from, from+1, … upto-1``.
- Three args ``from, upto, step`` → arithmetic progression, stops
  strictly before *upto* (positive step) or strictly after *upto*
  (negative step).

A *step* of zero raises ``BuiltinError``.  All arguments must be
integers.

Useful for synthetic streams: indexed enumeration, cartesian-style
pairings with array generators, fixed-length placeholder runs.

Related: ``limit``, ``nth``.

**Examples**

```
range(3)                                 # -> 0, 1, 2
range(2, 6)                              # -> 2, 3, 4, 5
range(0, 10, 2)                          # -> 0, 2, 4, 6, 8
[range(5)] | map(. * 2)                  # -> [0, 2, 4, 6, 8]
```

### `reverse`

Reverse a list or string.

**Signatures**

- `reverse(value: list | stream) -> list`
- `reverse(value: string) -> string`

**Details**

Returns the input with its elements (or characters) in reverse
order.  Matches jq's ``reverse``: lists and arrays reverse
element-wise; strings reverse character-wise.

:class:`PathRef` values are reversed as their ``full_path``
string.  ``null`` returns ``null``.

Related: ``sort``, ``first``, ``last``.

**Examples**

```
[.ltm.virtual[].name] | sort | reverse   # descending names
reverse("abc")                           # -> "cba"
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
[.ltm.virtual[].destination | host] | sort | reverse  # descending hosts
```

### `sort_by`

Sort a list by the value of *body* evaluated against each item.

**Signatures**

- `sort_by(body) -> list`

**Details**

**Special form.**  Matches jq's ``sort_by``: for each input item,
evaluates *body* with ``.`` re-bound to that item and uses the
result as the sort key.  The original items are returned in order
of their derived keys, using jq's cross-type ordering.

The body is unevaluated AST and is re-run per item, so it may be a
field projection (``sort_by(.name)``), a builtin call
(``sort_by(partition(.))``), or an arithmetic expression.

Stable: ties keep input order (Python's ``sorted`` is stable).

Related: ``sort``, ``unique_by``, ``min_by``, ``max_by``,
``group_by``.

**Examples**

```
[.ltm.virtual[]] | sort_by(.name)
[.ltm.pool[]] | sort_by(.members | length)  # smallest pools first
[.ltm.virtual[]] | sort_by(partition(.name))
```

### `stderr`

Pass-through that writes the current value to stderr as JSON.

**Signatures**

- `stderr() -> any`

**Details**

**Special form.**  Matches jq's ``stderr``: returns the current
value unchanged but writes its JSON encoding to stderr.  Same
chokepoint as ``debug`` without the ``["DEBUG:", ...]`` wrapping.

Related: ``debug``, ``error``.

**Examples**

```
.ltm.virtual[] | stderr | .name
```

### `unique`

Return the unique items of a list, sorted.

**Signatures**

- `unique(value: list | stream) -> list`

**Details**

De-duplicates a list or stream and returns the unique items in
sorted order.  Matches jq's ``unique`` exactly: input is treated
as an array, output is the sorted unique values.  :class:`PathRef`
items collate by their ``full_path``, mixed types fall into jq's
cross-type ordering (``null < bool < number < string < array <
object``).

Unhashable items (rare — usually nested lists) collate through the
same :func:`_sort_key` the ``sort`` builtin uses, so the result is
deterministic across runs and Python interpreter versions.

For grouping with a key function use ``unique_by(f)`` instead,
which keeps one representative per equivalence class.

Related: ``sort``, ``unique_by``, ``dupes``, ``count``, ``map``.

**Examples**

```
[.ltm.virtual[].pool] | unique           # every distinct default pool
[.ltm.virtual[].name | partition(.)] | unique  # used partitions, sorted
```

### `unique_by`

Sorted unique items, where uniqueness is determined by *body*.

**Signatures**

- `unique_by(body) -> list`

**Details**

**Special form.**  Matches jq's ``unique_by``: returns the unique
items of the input, where two items are considered equal when
*body* evaluates to the same value for both.  The result is
sorted by the same key.

Equivalent to ``[sort_by(body)] | <dedupe-by-key>``.  One
representative per equivalence class survives — Python's ``sorted``
is stable, so the representative is the **first** input occurrence
of each key.

Related: ``unique``, ``sort_by``, ``group_by``, ``dupes``.

**Examples**

```
[.ltm.virtual[]] | unique_by(.pool)      # one VS per distinct default pool
[.ltm.virtual[]] | unique_by(partition(.name))
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

String predicates and rewrites: substring / prefix / suffix tests, regex `match` / `test` / `sub` / `gsub` / `scan` / `capture` / `splits` (all flag-aware), plain `split` / `join`, casing, trims (`ltrimstr` / `rtrimstr`), conversions (`tonumber` / `tostring` / `tojson` / `fromjson` / `explode` / `implode` / `ascii` / `utf8bytelength`), and jq-style encodings (`uri` / `base64` / `base64d` / `html` / `sh`).

### `ascii`

Codepoint integer to its single-character string form.

**Signatures**

- `ascii(value: integer) -> string`

**Details**

Sugar for ``[value] | implode``: returns the single-character
string for a Unicode codepoint integer.  Useful inside arithmetic
pipelines that compute characters by codepoint offset.

Related: ``explode``, ``implode``.

**Examples**

```
ascii(65)                                # -> "A"
65 | ascii                               # same via implicit receiver
```

### `ascii_downcase`

ASCII-only lowercase — jq parity alias of ``downcase``.

**Signatures**

- `ascii_downcase(value: string) -> string`

**Details**

Matches jq's ``ascii_downcase``.  Identical to ``downcase`` in
this DSL.  Provided for jq compatibility.

Related: ``downcase``, ``ascii_upcase``.

**Examples**

```
ascii_downcase("VS_PROD")                # -> "vs_prod"
```

### `ascii_upcase`

ASCII-only uppercase — jq parity alias of ``upcase``.

**Signatures**

- `ascii_upcase(value: string) -> string`

**Details**

Matches jq's ``ascii_upcase``: returns *value* with every ASCII
letter (a-z) converted to uppercase, leaving non-ASCII letters
untouched.  Identical to ``upcase`` in this DSL — both are
ASCII-only.

Provided for jq compatibility so snippets paste through.

Related: ``upcase``, ``ascii_downcase``.

**Examples**

```
ascii_upcase("vs_prod")                  # -> "VS_PROD"
```

### `base64`

Base64-encode a string — equivalent of jq's ``@base64`` format string.

**Signatures**

- `base64(value: string) -> string`

**Details**

Matches jq's ``@base64``: Base64-encodes the UTF-8 bytes of
*value* and returns the standard-alphabet result.

Related: ``base64d``, ``uri``.

**Examples**

```
base64("hello")                          # -> "aGVsbG8="
```

### `base64d`

Base64-decode a string — equivalent of jq's ``@base64d`` format string.

**Signatures**

- `base64d(value: string) -> string`

**Details**

Matches jq's ``@base64d``: decodes a Base64-encoded ASCII string
into the original UTF-8 text.  Malformed input raises
``BuiltinError``.

Related: ``base64``.

**Examples**

```
base64d("aGVsbG8=")                      # -> "hello"
```

### `capture`

Named-group regex match — returns an object of capture names → captured text.

**Signatures**

- `capture(value: string, pattern: string) -> object`
- `capture(value: string, pattern: string, flags: string) -> object`

**Details**

Matches jq's ``capture``: runs *pattern* against *value* and
returns an object mapping each **named** capture group to its
matched text.  Use ``(?P<name>...)`` syntax for named groups (jq
uses ``(?<name>...)``; both forms are accepted by Python's ``re``
when ``(?P<name>...)`` is used, and ``capture`` rewrites jq-style
``(?<name>...)`` to the Python spelling so jq snippets paste
through).

Returns an empty object when the pattern has no named groups but
matches; raises ``BuiltinError`` when the pattern doesn't match
anywhere (jq parity — jq raises a no-match error too).

Related: ``match``, ``scan``, ``sub``, ``gsub``.

**Examples**

```
capture(.destination, "(?<addr>[0-9.]+):(?<port>[0-9]+)")
capture("vs_prod_web", "(?<env>prod|dev|qa)_(?<app>.+)")
```

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

### `explode`

String to list of Unicode codepoints.

**Signatures**

- `explode(value: string) -> list[integer]`

**Details**

Matches jq's ``explode``: returns the input string as a list of
integer codepoints.  Useful for character-level manipulation
(case folding by codepoint table, ROT-N ciphers, codepoint
arithmetic) before reassembling with ``implode``.

:class:`PathRef` is accepted and exploded as its ``full_path``.

Related: ``implode``, ``length``, ``split``.

**Examples**

```
explode("abc")                           # -> [97, 98, 99]
explode(.name) | length                  # codepoint count
```

### `fromjson`

Parse a JSON-encoded string into a value (jq's ``fromjson``).

**Signatures**

- `fromjson(value: string) -> any`

**Details**

Matches jq's ``fromjson``: parses *value* as JSON and returns
the resulting Python value.  Numbers become int / float, strings
quote, booleans pass through, ``null`` becomes Python ``None``,
arrays become lists, objects become dicts.

Raises ``BuiltinError`` on malformed input.

Related: ``tojson``, ``json_load``, ``json_parse``.

**Examples**

```
fromjson("42")                         # -> 42
fromjson("{\"a\": 1}")                  # -> {a: 1}
```

### `gsub`

Replace every regex match in a string.

**Signatures**

- `gsub(value: string, pattern: string, replacement: string) -> string`
- `gsub(value: string, pattern: string, replacement: string, flags: string) -> string`

**Details**

Like ``sub`` but replaces **every** occurrence of *pattern* in
*value*.  Useful for blanket string rewrites inside iRule bodies
or data-group values.

Accepts the same optional *flags* string ``sub`` / ``test`` /
``scan`` do: ``i`` / ``x`` / ``s`` / ``m``.

For object full-path renames, prefer ``rename`` or
``rename_partition`` over a raw ``gsub`` — those route through a
token-bounded engine that won't touch substring collisions or
short-name references in unsafe contexts.

Related: ``sub``, ``match``, ``rename``, ``rename_partition``.

**Examples**

```
gsub(.body, "/Common/old_", "/Common/new_")
gsub(.body, "old_", "new_", "i")          # case-insensitive
.ltm.virtual[].destination |= gsub(., "%5", "%7")  # bulk RD change
```

### `html`

HTML-escape a string — equivalent of jq's ``@html`` format string.

**Signatures**

- `html(value: string) -> string`

**Details**

Matches jq's ``@html``: replaces ``< > & ' "`` with their HTML
entity equivalents so the result is safe to embed in HTML text
or attribute values.

Related: ``uri``, ``sh``.

**Examples**

```
html("<a href=\"x\">&copy;</a>")
```

### `implode`

List of Unicode codepoints back to a string.

**Signatures**

- `implode(value: list[integer]) -> string`

**Details**

Matches jq's ``implode``: inverse of ``explode``.  Each item must
be a non-negative integer that names a valid Unicode codepoint.

Related: ``explode``, ``join``.

**Examples**

```
implode([97, 98, 99])                    # -> "abc"
explode(.name) | implode                  # round-trip identity
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

### `ltrimstr`

Strip *prefix* from the start of a string if present; otherwise return unchanged.

**Signatures**

- `ltrimstr(value: string, prefix: string) -> string`

**Details**

Matches jq's ``ltrimstr``: if *value* starts with *prefix*, drops
that prefix and returns the rest; otherwise returns *value*
unchanged.  Accepts :class:`PathRef` on either side (coerced
through ``full_path``).

Idiomatic for normalising names:
``.ltm.virtual[].name | ltrimstr("vs_")``.

Related: ``rtrimstr``, ``startswith``, ``sub``.

**Examples**

```
ltrimstr("vs_prod_web", "vs_")            # -> "prod_web"
ltrimstr("api_vs", "vs_")                 # -> "api_vs" (unchanged)
.ltm.virtual[].name | ltrimstr(., "vs_")
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

### `rtrimstr`

Strip *suffix* from the end of a string if present; otherwise return unchanged.

**Signatures**

- `rtrimstr(value: string, suffix: string) -> string`

**Details**

Matches jq's ``rtrimstr``: if *value* ends with *suffix*, drops
that suffix and returns the rest; otherwise returns *value*
unchanged.

Pairs with ``ltrimstr`` for symmetric stripping:
``.name | ltrimstr("vs_") | rtrimstr("_pool")``.

Related: ``ltrimstr``, ``endswith``, ``sub``.

**Examples**

```
rtrimstr("web_pool", "_pool")            # -> "web"
rtrimstr("web_pool", "_xxx")             # -> "web_pool" (unchanged)
```

### `scan`

Stream of every regex match in a string.

**Signatures**

- `scan(value: string, pattern: string) -> list`
- `scan(value: string, pattern: string, flags: string) -> list`

**Details**

Matches jq's ``scan``: walks *value* finding every non-overlapping
match of *pattern* and returns them as a list.

- When *pattern* has **no capture groups**, each element is the
  matched substring.
- When *pattern* has **one or more capture groups**, each element
  is a list of capture values (matching jq's array-per-match
  shape; the full match is **not** included).

Empty matches at advancing positions are skipped to avoid infinite
loops — Python's ``finditer`` already does this.

Related: ``match`` / ``test`` (predicate), ``capture`` (named
groups), ``splits``.

**Examples**

```
scan("a1 b22 c333", "[0-9]+")            # -> ["1", "22", "333"]
scan("a=1 b=2 c=3", "([a-z])=([0-9])")   # -> [["a","1"], ["b","2"], ["c","3"]]
```

### `sh`

POSIX-shell-quote a string or list of strings — jq's ``@sh``.

**Signatures**

- `sh(value: string) -> string`
- `sh(value: list[string]) -> string`

**Details**

Matches jq's ``@sh``: returns a representation safe to interpolate
into a POSIX shell command.  **Every** value is wrapped in single
quotes (with embedded ``'`` escaped as ``'\''``) — jq parity, and
cheaper to reason about than Python's ``shlex.quote`` which leaves
"obviously safe" tokens unquoted.  Lists become space-separated
single-quoted fields.

Related: ``uri``, ``base64``, ``join``.

**Examples**

```
sh("hello world")                       # -> "'hello world'"
sh(["a", "b c"])                       # -> "'a' 'b c'"
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

### `splits`

Regex-based split — returns the substrings between matches.

**Signatures**

- `splits(value: string, pattern: string) -> list[string]`
- `splits(value: string, pattern: string, flags: string) -> list[string]`

**Details**

Matches jq's ``splits``: splits *value* on every (possibly empty)
match of *pattern* and returns the substrings between matches as
a list.

Unlike ``split``, which takes a literal separator, ``splits``
interprets its second argument as a regex.  Useful when the
separator is irregular: variable whitespace, optional punctuation,
multi-character alternatives.

Related: ``split`` (literal), ``scan``, ``join``.

**Examples**

```
splits("a, b ,c,  d", " *, *")           # -> ["a", "b", "c", "d"]
splits("v1.2.3-rc4", "[.-]")             # -> ["v1", "2", "3", "rc4"]
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
- `sub(value: string, pattern: string, replacement: string, flags: string) -> string`

**Details**

Replaces the **first** occurrence of *pattern* in *value* with
*replacement* and returns the new string.  *pattern* is a Python
regex; *replacement* may use ``\1`` / ``\g<name>`` backrefs.

Optional *flags* string takes the same letters as ``test`` /
``scan`` / ``capture`` / ``splits``: ``i`` for case-insensitive,
``x`` for free-spacing, ``s`` for dot-matches-newline, ``m`` for
multi-line.

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
sub(.name, "^VS_", "vs_", "i")           # case-insensitive
.ltm.virtual[].destination |= sub(., ":443$", ":8443")
```

### `test`

Regex test — true when the pattern matches anywhere in the string (jq's ``test``).

**Signatures**

- `test(value: string, pattern: string) -> boolean`
- `test(value: string, pattern: string, flags: string) -> boolean`

**Details**

Matches jq's ``test``: a Boolean predicate that returns ``true``
when *pattern* matches anywhere in *value*.  Same engine as
``match`` / ``sub`` / ``gsub`` — see those for the trust-boundary
notes (length cap, refusal of catastrophic-backtracking shapes).

Flags string is a subset of jq's: ``i`` for case-insensitive,
``x`` for free-spacing, ``s`` for dot-matches-newline, ``m`` for
multi-line.  Unknown flags raise.

This is the jq name; ``match`` is the legacy DSL name and remains
available as the same Boolean predicate.  Pick whichever reads
more naturally.

Related: ``match``, ``scan``, ``capture``, ``sub``, ``gsub``.

**Examples**

```
test(.name, "^vs_")
test(.name, "^VS_", "i")                 # case-insensitive
```

### `tojson`

Encode the current value as a JSON string (jq's ``tojson`` / ``@json``).

**Signatures**

- `tojson(value: any) -> string`

**Details**

Matches jq's ``tojson`` and the ``@json`` format string: returns
a JSON encoding of *value*.  Numbers, booleans, and ``null``
serialise as JSON literals; strings and PathRef values quote;
lists and dicts recurse.

Identical in output to ``tostring`` for aggregates, but always
returns valid JSON (``tostring`` of a string returns the string
unchanged — ``tojson`` quotes it).

Related: ``fromjson``, ``tostring``, ``str``.

**Examples**

```
tojson(42)                               # -> "42"
tojson("hi")                            # -> '"hi"'
tojson({a: 1, b: [2, 3]})                # -> '{"a":1,"b":[2,3]}'
```

### `tonumber`

Parse a string as a number; return numbers unchanged.

**Signatures**

- `tonumber(value: string | number) -> number`

**Details**

Matches jq's ``tonumber``: numeric input passes through; a string
is parsed as an integer when possible, otherwise as a float.
Leading / trailing whitespace is tolerated.

Booleans are rejected — they are not numbers in jq.  ``null`` and
non-numeric strings raise ``BuiltinError`` (jq raises too).

Related: ``tostring``, ``str``, ``floor``, ``ceil``.

**Examples**

```
tonumber("42")                           # -> 42
tonumber("3.14")                         # -> 3.14
tonumber(.ltm.pool[].monitor.interval)
```

### `tostring`

Convert any value to its string form — jq parity alias of ``str``.

**Signatures**

- `tostring(value: any) -> string`

**Details**

Matches jq's ``tostring``:

- **string** / :class:`PathRef`: returned as-is (PathRef →
  ``full_path``).
- **integers** and **floats**: decimal form.
- **booleans**: ``"true"`` / ``"false"``.
- **null**: ``"null"``.
- **lists** / **objects**: JSON-style encoding (jq parity —
  objects emit as JSON, not as TMSH stanzas).

Use ``str`` for the scalar-only form that refuses to stringify
aggregates; use ``tostring`` when you want the round-trip JSON
spelling.

Related: ``str``, ``tonumber``, ``join``.

**Examples**

```
tostring(42)                             # -> "42"
tostring([1, 2, 3])                      # -> "[1,2,3]"
tostring({a: 1})                         # -> "{\"a\":1}"
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

### `uri`

URL-encode a string — equivalent of jq's ``@uri`` format string.

**Signatures**

- `uri(value: string) -> string`

**Details**

Matches jq's ``@uri``: percent-encodes characters that need
escaping in URL components (everything outside the unreserved
set ``A-Z a-z 0-9 - _ . ~``).

Related: ``base64``, ``html``, ``sh``.

**Examples**

```
uri("hello world")                       # -> "hello%20world"
uri("/Common/web_vs?x=1&y=2")
```

### `utf8bytelength`

Number of UTF-8 bytes the string encodes to.

**Signatures**

- `utf8bytelength(value: string) -> integer`

**Details**

Matches jq's ``utf8bytelength``: returns the byte count of
*value*'s UTF-8 encoding.  Differs from ``length`` (which
counts codepoints) whenever the string contains non-ASCII.

Related: ``length``, ``explode``.

**Examples**

```
utf8bytelength("hello")                  # -> 5
utf8bytelength("héllo")                  # -> 6  (é is 2 bytes)
```

## math

Numeric helpers matching jq's C-math surface: rounding (`floor` / `ceil` / `round` / `trunc` / `rint`), magnitude / sign (`abs` / `fabs` / `copysign` / `fdim`), powers / roots (`sqrt` / `cbrt` / `pow` / `exp` / `exp2` / `exp10`), logarithms (`log` / `log2` / `log10` / `logb`), trigonometry (`sin` / `cos` / `tan` / `asin` / `acos` / `atan` / `atan2`), hyperbolics (`sinh` / `cosh` / `tanh` / `asinh` / `acosh` / `atanh`), special functions (`gamma` / `tgamma` / `lgamma`, Bessel `j0` / `j1` / `y0` / `y1`), bit-level decomposition (`frexp` / `ldexp` / `modf` / `significand`), and IEEE-754 sentinels (`nan` / `infinite` / `isnan` / `isinfinite` / `isnormal`).

### `abs`

Absolute value of a number.

**Signatures**

- `abs(value: number) -> number`

**Details**

Matches jq 1.7+'s ``abs``: returns the magnitude of a number.
Integer in → integer out; float in → float out.

Related: ``fabs`` (always float), ``floor``, ``ceil``.

**Examples**

```
abs(-5)                                  # -> 5
abs(3.14)                                # -> 3.14
```

### `acos`

Inverse cosine in radians. Matches jq's namesake C-math function.

**Signatures**

- `acos(value: number) -> number`

**Details**

Matches jq's ``acos``: thin wrapper over Python's
``math.acos``.  Domain errors (``acos(2)`` etc.)
raise ``BuiltinError`` rather than returning NaN so the
failure shows in query output.

**Examples**

```
acos(0)
acos(.angle)
```

### `acosh`

Inverse hyperbolic cosine. Matches jq's namesake C-math function.

**Signatures**

- `acosh(value: number) -> number`

**Details**

Matches jq's ``acosh``: thin wrapper over Python's
``math.acosh``.  Domain errors (``acos(2)`` etc.)
raise ``BuiltinError`` rather than returning NaN so the
failure shows in query output.

**Examples**

```
acosh(0)
acosh(.angle)
```

### `asin`

Inverse sine in radians. Matches jq's namesake C-math function.

**Signatures**

- `asin(value: number) -> number`

**Details**

Matches jq's ``asin``: thin wrapper over Python's
``math.asin``.  Domain errors (``acos(2)`` etc.)
raise ``BuiltinError`` rather than returning NaN so the
failure shows in query output.

**Examples**

```
asin(0)
asin(.angle)
```

### `asinh`

Inverse hyperbolic sine. Matches jq's namesake C-math function.

**Signatures**

- `asinh(value: number) -> number`

**Details**

Matches jq's ``asinh``: thin wrapper over Python's
``math.asinh``.  Domain errors (``acos(2)`` etc.)
raise ``BuiltinError`` rather than returning NaN so the
failure shows in query output.

**Examples**

```
asinh(0)
asinh(.angle)
```

### `atan`

Inverse tangent in radians. Matches jq's namesake C-math function.

**Signatures**

- `atan(value: number) -> number`

**Details**

Matches jq's ``atan``: thin wrapper over Python's
``math.atan``.  Domain errors (``acos(2)`` etc.)
raise ``BuiltinError`` rather than returning NaN so the
failure shows in query output.

**Examples**

```
atan(0)
atan(.angle)
```

### `atan2`

Two-argument inverse tangent — ``atan2(y, x)``.

**Signatures**

- `atan2(y: number, x: number) -> number`

**Details**

Matches jq's ``atan2``: returns the angle in radians between the
positive x-axis and the point (x, y), with the correct quadrant.

Related: ``atan``, ``sin``, ``cos``.

**Examples**

```
atan2(1, 1)                              # -> pi/4
```

### `atanh`

Inverse hyperbolic tangent. Matches jq's namesake C-math function.

**Signatures**

- `atanh(value: number) -> number`

**Details**

Matches jq's ``atanh``: thin wrapper over Python's
``math.atanh``.  Domain errors (``acos(2)`` etc.)
raise ``BuiltinError`` rather than returning NaN so the
failure shows in query output.

**Examples**

```
atanh(0)
atanh(.angle)
```

### `cbrt`

Cube root of a number.

**Signatures**

- `cbrt(value: number) -> number`

**Details**

Matches jq's ``cbrt``: returns the real cube root of *value*.
Handles negative inputs (``cbrt(-8) == -2``) — distinct from
``pow(., 1.0/3)`` which is undefined for negative bases.

Related: ``sqrt``, ``pow``.

**Examples**

```
cbrt(27)                                 # -> 3.0
```

### `ceil`

Round a number up to the nearest integer.

**Signatures**

- `ceil(value: number) -> integer`

**Details**

Matches jq's ``ceil``: returns the smallest integer ``>= value``.
Integers pass through unchanged.

Related: ``floor``, ``round``.

**Examples**

```
ceil(3.2)                                # -> 4
ceil(-3.7)                               # -> -3
```

### `copysign`

Magnitude of *x* with the sign of *y*.

**Signatures**

- `copysign(x: number, y: number) -> number`

**Details**

Matches jq's ``copysign``: ``copysign(x; y)`` returns a value with
the magnitude of ``x`` and the sign of ``y``.

Related: ``abs``, ``fabs``.

**Examples**

```
copysign(3, -1)                          # -> -3.0
```

### `cos`

Cosine of a radian angle. Matches jq's namesake C-math function.

**Signatures**

- `cos(value: number) -> number`

**Details**

Matches jq's ``cos``: thin wrapper over Python's
``math.cos``.  Domain errors (``acos(2)`` etc.)
raise ``BuiltinError`` rather than returning NaN so the
failure shows in query output.

**Examples**

```
cos(0)
cos(.angle)
```

### `cosh`

Hyperbolic cosine. Matches jq's namesake C-math function.

**Signatures**

- `cosh(value: number) -> number`

**Details**

Matches jq's ``cosh``: thin wrapper over Python's
``math.cosh``.  Domain errors (``acos(2)`` etc.)
raise ``BuiltinError`` rather than returning NaN so the
failure shows in query output.

**Examples**

```
cosh(0)
cosh(.angle)
```

### `drem`

Alias of ``remainder`` (legacy BSD name).

**Signatures**

- `drem(x: number, y: number) -> number`

**Details**

Matches jq's ``drem``: alias of ``remainder``.

Related: ``remainder``, ``fmod``.

**Examples**

```
drem(7, 3)                               # -> 1.0
```

### `exp`

``e`` raised to the *value* power.

**Signatures**

- `exp(value: number) -> number`

**Details**

Matches jq's ``exp``: returns ``e^value``.

Related: ``log``, ``pow``.

**Examples**

```
exp(1)                                   # -> 2.718...
```

### `exp10`

``10`` raised to the *value* power.

**Signatures**

- `exp10(value: number) -> number`

**Details**

Matches jq's ``exp10``: returns ``10^value``.

Related: ``exp``, ``exp2``, ``pow``, ``log10``.

**Examples**

```
exp10(3)                                 # -> 1000.0
```

### `exp2`

``2`` raised to the *value* power.

**Signatures**

- `exp2(value: number) -> number`

**Details**

Matches jq's ``exp2``: returns ``2^value``.

Related: ``exp``, ``exp10``, ``pow``, ``log2``.

**Examples**

```
exp2(10)                                 # -> 1024.0
```

### `expm1`

``exp(value) - 1`` with high precision near zero.

**Signatures**

- `expm1(value: number) -> number`

**Details**

Matches jq's ``expm1``: thin wrapper over Python's ``math.expm1``.
Avoids the loss of precision that ``exp(x) - 1`` suffers when
``x`` is small.

Related: ``exp``, ``log1p``.

**Examples**

```
expm1(1e-10)                             # -> 1e-10 (high precision)
```

### `fabs`

Absolute value as a float.

**Signatures**

- `fabs(value: number) -> number`

**Details**

Matches jq's ``fabs``: ``math.fabs`` from C — like ``abs`` but
always returns a float, even for integer input.

Related: ``abs``, ``floor``.

**Examples**

```
fabs(-5)                                 # -> 5.0
```

### `fdim`

Positive difference — ``max(x - y, 0)``.

**Signatures**

- `fdim(x: number, y: number) -> number`

**Details**

Matches jq's ``fdim``: returns ``max(x - y, 0)``.

Related: ``min``, ``max``, ``abs``.

**Examples**

```
fdim(5, 3)                               # -> 2.0
```

### `floor`

Round a number down to the nearest integer.

**Signatures**

- `floor(value: number) -> integer`

**Details**

Matches jq's ``floor``: returns the largest integer ``<= value``.
Integers pass through unchanged.

Related: ``ceil``, ``round``, ``abs``.

**Examples**

```
floor(3.7)                               # -> 3
floor(-3.2)                              # -> -4
```

### `fma`

Fused multiply-add — ``x * y + z`` in one rounding.

**Signatures**

- `fma(x: number, y: number, z: number) -> number`

**Details**

Matches jq's ``fma``: returns ``x*y + z``.  Python doesn't expose
hardware FMA in older versions; this implementation computes the
naive expression, which agrees with FMA to within one ULP.

Related: ``pow``, ``hypot``.

**Examples**

```
fma(2, 3, 1)                             # -> 7.0
```

### `fmax`

Larger of two numbers — ``max(x, y)`` (jq parity).

**Signatures**

- `fmax(x: number, y: number) -> number`

**Details**

Matches jq's ``fmax``: two-argument numeric maximum.  Unlike
``max`` (which is stream-aware), ``fmax`` takes exactly two
scalar numbers.

Related: ``max``, ``fmin``, ``fdim``.

**Examples**

```
fmax(3, 7)                               # -> 7
```

### `fmin`

Smaller of two numbers — ``min(x, y)`` (jq parity).

**Signatures**

- `fmin(x: number, y: number) -> number`

**Details**

Matches jq's ``fmin``: two-argument numeric minimum.

Related: ``min``, ``fmax``.

**Examples**

```
fmin(3, 7)                               # -> 3
```

### `fmod`

Floating-point modulo — ``x - y * trunc(x / y)``.

**Signatures**

- `fmod(x: number, y: number) -> number`

**Details**

Matches jq's ``fmod`` (C's ``fmod``): the floating-point remainder
truncated toward zero.  Result has the sign of *x*.

Related: ``remainder``, ``drem``, ``trunc``.

**Examples**

```
fmod(7, 3)                               # -> 1.0
```

### `frexp`

Decompose a number into mantissa and exponent — returns ``[m, e]``.

**Signatures**

- `frexp(value: number) -> list`

**Details**

Matches jq's ``frexp``: returns ``[mantissa, exponent]`` such
that ``value == mantissa * 2**exponent`` and ``0.5 <= |mantissa|
< 1`` (or both parts zero when ``value`` is zero).

jq returns a 2-element array; this DSL returns a Python list of
the same shape.

Related: ``ldexp``, ``modf``, ``logb``.

**Examples**

```
frexp(12)                                # -> [0.75, 4]
```

### `gamma`

Gamma function — ``tgamma`` (jq alias).

**Signatures**

- `gamma(value: number) -> number`

**Details**

Matches jq's ``gamma`` / ``tgamma``: returns the gamma function
of *value*.  Domain errors raise ``BuiltinError``.

Related: ``lgamma``, ``tgamma``.

**Examples**

```
gamma(5)                                 # -> 24.0  (4!)
```

### `hypot`

``sqrt(x*x + y*y)`` without intermediate overflow.

**Signatures**

- `hypot(x: number, y: number) -> number`

**Details**

Matches jq's ``hypot``: the Euclidean distance.  Uses Python's
``math.hypot``, which avoids overflow for large operands.

Related: ``sqrt``, ``pow``.

**Examples**

```
hypot(3, 4)                              # -> 5.0
```

### `infinite`

Positive infinity floating-point value.

**Signatures**

- `infinite() -> number`

**Details**

Matches jq's ``infinite``: returns positive ``inf``.  Negate with
the unary ``-`` operator for negative infinity.

Related: ``nan``, ``isinfinite``.

**Examples**

```
infinite                                 # -> Infinity
```

### `isinfinite`

True when the value is positive or negative infinity.

**Signatures**

- `isinfinite(value: number) -> boolean`

**Details**

Matches jq's ``isinfinite``.  Returns ``false`` for non-number
input.

Related: ``infinite``, ``isnan``, ``isnormal``.

**Examples**

```
isinfinite(infinite)                     # -> true
isinfinite(1.0)                          # -> false
```

### `isnan`

True when the value is the IEEE-754 NaN.

**Signatures**

- `isnan(value: number) -> boolean`

**Details**

Matches jq's ``isnan``.  Returns ``false`` for non-number input
(which differs from jq's "is the input not a number" interpretation
— that's a rarely-useful question, and a misuse on non-numbers is
almost always a typo).

Related: ``nan``, ``isinfinite``, ``isnormal``.

**Examples**

```
isnan(nan)                               # -> true
isnan(1.0)                               # -> false
```

### `isnormal`

True when the value is a finite, non-zero, non-subnormal number.

**Signatures**

- `isnormal(value: number) -> boolean`

**Details**

Matches jq's ``isnormal``: true when the value is a finite,
non-zero, non-subnormal number.  Integers count as normal when
they are non-zero.

Returns ``false`` for non-number input.

Related: ``isnan``, ``isinfinite``.

**Examples**

```
isnormal(1.0)                            # -> true
isnormal(0)                              # -> false
isnormal(nan)                            # -> false
```

### `j0`

Bessel function of the first kind, order 0 — series approximation.

**Signatures**

- `j0(value: number) -> number`

**Details**

Matches jq's ``j0``: returns ``J_0(value)``, the Bessel function
of the first kind at order 0.  Implemented via a polynomial /
asymptotic approximation (Abramowitz & Stegun 9.4.1 / 9.2.5)
accurate to ~1e-7 — adequate for the ad-hoc audit queries this
DSL targets.  For research-grade precision use SciPy.

Related: ``j1``, ``y0``, ``y1``.

**Examples**

```
j0(0)                                    # -> 1.0
```

### `j1`

Bessel function of the first kind, order 1.

**Signatures**

- `j1(value: number) -> number`

**Details**

Matches jq's ``j1``: ``J_1(value)``, accuracy ~1e-7.  See
``j0`` for the approximation notes.

Related: ``j0``, ``y0``, ``y1``.

**Examples**

```
j1(0)                                    # -> 0.0
```

### `jn`

Bessel J_n(x) by upward recurrence — accurate for small ``n``.

**Signatures**

- `jn(n: integer, x: number) -> number`

**Details**

Matches jq's ``jn``: ``J_n(x)`` for integer order *n*.  Uses the
standard upward recurrence ``J_{n+1}(x) = (2n/x) J_n(x) -
J_{n-1}(x)`` seeded by ``j0`` / ``j1``.  Stable for ``n <= |x|``;
callers asking for ``n`` far above ``|x|`` may see precision loss
— for research-grade results use SciPy.

Related: ``j0``, ``j1``, ``yn``.

**Examples**

```
jn(2, 5)
```

### `ldexp`

``mantissa * 2**exponent`` — inverse of ``frexp``.

**Signatures**

- `ldexp(mantissa: number, exponent: integer) -> number`

**Details**

Matches jq's ``ldexp``: returns ``mantissa * 2**exponent``.

Related: ``frexp``, ``exp2``.

**Examples**

```
ldexp(0.75, 4)                           # -> 12.0
```

### `lgamma`

Natural log of the absolute value of the gamma function.

**Signatures**

- `lgamma(value: number) -> number`

**Details**

Matches jq's ``lgamma`` / ``lgamma_r``: returns ``log(|gamma(x)|)``
— useful for combinatoric calculations where the gamma value
would overflow.

Related: ``gamma``, ``log``.

**Examples**

```
lgamma(5)                                # -> log(24)
```

### `lgamma_r`

lgamma with the sign of gamma — returns ``[lgamma, sign]``.

**Signatures**

- `lgamma_r(value: number) -> list`

**Details**

Matches jq's ``lgamma_r``: returns a 2-element list
``[log(|gamma(x)|), sign]`` where *sign* is +1 or -1.

Related: ``lgamma``, ``gamma``.

**Examples**

```
lgamma_r(5)                              # -> [log(24), 1]
```

### `log`

Natural logarithm (base e) of a positive number.

**Signatures**

- `log(value: number) -> number`

**Details**

Matches jq's ``log``: returns ``ln(value)``.  Non-positive input
raises ``BuiltinError``.

Related: ``log10``, ``log2``, ``exp``.

**Examples**

```
log(2.71828)                             # -> ~1.0
```

### `log10`

Base-10 logarithm of a positive number.

**Signatures**

- `log10(value: number) -> number`

**Details**

Matches jq's ``log10``.  Non-positive input raises.

Related: ``log``, ``log2``.

**Examples**

```
log10(1000)                              # -> 3.0
```

### `log1p`

``log(1 + value)`` with high precision near zero.

**Signatures**

- `log1p(value: number) -> number`

**Details**

Matches jq's ``log1p``: thin wrapper over Python's ``math.log1p``.
Avoids the loss of precision that ``log(1 + x)`` suffers when
``x`` is small.

Related: ``log``, ``expm1``.

**Examples**

```
log1p(1e-10)                             # -> 1e-10 (high precision)
```

### `log2`

Base-2 logarithm of a positive number.

**Signatures**

- `log2(value: number) -> number`

**Details**

Matches jq's ``log2``.  Non-positive input raises.

Related: ``log``, ``log10``.

**Examples**

```
log2(1024)                               # -> 10.0
```

### `logb`

Integer binary exponent of *value* (``floor(log2(|value|))``).

**Signatures**

- `logb(value: number) -> number`

**Details**

Matches jq's ``logb`` / C's ``logb``: returns the exponent of
*value*'s base-2 representation as a floating-point number.  For
a non-zero finite *value* this is the integer part of
``log2(|value|)``.

Returns ``-inf`` for zero and ``+inf`` for infinity (jq parity).

Related: ``log2``, ``frexp``.

**Examples**

```
logb(8)                                  # -> 3.0
logb(0.25)                               # -> -2.0
```

### `modf`

Split a number into fractional and integer parts — returns ``[frac, int]``.

**Signatures**

- `modf(value: number) -> list`

**Details**

Matches jq's ``modf``: returns ``[frac, int]`` where ``frac`` is
the fractional part of *value* and ``int`` is the integer part,
both with the same sign as *value*.

Related: ``trunc``, ``floor``.

**Examples**

```
modf(3.75)                               # -> [0.75, 3.0]
```

### `nan`

The not-a-number floating-point value.

**Signatures**

- `nan() -> number`

**Details**

Matches jq's ``nan``: returns the IEEE-754 NaN value.  Useful as
a sentinel when arithmetic over partial data needs to propagate
"not measured" through pipelines.

Related: ``infinite``, ``isnan``.

**Examples**

```
nan                                      # -> NaN
```

### `nearbyint`

Round to nearest integer using current rounding mode (banker's).

**Signatures**

- `nearbyint(value: number) -> integer`

**Details**

Matches jq's ``nearbyint``: same result as ``rint`` (Python's
``round`` uses banker's rounding).  C distinguishes them by
whether they raise the inexact flag; Python doesn't expose
that, so the two are aliases here.

Related: ``rint``, ``round``, ``trunc``.

**Examples**

```
nearbyint(2.5)                           # -> 2
```

### `pow`

Raise *base* to the *exponent* power.

**Signatures**

- `pow(base: number, exponent: number) -> number`

**Details**

Matches jq's ``pow``: ``pow(x; y)`` = x^y.  Returns a float.

Related: ``sqrt``, ``exp``, ``log``.

**Examples**

```
pow(2, 10)                               # -> 1024.0
pow(2, 0.5)                              # -> sqrt(2)
```

### `pow10`

Alias of ``exp10`` — ``10 ** value``.

**Signatures**

- `pow10(value: number) -> number`

**Details**

Matches jq's ``pow10``: ``10**value``.  Identical to ``exp10``.

Related: ``exp10``, ``pow``, ``log10``.

**Examples**

```
pow10(3)                                 # -> 1000.0
```

### `remainder`

IEEE-754 remainder of ``x / y`` (rounded to nearest).

**Signatures**

- `remainder(x: number, y: number) -> number`

**Details**

Matches jq's ``remainder`` / ``drem``: the IEEE-754 remainder,
which rounds the quotient to the nearest integer (ties to even)
rather than truncating.  Result is in ``[-y/2, y/2]``.

Related: ``fmod``, ``drem``.

**Examples**

```
remainder(7, 3)                          # -> 1.0
```

### `rint`

Round to the nearest integer, ties to even (banker's rounding).

**Signatures**

- `rint(value: number) -> integer`

**Details**

Matches jq's ``rint``: rounds to the nearest integer with
ties-to-even semantics — distinct from ``round`` which uses
ties-away-from-zero.

Related: ``round``, ``floor``, ``ceil``.

**Examples**

```
rint(2.5)                                # -> 2  (banker's)
rint(3.5)                                # -> 4
```

### `round`

Round a number to the nearest integer (ties away from zero — jq parity).

**Signatures**

- `round(value: number) -> integer`

**Details**

Matches jq's ``round`` (which calls C's ``round``, rounding ties
away from zero — **not** Python's banker's rounding).
``round(0.5)`` → 1, ``round(-0.5)`` → -1, ``round(2.5)`` → 3.

Related: ``floor``, ``ceil``, ``abs``.

**Examples**

```
round(2.5)                               # -> 3
round(-2.5)                              # -> -3
round(2.49)                              # -> 2
```

### `significand`

Significand (mantissa with exponent normalised to zero).

**Signatures**

- `significand(value: number) -> number`

**Details**

Matches jq's ``significand``: returns the value scaled to the
range ``[1, 2)`` (or ``[-2, -1]`` for negatives), i.e. divided by
``2 ** logb(value)``.  Returns the input unchanged for zero,
infinities, and NaN.

Related: ``frexp``, ``logb``.

**Examples**

```
significand(12)                          # -> 1.5
```

### `sin`

Sine of a radian angle. Matches jq's namesake C-math function.

**Signatures**

- `sin(value: number) -> number`

**Details**

Matches jq's ``sin``: thin wrapper over Python's
``math.sin``.  Domain errors (``acos(2)`` etc.)
raise ``BuiltinError`` rather than returning NaN so the
failure shows in query output.

**Examples**

```
sin(0)
sin(.angle)
```

### `sinh`

Hyperbolic sine. Matches jq's namesake C-math function.

**Signatures**

- `sinh(value: number) -> number`

**Details**

Matches jq's ``sinh``: thin wrapper over Python's
``math.sinh``.  Domain errors (``acos(2)`` etc.)
raise ``BuiltinError`` rather than returning NaN so the
failure shows in query output.

**Examples**

```
sinh(0)
sinh(.angle)
```

### `sqrt`

Square root of a non-negative number.

**Signatures**

- `sqrt(value: number) -> number`

**Details**

Matches jq's ``sqrt``: returns ``math.sqrt(value)``.  Negative
input raises ``BuiltinError`` (jq returns NaN; we prefer a
visible error).

Related: ``pow``, ``exp``.

**Examples**

```
sqrt(16)                                 # -> 4.0
```

### `tan`

Tangent of a radian angle. Matches jq's namesake C-math function.

**Signatures**

- `tan(value: number) -> number`

**Details**

Matches jq's ``tan``: thin wrapper over Python's
``math.tan``.  Domain errors (``acos(2)`` etc.)
raise ``BuiltinError`` rather than returning NaN so the
failure shows in query output.

**Examples**

```
tan(0)
tan(.angle)
```

### `tanh`

Hyperbolic tangent. Matches jq's namesake C-math function.

**Signatures**

- `tanh(value: number) -> number`

**Details**

Matches jq's ``tanh``: thin wrapper over Python's
``math.tanh``.  Domain errors (``acos(2)`` etc.)
raise ``BuiltinError`` rather than returning NaN so the
failure shows in query output.

**Examples**

```
tanh(0)
tanh(.angle)
```

### `tgamma`

Gamma function (alias of ``gamma``).

**Signatures**

- `tgamma(value: number) -> number`

**Details**

Matches jq's ``tgamma`` (C's ``tgamma``).  Same result as
``gamma``.

Related: ``gamma``, ``lgamma``.

**Examples**

```
tgamma(5)                                # -> 24.0
```

### `trunc`

Truncate toward zero — drop the fractional part.

**Signatures**

- `trunc(value: number) -> integer`

**Details**

Matches jq's ``trunc``: returns *value* with its fractional part
removed, rounding toward zero.

Related: ``floor``, ``ceil``, ``round``.

**Examples**

```
trunc(3.9)                               # -> 3
trunc(-3.9)                              # -> -3
```

### `y0`

Bessel function of the second kind, order 0.

**Signatures**

- `y0(value: number) -> number`

**Details**

Matches jq's ``y0``: ``Y_0(value)``, defined for ``value > 0``.
Series / asymptotic approximation accurate to ~1e-7.

Related: ``y1``, ``j0``, ``j1``.

**Examples**

```
y0(1)
```

### `y1`

Bessel function of the second kind, order 1.

**Signatures**

- `y1(value: number) -> number`

**Details**

Matches jq's ``y1``: ``Y_1(value)``, defined for ``value > 0``.
Series / asymptotic approximation accurate to ~1e-7.

Related: ``y0``, ``j0``, ``j1``.

**Examples**

```
y1(1)
```

### `yn`

Bessel Y_n(x) by upward recurrence (``x > 0``).

**Signatures**

- `yn(n: integer, x: number) -> number`

**Details**

Matches jq's ``yn``: ``Y_n(x)`` for integer order *n*, defined
for positive *x*.  Upward recurrence seeded by ``y0`` / ``y1``;
upward recurrence is stable for Y_n at all orders.

Related: ``y0``, ``y1``, ``jn``.

**Examples**

```
yn(2, 5)
```

## time

Time and date helpers matching jq's surface: epoch reads (`now`), ISO-8601 conversions (`todate` / `todateiso8601` / `fromdate` / `fromdateiso8601` / `date`), broken-down time (`gmtime` / `localtime` / `mktime`), formatting / parsing (`strftime` / `strptime`), and epoch arithmetic (`dateadd` / `datesub`).

### `date`

Alias of ``todate`` for jq snippet compatibility.

**Signatures**

- `date(value: number) -> string`

**Details**

Matches jq's ``date`` (jq 1.7+): same as ``todate``.

Related: ``todate``, ``fromdate``.

**Examples**

```
date(0)
```

### `dateadd`

Add a number of seconds to a Unix epoch value.

**Signatures**

- `dateadd(value: number, seconds: number) -> number`

**Details**

Matches jq's ``dateadd`` (jq 1.7+): ``dateadd(t; s)`` is
``t + s``.  Provided for jq-snippet portability — plain
arithmetic works too.

Related: ``datesub``, ``now``.

**Examples**

```
fromdate("2025-01-01T00:00:00Z") | dateadd(., 86400) | todate
```

### `datesub`

Subtract a number of seconds from a Unix epoch value.

**Signatures**

- `datesub(value: number, seconds: number) -> number`

**Details**

Matches jq's ``datesub`` (jq 1.7+): ``datesub(t; s)`` is
``t - s``.

Related: ``dateadd``, ``now``.

**Examples**

```
now | datesub(., 3600) | todate           # one hour ago
```

### `fromdate`

ISO-8601 UTC string → Unix epoch seconds.

**Signatures**

- `fromdate(value: string) -> number`

**Details**

Matches jq's ``fromdate`` / ``fromdateiso8601``: parses an
ISO-8601 UTC timestamp (``YYYY-MM-DDTHH:MM:SS[.fff]Z`` or with
``+00:00`` offset) and returns the corresponding Unix epoch
seconds.

Related: ``todate``, ``strptime``, ``now``.

**Examples**

```
fromdate("1970-01-01T00:00:00Z")          # -> 0
fromdate("2025-01-01T00:00:00Z")
```

### `fromdateiso8601`

Alias of ``fromdate``.

**Signatures**

- `fromdateiso8601(value: string) -> number`

**Details**

Matches jq's ``fromdateiso8601``.

Related: ``fromdate``, ``todateiso8601``.

**Examples**

```
fromdateiso8601("1970-01-01T00:00:00Z")
```

### `gmtime`

Unix epoch seconds → broken-down UTC time array.

**Signatures**

- `gmtime(value: number) -> list`

**Details**

Matches jq's ``gmtime``: returns a length-8 list of integers in
jq's order — ``[year - 1900, month, day, hour, minute, second,
weekday, yearday]`` (months 0..11, weekday 0=Sunday, yearday
0..365).

Related: ``localtime``, ``mktime``, ``strftime``.

**Examples**

```
gmtime(0)                                # -> [70, 0, 1, 0, 0, 0, 4, 0]
```

### `localtime`

Unix epoch seconds → broken-down local time array.

**Signatures**

- `localtime(value: number) -> list`

**Details**

Matches jq's ``localtime``: same shape as ``gmtime`` but in the
process's local timezone.

Related: ``gmtime``, ``mktime``.

**Examples**

```
localtime(0)
```

### `mktime`

Broken-down UTC time array → Unix epoch seconds.

**Signatures**

- `mktime(value: list) -> number`

**Details**

Matches jq's ``mktime``: inverse of ``gmtime``.  Accepts the
broken-down array ``[year - 1900, month, day, hour, minute,
second, ...]`` and returns the corresponding Unix epoch seconds,
interpreting the input as UTC.

Related: ``gmtime``, ``fromdate``.

**Examples**

```
mktime([70, 0, 1, 0, 0, 0])              # -> 0
```

### `now`

Current Unix epoch time as a float.

**Signatures**

- `now() -> number`

**Details**

Matches jq's ``now``: returns the current time in seconds since
the Unix epoch (UTC) as a float with sub-second resolution.

Related: ``todate``, ``fromdate``, ``strftime``.

**Examples**

```
now
```

### `strftime`

Format a Unix epoch-seconds value using ``strftime``.

**Signatures**

- `strftime(value: number, fmt: string) -> string`

**Details**

Matches jq's ``strftime``: formats a Unix epoch-seconds value
(UTC) using a strftime-style format string.

jq's two-arg form is ``strftime(fmt)`` applied to the broken-down
time as input; this DSL flattens to the more common case
``strftime(unix_seconds, fmt)``.  Use the implicit-receiver form
for the jq feel: ``now | strftime("%Y-%m-%d")``.

Related: ``strptime``, ``todate``, ``now``.

**Examples**

```
strftime(0, "%Y-%m-%d")                   # -> "1970-01-01"
now | strftime("%Y-%m-%d %H:%M:%S")
```

### `strptime`

Parse a timestamp string into a broken-down UTC time array.

**Signatures**

- `strptime(value: string, fmt: string) -> list`

**Details**

Matches jq's ``strptime``: parses *value* using *fmt* and returns
the broken-down array compatible with ``mktime`` /
``gmtime`` (jq's order).

Related: ``strftime``, ``mktime``, ``fromdate``.

**Examples**

```
strptime("2025-01-01", "%Y-%m-%d") | mktime
```

### `todate`

Unix epoch seconds → ISO-8601 UTC string.

**Signatures**

- `todate(value: number) -> string`

**Details**

Matches jq's ``todate`` / ``todateiso8601``: formats a Unix
epoch-seconds value as an ISO-8601 string in UTC, second
precision (``YYYY-MM-DDTHH:MM:SSZ``).

Related: ``fromdate``, ``strftime``, ``now``.

**Examples**

```
todate(0)                                # -> "1970-01-01T00:00:00Z"
now | todate
```

### `todateiso8601`

Alias of ``todate`` — Unix epoch → ISO-8601 UTC string.

**Signatures**

- `todateiso8601(value: number) -> string`

**Details**

Matches jq's ``todateiso8601`` (same as ``todate``).  Both names
available for jq snippet portability.

Related: ``todate``, ``fromdateiso8601``.

**Examples**

```
todateiso8601(0)
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
partition(.)] | unique`` enumerates every partition that owns
at least one virtual server (``unique`` returns sorted output
— no need to chain ``| sort``).

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

### `ip_translate`

Map an address from a source network to a destination network, across address families when needed.

**Signatures**

- `ip_translate(src_net: string, dst_net: string, addr: string) -> string`

**Details**

Computes the host-bit offset of *addr* within *src_net* and applies
that same offset within *dst_net*.  When the two networks belong to
different families (IPv4 / IPv6) this performs an address-family
translation: the host portion of an IPv4 address can be re-emitted
inside an IPv6 prefix and vice versa.

*src_net* must cover *addr*: if the host bits of *addr* relative to
*src_net* don't fit inside *dst_net* (i.e. ``dst_net`` is more
specific than ``src_net``), :class:`BuiltinError` is raised so
silent truncation can't slip through.

The returned string is the bare address — partition prefix and port
are not preserved (callers building tmsh stanzas can re-attach
them with ``+`` concatenation).  Use ``ip(net, src)`` instead when
the operation stays in one family and you want to keep partition /
route-domain / port from the source.

Related: ``ip``, ``in_cidr``, ``net``.

**Examples**

```
ip_translate("10.0.0.0/8", "2001:db8::/32", "10.1.2.3")
# -> "2001:db8::1:203"
ip_translate("192.168.50.0/24", "2001:db8:50::/64", "192.168.50.10")
# -> "2001:db8:50::a"
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

### `ucs_cert`

Parse the real certificate a BIG-IP cert object points at, read from its UCS.

**Signatures**

- `ucs_cert(cert: object) -> object`

**Details**

Takes a ``sys file ssl-cert`` (or ``cm cert``) object that was
loaded from a UCS archive and returns the *actual* certificate,
parsed into the same shape :func:`x509_parse` produces (``subject``
/ ``issuer`` / ``serial`` / ``fingerprint_sha256`` / ``sans`` /
``not_after`` / ``key_size`` / ``public_key_pem`` / …).

Unlike :func:`x509_from_config`, which only surfaces the metadata
the ``.conf`` stanza happens to record — on a real archive that is
often just file pointers (``cache-path`` / ``revision``), with no
``fingerprint`` / ``serial`` / ``sans`` — ``ucs_cert`` re-opens the
UCS the object came from, reads the PEM out of the filestore
(located by the stanza's ``cache-path``), and parses it.  That
recovers the full identity even when the stanza carries nothing.

Certificates are public, so no key or master key is involved.  An
encrypted UCS is decrypted with the same passphrase resolution the
other verbs use (``$F5_UCS_PASSPHRASE`` / prompt).  Reads from disk
like :func:`cert_load`; it is not gated by ``--enable-probes``.

Raises when the object did not come from a file-backed UCS, when
the stanza has no ``cache-path``, or when no matching member is in
the archive.

Related: ``x509_from_config`` (stanza metadata only), ``x509_eq``,
``cert_load``, ``tls_handshake``.

**Examples**

```
.sys["file-ssl-cert"]["/Common/app.crt"] | ucs_cert(.)
.sys["file-ssl-cert"][] | ucs_cert(.) | {subject, fingerprint_sha256, not_after}
```

### `files` / `file` / `glob` / `grep`

Forensic access to the OS files **inside** a source's UCS archive — the paths an
attacker tampers with on a compromised BIG-IP (login-persistence dotfiles and
SSH keys, the local account databases, external-auth / PAM config, syslog-ng,
cron). Only available when the source is a `.ucs` read through the f5 CLI (the
in-browser console has no archive to read and these error clearly).

**Signatures**

- `files() -> [object]` — the forensic member inventory; each entry is
  `{path, size, sha256, is_text}` (metadata only, no content).
- `glob(pattern: string) -> [object]` — inventory entries whose `path` matches a
  shell glob (`*`, `?`).
- `file(path: string) -> object` — one member by exact path, with `content`
  attached for text files (`null` for binary / missing / sensitive). `null` when
  the path is absent.
- `grep(pattern: string[, path_glob: string]) -> [object]` — regex-search text
  members (optionally scoped by a path glob), returning `{path, line, text}` per
  match.

**Details**

`content` is only ever populated for small non-sensitive text files; `etc/shadow`
and private keys are classified sensitive and their bytes are never returned.
The inventory is scoped to the same forensic member set the report's Forensics
tab uses.

**Examples**

```
files | length
glob("home/*/.ssh/authorized_keys")
file("etc/passwd") | .content
grep("^[^:]+:[^:]*:0:", "etc/passwd")          # UID 0 accounts
grep("nohup|curl .*\\| *sh", "home/*/.bashrc") # login-hook persistence
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

### `x509_eq`

Compare two ``x509_parse``-shaped dicts for cert identity.

**Signatures**

- `x509_eq(a: object, b: object) -> boolean`

**Details**

Returns ``true`` when the two dicts describe the same X.509
certificate.  Comparison order, strongest → weakest:

1. ``fingerprint_sha256`` — the canonical identity.  Two certs
   with the same SHA-256 fingerprint are the same cert.
2. ``subject`` + ``issuer`` + ``serial`` — the X.509-defined
   primary key.  Used when one side is a BIG-IP ``sys file
   ssl-cert`` projection that doesn't carry a SHA-256
   fingerprint.

Plain ``==`` on the dicts compares every field — including
``not_before`` / ``sig_alg`` / ``public_key_pem`` which the
BIG-IP side leaves ``null`` — and so reports certs as
different even when they describe the same key material.  Use
this helper instead for "same cert" semantics.

Related: ``x509_parse``, ``x509_from_config``.

**Examples**

```
x509_eq(x509_parse(.body), x509_from_config($cert))
.sys.file.ssl-cert[] | select(x509_eq(x509_from_config(.), $peer))
```

### `x509_from_config`

Project a BIG-IP config-object cert into the ``x509_parse`` shape.

**Signatures**

- `x509_from_config(cert: object) -> object`

**Details**

Takes a BIG-IP config object that carries cert metadata and
returns a dict in the same shape :func:`x509_parse` produces:
``subject`` / ``issuer`` / ``not_after`` / ``serial`` /
``fingerprint_sha256`` / ``sans`` / ``key_alg`` / ``key_size``
/ ``version`` / etc.

Supported config objects (anywhere a cert appears in the
parsed model):

- ``sys file ssl-cert`` — cert / chain / bundle store, the
  target of every client-ssl / server-ssl ``cert-key-chain``
  and ``ltm monitor https.cert`` PathRef.
- ``cm cert`` — device-trust certs, the target of ``cm
  device.cert``, ``cm trust-domain.ca-cert``, ``cm
  trust-domain.ca-cert-bundle``.

For config objects that *reference* a cert by PathRef
(``ltm monitor https``, ``ltm profile client-ssl``,
``ltm profile server-ssl``, ``cm device``, ``cm
trust-domain``), index into the referent first then pipe to
this builtin.  ``sys crypto cert`` is a minimal projection
that doesn't carry metadata — load its PEM with
:func:`cert_load` instead.

Normalisation handled at the boundary: SAN strings are split
on commas with the ``DNS:`` / ``IP:`` prefix stripped; the
fingerprint is converted from ``SHA256/12:34:…`` to bare
uppercase hex; ``serial-number`` is converted from decimal to
uppercase hex; ``version`` is mapped from ``"3"`` to ``"v3"``;
``key-type`` is mapped to cryptography's public-key class
names (``rsa-public`` → ``RSAPublicKey``).  Fields BIG-IP's
TMSH surface doesn't carry (``not_before``, ``sig_alg``,
``public_key_pem``) come back ``null``.

Pair with :func:`x509_eq` to compare a BIG-IP cert against one
fetched from a live endpoint or a PEM file.

Related: ``x509_parse`` (parse a PEM string),
``x509_load_file`` (parse a PEM file), ``x509_eq``.

**Examples**

```
.sys.file.ssl-cert["/Common/example.crt"] | x509_from_config(.)
.cm.cert["/Common/dtca.crt"] | x509_from_config(.)
x509_eq(.sys.file.ssl-cert["/Common/example.crt"] | x509_from_config(.), x509_parse(url_get("https://example.com/cert.pem").body))
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

Type / identity introspection (`kind`, `path`, `length`, `defined`, `type`), object-shape conversions (`to_entries` / `from_entries` / `with_entries` / `has` / `in`), and jq-style tree manipulation (`paths` / `leaf_paths` / `getpath` / `setpath` / `del` / `delpaths` / `walk` / `recurse` / `until` / `repeat`).

### `cert_load`

Load and parse an X.509 cert from disk (PEM / DER / PKCS#12).

**Signatures**

- `cert_load(path: string) -> object | list[object]`
- `cert_load(path: string, password: string) -> object | list[object]`

**Details**

Reads *path* from disk and returns a structured cert dict in the
same shape :func:`x509_parse` produces.  The file format is
sniffed from the bytes — extension hints (``.crt``, ``.pem``,
``.cer``, ``.der``, ``.pfx``, ``.p12``) are tolerated but not
required:

- **PEM** (``-----BEGIN CERTIFICATE-----``): parsed directly.
  When the file contains a *chain* (multiple PEM blocks) a
  list is returned, leaf first.
- **DER**: re-encoded to PEM and parsed.
- **PKCS#12** (``.pfx`` / ``.p12``): unpacked into the
  end-entity cert plus any chain certs.  Pass *password* as
  the optional second argument when the bundle is encrypted;
  omit it for plain bundles.  Returns ``[leaf, *chain]`` when
  a chain is present, otherwise just the leaf dict.

Tilde expansion is honoured.  Raises :class:`BuiltinError` for
missing files, unreadable formats, or wrong passwords.  No
network access — purely local file IO.

Related: ``x509_parse`` (parse an in-memory PEM string),
``tls_handshake`` (peer cert pre-parsed in ``peer_cert``).

**Examples**

```
cert_load("/etc/ssl/certs/server.crt")
cert_load("./bundle.pfx", "trustno1")
cert_load("chain.pem") | first | .subject
```

### `csv_load`

Read a CSV file from disk and parse it into a list of records.

**Signatures**

- `csv_load(path: string) -> list[object]`
- `csv_load(path: string, headers: list[string]) -> list[object]`

**Details**

Reads *path* as CSV.  With one argument the first row of the
file names the columns (the jq-natural shape, matches what most
spreadsheet exports look like).  With two arguments *headers*
is a list of column names and every row of the file is treated
as data — use this form for header-less CSVs (firewall NAT
exports, RFC 4180 fragments, etc.).

Values are returned as strings.  The DSL's ``+`` operator
coerces scalars when one side is a string, so number-shaped
cells (``"443"``) flow through arithmetic without an explicit
cast.  Missing trailing columns become empty strings; rows
that overflow the header list land their extras in an
``_extra`` list.

Tilde expansion is honoured.  Raises :class:`BuiltinError` for
missing files or unreadable CSV.

Related: ``jsonl_load``, ``json_load``, ``csv`` /
``tsv`` (render to one-row strings).

**Examples**

```
csv_load("/etc/inventory/servers.csv")
csv_load("nats.csv", ["internal", "external"])
csv_load("vips.csv") | map(.name)
```

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

### `del`

Return the current value with the slot at *path* removed.

**Signatures**

- `del(path: list) -> any`

**Details**

Matches jq's ``del`` for the single-path form.  Removes the slot
at *path* and returns a fresh copy of the current value.  Missing
paths are silently no-ops (jq parity).

jq's ``del`` accepts a path **expression** (``del(.foo)``); this
DSL takes a path **list** instead (``del(["foo"])``) so we don't
have to introduce a third evaluation mode.  Use ``delpaths`` for
deleting multiple paths in one call.

Related: ``delpaths``, ``setpath``, ``getpath``.

**Examples**

```
{a: 1, b: 2} | del(["a"])               # -> {b: 2}
{a: [1, 2, 3]} | del(["a", 1])           # -> {a: [1, 3]}
```

### `delpaths`

Return the current value with every slot in *paths* removed.

**Signatures**

- `delpaths(paths: list[list]) -> any`

**Details**

Matches jq's ``delpaths``: takes a list of path-lists and returns
a copy of the current value with every slot deleted.  Paths must
be sorted long-to-short (jq's expectation) — this DSL sorts them
internally so callers don't have to worry about deletion order
invalidating later paths.

Related: ``del``, ``setpath``, ``paths``.

**Examples**

```
{a: 1, b: 2, c: 3} | delpaths([["a"], ["c"]])  # -> {b: 2}
```

### `env`

The process environment as a dict — jq's ``env``.

**Signatures**

- `env() -> object`

**Details**

Matches jq's ``env`` builtin: returns the process environment
variables as an object.  Read-only — modifications don't
propagate back.

Related: ``$ENV`` (jq's variable form, not exposed here).

**Examples**

```
env | .HOME
env | with_entries(select(.key | startswith("F5_")))
```

### `f5log_load`

Read a BIG-IP log file from disk and parse it into structured events.

**Signatures**

- `f5log_load(path: string) -> list[object]`

**Details**

Reads *path* as a BIG-IP log and parses each line into a
structured event dict:

.. code-block:: text

   { "timestamp": "Nov 28 09:53:00"
   , "host": "bigip01"
   , "severity": "info"
   , "daemon": "tmm"
   , "pid": 12345
   , "code": "01230140:6"
   , "module": "01230140"
   , "level": 6
   , "message": "Connection limit reached for pool /Common/web_pool"
   , "raw": "<original line>"
   }

Handles classic syslog, RFC3164-with-PRI, and the F5
``XXXXXXXX:N:`` message-code form.  Lines that don't match
land with ``message`` set to the original text and the typed
fields blank, so a grep / filter pipeline never silently
drops unknown shapes.

Tilde expansion is honoured.  Pairs naturally with the
classification predicates (``in_cidr`` / ``is_private``) when
the message body contains an IP — split on whitespace inside
the message and feed candidates through.

Related: ``jsonl_load``, ``csv_load``, ``json_load``.

**Examples**

```
f5log_load("/var/log/ltm") | last
[f5log_load("/var/log/tmm") | select(.severity == "err")] | count
f5log_load("audit.log") | select(.daemon == "logger" and .module == "01070417")
```

### `from_entries`

Convert a list of ``{key, value}`` entries back into an object.

**Signatures**

- `from_entries(value: list[object]) -> object`

**Details**

Inverse of ``to_entries``.  Each entry may spell its key as
``key`` / ``k`` / ``name`` and its value as ``value`` / ``v``,
matching jq's flexibility.  Missing values default to ``null``;
missing keys raise.

Keys are coerced to strings (jq parity — JSON object keys are
strings).  Duplicate keys: later entries overwrite earlier ones.

Related: ``to_entries``, ``with_entries``.

**Examples**

```
[{key: "a", value: 1}, {key: "b", value: 2}] | from_entries
to_entries | from_entries                # round-trips an object
```

### `getpath`

Read the value at the given path (list of string / integer keys).

**Signatures**

- `getpath(path: list) -> any`

**Details**

Matches jq's ``getpath``: walks the current value following each
element of *path* (strings index objects, integers index arrays /
streams).  Returns ``null`` when any element is missing or
out-of-range.

Composes naturally with ``paths``: ``[paths] | map(getpath(.))``
enumerates every reachable value.

Related: ``setpath``, ``del``, ``delpaths``, ``paths``.

**Examples**

```
getpath(["a", "b"])                     # walk .a.b safely
{a: {b: 42}} | getpath(["a", "b"])      # -> 42
```

### `has`

True when an object has the given field, or an array has the given index.

**Signatures**

- `has(value: object, key: string) -> boolean`
- `has(value: list, index: integer) -> boolean`

**Details**

Matches jq's ``has``:

- For an **object** (``ObjectRef`` / ``dict``), tests whether the
  key is present.  Keys are strings; integer keys are coerced.
- For a **list / stream**, tests whether the integer index is
  within bounds (``0 <= index < length``).

Use the implicit-receiver form for the natural reading:
``.ltm.virtual[] | select(has(.snatpool))`` keeps only VSes whose
object exposes a ``snatpool`` field.  Pair with ``defined`` when
you also need to filter out an explicit empty string.

Related: ``in`` (inverse), ``keys``, ``defined``.

**Examples**

```
.ltm.virtual[] | select(has(.snatpool)) | .name
{a: 1, b: 2} | has("a")                  # -> true
[10, 20, 30] | has(1)                    # -> true
```

### `in`

True when the input is a key of the given object (or a valid index of the given array).

**Signatures**

- `in(key: string, value: object) -> boolean`
- `in(index: integer, value: list) -> boolean`

**Details**

Matches jq's ``in``: the inverse of ``has``.  The input is the key
being tested; the argument is the container.  Reads naturally with
the implicit-receiver form:
``"snatpool" | in(.ltm.virtual.web_vs)``.

Related: ``has`` (inverse), ``keys``.

**Examples**

```
"name" | in({name: "x", pool: "y"})       # -> true
.ltm.virtual[].name | select(in($wanted)) # keep VSes named in $wanted
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

### `jsonl_load`

Read a file from disk and parse it as JSON Lines (NDJSON).

**Signatures**

- `jsonl_load(path: string) -> list`

**Details**

Reads *path* line by line and parses each non-blank line as a
JSON value, returning the list in file order.  This is the
natural shape for log streams, event archives, and any other
one-record-per-line dump where loading the whole file as one
JSON value would force every consumer to know about the
framing.

Blank lines are skipped.  Any line that fails to parse raises
:class:`BuiltinError` with the offending line number so a bad
record in a large dump is easy to find.

Tilde expansion is honoured.

Related: ``json_load`` (whole-file JSON), ``json_parse``
(in-memory string), ``csv_load`` (CSV with or without headers).

**Examples**

```
jsonl_load("/var/log/events.jsonl")
.ltm.virtual[].name as $n | jsonl_load("events.jsonl")[] | select(.vs == $n)
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
[.ltm.virtual[] | refs(.)[]] | unique     # sorted by ``unique`` itself
```

### `leaf_paths`

Every path through the value that ends at a non-container leaf.

**Signatures**

- `leaf_paths() -> stream[list]`

**Details**

Matches jq's ``leaf_paths``: emits the path to every leaf value
(string, number, bool, null) inside the current value.  Internal
composite positions are not included.

Equivalent to ``paths(. | type != "object" and . | type != "array")``.

Related: ``paths``, ``getpath``.

**Examples**

```
{a: 1, b: [2, 3]} | leaf_paths
[.ltm.virtual.web_vs] | leaf_paths      # every leaf inside one VS
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

### `paths`

Every path through the value as a stream of lists.

**Signatures**

- `paths() -> stream[list]`
- `paths(filter) -> stream[list]`

**Details**

**Special form.**  Matches jq's ``paths``: emits a stream where
each element is a path (list of strings / integers) into the
current value.  Every reachable composite **and** leaf position
is enumerated except the empty root path.

With a *filter* argument, only emits paths whose value satisfies
*filter*: ``paths(type == "string")`` yields every path to a
string leaf.

Useful for introspection, audit, and as input to ``getpath`` /
``setpath`` / ``delpaths``.

Related: ``leaf_paths``, ``getpath``, ``setpath``, ``del``,
``delpaths``, ``walk``.

**Examples**

```
{a: 1, b: [2, 3]} | paths
paths(type == "number")
```

### `pick`

Keep only the slots named by *path_exprs*; everything else is dropped.

**Signatures**

- `pick(...path_exprs) -> any`

**Details**

**Special form.**  Matches jq 1.7's ``pick``: enumerates one or
more path expressions against the current value, then returns
a value containing only those paths (with intermediate
containers reconstructed).

Each argument should be a static path projection (``.foo``,
``.bar.baz``, ``.list[0]``).  Multiple paths may be passed either
as separate function arguments (``pick(.a, .c)``) or as a single
comma-stream argument (``pick((.a, .c))``) — both work.  Missing
slots are silently ignored.

Related: ``getpath``, ``setpath``, ``del``, ``paths``.

**Examples**

```
{a: 1, b: 2, c: 3} | pick(.a, .c)         # -> {a: 1, c: 3}
{a: {b: 1, c: 2}, d: 3} | pick(.a.b, .d)
```

### `recurse`

Emit every value reachable from the current input, optionally driven by *body*.

**Signatures**

- `recurse() -> stream`
- `recurse(body) -> stream`
- `recurse(body, cond) -> stream`

**Details**

**Special form.**  Matches jq's ``recurse`` family.

- **Zero args** (``recurse``) — emits the current value, then
  every reachable value (every dict value, every array element,
  every nested composite, recursively).
- **One arg** (``recurse(body)``) — emits the current value, then
  ``body`` applied once, then ``body`` applied to that, etc.
  Stops on null.
- **Two args** (``recurse(body, cond)``) — same as the one-arg
  form but stops when ``cond`` is false.

To prevent runaway loops, this DSL caps total emissions at
100,000 — pipelines that legitimately need more should narrow
*body* or pre-collect.

Related: ``walk``, ``map``, ``paths``.

**Examples**

```
{a: 1, b: {c: 2}} | recurse
1 | recurse(. + 1, . < 5)              # -> 1, 2, 3, 4
```

### `recurse_down`

Alias of ``recurse`` with no arguments — emit every reachable value.

**Signatures**

- `recurse_down() -> stream`

**Details**

Matches jq's ``recurse_down`` (deprecated in jq itself, kept for
backward compatibility).  Same as ``recurse`` with no arguments.

Related: ``recurse``, ``walk``.

**Examples**

```
{a: 1, b: {c: 2}} | [recurse_down]
```

### `repeat`

Emit ``f(.), f(f(.)), …`` capped at 100,000 iterations.

**Signatures**

- `repeat(body) -> stream`

**Details**

**Special form.**  Matches jq's ``repeat`` modulo a safety cap.
Emits an infinite-by-design stream of successive applications of
*body* to the current value.  jq pairs this with its generator-
form ``limit(n; gen)`` to bound the result; this DSL's ``limit``
is the **value form** (``stream | limit(n)``), so collect the
repeat output first.  An absolute cap of 100,000 emissions
prevents a forgetful pipeline wedging the evaluator.

Common pattern: ``1 | [repeat(. + 1)] | limit(5)`` — though
``[range(1, 6)]`` is usually shorter when the body is just
``. + 1``.

Related: ``until``, ``recurse``, ``range``, ``limit``.

**Examples**

```
1 | [repeat(. + 1)] | limit(5)        # -> [2, 3, 4, 5, 6]
```

### `setpath`

Return the current value with *path* set to *new_value*.

**Signatures**

- `setpath(path: list, new_value: any) -> any`

**Details**

Matches jq's ``setpath``: creates a copy of the current value with
the slot at *path* set to *new_value*.  Missing intermediate
containers are auto-created based on the next key type: a string
key creates a dict, an integer key creates a list.

Returns a fresh Python value — this is a functional update, not
an in-place mutation.  For BIG-IP edit-pipeline writes, use the
``=`` / ``|=`` assignment operators instead.

Related: ``getpath``, ``del``, ``delpaths``.

**Examples**

```
{a: 1} | setpath(["b", "c"], 9)         # -> {a:1, b:{c:9}}
{a: [1, 2, 3]} | setpath(["a", 1], 99)
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

### `to_entries`

Convert an object to a list of ``{key, value}`` entries.

**Signatures**

- `to_entries(value: object) -> list[object]`

**Details**

Matches jq's ``to_entries``: an object ``{a: 1, b: 2}`` becomes
the list ``[{"key": "a", "value": 1}, {"key": "b", "value": 2}]``.
Entries are emitted in sorted key order so the result is
deterministic.

Useful for treating an object as a stream of named slots —
iterate, filter, transform, then put the object back together
with ``from_entries``.

Related: ``from_entries``, ``with_entries``, ``keys``, ``values``.

**Examples**

```
{a: 1, b: 2} | to_entries
.ltm.virtual.web_vs | to_entries | map(.key)  # field names, same as keys
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

### `until`

Iterate *update* against the current value until *cond* becomes true.

**Signatures**

- `until(cond, update) -> any`

**Details**

**Special form.**  Matches jq's ``until``: tests *cond* against
the current value first, and if it is already truthy returns
that value unchanged.  Otherwise applies *update* (with ``.``
re-bound to the running value), re-checks *cond*, and repeats —
so the result is the first value (current input or any
transformed iteration) for which *cond* is truthy.

Capped at 100,000 iterations to prevent runaway loops — pipelines
that legitimately need more should restructure.

Related: ``recurse``, ``repeat``.

**Examples**

```
1 | until(. >= 100, . * 2)              # -> 128
0 | until(. > 5, . + 1)                  # -> 6
```

### `walk`

Apply *body* to every value bottom-up, returning the rebuilt structure.

**Signatures**

- `walk(body) -> any`

**Details**

**Special form.**  Matches jq's ``walk``: traverses the current
value's tree bottom-up.  For each composite, ``walk`` first
recurses into its children, then evaluates *body* with ``.``
re-bound to the (now-transformed) composite.  Leaves are passed
directly to *body*.

Classic uses:

- **String normalisation across a whole config**:
  ``walk(if type == "string" then ascii_downcase else . end)``.
- **Field renames**: ``walk(if type == "object" then
  with_entries(...) else . end)``.
- **Pruning**: ``walk(if type == "array" then map(select(.))
  else . end)`` drops empty entries from every array.

Related: ``recurse``, ``map``, ``with_entries``.

**Examples**

```
walk(if type == "string" then ascii_downcase else . end)
walk(if type == "number" then . + 1 else . end)
```

### `with_entries`

Apply *body* to each ``{key, value}`` entry of an object and reassemble.

**Signatures**

- `with_entries(body) -> object`

**Details**

**Special form.**  Matches jq's ``with_entries``: equivalent to
``to_entries | map(body) | from_entries``.  For each entry of the
input object, evaluates *body* with ``.`` re-bound to a
``{key, value}`` object and collects the results into a new
object.

The body must yield ``{key, value}``-shaped objects (or the
relaxed ``k`` / ``v`` / ``name`` spellings ``from_entries``
accepts).  This DSL doesn't support property assignment on plain
dicts, so use object literals to reshape entries (jq's
``with_entries(.key |= upcase)`` becomes
``with_entries({key: upcase(.key), value: .value})``).

Returning the ``select`` drop sentinel drops the entry — handy
for filtering object fields.

Related: ``to_entries``, ``from_entries``, ``map``, ``select``.

**Examples**

```
with_entries({key: upcase(.key), value: .value})   # uppercase field names
with_entries(select(.value | type == "string"))    # keep only string fields
```

---

*Mirrors the builtin registry in `rust/tcl-bigip-query/src/builtins/`;
`f5 query --help-builtins` renders the same catalogue from the binary.*
