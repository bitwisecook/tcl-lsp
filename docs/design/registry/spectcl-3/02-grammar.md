# 2. Lexical rules and the complete grammar

This chapter is the closed grammar of SpecTcl 3: every form any source design
proposes, in one EBNF, with the vocabulary minor each form arrives in. The
vocabulary manifest (item 400) is generated from this grammar, and the
reader, printer, LSP surface, Studio schema and `registry-axes` vocabulary are
generated from the manifest.

## Lexical rules

A pack is a sequence of Tcl words, lexed by `tcl-source` (compiler split item
251) under a fixed grammar profile: `tcl` 9.0's script profile with
substitution set to `literal` (chapter 4).

- **Words.** Braces, double quotes, backslash-newline and `;` and newline
  separators are Tcl's. A braced word is verbatim. A quoted word decodes
  backslash escapes under the `tcl90` escape grammar (open decision D18), so
  any prose has a spelling.
- **No substitution.** `$`, `[` and `]` are literal characters in every word.
  `{*}` is a literal word. Nothing a pack says is evaluated.
- **Comments.** `#` where a command may start, to the end of the line, with
  Tcl's backslash-newline continuation. `;#` ends a row and starts a comment.
- **Rows.** A row is one command: a keyword, then words. A block is a braced
  word holding rows.
- **Numbers.** Arity bounds and indices are decimal. Versions are Tcl version
  numbers (`8.6.18`, `21.1.0.1`); BIG-IP lines compare on the BIG-IP axis.
- **Encoding.** UTF-8. A leading BOM is skipped.
- **Limits.** The reader caps bytes, nesting depth, rows per pack and `check`
  tree depth (item 407), and reports a coded notice when it stops.

Errors carry a `SpecCode` and a byte span, and the reader recovers per
top-level item (item 401). An unknown word is classified by the manifest's
`VocabularyClass`, as today: presentation words warn and drop, assistance
words degrade the command to `Unknown` for that capability, and semantic words
exclude the command or block from strong analysis.

## Version ranges

Every `RANGE` is a Tcl requirement read with `package vsatisfies`:

| Written | Set |
|---|---|
| `V` | `[V, next major)`: `8.6` holds 8.6.18 and not 9.0 |
| `A-B` | `[A, B)`: `8.4-8.6` excludes 8.6.18 |
| `A-` | `[A, ∞)` |
| `A-A` | exactly A |
| `{R1 R2 …}` | the union |

A whole line is written `8.4-8.7` or `8.4-9`. The printer, `tcl spec check`
and `tcl spec upgrade` flag a range that probably meant the whole line (an
upper bound equal to a ladder line the author lists elsewhere), and the 1.x
and 2.x shims translate the old whole-line reading into the algebraic range.

**Source:**

```tcl
foreach {v r} {8.6.18 8.4-8.6  8.6.18 8.4-8.7  8.6.18 8.4-9  9.1.0 8.4-
               9.1.0 8.4-9.1  9.1.0 8.4-9.2  4.2.4 3-4  3.4.0 3.4} {
    puts "$v in $r: [package vsatisfies $v $r]"
}
```

**In memory:**

```rust
// VersionSet is real (rust/tcl-dialect/src/model/version_set.rs:337);
// the interval spelling below is this chapter's
VersionSet("8.4-8.6") == [Interval { min: V(8.4), max: Excl(V(8.6)) }]
VersionSet("8.4-")    == [Interval { min: V(8.4), max: Unbounded }]
VersionSet("3.4")     == [Interval { min: V(3.4), max: Excl(V(4)) }]
```

**Bytecode (tclsh 9.1.0):**

```text
ByteCode
  Source "puts [package vsatisfies 8.6.18 8.4-8.6]\n"
  Cmds 2, src 41, inst 36, litObjs 5, aux 0, stkDepth 5, code/src 0.00
  Commands 2:
      1: pc 0-34, src 0-39        2: pc 5-29, src 6-38
  Command 1: "puts [package vsatisfies 8.6.18 8.4-8.6]..."
    (0) push 0 	# "puts"
  Command 2: "package vsatisfies 8.6.18 8.4-8.6..."
    (5) push 1 	# "package"
    (10) push 2 	# "vsatisfies"
    (15) push 3 	# "8.6.18"
    (20) push 4 	# "8.4-8.6"
    (25) invokeStk 4 
    (30) invokeStk 2 
    (35) done 
```

**Result:**

```text
$ tclsh8.5 ranges.tcl      (tclsh8.6, tclsh9.0, tclsh9.1 and ./target/debug/tclvm print the same)
8.6.18 in 8.4-8.6: 0
8.6.18 in 8.4-8.7: 1
8.6.18 in 8.4-9: 1
9.1.0 in 8.4-: 1
9.1.0 in 8.4-9.1: 0
9.1.0 in 8.4-9.2: 1
4.2.4 in 3-4: 0
3.4.0 in 3.4: 1
$ tclsh8.4 ranges.tcl
expected version number but got "8.4-8.6"
```

Tcl 8.4's `vsatisfies` takes no range at all, which is why tcllib 2.0's
indexes fail to read there. The pack reader runs the algebra itself, so packs
describe 8.4 with the same spelling.

## The grammar

Notation: `'word'` is a literal keyword, `X?` optional, `X*` any number, `X+`
one or more, `( … )` grouping, `|` alternatives. `{ … }` in a production is a
braced block of rows. A bracketed tag `[3.0]` or `[3.1]` names the vocabulary
minor a form arrives in; an untagged production is 3.0.

### Pack and identity

```text
file        := 'speclib' PACKNAME VOCAB '{' item* '}'
VOCAB       := '3' | '3.' MINOR                       ; '3' means 3.0
item        := pack-block | include | default | dialect | library | extend
             | environment | command | handle | errorclass | check | hook
             | event | flow | precedes | profile | transport-default | connection
             | template-section | apl-variable | domain | document | data
             | context | helper | map-type | limit | expr-profile
             | migrate | test | mock | message | taint-item
             | catalogue [3.1] | scope [3.1] | virtual-event [3.1]
             | style [3.1] | option-class [3.1] | token [3.1]
             | variable | roster | values | descriptor
pack-block  := 'pack' '{' ( 'version' VERSION | 'describes' LIBRARY RANGE
             | 'license' TEXT | 'upstream' URL | 'generated-by' TEXT
             | 'docs' URL-TEMPLATE )* '}'
include     := 'include' FILE                         ; sibling file, no '/' or '..'
default     := 'default' '{' cmd-row* '}'             ; applied to every command at read
variable    := 'variable' NAME '-kind' K '-access' A '-origin' O
               ('-available' LIB-RANGE)? ('-startup' B)?   ; 2.x special_var
roster      := 'roster' 'from' FAMILY 'into' FAMILY ('-available' RANGE)? '{' NAME* '}'
values      := 'values' ID '{' VALUE* '}'             ; a named value list
descriptor  := 'descriptor' KIND ID '{' ROW* '}'      ; a named descriptor (DescriptorId)
```

`roster` is 2.x's `include from tcl into jim`; `include FILE` is file
composition only. `variable` is 2.x's `special_var`.

### Dialects and grammar profiles

```text
dialect     := 'dialect' FAMILY '{' dia-row* '}'      ; BuiltIn or BundledPack, or a namespaced new family
dia-row     := 'release' LINE ('-patch' VERSION+)? ('-evidence' CLASS)?
             | 'build' BUILD ('-capabilities' '{' CAP-SETTING* '}')?
             | 'capability' CAP ('-default' ('on'|'off'|'optional'))?
             | 'grammar' GRAMMAR '{' grow* '}'
             | 'semantics' '{' srow* '}'
             | 'axis' AXIS VALUE ('-from' LINE)?      ; shim, read as a grammar row
GRAMMAR     := 'script' | 'number' | 'expr' | 'list' | 'format' | 'scan'
             | 'regex' | 'glob' | 'binary' | 'clock' | 'chars' | 'values'
grow        := AXIS VALUE ('-from' LINE)? ('-build' BUILD)?
srow        := ('namespace-eval-frame' ('namespace'|'new-frame')
             | 'creative-writing' ('yes'|'no')
             | 'proc-params' ('tcl'|'jim')
             | 'return-codes' CODE-TABLE
             | 'unknown' ('auto-load'|'none'|'user'))
               ('-from' LINE)? ('-build' BUILD)?
CAP-SETTING := ('+'|'-') CAP | 'wordSize' N
```

The axes and values are chapter 4's closed tables. A row without `-from`
holds from the first line; a later `-from` row for the same axis takes over.

### Libraries and dependencies

```text
library     := 'library' NAME '{' lib-row* '}'
lib-row     := 'versions' VERSION+
             | 'train' TRAIN RANGE
             | 'compatible' LIB-RANGE '{' FAMILY RANGE ('-build' BUILD)? ('-requires' CAPS)? '}'
             | 'requires' LIB-RANGE NEED ('-mode' ('load'|'lazy'|'optional'))?
                 ('-trigger' '{' ITEM+ '}')? ('-when' PRED)? ('-evidence' TEXT)?
             | 'provides' LIB-RANGE NAME ('same'|VERSION) ('-via' ('direct'|'exact-chain'))? ('-when' PRED)?
             | 'conflicts' LIB-RANGE '{' TARGET RANGE '}' '-message' TEXT
             | 'provides-early' LIB-RANGE
             | 'alias' NAME+                          ; sugar: provides … same -via exact-chain
             | 'tracks-host' FAMILY ('-build' BUILD)?
             | 'provided-by-core' FAMILY RANGE
             | 'placement-hint' ('autoload'|'module')
             | 'ifneeded' VERSION '{' 'source' FILE+ '}'   ; informational, never executed
             | 'loaded-by' '{' COMMAND PATTERN '}'
             | 'catalogue' DIR [3.1]
             | 'docs' URL-TEMPLATE | 'manual' PAGE
NEED        := '{' TARGET RANGE '}' | '-any' '{' ( '{' TARGET RANGE '}' )+ '}'
TARGET      := LIBRARY | 'core' FAMILY?
LIB-RANGE   := LIBRARY RANGE | 'core' FAMILY? RANGE
```

### Environments

```text
environment := 'environment' NAME ('-extend')? '{' env-row* '}'
env-row     := 'dialect' FAMILY RANGE ('-default' LINE)? ('-build' BUILD)? ('-capabilities' '{' CAP-SETTING* '}')?
             | 'library' NAME RANGE '-placement' ('ambient'|'hosted'|'bundled')
             | 'policy' ('open'|'closed'|'ambient-plus-require') ('-fixed-ensembles')?
                 ('-strict-ascii')? ('-evaluation' ('never'|'analysis'))?
             | 'alias' NAME+ | 'editor_identity' ID | 'selecting_identity' ID
             | 'file_extension' EXT DISPLAY | 'filename' NAME | 'shebang' WORD
             | 'signature' TEXT | 'signature' '-header' WORD '-arg' ('integer'|'word')
             | 'document' GRAMMAR-ID
             | 'unknown' ('auto-load'|'none') | 'auto_path' PATH+
             | 'display_name' TEXT | 'short_name' TEXT | 'kind' ('language'|'packages')
             | 'help_terms' WORD+
```

### Commands, subcommands, options and arguments

```text
command     := 'command' CMDPATH '{' cmd-row* '}'      ; CMDPATH is a name or a {name sub} list
extend      := 'extend' ('command'|'namespace'|'library') NAME '{' cmd-row* | lib-row* '}'
cmd-row     := avail-row | facet-block | arity-row | arg-row | option | subcommand
             | form | result | throws | effect-row | hook-row | name-row
             | layout | optimise | refactor | report | rewrite | implementation
             | runtime-row | event-row | ebpf-row | presentation | snippet
             | test | legacy-row
avail-row   := 'available' LIB-RANGE ('-dialect' '{' (FAMILY RANGE)+ '}')? ('-requires' CAPS)?
             | 'deprecated' LIB-RANGE ('-message' TEXT)? ('-fix' FIX)?
             | 'retired' LIB-RANGE
facet-block := 'versions' KEY '{' cmd-row* '}'           ; facetable rows only (manifest)
KEY         := LIB-RANGE ('-dialect' '{' FAMILY RANGE '}')? ('-build' BUILD)? ('-requires' CAPS)?
arity-row   := 'arity' (N | N '..' | N '..' N)
             | 'repeat' …                                ; 2.x, unchanged
arg-row     := 'arg' (N | NAME) ('-role' ROLE ('+' ROLE)*)? ('-values' LIST | '-values+' LIST | '-values-' LIST)?
                 ('-domain' DOMAIN)? ('-optional')? ('-optional-first')?
                 ('-scope' SCOPE)? ('-resolve' RESOLVE)? ('-defines'|'-probes'|'-links')? ('-in' CONTAINER)?
                 ('-frozen')? ('-binds-vars' 'data')? ('-object-kind' KIND)?
                 ('-frame' FRAME)? ('-appends' (N | '{' NAME+ '}'))? ('-percent-subst')? ('-concat')?
                 ('-callback')? ('-embedded' LANG ('-dialect' WORD)?)? taint-flags*
                 ('-handle' TYPE)? ('-needs' CAP+)? ('-releases'|'-moves-to' WORD|'-transforms'|'-stores')?
                 ('-creates' TYPE)? ('-array-of' '{' KEY+ '}')? ('-expr-profile' ID)?
                 ('-declares' KIND | '-references' KIND | '-references-each' KIND)?
option      := 'option' OPTNAME ( option-flag* | '{' cmd-row* '}' )
option-flag := '-takes' HINT | '-detail' TEXT | '-value-role' ROLE | '-db' '{' DBNAME DBCLASS '}'
             | '-ends-options' | arg-row flags | 2.x option flags (-also, -closed, -integer, …)
subcommand  := 'subcommand' NAME ('-exported' ('instance'|'class'))? '{' cmd-row* '}'
form        := 'form' NAME TEXT ('{' cmd-row* '}')?
ROLE        := 2.x role words (28) | 'MethodName' | 'WindowPath' | 'OptionName' | 'OptionDbValue'
             | 'tmsh-command' ('-verb' VERB)? | 'symbol-def' ('-observable')?
SCOPE       := 'local' | 'caller' N? | 'global' | 'namespace' | 'object' | 'connection'
RESOLVE     := 'current' | 'global' | 'ensemble' | 'receiver' '{' 'arg' N '}' | 'self'
CONTAINER   := 'word' | 'list' | 'script' | 'percent-script' | 'dict-value'
FRAME       := 'same' | 'global' | 'new' | 'shifted' N | 'namespace' WORD | 'connection'
name-row    := 'constructs' ('words'|'script'|'current-namespace'|'qualified'
                 |'method-prefix' ('-receiver' 'self')?|'which') ('-from' WORDS)?
             | 'transition' 'alias' '-name' WORD '-target' WORD ('-name-interp' WORD)? ('-target-interp' WORD)?
             | 'transition' 'tmsh' ('begin'|'commit'|'cancel')
             | 'mode-default' OPTNAME
```

`legacy-row` is every 2.3 command-body word the 2.x loader reads today
(`ROW_WORDS`, `rust/tcl-spectcl/src/loader/eval.rs:227`) that 3 keeps
unchanged: `traits`, `implementation_namespace`, `warn_missing_import`,
`is_namespace_exported`, `unsafe_command`, `excluded_events`,
`safe_on_uninit`, `deprecated_replacement`, `deprecated_replacement_drop_in`,
`source_deprecation_advice`, `alias_of`, `xc_translatable`,
`reserved_trailing_words`, `option_prefix_words`, `assigns_variable_at`,
`creates_instance_at`, `defines_command_at`, `body_arg_implicit_args`,
`body_kind`, `variable_receivers`, `allow_unknown_subcommands`,
`dynamic_surface`, `unknown_members`, `prefix_matching`,
`self_receiver_words`, `return_type`, `var_write_typing`,
`inferred_storage_type`, `byte_array_effect`, `byte_array_payload`,
`pattern_type`, `format_string_type`, `return_elements`,
`var_elements_effect`, `representation_effect`, `default_form_first_word`,
`defines_symbol`, `deprecation_fix`, `setter_constraint`, `hover`, `refine`,
`command_table_effect`, `frame_effect`, `world_effects`,
`state_transitions`, `result_stability`, `side_switch_target`,
`irules_source_flow`, `event_handler_priority`, `data_collection`,
`event_requirement_form`, `event_requires`, `body_scope`,
`channel_configuration`, `credential_options`, `sensitive_headers`,
`option_conflict`, `option_requires`, `option_requires_one_of`,
`option_forbids`, `option_placement`, `case_list`, `definition_body`,
`manufacturer`, `clause_grammar`, `binds_handle`, `object_class`,
`oo_context_fact`, `semantic_operation`, `detail`, `synopsis`, `pure`,
`mutator`, `header_write`, `destructive`, `returns_path`,
`source_index_bounds`, `source_path_operation`,
`source_configuration_reference`, `is_unescape`, `loop_list_header`,
`creates_scope_alias`, `arg_values_accept_prefix`, `cfg_rewrite_name`,
`min_abbrev`, `max_leading_option_words`, `credential_arg`,
`sub_subcommand`, `tk_geometry`, plus the words 332 adds for the fields no
pack can write today (`evaluation_context`, `callback_protocol`,
`body_interpreter`, `variable_write_min_args`, `event_emits`,
`event_emission_form`, `irules_top_level_effect`, `teardown_argument_layout`,
`remote_method`, `dispatch_dependencies`, `native_lowering`). The 2.x words
`dialects`, `required_package`, `tcllib_package`, `introduced_version`,
`deprecated_version`, `retired_version`, `versioned_arg_value`,
`ambient_package`, `co_provides`, `const_fold_versioned` and
`file_extension -dialect` are shim words only (chapter 10). The 2.x taint
words (`taint_source`, `taint_*sink*`, `taint_transform*`,
`taint_double_encode_colour`, `taint_sink_safe_colour`,
`taint_numeric_coercion`, `taint_sink_gate`, `callback_taint_inputs`,
`credential_options`, `sensitive_headers`) are shim words too: they map to the
taint forms below.

### Results, effects and handles

```text
result      := 'result' ( '-handle' TYPE ('-capability' CAPSPEC)? ('-child-of' 'self')?
                 | '-dict' ('-open')? '{' (KEY TYPE)+ '}' | '-list' TYPE | '-pairs' ) ('-when' PRED)?
effect-row  := 'side_effects' (NONE | TARGET+) | 'effects' '-complete' LIB-RANGE
             | 'effect' ('reads'|'writes') (TARGET | 'connection' FIELD | 'config' ('-verb' VERB)?)
             | 'decision' CATEGORY ACTION | 'ends' ('connection'|'message') ('-reset'|'-silent')
             | 'disables' 'events' ('-all' | '-arg' N) ('-this-connection')?
             | 'emits' EVENT '-certainty' ('definite'|'possible')
             | 'profile_effect' ('add'|'remove') PROFILE
             | 'releases' 'self' | 'returns' ('config-objects'|TYPE) | 'loads' 'library'
handle      := 'handle' TYPE '{' ( 'kind' ('token'|'command'|'object'|'path')
             | 'is' TYPE+ | 'parent' TYPE ('-released-with')?
             | 'lifetime' ('scope'|'interp'|'thread'|'connection'|'event'|'static')
             | 'spelling' PATTERN | 'instances' NAME+ | 'capability' NAME+
             | 'superclass' TYPE+ | 'mixin' TYPE+ | 'filter' NAME+   ; kind object
             | subcommand | option | 'hover' '{' … '}' )* '}'
errorclass  := 'errorclass' CLASS '{' ( 'code' CODEPATH ('-fields' '{' FIELD+ '}')? ('-message' TEXT)? avail-row*
             | '-fields' '{' FIELD+ '}' | avail-row )* '}'
FIELD       := NAME (':' ROLE)?
throws      := 'throws' PATTERN+ ('-when' PRED)? ('-on' ('wrong-args'|'unknown-command'))?
                 ('-default' PATTERN)? ('-detail' TEXT)?
PATTERN     := '{' WORD+ '}' | '{' 'arg' N '}'
```

### Rules, hooks and consumer rows

```text
check       := 'check' ID '{' 'on' CMDPATH 'when' PRED 'emit' EMIT '}'
PRED        := '{' 'all' PRED+ '}' | '{' 'any' PRED+ '}' | '{' 'not' PRED '}' | ATOM
ATOM        := '{' 'present' OPTNAME '}' | '{' 'argc' OP N '}'
             | '{' 'kind' WORD KIND '}' | '{' 'known' WORD '}'
             | '{' 'value' WORD ('in'|'notin') LIST '}' | '{' 'value' WORD 'matches' GLOB '}'
             | '{' 'value' WORD 'prefix' TEXT '}' | '{' 'value' WORD 'version-in' RANGE '}'
             | '{' 'value' WORD OP N '}' | '{' 'same' WORD WORD '}'
             | '{' 'count' OPTNAME OP N '}' | '{' 'before' WORD WORD '}'
             | '{' 'within' ('proc' NAME|'event' NAME|'namespace' PATTERN) '}'
             | '{' 'library' LIBRARY RANGE ('-must')? '}' | '{' 'dialect' FAMILY RANGE '}'
             | '{' 'build' BUILD '}' | '{' 'capability' CAP '}'
             | '{' 'native' CONDITION-ID '}'          ; a member of a closed Rust catalogue (393)
KIND        := 'literal' | 'known-bytes' | 'var' | 'command' | 'script' | 'expand' | 'opaque'
WORD        := 'arg' N | 'option' OPTNAME | 'subcommand' | 'command'
OP          := '<' | '<=' | '==' | '!=' | '>=' | '>'
EMIT        := CODE ('-severity' SEVERITY)? ('-category' CAT)? '-at' WORD '-message' TEMPLATE ('-fix' FIX)?
FIX         := '{' 'replace' WORD TEMPLATE '}' | '{' 'delete' WORD '}' | '{' 'insert-after' WORD TEMPLATE '}'
CODE        := PACK '/' ID                              ; a pack code; BuiltIn may name a DiagCode
hook        := 'hook' ID FAMILY ('-native' NATIVE-ID | '-body' PARAMS BODY) ('-inputs' SHAPE)?
hook-row    := FAMILY ('-native' NATIVE-ID | '-body' PARAMS BODY | '-ref' ID) ('-inputs' SHAPE)? ('-proof' RANGE)?
FAMILY      := 'arg_role_resolver' | 'command_prefix_resolver' | 'script_timing_resolver'
             | 'arg_role_count_resolver' | 'arg_role_layout_resolver' | 'const_fold'
             | 'context_gate' | 'literal_argument_validator' | 'constraints'
             | 'clause_shape_check' | 'state_transitions' 'resolver'
             | 'layout' | 'rewrite' | 'validate'
             | 'lowering_hook' | 'codegen_hook' | 'inline_codegen_hook' | 'analyser_hook'
             | 'return_type_hook' | 'native_compilation' | 'successful_handler'
             | 'procedure_definition' | 'completion' | 'body_execution'   ; BuiltIn and BundledPack only
SHAPE       := '{' ('kinds'|'argc'|'options'|'values'|'libraries'|'dialect')+ '}'
layout      := 'layout' '{' ( 'lead' N | 'pairs' ('options'|N) | 'break' ('fit'|'each'|'never')
             | 'align' ('options'|N) | 'clause' ROW '-break' ('before'|'never') )* '}'
             | 'layout' '-inputs' SHAPE '-body' PARAMS BODY
optimise    := 'optimise' '-no' '{' ('cse'|'hoist'|'dead-call')+ '}'
refactor    := 'refactor' '-no' '{' ('rename'|'extract'|'inline'|'signature'|'move')+ '}'
report      := 'report' 'redundant'
rewrite     := 'rewrite' ID '-when' PRED ('-to' TEMPLATE | '-native' NATIVE-ID | '-body' PARAMS BODY) '-proof' RANGE
implementation := 'implementation' '-native' IMPL-ID '-versions' '{' LIBRARY RANGE '}'
runtime-row := 'runtime_backing' ('none'|'host-native'|'shipped-builtin' ID
                 |'tcl-body' '{' ('-package-source' PATH | '-pack-text' TEXT) ('-evaluate')? '}')
```

### Domain forms

```text
event       := 'event' NAME '{' ( avail-row | 'side' ('client'|'server'|'both') | '-command-side' SIDE
             | 'transport' WORD+ | 'profiles' '{' ('any'|'all') PROFILE+ '}'
             | 'phase' ID | 'message' ('request'|'response') | 'multiplicity' ID
             | 'frame' 'connection' | 'establishes' '{' FIELD+ '}' | 'setup' EVENT
             | 'collect' PROTO '-side' SIDE | 'hot' | 'common' | 'category' ID
             | 'program-type' ID '-section' NAME '-context' ID ('-link')?          ; eBPF
             | 'capabilities' '{' CAP+ '}' | 'verdict' NAME N | 'default' NAME
             | 'out-of-bounds' NAME | 'attach' NAME '-kind' KIND ('-required')?
             | 'codegen' ('compiled'|'described') | 'hover' '{' … '}' )* '}'
flow        := 'flow' ID '-profiles' '{' PROFILE+ '}' '{' ( 'step' EVENT PHASE ('-conditional' TEXT)? )+ '}'
precedes    := 'precedes' EVENT EVENT '-evidence' TEXT
profile     := 'profile' NAME '{' ( avail-row | 'layer' ID | 'side' SIDE | 'implies' PROFILE+
             | 'forbids' PROFILE+ | 'conflict' PROFILE+ | 'schema-kind' KIND+ ('-via' NAME)?
             | 'namespace' NS ('-also' '{' NS+ '}')? )* '}'
transport-default := 'transport-default' PROFILE '-unless' PRED
connection  := 'connection' '{' ( 'field' FIELD '-side' SIDE '-lifetime' LIFETIME
                 '-origin' ('client'|'server'|'rule') '-target' TARGET )+ '}'
event-row   := 'rule_init' ('warn'|'error')
mock        := 'mock' CMDPATH ('-tcl' PROC | '-native' NATIVE-ID)
template-section := 'template-section' PROPERTY '-language' ('tcl'|'apl'|'html') ('-environment' ENV)?
apl-variable := 'apl-variable' '-root' NS '-separator' TEXT
domain      := 'domain' ID '{' 'values' LIST ('-nocase')? ('-closed')? ('aliases' LIST)? '}'
document    := 'document' ID '{' ( 'header' WORD '-version' N ('-newer' CODE)? ('-missing' CODE)?
                 ('-duplicate' CODE)? ('-older' CODE)? | ('open'|'closed') ('-unknown' CODE)?
             | 'substitution' ('reject'|'literal') ('-code' CODE)? | 'members' '{' WORD+ '}'
             | 'codes' '{' (CHECK CODE)+ '}' )* '}'
data        := 'data' ID '-source' PATH ('-provenance' PATH)?
member      := 'member' WORD ('-value' DOMAIN | '-block' ID | '-references' KIND)?
                 ('-read-by' '{' PHASE* '}')? ('-required')? ('-frozen')? ('-validate' '-native' NATIVE-ID)?
             | 'exclusive' '{' WORD+ '}'             ; inside definition_body
ebpf-row    := 'compile' 'ebpf' OP ('-' KEY VALUE)*
context     := 'context' ID '{' ( 'field' NAME OFFSET WIDTH ORDER ('-role' ROLE)? )+ '}'
helper      := 'helper' NAME '{' avail-row* 'id' N 'args' LIST 'returns' TYPE ('hover' '{' … '}')? '}'
map-type    := 'map-type' NAME '{' 'id' N ('key-size' N)? ('preallocated')? '}'
limit       := 'limit' NAME VALUE                       ; facetable
expr-profile := 'expr-profile' ID '{' 'ops' LIST 'div' ('truncate'|'floor') 'wrap' N '}'
```

### Package surfaces, tests and localisation

```text
migrate     := 'migrate' ID '{' 'library' LIB-RANGE 'on' CMDPATH ('when' PRED)?
                 ('rewrite' TEMPLATE TEMPLATE)* ('note' TEXT)? '}'
test        := 'test' ID '{' 'source' TEXT ('-on' KEY)? ('-consumer' CONSUMER)?
                 ('-profiles' '{' PROFILE+ '}')? ('-setup' TEXT)? ('at' LINE COL)? ('new' NAME)?
                 'expect' EXPECT ('run' '{' RELEASE+ '}')? ('-privileged')? '}'
EXPECT      := '{' ( 'diagnostic' CODE 'at' LINE ':' COL | 'hover' 'contains' TEXT
             | 'result' TEXT | 'edits' TEXT | 'abstain' ABSTAIN-CODE | 'output' TEXT
             | 'run' TEXT 'decision' TEXT | 'grade' G | 'finding' CODE )+ '}'
message     := 'message' LOCALE CODE TEXT | 'doc' LOCALE ITEM TEXT          ; reserved; English only ships
presentation := 'presentation' '-class' ('control'|'keyword'|'builtin') ('-context-sensitive')?
snippet     := 'snippet' PREFIX LABEL BODY
token       := 'token' CATEGORY [3.1]
catalogue   := 'catalogue' DIR [3.1]
scope       := 'scope' 'project' [3.1]                ; a policy pack placed by tclpkg.tcl
virtual-event := 'virtual-event' NAME '-widget' CLASS [3.1]
style       := 'style' NAME ('-element' NAME)* ('-theme' NAME)? [3.1]
option-class := 'option-class' NAME '-widget' CLASS [3.1]
taint-item  := colour | origin | sink | header-class
```

### Taint

These are design [393](../model/393-taint.md)'s forms. A value's taint fact is
a pair: **origins**, a "may" set (where it may have come from), and
**colours**, a "must" set (properties proved of every tainted contributor).
`TAINTED` is gone: a value is tainted when its origins are non-empty.

```text
colour      := 'colour' COLOUR '{' ( 'core' | 'concat' ('all'|'first'|'none')
             | 'implies' (COLOUR | '{' COLOUR+ '}') | avail-row | 'hover' '{' … '}' )* '}'
origin      := 'origin' ORIGIN '{' ( 'discloses' | 'hover' '{' … '}' )* '}'
sink        := 'sink' KIND '{' '-origins' ('any' | '{' ORIGIN+ '}') '-safe' ( COLOUR | '{' COLOUR* '}' )
                 '-code' ( CODE | '{' (CODE | FAMILY CODE)+ '}' ) ('-severity' SEVERITY)? ('-discloses')? '}'
header-class := 'header-class' CLASS '{' HEADER+ '}'
COLOUR      := a core name (crlf-free) | LIBRARY '.' NAME (sql.bound)       ; 'core' is trusted tiers only
taint-flags := '-sink' KIND ('-safe+' COLOUR+)? ('-when' PRED)?   ; on arg-row: SinkWhen semantics
             | '-origin' ORIGIN | '-writes' '-origin' ORIGIN | '-reads'
             | '-double-encodes' COLOUR | '-secret' | '-secret-header-of' WORD
             | '-callback-origin' '{' (SUBST ORIGIN)+ '}'
result      += 'result' ( '-origin' ORIGIN ('-colour' COLOUR)? | '-adds' COLOUR+ | '-clean'
             | '-from' WORD ('-unescapes')? | '-element-of' WORD | '-elements-from' WORD ) ('-when' PRED)?
refines     := 'refines' WORD ( '-adds' COLOUR+ | '-adds-from-pattern' WORD ) ('-when' PRED)? '-on' ('true'|'false')
errors-row  := 'errors' '-message-from' WORD                     ; catch messages carry origins
variable    += ('-origin' ORIGIN)?
cmd-row     += refines | errors-row | 'taint' '-complete' LIB-RANGE
```

- **`implies`** closes colours (a cycle is a load error); **`concat`** says
  what concatenation does to a colour, read by `strcat`, `format`, `append`,
  `subst`, the analyser and the run-time mode alike.
- **`-safe`** lists colours any one of which makes a value safe at the sink. A
  pack may widen a core sink's `-safe` only by a colour that implies one
  already there, so the security floor holds.
- **`-when` on a sink is `SinkWhen`**: the sink is suppressed only when the
  predicate is **proved false**; an unknown leaves it in force. This is the
  reverse of every other predicate, and the image marks the rule so the
  evaluator cannot get it backwards. It replaces the `taint_sink_gate` hook.
- **`refines … -on true|false`** colours the named word's variable on the
  branch where the call returned true (or false): a validator.
- `channel` is not a colour; a channel read's origin comes from its handle type
  (391).

## The forms by vocabulary minor

| Minor | Forms |
|---|---|
| 3.0 | everything above not tagged `[3.1]`: identity, `include`, dialects and grammars, libraries and dependencies, `extend`, environments, commands and facets, `deprecated`/`retired`, `check`, hooks, consumer rows (`layout`, `optimise`, `refactor`, `rewrite`, `implementation`), name and frame facts, `handle`, `errorclass`/`throws`, `result` shapes, `-embedded`, `-callback -appends`, `migrate`, `event`/`flow`/`precedes`/`profile`/`connection`, iApps and tmsh rows, SslicTcl's `domain`/`document`/`data`, eBPF's `compile ebpf`/`context`/`helper`/`map-type`/`limit`/`expr-profile`, `test`, `mock`, `presentation`, `snippet`, reserved `message`/`doc LOCALE`, the taint forms (`colour`, `origin`, `sink`, `header-class`, `refines`, `-sink`, `result -origin`/`-adds`) |
| 3.1 | `catalogue`, project `scope`, `virtual-event`, `style`, `option-class`, `token` |

A new word arrives in the minor its manifest row names (`since 3.N`). A pack
declaring `speclib x 3.1` read by a 3.0 reader keeps loading maximally: an
unknown 3.1 word is classified by its `VocabularyClass`, as 2.x does today.
A major other than 3 is refused with one notice naming the fix.

## The pack header

```mermaid
flowchart TB
  header["speclib NAME VOCAB"] --> major{"major is 3?"}
  major -- "1 or 2" --> shim["shims, chapter 10"]
  major -- "other" --> refuse["refused: one notice"]
  major -- "3" --> minor{"minor known to this reader?"}
  minor -- yes --> read["read every row"]
  minor -- "newer" --> classify["read known rows; classify unknown words by VocabularyClass"]
```

Item 400 owns this grammar's manifest; items 401 (errors), 408 (identity and
`include`), 409 (`extend`) and 413 (`check` atoms) own its parts.
