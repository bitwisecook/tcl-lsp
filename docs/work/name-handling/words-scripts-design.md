# Scripts, words, lists and strings: reading has owners, writing does not

How Tcl script, command, word, list and string syntax is read, split, joined and written across execution and analysis, measured against the [parser plan](parser-crate-plan.md) and the `segmentation-drift` gate. Part of [tidying the architecture](architecture-tidy-plan.md).

**Verdict:** reading has largely converged: script segmentation, completeness, substitution and list reading have owners, and three of the four segmenters the `segmentation-drift` gate names now delegate to `tcl_lexer::script::group_commands`. What remains is the half the plan does not inventory: **writers** (code that builds Tcl source text), small **private re-splitters** in analysis code, and **release-blind copies** of escape, `concat` and character-count rules. The parser census and the drift gate cannot see most of it.

## Measured behaviour

The 19 fixtures in [`oo-dispatch-probe/words-scripts/`](handover/oo-dispatch-probe/words-scripts/) cover comments (only at command start; a brace inside a comment still counts, so `# }` in a procedure body ends the body), backslash-newline continuation in brackets, braces and quotes, `{*}` expansion, characters after a closing brace or quote, escapes by release, variable-name boundaries (`$a(k)`, `${a b}`, `${x}y`, `$x::`), nested substitution, malformed lists, list quoting of special strings, `split`/`join`/`concat`, `subst` flags, `info complete`, astral characters, `\x` width by release, dictionary string forms, semicolons and braces in quotes. Release-dependent results: `\x414` is one character in 8.4 (unlimited hex digits) and `A4` from 8.6; `\U0001F600` is U+FFFD in 8.6 and one code point in 9.0; a string holding it has 3 characters in both but 5 UTF-8 bytes in 8.6 and 6 in 9.0; 8.4 counts `\U0001F600` as a literal `U` followed by text.

**Execution:** of 38 runs (8.6 and 9.0), 33 match tclsh in both engines; the runtime's remaining differences are its runner's error formatting. Two faults:

- **The VM fails on `{*}` applied to a literal braced word** in some compile contexts: `puts [list {*}{a b}]` fails with `native compiler preflight provider required` (inside `catch {…}` it works), and **in file mode the VM exits with status 1 and prints nothing**, so the error is lost. The Tcl 9.0 library's `msgcat.tcl` uses `{*}{…}` inside `namespace eval` (line 55), which is the cause of the silent exit recorded in [packages](package-design.md).
- **Script files are read as UTF-8 in 8.6 mode.** Tcl 8.6 reads source files in the system encoding (iso8859-1 in this container, so a UTF-8 `é` is two characters); 9.0 reads UTF-8. Both engines use UTF-8 for 8.6 too. Environment-dependent, but a real release difference.

**Analysis:** clean on all fixtures except W102 ("`subst` with a variable argument enables code injection") on `subst {…}` with a literal braced template, which is not a variable, including with `-nocommands`, where it still warns about `[`. The genuine error in the brace-in-comment fixture is reported (E102, E204).

## Reading

- **Owners:** `group_commands`/`group_commands_bytes`, `NativeWord` and `first_parse_cut` in `tcl-lexer`; configured completeness (`script_image_is_complete`) used by `info complete` in both engines; substitution in `tcl-lexer/src/substitution.rs` with per-release `EscapeSyntax`; list reading and writing in `tcl-value-syntax`.
- **Delegating now:** the CST builder, the compiler segmenter (pinned by differential tests), the runtime's parser and its 8.4 compiler.
- **Still separate:** the dialect-blind byte scanner in `structural_index.rs` (`script_is_complete`, `command_boundaries`, `reparse_window`; the last has no production caller, the second serves only the explorer under every dialect); the F5 `if … \n else` merge, which lives only in the compiler segmenter, so the formatter, minifier, optimiser and runtime split it differently under iRules (inferred); private groupers in the formatter, `value_shapes.rs`, `expr_context.rs` and optimiser span helpers (waived), and `analyser/diagnostics/usage/unbraced_words.rs:102` (not waived, missed by the gate's spelling list); the 8.4 expression parser's own brace matcher and backslash width.

## Release-blind copies

- **Escape widths:** `tcl-compiler/src/rendered_properties.rs` (`escape_byte_len`, *read*) hard-codes 8.6+ widths (`\x` two digits, `\U` an escape) and feeds taint; 8.4 reads `\x` with unlimited digits and 8.4/8.5 treat `\U` as a literal `U`.
- **`concat`:** the owner trims ASCII whitespace only; codegen's `concat` fold uses Rust's Unicode `trim()` (`tcl-codegen/src/codegen/emitter/bytecoded.rs`, *read*), so a literal non-breaking space is folded away where Tcl keeps it (`string length [concat " x "]` is 3); `deferred_writes.rs` lacks the trailing-backslash rule.
- **Character counts:** `.chars().count()` with no release policy in the registry's value transfers, the VM's `char_find`, `tcl-cmd-core`'s `justify` and the VM's `string` command, while the runtime selects a counter by protocol version.
- **Comments:** the registry's `strip_comment_lines` decides comments line by line, missing continued comment lines and dropping `#` lines inside braced data.
- `subst` options are read in three places (registry, VM, runtime), not checked for agreement.

## Writers

Shared writers exist but are fragmented: `list_element`/`join_list` (value syntax), `lexical_*_source_word` (word rules), `native_literal_source_word` and `quote_literal_source_content` (backslash), `native_command_source_word` (naming), `pack_verbatim_source_word` (SpecTcl), with no single writer that takes a purpose as the plan's contract describes. Outside them:

- **SslicTcl's `tcl_word` emits unbalanced Tcl** (`tcl-sslictcl/src/emit.rs:44-62`, *read*): any value without `{`, `}` or backslash-newline is braced, so `a\` becomes `{a\}`, which tclsh reports incomplete; `tcl_list` then wraps quoted elements in another pair of braces.
- **Spec Studio** writes brace-joined rows (`effects.rs`, `render_stub.rs`, `definition_grammar.rs`, `properties.rs`, `argument_rows.rs`) and falls back to unchecked braces when its own checked writer refuses (`tcl_words.rs`); `tcl-spectcl/src/upgrade.rs` likewise.
- **f5-xc** has no shared-writer call; its Terraform `quote` does not escape `${` (inferred to be needed).
- **Test generators** (`tcl-irule-test/src/sim.rs`, `tcl-mcp/src/irule_gen.rs`) join lists with spaces beside correct `list_element` calls; one path in `irule_gen.rs` writes and re-parses to verify, which is the right pattern.
- **Optimiser and codegen** brace-join words behind private safe-character gates (`is_safe_word`, `is_plain_literal`); inlining writes `${name}` without checking the name for `}` (inferred risk for unusual formal names).
- **The minifier** re-wraps minified bodies and expressions in braces without a balance or re-parse check, except on its lambda path.

No vector or property test checks `read(write(v)) == v` for any writer.

## Re-splitting in analysis

`split_whitespace` over Tcl lists and `[…]` contents (`analyser/handlers/interpreters.rs` interpreter path keys, `interprocedural/call_by_name.rs`, `classes/unknown_dispatch.rs`, codegen formals, LSP method receivers), line-based scanning in `per_item/body_pass.rs`, and unchecked `strip_prefix('{')`/`strip_suffix('}')` brace stripping in about a dozen places across the compiler, registry and LSP; `tcl-bigip`'s graph re-splits tmsh bodies on whitespace beside its own parser. Non-test counts of `split_whitespace` and brace stripping: compiler 22 and 22, registry 15 and 5, `tcl-bigip` 31 and 21, LSP 7 and 3.

## Coverage of the plan and gates

The parser plan's figures stand (7,176 review leads; all 22 grammar families open; 1,665 of 1,665 analyser-binding leads awaiting review; 1,940 of 1,996 tooling leads open). The census only finds functions *named* quote, escape or render, so brace-`format!` writers are invisible, and its host-primitive list omits `strip_prefix('{')`, `split(' ')`, `.join(" ")` and `.chars().count()`. The `segmentation-drift` gate matches four spellings and admits its blind spots; its header's "four implementations" is out of date.

## Plan

1. **Fix the VM:** `{*}` on literal braced words in every compile context, and report script-file errors instead of exiting silently (both behind the `msgcat` failure); decide and implement 8.6's system-encoding source reading.
2. **A writer owner that takes a purpose** (command word, list element, braced body, expression, quoted string) in `tcl-syntax`/`tcl-value-syntax`, with a `read(write(v)) == v` property test per dialect; move SslicTcl, Spec Studio, SpecTcl upgrade, f5-xc, the test generators, the optimiser, codegen inlining and the minifier onto it; fix SslicTcl's unbalanced output first.
3. **Release policy everywhere a copy exists:** escape widths from `EscapeSyntax` in taint, the owner's `concat` in codegen and deferred writes, a release-selected character counter for the registry, VM and `tcl-cmd-core`; comment detection from the lexer in the registry.
4. **One segmentation, including dialect rules:** move the F5 `else` merge into the owner so every consumer sees it; retire `structural_index`'s unused or display-only paths; route the remaining private groupers through `CommandTokens`/`NativeWord`.
5. **Replace re-splitting** of Tcl lists and words in analysis with the list owner and the structured words of the [architecture plan](architecture-tidy-plan.md); stop unchecked brace stripping.
6. **Extend the census and gates:** add writers (`format!`/`join`/`push_str` producing Tcl source), brace stripping, whitespace splitting and character counting to the parser census and the drift gates; refresh `segmentation-drift`'s description; fix W102's literal-template false positive.
