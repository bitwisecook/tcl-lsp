# Diagnostics by registry field

> Verified against `rust/tcl-core-types/src/diag_code.rs` and the emitters
> in `rust/tcl-compiler` / `rust/tcl-lsp-core`, 2026-08-12. **Causes**
> means declaring the field makes the diagnostic able to fire on your
> command; **suppresses** means it silences one that would otherwise fire.

## Arity, dispatch, and keywords

| Code | Reports | Registry drivers |
|---|---|---|
| E001 | Missing subcommand | Non-empty `subcommands` + `arity.min > 0` causes; `arity.min == 0` means a bare call is a legal default form |
| E002 / E003 / E005 | Too few / too many / off-step argument count | `arity` (`min`, `max`, `step`, `also_exact`) causes; declared `options` and their value words are skipped before counting; `Traits::STRUCTURALLY_CHECKED_ARITY` suppresses all three in favour of E004; `hover.synopsis` supplies the `usage:` suffix |
| E004 | Malformed clause chain | `clause_shape_check` causes (pair with the trait above) |
| W001 | Unknown subcommand | Undeclared word causes; `allow_unknown_subcommands`, `default_form_first_word`, a unique prefix (`prefix_matching`), and `implementation_namespace` each suppress |
| W145 | Ambiguous abbreviation | `PrefixMatching::Enabled` + overlapping names causes; `Strict` suppresses (falls through to W001); `min_abbrev` shrinks the match set |
| W142 | Invalid in this context | `context_gate` returning a message causes |

## Options and literal values

| Code | Reports | Registry drivers |
|---|---|---|
| W004 | Option not in this dialect | `OptionSpec::dialects` / `lifecycle` failing the profile causes |
| W127 | Value outside a closed set | `closed_value_args` + `arg_values` causes (positional); a closed `OptionValue::enumerated` causes (option values); `arg_values_accept_prefix` accepts prefixes |
| W141 | Option value fails a shape check | An option-arity hook returning invalid causes |
| W146 | Literal-argument relationship violated | `literal_argument_validator` returning Invalid causes; a `replacement_value` adds the quick fix |
| W147 | Mutually exclusive options together, or a directional exclusion | an `option_conflict` / `option_forbids` row in `option_relations` causes; `option_placement` decides where the options are looked for |
| W152 | An option relation is unmet — this one needs that one | an `option_requires` / `option_requires_one_of` row in `option_relations` causes; the `constraints` hook can also report one |
| W304 / T102 | Missing `--` before a dynamic value / tainted option-position value | Only exist if an `OptionSpec` named `"--"` is declared; `reserved_trailing_words` exempts trailing words |

## Availability and lifecycle

| Code | Reports | Registry drivers |
|---|---|---|
| W002 | Command disabled in this dialect | `dialects` excluding the profile causes (with the "available in" suffix) |
| W120 | Missing `package require` | `required_package` causes; a matching require, an ambient package, or a same-file definition suppresses |
| W123 | Unknown command | Absence from the registry causes; any spec, a `body_scope` env, or an `UNRESOLVED_COMMAND_HANDLER` command suppresses |
| W135 / W136 / W137 / W139 / W144 | Needs newer package (command / option / value) / retired / deprecated | `lifecycle.introduced` / `.retired` / `.deprecated` on the command, subcommand, option, or versioned value causes; `lifecycle.deprecation_fix` adds the W144 quick fix; retirement supersedes deprecation |
| W138 | Conversion needs newer Tcl | `format_string_type == Sprintf` + a format-string role |
| W143 | Private `::tcl::` namespace call | `implementation_namespace` on the private spec causes; the public ensemble's `subcommands` shape the fix |
| W129 | Hidden in a safe interpreter | `Traits::SAFE_INTERP_HIDDEN` causes |
| W113 / W116 / W117 | Proc / stub shadows a built-in | Registry name membership causes; a non-ambient `required_package` suppresses W113 |
| IRULE2002 / 2003 | Deprecated / unsafe iRules command | `deprecated_replacement` (+ `_drop_in` for the auto-fix) / `unsafe_command` |

## Variables and dataflow

| Code | Reports | Registry drivers |
|---|---|---|
| W210 | Read before set | `ArgRole::VarWrite` records the eventual def; `safe_on_uninit` suppresses the command's own read-before-write only when its surface admits the document's point (and lowers conservatively to false without one); `Traits::CREATES_SCOPE_ALIAS` suppresses; `DESTROYS_VARIABLE` diverts to W213 |
| W211 / W220 | Unused variable / dead store | `ArgRole::VarRead` records the read; `Traits::READS_BEFORE_WRITE` keeps the feeding store alive |
| W212 | `$` where a name is expected | Any `VarWrite` / `VarRead` role position receiving `$var` causes |
| W126 | Non-channel in a channel slot | `ArgRole::Channel` causes (and filters that position out of taint output sinks) |
| W128 | Call to a renamed command | `command_table_effect` on the mutating command is what makes the rebinding visible |
| W307 / W308 / W250 / W315 | Dynamic head / unknown method / abstract class / undefinable member | `object_class` (+ `allow_unknown_methods`), `definition_body`, `manufacturer_methods`, `Traits::ABSTRACT_CLASS_FACTORY` |

## Security, taint, and iRules

| Code | Reports | Registry drivers |
|---|---|---|
| W100 / W105 | Unbraced expression / body | `ArgRole::Expr` / `ArgRole::Body` causes at that position; `EXPR_CONCATENATES_ARGS` widens the anchor |
| W101 / W301 | String-built script into eval / uplevel | Exactly `SCRIPT_CONCATENATES_ARGS \| TAINT_SINK`, split by `EVALUATES_IN_SHIFTED_FRAME` |
| W102 / W309 | `subst` risks | `Traits::PERFORMS_SUBSTITUTION` (with its option flags as suppressors); `EVALUATES_CODE \| TAINT_SINK` for the outer half |
| W103 / W300 | Pipeline `open` / variable `source` | `OPENS_CHANNEL` (adding `TAINT_SOURCE` suppresses, the socket case) / `SOURCES_FILE` |
| W302 | `catch` without result variable | Trailing `VarWrite` roles drive the fix; `FIRE_AND_FORGET_TEARDOWN` in the body suppresses |
| W303 / W306 / T103 | Regex ReDoS / substitution in a pattern / tainted pattern | `pattern_type == Regex` |
| W310 | Hardcoded credential | `credential_options`; `SubCommand::credential_arg` + `sensitive_headers` |
| W312 / T105 | Cross-interp eval risks | `taint_interp_eval_subcommands` |
| W313 | Destructive op on a variable path | `SubCommand::destructive`; path taint colours soften or suppress |
| taint sources | Result is attacker-influenced | `Traits::TAINT_SOURCE` (+ `taint_source` colours), the `IRULES_TAINT_SOURCE_PREFIXES` namespace rule, `UNNORMALISED_HTTP_GETTER` |
| T100 | Taint into code execution | `TAINT_SINK` or `EVALUATES_CODE` causes; `taint_code_sink_args` narrows the slots; `taint_sink_gate` and `taint_sink_safe_colour` suppress |
| T101 / IRULE3001–3004 | Taint into output / header / log / redirect | The `taint_output_sink` / `taint_log_sink` code string picks the diagnostic; `taint_output_sink_subcommands` restricts it; the matching safe colour suppresses |
| T104 | Taint into a network address | `taint_network_sink_args`; IP/port/FQDN colours suppress |
| T106 / sanitisers | Double-encoding / cleaning | `taint_double_encode_colour` causes; `taint_transform` colours (and a numeric `return_type`) sanitise |
| IRULE3101 / 3102 | Setter prefix / missing `-normalized` | `setter_constraints` (the constraint carries its own code and message); an `OptionSpec` named `-normalized` is the whole 3102 gate |
| IRULE1001–1008 | Event validity, collect/release pairing | `event_requires`, `event_requirement_forms`, `excluded_events`, `data_collection`; `event_handler_priority.warn_when_implicit` drives IRULE1004 |
| IRULE1201 / 1202 / 5006 / 5007 | HTTP context, double respond, placement | `Traits::REQUIRES_HTTP_CONTEXT`; a `ResponseCommit` write in `side_effects`; `IRULES_TOP_LEVEL_ONLY`; any event requirement |
| S100–S103 / S110 | Shimmer and byte-array corruption | `arg_types` (`shimmers`, `transparent_from`), `return_type`, `representation_effect`, `byte_array_effect` (`Transparent` suppresses, `Rebinarifies` clears), `byte_array_payload` |

## Not registry-driven

Lexer/parser structure (E1xx / E2xx), encoding and style (W107–W118,
W305), pure-dataflow checks (W214–W218, W230–W233, W240–W242, H300,
I230/I231), and a handful of name-hardcoded checks (`append` W104,
`switch` W106, `binary` W200, `fconfigure` W311, iRules drop/log/proc
checks). W130–W134 are reserved with no producer. `const_fold` still
matters here indirectly: folded constants are what let the bounds and
constant-branch checks see through your command's results.
