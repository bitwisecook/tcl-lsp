# Luna-23 review of Sol-19 `lone-cr-document-channel-v2.patch`

Repository: `/home/user/tcl-lsp`, branch `work`, HEAD `bfa47846e` (source identical to `9cc6b4089`). No cargo was run and the repository was not modified. The main index was not touched: the apply check used a temporary `GIT_INDEX_FILE`, which has since been deleted. The other agent's uncommitted files (`command_binding*.rs`, `body_effects.rs`) do not overlap with the patch.

## Verdict: ADMIT-WITH-CONDITIONS

v2 meets Luna-21's conditions 1–3:
- The lone-CR translation is now gated by grammar data.
- Jim keeps HEAD behaviour.
- The assertion corrections are folded in verbatim.
- The contract is narrowed to the lexer and its token consumers.

I found nothing that would fail to compile or silently get the wrong value. The remaining conditions are narrow fixes to text and tests, plus the validation that is still owed.

### Conditions

1. **Fix a wrong oracle claim in a new test comment.** The test is `source_map.rs` `native_value_and_document_lone_cr_have_distinct_command_boundaries`. Its comment says "`eval` of the same bytes is one seven-word `proc` command ("wrong # args")". For the bytes it actually uses, `# c\rproc t {a b} {}\rt x value\r`, `eval` on tclsh 8.4–9.1 is one comment: nothing runs and the return code is 0 (`probe/tcl.out`, `srcmap.tcl`). The seven-word / wrong-args result holds only without the leading `# c\r` (`proccase.tcl`). The assertion `group_commands_bytes(&n).is_empty()` is correct; only the comment is wrong. v1 carried the same text.
2. **Update the other exhaustive copy of the axis vocabulary.** `docs/design/registry/dialect-and-package-registry-redesign.md:856` still lists the closed axis vocabulary with 14 names. Add `source_line_endings`. No gate covers this file, which is why it was missed.
3. **`dialect-profile-model.md` struct sketch.** The new field sits under the `// ---- the five JimTcl lexical axes ----` header, so that header now heads six fields. The prose below calls it "a sixth Jim difference", which "describes the file reader rather than the parser". Either give the field its own group or comment, or relabel the header.
4. **Stale claim in a test that now runs vacuously.** In `lexer.rs`, `vertical_tab_is_a_separator_only_in_tcl` says "The four shared separators behave identically either way" and loops over `"\r"` with `.any(Sep)`. Under v2 a Document `\r` differs between the two configs: it is a `Sep` for Jim (`Preserve`) and an `Eol` for Tcl (`TranslateAuto`). The test passes only because of the spaces in `set x `. Fix: drop `"\r"` from that loop and assert the CR token's own kind for each config. That also gives a lexer-level pin for Jim's `Preserve`. AGENTS.md's stale-claims rule applies to test comments.
5. **"Tracked obligation" needs a location.** The contract paragraph says the following are tracked obligations:
   - moving the eight raw-byte scanners onto `script_byte`
   - `^Z` end of file
   - per-dialect normalisation by the issuer
   - gating Jim backslash-CR

   Name where they are tracked, either an issue or the active coordination doc, and record them there at commit time. Otherwise, phrase the paragraph as a current limitation.
6. **Owed validation.** None of the following has been run by Sol-19, Luna-21 or me:
   - `cargo test -p tcl-dialect -p tcl-lexer -p tcl-spectcl`
   - the `tcl-registry` spectcl tests
   - the `tcl-lsp-core`, `tcl-compiler` and `tcl-lsp-server` suites
   - `make rust-check`: clippy pedantic, plus the dialect-drift, segmentation-drift and owner-resolution gates

Optional: `docs/design/compiler/lexing-segmentation.md` is the lexer's design doc. It could gain one sentence saying that a Document's lone CR is an `Eol` for command structure under `TranslateAuto` grammars, while positions still break only on `\n`.

## 1. The `SourceLineEndings` axis and where it is built

**Values:**

| grammar | `source_line_endings` |
|---|---|
| `GRAMMAR_TCL84`, `GRAMMAR_TCL9X` (`family.rs`) | `TranslateAuto` |
| `GRAMMAR_TCL85` and `GRAMMAR_TCL86` (struct updates of 8.4) | `TranslateAuto`, inherited |
| `GRAMMAR_F5_TCL` (`..GRAMMAR_TCL84`) and `GRAMMAR_IRULES` (`= F5_TCL`) | `TranslateAuto`; the no-oracle note is present in both `family.rs` and `profile.rs` |
| `GRAMMAR_JIM` | `Preserve` |
| `JIM_0_80`, `JIM_0_81` | `Preserve`, inherited |
| `profile.rs` catalogue: TCL84, TCL85, TCL9X, F5 | `TranslateAuto` (TCL86 is an update) |
| `LexerGrammar::default()`, `LexerConfig::default()` | `TranslateAuto` |
| SpecTcl `to_grammar` | default `TranslateAuto` |

The Jim ladder still has exactly three distinct grammars, and `grammar_values_match_the_old_catalogue` still holds: both catalogues are updated identically.

**`LexerConfig` carriage:**
- `from_grammar` and `grammar_over` set the field explicitly.
- `with_grammar` (`..Self::from_grammar`) and `normalized` (`..self`) carry it.
- `for_dialect`, `for_profile` and `for_file_grammar` all route through `from_grammar`.

**Gated read:** `Lexer::script_byte` → `channel().script_byte(config.source_line_endings, …)`. The function translates only when the channel is Document, the axis says it translates, and the CR is lone.

**Construction sites.** I scanned the 202 files that contain `LexerConfig {` or `LexerGrammar {`, using a script in `tools/literals2.py`. The only struct literals without a `..` update were:
- the 7 grammar constants
- `dialect_block::to_grammar`
- the two `Default` impls, which use `Self {`

The patch updates all of them. Every other site is a struct update, so it compiles and inherits from its base:
- `dynamic.rs:513`
- `expr/parser.rs:685`
- `cmd-core/index.rs:937`
- `cli/graphs.rs:523`
- `expr_lexer.rs:1461`
- `semantic_tokens/original.rs:3054`
- `minify/original.rs:1797`
- `word_parts/cursor.rs:860`

No exhaustive destructuring exists.

**Derives, serde and snapshots.**
- Both structs derive `PartialEq`, `Eq` and `Hash`, and so does the new enum. There is no serde.
- No Debug snapshot or golden file contains a grammar.
- No generated catalogue embeds the `dialect` hover.

**The Jim roster guard.** `jim_script_roster` checks `with_grammar(jim) != config`. That guard now also refuses a hand-built Jim-shaped config that carries `TranslateAuto`, so it fails closed. Every production Jim config comes from `from_grammar` or `for_file_grammar`. The only hand-built Jim-ish configs are in tests, and none of them feed a lone CR.

## 2. SpecTcl vocabulary and the issue-2070 gates (15 axes)

- **`axes_lists_every_axis_to_grammar_reads`.** `to_grammar`'s body has 11 `self.axis("…")` reads and 4 `flag("…")` reads, which is 15. `AXES` has 15 entries. `self.axis(axis)` inside the `flag` closure has no quote, so it does not match the marker. **Passes.**
- **Closed vocabulary.** `translate-auto|preserve` are both in `AXES`, and other values already use hyphens. **Passes.**
- **Distinct grammars.** `TranslateAuto` and `Preserve` produce different grammars. **Passes.**
- **Jim classification.** With `("source_line_endings", "preserve")` added, the block equals `GRAMMAR_JIM`. Without that row it would fail, and v2 adds it. **Passes.**
- **Registry hover.** The `blocks.rs` hover lists `` `source_line_endings` ``. **Passes.**
- **`loader.rs` tests at 12653 and 12703.** These are a novel-dialect block and an 8.5-duplicate block. Both are unaffected because the default is `TranslateAuto`.

**Other exhaustive axis lists:**
- In code: every file that enumerates three or more Jim axes is in the patch.
- In docs: there are only two such files, `dialect-profile-model.md` (updated) and `dialect-and-package-registry-redesign.md:856` (**not updated**, condition 2). The `dialect` hover has no KCS or generated copy.
- No studio, wasm or editor code reads the grammar axes.

## 3. The Jim test and the folded corrections, checked against the interpreters

My own probes are in `probe/`: the outputs `tcl.out` and `jim.out`, and the drivers `drive.tcl` and `drive2.tcl`.

| case | tclsh 8.4.20 / 8.5.19 / 8.6.18 / 9.0.4 / 9.1.0 | jimsh 0.84 |
|---|---|---|
| `#\r{\nputs reached\n` | `source`: "missing close-brace"; `eval`: prints `reached`; `info complete` 1 | `source` and `eval` both print `reached`; complete 1 |
| `proc t {a b} {…}\rt x value\r` | `source` runs it; `eval` gives "wrong # args" | both give "wrong # args" (as the `grammar.rs` doc claims) |
| `# c\rproc t {a b} {}\rt x value\r` | `source` runs it; `eval` is one comment, rc 0 | — |
| `strng is\rNEXT`, `C\t\rNEXT`, `foo\rbar` | `source` gives two commands; `eval` gives one | — |

**`Jim_EvalFile` source.** `jim.c:12130` is `fopen(filename, "rt")` followed by `fread`, as cited.

**The new test, `a_lone_cr_ends_a_document_comment_only_where_source_translates_it`:**

| image | expected | matches oracle |
|---|---|---|
| Jim Document | true | yes |
| Jim native | true | yes |
| Tcl 8.6 Document | false | yes |
| Tcl 8.6 native | true | yes |

The test comment names only 8.6 and 9.0, but all five Tcl versions agree.

**Folded corrections.** All 31 changed lines of `luna-21/assertion-corrections.patch` appear verbatim in v2:
- `cr_is_separator_not_eol`: native `[Esc,Sep,Esc,Eol]` and Document `[Esc,Eol,Esc,Eol]`.
- `original_context` `("\r", None)`: single dialect `tcl8.6`, so no Jim row is affected.
- `original_oo` `("\t\r", false)`: also single dialect `tcl8.6`.

All three match `source`.

## 4. Does the studio need a GAPS entry?

No.
- The parity rule in `command-spec-studio.md` is about `CommandSpec` fields and descriptors. `LexerGrammar` is not one of them.
- The studio has no form or draft key for `dialect` blocks. In `tcl-spec-studio`, "axis" means only effect axes.
- `GAPS`' `PackLevel` entries cover only `environment` rows. None of the 14 existing dialect axes has an entry.
- The SpecTcl loader can now spell the new axis, so the DSL has parity.

Adding a per-axis GAPS row would be inconsistent with the existing register. Sol-19's reasoning is correct.

## 5. Mechanics

- **Apply check.** I ran `GIT_INDEX_FILE=<scratch> git read-tree HEAD && git apply --check --cached` against HEAD `bfa47846e`: rc=0. Numstat matches the stated 14 files and +328/−24.
- **rustfmt.** I rebuilt the patched copies from `git show HEAD:` plus `patch -p1` (in `b/`). `rustfmt --edition 2024 --emit stdout` on each of the 12 `.rs` files gives byte-identical output.
- **No new `#[allow]`.**
- **Gate surfaces.**
  - **dialect-drift.** The new `LexerConfig::default()` uses are in `tcl-lexer`, which is sanctioned, or in `#[cfg(test)]` modules.
  - **segmentation-drift.** `script_byte` has no `\n | ;` alternation.
  - **`differential_boundaries` and its corpora.** This test compares the channel-less `command_boundaries` with the lexer owner. I scanned `samples/`, `tests/` and the `tmp/tcl*`/`tk*` trees: they contain no lone CR, so the corpora cannot diverge. tcllib is not present locally, but it feeds only the ignored deep tier.
- **Name collisions.** `SourceLineEndings` and `source_line_endings` do not exist anywhere at HEAD.

## Residual risks (non-blocking, already documented by Sol-19 or Luna-21)

- The eight raw-byte scanners have no channel; they are correct only behind the normalising boundary.
- A region sliced between the CR and LF of a CRLF.
- `&str` Document-default entry points given computed values.
- Jim backslash-CR continuations, a pre-existing infidelity.
- F5 is inherited, not measured.
- `script_image_is_complete` dispatches on `var_syntax == Jim`. That is pre-existing and not introduced here.
