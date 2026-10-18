# Luna-21 review of Sol-19 `lone-cr-document-channel.patch`

Repository: `/home/user/tcl-lsp`, branch `work`, HEAD `9cc6b4089`. Sol-19 worked from `6de8fce66`. The only commit between the two adds a JSON report, so the patch still applies to HEAD (`git apply --check --cached` against a temporary index passes). The repository was not modified, and no cargo was run.

## Verdict: ADMIT-WITH-CONDITIONS

The lexer change does what tclsh `source` does for C Tcl 8.4–9.1. It is internally consistent, and it does not move any offset, span, line or UTF-16 position. It does have one regression outside C Tcl: Jim documents. It also leaves a contract claim that is stronger than the code. The conditions below fix both.

### Conditions

1. **Gate the translation by grammar data in the same admission (the lexer half of Sol-19's step 4).** `Lexer::script_byte` must read a lone CR as LF only when the selected `LexerGrammar` says the dialect's `source` channel uses `-translation auto`. Proposed design:
   - Add an axis such as `source_line_endings: {TranslateAuto, Preserve}`.
   - Values: `GRAMMAR_TCL84` and its 8.5–9.1 heirs = `TranslateAuto`. `GRAMMAR_JIM` = `Preserve`. The F5 trunk inherits `TranslateAuto` from `GRAMMAR_TCL84`; document that inheritance as not decided by any oracle.
   - Carry the axis in `LexerConfig` (`from_grammar`, `with_grammar`, `grammar_over`).
   - Mirror it in the SpecTcl dialect-block vocabulary and the studio form, or add a `GAPS` entry, as AGENTS.md's parity rule requires.
   - Add a Jim regression test pinned to jimsh evidence (below).
   - The issuer half of step 4 can follow later: per-dialect boundary normalisation, and Jim documents as NativeValue.
2. **Apply `assertion-corrections.patch`** (this directory). It covers the three predicted flips, and each correction is a genuine tclsh `source` contradiction.
3. **Narrow the contract text, or land step 2.** The new paragraph in `shared-utility-contracts-rust.md` and the `line_index.rs` docs must not suggest that all structural owners agree. They should name the scanners that still have no channel and stay correct only on text normalised at the boundary:
   - `structural_index` `BracketIndex` and `BraceIndex`
   - `ranges::command_subst_close` and `word_parts::command_subst_close`
   - the `Grouper::close` blank-line count
   - segmenter recovery and `single_newline_gap`
   - the E201 line heuristics
   - the line filter in `comment_line_starts`
   - the line-keyed suppressions

   Record them as tracked follow-up obligations (Sol-19 step 2), not as completeness.
4. **Owed validation, unchanged from Sol-19.** Run `cargo test -p tcl-lexer`, the `tcl-lsp-core`, `tcl-compiler` and `tcl-lsp-server` suites, and `make rust-check`. None of these were executed by Sol-19 or by Luna-21.

## 1. Independent probes

The probe is my own script, `probe/luna_probe.tcl`. It has 22 cases, and each case is run under `source` and under `eval` of the same bytes read with `-translation binary`. Outputs are in `probe/out{8.4,8.5,8.6,9.0,9.1}.txt`.

The `source` column is identical on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0. In the `eval` column, 8.4 differs from the other versions only in the `set` wrong-args message text.

| claim | evidence | result |
|---|---|---|
| A lone CR ends a command under `source` | `cmd`: `{a b}` vs eval `{a note b}` | confirmed |
| A lone CR ends a comment | `comment`: `after` vs eval empty | confirmed |
| … inside `[...]` | `bracket`: `2` vs eval wrong args | confirmed |
| … inside a proc body (value translated) | `body_cr`: `in1 in2` | confirmed |
| CRLF behaves like LF, and CR CR LF also gives two lines | `crlf`, `crcrlf`, `body_crlf` | confirmed |
| Backslash-CR and backslash-CRLF are continuations | `bscr` 2 / `bscrlf` 2 (eval 1 / error) | confirmed |
| A backslash-CR inside a comment continues the comment | `comment_bs`: `shown` | confirmed |
| Braced and quoted CR and CRLF become LF in the value | `brace_val` `{3 1}`, `crlf_brace` 3 (eval 4), `quote_val` 1 | confirmed |
| `^Z` ends the script, including inside an open word | `eof_top`, `eof_brace` `missing close-brace`, `eof_comment` | confirmed (step 3, not in this patch) |
| A CR after a close brace or quote, `;\r`, or `{*}\r` is a terminator | `brace_close`, `quote_close`, `semi_cr`, `expand_cr` | confirmed |

The jimsh 0.84 control is in `probe/jim_control.out`:
- A lone CR is whitespace: `cmd` gives `{a note b}`.
- A comment runs to LF.
- Backslash-CR is not a continuation (`bscr` gives 1).
- Backslash-CRLF is an error.
- A CRLF inside braces keeps the CR (`crlf_brace` gives 4).
- There is no `^Z` end of file.

Sol-19's Jim claims are confirmed.

The completion fixtures are in `probe/fixture_probe.out`. For `strng is\rNEXT` and `C\t\rNEXT`, all five tclsh versions show that `source` ends the first command at the CR, while `eval` keeps one command.

## 2. Soundness of `script_byte` in the lexer

**Sound:**
- `current_byte` (and through it `current_char`) and `peek_byte` both route through `script_byte`. Ghost delimiters are still checked first, so current and peek reads stay consistent.
- `newline_continues_command` now looks ahead through `script_byte`, which fixes a raw read that would otherwise disagree.
- Each of these then sees a Document's lone CR as `\n`:
  - top-level dispatch, through `is_eol_char` and the F5 `'\n'` arm
  - `parse_sep` and `parse_eol`
  - `parse_esc`'s word-end test
  - `parse_comment` and `advance_command_comment`
  - the `[...]` scanner's `'\n' | ';'` arm
  - the extra-characters-after-close-brace and close-quote checks
  - the `{*}` look-ahead
- For all of the above the result matches the `source` column. The brace and quote content scanners treat `\n` as plain content, so nothing changes there.
- **CRLF:** the CR of a pair is unchanged. It is a SEP before an LF EOL, which is the same token stream as HEAD.
- **CR CR LF:** the first CR becomes EOL and the second CR and the LF join the same EOL run. That is one EOL token, the same command structure as tclsh's two line ends. `Grouper::close` counts raw `\n`, so it misses the blank line (step 2).
- **Backslash handling:** `source_backslash_escape_end` and `source_is_line_continuation` read raw bytes under the channel and already handle backslash-CR and backslash-CRLF for a Document, which agrees with `bscr`, `bscrlf` and `comment_bs`. Continuations consume only spaces and tabs, as `TclParseBackslash` does.
- **F5 N1 rule:** a lone CR followed by `{` now continues the command. HEAD gave the same structure, because the CR was a SEP before `{`.
- **Offsets:** the change is one byte for one byte. Spans, `LineIndex` (`\n`-only) and UTF-16 positions are unchanged. Values still come from `token_bytes` and `source_literal_bytes`, never from `script_byte`.
- **Raw reads left:** these do not make structural decisions.
  - `parse_esc`'s `get(pos+1).is_some()` is an existence test.
  - `scan_array_index_in`, `skip_braced_var_name_body` and `skip_jim_paren_body` are content scanners.
  - The BOM test reads raw bytes.
  - The Jim roster `line_delta` counts only `\n`.

**Residual hazard (not blocking):** `native_script_words::local_region_tokens` slices the image's bytes and keeps the channel. A region that ends between the CR and LF of a CRLF therefore reads that CR as lone and gives a trailing EOL instead of a trailing SEP. The command structure is the same. EOL-presence predicates such as `codegen::cmd_subst::has_command_separator` would answer `true`, but they fail closed into runtime `eval`. LSP cursors cannot sit inside a CRLF. Suggested hardening for later: give `SourceMap` the byte after the region.

**Out of the lexer, worth tracking:**
- `codegen::cmd_subst::has_command_separator` (Document image) now sends `[a\rb]` in a compiled Document to the runtime as a literal plus `EVAL_STK`. The runtime evaluates that value with CR as whitespace. Execution was wrong against tclsh `source` before the patch and is still wrong after it. Script fragments that move from a Document into a runtime value are not translated, and this is a pre-existing gap.
- VM host `eval_source` and `compile_service::compile_target_with` use Document images. They are host or source entries, and runtime value paths use NativeValue bytes (`native_script_source_image`, `compile_procedure_bytes_with_entry`), so no runtime `eval` regression was found.

## 3. The Jim dialect

The lexer can tell the dialect today: `LexerConfig` is built from the per-dialect `LexerGrammar` and already carries Jim-only axes (`var_syntax`, `quote_termination`, `word_separators`). There is no source-channel axis yet. `script_skips_leading_bom` → `leading_bom` is the precedent for a grammar axis that describes how a script file is read.

The patch applies C Tcl `source` translation to **every** Document, including Jim. `jim_script_tokens` and `jim_subst_tokens` refuse Document input, but two entry points do not:
- `jim_script_syntax_tokens`, used by `command_binding.rs:8465` for the Jim Direct WholeScript parse
- `jim_script_is_complete`, which backs `script_image_is_complete` for Jim

Under the JimNative protocol, `parse_sep`'s `is_ascii_whitespace() && byte != b'\n'` now stops at a lone CR, and `parse_comment` stops there too.

**Failing scenario.** The source text is the bytes `#\r{\nputs reached\n` under the Jim grammar.

| | result |
|---|---|
| jimsh 0.84 `source` | succeeds and prints `reached` (`probe/jim_regression.out`) |
| HEAD | the comment runs to LF, so `missing = None`, matching jimsh |
| Patched | the comment ends at the CR, and `{` opens to EOF, so `missing = Some(b'{')` |

With the patch, `script_image_is_complete` changes from true to false. The Jim WholeScript Direct path records the `source` as an Error completion, which disagrees with jimsh. tclsh 8.6 and 9.0 really do fail here with `missing close-brace`, so the new rule is right only for C Tcl.

No existing Jim test flips, because there is no Jim lone-CR fixture. `vertical_tab_is_a_separator_only_in_tcl` still passes for `"\r"` only because the spaces in `set x ` already produce `Sep` tokens. That passing test is not evidence that Jim is unaffected.

Product boundaries are unchanged for Jim: the server (`normalised_for_analysis`, `provider_source`), the CLI (`analysis_source`) and MCP already normalise lone CR for every dialect. That is a pre-existing Jim infidelity. The regression is limited to Core entry points given raw text, but it moves Jim away from its oracle. Under "no absence-as-completeness", step 4's lexer gate should land in the same admission (condition 1).

A follow-up should decide whether the same axis also gates `source_backslash_continuation_end`. That function already applies backslash-CR and backslash-CRLF continuation to Jim Documents, which jimsh does not (`bscr` 1, `bscrlf` error). The infidelity is pre-existing, and fixing it would change current Jim behaviour.

## 4. The three predicted assertion flips

The flips are genuine contradictions of tclsh `source`, and each fixture is C Tcl document analysis. The corrections are in `assertion-corrections.patch`, which passes `git apply --check` on HEAD, on HEAD plus Sol-19's patch, and on the current worktree.

1. **`tcl-lexer lexer::tests::cr_is_separator_not_eol`.** `Lexed::run` → `Lexer::new` → Document. The asserted `[Esc, Sep, Esc, Eol]` is tclsh's **`eval`** behaviour, so the expectation is right for a native value. The correction keeps the assertion and moves it to `Lexer::with_bytes` (NativeValue). It also adds the Document expectation `[Esc, Eol, Esc, Eol]` (`probe` case `cmd`).
2. **`completion::original_context` `("\r", Some(1))` → `("\r", None)`.** The fixture is `string is\r` analysed as `tcl8.6` Document text. Under `source` the CR ends the command (`fixture_probe` `str_is_cr`), so the cursor starts a new command, exactly as in the `"\n"` row. There is no NativeValue route, because completion only works on documents.
3. **`original_oo` `("\t\r", true)` → `("\t\r", false)`.** The fixture is `oo::class create C {}; C\t\r` under `tcl8.6`. `fixture_probe` `oo_cr` shows `C` running as its own command. Both assertions (`class_completion_context` and `method_selector_replacement`) take the `"\n"` row's value.

None of the three is a Jim or native-value fixture that should keep its old expectation unchanged. Fixture 1 keeps its expectation, but on the correct channel.

## 5. Other tests that could flip

I searched all lone-CR literals in `rust/` (the regex `"[^"]*\\r(?!\\n)[^"]*"`, which matched 106 files) and the computed `replace('\n', "\r")` fixtures. No other expectation flip was found.

| test | why it does not flip |
|---|---|
| `lexer.rs` `vertical_tab_is_a_separator_only_in_tcl` | Still passes, but vacuously for `"\r"`. Its comment "behave identically either way" is now false for Document CR, and becomes Tcl-versus-Jim different once condition 1 lands. Recommendation: check the `ws` token's own kind, not `.any(Sep)`. |
| `source_map.rs` `document_projection_*` (`{α\r\nβ}\rset γ 3`, `set value 1\r`) | Projection only; no lexing. |
| `parse_lexer.rs` `t1_4` | Asserts only that the result is non-empty. |
| `script_completeness.rs` | The `lone_cr` row is inside braces; `# c\\\r` is a backslash path. |
| `red.rs` and `line_index.rs` line models (including the fuzz test) | `LineIndex` is unchanged. |
| `ranges.rs` `end_position_bare_cr…` | Positions only. |
| `package_resolver/original_names.rs:1118` | Already one entry. |
| `folding.rs:1677` | Bounds only. |
| `diagnostic_report.rs:976`, `formatting/mod.rs:477`, `source_style`, `formatting_engine_residual` | Normalised or projected before lexing. |
| `minify.rs:4584`, `:5738` | Backslash-r escape or not lexed. |
| `fp/sty.rs:457`, `subst_nocommands`, `substitution.rs`, `lexer_depth.rs:721` | Backslash escapes or value paths. |
| `tcl-vm` `run_script.rs:475`, `cmd_string_e2e.rs:1068` (CR inside braces), `cmd_info.rs:853` (backslash path), `native_compilation_conformance.rs:74` (whitespace-only body), `value.rs` | No change. |
| `tcl-syntax` `backslash.rs` channel rows | Value decoding only. |
| `tcl-cli` `cli.rs` CR twins (including the issue977 multi-file test), `tcl-mcp` `tools.rs:3256`, `tcl-lsp-server` lone-CR tests | Normalised at the boundary, including disk provider sources. |
| `tcl-fuzz` | The generator emits no `\r`. |

Helper scanners that now disagree with the lexer on raw lone-CR Document text all fail closed or are only presentation. None is reached by product boundaries, because those normalise first.
- `structural_index` comment and command arms (e.g. `#c\r[` is inert for `BracketIndex` but a real `[` for the lexer)
- `ranges::command_subst_close` and `word_parts::command_subst_close`: `[# c\r]` is unterminated for the helper and closed for the lexer. In codegen, `.ok()?` makes this fall back.
- `Grouper::close` blank-line count: `\r\r` is not a blank line
- segmenter `find_recovery_offset` `split('\n')`, plus the E201 heuristic ghost placement. A ghost placed outside the lexer's `[...]` would be eaten by `parse_esc`: this is mis-recovery, not a panic.
- the `comment_line_starts` line filter
- the `\n`-keyed suppressions

These are Sol-19's step-2 list. Condition 3 requires the contract to say so.

Predicted, unexecuted fail→pass (Sol-19's targets): `signature_help` `signature_cursor_unicode_cr_and_utf16_positions_do_not_clamp` and `call_hierarchy::logical_source` `original_unicode_cr_geometry_…`. Both depend on the whole Analyser pipeline, including the helper scanners above, so treat them as likely, not certain.
