# Sol-19 v3: Luna-23's conditions

Patch: `lone-cr-document-channel-v3.patch`, made with `diff -ruN a b` from `v3/`. It changes 16 files, +349/−27. It is v2 plus the items below.

Checks:
- `git apply --check --cached` against a temporary `GIT_INDEX_FILE` read from `6f01dc10c` returns 0.
- `rustfmt --check --edition 2024` reports no diffs on all 12 changed `.rs` files.
- The patch adds no `#[allow]`.
- No cargo was run, and `/home/user/tcl-lsp` was not modified.

Changes for each condition:
1. **`source_map.rs` test comment.** The comment in `native_value_and_document_lone_cr_have_distinct_command_boundaries` now states the correct `eval` oracle: the bytes form one comment, nothing runs, and the return code is 0. Under `source`, the comment ends at the first CR, then `proc` and the call run.
2. **`dialect-and-package-registry-redesign.md:856`.** The closed axis vocabulary now lists `source_line_endings`.
3. **`dialect-profile-model.md` struct sketch.** `source_line_endings` now has its own group header, `// ---- the script-file reader (Jim differs here too) ----`. The "five JimTcl lexical axes" header again covers exactly five fields.
4. **`lexer.rs` `vertical_tab_is_a_separator_only_in_tcl`.**
   - `\r` is removed from the shared loop, and the comment now says "three shared separators".
   - New asserts check the token kind of a lone CR in a Document for each config. The Jim config (`Preserve`) gives `(Sep, "\r")`, and the Tcl 8.6 config (`TranslateAuto`) gives `(Eol, "\r")`.
   - This is a lexer-level pin for Jim's `Preserve` value.
5. **Contract paragraph.** It is reworded as current limitations, so it no longer says these obligations are "tracked". It covers the eight raw-byte scanners plus three items: `^Z`, per-dialect issuer normalisation, and Jim backslash-CR gating. If the coordinator registers them in an issue or the coordination doc, the paragraph can point there.
6. **`lexing-segmentation.md` (optional).** One short paragraph was added after the line-tracking convention. Under `TranslateAuto`, a Document's lone CR is an `Eol` for command structure (through `SourceChannel::script_byte`), and positions still break only on `\n`.

The predicted test flips and residual risks are unchanged from `NOTES-v2.md`. `vertical_tab_is_a_separator_only_in_tcl` is not a flip: its new asserts are expected to pass under the patch.
