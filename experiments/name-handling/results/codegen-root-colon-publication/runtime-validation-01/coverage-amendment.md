# Compiled input coverage

[The post-capture dep-info audit](compiled-input-coverage-supplement.json)
identifies 2,574 normalised local compiled inputs. The original before/after
manifests cover 2,495. The remaining79 are supplemental:78 are byte-identical
committed fixtures at the captured Git head (primarily extensionless oracle
stdout and two external F5 text records), and one is generated tcl-dialect
source in `OUT_DIR`.

These79 inputs were hashed after execution. This supplement does not establish
independent before/after stability for them, and the original suffix-filtered
15,175-input capture must not be described as a full compiled-input freeze.
The original manifests, Runtime6 test transcripts, executable hashes and seal
are unchanged. Six observed Runtime software passes retain their scoped meaning;
no complete compiler, Native receipt or WASM acceptance follows.
