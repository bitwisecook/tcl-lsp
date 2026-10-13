# Runtime procedure argument transport

The actual standalone Runtime builds and all six selected controls pass on the
captured source image. The five `original_definition_argv` tests check actual
namespace holders, missing-holder rejection, replacement, borrowed argument
ownership, malformed ABI input and pending Host refusal with untouched output.
The root-entry control and replacement's compiled callable use the existing
Rust software ABI stub; they establish callable/lifetime behaviour, not native
compiler authority or an emitted-WASM result.

[validation.json](validation.json) records exact build/test argv, environment,
tool launch identities, timestamps and the selected executable. The build takes
73.37 seconds; the five-control group takes 5.05 seconds and the root-entry
control 2.97 seconds. Each test runs once with `--test-threads=1`. Raw outputs are
[test-0.stdout](test-0.stdout), [test-0.stderr](test-0.stderr),
[test-1.stdout](test-1.stdout), [test-1.stderr](test-1.stderr),
[build.stdout.jsonl](build.stdout.jsonl) and [build.stderr](build.stderr).

All 15,175 captured source/data/manifests/locks and selected libtommath C/header
inputs remain identical in [inputs-before.json](inputs-before.json) and
[inputs-after.json](inputs-after.json). The fourteen local dependency packages
are in [packages.json](packages.json); this scope does not claim every retained
fixture compiled. [artifacts.json](artifacts.json) preserves Cargo's actual
artifact reports; [artifact-identities.json](artifact-identities.json) records
existing output hashes. [actual-tool-identities.json](actual-tool-identities.json)
adds the actual compiler/Cargo identities behind the recorded Rustup launchers.
The selected Runtime test executable is unchanged before/after execution.

[run.py](run.py), [metadata.json](metadata.json) and
[orchestration-identities.json](orchestration-identities.json) retain the exact
orchestration. It ran from the recorded `/tmp` script and metadata paths with
an exclusive target directory and frozen producer sources. Reproduction uses a
fresh output directory; preserve these observations rather than overwriting them.

This capture executes Runtime software controls. The separate original C/Jim
programs own oracle expectations; compiler admission, current ABI descriptor
checks, actual WASM table invocation and F5 conformance remain independent.
