# Error recovery — ghost delimiter injection

The parser must produce well-formed `SegmentedCommand`s even when the source
has a missing delimiter, so that a single unclosed bracket near the top of a
file does not cost the user every symbol, diagnostic, and completion below it.
Recovery repairs the token stream, reports the break as an `E20x` diagnostic,
and lets the rest of the pipeline proceed on the repaired parse.

Source:

- `rust/tcl-compiler/src/segmenter.rs` — `segment_with_recovery`, the re-lex
  driver.
- `rust/tcl-compiler/src/analyser/syntax_checks.rs` — the per-code heuristics
  (`detect_e201` and its `e201_at_comment` / `e201_at_command` / `e201_at_brace`
  candidates, plus the E202 / E203 builders).
- `rust/tcl-lexer/src/lexer.rs` — ghost delimiter injection
  (`Lexer::with_ghosts`).
- `rust/tcl-lexer/src/structural_index.rs` — the structural-state index used to
  veto inert insertion points.

## Ghosts

A **ghost** is a zero-width closing delimiter attached to the lexer at a source
offset. When the scanner reaches that offset it sees the ghost byte before the
real one, so the token stream closes as if the delimiter had been typed, while
every real byte keeps its original offset. Ghosts therefore never shift the
positions of anything after them — diagnostics, symbols, and ranges derived
from a recovered parse stay correct against the user's untouched buffer.

## The recovery loop

```
source
   │
   ▼
segment_commands_local()          ← first plain parse (CST → SegmentedCommands)
   │
   ▼
unterminated_bracket_diagnostics()  per command
   │  each heuristic proposes an insertion offset, carried as the
   │  diagnostic's suggested fix
   ▼
structural-index veto              ← is that offset inert?
   │
   ▼
ghosts: {offset → b']'}
   │
   ▼
build_document_with_ghosts() + segments_from_document()   ← re-lex
   │
   └── repeat while a pass adds a new ghost (capped at 32 passes)
```

The loop iterates because one re-lex can expose a *further* unterminated `[`
that the previous parse had swallowed whole. The pass cap bounds the work on
pathological input.

One `E201` is kept per bracket, keyed on the `[` offset, preferring a
fix-bearing diagnostic over the fix-less fallback — so a bracket that was
actually recovered reports its insertion fix rather than the bare "unterminated"
fallback the re-lexed stream would otherwise yield. When no heuristic produces
an insertion, no re-lex happens at all and the caller keeps its own
scan-to-next stream.

## Codes

| Code | Condition |
|------|-----------|
| E201 | Unterminated command substitution — missing `]` |
| E202 | Unterminated double-quoted string — missing `"` |
| E203 | Unterminated braced word — missing `}` |
| E204 | Extra characters after the close brace of a `${name}` reference |
| E205 | Extra characters after the close quote in a variable name |
| E206 | Missing close brace for a `${name}` reference |
| E207 | Nesting depth exceeds the analysis limit |

E201–E203 drive recovery; E204–E206 are lexer warnings with nothing to inject.

### E201 heuristics, in priority order

1. **Comment break** — a `#` comment line follows; insert `]` at the end of the
   preceding line.
2. **Known-command break** — a line starting with a known command follows;
   insert `]` at the end of the preceding line.
3. **Brace break** — a `{` swallowed the remainder; insert `]` before it.
4. **Fallback** — no insertion; report the bare unterminated `[`.

The "known command" universe is the active registry's names *plus* every proc,
class, and alias the document itself defines
(`analyser::utils::recovery_known_commands`), so a break just before a call to a
user-defined proc recovers as readily as one before a builtin.

E202 uses the same known-command-on-the-next-line signal; E203 uses a
de-indented line starting with a known command.

## Semantic proposes, syntactic validates

The heuristics are *semantic*: they read the shape of the following text. They
can propose an offset that is not a real closing position — one that sits inside
a brace word, a quoted run, a backslash escape pair, a `${…}` variable-name
brace, or a command-substitution brace interior. A `]` inserted there is a
literal, and the command stays incomplete.

So every proposed fix is validated against the structural index before it is
accepted, and a proposal landing on an inert offset is vetoed; recovery falls
through to the next heuristic, then to the fix-less fallback. `is_inert` is the
*sound* direction — it never marks a structural position inert — so the veto can
only remove wrong fixes, never good ones.

Worked example:

```tcl
set x [foo {bar
puts baz}
```

`puts` is inside the balanced brace word `{bar … baz}`. The bare command-break
heuristic would insert `]` after `bar`, yielding `set x [foo {bar]…}`, which C
Tcl 9.0.3 `info complete` reports as `0` — objectively wrong. The veto rejects
that candidate; a later script-level position yields the end-insert
(`… baz}]`, complete). Plain-text recoveries such as `set x [foo bar` followed
by `puts done` are unaffected.

## The structural-state index

The index records the lexer's own delimiter state during a single scan, so the
question *"does inserting closer C at offset O balance the outermost open
delimiter?"* is answered with a prefix lookup and a sparse forward walk — no
re-lex.

Three dimensions exist (`rust/tcl-lexer/src/structural_index.rs`):

- **`BracketIndex`** — the script `[` / `]` sublanguage. This is the one wired
  into production, as the E201 inert veto.
- **`BraceIndex`** — the script `{` / `}` sublanguage.
- **`ExprParenIndex`** — the expr `(` / `)` sublanguage, built directly from the
  expr lexer's token stream so `$arr(idx)` is one `Variable` token and strings /
  command substitutions are whole tokens. Its grouping-paren count is therefore
  the lexer's own.

### Snapshot contents

Snapshot `(offset, bracket_level, brace_level, in_quote, bracket_delta)` at each
effective delimiter, plus **inert spans** for backslash escapes and `${…}`
variable-name braces. The absolute level at an offset is the prefix sum;
`bracket_delta ∈ {+1, 0, -1}` distinguishes an opener from a closer and from a
quote/brace toggle. The expr dimension adds `paren_level`, since parens nest in
expressions where they do not at script level.

```
if O in an inert span:                      return false   # closer is literal
state = last snapshot before O
if O inside a quote/brace where C is inert: return false
L = level(O)                                               # prefix sum
if L - 1 == 0:                              return true    # closes at O
# otherwise the insert shifts the running level by -1, so a *later* real
# closer can now balance the outer one — walk the sparse deltas forward to
# the first closer whose shifted level reaches 0 outside any inert span.
```

### The two contexts the scanner must mirror

A scalar bracket level is not enough. Whether a `[` or `]` is structural or
inert depends on the surrounding sublanguage, and Tcl has two different brace
rules:

- **Top level and quoted words** — `{` opens a brace word only at a word
  boundary; the whole verbatim `{…}` span is inert for brackets. `"` toggles a
  quoted run (brackets still count inside it; braces are literal).
- **Inside a `[…]` command substitution** — brace handling is *count-based*, not
  word-based (mirroring `Lexer::scan_command_substitution`): `{` and `}` adjust
  the brace level, and a `]` only closes when the brace level is 0 and the
  scanner is not in quotes. So a `]` inside `{…}` inside `[…]` is inert.

Both share the inert leaves: backslash-escape pairs and `${…}` variable-name
braces.

`info complete` parses a `[…]` interior recursively as a script — word-based
braces plus a terminal "extra characters after close-brace" error — so
`[set x {b}{` is *complete* even though the outer `[` is unterminated. The brace
scanner mirrors this: `BraceBuilder::scan_script` handles both the top level and
a command-sub interior (terminating at the matching `]`), and a terminal
extra-chars error propagates up through every enclosing scope.

### Two rules that are easy to get wrong

1. **An extra closer closes nothing.** A `)`, `]`, or `}` with nothing open must
   not drive the running level negative — clamp at 0, matching stack semantics.
2. **An unterminated opaque token reaching EOF makes the tail inert.** A
   `$arr(idx`, `[…`, or `"…` with no closer swallows to EOF, so a closer
   inserted after it is absorbed by *that* token, not by the outer delimiter.
   Extend such a token's inert span to EOF.

## Routing by `ArgRole`

The command registry's `ArgRole` classifies the argument an unterminated `{`
belongs to, which selects the sublanguage recovery should reason in:

- **`BODY`** — recover as a script (close before a de-indented known command).
- **`EXPR`** — recover as an expression, consulting the expr index. Expression
  content is structured, so a bare known-command word at the start of a
  following line is a strong forgotten-close signal even without a de-indent;
  `EXPR` braces can use the aggressive command-break that data braces cannot.
- **neither (data)** — keep the conservative de-indent heuristic.

The role is for **routing**, never for suppression. Do not gate recovery off for
data braces on the basis of the role: a de-indented known command is a strong
forgotten-close signal regardless of role.

```tcl
array set b32 {
  A 0 B 1
set totp_secret foo     ;# the `}` was forgotten; recovering here is correct
```

## Contract

Recovery behaviour is pinned by implementation-agnostic black-box tests
(`editors/vscode/src/test/errorRecovery.test.ts` and the server e2e suite)
asserting six contracts:

1. Unterminated delimiters are flagged.
2. Recovery is non-fatal — a proc after the break is still a document symbol.
3. The recovered token stream is well-formed and re-lexes the tail, including
   for pathological and deeply nested input, without hanging.
4. No duplicate diagnostics. The first parse and the recovered re-parse can emit
   the same warning, so diagnostics are assembled once and de-duplicated; the
   token path and the analyser path share one detection so they surface
   byte-identical diagnostics.
5. Edits toggle recovery.
6. Well-formed code is clean.

Language-internal soundness is pinned by a differential oracle against a real
re-lex and by the recovery fuzz campaign.

## Adding a heuristic

- If the pipeline mishandles malformed input, check whether `syntax_checks.rs`
  has a heuristic for that specific delimiter pattern.
- A new heuristic proposes an insertion offset as its diagnostic's fix; the
  structural-index veto validates it. Never insert mid-token.
- Ghost insertions are zero-width, so they must not change any later offset.

## Related docs

- [Example 20 in walkthroughs](../example-script-walkthroughs.md#example-20-error-recovery--unclosed-bracket)
- [green-token-tree.md](green-token-tree.md) — the lossless tree the recovered
  parse feeds.
- [GLOSSARY.md](../../GLOSSARY.md)
