# Encodings, channels and how scripts are read

**Question:** do the engines, the lexer and the LSP read text the way tclsh does: `encoding convertto/convertfrom/names/system/profiles`, the Tcl 9 profiles and `-failindex`, channel configuration (`-encoding`, `-translation`, `-eofchar`, `-profile`, `-buffering`, `-blocking`), binary channels, `\u`/`\U`/`\x` escapes and lone surrogates, and script files with a BOM, CRLF, a lone CR, Ctrl-Z or invalid UTF-8? **Method:** 15 behaviour fixtures and 9 raw-byte script files in [`oo-dispatch-probe/encodings-channels/`](handover/oo-dispatch-probe/encodings-channels/), each run (by `run.sh`, from a fresh temporary directory, `LANG=C.UTF-8`) under tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0, Jim, the VM (`tclvm --tcl-version 8.6|9.0`) and the Rust runtime (`run_script --quiet --tcl-version 8.6|9.0`), with the BOM and Latin-1 files repeated under `LANG=C`; results in `engines.tsv`. The analyser and optimiser were asked about the same texts through MCP (`analyser-mcp.out`; a cached build that may lag `afe2f2d8`). The code was then read for every owner.

**Verdict:** the conversion commands are right and the reading of bytes is wrong. `encoding convertto/convertfrom`, the three profiles, `-failindex`, output translation and Tcl 9's strict output errors match tclsh in both engines, because both call one codec in `tcl-cmd-core`. Everything that turns *input bytes* into text does not use that codec: channel reads ignore `-encoding`, `-translation`, `-eofchar` and `-profile` and count bytes, and the three script readers (the VM's command line, the runtime's command line, and the shared `source`) skip none of the BOM, CRLF folding, Ctrl-Z or Tcl 9's strict decoding. The analysis side has a fourth reader (the LSP's), a dialect-only BOM rule that is wrong for 8.x under a UTF-8 locale, and no channel model at all. There is one crash and one silent exit.

## Measured

Wrong answers first; "VM" and "runtime" mean both releases unless a release is named.

| Case (fixture) | tclsh | VM | runtime |
| --- | --- | --- | --- |
| `read $f 2` on `éé` (e14) | `éé` | `é` (bytes counted) | `é` |
| `fconfigure -encoding iso8859-1`, read `éé` written as UTF-8 (e14) | 5 characters | 3 | 3 |
| `-translation binary` read of the same (e14) | 5 | 3 | 3 |
| read `a\r\nb\rc\nd` with `-translation auto/crlf/cr` (e07) | `a\nb\nc\nd`, `a\nb\rc\nd`, `a\n\nb\nc\nd` | raw bytes for every mode | raw bytes |
| `-eofchar \x1a` / `c` on input (e13) | `abc` / `ab` | `abcdef` | `abcdef` |
| read invalid UTF-8, 9.0 default and `-profile strict` (e09) | error | `U+FFFD`, no error | run refused |
| same, 8.6 (e09) | `U+00FF` | `U+FFFD` | `U+00FF` |
| `source` Latin-1 file, no `-encoding` (e10) | 8.6 `é`; 9.0 error | `U+FFFD` | `U+FFFD` |
| leading BOM, run or `source`d (s01, e10) | skipped (9.x always; 8.x when the system encoding is UTF-8, not under `LANG=C`) | `invalid command name "<U+FEFF>set"` | same |
| `set v {a<CR><LF>b}` in a file (s02, e10) | length 3 | 4 | 4 |
| lone CR between commands (s03) | runs | runs | `can't read "v"` |
| Ctrl-Z mid-file (s06, e10) | script ends | runs on (`invalid command name "␚puts"`) | same |
| invalid UTF-8 script file (s04, s05, s08) | 8.x runs (bytes as Latin-1); 9.x refuses | refuses on every release | runs on every release |
| `scan \uD800 %c` (e15, e05) | `55296` | **panic** (`value/string_rendering.rs:36`) | 8.6 right; 9.0 `65533` then refused |
| `scan \x00 %c`, 9.0 (e15) | `0` | `0` | `65533` |
| `encoding system nosuch` inside `catch` (e12) | error caught | exits status 1, no message | run refused |
| `encoding names`, 9.0 (e01) | 95 names incl. `cp1252`, `utf-16`, `utf-32` | 4; `cp1252` unknown | 4 |
| `encoding profiles`, 9.0 (e04) | `replace strict tcl8` | right | bad subcommand |
| `chan names`, `chan configure` (e11) | work | `invalid command name "chan"` | `names`, `pending`, `pipe` refused |
| `fconfigure $w -eofchar`, 8.x (e06) | `{}` (a two-element value) | empty | empty |

Right in both engines: `encoding convertto/convertfrom` for UTF-8, ISO 8859-1 and ASCII, Tcl 9's strict default for both directions and the `tcl8`/`replace` profiles, `-failindex` (e02–e04); output `-translation` (e07); Tcl 9's error on writing U+0100 to a binary or ASCII channel (e08); channel handles stored in lists and dicts and closed through an alias (e11, apart from `chan`); every `\u`, `\U`, `\x` and octal width rule on 8.6 and 9.0 (e05).

Analysis (`analyser-mcp.out`):

| Case | Report |
| --- | --- |
| leading BOM, dialect 8.6 | W123 `Unknown command '<U+FEFF>set'; did you mean 'lset'?` with a fix to `lset`: false under a UTF-8 system encoding, and the fix is wrong under any |
| `fconfigure -translation sideways`, `-encoding nosuch-enc`, `-profile bogus`, `-eofchar {x y}` on 9.0 | nothing (each is a runtime error) |
| `-profile`, `encoding profiles` on 8.6 | W004/W002, correct |
| `open $d` where `d` is `[file join [pwd] …]` | W103 pipeline warning, false (the value starts with `/`) |
| `puts` on a closed channel reached through an alias | nothing (no channel model) |
| `string length` of a word holding `\u` escapes | not folded under 8.6 or 9.0 (safe decline) |

## Census

**Codecs.** One version- and profile-aware UTF-8 codec, `tcl_cmd_core::encoding::convert_from/convert_to` (`rust/tcl-cmd-core/src/encoding.rs:410`, `:620`, decoder `:503`), serves both engines' `encoding` command and the registry folder. Beside it, text is decoded by:

- `tcl_cmd_core::channel::decode_text` (`channel.rs:129`), lossy UTF-8 with no version or profile, used by `source` (`encoding/original_names.rs:348`);
- the VM's `read`/`gets` (`rust/tcl-vm/src/cmd_chan.rs:359`, `:669`, `:682`), `String::from_utf8_lossy` whatever the channel's encoding, plus an unconditional trailing-CR strip in `gets` (`:354`);
- the runtime's `read`/`gets` (`runtime/rust/src/cmd_chan.rs:436`), raw bytes with no decoding;
- the VM command line (`rust/tcl-vm-cli/src/main.rs:388`, `read_to_string`, refusing any invalid UTF-8) and the runtime command line (`runtime/rust/examples/run_script.rs:174`, raw bytes), both outside `prepare_source_file`;
- the LSP's `decode_source` (`rust/tcl-lsp-core/src/source_decode.rs:243`, lossy plus a W107/W109 report) and `decode_source_as` (`:279`), which pins every declared encoding to `TclVersion::V8_6` and the `tcl8` profile (`:295–299`);
- `external_utf8_octets` (`encoding.rs:72`) for file names, and `NativeTclUtf` (`tcl-value-syntax`) for the internal representation.

That is seven readers of UTF-8 for one rule; only the first is release-aware.

**Encoding catalogues.** `SystemEncoding` (`rust/tcl-platform/src/lib.rs:197`, five members), `ChannelEncoding` (`channel.rs:54`, six, adding `binary` and `identity`), `implemented_encoding` and the name table (`encoding/original_names.rs`), the registry's `encoding` spec (`rust/tcl-registry/src/commands/tcl/encoding_.rs`), and `ScriptEncoding` for the LSP setting. Subcommand tables: the VM's (`rust/tcl-vm/src/command/encoding_command.rs:34–94`, with `profiles`), the runtime's (`runtime/rust/src/cmd_misc.rs:129`, without), and the registry's. `EncodingProfile` (`channel.rs:160`) is the single profile owner.

**Channels.** `ChannelConfig` (`channel.rs:305`) stores `input_translation`, `-eofchar` and `-profile`, but nothing applies them on input: `input_translation` is only parsed and reported (`channel.rs:380`, `:453`); `OutputTranslation` and `encode_output` (`channel.rs:197`, `:764`) are the only translation code. The engines keep separate channel tables and read paths; `tcl-runtime-api` has no channel trait. The VM registers no `chan` command. The registry's option values for `-translation`, `-profile` and `-eofchar` (`fconfigure_.rs:68–112`) are marked "completion/hover data only".

**Script text.** The release rules live as grammar data: `script_skips_leading_bom` (`rust/tcl-dialect/src/grammar.rs:317`; false for 8.x, `model/family.rs:484`, whose comment says 8.x "really does fail" — measured wrong under a UTF-8 system encoding, because `Tcl_FSEvalFileEx` compares the *decoded* first character, `tcl8.6.18/generic/tclIOUtil.c:1799`) and `source_line_endings` (`grammar.rs:342`). The lexer reads a lone CR as LF for a `Document` (`rust/tcl-lexer/src/source_map.rs:44`, `:76`), and CRLF folding inside values is in `tcl_syntax::backslash::source_literal_bytes`. Analysis uses the BOM flag (`tcl-compiler/src/signature_scan/mod.rs:827`, `tcl-cli/src/commands/compile.rs:212`); neither engine does. Ctrl-Z as end of script is modelled nowhere (no reference in the lexer, dialect, compiler or LSP). The VM honours the lone-CR rule (it lexes as a `Document`); the runtime does not.

**Escapes.** One decoder, `tcl_lexer::substitution::backslash_subst*` with `EscapeSyntax` per release, used by both engines (`runtime/rust/src/subst.rs` wraps it). It is right; the failures above are in value representation (the VM's lone-surrogate accessor, the runtime's 9.0 NUL).

**Diagnostics.** W107/W109 come only from `source_decode` and are dialect-blind: they describe a U+FFFD replacement, whereas tclsh 8.x reads those bytes as U+0080–U+00FF and tclsh 9.x refuses the file. W118 (line endings) and W108 (non-ASCII) are separate producers.

## Design

- **One script reader (L3).** A single owner turns script bytes into text plus a source map, given (release, system-encoding assumption, entry: command line, `source`, `source -encoding`). It applies, in C's order: the encoding through the shared codec with the release's profile (`tcl8` for 8.x, `strict` for 9.x), `-translation auto` (CRLF and lone CR), `-eofchar \x1a`, and the BOM check on the decoded first character. Both engines' command lines and `source` call it; the LSP calls it with the workspace's `tclLsp.sourceEncoding` assumption. The lexer then reads translated text and stops needing its one-byte CR substitution as a separate rule. The system encoding is an environment assumption, so the reader's answer carries a receipt for it; with no assumption, a leading BOM on 8.x is `Answer::Unknown` and W123 abstains (decision 7).
- **One input path for channels (value kernel).** Channel input becomes bytes → translation → eofchar → `convert_from(encoding, profile)`, counting characters, implemented once in `tcl-cmd-core` beside `encode_output`, behind a `Channels` trait in `tcl-runtime-api` (open, configure, read, gets, eof, close, names) that both engines implement over their own handle tables. `decode_text` and the engines' `from_utf8_lossy` reads are deleted.
- **Registry.** The channel and encoding options become value grammars with release gates (profiles from 9.0, the two-element `-eofchar` until 8.6, `binary` as an encoding name until 8.6), checked by the existing closed-value rule; `-failindex` and `regexp`-style output variables are variable-write effect descriptors; `encoding system` and `fconfigure` are effects on process state and on a channel instance.
- **Channels as instance values (L4).** `open`, `socket`, `chan create`, `chan pipe` and `file tempfile` create a channel instance; `close` and `chan close` destroy it; its configuration is abstract fields. This lets navigation and diagnostics follow a handle through variables, lists and dicts (`Answer<ChannelRef>`), as the object work does for TclOO instances, and gives W103 a value to consult instead of a word shape.
- **Whole-program facts.** `encoding system` changes the default for `encoding convertto/convertfrom` without a name and for every later `open`; it belongs with `tcl_precision` among the facts the potential-execution owner proves absent (decision 8) before any fold relies on the default.
- **Conformance.** The fixtures here become vector rows, with columns for each tclsh release under `LANG=C.UTF-8` and `LANG=C`, Jim, both engines, the script reader and the LSP's decode receipts.

## Plan

- Fix the crash and the silent exit: make `scan`/`split` of a lone surrogate in the VM read the checked value instead of the legacy accessor (`rust/tcl-vm/src/value/string_rendering.rs:36`), make the VM report the `encoding system nosuch` error through `catch`, and fix the runtime's 9.0 NUL representation (`scan \x00 %c`); done when e05, e12 and e15 match tclsh in both engines.
- Add the `Channels` trait to `tcl-runtime-api` and one input path in `tcl-cmd-core/src/channel.rs` (translation, eofchar, codec with profile, character counts); route the VM's `cmd_chan.rs` and the runtime's `cmd_chan.rs` reads through it and delete their lossy and raw paths; done when e07, e09, e13 and e14 match tclsh on 8.6 and 9.0.
- Implement `chan` in the VM over that trait and the missing `chan names/pending/pipe` in the runtime, or record each as an explicit non-goal in the engine contract; done when e11 runs.
- Add one script reader in `tcl-cmd-core` (beside `prepare_source_file`, `encoding/original_names.rs:370`) and call it from `tcl-vm-cli/src/main.rs:388`, `runtime/rust/examples/run_script.rs:174` and `source`; done when every `raw/` file and e10 match tclsh 8.6 and 9.0 in both engines.
- Correct `script_skips_leading_bom` for 8.x into a rule over (release, system encoding) in `tcl-dialect`, fix the comment at `model/family.rs:484`, and make W123 abstain on a leading BOM when the system encoding is not known; done when the BOM probe gives no W123 under 8.6 with a UTF-8 assumption.
- Move `source_decode.rs` onto the script reader: pass the dialect, drop the pinned `V8_6`/`tcl8` in `decode_source_as`, and word W107 by release (8.x: bytes read as Latin-1; 9.x: tclsh refuses the file); done when the LSP's text for s04/s05 equals what each tclsh reads.
- Model Ctrl-Z end of script in the reader and the lexer's `Document` channel, with a hint for text after it; done when s06 analyses only `puts before`.
- Make the registry's `-translation`, `-profile`, `-eofchar` and `-encoding` values checked value grammars with their release gates (`rust/tcl-registry/src/commands/tcl/fconfigure_.rs:68–112`, `chan_.rs`); done when the four 9.0 probes in `analyser-mcp.out` each give one diagnostic.
- Make one encoding catalogue: generate the name list from each release's `library/encoding` directory, keep `SystemEncoding`/`ChannelEncoding` as projections of it, share the `encoding` subcommand table between the VM, the runtime and the registry, and say in the engine contract which encodings are implemented; done when `encoding names` matches tclsh or the difference is a documented non-goal, and the runtime has `encoding profiles` on 9.0.
- Add channel instance values to the instance-value work (creation, destruction, configuration fields) and make W103 consult the value of the `open` argument; done when e11's closed-handle use is reported and e06's W103 is gone.
- Turn these fixtures into conformance vectors run against tclsh (both locales), Jim, the VM, the runtime, the script reader and the LSP decode, in the probe harness of cross-cutting fault 14.
