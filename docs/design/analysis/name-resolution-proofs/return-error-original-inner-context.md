# naming.return-error-original-inner-context

Kind: `native-observation`

## Problem statement

An Error result may share or differ from its registered message and inner context. Reading options or errorInfo after the primary snapshot can change those original owners.

## Question

Which original message/literal/innerContext identity survives the fifteen ReturnImmediate and compiled Error bodies?

## Conclusion

The exact return-context rows preserve original result/literal/innerContext identity for each selected native body. C8.4 absence of modern context rows is retained rather than filled by a later release. These windows do not certify an arbitrary source Return or callback completion.

## Scope

Five C matching-private-header probes of return-context.c, original15 bodies, snapshots before observers.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded executable SHA-256 5d5ee8e13f9cb7d1aab37a2a7937a529518ec33c2102279566040986aac8260a. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"context_rows": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded executable SHA-256 10eea70cd9f4d0e84265a3e2edef789d9fb9c3ece059d5cee4b6c7cc1bcbc9cf. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"context_rows": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded executable SHA-256 324924e1d75c46eefc63dd5506b9a811619664b1d14d760a4c67ca577ed0eb67. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"context_rows": 9, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable SHA-256 2941ef226a2dbdcc89d5cd9e1b43b8c745a9df7e0242ae5c1fb6cb6e93e1aa7d. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"context_rows": 9, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable SHA-256 8a866bc47488a322cc5680e11f7d6b038915215df983707ea4ac42625eff70f7. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"context_rows": 9, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Exact evidence

- `receipt-0` (observation): [rust/tcl-registry/tests/data/native_control_expression/return-context-manifest.json](../../../../rust/tcl-registry/tests/data/native_control_expression/return-context-manifest.json). SHA-256 `51c69b8dd20baac24b8e137a9081ba4f35618b4f4626ee74e6e0301dba79ee82`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-registry/tests/data/native_control_expression/catch-publication6.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/catch-publication6.tsv). SHA-256 `5d4422ff5d20653805aa7e89ea792ad121f528fee787141c3c8081975d9f9fb5`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-registry/tests/data/native_control_expression/catch4.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/catch4.tsv). SHA-256 `13588e6d176b3564ef99acd80e196a2f80b8fdf4510c0815030eb1a791fb4617`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (input): [rust/tcl-registry/tests/data/native_control_expression/compound40.tcl](../../../../rust/tcl-registry/tests/data/native_control_expression/compound40.tcl). SHA-256 `c6dc99811ac803593930d778e5fe09e7fbd9707f81d4bf9f12604586fc13f6d8`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-registry/tests/data/native_control_expression/compound40.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/compound40.tsv). SHA-256 `7881a3ba2147109424c65769a858fa6a404a086356b2d5823dd3c15a3eb52795`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (input): [rust/tcl-registry/tests/data/native_control_expression/control75.tcl](../../../../rust/tcl-registry/tests/data/native_control_expression/control75.tcl). SHA-256 `e2c569f12e07c1023957acf1d81eb075e8b4c7941f5b8958a00b40ff3f6dd946`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (observation): [rust/tcl-registry/tests/data/native_control_expression/control75.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/control75.tsv). SHA-256 `b72364c49fdc10c762950a94fa1670f2aedc0a31399450fdd37344007288695f`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (input): [rust/tcl-registry/tests/data/native_control_expression/folded-boolean8.c](../../../../rust/tcl-registry/tests/data/native_control_expression/folded-boolean8.c). SHA-256 `790a5c6f78e5011ac7c0f5b4e7383594cdf13aabcd331f00d36d103fe42a7621`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-registry/tests/data/native_control_expression/folded-boolean8.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/folded-boolean8.tsv). SHA-256 `675f499aad9332a071f333ff8ad1b99247393c0a12c1007031b0ea3cb0ac2a5f`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (input): [rust/tcl-registry/tests/data/native_control_expression/logical84.c](../../../../rust/tcl-registry/tests/data/native_control_expression/logical84.c). SHA-256 `6ad756616dc91a409680bb24fc15358291a2421fbda864be4e33dae9421531ff`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-10` (observation): [rust/tcl-registry/tests/data/native_control_expression/logical84.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/logical84.tsv). SHA-256 `97f06e66799e9c4dee90462fed8af955dfa0692df4fb97e8359b6ecf7073f6e7`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-11` (observation): [rust/tcl-registry/tests/data/native_control_expression/return-context-8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/return-context-8.4.20.tsv). SHA-256 `ef4edac7d78c4f052857f87c5aa8a6cc52383a85cd5d8d659090fdda9553197c`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-12` (observation): [rust/tcl-registry/tests/data/native_control_expression/return-context-8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/return-context-8.5.19.tsv). SHA-256 `8a348786e768589f3ae175ab12dc2edcb3e3e2062021258c3c3c8e09a1bab214`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-13` (observation): [rust/tcl-registry/tests/data/native_control_expression/return-context-8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/return-context-8.6.18.tsv). SHA-256 `2f1c6cd50982c1188724e9daca7ffa57e7a96b32e358e7f332db49d726664756`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-14` (observation): [rust/tcl-registry/tests/data/native_control_expression/return-context-9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/return-context-9.0.4.tsv). SHA-256 `7e380a332dc6c7b06d04db3ce742b4d58db69a1d1ac06f493dece5787d13d73e`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-15` (observation): [rust/tcl-registry/tests/data/native_control_expression/return-context-9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/return-context-9.1.0.tsv). SHA-256 `7e380a332dc6c7b06d04db3ce742b4d58db69a1d1ac06f493dece5787d13d73e`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-16` (input): [rust/tcl-registry/tests/data/native_control_expression/return-context.c](../../../../rust/tcl-registry/tests/data/native_control_expression/return-context.c). SHA-256 `8b84762eeee042f6092eb8f80bb62f67c202b74b2161fd0bba56871f10484a3f`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-17` (input): [rust/tcl-registry/tests/data/native_control_expression/storage40.c](../../../../rust/tcl-registry/tests/data/native_control_expression/storage40.c). SHA-256 `d587b8f667589f8752031b01b235ec4bdb3f7318fcb3528ec375d752f1d3e724`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-18` (observation): [rust/tcl-registry/tests/data/native_control_expression/storage40.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/storage40.tsv). SHA-256 `53829a91df7b3e1356fadb8541cdb766c5e63df782c4b0e63d611563e4d5b680`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-19` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-context-8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-context-8.4.20.tsv). SHA-256 `6119824be9e34629e03ac4c5756823300e16b051b73aabb22294adee2f0f487e`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-20` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-context-8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-context-8.5.19.tsv). SHA-256 `72b7a970d673bbea3d07e7fd90e33e1fe78669cf9e4f338463cf530374306624`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-21` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-context-8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-context-8.6.18.tsv). SHA-256 `3716bf2e3d607acc7a0f7182007d2c5016344bbbea1edfd8ee85f37b11f5928a`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-22` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-context-9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-context-9.0.4.tsv). SHA-256 `07cd6c72120ed306ed2b4736c47b4ba47c0422eb13267940280f7862043c0737`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-23` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-context-9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-context-9.1.0.tsv). SHA-256 `a429b61f021e111f4e4c1acb79e2f416922190966f014a540bef56d3ad91ddfa`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-24` (input): [rust/tcl-registry/tests/data/native_control_expression/syntax-context.c](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-context.c). SHA-256 `355df31612c7da9b2c774af15906a40269f8c48cbd77fec7eebf59951f231b2e`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-25` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-lifecycle-9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-lifecycle-9.1.0.tsv). SHA-256 `37ab04dd7316a67d1e7d55b2d0582ee73d3bc08ecafd68bfcb3d2b7998c21308`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-26` (input): [rust/tcl-registry/tests/data/native_control_expression/syntax-lifecycle.c](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-lifecycle.c). SHA-256 `dfae9c940ab97ea5846bd7e5ab94a115634bb4c76d2ed682bffc4473f5d2db82`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-27` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-owners-8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-owners-8.6.18.tsv). SHA-256 `f2a978db77793ba4f1bea90d1f6c06f78dd30e664c7c5c398d3632cc555918d0`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-28` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-owners-9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-owners-9.0.4.tsv). SHA-256 `d181bdc61d42b5e8d9376ac2dd3650fc04f7ecfc632c779dab44cec186d56855`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-29` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-owners-9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-owners-9.1.0.tsv). SHA-256 `07d40f2bc875af0f614799ce60bd1fd63dc04f14191ab250ef1b2e8f957ec89e`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-30` (input): [rust/tcl-registry/tests/data/native_control_expression/syntax-owners.c](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-owners.c). SHA-256 `64e9d956f18671c0f87ae1826c451385f41ea78b8f92fda8078cb7fb419ce1b2`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
