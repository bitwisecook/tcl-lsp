# naming.variable.namespace-upvar-original-compiler-frontiers

Kind: `native-observation`

## Problem statement

Namespace-upvar compilation allocates local aliases after projecting the actual public/private worker argv. Escaped, empty, array-shaped and dynamic locals, child substitutions and expansion can change acceptance or roll back a partial preparation, so a role or decoded display cannot supply the genuine compiler operand/ordered local table.

## Question

Which nsupvar opcode counts and original local/literal vectors occur for the 15 retained namespace-upvar bodies under C8.5 and the modern C workers?

## Conclusion

The exact C8.5 and modern tables retain 60 compiler frontiers and their original ordered local/literal vectors. C8.5 declines the braced selector while C8.6/C9 accept it; other source variants retain their measured opcode counts, including complete decline of partial invalid/dynamic locals. These are original compilation facts, not alias-installation success, another activation or a native local-table capability.

## Scope

Fifteen ASCII source bodies from cases.rs, recorded native procedure compilation for C8.5.19/8.6.18/9.0.4/9.1.0. C8.4/Jim/BIG-IP not queried for this compiler question. Original Tcl compiler presenter scripts/logs are referenced by hashes but not retained; tables, original manifest source bodies and binary hashes remain. No opaque-byte or native execution authority inferred.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Native shell SHA 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935; full build/header recipe not recorded. Channel: ASCII procedure source under the recorded original compiler presenter. Dialect: Tcl.

15 original compiler observations, each presenter exit0. Fields case, nsupvar count, locals_hex and literals_hex retain independent preparation order.

```tsv
case	nsupvar	locals_hex	literals_hex
0	1	7061697273,6c6f63616c	3a3a4e,78,
1	1	7061697273,6c6f63616c,6e73	3a3a4e,78,
2	1	7061697273,6c6f63616c,6f74686572	3a3a4e,78,
3	2	7061697273,6c6f63616c,6e73,61,6c656674,62,7269676874	3a3a4e,78,79,
4	1	7061697273,6c6f63616c	3a3a4e,78,
5	1	7061697273,6c6f63616c	3a3a4e,78,
6	0	7061697273,6c6f63616c	3a3a4e,78,6e616d657370616365,7570766172,6c6f3a6c
7	1	7061697273,6c6f63616c,	3a3a4e,78,
8	0	7061697273,6c6f63616c,61	3a3a4e,78,6e616d657370616365,7570766172,61286b29
9	0	7061697273,6c6f63616c	3a3a4e,78,6e616d657370616365,7570766172,3a3a6c6f63616c
10	0	7061697273,6c6f63616c,6e73,61,676f6f64,62,626164	3a3a4e,78,79,6e616d657370616365,7570766172,676f6f64,626164286b29
11	0	7061697273,6c6f63616c,6e73,61,676f6f64,62	3a3a4e,78,79,6e616d657370616365,7570766172,676f6f64
12	0	7061697273,6c6f63616c	6e616d657370616365,7570766172,3a3a4e,78,6c6f63616c
13	1	7061697273,6c6f63616c	3a3a4e,78,
14	0	7061697273,6c6f63616c	6e616d657370616365,7570766172,3a3a4e
```

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Native shell SHA b307c18ab4946e932f72918d4803f01a1cd6ce8c28e8dd412c94876537e1190c; full build/header recipe not recorded. Channel: ASCII procedure source under the recorded original compiler presenter. Dialect: Tcl.

15 original compiler observations, each presenter exit0. Fields case, nsupvar count, locals_hex and literals_hex retain independent preparation order.

```tsv
case	nsupvar
0	1
1	1
2	1
3	2
4	1
5	1
6	0
7	1
8	0
9	0
10	0
11	0
12	1
13	1
14	0
```

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Native shell SHA 647d9627cd7c2aea30a8c629ed3386d2b06380aa096443ae1595eac9052b2553; full build/header recipe not recorded. Channel: ASCII procedure source under the recorded original compiler presenter. Dialect: Tcl.

15 original compiler observations, each presenter exit0. Fields case, nsupvar count, locals_hex and literals_hex retain independent preparation order.

```tsv
case	nsupvar
0	1
1	1
2	1
3	2
4	1
5	1
6	0
7	1
8	0
9	0
10	0
11	0
12	1
13	1
14	0
```

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Native shell SHA baa7535f758da4c3be4fdd3e7ba9d7ba48d6b863cacabfff6681f2ce75ec0cee; full build/header recipe not recorded. Channel: ASCII procedure source under the recorded original compiler presenter. Dialect: Tcl.

15 original compiler observations, each presenter exit0. Fields case, nsupvar count, locals_hex and literals_hex retain independent preparation order.

```tsv
case	nsupvar
0	1
1	1
2	1
3	2
4	1
5	1
6	0
7	1
8	0
9	0
10	0
11	0
12	1
13	1
14	0
```

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (provider): [rust/tcl-registry/tests/data/native_namespace_upvar_compilation/provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/provenance.json). SHA-256 `fc63dd3a41039235ef22369e3c229029e0cafc64a621536ee62340ab17923b6f`. C8.5 original source body/log hashes, binary hash and compiler frontier answers; execution subset is separately documented.
- `e1` (provider): [rust/tcl-registry/tests/data/native_namespace_upvar_compilation/modern-provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/modern-provenance.json). SHA-256 `d1bc42c9d7f785f885f6f74bcf717d661e1aba5afba446b400bb1e3e79e6da8b`. 45 modern individual script/status/nsupvar-count/source/binary provenance records.
- `e2` (input): [rust/tcl-registry/tests/data/native_namespace_upvar_compilation/cases.rs](../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/cases.rs). SHA-256 `7fff38f03a603ae40e0b633f8986718101ab3b574823459ce87251c5af52e937`. Exact fifteen original bodies, with escaped/quoted/braced/empty/expanded/partial locals.
- `e3` (observation): [rust/tcl-registry/tests/data/native_namespace_upvar_compilation/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/8.5.19.tsv). SHA-256 `fcd5396c08a0a349f7159b2cbc0efb32cd615271a3b4759f81d59d56b595f00b`. Fifteen original opcode/local/literal frontiers, plus field header.
- `e4` (observation): [rust/tcl-registry/tests/data/native_namespace_upvar_compilation/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/8.6.18.tsv). SHA-256 `af2621ad53a54fe0c56b7302bc9a8be3382b56599ddb727e573c3778bc1debe3`. Fifteen original opcode/local/literal frontiers, plus field header.
- `e5` (observation): [rust/tcl-registry/tests/data/native_namespace_upvar_compilation/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/9.0.4.tsv). SHA-256 `af2621ad53a54fe0c56b7302bc9a8be3382b56599ddb727e573c3778bc1debe3`. Fifteen original opcode/local/literal frontiers, plus field header.
- `e6` (observation): [rust/tcl-registry/tests/data/native_namespace_upvar_compilation/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/9.1.0.tsv). SHA-256 `af2621ad53a54fe0c56b7302bc9a8be3382b56599ddb727e573c3778bc1debe3`. Fifteen original opcode/local/literal frontiers, plus field header.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_namespace_upvar_compilation.rs](../../../../rust/tcl-registry/src/native_namespace_upvar_compilation.rs), `original_c85_namespace_upvar_matches_15_native_compiler_frontiers`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-registry/src/native_namespace_upvar_compilation.rs](../../../../rust/tcl-registry/src/native_namespace_upvar_compilation.rs), `original_modern_namespace_upvar_matches_45_native_compiler_frontiers`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-registry/src/native_namespace_upvar_compilation.rs](../../../../rust/tcl-registry/src/native_namespace_upvar_compilation.rs), `native_namespace_upvar_compilation::tests::original_c85_namespace_upvar_matches_15_native_compiler_frontiers` (linked): Checks C8.5 complete original compiler acceptance/decline frontiers.
- [rust/tcl-registry/src/native_namespace_upvar_compilation.rs](../../../../rust/tcl-registry/src/native_namespace_upvar_compilation.rs), `native_namespace_upvar_compilation::tests::original_modern_namespace_upvar_matches_45_native_compiler_frontiers` (linked): Checks modern independently selected workers and original word vectors.

A named test is a coverage binding, not a claim that it executed.

## Replay

The manifest refers to per-case compiler presenter source/log files outside the repository; those wrappers are not retained. cases.rs preserves exact body bytes, but running a body alone cannot produce the private opcode/local/literal vectors. A fresh replay must retain a matching-version procedure/disassembler presenter, full source/build/artifact hashes and every one of the15 original body/vector windows; compare each native table without treating opcode presence as alias execution success.
