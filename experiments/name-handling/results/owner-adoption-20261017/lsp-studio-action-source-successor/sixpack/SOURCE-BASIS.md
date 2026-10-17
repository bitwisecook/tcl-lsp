# Six-ingredient source composition

Patch SHA-256: `96e5499273d60a7019d96d086af44ee19022e8a660b89100d1b6b67be054c852`.

The 29-file composition includes exactly705,569,724,601,691V3 and xtaskSyn3,
in that order. Ingredient manifests, exact patch hashes and independent review
receipt hashes are retained in manifest.json. Every retained ingredient before
image matched current Main and its own declared hash. Every ingredient patch
was independently replayed against its exact before images and matched all
retained after images.

Composition uses independent common-base line changes. Non-insertion overlaps
fail closed. The only coincident insertion points were contract end appends:
name-handling.md705 then569 then691V3; shared-utility-contracts-rust.md705 then691V3.
The exact original insertion bytes are concatenated in that order. Other edits
are disjoint. No semantic changes or conflict repairs were introduced.
composition.json records every source interval, ingredient and replacement hash.

The joint patch fresh forward replay matched all29 exact after images; inverse
replay restored all original files and removed all newly created files. Current
Main before images remained unchanged throughout.703 is excluded pending peer
clearance. No source/count/Git/test/provider changes were made to Main.

Scoped rustfmt --check: exit 0 over 17 Rust files; see format-check.log.
