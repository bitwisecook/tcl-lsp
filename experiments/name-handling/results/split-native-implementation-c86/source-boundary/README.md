# Split prerequisite: escaped source producer

Fresh benign controls in capture-01 identify a producer difference before split runs.
All six processes exit 0 under actual real C86 Tcl_Init/Tcl_EvalEx or retained Native C86 --init.
Written `set d \0`: C86 resident element length 2, bytes c080; Native byte extent 1.
Literal raw 00 inside braces: both report extent 1 (C resident bytes 00).
Literal c080 inside braces: both report extent 2 (C resident bytes c080).
Only the C86/Native C86 producer pair was executed here; C84/85/C9/Jim/F5 producer contexts remain unexecuted for this question. Existing central backslash C5 proofs are independent.
The Native binary still matches the frozen split-reference runner; no new build is represented.
Exact sources, binary/source hashes, commands and separate raw C/native channels are retained; no environment values are queried.

Actual ingress is counted C Tcl_EvalEx versus retained Native std::fs::read -> eval_sourced_completion -> eval_script_mode. The Source frame supplies location; parse::parse_script_with_config carries LexerConfig plus borrowed original bytes.
Lexer::with_bytes and SourceMap::from_bytes select SourceChannel::NativeValue; no Document or ExternalFile transformation is substituted by this parser.
Runtime parse::build_word (parse.rs:562) decodes Esc tokens with the lexical backslash::decode_bytes_in and retains only decoded WordPart::Text bytes.
Interp::substitute_word (interp.rs:18531) appends those decoded bytes unchanged into its fresh String.
Consequently split cannot distinguish a written escaped NUL producer from literal resident 00 using its object alone.
C86's single-byte delimiter branch differs from the multi-byte/general branch; rewriting resident raw 00 as c080 inside split would manufacture the wrong input purpose.

The correct shared physical escape owner already exists: backslash::native_source_string_bytes_channel_in / native_arena_text, selected by Interp::source_string_protocol.
The tree-word producer must retain original escaped token bytes/extent until substitution and invoke that owner at the actual operand stage for independently admitted Native C86.
Keep the existing lexical parse API, Logical text bytes, literal raw bytes, other Native routes and original tests unchanged; preserve getter/evaluation order and operational decoder refusals.
A small metadata-bearing Text wrapper in parse.rs can retain decoded lexical bytes plus optional original escaped source; its dereference preserves existing lexical consumers.
Interp's Text substitution then uses the original source only for actual Native C86 physical decoding; it must not infer provenance from the decoded value or per-command names.
This adds runtime/rust/src/parse.rs and runtime/rust/src/interp.rs to the original five-file split scope; no compiler/registry/platform changes are proposed.
If the metadata change requires another consumer owner, enumerate it before editing rather than silently broadening the window.

Then the original five owners can implement the measured C86 counted-byte split plan, central NativeTclUtf unit/UCS4 purposes and genuine runtime String/List child sharing under original currency.
Required producer controls include written/literal NUL, concatenated escaped parts, variable/command substitutions, Native surrogate escapes, source-native unavailable capability and unchanged original Logical/default/other-profile tests.
The seven-file scope is authorised after the coordinated W210 shared dependency freeze releases; implementation has not started. Native source admission must use actual native_source_string_protocol plus the entered original operation; source_string_protocol also admits a Logical provider, so Some(C86) alone is insufficient. No Rust edits, Cargo, Git, network or canonical documentation mutations occurred.
