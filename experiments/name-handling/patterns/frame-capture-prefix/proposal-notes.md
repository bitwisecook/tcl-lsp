# Does namespace capture change a forwarded caller-relative link?

Status: **proposed, unexecuted, unregistered**. No Rust/Cargo/native interpreter,
network, appliance load or message was used. Existing questions and inputs are
unchanged. [Proposal receipts](proposal.json) retain two authored inputs and
three existing source contexts without new source or licence copies.

The small question is whether adding namespace capture around the same aliased
command prefix changes which cell `upvar 1` reaches after `uplevel 1` forwarding.
Command selection, list-word preservation and caller-relative cell selection
must remain separate. A namespace receipt does not grant a procedure-local
closure or caller-frame receipt.

The reviewed ingredients are pinned [ruff `parse_preformatted_state`](https://github.com/georgtree/ruff/blob/b63da69eed625bc544d1f4da76371e8bd6b02d29/src/ruff.tcl#L1137)
(`corpus-state-upvar`, source manifest index 3), pinned
[pave `window`](https://github.com/aplsimple/pave/blob/875de1f1539d62e46e48e206f34f82848bca1736/apavebase.tcl#L3673)
(`corpus-frame-forward`, index 5), and the existing
[Wiki namespace-code snapshot](https://wiki.tcl-lang.org/page/namespace+code)
(index 2, retrieved 2026-10-12). The original full-source/range hashes match
local corpus bytes. Existing licence coordinates/hashes are retained without
copying their text; the Wiki receipt has no inspected licence or verified
revision. None of these three sources contains the exact proposed composition.
Ruff's array state and pave's OO receiver/expanded arguments remain unmeasured.

[Plain prefix](plain-prefix.tcl) sends a list-built, explicitly named same-
interpreter alias through a caller-frame forwarding procedure.
[Captured prefix](namespace-captured-prefix.tcl) adds only `namespace code`
around that list. Both final source lines call the same caller procedure and
request two values: the original spaced caller-local variable and existence of
that name in `::writer` namespace storage. Neither input catches guest errors.
A future driver must keep setup failure separate from the unreached final
question and retain full catch options and exact original bytes.

The positive hypothesis is that the plain prefix reaches the caller local.
The adversarial hypothesis is that namespace capture introduces a different
variable-frame relationship and may reach namespace storage instead. These
are hypotheses, not expected oracle values: same-interpreter alias invocation
and `namespace inscope` frame behaviour must both be measured. An alias/API
setup rejection cannot answer the later caller-relative question.

Catalogue comparison found `frame-wrapper` and its `eval` near miss cover
forwarding without namespace capture or aliases; `wiki-upvar-3..4` cover link
wrappers without this namespace/alias composition; `wiki-code-1..2` cover
namespace command/local capture without a forwarded relative link; and
`wiki-alias-1..2` cover binding/interpreter selection without variable frames.
This proposed pair therefore needs a new question identity rather than an
additional answer to those existing inputs. It establishes no new source-site
count, prevalence, static recognition result or F5 runtime result.
