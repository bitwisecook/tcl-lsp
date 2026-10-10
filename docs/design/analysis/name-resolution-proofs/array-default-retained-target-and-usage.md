# naming.array.default-retained-target-and-usage

Kind: `native-observation`

## Problem statement

An ARRAY callback can retarget a local alias or destroy and recreate its target. Resolving the variable once, resolving it again, and retaining an ensemble usage header have different observable results.

## Question

What target, default value and wrong-argument header do the original array-default controls observe before and after callbacks and public/private dispatch?

## Conclusion

The retained Tcl 9 controls distinguish alias retargeting from deletion/recreation, and literal private-worker usage from dynamic ensemble usage. Callback evaluation can change the target and clear the outer rewrite. The exact state and diagnostic bytes are attached; they establish neither private allocator identity nor a trace-free Rust operation.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.4.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.5.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.6.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl9.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`usage-manifest.json` (`file-2-row-3`):

```json
{
  "version": "9.0.4",
  "case": "literal_narrow",
  "result_hex": "77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d6522",
  "exit": 0
}
```

`usage-manifest.json` (`file-2-row-4`):

```json
{
  "version": "9.0.4",
  "case": "dynamic_narrow",
  "result_hex": "77726f6e67202320617267733a2073686f756c64206265202261727261792064656661756c74206765742061727261794e616d6522",
  "exit": 0
}
```

`usage-manifest.json` (`file-2-row-5`):

```json
{
  "version": "9.0.4",
  "case": "private_narrow",
  "result_hex": "77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d6522",
  "exit": 0
}
```

`usage-manifest.json` (`file-2-row-6`):

```json
{
  "version": "9.0.4",
  "case": "dynamic_array_trace",
  "result_hex": "77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d6522",
  "exit": 0
}
```

`usage-manifest.json` (`file-2-row-7`):

```json
{
  "version": "9.0.4",
  "case": "literal_array_trace",
  "result_hex": "77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d6522",
  "exit": 0
}
```

`usage-manifest.json` (`file-2-row-8`):

```json
{
  "version": "9.0.4",
  "case": "dynamic_broad",
  "result_hex": "77726f6e67202320617267733a2073686f756c64206265202261727261792064656661756c74206f7074696f6e2061727261794e616d65203f76616c75653f22",
  "exit": 0
}
```

`usage-manifest.json` (`file-2-row-9`):

```json
{
  "version": "9.0.4",
  "case": "literal_broad",
  "result_hex": "77726f6e67202320617267733a2073686f756c64206265202261727261792064656661756c74206f7074696f6e2061727261794e616d65203f76616c75653f22",
  "exit": 0
}
```

`usage-manifest.json` (`file-2-row-10`):

```json
{
  "version": "9.0.4",
  "case": "callback_isolation",
  "result_hex": "7b77726f6e67202320617267733a2073686f756c642062652022736574207661724e616d65203f6e657756616c75653f227d207b77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d65227d",
  "exit": 0
}
```

`state controls` (`file-0`):

```json
{
  "rows": [
    {
      "case": "alias-retarget-get",
      "result_hex": "30204f4c442061727261792030204f4c442030204e4557"
    },
    {
      "case": "delete-recreate-get",
      "result_hex": "30204e45572061727261792030204e4557"
    },
    {
      "case": "missing-created-get",
      "result_hex": "30204e45572061727261792030204e4557"
    },
    {
      "case": "alias-retarget-set",
      "result_hex": "30207b7d2061727261792030204f4c4420302056414c5545"
    },
    {
      "case": "delete-recreate-set",
      "result_hex": "30207b7d20617272617920302056414c5545"
    },
    {
      "case": "missing-created-set",
      "result_hex": "30207b7d20617272617920302056414c5545"
    },
    {
      "case": "alias-retarget-exists",
      "result_hex": "3020312061727261792030204f4c442030204e4557"
    },
    {
      "case": "delete-recreate-exists",
      "result_hex": "3020312061727261792030204e4557"
    },
    {
      "case": "missing-created-exists",
      "result_hex": "3020312061727261792030204e4557"
    },
    {
      "case": "alias-retarget-unset",
      "result_hex": "30207b7d2061727261792031207b617272617920686173206e6f2064656661756c742076616c75657d2030204e4557"
    },
    {
      "case": "delete-recreate-unset",
      "result_hex": "30207b7d2061727261792031207b617272617920686173206e6f2064656661756c742076616c75657d"
    },
    {
      "case": "missing-created-unset",
      "result_hex": "30207b7d2061727261792031207b617272617920686173206e6f2064656661756c742076616c75657d"
    },
    {
      "case": "option-arity-after-trace-get",
      "result_hex": "31207b77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d65227d206172726179207b54434c2057524f4e47415247537d"
    },
    {
      "case": "option-arity-after-trace-set",
      "result_hex": "31207b77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74207365742061727261794e616d652076616c7565227d206172726179207b54434c2057524f4e47415247537d"
    },
    {
      "case": "option-arity-after-trace-exists",
      "result_hex": "31207b77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206578697374732061727261794e616d65227d206172726179207b54434c2057524f4e47415247537d"
    },
    {
      "case": "option-arity-after-trace-unset",
      "result_hex": "31207b77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c7420756e7365742061727261794e616d65227d206172726179207b54434c2057524f4e47415247537d"
    },
    {
      "case": "missing-get",
      "result_hex": "312032323631323232303639373336653237373432303631366532303631373237323631373920353434333463323034633466346634623535353032303431353235323431353932303631"
    },
    {
      "case": "missing-set",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "missing-exists",
      "result_hex": "30203330207b7d"
    },
    {
      "case": "missing-unset",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "scalar-get",
      "result_hex": "312032323631323232303639373336653237373432303631366532303631373237323631373920353434333463323034633466346634623535353032303431353235323431353932303631"
    },
    {
      "case": "scalar-set",
      "result_hex": "3120363336313665323737343230363137323732363137393230363436353636363137353663373432303733363537343230323236313232336132303736363137323639363136323663363532303639373336653237373432303631373237323631373920353434333463323035373532343935343435323034313532353234313539"
    },
    {
      "case": "scalar-exists",
      "result_hex": "312032323631323232303639373336653237373432303631366532303631373237323631373920353434333463323034633466346634623535353032303431353235323431353932303631"
    },
    {
      "case": "scalar-unset",
      "result_hex": "312032323631323232303639373336653237373432303631366532303631373237323631373920353434333463323034633466346634623535353032303431353235323431353932303631"
    },
    {
      "case": "array-get",
      "result_hex": "312036313732373236313739323036383631373332303665366632303634363536363631373536633734323037363631366337353635203534343334633230353234353431343432303431353235323431353932303434343534363431353534633534"
    },
    {
      "case": "array-set",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "array-exists",
      "result_hex": "30203330207b7d"
    },
    {
      "case": "array-unset",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "element-get",
      "result_hex": "312032323631323837383239323232303639373336653237373432303631366532303631373237323631373920353434333463323034633466346634623535353032303431353235323431353932303631323837383239"
    },
    {
      "case": "element-set",
      "result_hex": "312036333631366532373734323036313732373236313739323036343635363636313735366337343230373336353734323032323631323837383239323233613230373636313732363936313632366336353230363937333665323737343230363137323732363137392035343433346332303463346634663462353535303230353634313532346534313464343532303631323837383239"
    },
    {
      "case": "element-exists",
      "result_hex": "30203330207b7d"
    },
    {
      "case": "element-unset",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "missing-namespace-get",
      "result_hex": "3120323233613361366536663665363533613361363132323230363937333665323737343230363136653230363137323732363137392035343433346332303463346634663462353535303230343135323532343135393230336133613665366636653635336133613631"
    },
    {
      "case": "missing-namespace-set",
      "result_hex": "31203633363136653237373432303631373237323631373932303634363536363631373536633734323037333635373432303232336133613665366636653635336133613631323233613230373036313732363536653734323036653631366436353733373036313633363532303634366636353733366532373734323036353738363937333734203534343334633230346334663466346235353530323035363431353234653431346434353230336133613665366636653635336133613631"
    },
    {
      "case": "missing-namespace-exists",
      "result_hex": "30203330207b7d"
    },
    {
      "case": "missing-namespace-unset",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "literal-nul-get",
      "result_hex": "3120323236313030363232323230363937333665323737343230363136653230363137323732363137392035343433346332303463346634663462353535303230343135323532343135393230363130303632"
    },
    {
      "case": "literal-nul-set",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "literal-nul-exists",
      "result_hex": "30203330207b7d"
    },
    {
      "case": "literal-nul-unset",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "opaque-get",
      "result_hex": "3120323236316666363232323230363937333665323737343230363136653230363137323732363137392035343433346332303463346634663462353535303230343135323532343135393230363166663632"
    },
    {
      "case": "opaque-set",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "opaque-exists",
      "result_hex": "30203330207b7d"
    },
    {
      "case": "opaque-unset",
      "result_hex": "30207b7d207b7d"
    }
  ]
}
```

`controls.tsv`: selected capture `file-0` retains 24484 characters of per-input outcomes; use its exact evidence link for all rows.

`usage.tsv` (`file-20`):

```json
{
  "provider_rows": [
    "9.0.4\tliteral_narrow\t6361746368207b61727261792064656661756c742067657420612045585452417d20723b7365742072\t77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d6522",
    "9.0.4\tdynamic_narrow\t73657420636d642061727261793b6361746368207b24636d642064656661756c742067657420612045585452417d20723b7365742072\t77726f6e67202320617267733a2073686f756c64206265202261727261792064656661756c74206765742061727261794e616d6522",
    "9.0.4\tprivate_narrow\t6361746368207b3a3a74636c3a3a61727261793a3a64656661756c742067657420612045585452417d20723b7365742072\t77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d6522",
    "9.0.4\tdynamic_array_trace\t73657420636d642061727261793b61727261792064656661756c74207365742061204f4c443b70726f63207761746368207b6e206b206f707d207b7d3b747261636520616464207661726961626c6520612061727261792077617463683b6361746368207b24636d642064656661756c742067657420612045585452417d20723b7365742072\t77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d6522",
    "9.0.4\tliteral_array_trace\t61727261792064656661756c74207365742061204f4c443b70726f63207761746368207b6e206b206f707d207b7d3b747261636520616464207661726961626c6520612061727261792077617463683b6361746368207b61727261792064656661756c742067657420612045585452417d20723b7365742072\t77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d6522",
    "9.0.4\tdynamic_broad\t73657420636d642061727261793b6361746368207b24636d642064656661756c74206765747d20723b7365742072\t77726f6e67202320617267733a2073686f756c64206265202261727261792064656661756c74206f7074696f6e2061727261794e616d65203f76616c75653f22",
    "9.0.4\tliteral_broad\t6361746368207b61727261792064656661756c74206765747d20723b7365742072\t77726f6e67202320617267733a2073686f756c64206265202261727261792064656661756c74206f7074696f6e2061727261794e616d65203f76616c75653f22",
    "9.0.4\tcallback_isolation\t73657420636d642061727261793b61727261792064656661756c74207365742061204f4c443b70726f63207761746368207b6e206b206f707d207b6361746368207b7365747d203a3a696e736964657d3b747261636520616464207661726961626c6520612061727261792077617463683b6361746368207b24636d642064656661756c742067657420612045585452417d20723b6c6973742024696e73696465202472\t7b77726f6e67202320617267733a2073686f756c642062652022736574207661724e616d65203f6e657756616c75653f227d207b77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d65227d"
  ]
}
```

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl9.1.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`usage-manifest.json` (`file-2-row-11`):

```json
{
  "version": "9.1.0",
  "case": "literal_narrow",
  "result_hex": "77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d6522",
  "exit": 0
}
```

`usage-manifest.json` (`file-2-row-12`):

```json
{
  "version": "9.1.0",
  "case": "dynamic_narrow",
  "result_hex": "77726f6e67202320617267733a2073686f756c64206265202261727261792064656661756c74206765742061727261794e616d6522",
  "exit": 0
}
```

`usage-manifest.json` (`file-2-row-13`):

```json
{
  "version": "9.1.0",
  "case": "private_narrow",
  "result_hex": "77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d6522",
  "exit": 0
}
```

`usage-manifest.json` (`file-2-row-14`):

```json
{
  "version": "9.1.0",
  "case": "dynamic_array_trace",
  "result_hex": "77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d6522",
  "exit": 0
}
```

`usage-manifest.json` (`file-2-row-15`):

```json
{
  "version": "9.1.0",
  "case": "literal_array_trace",
  "result_hex": "77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d6522",
  "exit": 0
}
```

`usage-manifest.json` (`file-2-row-16`):

```json
{
  "version": "9.1.0",
  "case": "dynamic_broad",
  "result_hex": "77726f6e67202320617267733a2073686f756c64206265202261727261792064656661756c74206f7074696f6e2061727261794e616d65203f76616c75653f22",
  "exit": 0
}
```

`usage-manifest.json` (`file-2-row-17`):

```json
{
  "version": "9.1.0",
  "case": "literal_broad",
  "result_hex": "77726f6e67202320617267733a2073686f756c64206265202261727261792064656661756c74206f7074696f6e2061727261794e616d65203f76616c75653f22",
  "exit": 0
}
```

`usage-manifest.json` (`file-2-row-18`):

```json
{
  "version": "9.1.0",
  "case": "callback_isolation",
  "result_hex": "7b77726f6e67202320617267733a2073686f756c642062652022736574207661724e616d65203f6e657756616c75653f227d207b77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d65227d",
  "exit": 0
}
```

`state controls` (`file-0`):

```json
{
  "rows": [
    {
      "case": "alias-retarget-get",
      "result_hex": "30204f4c442061727261792030204f4c442030204e4557"
    },
    {
      "case": "delete-recreate-get",
      "result_hex": "30204e45572061727261792030204e4557"
    },
    {
      "case": "missing-created-get",
      "result_hex": "30204e45572061727261792030204e4557"
    },
    {
      "case": "alias-retarget-set",
      "result_hex": "30207b7d2061727261792030204f4c4420302056414c5545"
    },
    {
      "case": "delete-recreate-set",
      "result_hex": "30207b7d20617272617920302056414c5545"
    },
    {
      "case": "missing-created-set",
      "result_hex": "30207b7d20617272617920302056414c5545"
    },
    {
      "case": "alias-retarget-exists",
      "result_hex": "3020312061727261792030204f4c442030204e4557"
    },
    {
      "case": "delete-recreate-exists",
      "result_hex": "3020312061727261792030204e4557"
    },
    {
      "case": "missing-created-exists",
      "result_hex": "3020312061727261792030204e4557"
    },
    {
      "case": "alias-retarget-unset",
      "result_hex": "30207b7d2061727261792031207b617272617920686173206e6f2064656661756c742076616c75657d2030204e4557"
    },
    {
      "case": "delete-recreate-unset",
      "result_hex": "30207b7d2061727261792031207b617272617920686173206e6f2064656661756c742076616c75657d"
    },
    {
      "case": "missing-created-unset",
      "result_hex": "30207b7d2061727261792031207b617272617920686173206e6f2064656661756c742076616c75657d"
    },
    {
      "case": "option-arity-after-trace-get",
      "result_hex": "31207b77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d65227d206172726179207b54434c2057524f4e47415247537d"
    },
    {
      "case": "option-arity-after-trace-set",
      "result_hex": "31207b77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74207365742061727261794e616d652076616c7565227d206172726179207b54434c2057524f4e47415247537d"
    },
    {
      "case": "option-arity-after-trace-exists",
      "result_hex": "31207b77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206578697374732061727261794e616d65227d206172726179207b54434c2057524f4e47415247537d"
    },
    {
      "case": "option-arity-after-trace-unset",
      "result_hex": "31207b77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c7420756e7365742061727261794e616d65227d206172726179207b54434c2057524f4e47415247537d"
    },
    {
      "case": "missing-get",
      "result_hex": "312032323631323232303639373336653237373432303631366532303631373237323631373920353434333463323034633466346634623535353032303431353235323431353932303631"
    },
    {
      "case": "missing-set",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "missing-exists",
      "result_hex": "30203330207b7d"
    },
    {
      "case": "missing-unset",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "scalar-get",
      "result_hex": "312032323631323232303639373336653237373432303631366532303631373237323631373920353434333463323034633466346634623535353032303431353235323431353932303631"
    },
    {
      "case": "scalar-set",
      "result_hex": "3120363336313665323737343230363137323732363137393230363436353636363137353663373432303733363537343230323236313232336132303736363137323639363136323663363532303639373336653237373432303631373237323631373920353434333463323035373532343935343435323034313532353234313539"
    },
    {
      "case": "scalar-exists",
      "result_hex": "312032323631323232303639373336653237373432303631366532303631373237323631373920353434333463323034633466346634623535353032303431353235323431353932303631"
    },
    {
      "case": "scalar-unset",
      "result_hex": "312032323631323232303639373336653237373432303631366532303631373237323631373920353434333463323034633466346634623535353032303431353235323431353932303631"
    },
    {
      "case": "array-get",
      "result_hex": "312036313732373236313739323036383631373332303665366632303634363536363631373536633734323037363631366337353635203534343334633230353234353431343432303431353235323431353932303434343534363431353534633534"
    },
    {
      "case": "array-set",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "array-exists",
      "result_hex": "30203330207b7d"
    },
    {
      "case": "array-unset",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "element-get",
      "result_hex": "312032323631323837383239323232303639373336653237373432303631366532303631373237323631373920353434333463323034633466346634623535353032303431353235323431353932303631323837383239"
    },
    {
      "case": "element-set",
      "result_hex": "312036333631366532373734323036313732373236313739323036343635363636313735366337343230373336353734323032323631323837383239323233613230373636313732363936313632366336353230363937333665323737343230363137323732363137392035343433346332303463346634663462353535303230353634313532346534313464343532303631323837383239"
    },
    {
      "case": "element-exists",
      "result_hex": "30203330207b7d"
    },
    {
      "case": "element-unset",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "missing-namespace-get",
      "result_hex": "3120323233613361366536663665363533613361363132323230363937333665323737343230363136653230363137323732363137392035343433346332303463346634663462353535303230343135323532343135393230336133613665366636653635336133613631"
    },
    {
      "case": "missing-namespace-set",
      "result_hex": "31203633363136653237373432303631373237323631373932303634363536363631373536633734323037333635373432303232336133613665366636653635336133613631323233613230373036313732363536653734323036653631366436353733373036313633363532303634366636353733366532373734323036353738363937333734203534343334633230346334663466346235353530323035363431353234653431346434353230336133613665366636653635336133613631"
    },
    {
      "case": "missing-namespace-exists",
      "result_hex": "30203330207b7d"
    },
    {
      "case": "missing-namespace-unset",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "literal-nul-get",
      "result_hex": "3120323236313030363232323230363937333665323737343230363136653230363137323732363137392035343433346332303463346634663462353535303230343135323532343135393230363130303632"
    },
    {
      "case": "literal-nul-set",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "literal-nul-exists",
      "result_hex": "30203330207b7d"
    },
    {
      "case": "literal-nul-unset",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "opaque-get",
      "result_hex": "3120323236316666363232323230363937333665323737343230363136653230363137323732363137392035343433346332303463346634663462353535303230343135323532343135393230363166663632"
    },
    {
      "case": "opaque-set",
      "result_hex": "30207b7d207b7d"
    },
    {
      "case": "opaque-exists",
      "result_hex": "30203330207b7d"
    },
    {
      "case": "opaque-unset",
      "result_hex": "30207b7d207b7d"
    }
  ]
}
```

`controls.tsv`: selected capture `file-0` retains 24484 characters of per-input outcomes; use its exact evidence link for all rows.

`usage.tsv` (`file-20`):

```json
{
  "provider_rows": [
    "9.1.0\tliteral_narrow\t6361746368207b61727261792064656661756c742067657420612045585452417d20723b7365742072\t77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d6522",
    "9.1.0\tdynamic_narrow\t73657420636d642061727261793b6361746368207b24636d642064656661756c742067657420612045585452417d20723b7365742072\t77726f6e67202320617267733a2073686f756c64206265202261727261792064656661756c74206765742061727261794e616d6522",
    "9.1.0\tprivate_narrow\t6361746368207b3a3a74636c3a3a61727261793a3a64656661756c742067657420612045585452417d20723b7365742072\t77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d6522",
    "9.1.0\tdynamic_array_trace\t73657420636d642061727261793b61727261792064656661756c74207365742061204f4c443b70726f63207761746368207b6e206b206f707d207b7d3b747261636520616464207661726961626c6520612061727261792077617463683b6361746368207b24636d642064656661756c742067657420612045585452417d20723b7365742072\t77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d6522",
    "9.1.0\tliteral_array_trace\t61727261792064656661756c74207365742061204f4c443b70726f63207761746368207b6e206b206f707d207b7d3b747261636520616464207661726961626c6520612061727261792077617463683b6361746368207b61727261792064656661756c742067657420612045585452417d20723b7365742072\t77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d6522",
    "9.1.0\tdynamic_broad\t73657420636d642061727261793b6361746368207b24636d642064656661756c74206765747d20723b7365742072\t77726f6e67202320617267733a2073686f756c64206265202261727261792064656661756c74206f7074696f6e2061727261794e616d65203f76616c75653f22",
    "9.1.0\tliteral_broad\t6361746368207b61727261792064656661756c74206765747d20723b7365742072\t77726f6e67202320617267733a2073686f756c64206265202261727261792064656661756c74206f7074696f6e2061727261794e616d65203f76616c75653f22",
    "9.1.0\tcallback_isolation\t73657420636d642061727261793b61727261792064656661756c74207365742061204f4c443b70726f63207761746368207b6e206b206f707d207b6361746368207b7365747d203a3a696e736964657d3b747261636520616464207661726961626c6520612061727261792077617463683b6361746368207b24636d642064656661756c742067657420612045585452417d20723b6c6973742024696e73696465202472\t7b77726f6e67202320617267733a2073686f756c642062652022736574207661724e616d65203f6e657756616c75653f227d207b77726f6e67202320617267733a2073686f756c6420626520223a3a74636c3a3a61727261793a3a64656661756c74206765742061727261794e616d65227d"
  ]
}
```

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [runtime/rust/tests/data/native_array_default/controls.tsv](../../../../runtime/rust/tests/data/native_array_default/controls.tsv). SHA-256 `063a0e4a8789905903f3fb0bd0f38e5f3819d18f773155c0446c8d6176f61f87`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [runtime/rust/tests/data/native_array_default/manifest.json](../../../../runtime/rust/tests/data/native_array_default/manifest.json). SHA-256 `f27737da08bae58ccc6e60897b7341280456db0aac1123f7be830622fc995bf6`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [runtime/rust/tests/data/native_array_default/usage-manifest.json](../../../../runtime/rust/tests/data/native_array_default/usage-manifest.json). SHA-256 `87296c778f069e94eea6ce509a2ce07abe480e13c3b9528d801b2c28a54d4342`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2-row-3` (provider): [runtime/rust/tests/data/native_array_default/usage-manifest.json](../../../../runtime/rust/tests/data/native_array_default/usage-manifest.json). SHA-256 `87296c778f069e94eea6ce509a2ce07abe480e13c3b9528d801b2c28a54d4342`. JSON pointer `/0`. Original provider/capture association at its exact selected row.
- `file-2-row-4` (provider): [runtime/rust/tests/data/native_array_default/usage-manifest.json](../../../../runtime/rust/tests/data/native_array_default/usage-manifest.json). SHA-256 `87296c778f069e94eea6ce509a2ce07abe480e13c3b9528d801b2c28a54d4342`. JSON pointer `/1`. Original provider/capture association at its exact selected row.
- `file-2-row-5` (provider): [runtime/rust/tests/data/native_array_default/usage-manifest.json](../../../../runtime/rust/tests/data/native_array_default/usage-manifest.json). SHA-256 `87296c778f069e94eea6ce509a2ce07abe480e13c3b9528d801b2c28a54d4342`. JSON pointer `/2`. Original provider/capture association at its exact selected row.
- `file-2-row-6` (provider): [runtime/rust/tests/data/native_array_default/usage-manifest.json](../../../../runtime/rust/tests/data/native_array_default/usage-manifest.json). SHA-256 `87296c778f069e94eea6ce509a2ce07abe480e13c3b9528d801b2c28a54d4342`. JSON pointer `/3`. Original provider/capture association at its exact selected row.
- `file-2-row-7` (provider): [runtime/rust/tests/data/native_array_default/usage-manifest.json](../../../../runtime/rust/tests/data/native_array_default/usage-manifest.json). SHA-256 `87296c778f069e94eea6ce509a2ce07abe480e13c3b9528d801b2c28a54d4342`. JSON pointer `/4`. Original provider/capture association at its exact selected row.
- `file-2-row-8` (provider): [runtime/rust/tests/data/native_array_default/usage-manifest.json](../../../../runtime/rust/tests/data/native_array_default/usage-manifest.json). SHA-256 `87296c778f069e94eea6ce509a2ce07abe480e13c3b9528d801b2c28a54d4342`. JSON pointer `/5`. Original provider/capture association at its exact selected row.
- `file-2-row-9` (provider): [runtime/rust/tests/data/native_array_default/usage-manifest.json](../../../../runtime/rust/tests/data/native_array_default/usage-manifest.json). SHA-256 `87296c778f069e94eea6ce509a2ce07abe480e13c3b9528d801b2c28a54d4342`. JSON pointer `/6`. Original provider/capture association at its exact selected row.
- `file-2-row-10` (provider): [runtime/rust/tests/data/native_array_default/usage-manifest.json](../../../../runtime/rust/tests/data/native_array_default/usage-manifest.json). SHA-256 `87296c778f069e94eea6ce509a2ce07abe480e13c3b9528d801b2c28a54d4342`. JSON pointer `/7`. Original provider/capture association at its exact selected row.
- `file-2-row-11` (provider): [runtime/rust/tests/data/native_array_default/usage-manifest.json](../../../../runtime/rust/tests/data/native_array_default/usage-manifest.json). SHA-256 `87296c778f069e94eea6ce509a2ce07abe480e13c3b9528d801b2c28a54d4342`. JSON pointer `/8`. Original provider/capture association at its exact selected row.
- `file-2-row-12` (provider): [runtime/rust/tests/data/native_array_default/usage-manifest.json](../../../../runtime/rust/tests/data/native_array_default/usage-manifest.json). SHA-256 `87296c778f069e94eea6ce509a2ce07abe480e13c3b9528d801b2c28a54d4342`. JSON pointer `/9`. Original provider/capture association at its exact selected row.
- `file-2-row-13` (provider): [runtime/rust/tests/data/native_array_default/usage-manifest.json](../../../../runtime/rust/tests/data/native_array_default/usage-manifest.json). SHA-256 `87296c778f069e94eea6ce509a2ce07abe480e13c3b9528d801b2c28a54d4342`. JSON pointer `/10`. Original provider/capture association at its exact selected row.
- `file-2-row-14` (provider): [runtime/rust/tests/data/native_array_default/usage-manifest.json](../../../../runtime/rust/tests/data/native_array_default/usage-manifest.json). SHA-256 `87296c778f069e94eea6ce509a2ce07abe480e13c3b9528d801b2c28a54d4342`. JSON pointer `/11`. Original provider/capture association at its exact selected row.
- `file-2-row-15` (provider): [runtime/rust/tests/data/native_array_default/usage-manifest.json](../../../../runtime/rust/tests/data/native_array_default/usage-manifest.json). SHA-256 `87296c778f069e94eea6ce509a2ce07abe480e13c3b9528d801b2c28a54d4342`. JSON pointer `/12`. Original provider/capture association at its exact selected row.
- `file-2-row-16` (provider): [runtime/rust/tests/data/native_array_default/usage-manifest.json](../../../../runtime/rust/tests/data/native_array_default/usage-manifest.json). SHA-256 `87296c778f069e94eea6ce509a2ce07abe480e13c3b9528d801b2c28a54d4342`. JSON pointer `/13`. Original provider/capture association at its exact selected row.
- `file-2-row-17` (provider): [runtime/rust/tests/data/native_array_default/usage-manifest.json](../../../../runtime/rust/tests/data/native_array_default/usage-manifest.json). SHA-256 `87296c778f069e94eea6ce509a2ce07abe480e13c3b9528d801b2c28a54d4342`. JSON pointer `/14`. Original provider/capture association at its exact selected row.
- `file-2-row-18` (provider): [runtime/rust/tests/data/native_array_default/usage-manifest.json](../../../../runtime/rust/tests/data/native_array_default/usage-manifest.json). SHA-256 `87296c778f069e94eea6ce509a2ce07abe480e13c3b9528d801b2c28a54d4342`. JSON pointer `/15`. Original provider/capture association at its exact selected row.
- `file-19` (input): [runtime/rust/tests/data/native_array_default/usage-sources.json](../../../../runtime/rust/tests/data/native_array_default/usage-sources.json). SHA-256 `1ad3211b7464d929ac237b9038b33e4db23f8f2e2950ae833192c04297ee9895`. Exact retained input/program bytes; purpose is limited to this question.
- `file-20` (observation): [runtime/rust/tests/data/native_array_default/usage.tsv](../../../../runtime/rust/tests/data/native_array_default/usage.tsv). SHA-256 `b884ddcfe47872c020bdb87cdd7c1160467db868ad0ffd048f3902eb0743ae6e`. Exact retained capture/provenance artifact; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
