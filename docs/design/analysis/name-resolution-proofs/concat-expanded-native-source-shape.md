# naming.concat-expanded-native-source-shape

Kind: `native-observation`

## Problem statement

A literal expansion marker, parsed expansion word and runtime expanded argument list are different producers. Reconstructing argv from source text can change grammar, allocation and result identity.

## Question

How do native parser and compiler routes treat the original concat expansion-shaped sources and their result owners?

## Conclusion

The recovered C/Jim probe outputs retain sourcehex, parser completion and result/cache identity windows separately. C8.4 parser failures and other release-specific expansion results remain exact. Successful source parsing or concat completion does not authenticate a captured argv producer.

## Scope

Original probe.c/jim-probe.c sources and complete six-provider native output; native source shape and value/physical result windows kept distinct.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded executable SHA-256 fe4b0e878869928b254af27f066748db2b8fbe204c0074eb05365a405cd4aa4e. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded executable SHA-256 1f36bfe95411f37c0c3e7427303b9679de64eae42385bc9baf20231645a010df. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded executable SHA-256 c5dc0d7dc4fff27a9faac950bb24c67033e2b594d47cca7890c746d4695f0641. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable SHA-256 272ce951aa081819d5a2cc37ecc97d2b2329b59d2af6431e08dda7b5f3f376de. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable SHA-256 a3c87dc97131e71da22b6d5aec346d5e1610e6130b3e68a87c363974e18bfc88. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `observed`. Version: jim0.84. Build: Recorded executable SHA-256 9db07b437cce6fbb8178ca5fb3d6b8764ac83cf58ed02bc0bef092a47b91888d. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Jim Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Exact evidence

- `receipt-0` (observation): [rust/tcl-registry/tests/data/native_concat_expansion/manifest.json](../../../../rust/tcl-registry/tests/data/native_concat_expansion/manifest.json). SHA-256 `fdb9fb79f504324d0bf635573868a54990a420572d286f058a8c78a2c9071459`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-registry/tests/data/native_concat_expansion/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_concat_expansion/8.4.20.tsv). SHA-256 `4f26f1b595bd889dc657d0f4a41ad08ceb04b22532ff2b06f8c1709d70a62ee0`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-registry/tests/data/native_concat_expansion/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_concat_expansion/8.5.19.tsv). SHA-256 `a5576704e2e3dadb6a59f227aa48e6de1296486cfc72c621e14d4083a75f702b`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-registry/tests/data/native_concat_expansion/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_concat_expansion/8.6.18.tsv). SHA-256 `55a9c545fd704ed51143cd702274b44a809a9ceee0bca069c1b287dd1c3ebfc6`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-registry/tests/data/native_concat_expansion/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_concat_expansion/9.0.4.tsv). SHA-256 `65f2ba830f3c3b905e9a041ac6d61d2e72171b94332dde1173806e469ff552e7`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-registry/tests/data/native_concat_expansion/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_concat_expansion/9.1.0.tsv). SHA-256 `a671d34ffd42e513c42b0531f2d143c8cd602ef9bf7a894107eac9772f2bdfb8`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (input): [rust/tcl-registry/tests/data/native_concat_expansion/cases.rs](../../../../rust/tcl-registry/tests/data/native_concat_expansion/cases.rs). SHA-256 `b7785e396ad8e03f9d90172b671cbd13bd6ebc6a677fdc8752f74d01e9c3b8ee`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (input): [rust/tcl-registry/tests/data/native_concat_expansion/jim-probe.c](../../../../rust/tcl-registry/tests/data/native_concat_expansion/jim-probe.c). SHA-256 `99dd877e8b39ba00cb52a28881616469fa403f1ea61c588c8a3d5f243ef1d879`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-registry/tests/data/native_concat_expansion/jim0.84.tsv](../../../../rust/tcl-registry/tests/data/native_concat_expansion/jim0.84.tsv). SHA-256 `58ca990b0d3a5af025695c3d5bcc671a3c29c16f4e8145e6e5c8e72e2e3f191a`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (input): [rust/tcl-registry/tests/data/native_concat_expansion/probe.c](../../../../rust/tcl-registry/tests/data/native_concat_expansion/probe.c). SHA-256 `5bdcde696ff547995c5ec051c24e0cbb5de702c9c1f1f0dba0a330efc1afb11c`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
