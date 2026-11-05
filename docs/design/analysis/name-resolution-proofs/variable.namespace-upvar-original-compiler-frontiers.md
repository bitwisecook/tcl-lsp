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

### tcl8.4, jim

Status: `not-tested`.

No observation for this question.

### tcl8.5

Status: `observed`. Channel: ASCII procedure source under the recorded original compiler presenter. Version: tcl8.5 8.5.19.

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

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII procedure source under the recorded original compiler presenter. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

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

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-registry/src/native_namespace_upvar_compilation.rs](../../../../rust/tcl-registry/src/native_namespace_upvar_compilation.rs), `original_c85_namespace_upvar_matches_15_native_compiler_frontiers`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-registry/src/native_namespace_upvar_compilation.rs](../../../../rust/tcl-registry/src/native_namespace_upvar_compilation.rs), `original_modern_namespace_upvar_matches_45_native_compiler_frontiers`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-registry/src/native_namespace_upvar_compilation.rs](../../../../rust/tcl-registry/src/native_namespace_upvar_compilation.rs), `native_namespace_upvar_compilation::tests::original_c85_namespace_upvar_matches_15_native_compiler_frontiers` (linked): Checks C8.5 complete original compiler acceptance/decline frontiers.
- [rust/tcl-registry/src/native_namespace_upvar_compilation.rs](../../../../rust/tcl-registry/src/native_namespace_upvar_compilation.rs), `native_namespace_upvar_compilation::tests::original_modern_namespace_upvar_matches_45_native_compiler_frontiers` (linked): Checks modern independently selected workers and original word vectors.

## Evidence

- [rust/tcl-registry/tests/data/native_namespace_upvar_compilation/provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/provenance.json)
- [rust/tcl-registry/tests/data/native_namespace_upvar_compilation/modern-provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/modern-provenance.json)
- [rust/tcl-registry/tests/data/native_namespace_upvar_compilation/cases.rs](../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/cases.rs)
- [rust/tcl-registry/tests/data/native_namespace_upvar_compilation/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/8.5.19.tsv)
- [rust/tcl-registry/tests/data/native_namespace_upvar_compilation/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/8.6.18.tsv)
- [rust/tcl-registry/tests/data/native_namespace_upvar_compilation/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/9.0.4.tsv)
- [rust/tcl-registry/tests/data/native_namespace_upvar_compilation/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/9.1.0.tsv)
