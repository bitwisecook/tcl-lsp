# Tk original literal word caller migration

Exact patch: tk-original-literal.e06f54e9380172a46cb5a1ea14a7e385f40f4d69e40a5af15fe1d29f5513d924.patch
SHA-256: e06f54e9380172a46cb5a1ea14a7e385f40f4d69e40a5af15fe1d29f5513d924

Existing baseline unit lsp-tk-original-literal-word-value. One actual caller purpose; no new semantic owner count. The private view is a transport adapter to existing owners and is not a parser, resolver or naming policy.

TkSourceCommand holds the actual command visited by the existing executable-region walker and an optional complete original NativeWord vector. CurrentSourceContext checks the complete retained Document image, full supplied LexerConfig, input, context and realm. CommandTokens are built with that actual image/config and vector; the existing public original_native_compiler_words owner checks whole original source-vector, full word spans, configured original parse and reconstruction correspondence. No detached Native source, fresh child analysis, cooked-value coordinate inverse or private lexical parser is introduced. Its NativeWord name describes original parser geometry and supplies no Native handler or runtime proof.

literal_word delegates to shared original_static_word_unicode_value, retaining word.span as the original complete word anchor. Substitution, expansion, partial or unavailable original ownership declines. Escapes decode with the retained source grammar; braced metacharacters remain literal. No representative command.texts value or one-token Esc heuristic can supply a literal. The same held view flows through existing private Tk collectors, including the widget-method fallback head. Registry selection, availability/version and source context behavior are unchanged.

Option-name recognition and geometry container-option presence must use the same decoded original words, otherwise escaped -text/-in spellings would still select the wrong source datum. The existing option-prefix resolver and existing option scanning policy remain; this patch does not claim to repair independent option-grammar or runtime identity/effect consumers. Boolean option anchors also use the complete original word.

New fixed controls:
- original_tk_literals_decode_escapes_and_retain_complete_word_spans: actual UI model decodes escaped pathname, option name, Unicode/escaped literal sigil/brackets and geometry -in; checks original whole quoted-value and path source spans.
- original_tk_literals_keep_braced_data_and_the_actual_full_configuration: preserves literal braced sigils/brackets/escape text and distinct Folds/Literal brace-newline configurations on genuine whole retained analyses.
- original_tk_literals_refuse_substitution_expansion_and_foreign_source: actual variable/command/compound substitution, expansion, unrepresentable surrogate escape, changed complete source, partial command and missing input decline.

Every original Tk feature test from supplied_tk_preview_keeps_actual_availability_and_refuses_withheld_source through the file end is byte-for-byte unchanged, including existing source/context/availability, rebinding/captured-prefix, nested body, dynamic path, geometry, widget mutation and bounds controls. No new allow attributes.

Validation: direct TMP rustfmt and rustfmt --check PASS; exact patch application to independent TMP beforeimage PASS; resulting file hash equals manifest afterimage; live main source matched the retained beforeimage. No Cargo, compiler, test binary, main, Git or network operation. Root and independent reviewer must review and execute controls on fresh binaries; authored tests are not acceptance evidence.
