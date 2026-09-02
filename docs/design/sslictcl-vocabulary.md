# SslicTcl vocabulary 1

`.sslictcl` is a declarative document describing TLS endpoints, certificates,
trust programs, catalogue facts, and assurance policies. It is written in Tcl
*syntax* — Tcl supplies quoting, comments, line continuation, and nested braced
blocks — and it is **never evaluated**. `tcl_sslictcl::dsl` walks the canonical
concrete syntax tree and rejects command substitution, variable substitution,
and `{*}` argument expansion instead of guessing at them.

The machine-readable form of this table is
[`tcl_sslictcl::vocabulary::DECLARATIONS`](../../rust/tcl-sslictcl/src/vocabulary.rs).
A unit test synthesises a fully-declared instance of every declaration and
asserts the loader knows exactly the words the table declares — and no others —
so the two cannot drift.

## Declarations

The top level is **open**: an unrecognised statement is preserved as an
extension and reported as `SSLIC1101`.

| Declaration | Key | Body | Members |
|---|---|---|---|
| `sslictcl VERSION` | `VERSION` | — | exactly once, exactly two words |
| `certificate NAME { … }` | `NAME` | open | `pem TEXT` \| `material TEXT` (exactly one required), `key NAME` |
| `endpoint NAME { … }` | `NAME` | open | `hostname TEXT`; `protocols LIST`; `ciphers LIST`; `groups LIST`; `signature-schemes LIST`; `certificate-chain LIST`; `chain NAME`; `policy NAME`; `hsts { … }` |
| ⤷ `hsts { … }` | — | closed | `enabled BOOL`; `max-age INT`; `include-subdomains BOOL`; `preload BOOL` |
| `testssl-import NAME { … }` | `NAME` | closed | `schema 1`; `raw-json-hex HEX` |
| `trust-program NAME { … }` | `NAME` | open | `client CLIENT`; `version TEXT`; `generated-at TEXT`; `source-name TEXT`; `source-url TEXT`; `source-revision TEXT`; `source-license TEXT`; `anchor SHA256 { … }` |
| ⤷ `anchor SHA256 { … }` | `SHA256` | closed | `subject TEXT`; `der-base64 TEXT`; `purposes LIST`; `trusted BOOL`; `distrust-after INT` |
| `protocol VERSION { … }` | `VERSION` | closed | `status STATUS`; `score INT` (0–100); `reference TEXT` |
| `cipher NAME { … }` | `NAME` | closed | `iana-name TEXT`; `openssl-name TEXT`; `key-exchange TEXT`; `authentication TEXT`; `encryption TEXT`; `bits INT`; `forward-secrecy BOOL`; `aead BOOL`; `status STATUS`; `protocols LIST` |
| `chain NAME { … }` | `NAME` | closed | `certificates LIST` (**required**; certificate names, leaf first) |
| `policy NAME { … }` | `NAME` | closed | `check ID { … }`; `grade { … }` |
| ⤷ `check ID { … }` | `ID` | closed | `severity SEVERITY`; `message TEXT`; `require-protocols LIST`; `forbid-protocols LIST`; `forbid-ciphers LIST`; `require-forward-secrecy BOOL`; `min-key-bits INT`; `require-hsts BOOL`; `min-hsts-max-age INT`; `predicate SCRIPT` |
| ⤷ `grade { … }` | — | closed | `minimum GRADE` |

## Value domains

| Domain | Accepts |
|---|---|
| `BOOL` | `true` \| `false` \| `yes` \| `no` \| `on` \| `off` \| `1` \| `0`, case-insensitive |
| `INT` | an unsigned decimal integer |
| `LIST` | one braced Tcl list of literal words, or a single bare word |
| `TEXT` | one literal word |
| `CLIENT` | `mozilla` \| `chrome` \| `apple` \| `microsoft` \| `android` \| `openjdk` (`trust::ClientFamily`) |
| `STATUS` | `recommended` \| `acceptable` \| `deprecated` \| `prohibited` (`model::TlsStatus`) |
| `SEVERITY` | `info` \| `warning` \| `error` \| `critical` (`estimate::EstimateSeverity`) |
| `GRADE` | `A+` \| `A` \| `B` \| `C` \| `D` \| `E` \| `F` (`estimate::Grade`; `T`, `M`, and unknown are estimator outcomes, not declarable) |
| `VERSION` | any `model::ProtocolVersion` spelling — `ssl2`, `ssl3`, `tls1.0` … `tls1.3`, and their accepted aliases |
| `SHA256` | 64 hexadecimal digits, case-insensitive |
| `HEX` | an even number of hexadecimal digits |
| `SCRIPT` | one braced literal word, retained verbatim |

`purposes` is a `LIST` whose elements are trust purposes — `server-auth`,
`client-auth`, `email-protection`, `code-signing`, `any` — and an unrecognised
element is `SSLIC1009`.

## The openness rule

A block is either **open** or **closed**, and the distinction is the whole
forwards-compatibility story:

- **open** (`certificate`, `endpoint`, `trust-program`, and the top level) —
  an unrecognised member is preserved losslessly in the declaration's
  `extensions` map and reported as `SSLIC1101` (hint). A newer document keeps
  working on an older build, and re-emitting it does not lose the words this
  build did not understand.
- **closed** (everything else) — an unrecognised member is `SSLIC1007`
  (error). The member is skipped and the rest of the block still loads. These
  blocks describe fixed, fully-specified records where a stray word is far more
  likely a typo than a future feature.

A `sslictcl VERSION` greater than 1 is accepted with an `SSLIC1102` warning:
the document loads, and everything this build does not recognise is preserved
by the open blocks.

## The never-evaluated guarantee

Nothing in a `.sslictcl` document is ever executed, and nothing outside the
document is ever consulted while loading it.

- The loader walks the concrete syntax tree. It never constructs a Tcl
  interpreter and never calls one.
- Substitution in any form — `$var`, `[cmd]`, `{*}` expansion — is `SSLIC1002`,
  not an evaluation.
- A `check`'s `predicate SCRIPT` is stored on the check as the exact inner text
  of its braced word. It is **not** parsed into statements, **not** analysed,
  and **not** evaluated by [`policy::evaluate_policy`]; vocabulary 1 reports it
  as `SSLIC1103` (hint) and ignores it. It exists so a policy can carry a
  richer rule forward to a vocabulary that can evaluate it, without any build
  in between silently mis-implementing it.
- `testssl-import` carries its source document as hexadecimal precisely so that
  no JSON payload can ever reach the Tcl quoting rules.

## Name resolution

Cross-declaration names resolve in a post-pass, after the whole document has
been read, so **declaration order is irrelevant**:

- `endpoint … chain NAME` must name a declared `chain`; resolving it fills the
  endpoint's `certificate_chain` from that chain so downstream consumers see
  the same field they always did.
- `endpoint … policy NAME` must name a declared `policy`.
- every name in `chain … certificates` must be a declared `certificate`.

An unresolved name is `SSLIC1011`, ranged over the referring word. `chain` and
`certificate-chain` on one endpoint are mutually exclusive: declaring both is
`SSLIC1012`.

## Policy evaluation and finding identity

Loading produces `model::Policy` values. Nothing is evaluated until a caller
hands one to `policy::evaluate_policy(policy, endpoint, certificates,
estimate)` — a separate phase, never part of loading.

A check is the **conjunction** of its populated members: every one must be
satisfied. A failing check yields exactly one `PolicyFinding`, and **the
identity of a finding is the pair `(check_id, endpoint)`** — one row per check
per endpoint, never one per unsatisfied conjunct. The reasons are collected in
the finding's `evidence` list. The finding's `code` is
`SSLICTL-POLICY-<check_id>`, its severity defaults to `warning`, and its
message defaults to one derived from the check identity.

`forbid-ciphers` entries are Tcl-style glob patterns evaluated with the
workspace's shared `tcl_syntax::glob::string_match`, so they mean exactly what
`string match` means. `grade.minimum` below the estimate's grade rank yields a
finding whose `check_id` is `grade`.

## Diagnostics

Every loader problem carries a published `DiagCode` and a byte range into the
**original** document — nested block members included — so an editor can
underline the exact word.

| Code | Meaning |
|---|---|
| `SSLIC1001` | not valid Tcl syntax, or an unclosed delimiter |
| `SSLIC1002` | substitution or argument expansion; the vocabulary is declarative |
| `SSLIC1003` | missing `sslictcl VERSION` header |
| `SSLIC1004` | the `sslictcl` header is declared more than once |
| `SSLIC1005` | wrong number of words |
| `SSLIC1006` | a declaration body must be a braced literal |
| `SSLIC1007` | unknown member in a closed block |
| `SSLIC1008` | duplicate declaration of the same kind and name |
| `SSLIC1009` | value outside its declared domain |
| `SSLIC1010` | missing required member |
| `SSLIC1011` | reference to a name that is not declared |
| `SSLIC1012` | mutually exclusive members |
| `SSLIC1101` | unknown declaration preserved as an extension (hint) |
| `SSLIC1102` | vocabulary newer than this build supports (warning) |
| `SSLIC1103` | `predicate` retained but never evaluated (hint) |

`dsl::load_with_diagnostics(source) -> DslLoad` recovers: a bad statement is
skipped and loading continues, a bad member is skipped and its block continues,
and `DslLoad::document` is `Some` whenever the top-level statement stream
segmented and a usable header was seen. `dsl::load(source)` is the thin wrapper
that returns the first error as a `DslError`.

## Emitting

`SslicModel::to_sslictcl()` is deterministic: declarations in the vocabulary
order of the table above, `BTreeMap` iteration order within each kind,
four-space indent per level, and one shared word-quoting helper. Loading the
emitted text reproduces an equal model, and emitting that model reproduces
byte-identical text.

## See also

- `samples/sslictcl/example.sslictcl` — a document exercising every
  declaration and member, plus an unknown word in each open block.
- [contracts/sslictcl-source-data.md](contracts/sslictcl-source-data.md) — the
  embedded trust-store and TLS source-data bundle, its provenance schema, and
  its offline drift gate.
