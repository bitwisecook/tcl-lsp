# Executed Rust validation receipts

[ledger.json](ledger.json) records executed Rust tests separately from the
per-question native observations. Every entry retains the exact original
receipt and log, their complete digests, the selected tests and their limits.
The receipt's source inventory identifies the frozen source that executed;
later working source requires separate validation. Native provider answers and
linked test assertions keep their independent scope.

| Receipt | Executed assertions | Scope |
| --- | --- | --- |
| [Runtime child-command lifetime 074](runtime-childlifetime074/receipt.json) · [log](runtime-childlifetime074/tests.log) | 3 passed, 0 failed | One source/control consumer test covers 45 retained C comparisons; two siblings check Rust allocation and retirement. Other tests were filtered out. |

Each result applies only to its recorded execution. A compile-blocked command
executes no assertions. Exact gzip receipts decompress to their independently
verified original bytes and retain the complete source inventory. These
results supply no native rerun, unexecuted branch result or claim about changed
working source.

## Frozen source 086 commands

| Receipt and log | Actual result | Bound question |
| --- | --- | --- |
| [body-mapping](frozen086/body-mapping/receipt.json.gz) · [log](frozen086/body-mapping/tests.log) | 1 passed, 0 failed | `naming.source.original-native-script-body-value` |
| [root-frame](frozen086/root-frame/receipt.json.gz) · [log](frozen086/root-frame/tests.log) | 1 passed, 0 failed | `naming.variable.original-root-entry-frame` |
| [body-image](frozen086/body-image/receipt.json.gz) · [log](frozen086/body-image/tests.log) | 1 passed, 0 failed | `naming.variable.scalar-formal-body-alpha-equivalence` |
| [diagrams](frozen086/diagrams/receipt.json.gz) · [log](frozen086/diagrams/tests.log) | Compilation blocked; no assertions executed | `naming.consumer.original-structural-diagrams` |
| [info-functions](frozen086/info-functions/receipt.json.gz) · [log](frozen086/info-functions/tests.log) | 0 passed, 1 failed | `naming.info.functions-native-script` |
| [info-loaded](frozen086/info-loaded/receipt.json.gz) · [log](frozen086/info-loaded/tests.log) | 0 passed, 1 failed | `naming.info.loaded-original-interpreter-path` |
| [document-links](frozen086/document-links/receipt.json.gz) · [log](frozen086/document-links/tests.log) | Compilation blocked; no assertions executed | `naming.core.original-document-link-selection` |
| [context-completion](frozen086/context-completion/receipt.json.gz) · [log](frozen086/context-completion/tests.log) | Compilation blocked; no assertions executed | `naming.core.original-registry-context-completion` |
| [document-grammar](frozen086/document-grammar/receipt.json.gz) · [log](frozen086/document-grammar/tests.log) | Compilation blocked; no assertions executed | `naming.core.original-document-grammar-completion` |

## Runtime 080 and source 091 commands

The exact argument vectors and source inventories remain in the compressed receipts. Individual assertion results, including mixed passing and failing commands, remain in the ledger. Zero-matched filters provide no assertion coverage.

| Receipt and log | Actual result |
| --- | --- |
| [runtime-check080](runtime080/runtime-check080/receipt.json.gz) · [log](runtime080/runtime-check080/tests.log) | Compilation passed; no assertions executed |
| [runtime-info-functions080](runtime080/runtime-info-functions080/receipt.json.gz) · [log](runtime080/runtime-info-functions080/tests.log) | 1 passed, 0 failed |
| [runtime-methods080](runtime080/runtime-methods080/receipt.json.gz) · [log](runtime080/runtime-methods080/tests.log) | 1 passed, 0 failed |
| [runtime-oo-explicit080](runtime080/runtime-oo-explicit080/receipt.json.gz) · [log](runtime080/runtime-oo-explicit080/tests.log) | 0 passed, 1 failed |
| [runtime-oo-info080](runtime080/runtime-oo-info080/receipt.json.gz) · [log](runtime080/runtime-oo-info080/tests.log) | 1 passed, 0 failed |
| [runtime-trace-original080](runtime080/runtime-trace-original080/receipt.json.gz) · [log](runtime080/runtime-trace-original080/tests.log) | 5 passed, 0 failed |
| [check091](frozen091/check091/receipt.json.gz) · [log](frozen091/check091/tests.log) | Compilation blocked; no assertions executed |
| [cli-graphs091](frozen091/cli-graphs091/receipt.json.gz) · [log](frozen091/cli-graphs091/tests.log) | Zero assertions matched |
| [cli-package091](frozen091/cli-package091/receipt.json.gz) · [log](frozen091/cli-package091/tests.log) | Zero assertions matched |
| [context-completion091](frozen091/context-completion091/receipt.json.gz) · [log](frozen091/context-completion091/tests.log) | 2 passed, 0 failed |
| [core-alpha091](frozen091/core-alpha091/receipt.json.gz) · [log](frozen091/core-alpha091/tests.log) | 3 passed, 0 failed |
| [core-command091](frozen091/core-command091/receipt.json.gz) · [log](frozen091/core-command091/tests.log) | 1 passed, 1 failed |
| [core-current091](frozen091/core-current091/receipt.json.gz) · [log](frozen091/core-current091/tests.log) | 1 passed, 0 failed |
| [core-indexed091](frozen091/core-indexed091/receipt.json.gz) · [log](frozen091/core-indexed091/tests.log) | 4 passed, 0 failed |
| [core-member091](frozen091/core-member091/receipt.json.gz) · [log](frozen091/core-member091/tests.log) | 1 passed, 1 failed |
| [core-primary091](frozen091/core-primary091/receipt.json.gz) · [log](frozen091/core-primary091/tests.log) | 1 passed, 1 failed |
| [core-sourcerewrite091](frozen091/core-sourcerewrite091/receipt.json.gz) · [log](frozen091/core-sourcerewrite091/tests.log) | 2 passed, 0 failed |
| [core-symbolsmethods091](frozen091/core-symbolsmethods091/receipt.json.gz) · [log](frozen091/core-symbolsmethods091/tests.log) | 1 passed, 0 failed |
| [core-symbolsquery091](frozen091/core-symbolsquery091/receipt.json.gz) · [log](frozen091/core-symbolsquery091/tests.log) | 1 passed, 0 failed |
| [core-vendor091](frozen091/core-vendor091/receipt.json.gz) · [log](frozen091/core-vendor091/tests.log) | 9 passed, 1 failed |
| [core-wscompletion091](frozen091/core-wscompletion091/receipt.json.gz) · [log](frozen091/core-wscompletion091/tests.log) | 5 passed, 0 failed |
| [db091](frozen091/db091/receipt.json.gz) · [log](frozen091/db091/tests.log) | 5 passed, 1 failed |
| [diagrams091](frozen091/diagrams091/receipt.json.gz) · [log](frozen091/diagrams091/tests.log) | 4 passed, 1 failed |
| [document-grammar091](frozen091/document-grammar091/receipt.json.gz) · [log](frozen091/document-grammar091/tests.log) | 0 passed, 1 failed |
| [document-links091](frozen091/document-links091/receipt.json.gz) · [log](frozen091/document-links091/tests.log) | 0 passed, 2 failed |
| [explorer-event091](frozen091/explorer-event091/receipt.json.gz) · [log](frozen091/explorer-event091/tests.log) | 1 passed, 0 failed |
| [expressions091](frozen091/expressions091/receipt.json.gz) · [log](frozen091/expressions091/tests.log) | 4 passed, 0 failed |
| [formatting091](frozen091/formatting091/receipt.json.gz) · [log](frozen091/formatting091/tests.log) | 2 passed, 0 failed |
| [highlights091](frozen091/highlights091/receipt.json.gz) · [log](frozen091/highlights091/tests.log) | 1 passed, 0 failed |
| [inlinevar091](frozen091/inlinevar091/receipt.json.gz) · [log](frozen091/inlinevar091/tests.log) | 12 passed, 1 failed |
| [mcp-authority091](frozen091/mcp-authority091/receipt.json.gz) · [log](frozen091/mcp-authority091/tests.log) | 5 passed, 1 failed |
| [mcp-tooldocs091](frozen091/mcp-tooldocs091/receipt.json.gz) · [log](frozen091/mcp-tooldocs091/tests.log) | 5 passed, 1 failed |
| [method-refs091](frozen091/method-refs091/receipt.json.gz) · [log](frozen091/method-refs091/tests.log) | 1 passed, 0 failed |
| [registry-package091](frozen091/registry-package091/receipt.json.gz) · [log](frozen091/registry-package091/tests.log) | 1 passed, 0 failed |
| [scoped-completion091](frozen091/scoped-completion091/receipt.json.gz) · [log](frozen091/scoped-completion091/tests.log) | 0 passed, 1 failed |
| [server-command091](frozen091/server-command091/receipt.json.gz) · [log](frozen091/server-command091/tests.log) | 1 passed, 0 failed |
| [server-indexed091](frozen091/server-indexed091/receipt.json.gz) · [log](frozen091/server-indexed091/tests.log) | 0 passed, 1 failed |
| [server-math091](frozen091/server-math091/receipt.json.gz) · [log](frozen091/server-math091/tests.log) | 1 passed, 0 failed |
| [server-member091](frozen091/server-member091/receipt.json.gz) · [log](frozen091/server-member091/tests.log) | 0 passed, 1 failed |
| [server-minify091](frozen091/server-minify091/receipt.json.gz) · [log](frozen091/server-minify091/tests.log) | 2 passed, 1 failed |
| [signature091](frozen091/signature091/receipt.json.gz) · [log](frozen091/signature091/tests.log) | 31 passed, 0 failed |
| [source-performance091](frozen091/source-performance091/receipt.json.gz) · [log](frozen091/source-performance091/tests.log) | Process aborted; assertion incomplete |
| [specloader091](frozen091/specloader091/receipt.json.gz) · [log](frozen091/specloader091/tests.log) | 1 passed, 0 failed |
| [specnotice091](frozen091/specnotice091/receipt.json.gz) · [log](frozen091/specnotice091/tests.log) | 0 passed, 1 failed |

## Runtime 084 original-object commands

The four exact command receipts and logs retain the same frozen source inventory. The five selected assertions passed; native observations and their original-object/result-channel scopes remain separate. No VM or later-source result is inferred.

| Receipt and log | Executed assertions |
| --- | --- |
| [runtime-inventory084](runtime084/runtime-inventory084/receipt.json.gz) · [log](runtime084/runtime-inventory084/tests.log) | 1 passed, 0 failed |
| [runtime-oo-explicit084](runtime084/runtime-oo-explicit084/receipt.json.gz) · [log](runtime084/runtime-oo-explicit084/tests.log) | 1 passed, 0 failed |
| [runtime-lambda-diagnostics084](runtime084/runtime-lambda-diagnostics084/receipt.json.gz) · [log](runtime084/runtime-lambda-diagnostics084/tests.log) | 1 passed, 0 failed |
| [runtime-apply-original084](runtime084/runtime-apply-original084/receipt.json.gz) · [log](runtime084/runtime-apply-original084/tests.log) | 2 passed, 0 failed |

## Source 100 consumer commands

The exact argument vectors and source inventories remain in the compressed receipts. Individual assertion results, including mixed passing and failing commands, remain in the ledger. Zero-matched filters provide no assertion coverage.

| Receipt and log | Actual result |
| --- | --- |
| [specnotice100](frozen100/specnotice100/receipt.json.gz) · [log](frozen100/specnotice100/tests.log) | 1 passed, 0 failed |
| [diagrams100](frozen100/diagrams100/receipt.json.gz) · [log](frozen100/diagrams100/tests.log) | 5 passed, 3 failed |
| [source-diagnostic100](frozen100/source-diagnostic100/receipt.json.gz) · [log](frozen100/source-diagnostic100/tests.log) | 0 passed, 1 failed |
| [snippet-context100](frozen100/snippet-context100/receipt.json.gz) · [log](frozen100/snippet-context100/tests.log) | 2 passed, 0 failed |
| [snippets100](frozen100/snippets100/receipt.json.gz) · [log](frozen100/snippets100/tests.log) | 9 passed, 0 failed |
| [mcp-tooldocs100](frozen100/mcp-tooldocs100/receipt.json.gz) · [log](frozen100/mcp-tooldocs100/tests.log) | 6 passed, 2 failed |

## Runtime 087 coroutine commands

The exact argument vectors and complete source inventories remain in the compressed receipts. Five actual assertions passed across the original-object, publication and injection selectors. These results describe this immutable Runtime source only; no VM, complete-suite, later-source or additional native provider result is inferred.

| Receipt and log | Actual result |
| --- | --- |
| [runtime-coroutine-objects087](frozen-runtime087/runtime-coroutine-objects087/receipt.json.gz) · [log](frozen-runtime087/runtime-coroutine-objects087/tests.log) | 2 passed, 0 failed |
| [runtime-coroutine-publication087](frozen-runtime087/runtime-coroutine-publication087/receipt.json.gz) · [log](frozen-runtime087/runtime-coroutine-publication087/tests.log) | 1 passed, 0 failed |
| [runtime-coroutine-injection087](frozen-runtime087/runtime-coroutine-injection087/receipt.json.gz) · [log](frozen-runtime087/runtime-coroutine-injection087/tests.log) | 2 passed, 0 failed |

## Source 101 consumer commands

| Receipt and log | Actual result |
| --- | --- |
| [specnotice101](frozen101/specnotice101/receipt.json.gz) · [log](frozen101/specnotice101/tests.log) | 1 passed, 0 failed |
| [diagrams101](frozen101/diagrams101/receipt.json.gz) · [log](frozen101/diagrams101/tests.log) | 7 passed, 1 failed |
| [source-diagnostic101](frozen101/source-diagnostic101/receipt.json.gz) · [log](frozen101/source-diagnostic101/tests.log) | 0 passed, 1 failed |
| [mcp-tooldocs101](frozen101/mcp-tooldocs101/receipt.json.gz) · [log](frozen101/mcp-tooldocs101/tests.log) | 7 passed, 1 failed |
| [ilx101](frozen101/ilx101/receipt.json.gz) · [log](frozen101/ilx101/tests.log) | 17 passed, 5 failed |
| [ilx-navigation101](frozen101/ilx-navigation101/receipt.json.gz) · [log](frozen101/ilx-navigation101/tests.log) | 2 passed, 12 failed |

## Source 103 consumer commands

| Receipt and log | Actual result |
| --- | --- |
| [specnotice103](frozen103/specnotice103/receipt.json.gz) · [log](frozen103/specnotice103/tests.log) | 1 passed, 0 failed |
| [diagrams103](frozen103/diagrams103/receipt.json.gz) · [log](frozen103/diagrams103/tests.log) | 7 passed, 1 failed |
| [source-diagnostic103](frozen103/source-diagnostic103/receipt.json.gz) · [log](frozen103/source-diagnostic103/tests.log) | 1 passed, 0 failed |
| [mcp-tooldocs103](frozen103/mcp-tooldocs103/receipt.json.gz) · [log](frozen103/mcp-tooldocs103/tests.log) | 8 passed, 0 failed |
| [ilx103](frozen103/ilx103/receipt.json.gz) · [log](frozen103/ilx103/tests.log) | 19 passed, 3 failed |
| [ilx-navigation103](frozen103/ilx-navigation103/receipt.json.gz) · [log](frozen103/ilx-navigation103/tests.log) | 15 passed, 0 failed |

## Source 103 VM commands

| Receipt and log | Actual result |
| --- | --- |
| [vm-apply-original103](frozen103/vm-apply-original103/receipt.json.gz) · [log](frozen103/vm-apply-original103/tests.log) | 2 passed, 1 failed |
| [vm-coroutine-injection103](frozen103/vm-coroutine-injection103/receipt.json.gz) · [log](frozen103/vm-coroutine-injection103/tests.log) | 2 passed, 0 failed |
| [vm-coroutine-objects103](frozen103/vm-coroutine-objects103/receipt.json.gz) · [log](frozen103/vm-coroutine-objects103/tests.log) | 2 passed, 0 failed |
| [vm-coroutine-publication103](frozen103/vm-coroutine-publication103/receipt.json.gz) · [log](frozen103/vm-coroutine-publication103/tests.log) | 0 passed, 1 failed |
| [vm-functions103](frozen103/vm-functions103/receipt.json.gz) · [log](frozen103/vm-functions103/tests.log) | 0 passed, 1 failed |
| [vm-lambda-diagnostics103](frozen103/vm-lambda-diagnostics103/receipt.json.gz) · [log](frozen103/vm-lambda-diagnostics103/tests.log) | 1 passed, 0 failed |
| [vm-loaded103](frozen103/vm-loaded103/receipt.json.gz) · [log](frozen103/vm-loaded103/tests.log) | 1 passed, 0 failed |
| [vm-oo-explicit103](frozen103/vm-oo-explicit103/receipt.json.gz) · [log](frozen103/vm-oo-explicit103/tests.log) | 0 passed, 1 failed |
| [vm-oo-info103](frozen103/vm-oo-info103/receipt.json.gz) · [log](frozen103/vm-oo-info103/tests.log) | 1 passed, 0 failed |
| [vm-oo-methods103](frozen103/vm-oo-methods103/receipt.json.gz) · [log](frozen103/vm-oo-methods103/tests.log) | 1 passed, 0 failed |
| [vm-stock-destroy103](frozen103/vm-stock-destroy103/receipt.json.gz) · [log](frozen103/vm-stock-destroy103/tests.log) | 1 passed, 0 failed |
| [vm-trace103](frozen103/vm-trace103/receipt.json.gz) · [log](frozen103/vm-trace103/tests.log) | 7 passed, 0 failed |

## Bounded source depth validation

The exact 80-depth source assertion passed in 32.78 seconds on its pinned Core binary with the default test-thread stack and a 60-second process bound. This is one frozen-source Rust result; native observations and broader performance remain separate.

[Command receipt](frozen105/source-performance105/receipt.json.gz), [complete log](frozen105/source-performance105/tests.log), and [pinned binary/source association](frozen105/source-performance105/pinned-artifact.json.gz) retain the exact inputs and hashes.

## Runtime 113 namespace assertions

The exact original_namespace_ filter executed eleven passing assertions under the retained uniform Runtime source inventory. Tail, Qualifiers and namespace producer/getter comparisons retain their independent finite native fixtures. The complete source inventory and exact argument vector remain in the receipt; no VM, complete-suite, later-source or new native execution result is inferred.

| Receipt and log | Actual result |
| --- | --- |
| [runtime-namespace113](frozen-runtime113/runtime-namespace113/receipt.json.gz) · [log](frozen-runtime113/runtime-namespace113/tests.log) | 11 passed, 0 failed |

## Source 115 and 117 purpose-specific commands

| Receipt and log | Actual result |
| --- | --- |
| [vm-append30115](frozen115/vm-append30115/receipt.json.gz) · [log](frozen115/vm-append30115/tests.log) | 1 passed, 0 failed |
| [vm-event-script115](frozen115/vm-event-script115/receipt.json.gz) · [log](frozen115/vm-event-script115/tests.log) | 1 passed, 0 failed |
| [vm-event-list115](frozen115/vm-event-list115/receipt.json.gz) · [log](frozen115/vm-event-list115/tests.log) | 1 passed, 0 failed |
| [runtime-event-script115](frozen115/runtime-event-script115/receipt.json.gz) · [log](frozen115/runtime-event-script115/tests.log) | Compilation blocked; no assertions executed |
| [advice-domain117](frozen117/advice-domain117/receipt.json.gz) · [log](frozen117/advice-domain117/tests.log) | 3 passed, 0 failed |
| [body-inventory117](frozen117/body-inventory117/receipt.json.gz) · [log](frozen117/body-inventory117/tests.log) | 1 passed, 1 failed |
| [formal-entry117](frozen117/formal-entry117/receipt.json.gz) · [log](frozen117/formal-entry117/tests.log) | 1 passed, 0 failed |
| [hosted-image117](frozen117/hosted-image117/receipt.json.gz) · [log](frozen117/hosted-image117/tests.log) | 1 passed, 0 failed |
| [hosted-source-taint117](frozen117/hosted-source-taint117/receipt.json.gz) · [log](frozen117/hosted-source-taint117/tests.log) | 1 passed, 0 failed |
| [source-transitions117](frozen117/source-transitions117/receipt.json.gz) · [log](frozen117/source-transitions117/tests.log) | 0 passed, 3 failed |
| [vwait-grammar117](frozen117/vwait-grammar117/receipt.json.gz) · [log](frozen117/vwait-grammar117/tests.log) | 3 passed, 0 failed |
| [fresh-namespace-delete117](frozen117/fresh-namespace-delete117/receipt.json.gz) · [log](frozen117/fresh-namespace-delete117/tests.log) | 0 passed, 1 failed |
| [namespace-delete117](frozen117/namespace-delete117/receipt.json.gz) · [log](frozen117/namespace-delete117/tests.log) | 1 passed, 1 failed |
| [namespace-publications117](frozen117/namespace-publications117/receipt.json.gz) · [log](frozen117/namespace-publications117/tests.log) | 0 passed, 1 failed |

## Source 122 purpose-specific commands

| Receipt and log | Actual result |
| --- | --- |
| [file-input122](frozen122/file-input122/receipt.json.gz) · [log](frozen122/file-input122/tests.log) | 1 passed, 0 failed |
| [future-source-schema122](frozen122/future-source-schema122/receipt.json.gz) · [log](frozen122/future-source-schema122/tests.log) | 2 passed, 0 failed |
| [deferred-family122](frozen122/deferred-family122/receipt.json.gz) · [log](frozen122/deferred-family122/tests.log) | 0 passed, 1 failed |
| [retained-source-context122](frozen122/retained-source-context122/receipt.json.gz) · [log](frozen122/retained-source-context122/tests.log) | 1 passed, 0 failed |
| [body-inventory122](frozen122/body-inventory122/receipt.json.gz) · [log](frozen122/body-inventory122/tests.log) | 1 passed, 1 failed |
| [core-pinned-ilx-navigation122](frozen122/core-pinned-ilx-navigation122/receipt.json.gz) · [log](frozen122/core-pinned-ilx-navigation122/tests.log) | 16 passed, 0 failed |
| [core-pinned-vendor122](frozen122/core-pinned-vendor122/receipt.json.gz) · [log](frozen122/core-pinned-vendor122/tests.log) | 9 passed, 1 failed |
| [core-pinned-hover122](frozen122/core-pinned-hover122/receipt.json.gz) · [log](frozen122/core-pinned-hover122/tests.log) | 5 passed, 0 failed |
| [core-pinned-minifier-all122](frozen122/core-pinned-minifier-all122/receipt.json.gz) · [log](frozen122/core-pinned-minifier-all122/tests.log) | 23 passed, 0 failed |
| [advice-domain122](frozen122/advice-domain122/receipt.json.gz) · [log](frozen122/advice-domain122/tests.log) | 3 passed, 0 failed |
| [hosted-source-taint122](frozen122/hosted-source-taint122/receipt.json.gz) · [log](frozen122/hosted-source-taint122/tests.log) | 1 passed, 0 failed |
| [hosted-image122](frozen122/hosted-image122/receipt.json.gz) · [log](frozen122/hosted-image122/tests.log) | 1 passed, 0 failed |

## Retained source 126 and 127 commands

| Receipt and log | Actual result |
| --- | --- |
| [core-pinned-format-cases126](frozen126/core-pinned-format-cases126/receipt.json.gz) · [log](frozen126/core-pinned-format-cases126/tests.log) | 11 passed, 2 failed |
| [core-pinned-formatter-all126](frozen126/core-pinned-formatter-all126/receipt.json.gz) · [log](frozen126/core-pinned-formatter-all126/tests.log) | 78 passed, 5 failed |
| [core-pinned-hover126](frozen126/core-pinned-hover126/receipt.json.gz) · [log](frozen126/core-pinned-hover126/tests.log) | 5 passed, 0 failed |
| [core-pinned-ilx-navigation126](frozen126/core-pinned-ilx-navigation126/receipt.json.gz) · [log](frozen126/core-pinned-ilx-navigation126/tests.log) | 16 passed, 0 failed |
| [core-pinned-minifier-all126](frozen126/core-pinned-minifier-all126/receipt.json.gz) · [log](frozen126/core-pinned-minifier-all126/tests.log) | 26 passed, 2 failed |
| [core-pinned-namespace-rename126](frozen126/core-pinned-namespace-rename126/receipt.json.gz) · [log](frozen126/core-pinned-namespace-rename126/tests.log) | 7 passed, 0 failed |
| [core-pinned-package126](frozen126/core-pinned-package126/receipt.json.gz) · [log](frozen126/core-pinned-package126/tests.log) | 6 passed, 0 failed |
| [core-pinned-vendor126](frozen126/core-pinned-vendor126/receipt.json.gz) · [log](frozen126/core-pinned-vendor126/tests.log) | 10 passed, 0 failed |
| [hosted-body-input127](frozen127/hosted-body-input127/receipt.json.gz) · [log](frozen127/hosted-body-input127/tests.log) | 1 passed, 1 failed |
| [namespace-ensure-trace127](frozen127/namespace-ensure-trace127/receipt.json.gz) · [log](frozen127/namespace-ensure-trace127/tests.log) | 0 passed, 1 failed |

## Retained Runtime source 133 command

| Receipt and log | Actual result |
| --- | --- |
| [runtime-native-ports133](frozen133/runtime-native-ports133/receipt.json.gz) · [log](frozen133/runtime-native-ports133/tests.log) | 148 passed, 20 failed |

## Retained Core source 135 commands

The exact argument vectors and source inventories remain in the compressed receipts. The separately retained [executable pin](frozen135/pinned-core135.json) identifies the exact Core executable and its build receipt. Every command source inventory equals that build inventory. Individual assertion results, including mixed passing and failing commands, remain in the ledger. Zero-matched filters provide no assertion coverage.

| Receipt and log | Actual result |
| --- | --- |
| [core-formatting135](frozen135/core-formatting135/receipt.json.gz) · [log](frozen135/core-formatting135/tests.log) | 78 passed, 5 failed |
| [core-minify135](frozen135/core-minify135/receipt.json.gz) · [log](frozen135/core-minify135/tests.log) | 26 passed, 2 failed |
| [core-pinned-caller-frame135](frozen135/core-pinned-caller-frame135/receipt.json.gz) · [log](frozen135/core-pinned-caller-frame135/tests.log) | 32 passed, 2 failed |
| [core-pinned-case-list135](frozen135/core-pinned-case-list135/receipt.json.gz) · [log](frozen135/core-pinned-case-list135/tests.log) | Zero assertions matched |
| [core-pinned-completion135](frozen135/core-pinned-completion135/receipt.json.gz) · [log](frozen135/core-pinned-completion135/tests.log) | 2 passed, 0 failed |
| [core-pinned-formatter-engine135](frozen135/core-pinned-formatter-engine135/receipt.json.gz) · [log](frozen135/core-pinned-formatter-engine135/tests.log) | Zero assertions matched |
| [core-pinned-hover-pattern135](frozen135/core-pinned-hover-pattern135/receipt.json.gz) · [log](frozen135/core-pinned-hover-pattern135/tests.log) | 3 passed, 0 failed |
| [core-pinned-inlay-context135](frozen135/core-pinned-inlay-context135/receipt.json.gz) · [log](frozen135/core-pinned-inlay-context135/tests.log) | 5 passed, 0 failed |
| [core-pinned-inlay-original135](frozen135/core-pinned-inlay-original135/receipt.json.gz) · [log](frozen135/core-pinned-inlay-original135/tests.log) | 5 passed, 0 failed |
| [core-pinned-math-transport135](frozen135/core-pinned-math-transport135/receipt.json.gz) · [log](frozen135/core-pinned-math-transport135/tests.log) | 0 passed, 1 failed |
| [core-pinned-minifier-original135](frozen135/core-pinned-minifier-original135/receipt.json.gz) · [log](frozen135/core-pinned-minifier-original135/tests.log) | Zero assertions matched |
| [core-pinned-package-files135](frozen135/core-pinned-package-files135/receipt.json.gz) · [log](frozen135/core-pinned-package-files135/tests.log) | 4 passed, 0 failed |
| [core-pinned-semantic-original135](frozen135/core-pinned-semantic-original135/receipt.json.gz) · [log](frozen135/core-pinned-semantic-original135/tests.log) | 7 passed, 0 failed |
| [core-pinned-signature135](frozen135/core-pinned-signature135/receipt.json.gz) · [log](frozen135/core-pinned-signature135/tests.log) | 4 passed, 0 failed |

## Retained source 136 builds and Runtime commands

The exact argument vectors and source inventories remain in the compressed receipts. The [Runtime executable pin](frozen136/pinned-runtime136.json) identifies the actual test binary and successful build. All focused command inventories equal that build inventory. The Compiler test build has ten exact error diagnostics and executes no assertions; the Runtime build executes no assertions. Individual passing and failing assertion results remain in the ledger. Zero-matched filters provide no assertion coverage.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build136](frozen136/compiler-build136/receipt.json.gz) · [log](frozen136/compiler-build136/tests.log) | Compilation blocked; no assertions executed |
| [runtime-build136](frozen136/runtime-build136/receipt.json.gz) · [log](frozen136/runtime-build136/tests.log) | Compilation passed; no assertions executed |
| [runtime-focused136-0](frozen136/runtime-focused136-0/receipt.json.gz) · [log](frozen136/runtime-focused136-0/tests.log) | 0 passed, 1 failed |
| [runtime-focused136-1](frozen136/runtime-focused136-1/receipt.json.gz) · [log](frozen136/runtime-focused136-1/tests.log) | 2 passed, 0 failed |
| [runtime-focused136-2](frozen136/runtime-focused136-2/receipt.json.gz) · [log](frozen136/runtime-focused136-2/tests.log) | 1 passed, 0 failed |
| [runtime-focused136-3](frozen136/runtime-focused136-3/receipt.json.gz) · [log](frozen136/runtime-focused136-3/tests.log) | 0 passed, 1 failed |
| [runtime-focused136-4](frozen136/runtime-focused136-4/receipt.json.gz) · [log](frozen136/runtime-focused136-4/tests.log) | 1 passed, 0 failed |
| [runtime-focused136-5](frozen136/runtime-focused136-5/receipt.json.gz) · [log](frozen136/runtime-focused136-5/tests.log) | 0 passed, 1 failed |
| [runtime-focused136-6](frozen136/runtime-focused136-6/receipt.json.gz) · [log](frozen136/runtime-focused136-6/tests.log) | 1 passed, 0 failed |
| [runtime-focused136-7](frozen136/runtime-focused136-7/receipt.json.gz) · [log](frozen136/runtime-focused136-7/tests.log) | 1 passed, 0 failed |
| [runtime-focused136-8](frozen136/runtime-focused136-8/receipt.json.gz) · [log](frozen136/runtime-focused136-8/tests.log) | 1 passed, 0 failed |
| [runtime-focused136-9](frozen136/runtime-focused136-9/receipt.json.gz) · [log](frozen136/runtime-focused136-9/tests.log) | 0 passed, 1 failed |
| [runtime-focused136-10](frozen136/runtime-focused136-10/receipt.json.gz) · [log](frozen136/runtime-focused136-10/tests.log) | 1 passed, 0 failed |
| [runtime-focused136-11](frozen136/runtime-focused136-11/receipt.json.gz) · [log](frozen136/runtime-focused136-11/tests.log) | 1 passed, 0 failed |
| [runtime-focused136-12](frozen136/runtime-focused136-12/receipt.json.gz) · [log](frozen136/runtime-focused136-12/tests.log) | 0 passed, 1 failed |
| [runtime-focused136-13](frozen136/runtime-focused136-13/receipt.json.gz) · [log](frozen136/runtime-focused136-13/tests.log) | 0 passed, 1 failed |
| [runtime-focused136-14](frozen136/runtime-focused136-14/receipt.json.gz) · [log](frozen136/runtime-focused136-14/tests.log) | 0 passed, 1 failed |
| [runtime-focused136-15](frozen136/runtime-focused136-15/receipt.json.gz) · [log](frozen136/runtime-focused136-15/tests.log) | 0 passed, 1 failed |
| [runtime-focused136-16](frozen136/runtime-focused136-16/receipt.json.gz) · [log](frozen136/runtime-focused136-16/tests.log) | 0 passed, 1 failed |
| [runtime-focused136-17](frozen136/runtime-focused136-17/receipt.json.gz) · [log](frozen136/runtime-focused136-17/tests.log) | 1 passed, 0 failed |
| [runtime-focused136-18](frozen136/runtime-focused136-18/receipt.json.gz) · [log](frozen136/runtime-focused136-18/tests.log) | 1 passed, 0 failed |
| [runtime-focused136-19](frozen136/runtime-focused136-19/receipt.json.gz) · [log](frozen136/runtime-focused136-19/tests.log) | 1 passed, 0 failed |
| [runtime-focused136-20](frozen136/runtime-focused136-20/receipt.json.gz) · [log](frozen136/runtime-focused136-20/tests.log) | 0 passed, 1 failed |
| [runtime-focused136-21](frozen136/runtime-focused136-21/receipt.json.gz) · [log](frozen136/runtime-focused136-21/tests.log) | 1 passed, 0 failed |

## Retained Runtime source 137 commands

The exact argument vectors and source inventories remain in the compressed receipts. The [Runtime executable pin](frozen137/pinned-runtime137.json) identifies the actual test binary and successful build. All focused command inventories equal that build inventory. The Runtime build executes no assertions. Both selected comparisons fail; completed earlier controls within a failed assertion are not counted as independently passing tests. Individual passing and failing assertion results remain in the ledger. Zero-matched filters provide no assertion coverage.

| Receipt and log | Actual result |
| --- | --- |
| [runtime-build137](frozen137/runtime-build137/receipt.json.gz) · [log](frozen137/runtime-build137/tests.log) | Compilation passed; no assertions executed |
| [runtime-unset137](frozen137/runtime-unset137/receipt.json.gz) · [log](frozen137/runtime-unset137/tests.log) | 0 passed, 1 failed |
| [runtime-command-surface137](frozen137/runtime-command-surface137/receipt.json.gz) · [log](frozen137/runtime-command-surface137/tests.log) | 0 passed, 1 failed |

## Retained source 140 builds and selected commands

The three [Compiler](frozen140/pinned-compiler140.json), [Runtime](frozen140/pinned-runtime140.json) and [Core](frozen140/pinned-core140.json) executable pins identify the actual successful builds. Every selected command retains that same complete source inventory. Registry compilation is blocked and executes no assertions. Passing and failing assertions remain separate; these Rust results change no native provider answer or current-source proof.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build140](frozen140/compiler-build140/receipt.json.gz) · [log](frozen140/compiler-build140/tests.log) | Compilation passed; no assertions executed |
| [runtime-build140](frozen140/runtime-build140/receipt.json.gz) · [log](frozen140/runtime-build140/tests.log) | Compilation passed; no assertions executed |
| [core-build140](frozen140/core-build140/receipt.json.gz) · [log](frozen140/core-build140/tests.log) | Compilation passed; no assertions executed |
| [registry-build140](frozen140/registry-build140/receipt.json.gz) · [log](frozen140/registry-build140/tests.log) | Compilation blocked; no assertions executed |
| [compiler-focused140-0](frozen140/compiler-focused140-0/receipt.json.gz) · [log](frozen140/compiler-focused140-0/tests.log) | 0 passed, 1 failed |
| [compiler-focused140-1](frozen140/compiler-focused140-1/receipt.json.gz) · [log](frozen140/compiler-focused140-1/tests.log) | 1 passed, 3 failed |
| [compiler-focused140-2](frozen140/compiler-focused140-2/receipt.json.gz) · [log](frozen140/compiler-focused140-2/tests.log) | 2 passed, 1 failed |
| [compiler-focused140-3](frozen140/compiler-focused140-3/receipt.json.gz) · [log](frozen140/compiler-focused140-3/tests.log) | 2 passed, 0 failed |
| [compiler-focused140-4](frozen140/compiler-focused140-4/receipt.json.gz) · [log](frozen140/compiler-focused140-4/tests.log) | 1 passed, 1 failed |
| [compiler-focused140-5](frozen140/compiler-focused140-5/receipt.json.gz) · [log](frozen140/compiler-focused140-5/tests.log) | 1 passed, 0 failed |
| [compiler-focused140-6](frozen140/compiler-focused140-6/receipt.json.gz) · [log](frozen140/compiler-focused140-6/tests.log) | 5 passed, 1 failed |
| [compiler-focused140-7](frozen140/compiler-focused140-7/receipt.json.gz) · [log](frozen140/compiler-focused140-7/tests.log) | 1 passed, 2 failed |
| [compiler-focused140-8](frozen140/compiler-focused140-8/receipt.json.gz) · [log](frozen140/compiler-focused140-8/tests.log) | 1 passed, 0 failed |
| [compiler-focused140-9](frozen140/compiler-focused140-9/receipt.json.gz) · [log](frozen140/compiler-focused140-9/tests.log) | 2 passed, 0 failed |
| [compiler-focused140-10](frozen140/compiler-focused140-10/receipt.json.gz) · [log](frozen140/compiler-focused140-10/tests.log) | 10 passed, 1 failed |
| [compiler-math-realm140](frozen140/compiler-math-realm140/receipt.json.gz) · [log](frozen140/compiler-math-realm140/tests.log) | 1 passed, 0 failed |
| [compiler-math-owner140](frozen140/compiler-math-owner140/receipt.json.gz) · [log](frozen140/compiler-math-owner140/tests.log) | 1 passed, 0 failed |
| [runtime-focused140-0](frozen140/runtime-focused140-0/receipt.json.gz) · [log](frozen140/runtime-focused140-0/tests.log) | 1 passed, 0 failed |
| [runtime-focused140-1](frozen140/runtime-focused140-1/receipt.json.gz) · [log](frozen140/runtime-focused140-1/tests.log) | 0 passed, 1 failed |
| [runtime-focused140-2](frozen140/runtime-focused140-2/receipt.json.gz) · [log](frozen140/runtime-focused140-2/tests.log) | 1 passed, 0 failed |
| [runtime-focused140-3](frozen140/runtime-focused140-3/receipt.json.gz) · [log](frozen140/runtime-focused140-3/tests.log) | 1 passed, 0 failed |
| [runtime-focused140-4](frozen140/runtime-focused140-4/receipt.json.gz) · [log](frozen140/runtime-focused140-4/tests.log) | 1 passed, 0 failed |
| [runtime-focused140-5](frozen140/runtime-focused140-5/receipt.json.gz) · [log](frozen140/runtime-focused140-5/tests.log) | 0 passed, 1 failed |
| [runtime-focused140-6](frozen140/runtime-focused140-6/receipt.json.gz) · [log](frozen140/runtime-focused140-6/tests.log) | 1 passed, 0 failed |
| [runtime-focused140-7](frozen140/runtime-focused140-7/receipt.json.gz) · [log](frozen140/runtime-focused140-7/tests.log) | 1 passed, 0 failed |
| [runtime-focused140-8](frozen140/runtime-focused140-8/receipt.json.gz) · [log](frozen140/runtime-focused140-8/tests.log) | 0 passed, 1 failed |
| [runtime-focused140-9](frozen140/runtime-focused140-9/receipt.json.gz) · [log](frozen140/runtime-focused140-9/tests.log) | 0 passed, 1 failed |
| [runtime-focused140-10](frozen140/runtime-focused140-10/receipt.json.gz) · [log](frozen140/runtime-focused140-10/tests.log) | 0 passed, 1 failed |
| [runtime-focused140-11](frozen140/runtime-focused140-11/receipt.json.gz) · [log](frozen140/runtime-focused140-11/tests.log) | 0 passed, 1 failed |
| [runtime-focused140-12](frozen140/runtime-focused140-12/receipt.json.gz) · [log](frozen140/runtime-focused140-12/tests.log) | 1 passed, 0 failed |
| [runtime-focused140-13](frozen140/runtime-focused140-13/receipt.json.gz) · [log](frozen140/runtime-focused140-13/tests.log) | 1 passed, 0 failed |
| [core-focused140-0](frozen140/core-focused140-0/receipt.json.gz) · [log](frozen140/core-focused140-0/tests.log) | 80 passed, 3 failed |
| [core-focused140-1](frozen140/core-focused140-1/receipt.json.gz) · [log](frozen140/core-focused140-1/tests.log) | 26 passed, 2 failed |
| [core-focused140-2](frozen140/core-focused140-2/receipt.json.gz) · [log](frozen140/core-focused140-2/tests.log) | 0 passed, 1 failed |
| [core-focused140-3](frozen140/core-focused140-3/receipt.json.gz) · [log](frozen140/core-focused140-3/tests.log) | 24 passed, 1 failed |

## Retained source 141 builds and Runtime commands

The exact argument vectors and source inventories remain in the compressed receipts. The [Runtime executable pin](frozen141/pinned-runtime141.json) identifies the actual test binary and successful build. All focused command inventories equal that build inventory. The Compiler and VM test builds are blocked and execute no assertions; the successful Runtime build executes no assertions. Individual passing and failing assertion results remain in the ledger. Zero-matched filters provide no assertion coverage.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build141](frozen141/compiler-build141/receipt.json.gz) · [log](frozen141/compiler-build141/tests.log) | Compilation blocked; no assertions executed |
| [runtime-build141](frozen141/runtime-build141/receipt.json.gz) · [log](frozen141/runtime-build141/tests.log) | Compilation passed; no assertions executed |
| [vm-build141](frozen141/vm-build141/receipt.json.gz) · [log](frozen141/vm-build141/tests.log) | Compilation blocked; no assertions executed |
| [runtime-focused141-0](frozen141/runtime-focused141-0/receipt.json.gz) · [log](frozen141/runtime-focused141-0/tests.log) | 1 passed, 0 failed |
| [runtime-focused141-1](frozen141/runtime-focused141-1/receipt.json.gz) · [log](frozen141/runtime-focused141-1/tests.log) | 1 passed, 0 failed |
| [runtime-focused141-2](frozen141/runtime-focused141-2/receipt.json.gz) · [log](frozen141/runtime-focused141-2/tests.log) | 0 passed, 1 failed |
| [runtime-focused141-3](frozen141/runtime-focused141-3/receipt.json.gz) · [log](frozen141/runtime-focused141-3/tests.log) | 1 passed, 0 failed |
| [runtime-focused141-4](frozen141/runtime-focused141-4/receipt.json.gz) · [log](frozen141/runtime-focused141-4/tests.log) | 1 passed, 0 failed |
| [runtime-focused141-5](frozen141/runtime-focused141-5/receipt.json.gz) · [log](frozen141/runtime-focused141-5/tests.log) | 1 passed, 0 failed |
| [runtime-focused141-6](frozen141/runtime-focused141-6/receipt.json.gz) · [log](frozen141/runtime-focused141-6/tests.log) | 0 passed, 1 failed |
| [runtime-focused141-7](frozen141/runtime-focused141-7/receipt.json.gz) · [log](frozen141/runtime-focused141-7/tests.log) | 1 passed, 0 failed |
| [runtime-focused141-8](frozen141/runtime-focused141-8/receipt.json.gz) · [log](frozen141/runtime-focused141-8/tests.log) | 1 passed, 0 failed |
| [runtime-focused141-9](frozen141/runtime-focused141-9/receipt.json.gz) · [log](frozen141/runtime-focused141-9/tests.log) | 1 passed, 0 failed |
| [runtime-focused141-10](frozen141/runtime-focused141-10/receipt.json.gz) · [log](frozen141/runtime-focused141-10/tests.log) | 1 passed, 0 failed |
| [runtime-focused141-11](frozen141/runtime-focused141-11/receipt.json.gz) · [log](frozen141/runtime-focused141-11/tests.log) | 0 passed, 1 failed |
| [runtime-focused141-12](frozen141/runtime-focused141-12/receipt.json.gz) · [log](frozen141/runtime-focused141-12/tests.log) | 1 passed, 0 failed |

## Retained source 142 commands

The [compiler executable pin](frozen142/pinned-compiler142.json), [core executable pin](frozen142/pinned-core142.json) identify the actual successful test builds and selected binaries. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. Blocked builds and failed Clippy commands execute no assertions; these commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build142](frozen142/compiler-build142/receipt.json.gz) · [log](frozen142/compiler-build142/tests.log) | Compilation passed; no assertions executed |
| [core-build142](frozen142/core-build142/receipt.json.gz) · [log](frozen142/core-build142/tests.log) | Compilation passed; no assertions executed |
| [compiler-focused142-0](frozen142/compiler-focused142-0/receipt.json.gz) · [log](frozen142/compiler-focused142-0/tests.log) | 0 passed, 1 failed |
| [compiler-focused142-1](frozen142/compiler-focused142-1/receipt.json.gz) · [log](frozen142/compiler-focused142-1/tests.log) | 1 passed, 1 failed |
| [compiler-focused142-2](frozen142/compiler-focused142-2/receipt.json.gz) · [log](frozen142/compiler-focused142-2/tests.log) | 1 passed, 2 failed |
| [compiler-focused142-3](frozen142/compiler-focused142-3/receipt.json.gz) · [log](frozen142/compiler-focused142-3/tests.log) | 6 passed, 1 failed |
| [compiler-focused142-4](frozen142/compiler-focused142-4/receipt.json.gz) · [log](frozen142/compiler-focused142-4/tests.log) | 2 passed, 0 failed |
| [compiler-focused142-5](frozen142/compiler-focused142-5/receipt.json.gz) · [log](frozen142/compiler-focused142-5/tests.log) | 3 passed, 1 failed |
| [compiler-focused142-6](frozen142/compiler-focused142-6/receipt.json.gz) · [log](frozen142/compiler-focused142-6/tests.log) | 1 passed, 0 failed |
| [compiler-focused142-7](frozen142/compiler-focused142-7/receipt.json.gz) · [log](frozen142/compiler-focused142-7/tests.log) | 2 passed, 0 failed |
| [compiler-focused142-8](frozen142/compiler-focused142-8/receipt.json.gz) · [log](frozen142/compiler-focused142-8/tests.log) | 3 passed, 0 failed |
| [compiler-focused142-9](frozen142/compiler-focused142-9/receipt.json.gz) · [log](frozen142/compiler-focused142-9/tests.log) | 11 passed, 0 failed |
| [core-focused142-0](frozen142/core-focused142-0/receipt.json.gz) · [log](frozen142/core-focused142-0/tests.log) | 0 passed, 1 failed |
| [core-focused142-1](frozen142/core-focused142-1/receipt.json.gz) · [log](frozen142/core-focused142-1/tests.log) | 1 passed, 0 failed |
| [core-focused142-2](frozen142/core-focused142-2/receipt.json.gz) · [log](frozen142/core-focused142-2/tests.log) | 1 passed, 0 failed |
| [core-focused142-3](frozen142/core-focused142-3/receipt.json.gz) · [log](frozen142/core-focused142-3/tests.log) | 1 passed, 0 failed |
| [core-focused142-4](frozen142/core-focused142-4/receipt.json.gz) · [log](frozen142/core-focused142-4/tests.log) | 1 passed, 0 failed |
| [core-focused142-5](frozen142/core-focused142-5/receipt.json.gz) · [log](frozen142/core-focused142-5/tests.log) | 1 passed, 0 failed |
| [core-focused142-6](frozen142/core-focused142-6/receipt.json.gz) · [log](frozen142/core-focused142-6/tests.log) | 0 passed, 1 failed |
| [core-focused142-7](frozen142/core-focused142-7/receipt.json.gz) · [log](frozen142/core-focused142-7/tests.log) | 0 passed, 1 failed |
| [clippy-workspace142](frozen142/clippy-workspace142/receipt.json.gz) · [log](frozen142/clippy-workspace142/tests.log) | Clippy failed; no assertions executed |


## Retained source 143 commands

The [compiler executable pin](frozen143/pinned-compiler143.json), [runtime executable pin](frozen143/pinned-runtime143.json) identify the actual successful test builds and selected binaries. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. Blocked builds and failed Clippy commands execute no assertions; these commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build143](frozen143/compiler-build143/receipt.json.gz) · [log](frozen143/compiler-build143/tests.log) | Compilation passed; no assertions executed |
| [runtime-build143](frozen143/runtime-build143/receipt.json.gz) · [log](frozen143/runtime-build143/tests.log) | Compilation passed; no assertions executed |
| [registry-build143](frozen143/registry-build143/receipt.json.gz) · [log](frozen143/registry-build143/tests.log) | Compilation blocked; no assertions executed |
| [vm-build143](frozen143/vm-build143/receipt.json.gz) · [log](frozen143/vm-build143/tests.log) | Compilation blocked; no assertions executed |
| [compiler-focused143-0](frozen143/compiler-focused143-0/receipt.json.gz) · [log](frozen143/compiler-focused143-0/tests.log) | 3 passed, 0 failed |
| [compiler-focused143-1](frozen143/compiler-focused143-1/receipt.json.gz) · [log](frozen143/compiler-focused143-1/tests.log) | 1 passed, 1 failed |
| [compiler-focused143-2](frozen143/compiler-focused143-2/receipt.json.gz) · [log](frozen143/compiler-focused143-2/tests.log) | 0 passed, 1 failed |
| [compiler-focused143-3](frozen143/compiler-focused143-3/receipt.json.gz) · [log](frozen143/compiler-focused143-3/tests.log) | 0 passed, 1 failed |
| [runtime-focused143-0](frozen143/runtime-focused143-0/receipt.json.gz) · [log](frozen143/runtime-focused143-0/tests.log) | 1 passed, 0 failed |
| [runtime-focused143-1](frozen143/runtime-focused143-1/receipt.json.gz) · [log](frozen143/runtime-focused143-1/tests.log) | 1 passed, 0 failed |
| [runtime-focused143-2](frozen143/runtime-focused143-2/receipt.json.gz) · [log](frozen143/runtime-focused143-2/tests.log) | 1 passed, 0 failed |
| [runtime-focused143-3](frozen143/runtime-focused143-3/receipt.json.gz) · [log](frozen143/runtime-focused143-3/tests.log) | 0 passed, 1 failed |
| [runtime-focused143-4](frozen143/runtime-focused143-4/receipt.json.gz) · [log](frozen143/runtime-focused143-4/tests.log) | 1 passed, 0 failed |
| [runtime-focused143-5](frozen143/runtime-focused143-5/receipt.json.gz) · [log](frozen143/runtime-focused143-5/tests.log) | 1 passed, 0 failed |
| [runtime-focused143-6](frozen143/runtime-focused143-6/receipt.json.gz) · [log](frozen143/runtime-focused143-6/tests.log) | 1 passed, 0 failed |
| [runtime-focused143-7](frozen143/runtime-focused143-7/receipt.json.gz) · [log](frozen143/runtime-focused143-7/tests.log) | 1 passed, 0 failed |
| [runtime-focused143-8](frozen143/runtime-focused143-8/receipt.json.gz) · [log](frozen143/runtime-focused143-8/tests.log) | 0 passed, 1 failed |
| [runtime-focused143-9](frozen143/runtime-focused143-9/receipt.json.gz) · [log](frozen143/runtime-focused143-9/tests.log) | 0 passed, 1 failed |
| [clippy-workspace143](frozen143/clippy-workspace143/receipt.json.gz) · [log](frozen143/clippy-workspace143/tests.log) | Clippy failed; no assertions executed |

## Retained source 144 commands

The [compiler executable pin](frozen144/pinned-compiler144.json), [core executable pin](frozen144/pinned-core144.json), [vm executable pin](frozen144/pinned-vm144.json) identify the actual successful test builds and selected binaries. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. The blocked Runtime build and the three test-listing commands execute no assertions; these commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build144](frozen144/compiler-build144/receipt.json.gz) · [log](frozen144/compiler-build144/tests.log) | Compilation passed; no assertions executed |
| [core-build144](frozen144/core-build144/receipt.json.gz) · [log](frozen144/core-build144/tests.log) | Compilation passed; no assertions executed |
| [runtime-build144](frozen144/runtime-build144/receipt.json.gz) · [log](frozen144/runtime-build144/tests.log) | Compilation blocked; no assertions executed |
| [vm-build144](frozen144/vm-build144/receipt.json.gz) · [log](frozen144/vm-build144/tests.log) | Compilation passed; no assertions executed |
| [compiler-focused144-0](frozen144/compiler-focused144-0/receipt.json.gz) · [log](frozen144/compiler-focused144-0/tests.log) | 0 passed, 1 failed |
| [compiler-focused144-1](frozen144/compiler-focused144-1/receipt.json.gz) · [log](frozen144/compiler-focused144-1/tests.log) | 2 passed, 0 failed |
| [compiler-focused144-2](frozen144/compiler-focused144-2/receipt.json.gz) · [log](frozen144/compiler-focused144-2/tests.log) | 1 passed, 2 failed |
| [compiler-focused144-3](frozen144/compiler-focused144-3/receipt.json.gz) · [log](frozen144/compiler-focused144-3/tests.log) | 6 passed, 1 failed |
| [core-focused144-0](frozen144/core-focused144-0/receipt.json.gz) · [log](frozen144/core-focused144-0/tests.log) | 28 passed, 2 failed |
| [core-focused144-1](frozen144/core-focused144-1/receipt.json.gz) · [log](frozen144/core-focused144-1/tests.log) | 0 passed, 1 failed |
| [core-focused144-2](frozen144/core-focused144-2/receipt.json.gz) · [log](frozen144/core-focused144-2/tests.log) | 0 passed, 1 failed |
| [core-focused144-3](frozen144/core-focused144-3/receipt.json.gz) · [log](frozen144/core-focused144-3/tests.log) | 0 passed, 1 failed |
| [vm-focused144-0](frozen144/vm-focused144-0/receipt.json.gz) · [log](frozen144/vm-focused144-0/tests.log) | 0 passed, 1 failed |
| [vm-focused144-1](frozen144/vm-focused144-1/receipt.json.gz) · [log](frozen144/vm-focused144-1/tests.log) | 0 passed, 1 failed |
| [vm-focused144-2](frozen144/vm-focused144-2/receipt.json.gz) · [log](frozen144/vm-focused144-2/tests.log) | 1 passed, 0 failed |
| [vm-focused144-3](frozen144/vm-focused144-3/receipt.json.gz) · [log](frozen144/vm-focused144-3/tests.log) | 0 passed, 1 failed |
| [vm-focused144-4](frozen144/vm-focused144-4/receipt.json.gz) · [log](frozen144/vm-focused144-4/tests.log) | 0 passed, 1 failed |
| [vm-focused144-5](frozen144/vm-focused144-5/receipt.json.gz) · [log](frozen144/vm-focused144-5/tests.log) | 0 passed, 1 failed |
| [vm-focused144-6](frozen144/vm-focused144-6/receipt.json.gz) · [log](frozen144/vm-focused144-6/tests.log) | 0 passed, 1 failed |
| [vm-focused144-7](frozen144/vm-focused144-7/receipt.json.gz) · [log](frozen144/vm-focused144-7/tests.log) | 0 passed, 1 failed |
| [vm-focused144-8](frozen144/vm-focused144-8/receipt.json.gz) · [log](frozen144/vm-focused144-8/tests.log) | 0 passed, 1 failed |
| [vm-focused144-9](frozen144/vm-focused144-9/receipt.json.gz) · [log](frozen144/vm-focused144-9/tests.log) | 0 passed, 1 failed |
| [vm-focused144-10](frozen144/vm-focused144-10/receipt.json.gz) · [log](frozen144/vm-focused144-10/tests.log) | 1 passed, 0 failed |
| [vm-focused144-11](frozen144/vm-focused144-11/receipt.json.gz) · [log](frozen144/vm-focused144-11/tests.log) | 1 passed, 0 failed |
| [vm-focused144-12](frozen144/vm-focused144-12/receipt.json.gz) · [log](frozen144/vm-focused144-12/tests.log) | 1 passed, 0 failed |
| [vm-focused144-13](frozen144/vm-focused144-13/receipt.json.gz) · [log](frozen144/vm-focused144-13/tests.log) | 1 passed, 0 failed |
| [vm-focused144-14](frozen144/vm-focused144-14/receipt.json.gz) · [log](frozen144/vm-focused144-14/tests.log) | 1 passed, 0 failed |
| [compiler-lifecycle144-0](frozen144/compiler-lifecycle144-0/receipt.json.gz) · [log](frozen144/compiler-lifecycle144-0/tests.log) | 0 passed, 1 failed |
| [compiler-lifecycle144-1](frozen144/compiler-lifecycle144-1/receipt.json.gz) · [log](frozen144/compiler-lifecycle144-1/tests.log) | 0 passed, 1 failed |
| [core-formal-trace144-0](frozen144/core-formal-trace144-0/receipt.json.gz) · [log](frozen144/core-formal-trace144-0/tests.log) | 0 passed, 1 failed |
| [core-formal-trace144-1](frozen144/core-formal-trace144-1/receipt.json.gz) · [log](frozen144/core-formal-trace144-1/tests.log) | 0 passed, 1 failed |
| [core-formal-trace144-2](frozen144/core-formal-trace144-2/receipt.json.gz) · [log](frozen144/core-formal-trace144-2/tests.log) | 0 passed, 1 failed |
| [compiler-list144](frozen144/compiler-list144/receipt.json.gz) · [log](frozen144/compiler-list144/tests.log) | Tests listed; no assertions executed |
| [core-list144](frozen144/core-list144/receipt.json.gz) · [log](frozen144/core-list144/tests.log) | Tests listed; no assertions executed |
| [vm-list144](frozen144/vm-list144/receipt.json.gz) · [log](frozen144/vm-list144/tests.log) | Tests listed; no assertions executed |

## Retained source 147 commands

The [runtime executable pin](frozen147/pinned-runtime147.json) identify the actual successful test builds and selected binaries. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. The blocked Compiler and VM builds and the test-listing command execute no assertions; these commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build147](frozen147/compiler-build147/receipt.json.gz) · [log](frozen147/compiler-build147/tests.log) | Compilation blocked; no assertions executed |
| [runtime-build147](frozen147/runtime-build147/receipt.json.gz) · [log](frozen147/runtime-build147/tests.log) | Compilation passed; no assertions executed |
| [vm-build147](frozen147/vm-build147/receipt.json.gz) · [log](frozen147/vm-build147/tests.log) | Compilation blocked; no assertions executed |
| [runtime-focus147-0](frozen147/runtime-focus147-0/receipt.json.gz) · [log](frozen147/runtime-focus147-0/tests.log) | 1 passed, 0 failed |
| [runtime-focus147-1](frozen147/runtime-focus147-1/receipt.json.gz) · [log](frozen147/runtime-focus147-1/tests.log) | 1 passed, 0 failed |
| [runtime-focus147-2](frozen147/runtime-focus147-2/receipt.json.gz) · [log](frozen147/runtime-focus147-2/tests.log) | 0 passed, 1 failed |
| [runtime-focus147-3](frozen147/runtime-focus147-3/receipt.json.gz) · [log](frozen147/runtime-focus147-3/tests.log) | 1 passed, 0 failed |
| [runtime-focus147-4](frozen147/runtime-focus147-4/receipt.json.gz) · [log](frozen147/runtime-focus147-4/tests.log) | 1 passed, 0 failed |
| [runtime-focus147-5](frozen147/runtime-focus147-5/receipt.json.gz) · [log](frozen147/runtime-focus147-5/tests.log) | 1 passed, 0 failed |
| [runtime-focus147-6](frozen147/runtime-focus147-6/receipt.json.gz) · [log](frozen147/runtime-focus147-6/tests.log) | 0 passed, 0 failed |
| [runtime-list147](frozen147/runtime-list147/receipt.json.gz) · [log](frozen147/runtime-list147/tests.log) | Tests listed; no assertions executed |

## Retained source 149 commands

The [compiler executable pin](frozen149/pinned-compiler149.json), [core executable pin](frozen149/pinned-core149.json), [vm executable pin](frozen149/pinned-vm149.json) identify the actual successful test builds and selected binaries. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. The three build commands and two failed Clippy commands execute no assertions. The absent Compiler focused6 command supplies no receipt or assertion result. These commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build149](frozen149/compiler-build149/receipt.json.gz) · [log](frozen149/compiler-build149/tests.log) | Compilation passed; no assertions executed |
| [compiler-default-region149](frozen149/compiler-default-region149/receipt.json.gz) · [log](frozen149/compiler-default-region149/tests.log) | 1 passed, 0 failed |
| [compiler-extra149-0](frozen149/compiler-extra149-0/receipt.json.gz) · [log](frozen149/compiler-extra149-0/tests.log) | 1 passed, 2 failed |
| [compiler-extra149-1](frozen149/compiler-extra149-1/receipt.json.gz) · [log](frozen149/compiler-extra149-1/tests.log) | 1 passed, 2 failed |
| [compiler-extra149-2](frozen149/compiler-extra149-2/receipt.json.gz) · [log](frozen149/compiler-extra149-2/tests.log) | 22 passed, 11 failed |
| [compiler-extra149-3](frozen149/compiler-extra149-3/receipt.json.gz) · [log](frozen149/compiler-extra149-3/tests.log) | 1 passed, 1 failed |
| [compiler-focused149-0](frozen149/compiler-focused149-0/receipt.json.gz) · [log](frozen149/compiler-focused149-0/tests.log) | 0 passed, 1 failed |
| [compiler-focused149-1](frozen149/compiler-focused149-1/receipt.json.gz) · [log](frozen149/compiler-focused149-1/tests.log) | 1 passed, 2 failed |
| [compiler-focused149-2](frozen149/compiler-focused149-2/receipt.json.gz) · [log](frozen149/compiler-focused149-2/tests.log) | 3 passed, 1 failed |
| [compiler-focused149-3](frozen149/compiler-focused149-3/receipt.json.gz) · [log](frozen149/compiler-focused149-3/tests.log) | 7 passed, 0 failed |
| [compiler-focused149-4](frozen149/compiler-focused149-4/receipt.json.gz) · [log](frozen149/compiler-focused149-4/tests.log) | 1 passed, 1 failed |
| [compiler-focused149-5](frozen149/compiler-focused149-5/receipt.json.gz) · [log](frozen149/compiler-focused149-5/tests.log) | 1 passed, 1 failed |
| [compiler-math-trace149](frozen149/compiler-math-trace149/receipt.json.gz) · [log](frozen149/compiler-math-trace149/tests.log) | 0 passed, 1 failed |
| [compiler-math149](frozen149/compiler-math149/receipt.json.gz) · [log](frozen149/compiler-math149/tests.log) | 4 passed, 1 failed |
| [compiler-package-trace149](frozen149/compiler-package-trace149/receipt.json.gz) · [log](frozen149/compiler-package-trace149/tests.log) | 0 passed, 1 failed |
| [core-build149](frozen149/core-build149/receipt.json.gz) · [log](frozen149/core-build149/tests.log) | Compilation passed; no assertions executed |
| [core-focused149-0](frozen149/core-focused149-0/receipt.json.gz) · [log](frozen149/core-focused149-0/tests.log) | 30 passed, 0 failed |
| [core-focused149-1](frozen149/core-focused149-1/receipt.json.gz) · [log](frozen149/core-focused149-1/tests.log) | 1 passed, 0 failed |
| [core-focused149-2](frozen149/core-focused149-2/receipt.json.gz) · [log](frozen149/core-focused149-2/tests.log) | 0 passed, 1 failed |
| [core-focused149-3](frozen149/core-focused149-3/receipt.json.gz) · [log](frozen149/core-focused149-3/tests.log) | 1 passed, 0 failed |
| [core-math149](frozen149/core-math149/receipt.json.gz) · [log](frozen149/core-math149/tests.log) | 9 passed, 1 failed |
| [runtime-clippy149](frozen149/runtime-clippy149/receipt.json.gz) · [log](frozen149/runtime-clippy149/tests.log) | Clippy failed; no assertions executed |
| [vm-build149](frozen149/vm-build149/receipt.json.gz) · [log](frozen149/vm-build149/tests.log) | Compilation passed; no assertions executed |
| [vm-focused149-0](frozen149/vm-focused149-0/receipt.json.gz) · [log](frozen149/vm-focused149-0/tests.log) | 1 passed, 0 failed |
| [vm-focused149-1](frozen149/vm-focused149-1/receipt.json.gz) · [log](frozen149/vm-focused149-1/tests.log) | 1 passed, 0 failed |
| [vm-focused149-2](frozen149/vm-focused149-2/receipt.json.gz) · [log](frozen149/vm-focused149-2/tests.log) | 1 passed, 0 failed |
| [vm-focused149-3](frozen149/vm-focused149-3/receipt.json.gz) · [log](frozen149/vm-focused149-3/tests.log) | 1 passed, 0 failed |
| [vm-focused149-4](frozen149/vm-focused149-4/receipt.json.gz) · [log](frozen149/vm-focused149-4/tests.log) | 1 passed, 0 failed |
| [vm-focused149-5](frozen149/vm-focused149-5/receipt.json.gz) · [log](frozen149/vm-focused149-5/tests.log) | 1 passed, 0 failed |
| [vm-focused149-6](frozen149/vm-focused149-6/receipt.json.gz) · [log](frozen149/vm-focused149-6/tests.log) | 1 passed, 0 failed |
| [vm-focused149-7](frozen149/vm-focused149-7/receipt.json.gz) · [log](frozen149/vm-focused149-7/tests.log) | 1 passed, 0 failed |
| [vm-focused149-8](frozen149/vm-focused149-8/receipt.json.gz) · [log](frozen149/vm-focused149-8/tests.log) | 1 passed, 0 failed |
| [vm-focused149-9](frozen149/vm-focused149-9/receipt.json.gz) · [log](frozen149/vm-focused149-9/tests.log) | 1 passed, 0 failed |
| [workspace-clippy149](frozen149/workspace-clippy149/receipt.json.gz) · [log](frozen149/workspace-clippy149/tests.log) | Clippy failed; no assertions executed |

## Retained source 150 commands

The two retained Compiler and Runtime build commands share the same frozen source inventory. Both compilations are blocked before assertions. These receipts establish no executable pin, Native provider answer, Rust assertion outcome, current-source or final gate success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build150](frozen150/compiler-build150/receipt.json.gz) · [log](frozen150/compiler-build150/tests.log) | Compilation blocked; no assertions executed |
| [runtime-build150](frozen150/runtime-build150/receipt.json.gz) · [log](frozen150/runtime-build150/tests.log) | Compilation blocked; no assertions executed |


## Retained source 151 commands

The two retained Compiler and Runtime build commands share the same frozen source inventory. Both compilations are blocked before assertions. These receipts establish no executable pin, Native provider answer, Rust assertion outcome, current-source or final gate success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build151](frozen151/compiler-build151/receipt.json.gz) · [log](frozen151/compiler-build151/tests.log) | Compilation blocked; no assertions executed |
| [runtime-build151](frozen151/runtime-build151/receipt.json.gz) · [log](frozen151/runtime-build151/tests.log) | Compilation blocked; no assertions executed |

## Retained source 152 commands

The three retained Compiler, Runtime and Registry build commands share the same frozen source inventory. All three compilations are blocked before assertions. These receipts establish no executable pin, Native provider answer, Rust assertion outcome, current-source or final gate success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build152](frozen152/compiler-build152/receipt.json.gz) · [log](frozen152/compiler-build152/tests.log) | Compilation blocked; no assertions executed |
| [runtime-build152](frozen152/runtime-build152/receipt.json.gz) · [log](frozen152/runtime-build152/tests.log) | Compilation blocked; no assertions executed |
| [registry-build152](frozen152/registry-build152/receipt.json.gz) · [log](frozen152/registry-build152/tests.log) | Compilation blocked; no assertions executed |

## Retained source 148 commands

The [runtime executable pin](frozen148/pinned-runtime148.json) identifies the actual successful Runtime test build and selected binary. All nine commands retain the same frozen source inventory. Compiler and VM compilations are blocked before assertions; the Runtime build passes. Six selected Runtime commands execute eight passing assertions. No other command, complete suite, Native provider result, final gate or current-source success is established.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build148](frozen148/compiler-build148/receipt.json.gz) · [log](frozen148/compiler-build148/tests.log) | Compilation blocked; no assertions executed |
| [runtime-build148](frozen148/runtime-build148/receipt.json.gz) · [log](frozen148/runtime-build148/tests.log) | Compilation passed; no assertions executed |
| [runtime-focus148-0](frozen148/runtime-focus148-0/receipt.json.gz) · [log](frozen148/runtime-focus148-0/tests.log) | 2 passed, 0 failed |
| [runtime-focus148-1](frozen148/runtime-focus148-1/receipt.json.gz) · [log](frozen148/runtime-focus148-1/tests.log) | 1 passed, 0 failed |
| [runtime-focus148-2](frozen148/runtime-focus148-2/receipt.json.gz) · [log](frozen148/runtime-focus148-2/tests.log) | 2 passed, 0 failed |
| [runtime-focus148-3](frozen148/runtime-focus148-3/receipt.json.gz) · [log](frozen148/runtime-focus148-3/tests.log) | 1 passed, 0 failed |
| [runtime-focus148-4](frozen148/runtime-focus148-4/receipt.json.gz) · [log](frozen148/runtime-focus148-4/tests.log) | 1 passed, 0 failed |
| [runtime-frame-storage148](frozen148/runtime-frame-storage148/receipt.json.gz) · [log](frozen148/runtime-frame-storage148/tests.log) | 1 passed, 0 failed |
| [vm-build148](frozen148/vm-build148/receipt.json.gz) · [log](frozen148/vm-build148/tests.log) | Compilation blocked; no assertions executed |

## Retained source 153 commands

The [compiler executable pin](frozen153/pinned-compiler153.json), [runtime executable pin](frozen153/pinned-runtime153.json) identify the actual successful test builds and selected binaries. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. The three build commands and successful Runtime Clippy command execute no assertions. The Core build is blocked before assertions. Only retained completed commands supply results. These commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build153](frozen153/compiler-build153/receipt.json.gz) · [log](frozen153/compiler-build153/tests.log) | Compilation passed; no assertions executed |
| [compiler-caller-inventory153](frozen153/compiler-caller-inventory153/receipt.json.gz) · [log](frozen153/compiler-caller-inventory153/tests.log) | 1 passed, 0 failed |
| [compiler-default-region153](frozen153/compiler-default-region153/receipt.json.gz) · [log](frozen153/compiler-default-region153/tests.log) | 1 passed, 0 failed |
| [compiler-extra153-0](frozen153/compiler-extra153-0/receipt.json.gz) · [log](frozen153/compiler-extra153-0/tests.log) | 2 passed, 1 failed |
| [compiler-extra153-1](frozen153/compiler-extra153-1/receipt.json.gz) · [log](frozen153/compiler-extra153-1/tests.log) | 1 passed, 3 failed |
| [compiler-extra153-2](frozen153/compiler-extra153-2/receipt.json.gz) · [log](frozen153/compiler-extra153-2/tests.log) | 23 passed, 10 failed |
| [compiler-extra153-3](frozen153/compiler-extra153-3/receipt.json.gz) · [log](frozen153/compiler-extra153-3/tests.log) | 1 passed, 1 failed |
| [compiler-focused153-0](frozen153/compiler-focused153-0/receipt.json.gz) · [log](frozen153/compiler-focused153-0/tests.log) | 0 passed, 1 failed |
| [compiler-focused153-1](frozen153/compiler-focused153-1/receipt.json.gz) · [log](frozen153/compiler-focused153-1/tests.log) | 2 passed, 1 failed |
| [compiler-focused153-2](frozen153/compiler-focused153-2/receipt.json.gz) · [log](frozen153/compiler-focused153-2/tests.log) | 3 passed, 1 failed |
| [compiler-focused153-3](frozen153/compiler-focused153-3/receipt.json.gz) · [log](frozen153/compiler-focused153-3/tests.log) | 7 passed, 0 failed |
| [compiler-focused153-4](frozen153/compiler-focused153-4/receipt.json.gz) · [log](frozen153/compiler-focused153-4/tests.log) | 1 passed, 1 failed |
| [compiler-focused153-5](frozen153/compiler-focused153-5/receipt.json.gz) · [log](frozen153/compiler-focused153-5/tests.log) | 1 passed, 1 failed |
| [compiler-math-source-trace153](frozen153/compiler-math-source-trace153/receipt.json.gz) · [log](frozen153/compiler-math-source-trace153/tests.log) | 0 passed, 1 failed |
| [compiler-math153](frozen153/compiler-math153/receipt.json.gz) · [log](frozen153/compiler-math153/tests.log) | 4 passed, 1 failed |
| [compiler-new153-0](frozen153/compiler-new153-0/receipt.json.gz) · [log](frozen153/compiler-new153-0/tests.log) | 1 passed, 0 failed |
| [compiler-new153-1](frozen153/compiler-new153-1/receipt.json.gz) · [log](frozen153/compiler-new153-1/tests.log) | 1 passed, 0 failed |
| [compiler-new153-2](frozen153/compiler-new153-2/receipt.json.gz) · [log](frozen153/compiler-new153-2/tests.log) | 1 passed, 0 failed |
| [compiler-new153-3](frozen153/compiler-new153-3/receipt.json.gz) · [log](frozen153/compiler-new153-3/tests.log) | 0 passed, 1 failed |
| [compiler-new153-4](frozen153/compiler-new153-4/receipt.json.gz) · [log](frozen153/compiler-new153-4/tests.log) | 0 passed, 1 failed |
| [compiler-new153-5](frozen153/compiler-new153-5/receipt.json.gz) · [log](frozen153/compiler-new153-5/tests.log) | 2 passed, 0 failed |
| [compiler-new153-6](frozen153/compiler-new153-6/receipt.json.gz) · [log](frozen153/compiler-new153-6/tests.log) | 2 passed, 1 failed |
| [compiler-new153-7](frozen153/compiler-new153-7/receipt.json.gz) · [log](frozen153/compiler-new153-7/tests.log) | 1 passed, 0 failed |
| [compiler-receiver-formal-trace153](frozen153/compiler-receiver-formal-trace153/receipt.json.gz) · [log](frozen153/compiler-receiver-formal-trace153/tests.log) | 0 passed, 1 failed |
| [core-build153](frozen153/core-build153/receipt.json.gz) · [log](frozen153/core-build153/tests.log) | Compilation blocked; no assertions executed |
| [runtime-array-write153](frozen153/runtime-array-write153/receipt.json.gz) · [log](frozen153/runtime-array-write153/tests.log) | 3 passed, 1 failed |
| [runtime-build153](frozen153/runtime-build153/receipt.json.gz) · [log](frozen153/runtime-build153/tests.log) | Compilation passed; no assertions executed |
| [runtime-clippy153](frozen153/runtime-clippy153/receipt.json.gz) · [log](frozen153/runtime-clippy153/tests.log) | Clippy passed for this source inventory; no assertions executed |
| [runtime-container153](frozen153/runtime-container153/receipt.json.gz) · [log](frozen153/runtime-container153/tests.log) | 1 passed, 0 failed |

## Retained source 154 commands

The [runtime executable pin](frozen154/pinned-runtime154.json) identify the actual successful test builds and selected binaries. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. Build and Clippy commands execute no assertions. A combined Compiler/Core build retains one actual receipt, even when it supplies both independently pinned executables. Only retained completed commands supply results, and zero-matched filters supply no assertion coverage. These commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build154](frozen154/compiler-build154/receipt.json.gz) · [log](frozen154/compiler-build154/tests.log) | Compilation blocked; no assertions executed |
| [core-build154](frozen154/core-build154/receipt.json.gz) · [log](frozen154/core-build154/tests.log) | Compilation blocked; no assertions executed |
| [runtime-array154](frozen154/runtime-array154/receipt.json.gz) · [log](frozen154/runtime-array154/tests.log) | 0 passed, 1 failed |
| [runtime-build154](frozen154/runtime-build154/receipt.json.gz) · [log](frozen154/runtime-build154/tests.log) | Compilation passed; no assertions executed |


## Retained source 155 commands

The [compiler executable pin](frozen155/pinned-compiler155.json), [core executable pin](frozen155/pinned-core155.json) identify the actual successful test builds and selected binaries. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. Build and Clippy commands execute no assertions. A combined Compiler/Core build retains one actual receipt, even when it supplies both independently pinned executables. Only retained completed commands supply results, and zero-matched filters supply no assertion coverage. These commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-added155-0](frozen155/compiler-added155-0/receipt.json.gz) · [log](frozen155/compiler-added155-0/tests.log) | 1 passed, 0 failed |
| [compiler-added155-1](frozen155/compiler-added155-1/receipt.json.gz) · [log](frozen155/compiler-added155-1/tests.log) | 0 passed, 1 failed |
| [compiler-added155-2](frozen155/compiler-added155-2/receipt.json.gz) · [log](frozen155/compiler-added155-2/tests.log) | 1 passed, 0 failed |
| [compiler-added155-3](frozen155/compiler-added155-3/receipt.json.gz) · [log](frozen155/compiler-added155-3/tests.log) | 0 passed, 1 failed |
| [compiler-added155-4](frozen155/compiler-added155-4/receipt.json.gz) · [log](frozen155/compiler-added155-4/tests.log) | 0 passed, 1 failed |
| [compiler-build155](frozen155/compiler-build155/receipt.json.gz) · [log](frozen155/compiler-build155/tests.log) | Compilation passed; no assertions executed |
| [compiler-caller-inventory155](frozen155/compiler-caller-inventory155/receipt.json.gz) · [log](frozen155/compiler-caller-inventory155/tests.log) | 1 passed, 0 failed |
| [compiler-default-region155](frozen155/compiler-default-region155/receipt.json.gz) · [log](frozen155/compiler-default-region155/tests.log) | 1 passed, 0 failed |
| [compiler-extra155-0](frozen155/compiler-extra155-0/receipt.json.gz) · [log](frozen155/compiler-extra155-0/tests.log) | 3 passed, 0 failed |
| [compiler-extra155-1](frozen155/compiler-extra155-1/receipt.json.gz) · [log](frozen155/compiler-extra155-1/tests.log) | 2 passed, 2 failed |
| [compiler-extra155-2](frozen155/compiler-extra155-2/receipt.json.gz) · [log](frozen155/compiler-extra155-2/tests.log) | 23 passed, 10 failed |
| [compiler-extra155-3](frozen155/compiler-extra155-3/receipt.json.gz) · [log](frozen155/compiler-extra155-3/tests.log) | 2 passed, 0 failed |
| [compiler-extra155added-0](frozen155/compiler-extra155added-0/receipt.json.gz) · [log](frozen155/compiler-extra155added-0/tests.log) | 0 passed, 1 failed |
| [compiler-extra155added-1](frozen155/compiler-extra155added-1/receipt.json.gz) · [log](frozen155/compiler-extra155added-1/tests.log) | 1 passed, 0 failed |
| [compiler-focused155-0](frozen155/compiler-focused155-0/receipt.json.gz) · [log](frozen155/compiler-focused155-0/tests.log) | 0 passed, 1 failed |
| [compiler-focused155-1](frozen155/compiler-focused155-1/receipt.json.gz) · [log](frozen155/compiler-focused155-1/tests.log) | 1 passed, 2 failed |
| [compiler-focused155-2](frozen155/compiler-focused155-2/receipt.json.gz) · [log](frozen155/compiler-focused155-2/tests.log) | 2 passed, 2 failed |
| [compiler-focused155-3](frozen155/compiler-focused155-3/receipt.json.gz) · [log](frozen155/compiler-focused155-3/tests.log) | 8 passed, 0 failed |
| [compiler-focused155-4](frozen155/compiler-focused155-4/receipt.json.gz) · [log](frozen155/compiler-focused155-4/tests.log) | 1 passed, 1 failed |
| [compiler-math155](frozen155/compiler-math155/receipt.json.gz) · [log](frozen155/compiler-math155/tests.log) | 4 passed, 1 failed |
| [compiler-new155-0](frozen155/compiler-new155-0/receipt.json.gz) · [log](frozen155/compiler-new155-0/tests.log) | 1 passed, 0 failed |
| [compiler-new155-1](frozen155/compiler-new155-1/receipt.json.gz) · [log](frozen155/compiler-new155-1/tests.log) | 1 passed, 0 failed |
| [compiler-new155-2](frozen155/compiler-new155-2/receipt.json.gz) · [log](frozen155/compiler-new155-2/tests.log) | 1 passed, 0 failed |
| [compiler-new155-3](frozen155/compiler-new155-3/receipt.json.gz) · [log](frozen155/compiler-new155-3/tests.log) | 1 passed, 0 failed |
| [compiler-new155-4](frozen155/compiler-new155-4/receipt.json.gz) · [log](frozen155/compiler-new155-4/tests.log) | 1 passed, 0 failed |
| [compiler-new155-5](frozen155/compiler-new155-5/receipt.json.gz) · [log](frozen155/compiler-new155-5/tests.log) | 2 passed, 1 failed |
| [compiler-new155-6](frozen155/compiler-new155-6/receipt.json.gz) · [log](frozen155/compiler-new155-6/tests.log) | 2 passed, 1 failed |
| [compiler-new155-7](frozen155/compiler-new155-7/receipt.json.gz) · [log](frozen155/compiler-new155-7/tests.log) | 1 passed, 0 failed |
| [core-focused155-0](frozen155/core-focused155-0/receipt.json.gz) · [log](frozen155/core-focused155-0/tests.log) | 1 passed, 0 failed |
| [core-focused155-1](frozen155/core-focused155-1/receipt.json.gz) · [log](frozen155/core-focused155-1/tests.log) | 1 passed, 0 failed |
| [core-focused155-10](frozen155/core-focused155-10/receipt.json.gz) · [log](frozen155/core-focused155-10/tests.log) | 1 passed, 0 failed |
| [core-focused155-11](frozen155/core-focused155-11/receipt.json.gz) · [log](frozen155/core-focused155-11/tests.log) | 1 passed, 0 failed |
| [core-focused155-12](frozen155/core-focused155-12/receipt.json.gz) · [log](frozen155/core-focused155-12/tests.log) | 0 passed, 1 failed |
| [core-focused155-13](frozen155/core-focused155-13/receipt.json.gz) · [log](frozen155/core-focused155-13/tests.log) | 1 passed, 0 failed |
| [core-focused155-14](frozen155/core-focused155-14/receipt.json.gz) · [log](frozen155/core-focused155-14/tests.log) | 1 passed, 0 failed |
| [core-focused155-15](frozen155/core-focused155-15/receipt.json.gz) · [log](frozen155/core-focused155-15/tests.log) | 0 passed, 1 failed |
| [core-focused155-16](frozen155/core-focused155-16/receipt.json.gz) · [log](frozen155/core-focused155-16/tests.log) | 1 passed, 0 failed |
| [core-focused155-17](frozen155/core-focused155-17/receipt.json.gz) · [log](frozen155/core-focused155-17/tests.log) | 1 passed, 0 failed |
| [core-focused155-18](frozen155/core-focused155-18/receipt.json.gz) · [log](frozen155/core-focused155-18/tests.log) | 0 passed, 1 failed |
| [core-focused155-19](frozen155/core-focused155-19/receipt.json.gz) · [log](frozen155/core-focused155-19/tests.log) | 1 passed, 0 failed |
| [core-focused155-2](frozen155/core-focused155-2/receipt.json.gz) · [log](frozen155/core-focused155-2/tests.log) | 1 passed, 0 failed |
| [core-focused155-20](frozen155/core-focused155-20/receipt.json.gz) · [log](frozen155/core-focused155-20/tests.log) | 0 passed, 1 failed |
| [core-focused155-21](frozen155/core-focused155-21/receipt.json.gz) · [log](frozen155/core-focused155-21/tests.log) | 0 passed, 0 failed |
| [core-focused155-22](frozen155/core-focused155-22/receipt.json.gz) · [log](frozen155/core-focused155-22/tests.log) | 0 passed, 0 failed |
| [core-focused155-23](frozen155/core-focused155-23/receipt.json.gz) · [log](frozen155/core-focused155-23/tests.log) | 1 passed, 0 failed |
| [core-focused155-24](frozen155/core-focused155-24/receipt.json.gz) · [log](frozen155/core-focused155-24/tests.log) | 1 passed, 0 failed |
| [core-focused155-25](frozen155/core-focused155-25/receipt.json.gz) · [log](frozen155/core-focused155-25/tests.log) | 1 passed, 0 failed |
| [core-focused155-3](frozen155/core-focused155-3/receipt.json.gz) · [log](frozen155/core-focused155-3/tests.log) | 1 passed, 0 failed |
| [core-focused155-4](frozen155/core-focused155-4/receipt.json.gz) · [log](frozen155/core-focused155-4/tests.log) | 1 passed, 0 failed |
| [core-focused155-5](frozen155/core-focused155-5/receipt.json.gz) · [log](frozen155/core-focused155-5/tests.log) | 1 passed, 0 failed |
| [core-focused155-6](frozen155/core-focused155-6/receipt.json.gz) · [log](frozen155/core-focused155-6/tests.log) | 1 passed, 0 failed |
| [core-focused155-7](frozen155/core-focused155-7/receipt.json.gz) · [log](frozen155/core-focused155-7/tests.log) | 1 passed, 0 failed |
| [core-focused155-8](frozen155/core-focused155-8/receipt.json.gz) · [log](frozen155/core-focused155-8/tests.log) | 1 passed, 0 failed |
| [core-focused155-9](frozen155/core-focused155-9/receipt.json.gz) · [log](frozen155/core-focused155-9/tests.log) | 1 passed, 0 failed |
| [workspace-clippy155](frozen155/workspace-clippy155/receipt.json.gz) · [log](frozen155/workspace-clippy155/tests.log) | Clippy failed; no assertions executed |

## Retained source 155 additional commands

The [compiler executable pin](frozen155/pinned-compiler155.json) identify the actual successful test builds and selected binaries. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. These three additional selected commands retain their exact source inventory and assertion results independently of the other commands for this source. No later source result or complete-suite claim follows. These commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-advice155-0](frozen155/compiler-advice155-0/receipt.json.gz) · [log](frozen155/compiler-advice155-0/tests.log) | 1 passed, 0 failed |
| [compiler-advice155-1](frozen155/compiler-advice155-1/receipt.json.gz) · [log](frozen155/compiler-advice155-1/tests.log) | 1 passed, 0 failed |
| [compiler-advice155-2](frozen155/compiler-advice155-2/receipt.json.gz) · [log](frozen155/compiler-advice155-2/tests.log) | 1 passed, 0 failed |

## Retained source 156 commands

The [compiler executable pin](frozen156/pinned-compiler156.json), [core executable pin](frozen156/pinned-core156.json), [runtime executable pin](frozen156/pinned-runtime156.json) identify the actual successful test builds and selected binaries. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. Each actual combined Compiler/Core build owns both executable pins; the separate Runtime build and failed workspace Clippy execute no assertions. A zero-matched filter supplies no assertion coverage. Only retained completed commands supply results. These commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-added156-1](frozen156/compiler-added156-1/receipt.json.gz) · [log](frozen156/compiler-added156-1/tests.log) | 0 passed, 1 failed |
| [compiler-added156-4](frozen156/compiler-added156-4/receipt.json.gz) · [log](frozen156/compiler-added156-4/tests.log) | 0 passed, 1 failed |
| [compiler-build156](frozen156/compiler-build156/receipt.json.gz) · [log](frozen156/compiler-build156/tests.log) | Compilation passed; no assertions executed |
| [compiler-extra156-1](frozen156/compiler-extra156-1/receipt.json.gz) · [log](frozen156/compiler-extra156-1/tests.log) | 2 passed, 2 failed |
| [compiler-extra156-2](frozen156/compiler-extra156-2/receipt.json.gz) · [log](frozen156/compiler-extra156-2/tests.log) | 27 passed, 6 failed |
| [compiler-extra156added-0](frozen156/compiler-extra156added-0/receipt.json.gz) · [log](frozen156/compiler-extra156added-0/tests.log) | 1 passed, 0 failed |
| [compiler-focused156-0](frozen156/compiler-focused156-0/receipt.json.gz) · [log](frozen156/compiler-focused156-0/tests.log) | 1 passed, 0 failed |
| [compiler-focused156-1](frozen156/compiler-focused156-1/receipt.json.gz) · [log](frozen156/compiler-focused156-1/tests.log) | 3 passed, 0 failed |
| [compiler-focused156-2](frozen156/compiler-focused156-2/receipt.json.gz) · [log](frozen156/compiler-focused156-2/tests.log) | 4 passed, 0 failed |
| [compiler-focused156-4](frozen156/compiler-focused156-4/receipt.json.gz) · [log](frozen156/compiler-focused156-4/tests.log) | 2 passed, 0 failed |
| [compiler-math156](frozen156/compiler-math156/receipt.json.gz) · [log](frozen156/compiler-math156/tests.log) | 5 passed, 0 failed |
| [compiler-new156-5](frozen156/compiler-new156-5/receipt.json.gz) · [log](frozen156/compiler-new156-5/tests.log) | 2 passed, 1 failed |
| [compiler-new156-6](frozen156/compiler-new156-6/receipt.json.gz) · [log](frozen156/compiler-new156-6/tests.log) | 3 passed, 0 failed |
| [core-namespace156](frozen156/core-namespace156/receipt.json.gz) · [log](frozen156/core-namespace156/tests.log) | 1 passed, 0 failed |
| [runtime-array156](frozen156/runtime-array156/receipt.json.gz) · [log](frozen156/runtime-array156/tests.log) | 0 passed, 1 failed |
| [runtime-build156](frozen156/runtime-build156/receipt.json.gz) · [log](frozen156/runtime-build156/tests.log) | Compilation passed; no assertions executed |
| [workspace-clippy156](frozen156/workspace-clippy156/receipt.json.gz) · [log](frozen156/workspace-clippy156/tests.log) | Clippy failed; no assertions executed |


## Retained source 157 commands

The [compiler executable pin](frozen157/pinned-compiler157.json), [core executable pin](frozen157/pinned-core157.json), [runtime executable pin](frozen157/pinned-runtime157.json) identify the actual successful test builds and selected binaries. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. Each actual combined Compiler/Core build owns both executable pins; the separate Runtime build and failed workspace Clippy execute no assertions. A zero-matched filter supplies no assertion coverage. Only retained completed commands supply results. These commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [child-visibility-trace157](frozen157/child-visibility-trace157/receipt.json.gz) · [log](frozen157/child-visibility-trace157/tests.log) | 3 passed, 2 failed |
| [child-visibility157](frozen157/child-visibility157/receipt.json.gz) · [log](frozen157/child-visibility157/tests.log) | 3 passed, 2 failed |
| [compiler-advice157](frozen157/compiler-advice157/receipt.json.gz) · [log](frozen157/compiler-advice157/tests.log) | 1 passed, 0 failed |
| [compiler-build157](frozen157/compiler-build157/receipt.json.gz) · [log](frozen157/compiler-build157/tests.log) | Compilation passed; no assertions executed |
| [compiler-created-body157](frozen157/compiler-created-body157/receipt.json.gz) · [log](frozen157/compiler-created-body157/tests.log) | 1 passed, 0 failed |
| [compiler-expression157](frozen157/compiler-expression157/receipt.json.gz) · [log](frozen157/compiler-expression157/tests.log) | 1 passed, 0 failed |
| [compiler-parent-realm157](frozen157/compiler-parent-realm157/receipt.json.gz) · [log](frozen157/compiler-parent-realm157/tests.log) | 0 passed, 0 failed |
| [compiler-point-observers157](frozen157/compiler-point-observers157/receipt.json.gz) · [log](frozen157/compiler-point-observers157/tests.log) | 1 passed, 0 failed |
| [compiler-prefix157](frozen157/compiler-prefix157/receipt.json.gz) · [log](frozen157/compiler-prefix157/tests.log) | 2 passed, 0 failed |
| [compiler-tcltest157](frozen157/compiler-tcltest157/receipt.json.gz) · [log](frozen157/compiler-tcltest157/tests.log) | 1 passed, 0 failed |
| [core-source-cards157](frozen157/core-source-cards157/receipt.json.gz) · [log](frozen157/core-source-cards157/tests.log) | 4 passed, 1 failed |
| [core-source-layout157](frozen157/core-source-layout157/receipt.json.gz) · [log](frozen157/core-source-layout157/tests.log) | 13 passed, 1 failed |
| [runtime-build157](frozen157/runtime-build157/receipt.json.gz) · [log](frozen157/runtime-build157/tests.log) | Compilation passed; no assertions executed |
| [runtime-observed157](frozen157/runtime-observed157/receipt.json.gz) · [log](frozen157/runtime-observed157/tests.log) | 7 passed, 1 failed |
| [workspace-clippy157](frozen157/workspace-clippy157/receipt.json.gz) · [log](frozen157/workspace-clippy157/tests.log) | Clippy failed; no assertions executed |

## Retained source 159 commands

The combined Compiler/Core build, separate Runtime build and workspace Clippy share the same frozen source inventory. All three commands are blocked before assertions. These receipts establish no executable pin, Native provider answer, Rust assertion outcome, current-source or final gate success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build159](frozen159/compiler-build159/receipt.json.gz) · [log](frozen159/compiler-build159/tests.log) | Compilation blocked; no assertions executed |
| [runtime-build159](frozen159/runtime-build159/receipt.json.gz) · [log](frozen159/runtime-build159/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy159](frozen159/workspace-clippy159/receipt.json.gz) · [log](frozen159/workspace-clippy159/tests.log) | Clippy failed; no assertions executed |


## Retained source 161 commands

The combined Compiler/Core build, separate Runtime build and workspace Clippy share the same frozen source inventory. All three commands are blocked before assertions. These receipts establish no executable pin, Native provider answer, Rust assertion outcome, current-source or final gate success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build161](frozen161/compiler-build161/receipt.json.gz) · [log](frozen161/compiler-build161/tests.log) | Compilation blocked; no assertions executed |
| [runtime-build161](frozen161/runtime-build161/receipt.json.gz) · [log](frozen161/runtime-build161/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy161](frozen161/workspace-clippy161/receipt.json.gz) · [log](frozen161/workspace-clippy161/tests.log) | Clippy failed; no assertions executed |

## Retained source 162 commands

The [compiler executable pin](frozen162/pinned-compiler162.json), [core executable pin](frozen162/pinned-core162.json), [registry executable pin](frozen162/pinned-registry162.json), [syntax executable pin](frozen162/pinned-syntax162.json) identify the actual successful test builds and selected binaries. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. The one combined Compiler/Core/Registry/Syntax build owns all four executable pins; the blocked separate Runtime compilation and failed workspace Clippy execute no assertions. A zero-matched filter supplies no assertion coverage. Only retained completed commands supply results. These commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-arity162](frozen162/compiler-arity162/receipt.json.gz) · [log](frozen162/compiler-arity162/tests.log) | 0 passed, 1 failed |
| [compiler-build162](frozen162/compiler-build162/receipt.json.gz) · [log](frozen162/compiler-build162/tests.log) | Compilation passed; no assertions executed |
| [compiler-declared162](frozen162/compiler-declared162/receipt.json.gz) · [log](frozen162/compiler-declared162/tests.log) | 0 passed, 2 failed |
| [compiler-lifecycle-fix162](frozen162/compiler-lifecycle-fix162/receipt.json.gz) · [log](frozen162/compiler-lifecycle-fix162/tests.log) | 1 passed, 0 failed |
| [compiler-moved162](frozen162/compiler-moved162/receipt.json.gz) · [log](frozen162/compiler-moved162/tests.log) | 0 passed, 1 failed |
| [compiler-receiver-formal162](frozen162/compiler-receiver-formal162/receipt.json.gz) · [log](frozen162/compiler-receiver-formal162/tests.log) | 1 passed, 0 failed |
| [compiler-safe162](frozen162/compiler-safe162/receipt.json.gz) · [log](frozen162/compiler-safe162/tests.log) | 27 passed, 5 failed |
| [compiler-visibility162](frozen162/compiler-visibility162/receipt.json.gz) · [log](frozen162/compiler-visibility162/tests.log) | 4 passed, 3 failed |
| [core-source-cards162](frozen162/core-source-cards162/receipt.json.gz) · [log](frozen162/core-source-cards162/tests.log) | 5 passed, 0 failed |
| [core-source-roles162](frozen162/core-source-roles162/receipt.json.gz) · [log](frozen162/core-source-roles162/tests.log) | 16 passed, 1 failed |
| [registry-arity-layout-exact162](frozen162/registry-arity-layout-exact162/receipt.json.gz) · [log](frozen162/registry-arity-layout-exact162/tests.log) | 1 passed, 0 failed |
| [registry-arity-layout162](frozen162/registry-arity-layout162/receipt.json.gz) · [log](frozen162/registry-arity-layout162/tests.log) | 0 passed, 0 failed |
| [registry-expression162](frozen162/registry-expression162/receipt.json.gz) · [log](frozen162/registry-expression162/tests.log) | 0 passed, 1 failed |
| [registry-handle162](frozen162/registry-handle162/receipt.json.gz) · [log](frozen162/registry-handle162/tests.log) | 0 passed, 1 failed |
| [registry-source-roles162](frozen162/registry-source-roles162/receipt.json.gz) · [log](frozen162/registry-source-roles162/tests.log) | 0 passed, 1 failed |
| [runtime-build162](frozen162/runtime-build162/receipt.json.gz) · [log](frozen162/runtime-build162/tests.log) | Compilation blocked; no assertions executed |
| [syntax-boundaries162](frozen162/syntax-boundaries162/receipt.json.gz) · [log](frozen162/syntax-boundaries162/tests.log) | 5 passed, 0 failed |
| [syntax-root-proc162](frozen162/syntax-root-proc162/receipt.json.gz) · [log](frozen162/syntax-root-proc162/tests.log) | 1 passed, 0 failed |
| [workspace-clippy162](frozen162/workspace-clippy162/receipt.json.gz) · [log](frozen162/workspace-clippy162/tests.log) | Clippy failed; no assertions executed |

## Retained source 163 commands

The [runtime executable pin](frozen163/pinned-runtime163.json) identify the actual successful test builds and selected binaries. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. The successful Runtime build owns its one executable pin. The combined Compiler/Core/Registry/Syntax/VM build is blocked and supplies no test images or assertions; the blocked workspace Clippy supplies no assertion coverage. The two Runtime assertion commands are recorded independently. A zero-matched filter supplies no assertion coverage. Only retained completed commands supply results. These commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build163](frozen163/compiler-build163/receipt.json.gz) · [log](frozen163/compiler-build163/tests.log) | Compilation blocked; no assertions executed |
| [runtime-build163](frozen163/runtime-build163/receipt.json.gz) · [log](frozen163/runtime-build163/tests.log) | Compilation passed; no assertions executed |
| [runtime-lambda163](frozen163/runtime-lambda163/receipt.json.gz) · [log](frozen163/runtime-lambda163/tests.log) | 1 passed, 0 failed |
| [runtime-observed163](frozen163/runtime-observed163/receipt.json.gz) · [log](frozen163/runtime-observed163/tests.log) | 9 passed, 0 failed |
| [workspace-clippy163](frozen163/workspace-clippy163/receipt.json.gz) · [log](frozen163/workspace-clippy163/tests.log) | Clippy failed; no assertions executed |

## Retained source 165 commands

The [compiler executable pin](frozen165/pinned-compiler165-partial.json) identify the separately emitted actual Compiler lib-test artifact and selected binary. Its immutable pin retains overall_build_exit101, the actual fresh:false compiler-artifact event and Registry-only compiler errors. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. The combined build failed in Registry lib-test compilation. Its completed Compiler artifact was separately pinned and supports only the named Compiler assertion commands; no combined-build success or Core, Registry, Syntax, VM or Runtime test image follows. The failed workspace Clippy executes no assertions. A zero-matched filter supplies no assertion coverage. Only retained completed commands supply results. These commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-arity-exact165](frozen165/compiler-arity-exact165/receipt.json.gz) · [log](frozen165/compiler-arity-exact165/tests.log) | 1 passed, 0 failed |
| [compiler-arity165](frozen165/compiler-arity165/receipt.json.gz) · [log](frozen165/compiler-arity165/tests.log) | 0 passed, 0 failed |
| [compiler-body-owner165](frozen165/compiler-body-owner165/receipt.json.gz) · [log](frozen165/compiler-body-owner165/tests.log) | 1 passed, 0 failed |
| [compiler-build165](frozen165/compiler-build165/receipt.json.gz) · [log](frozen165/compiler-build165/tests.log) | Compilation blocked; no assertions executed |
| [compiler-channel165](frozen165/compiler-channel165/receipt.json.gz) · [log](frozen165/compiler-channel165/tests.log) | 2 passed, 1 failed |
| [compiler-declared-exact165](frozen165/compiler-declared-exact165/receipt.json.gz) · [log](frozen165/compiler-declared-exact165/tests.log) | 1 passed, 6 failed |
| [compiler-declared165](frozen165/compiler-declared165/receipt.json.gz) · [log](frozen165/compiler-declared165/tests.log) | 0 passed, 0 failed |
| [compiler-lambda-arity165](frozen165/compiler-lambda-arity165/receipt.json.gz) · [log](frozen165/compiler-lambda-arity165/tests.log) | 0 passed, 1 failed |
| [compiler-moved-exact165](frozen165/compiler-moved-exact165/receipt.json.gz) · [log](frozen165/compiler-moved-exact165/tests.log) | 0 passed, 1 failed |
| [compiler-moved165](frozen165/compiler-moved165/receipt.json.gz) · [log](frozen165/compiler-moved165/tests.log) | 0 passed, 0 failed |
| [compiler-produced-prefix165](frozen165/compiler-produced-prefix165/receipt.json.gz) · [log](frozen165/compiler-produced-prefix165/tests.log) | 2 passed, 1 failed |
| [compiler-visibility-exact165](frozen165/compiler-visibility-exact165/receipt.json.gz) · [log](frozen165/compiler-visibility-exact165/tests.log) | 5 passed, 2 failed |
| [compiler-visibility165](frozen165/compiler-visibility165/receipt.json.gz) · [log](frozen165/compiler-visibility165/tests.log) | 0 passed, 0 failed |
| [workspace-clippy165](frozen165/workspace-clippy165/receipt.json.gz) · [log](frozen165/workspace-clippy165/tests.log) | Clippy failed; no assertions executed |

## Retained source 164 commands

No test executable image or assertion outcome is supplied by these two commands. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. The combined Compiler/Core/Registry/Syntax/VM build is blocked. The workspace Clippy is also blocked; neither command supplies test images or assertions. No separate Runtime command is present. A zero-matched filter supplies no assertion coverage. Only retained completed commands supply results. These commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build164](frozen164/compiler-build164/receipt.json.gz) · [log](frozen164/compiler-build164/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy164](frozen164/workspace-clippy164/receipt.json.gz) · [log](frozen164/workspace-clippy164/tests.log) | Clippy failed; no assertions executed |

## Retained source 166 commands

No test executable image or assertion outcome is supplied by these two commands. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. The combined Compiler/Core/Registry/Syntax/VM build is blocked. The workspace Clippy is also blocked; neither command supplies test images or assertions. No separate Runtime command is present. A zero-matched filter supplies no assertion coverage. Only retained completed commands supply results. These commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build166](frozen166/compiler-build166/receipt.json.gz) · [log](frozen166/compiler-build166/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy166](frozen166/workspace-clippy166/receipt.json.gz) · [log](frozen166/workspace-clippy166/tests.log) | Clippy failed; no assertions executed |

## Retained source 167 commands

No test executable image or assertion outcome is supplied by these two commands. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. The combined Compiler/Core/Registry/Syntax/VM build is blocked. The workspace Clippy is also blocked; neither command supplies test images or assertions. No separate Runtime command is present. A zero-matched filter supplies no assertion coverage. Only retained completed commands supply results. These commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build167](frozen167/compiler-build167/receipt.json.gz) · [log](frozen167/compiler-build167/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy167](frozen167/workspace-clippy167/receipt.json.gz) · [log](frozen167/workspace-clippy167/tests.log) | Clippy failed; no assertions executed |

## Retained source 168 commands

The [compiler executable pin](frozen168/pinned-compiler168.json), [core executable pin](frozen168/pinned-core168.json), [registry executable pin](frozen168/pinned-registry168.json), [syntax executable pin](frozen168/pinned-syntax168.json), [vm executable pin](frozen168/pinned-vm168.json) identify the actual successful test builds and selected binaries. All commands in this group retain the same frozen source inventory. Passing and failing selected assertions are recorded individually. The one combined Compiler/Core/Registry/Syntax/VM build owns all five executable pins. The workspace Clippy fails and executes no assertions; no separate Runtime command is supplied. The VM SDK lambda comparison fails at the current Jim raw-string absolute-name result and establishes no complete VM constructor correspondence. Passing assertions retain their own source-ownership or bounded provider comparison scopes. A zero-matched filter supplies no assertion coverage. Only retained completed commands supply results. These commands do not establish final gate or current-source success.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-arity168](frozen168/compiler-arity168/receipt.json.gz) · [log](frozen168/compiler-arity168/tests.log) | 1 passed, 0 failed |
| [compiler-body-owner168](frozen168/compiler-body-owner168/receipt.json.gz) · [log](frozen168/compiler-body-owner168/tests.log) | 1 passed, 0 failed |
| [compiler-build168](frozen168/compiler-build168/receipt.json.gz) · [log](frozen168/compiler-build168/tests.log) | Compilation passed; no assertions executed |
| [compiler-channel168](frozen168/compiler-channel168/receipt.json.gz) · [log](frozen168/compiler-channel168/tests.log) | 3 passed, 0 failed |
| [compiler-declared-diagnostics168](frozen168/compiler-declared-diagnostics168/receipt.json.gz) · [log](frozen168/compiler-declared-diagnostics168/tests.log) | 1 passed, 0 failed |
| [compiler-declared-variable168](frozen168/compiler-declared-variable168/receipt.json.gz) · [log](frozen168/compiler-declared-variable168/tests.log) | 1 passed, 0 failed |
| [compiler-declared168](frozen168/compiler-declared168/receipt.json.gz) · [log](frozen168/compiler-declared168/tests.log) | 7 passed, 1 failed |
| [compiler-lambda-arity168](frozen168/compiler-lambda-arity168/receipt.json.gz) · [log](frozen168/compiler-lambda-arity168/tests.log) | 0 passed, 1 failed |
| [compiler-moved168](frozen168/compiler-moved168/receipt.json.gz) · [log](frozen168/compiler-moved168/tests.log) | 1 passed, 0 failed |
| [compiler-package168](frozen168/compiler-package168/receipt.json.gz) · [log](frozen168/compiler-package168/tests.log) | 13 passed, 2 failed |
| [compiler-produced-prefix168](frozen168/compiler-produced-prefix168/receipt.json.gz) · [log](frozen168/compiler-produced-prefix168/tests.log) | 3 passed, 0 failed |
| [compiler-safe168](frozen168/compiler-safe168/receipt.json.gz) · [log](frozen168/compiler-safe168/tests.log) | 27 passed, 5 failed |
| [compiler-source-structure168](frozen168/compiler-source-structure168/receipt.json.gz) · [log](frozen168/compiler-source-structure168/tests.log) | 19 passed, 1 failed |
| [compiler-visibility168](frozen168/compiler-visibility168/receipt.json.gz) · [log](frozen168/compiler-visibility168/tests.log) | 6 passed, 1 failed |
| [core-diagnostic-subject168](frozen168/core-diagnostic-subject168/receipt.json.gz) · [log](frozen168/core-diagnostic-subject168/tests.log) | 8 passed, 1 failed |
| [core-head168](frozen168/core-head168/receipt.json.gz) · [log](frozen168/core-head168/tests.log) | 2 passed, 0 failed |
| [core-if-head168](frozen168/core-if-head168/receipt.json.gz) · [log](frozen168/core-if-head168/tests.log) | 1 passed, 0 failed |
| [core-semantic168](frozen168/core-semantic168/receipt.json.gz) · [log](frozen168/core-semantic168/tests.log) | 22 passed, 3 failed |
| [registry-package168](frozen168/registry-package168/receipt.json.gz) · [log](frozen168/registry-package168/tests.log) | 1 passed, 0 failed |
| [registry-source-roles168](frozen168/registry-source-roles168/receipt.json.gz) · [log](frozen168/registry-source-roles168/tests.log) | 1 passed, 0 failed |
| [registry-structured-roles168](frozen168/registry-structured-roles168/receipt.json.gz) · [log](frozen168/registry-structured-roles168/tests.log) | 1 passed, 1 failed |
| [syntax-formal168](frozen168/syntax-formal168/receipt.json.gz) · [log](frozen168/syntax-formal168/tests.log) | 1 passed, 0 failed |
| [vm-lambda-sdk168](frozen168/vm-lambda-sdk168/receipt.json.gz) · [log](frozen168/vm-lambda-sdk168/tests.log) | 0 passed, 1 failed |
| [workspace-clippy168](frozen168/workspace-clippy168/receipt.json.gz) · [log](frozen168/workspace-clippy168/tests.log) | Clippy failed; no assertions executed |

## Retained source 169 commands

The [Runtime executable pin](frozen169/pinned-runtime169.json) retains the successful Runtime build. The combined Compiler/Core/Registry/Syntax/VM build is blocked and supplies no test image for those packages. Workspace Clippy also fails without assertions. The selected Runtime lambda constructor/getter assertion passes against its independently retained finite SDK rows: 81 APPLY/GETTER comparisons, without BEFORE/AFTER physical fields or header/frame claims. All four commands retain the same source inventory; this Runtime result does not change the separately retained VM168 failure.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build169](frozen169/compiler-build169/receipt.json.gz) · [log](frozen169/compiler-build169/tests.log) | Compilation blocked; no assertions executed |
| [runtime-build169](frozen169/runtime-build169/receipt.json.gz) · [log](frozen169/runtime-build169/tests.log) | Compilation passed; no assertions executed |
| [runtime-lambda169](frozen169/runtime-lambda169/receipt.json.gz) · [log](frozen169/runtime-lambda169/tests.log) | 1 passed, 0 failed |
| [workspace-clippy169](frozen169/workspace-clippy169/receipt.json.gz) · [log](frozen169/workspace-clippy169/tests.log) | Clippy failed; no assertions executed |


## Retained source 170 commands

The combined package build and workspace Clippy are blocked and supply no executable image or named assertion result. Both commands retain their own complete common source inventory. Later edited source and final gate results remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build170](frozen170/compiler-build170/receipt.json.gz) · [log](frozen170/compiler-build170/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy170](frozen170/workspace-clippy170/receipt.json.gz) · [log](frozen170/workspace-clippy170/tests.log) | Clippy failed; no assertions executed |

## Retained source 171 commands

The combined build and workspace Clippy both exit 101 for the same complete frozen source inventory. Neither command executes assertions. Later source changes and independently retained test images remain separate.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build171](frozen171/compiler-build171/receipt.json.gz) · [log](frozen171/compiler-build171/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy171](frozen171/workspace-clippy171/receipt.json.gz) · [log](frozen171/workspace-clippy171/tests.log) | Clippy failed; no assertions executed |


## Retained source 172 commands

The combined build fails, but its Cargo artifact event identifies a rebuilt Compiler test executable (`fresh: false`). The [exact executable pin](frozen172/pinned-compiler172.json) and [partial-artifact qualification](frozen172/partial-compiler-artifact.json) retain that distinction. Twenty-nine selected Compiler assertion commands ran against this image: 134 assertions passed and nine failed; one command matched zero tests. The failed combined build and Clippy command execute no assertions and do not establish an overall gate pass.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-arity172](frozen172/compiler-arity172/receipt.json.gz) · [log](frozen172/compiler-arity172/tests.log) | 1 passed, 0 failed |
| [compiler-body-owner172](frozen172/compiler-body-owner172/receipt.json.gz) · [log](frozen172/compiler-body-owner172/tests.log) | 1 passed, 0 failed |
| [compiler-build172](frozen172/compiler-build172/receipt.json.gz) · [log](frozen172/compiler-build172/tests.log) | Compilation blocked; no assertions executed |
| [compiler-captured-values172](frozen172/compiler-captured-values172/receipt.json.gz) · [log](frozen172/compiler-captured-values172/tests.log) | 1 passed, 0 failed |
| [compiler-channel172](frozen172/compiler-channel172/receipt.json.gz) · [log](frozen172/compiler-channel172/tests.log) | 3 passed, 0 failed |
| [compiler-child-script-roles172](frozen172/compiler-child-script-roles172/receipt.json.gz) · [log](frozen172/compiler-child-script-roles172/tests.log) | 1 passed, 0 failed |
| [compiler-control-advice172](frozen172/compiler-control-advice172/receipt.json.gz) · [log](frozen172/compiler-control-advice172/tests.log) | 6 passed, 0 failed |
| [compiler-control-catch172](frozen172/compiler-control-catch172/receipt.json.gz) · [log](frozen172/compiler-control-catch172/tests.log) | 22 passed, 0 failed |
| [compiler-control-clause172](frozen172/compiler-control-clause172/receipt.json.gz) · [log](frozen172/compiler-control-clause172/tests.log) | 0 passed, 1 failed |
| [compiler-control-event172](frozen172/compiler-control-event172/receipt.json.gz) · [log](frozen172/compiler-control-event172/tests.log) | Zero tests matched; no assertions executed |
| [compiler-declared-diagnostics172](frozen172/compiler-declared-diagnostics172/receipt.json.gz) · [log](frozen172/compiler-declared-diagnostics172/tests.log) | 1 passed, 0 failed |
| [compiler-declared-variable172](frozen172/compiler-declared-variable172/receipt.json.gz) · [log](frozen172/compiler-declared-variable172/tests.log) | 1 passed, 0 failed |
| [compiler-declared172](frozen172/compiler-declared172/receipt.json.gz) · [log](frozen172/compiler-declared172/tests.log) | 8 passed, 0 failed |
| [compiler-existence-advice172](frozen172/compiler-existence-advice172/receipt.json.gz) · [log](frozen172/compiler-existence-advice172/tests.log) | 1 passed, 1 failed |
| [compiler-existence-guards172](frozen172/compiler-existence-guards172/receipt.json.gz) · [log](frozen172/compiler-existence-guards172/tests.log) | 1 passed, 0 failed |
| [compiler-fixed-math172](frozen172/compiler-fixed-math172/receipt.json.gz) · [log](frozen172/compiler-fixed-math172/tests.log) | 0 passed, 1 failed |
| [compiler-formal-headers172](frozen172/compiler-formal-headers172/receipt.json.gz) · [log](frozen172/compiler-formal-headers172/tests.log) | 2 passed, 0 failed |
| [compiler-formal-records172](frozen172/compiler-formal-records172/receipt.json.gz) · [log](frozen172/compiler-formal-records172/tests.log) | 0 passed, 1 failed |
| [compiler-formal-topology172](frozen172/compiler-formal-topology172/receipt.json.gz) · [log](frozen172/compiler-formal-topology172/tests.log) | 2 passed, 0 failed |
| [compiler-lambda-arity172](frozen172/compiler-lambda-arity172/receipt.json.gz) · [log](frozen172/compiler-lambda-arity172/tests.log) | 1 passed, 0 failed |
| [compiler-moved172](frozen172/compiler-moved172/receipt.json.gz) · [log](frozen172/compiler-moved172/tests.log) | 1 passed, 0 failed |
| [compiler-package172](frozen172/compiler-package172/receipt.json.gz) · [log](frozen172/compiler-package172/tests.log) | 14 passed, 1 failed |
| [compiler-possible-trace172](frozen172/compiler-possible-trace172/receipt.json.gz) · [log](frozen172/compiler-possible-trace172/tests.log) | 0 passed, 1 failed |
| [compiler-produced-prefix172](frozen172/compiler-produced-prefix172/receipt.json.gz) · [log](frozen172/compiler-produced-prefix172/tests.log) | 3 passed, 1 failed |
| [compiler-receiver-body172](frozen172/compiler-receiver-body172/receipt.json.gz) · [log](frozen172/compiler-receiver-body172/tests.log) | 2 passed, 0 failed |
| [compiler-safe172](frozen172/compiler-safe172/receipt.json.gz) · [log](frozen172/compiler-safe172/tests.log) | 32 passed, 0 failed |
| [compiler-source-pattern172](frozen172/compiler-source-pattern172/receipt.json.gz) · [log](frozen172/compiler-source-pattern172/tests.log) | 1 passed, 0 failed |
| [compiler-source-shape172](frozen172/compiler-source-shape172/receipt.json.gz) · [log](frozen172/compiler-source-shape172/tests.log) | 0 passed, 1 failed |
| [compiler-source-structure172](frozen172/compiler-source-structure172/receipt.json.gz) · [log](frozen172/compiler-source-structure172/tests.log) | 21 passed, 1 failed |
| [compiler-visibility172](frozen172/compiler-visibility172/receipt.json.gz) · [log](frozen172/compiler-visibility172/tests.log) | 8 passed, 0 failed |
| [workspace-clippy172](frozen172/workspace-clippy172/receipt.json.gz) · [log](frozen172/workspace-clippy172/tests.log) | Clippy failed; no assertions executed |


## Retained source 173 commands

The combined build and workspace Clippy both exit 101 for the same complete frozen source inventory. Neither command executes assertions. Later source changes and independently retained test images remain separate.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build173](frozen173/compiler-build173/receipt.json.gz) · [log](frozen173/compiler-build173/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy173](frozen173/workspace-clippy173/receipt.json.gz) · [log](frozen173/workspace-clippy173/tests.log) | Clippy failed; no assertions executed |

## Retained source 174 commands

The combined build fails, but its Cargo artifact event identifies a rebuilt Compiler test executable (`fresh: false`). The [exact executable pin](frozen174/pinned-compiler174.json) and [partial-artifact qualification](frozen174/partial-compiler-artifact.json) retain that distinction. Twelve selected Compiler assertion commands ran against this image: 34 assertions passed and three failed. The failed combined build and Clippy command execute no assertions and do not establish an overall gate pass.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-append-advice174](frozen174/compiler-append-advice174/receipt.json.gz) · [log](frozen174/compiler-append-advice174/tests.log) | 1 passed, 0 failed |
| [compiler-build174](frozen174/compiler-build174/receipt.json.gz) · [log](frozen174/compiler-build174/tests.log) | Compilation blocked; no assertions executed |
| [compiler-case-advice174](frozen174/compiler-case-advice174/receipt.json.gz) · [log](frozen174/compiler-case-advice174/tests.log) | 1 passed, 0 failed |
| [compiler-control-clause174](frozen174/compiler-control-clause174/receipt.json.gz) · [log](frozen174/compiler-control-clause174/tests.log) | 1 passed, 0 failed |
| [compiler-existence-trace174](frozen174/compiler-existence-trace174/receipt.json.gz) · [log](frozen174/compiler-existence-trace174/tests.log) | 0 passed, 1 failed |
| [compiler-fixed-math174](frozen174/compiler-fixed-math174/receipt.json.gz) · [log](frozen174/compiler-fixed-math174/tests.log) | 0 passed, 1 failed |
| [compiler-formal-records174](frozen174/compiler-formal-records174/receipt.json.gz) · [log](frozen174/compiler-formal-records174/tests.log) | 1 passed, 0 failed |
| [compiler-next-source-owner174](frozen174/compiler-next-source-owner174/receipt.json.gz) · [log](frozen174/compiler-next-source-owner174/tests.log) | 1 passed, 0 failed |
| [compiler-option-advice174](frozen174/compiler-option-advice174/receipt.json.gz) · [log](frozen174/compiler-option-advice174/tests.log) | 1 passed, 0 failed |
| [compiler-produced-prefix174](frozen174/compiler-produced-prefix174/receipt.json.gz) · [log](frozen174/compiler-produced-prefix174/tests.log) | 4 passed, 0 failed |
| [compiler-shape-boundaries174](frozen174/compiler-shape-boundaries174/receipt.json.gz) · [log](frozen174/compiler-shape-boundaries174/tests.log) | 1 passed, 0 failed |
| [compiler-source-structure174](frozen174/compiler-source-structure174/receipt.json.gz) · [log](frozen174/compiler-source-structure174/tests.log) | 22 passed, 1 failed |
| [compiler-zeroarg-memo174](frozen174/compiler-zeroarg-memo174/receipt.json.gz) · [log](frozen174/compiler-zeroarg-memo174/tests.log) | 1 passed, 0 failed |
| [workspace-clippy174](frozen174/workspace-clippy174/receipt.json.gz) · [log](frozen174/workspace-clippy174/tests.log) | Clippy failed; no assertions executed |

## Retained source 175 commands

The combined build and workspace Clippy both exit 101 for the same complete frozen source inventory. Neither command executes assertions. Later source changes and independently retained test images remain separate.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build175](frozen175/compiler-build175/receipt.json.gz) · [log](frozen175/compiler-build175/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy175](frozen175/workspace-clippy175/receipt.json.gz) · [log](frozen175/workspace-clippy175/tests.log) | Clippy failed; no assertions executed |


## Retained source 176 commands

The combined build and workspace Clippy both exit 101 for the same complete frozen source inventory. Neither command executes assertions. Later source changes and independently retained test images remain separate.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build176](frozen176/compiler-build176/receipt.json.gz) · [log](frozen176/compiler-build176/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy176](frozen176/workspace-clippy176/receipt.json.gz) · [log](frozen176/workspace-clippy176/tests.log) | Clippy failed; no assertions executed |

## Restoring inactive external Rust replay inputs

The [inactive receipt inventory](replay-storage/inactive-run-receipts-20261010.json)
retains 1,796 external Rust receipt JSON paths and their lossless gzip archives.
The [inactive executable inventory](replay-storage/inactive-rust-pins-20261009-third.json)
retains six external Rust test-image paths and archives. Each original and
archive has its own SHA-256 and byte size; the retained archival receipt records
a verified decompression round trip before removing the inactive original.
These storage operations change no native provider evidence, canonical
repository receipt, raw log or validation result.

Repository `receipt.json.gz` files already retain their original bytes and can
be read directly. A replay tool that follows an external absolute JSON or ELF
path must first restore that exact path from the matching inventory entry.
Verify the archive digest, decompress to the original path, then verify the
restored digest and byte size; a restored executable also needs its recorded
executable permissions. The inventory restore command uses `ARCHIVE` and
`ORIGINAL` as placeholders for the exact entry paths. Restore only the required
inputs. Missing or mismatched bytes keep replay unavailable; current working
source or another executable cannot replace the pin. Restoration supplies no
new assertion, native observation or passing-gate claim.

## Retained source 178 commands

The combined build and workspace Clippy both exit 101 for the same complete frozen source inventory. Neither command executes assertions. Later source changes and independently retained test images remain separate.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build178](frozen178/compiler-build178/receipt.json.gz) · [log](frozen178/compiler-build178/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy178](frozen178/workspace-clippy178/receipt.json.gz) · [log](frozen178/workspace-clippy178/tests.log) | Clippy failed; no assertions executed |

## Retained source 179 commands

The combined build fails, but its Cargo artifact event identifies a rebuilt Compiler test executable (`fresh: false`). The [exact executable pin](frozen179/pinned-compiler179.json) and [partial-artifact qualification](frozen179/partial-compiler-artifact.json) retain that distinction. Seventeen selected Compiler filters ran against this image: 48 assertions passed and six failed across five commands; one filter matched no tests. Separately, the proof-gate Python unittest command passed 14 tests. Its log contains no Rust libtest summary. The failed combined build and Clippy command execute no assertions and do not establish an overall gate pass.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-arity179](frozen179/compiler-arity179/receipt.json.gz) · [log](frozen179/compiler-arity179/tests.log) | 1 passed, 0 failed |
| [compiler-baseline179](frozen179/compiler-baseline179/receipt.json.gz) · [log](frozen179/compiler-baseline179/tests.log) | 1 passed, 1 failed |
| [compiler-build179](frozen179/compiler-build179/receipt.json.gz) · [log](frozen179/compiler-build179/tests.log) | Compilation blocked; no assertions executed |
| [compiler-callback-prefix179](frozen179/compiler-callback-prefix179/receipt.json.gz) · [log](frozen179/compiler-callback-prefix179/tests.log) | 1 passed, 0 failed |
| [compiler-dataflow-purpose179](frozen179/compiler-dataflow-purpose179/receipt.json.gz) · [log](frozen179/compiler-dataflow-purpose179/tests.log) | 1 passed, 0 failed |
| [compiler-existence179](frozen179/compiler-existence179/receipt.json.gz) · [log](frozen179/compiler-existence179/tests.log) | 2 passed, 0 failed |
| [compiler-fixed-math179](frozen179/compiler-fixed-math179/receipt.json.gz) · [log](frozen179/compiler-fixed-math179/tests.log) | 1 passed, 0 failed |
| [compiler-formal-diags179](frozen179/compiler-formal-diags179/receipt.json.gz) · [log](frozen179/compiler-formal-diags179/tests.log) | 2 passed, 0 failed |
| [compiler-formal-header179](frozen179/compiler-formal-header179/receipt.json.gz) · [log](frozen179/compiler-formal-header179/tests.log) | 1 passed, 0 failed |
| [compiler-future-procedure179](frozen179/compiler-future-procedure179/receipt.json.gz) · [log](frozen179/compiler-future-procedure179/tests.log) | 3 passed, 1 failed |
| [compiler-inlining179](frozen179/compiler-inlining179/receipt.json.gz) · [log](frozen179/compiler-inlining179/tests.log) | 0 passed, 1 failed |
| [compiler-lambda-arity179](frozen179/compiler-lambda-arity179/receipt.json.gz) · [log](frozen179/compiler-lambda-arity179/tests.log) | 1 passed, 0 failed |
| [compiler-metadata-context179](frozen179/compiler-metadata-context179/receipt.json.gz) · [log](frozen179/compiler-metadata-context179/tests.log) | 1 passed, 0 failed |
| [compiler-nested179](frozen179/compiler-nested179/receipt.json.gz) · [log](frozen179/compiler-nested179/tests.log) | 3 passed, 1 failed |
| [compiler-replay-formals179](frozen179/compiler-replay-formals179/receipt.json.gz) · [log](frozen179/compiler-replay-formals179/tests.log) | 1 passed, 0 failed |
| [compiler-script-purpose179](frozen179/compiler-script-purpose179/receipt.json.gz) · [log](frozen179/compiler-script-purpose179/tests.log) | Zero tests matched; no assertions executed |
| [compiler-source-class179](frozen179/compiler-source-class179/receipt.json.gz) · [log](frozen179/compiler-source-class179/tests.log) | 6 passed, 2 failed |
| [compiler-source-structure179](frozen179/compiler-source-structure179/receipt.json.gz) · [log](frozen179/compiler-source-structure179/tests.log) | 23 passed, 0 failed |
| [proof-gate-unit179](frozen179/proof-gate-unit179/receipt.json.gz) · [log](frozen179/proof-gate-unit179/tests.log) | 14 passed, 0 failed |
| [workspace-clippy179](frozen179/workspace-clippy179/receipt.json.gz) · [log](frozen179/workspace-clippy179/tests.log) | Clippy failed; no assertions executed |

## Retained source 180 commands

The combined build and workspace Clippy both exit 101 for the same complete frozen source inventory. Neither command executes assertions. Later source changes and independently retained test images remain separate.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build180](frozen180/compiler-build180/receipt.json.gz) · [log](frozen180/compiler-build180/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy180](frozen180/workspace-clippy180/receipt.json.gz) · [log](frozen180/workspace-clippy180/tests.log) | Clippy failed; no assertions executed |


## Retained source 181 commands

The combined build and workspace Clippy both exit 101 for the same complete frozen source inventory. Neither command executes assertions. Later source changes and independently retained test images remain separate.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build181](frozen181/compiler-build181/receipt.json.gz) · [log](frozen181/compiler-build181/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy181](frozen181/workspace-clippy181/receipt.json.gz) · [log](frozen181/workspace-clippy181/tests.log) | Clippy failed; no assertions executed |

## Retained source 182 commands

The combined build and workspace Clippy both exit 101 for the same complete frozen source inventory. Neither command executes assertions or supplies a Compiler executable pin.

Both operations report four E0004 non-exhaustive diagnostic-accessor matches for `DiagnosticSubject::CallbackSourceArity` in `analyser/types.rs`. The build reports failed library and library-test compilation; these are repeated reports of the same four accessor errors.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build182](frozen182/compiler-build182/receipt.json.gz) · [log](frozen182/compiler-build182/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy182](frozen182/workspace-clippy182/receipt.json.gz) · [log](frozen182/workspace-clippy182/tests.log) | Clippy failed; no assertions executed |


## Retained source 183 commands

The combined build and workspace Clippy both exit 101 for the same complete frozen source inventory. Neither command executes assertions or supplies a Compiler executable pin.

Both operations report the sole E0603 error at `command_binding/source_declared_command.rs:221`: `registry_invocation::source_scoped_body` is private and `declared_source_script_bodies_for` is not publicly re-exported. Library and library-test compilation repeat this same error. The build took 75.12 seconds; Clippy took 80.00 seconds.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build183](frozen183/compiler-build183/receipt.json.gz) · [log](frozen183/compiler-build183/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy183](frozen183/workspace-clippy183/receipt.json.gz) · [log](frozen183/workspace-clippy183/tests.log) | Clippy failed; no assertions executed |

## Retained source 184 commands

The combined build and workspace Clippy both exit 101 for the same complete frozen source inventory. Neither command executes assertions or supplies an executable pin. The build took 103.60 seconds and reports five missing-documentation errors for callback public fields plus two dead-code warnings (`source_less` and `instance_method_command_prefix_invocations`). Clippy took 107.92 seconds and reports 18 errors: the same five field-documentation errors and thirteen production style/dead-code errors. Library and library-test compilation repeat these diagnostics. Exact complete logs preserve the individual sites and codes; no overall build or lint pass follows.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build184](frozen184/compiler-build184/receipt.json.gz) · [log](frozen184/compiler-build184/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy184](frozen184/workspace-clippy184/receipt.json.gz) · [log](frozen184/workspace-clippy184/tests.log) | Clippy failed; no assertions executed |

## Retained source 185 commands

The combined build and workspace Clippy both exit 101 for the same complete frozen source inventory. Neither command executes assertions or supplies a rebuilt Compiler executable pin. The build took 182.15 seconds and reports two distinct errors: E0599 for the absent `InvocationArguments::of` at `analyser/diagnostic_subject/callback.rs:110` and E0425 for the absent `looks_unresolvable` at `signature_scan/command_prefix.rs:59`. Library and library-test compilation repeat these diagnostics. Its Syntax and Lexer executable events are `fresh: true` cached artifacts, not rebuilt Compiler images. Clippy took 40.92 seconds and reports one `too_many_lines` error (102/100) in Registry `source_member_indices_where` at `definer.rs:2618`. Exact complete logs preserve these sites and codes; no overall build or lint pass follows.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build185](frozen185/compiler-build185/receipt.json.gz) · [log](frozen185/compiler-build185/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy185](frozen185/workspace-clippy185/receipt.json.gz) · [log](frozen185/workspace-clippy185/tests.log) | Clippy failed; no assertions executed |

## Retained source 186 commands

The combined Compiler build exits 101 after 136.91 seconds for the `DefinitionBodyGrammar` lifetime at `analyser/oo.rs:2495` (the borrowed input is declared at line 2458). Library and library-test compilation repeat the error. Workspace Clippy exits 101 after 117.07 seconds for that lifetime, the redundant `class_def` field, and two `similar_names` errors in `source_scoped_body.rs`. Neither command executes assertions or supplies a Compiler executable pin.

The independent Runtime build exits 0 after 81.39 seconds and reports a rebuilt test artifact (`fresh: false`). The [exact Runtime executable pin](frozen186/pinned-runtime186.json) retains the binary, build-receipt and source-snapshot digests. Its list command reports 1,231 tests without executing assertions. The selected 16-filter union runs 16 assertions: 15 pass and one fails, with 1,215 filtered out. These five commands all retain the same complete source inventory and `uniform_source: true`.

The failure is `cmd_array::tests::variable_container_storage_matches_all_native_columns`, Jim case 8: the retained `dict update` source assigns `ok` before encountering the array-valued `blocked` destination. The assertion compares Runtime `1 BEFORE 0 0 {k OLD}` with the older native fixture's `0 NEW 1 0 OTHER`; both byte arrays and the full source appear in the log. The failed union does not establish Runtime parity or a whole-suite pass. Independently passing assertions, including the selected SDK comparisons, remain exact assertion results for this image. New native specimens using another evaluation channel remain independent evidence.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build186](frozen186/compiler-build186/receipt.json.gz) · [log](frozen186/compiler-build186/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy186](frozen186/workspace-clippy186/receipt.json.gz) · [log](frozen186/workspace-clippy186/tests.log) | Clippy failed; no assertions executed |
| [runtime-build186](frozen186/runtime-build186/receipt.json.gz) · [log](frozen186/runtime-build186/tests.log) | Runtime compilation passed; no assertions executed |
| [runtime-list186](frozen186/runtime-list186/receipt.json.gz) · [log](frozen186/runtime-list186/tests.log) | 1,231 tests listed; no assertions executed |
| [consumer-runtime186](frozen186/consumer-runtime186/receipt.json.gz) · [log](frozen186/consumer-runtime186/tests.log) | 15 passed, 1 failed |

## Retained source 187 commands

The combined seven-crate build and workspace Clippy both exit 101 for the same complete frozen source inventory. Neither command executes assertions or supplies an executable pin. The build took 260.54 seconds and reports E0282 for an untyped retained definition-parent closure in Core `source_structure.rs:151`, plus an unused `DefinerFamily` import warning in `semantic_tokens.rs:98`. Clippy took 218.61 seconds and reports three Compiler style errors: a redundant callback `captured_arguments` closure, manual `is_multiple_of` implementation in `oo.rs`, and a definition-parent argument passed by value without consumption in `analyser/utils.rs`. Library and library-test compilation repeat the style failure. These are distinct command results, so the Clippy log is not used to claim it reached the Core build error. Exact complete logs retain every diagnostic site/code and source inventory; no overall build or lint pass follows.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build187](frozen187/compiler-build187/receipt.json.gz) · [log](frozen187/compiler-build187/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy187](frozen187/workspace-clippy187/receipt.json.gz) · [log](frozen187/workspace-clippy187/tests.log) | Clippy failed; no assertions executed |

## Retained source 188 commands

The combined seven-crate library/test build exits 101 after 207.52 seconds. Three Registry test import errors are retained: `SourceFormalValidationApplicability` at `definer.rs:4622`, and `SNIT_GRAMMAR` at lines 4650 and 4652. It executes no assertions and supplies no Compiler executable pin.

The separate Runtime build exits zero after 30.51 seconds and reports a rebuilt test artifact (`fresh: false`). Its [exact executable pin](frozen188/pinned-runtime188.json) retains binary, build-receipt and source-snapshot digests. The two-filter dictionary debug command exits 101 after 13.00 seconds: zero pass, two fail, 1,230 are filtered out. `dictionary_update_first_original_mapping_reaches_caller_storage` retains a caught missing `dict update` result with `BEFORE` and body-zero. `variable_container_storage_matches_all_native_columns` retains Jim case 8 `1 BEFORE 0 0 {k OLD}` against the unchanged native Jim `0 NEW 1 0 OTHER` column. Complete source and byte arrays remain in the log. These three operations share the complete exact source inventory and `uniform_source: true`.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build188](frozen188/compiler-build188/receipt.json.gz) · [log](frozen188/compiler-build188/tests.log) | Compilation blocked; no assertions executed |
| [runtime-build188](frozen188/runtime-build188/receipt.json.gz) · [log](frozen188/runtime-build188/tests.log) | Runtime compilation passed; no assertions executed |
| [runtime-dictionary-debug188](frozen188/runtime-dictionary-debug188/receipt.json.gz) · [log](frozen188/runtime-dictionary-debug188/tests.log) | 0 passed, 2 failed (rust-libtest) |


## Retained source 189 commands

The combined seven-crate library/test build exits 101 after 291.01 seconds for two `MemberSide` test references at `source_class_reference.rs:551` and line 602. Workspace Clippy exits 101 after 262.50 seconds with those two errors and three iRules style errors: explicit lifetimes in `source_context.rs:7`, the 167-line `from_analysis` function, and the 108-line `original_source_object_operands` function. Neither command executes assertions or supplies a Compiler executable pin.

The separate Runtime build exits zero after 185.01 seconds and reports a rebuilt test artifact (`fresh: false`). Its [exact executable pin](frozen189/pinned-runtime189.json) retains binary, build-receipt and source-snapshot digests. The list operation enumerates 1,233 tests without executing assertions. The selected 18-filter Runtime union exits zero after 27.45 seconds: 18 pass, none fail, and 1,215 are filtered out. Both original dictionary caller-storage comparisons and the explicit core-versus-distribution control pass in this exact image; the SDK lambda comparison is one separately named assertion in this union. No complete Runtime or workspace-suite result follows. The proof-gate Python command exits zero after 1.66 seconds and reports 14 unit tests passing. All six operations retain the same complete source inventory and `uniform_source: true`; the source188 inventory is separately retained.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build189](frozen189/compiler-build189/receipt.json.gz) · [log](frozen189/compiler-build189/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy189](frozen189/workspace-clippy189/receipt.json.gz) · [log](frozen189/workspace-clippy189/tests.log) | Clippy failed; no assertions executed |
| [runtime-build189](frozen189/runtime-build189/receipt.json.gz) · [log](frozen189/runtime-build189/tests.log) | Runtime compilation passed; no assertions executed |
| [runtime-list189](frozen189/runtime-list189/receipt.json.gz) · [log](frozen189/runtime-list189/tests.log) | 1,233 tests listed; no assertions executed |
| [consumer-runtime189](frozen189/consumer-runtime189/receipt.json.gz) · [log](frozen189/consumer-runtime189/tests.log) | 18 passed, 0 failed (rust-libtest) |
| [proof-python-tests189](frozen189/proof-python-tests189/receipt.json.gz) · [log](frozen189/proof-python-tests189/tests.log) | 14 passed, 0 failed (python-unittest) |

## Retained source 191 commands

The combined nine-crate library/test build exits 101 after 227.87 seconds. The Core `source_style.rs:916` test wraps a cloned `&ContextRegistry` in an `Arc` where an owned `ContextRegistry` is required; the retained diagnostic is E0308. It executes no assertions and supplies no Compiler executable pin.

Workspace Clippy exits 101 after 80.35 seconds. Six distinct production style sites are retained: the 101-line VM `array_op_after_trace`, the 101-line Compiler `original_configuration_operation` (repeated in library/library-test compilation), three missing statement semicolons in `compiled_preflight.rs` and `optimiser/tail_call/inventory.rs`, and the 107-line `emit_loop_conversion`. It executes no assertions.

The separate Runtime build exits zero after 81.61 seconds and reports a rebuilt test artifact (`fresh: false`). Its [exact executable pin](frozen191/pinned-runtime191.json) retains binary, build-receipt and source-snapshot digests. The list operation enumerates 1,234 tests without executing assertions. The selected 19-filter Runtime union exits zero after 17.14 seconds: 19 pass, none fail, and 1,215 are filtered out. The independently supported Jim alias installation API caller-projection control is one assertion; full original VM source execution remains a separate test and is not run by this union. Dictionary caller-storage and explicit core-versus-distribution controls also pass in this exact image. No complete Runtime or workspace-suite pass follows. These five operations retain the same complete source inventory and `uniform_source: true`. No Python command is included.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build191](frozen191/compiler-build191/receipt.json.gz) · [log](frozen191/compiler-build191/tests.log) | Compilation blocked; no assertions executed |
| [workspace-clippy191](frozen191/workspace-clippy191/receipt.json.gz) · [log](frozen191/workspace-clippy191/tests.log) | Clippy failed; no assertions executed |
| [runtime-build191](frozen191/runtime-build191/receipt.json.gz) · [log](frozen191/runtime-build191/tests.log) | Runtime compilation passed; no assertions executed |
| [runtime-list191](frozen191/runtime-list191/receipt.json.gz) · [log](frozen191/runtime-list191/tests.log) | 1,234 tests listed; no assertions executed |
| [consumer-runtime191](frozen191/consumer-runtime191/receipt.json.gz) · [log](frozen191/consumer-runtime191/tests.log) | 19 passed, 0 failed (rust-libtest) |

## Retained source 192 commands

The combined nine-crate library/test build exits zero after 308.59 seconds and reports nine rebuilt executable artifacts (`fresh: false`). Each exact binary/build-receipt/source-snapshot association has a separate retained pin. The independent Runtime build exits zero after 0.24 seconds with `fresh: true`: it reuses the exact f49e2217… Runtime image rather than reporting a rebuilt artifact. These compile/list results execute no assertions and predate the current callback, declaration-abort and alias integration changes.

Workspace Clippy exits 101 after 84.94 seconds. Its two retained errors are function length in `rust/tcl-diagram/src/data.rs`; the whole diagnostic stream remains in its log. The nine list operations independently enumerate their pinned library tests.

Completed selected consumer summaries remain independent per crate. The selected VM process exits on signal 6 with 32 explicit pass events, five explicit failure events and one incomplete event. The complete Runtime invocation also exits on signal 6: its retained stream contains 951 explicit pass events, 44 explicit failure events and one incomplete observer event. Neither process has a final framework summary; no full Runtime, VM or workspace pass follows. The isolated observer invocation reproduces a signal-6 termination with its only assertion incomplete. Main Compiler and Core consumer processes are independently terminated on signal15 after 1,478.13/1,478.11 seconds at their incomplete deep controls; neither has a final framework summary. Two bounded existing-instrumentation Compiler trace commands time out with exit124 after 10.04/5.04 seconds; each records its only assertion as incomplete. Independent bounded detail commands retain final failures separately: Runtime24, Compiler4, Core11, receiver3, diagnostic7 and bootstrap action1. These repeated controls do not supply a completed main-suite tally.

All 31 operations retain one exact complete frozen source inventory and `uniform_source: true`. Current source owners, native observations and the independently retained validation results remain separate. No Python operation is included.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build192](frozen192/compiler-build192/receipt.json.gz) · [log](frozen192/compiler-build192/tests.log) | Compilation passed; no assertions executed |
| [workspace-clippy192](frozen192/workspace-clippy192/receipt.json.gz) · [log](frozen192/workspace-clippy192/tests.log) | Clippy failed; no assertions executed |
| [runtime-build192](frozen192/runtime-build192/receipt.json.gz) · [log](frozen192/runtime-build192/tests.log) | Compilation passed; no assertions executed |
| [compiler-list192](frozen192/compiler-list192/receipt.json.gz) · [log](frozen192/compiler-list192/tests.log) | 8374 tests listed; no assertions executed |
| [core-list192](frozen192/core-list192/receipt.json.gz) · [log](frozen192/core-list192/tests.log) | 2706 tests listed; no assertions executed |
| [db-list192](frozen192/db-list192/receipt.json.gz) · [log](frozen192/db-list192/tests.log) | 107 tests listed; no assertions executed |
| [registry-list192](frozen192/registry-list192/receipt.json.gz) · [log](frozen192/registry-list192/tests.log) | 1488 tests listed; no assertions executed |
| [syntax-list192](frozen192/syntax-list192/receipt.json.gz) · [log](frozen192/syntax-list192/tests.log) | 603 tests listed; no assertions executed |
| [vm-list192](frozen192/vm-list192/receipt.json.gz) · [log](frozen192/vm-list192/tests.log) | 673 tests listed; no assertions executed |
| [lexer-list192](frozen192/lexer-list192/receipt.json.gz) · [log](frozen192/lexer-list192/tests.log) | 530 tests listed; no assertions executed |
| [irules-list192](frozen192/irules-list192/receipt.json.gz) · [log](frozen192/irules-list192/tests.log) | 67 tests listed; no assertions executed |
| [server-list192](frozen192/server-list192/receipt.json.gz) · [log](frozen192/server-list192/tests.log) | 652 tests listed; no assertions executed |
| [consumer-compiler192](frozen192/consumer-compiler192/receipt.json.gz) · [log](frozen192/consumer-compiler192/tests.log) | Signal 15; no final framework summary. 141 explicit pass, 38 explicit fail, 1 incomplete events. |
| [consumer-core192](frozen192/consumer-core192/receipt.json.gz) · [log](frozen192/consumer-core192/tests.log) | Signal 15; no final framework summary. 154 explicit pass, 17 explicit fail, 1 incomplete events. |
| [consumer-db192](frozen192/consumer-db192/receipt.json.gz) · [log](frozen192/consumer-db192/tests.log) | 23 passed, 7 failed, 77 filtered out |
| [consumer-registry192](frozen192/consumer-registry192/receipt.json.gz) · [log](frozen192/consumer-registry192/tests.log) | 31 passed, 1 failed, 1456 filtered out |
| [consumer-syntax192](frozen192/consumer-syntax192/receipt.json.gz) · [log](frozen192/consumer-syntax192/tests.log) | 2 passed, 0 failed, 601 filtered out |
| [consumer-vm192](frozen192/consumer-vm192/receipt.json.gz) · [log](frozen192/consumer-vm192/tests.log) | Signal 6; no final framework summary. 32 explicit pass, 5 explicit fail, 1 incomplete events. |
| [consumer-lexer192](frozen192/consumer-lexer192/receipt.json.gz) · [log](frozen192/consumer-lexer192/tests.log) | 7 passed, 0 failed, 523 filtered out |
| [consumer-irules192](frozen192/consumer-irules192/receipt.json.gz) · [log](frozen192/consumer-irules192/tests.log) | 3 passed, 1 failed, 63 filtered out |
| [consumer-server192](frozen192/consumer-server192/receipt.json.gz) · [log](frozen192/consumer-server192/tests.log) | 24 passed, 1 failed, 627 filtered out |
| [runtime-full192](frozen192/runtime-full192/receipt.json.gz) · [log](frozen192/runtime-full192/tests.log) | Signal 6; no final framework summary. 951 explicit pass, 44 explicit fail, 1 incomplete events. |
| [runtime-failure-details192](frozen192/runtime-failure-details192/receipt.json.gz) · [log](frozen192/runtime-failure-details192/tests.log) | 0 passed, 24 failed, 1210 filtered out |
| [runtime-observer-isolated192](frozen192/runtime-observer-isolated192/receipt.json.gz) · [log](frozen192/runtime-observer-isolated192/tests.log) | Signal 6; no final framework summary. 0 explicit pass, 0 explicit fail, 1 incomplete events. |
| [compiler-callback-details192](frozen192/compiler-callback-details192/receipt.json.gz) · [log](frozen192/compiler-callback-details192/tests.log) | 0 passed, 4 failed, 8370 filtered out |
| [core-failure-details192](frozen192/core-failure-details192/receipt.json.gz) · [log](frozen192/core-failure-details192/tests.log) | 0 passed, 11 failed, 2695 filtered out |
| [receiver-failure-details192](frozen192/receiver-failure-details192/receipt.json.gz) · [log](frozen192/receiver-failure-details192/tests.log) | 0 passed, 3 failed, 2703 filtered out |
| [diagnostic-failure-details192](frozen192/diagnostic-failure-details192/receipt.json.gz) · [log](frozen192/diagnostic-failure-details192/tests.log) | 0 passed, 7 failed, 8367 filtered out |
| [body-performance-trace192](frozen192/body-performance-trace192/receipt.json.gz) · [log](frozen192/body-performance-trace192/tests.log) | Timeout exit124; no final framework summary. 0 explicit pass, 0 explicit fail, 1 incomplete events. |
| [body-original-selection-trace192](frozen192/body-original-selection-trace192/receipt.json.gz) · [log](frozen192/body-original-selection-trace192/tests.log) | Timeout exit124; no final framework summary. 0 explicit pass, 0 explicit fail, 1 incomplete events. |
| [bootstrap-action-details192](frozen192/bootstrap-action-details192/receipt.json.gz) · [log](frozen192/bootstrap-action-details192/tests.log) | 0 passed, 1 failed, 2705 filtered out |

| Exact executable pin | Artifact reported fresh |
| --- | --- |
| [compiler](frozen192/pinned-compiler192.json) | `false` |
| [core](frozen192/pinned-core192.json) | `false` |
| [db](frozen192/pinned-db192.json) | `false` |
| [registry](frozen192/pinned-registry192.json) | `false` |
| [syntax](frozen192/pinned-syntax192.json) | `false` |
| [vm](frozen192/pinned-vm192.json) | `false` |
| [lexer](frozen192/pinned-lexer192.json) | `false` |
| [irules](frozen192/pinned-irules192.json) | `false` |
| [server](frozen192/pinned-server192.json) | `false` |
| [runtime](frozen192/pinned-runtime192.json) | `true` |

## Retained source 193 commands

The requested combined nine-crate library/test build exits 101 after 8.59 seconds, and the separate Runtime library/test build exits 101 after 3.25 seconds. Both retain the same E0425 in `tcl-syntax/src/naming/native.rs:1276`: `slot_for_projection` is unavailable in that scope. Neither executes assertions or completes its requested build.

Both operations in this source group retain the same complete immutable inventory and `uniform_source: true`. The full JSON logs retain any partial or cached artifact paths reported before failure; this ledger attaches no new verified executable pin. No Clippy, listing, Python command, native provider result or whole-suite pass is included.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build193](frozen193/compiler-build193/receipt.json.gz) · [log](frozen193/compiler-build193/tests.log) | Compilation blocked; no assertions executed |
| [runtime-build193](frozen193/runtime-build193/receipt.json.gz) · [log](frozen193/runtime-build193/tests.log) | Compilation blocked; no assertions executed |


## Retained source 194 commands

The requested combined nine-crate build exits 101 after 126.89 seconds. Its Compiler diagnostics retain two E0382 source sites for moved `interpreter` use in `source_callback.rs:487` and `:530`, repeated in library/library-test compilation, plus the unused `std::hash::Hash` import warning. The separate Runtime build exits 101 after 57.69 seconds with E0308 in `namespace.rs:1342`. Neither executes assertions or completes its requested build.

Both operations in this source group retain the same complete immutable inventory and `uniform_source: true`. The full JSON logs retain any partial or cached artifact paths reported before failure; this ledger attaches no new verified executable pin. No Clippy, listing, Python command, native provider result or whole-suite pass is included.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build194](frozen194/compiler-build194/receipt.json.gz) · [log](frozen194/compiler-build194/tests.log) | Compilation blocked; no assertions executed |
| [runtime-build194](frozen194/runtime-build194/receipt.json.gz) · [log](frozen194/runtime-build194/tests.log) | Compilation blocked; no assertions executed |

## Retained source 195 commands

The requested combined nine-crate library/test build exits 101 after 181.91 seconds with Core E0004: the diagnostic-subject projection lacks the `UnavailableCallbackLookupFrame` case. It executes no assertions and does not complete the requested build. Independent Runtime, Compiler and VM builds exit 0 after 52.60, 260.06 and 355.04 seconds respectively. Their independently verified executable/build-receipt/source-snapshot pins correspond to rebuilt artifacts (`fresh: false`). Any reported Compiler artifact in the blocked combined log remains distinct from the independent successful Compiler build/pin.

The full Runtime invocation exits on signal 6 after 270.87 seconds at `interp::tests::deeply_nested_foreach_errors_instead_of_crashing`. Its complete stream retains 988 explicit pass events, 38 explicit failures and one incomplete event, with no final framework summary. The independent Runtime detail command retains 38 failures. Compiler selected controls report 32 passed/eight failed; VM selected details report four passed/five failed; VM retirement controls report three passed/one failed. The separately instrumented W123 transfer trace reports one failure. These repeated explicit events supply neither a unique assertion tally nor a whole Runtime, VM or workspace result.

All ten commands retain one exact complete immutable inventory and `uniform_source: true`. Current source contracts, native observations and these process outcomes remain independent. No Clippy, listing or Python operation is included.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build195](frozen195/compiler-build195/receipt.json.gz) · [log](frozen195/compiler-build195/tests.log) | Core E0004 blocks requested combined build; no assertions executed |
| [runtime-build195](frozen195/runtime-build195/receipt.json.gz) · [log](frozen195/runtime-build195/tests.log) | Compilation passed; no assertions executed |
| [compiler-independent-build195](frozen195/compiler-independent-build195/receipt.json.gz) · [log](frozen195/compiler-independent-build195/tests.log) | Compilation passed; no assertions executed |
| [vm-build195](frozen195/vm-build195/receipt.json.gz) · [log](frozen195/vm-build195/tests.log) | Compilation passed; no assertions executed |
| [runtime-full195](frozen195/runtime-full195/receipt.json.gz) · [log](frozen195/runtime-full195/tests.log) | Signal 6; no final framework summary. 988 explicit pass, 38 explicit fail, 1 incomplete events. |
| [runtime-failure-details195](frozen195/runtime-failure-details195/receipt.json.gz) · [log](frozen195/runtime-failure-details195/tests.log) | 0 passed, 38 failed, 1199 filtered out |
| [compiler-finite-controls195](frozen195/compiler-finite-controls195/receipt.json.gz) · [log](frozen195/compiler-finite-controls195/tests.log) | 32 passed, 8 failed, 8350 filtered out |
| [vm-finite-details195](frozen195/vm-finite-details195/receipt.json.gz) · [log](frozen195/vm-finite-details195/tests.log) | 4 passed, 5 failed, 667 filtered out |
| [vm-retirement-controls195](frozen195/vm-retirement-controls195/receipt.json.gz) · [log](frozen195/vm-retirement-controls195/tests.log) | 3 passed, 1 failed, 672 filtered out |
| [w123-transfer-trace195](frozen195/w123-transfer-trace195/receipt.json.gz) · [log](frozen195/w123-transfer-trace195/tests.log) | 0 passed, 1 failed, 8389 filtered out |

| Exact executable pin | Artifact reported fresh |
| --- | --- |
| [runtime](frozen195/pinned-runtime195.json) | `false` |
| [compiler](frozen195/pinned-compiler195.json) | `false` |
| [vm](frozen195/pinned-vm195.json) | `false` |

## Retained source 196 commands

The requested combined command using package tcl-db exits 101 after 0.74 seconds: Cargo reports that the package ID matches no package. No compiler assertion or requested build executes under that command. The separate Runtime library/test build exits 101 after 51.14 seconds with five import/scope errors: E0425/E0433 for crate-root Interp at native_option_tables.rs:60/113 and three E0425 GLOBAL sites in native_coroutine_names.rs:123/126/202. The independently requested nine-crate command using tcl-lsp-db exits 101 after 98.44 seconds. Its diagnostics retain E0599 semantic_generation at commands.rs:5542/5543 repeated for library/test, plus E0603 private NativeCompiledBodyContext in original_compiler_effects.rs:602. None executes assertions or completes its requested build.

All three operations in this source group retain the same complete immutable inventory and `uniform_source: true`. The full JSON logs retain any partial or cached artifact paths reported before failure; this ledger attaches no new verified executable pin. No Clippy, listing, Python command, native provider result or whole-suite pass is included.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build196](frozen196/compiler-build196/receipt.json.gz) · [log](frozen196/compiler-build196/tests.log) | Request blocked; no assertions executed |
| [runtime-build196](frozen196/runtime-build196/receipt.json.gz) · [log](frozen196/runtime-build196/tests.log) | Request blocked; no assertions executed |
| [compiler-correct-packages-build196](frozen196/compiler-correct-packages-build196/receipt.json.gz) · [log](frozen196/compiler-correct-packages-build196/tests.log) | Request blocked; no assertions executed |

## Retained source 197 commands

The requested combined nine-crate library/test build exits101 after127.14 seconds with four Compiler/Registry import or method error sites repeated for library/test. It executes no assertions and does not complete the requested build. The independent Runtime library/test build exits0 after58.35 seconds and yields an independently verified rebuilt executable (`fresh: false`) with SHA a2c34e5ced4cd776bebe9668ca700096d14828ace88c38e514bd93f25dc6de71. Its exact successful build receipt and complete source inventory remain pinned together.

The bare filter `deeply_nested_foreach_reaches_the_explicit_host_evaluation_limit --exact` matches no assertion: zero passed and1240 filtered. The fully qualified `interp::tests::deeply_nested_foreach_reaches_the_explicit_host_evaluation_limit --exact` independently executes one assertion and passes after1.42 seconds with1239 filtered. The full Runtime invocation executes all1240 assertions and exits101 after249.22 seconds:1212 passed,28 failed, none ignored or filtered. Its whole stdout includes the final framework summary and all original failure detail; it is not a full-suite pass. The repeated focused assertion is separate from the full-run count.

All five commands retain the same complete immutable source inventory and `uniform_source: true`. Only the independently successful Runtime executable is pinned here. No Compiler/VM executable, Clippy, listing, Python command or native provider observation is claimed by this source group. Current edited source and its source/API proof bindings remain independent of these actual command results.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build197](frozen197/compiler-build197/receipt.json.gz) · [log](frozen197/compiler-build197/tests.log) | Four Compiler/Registry import/method sites repeated for library/test block the requested nine-crate build; no assertions executed |
| [runtime-build197](frozen197/runtime-build197/receipt.json.gz) · [log](frozen197/runtime-build197/tests.log) | Compilation passed; no assertions executed |
| [runtime-deep-foreach197](frozen197/runtime-deep-foreach197/receipt.json.gz) · [log](frozen197/runtime-deep-foreach197/tests.log) | 0 passed, 0 failed, 1240 filtered out |
| [runtime-deep-qualified197](frozen197/runtime-deep-qualified197/receipt.json.gz) · [log](frozen197/runtime-deep-qualified197/tests.log) | 1 passed, 0 failed, 1239 filtered out |
| [runtime-full197](frozen197/runtime-full197/receipt.json.gz) · [log](frozen197/runtime-full197/tests.log) | 1212 passed, 28 failed, 0 filtered out |

| Exact executable pin | Artifact reported fresh |
| --- | --- |
| [runtime](frozen197/pinned-runtime197.json) | `false` |

## Retained source 198 and 199 commands

Source198 retains five actual commands under one complete immutable source inventory and `uniform_source: true`. The requested combined twelve-package library/test build exits101 after178.14 seconds with eleven bounds-caller type mismatches and two Logical-operation test type errors. It executes no assertions and yields no completed requested Compiler image. The independent Runtime library/test build exits0 after117.95 seconds and produces a rebuilt executable (`fresh: false`) with SHA6421c02dcb784b632d18982de05c36989a9acc2a0ba83f59b5449e59e263c7a0. The exact Runtime build receipt, executable pin and complete inventory remain associated.

The exact Runtime `--list` command lists1240 assertions and executes none. The retained finite selector union executes36:17 pass,19 fail,1204 filtered, exit101 after23.16 seconds. Selector strings containing trailing ellipses are preserved literally; they grant no additional coverage. The separate fully qualified seven-control union executes7:0 pass,7 fail,1233 filtered, exit101 after12.82 seconds. Both commands retain whole logs and final framework summaries. These selected and repeated events do not establish a full-suite result.

Source199 retains two independent actual commands under its own complete immutable source inventory and `uniform_source: true`. The requested twelve-package library/test build exits101 after165.24 seconds with five Core consumer API errors. The Runtime library/test build exits101 after47.91 seconds with a missing test diagnostic method. Neither executes assertions or produces a completed requested test executable. No executable pin, source test pass, Clippy, Python or native provider observation is claimed for source199.

Current edited source/API contracts remain separate from these exact retained command results. Source198 Runtime selected coverage supplies no Compiler, VM, Server or whole-workspace result.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build198](frozen198/compiler-build198/receipt.json.gz) · [log](frozen198/compiler-build198/tests.log) | Twelve-package build blocked by eleven bounds callers and two Logical-operation test type errors; no assertions executed |
| [runtime-build198](frozen198/runtime-build198/receipt.json.gz) · [log](frozen198/runtime-build198/tests.log) | Compilation passed; no assertions executed |
| [runtime-list198](frozen198/runtime-list198/receipt.json.gz) · [log](frozen198/runtime-list198/tests.log) | 1240 tests listed; no assertions executed |
| [runtime-finite198](frozen198/runtime-finite198/receipt.json.gz) · [log](frozen198/runtime-finite198/tests.log) | 17 passed, 19 failed, 1204 filtered out |
| [runtime-new-controls198](frozen198/runtime-new-controls198/receipt.json.gz) · [log](frozen198/runtime-new-controls198/tests.log) | 0 passed, 7 failed, 1233 filtered out |
| [compiler-build199](frozen199/compiler-build199/receipt.json.gz) · [log](frozen199/compiler-build199/tests.log) | Twelve-package build blocked by five Core consumer API errors; no assertions executed |
| [runtime-build199](frozen199/runtime-build199/receipt.json.gz) · [log](frozen199/runtime-build199/tests.log) | Runtime build blocked by a missing test diagnostic method; no assertions executed |

| Exact executable pin | Artifact reported fresh |
| --- | --- |
| [runtime](frozen198/pinned-runtime198.json) | `false` |

## Retained source 200 commands

Source200 retains two actual library/test build commands under the same complete immutable source inventory and `uniform_source: true`. The requested twelve-package build exits101 after141.18 seconds with E0433 for an unimported `SourceCommandTransitionObligation` in a test and E0063 for a test `Module` initializer missing `source_metadata_input`. The independent Runtime build exits101 after55.82 seconds with E0277 for a test `&Vec<u8>` argument supplied to `new_bytes`.

Neither command executes assertions or supplies a completed requested test executable. No executable pin, Clippy result, Python result or Native provider observation is attributed to source200. The complete original receipts and whole logs retain the exact commands and inventory independently of current source contracts.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build200](frozen200/compiler-build200/receipt.json.gz) · [log](frozen200/compiler-build200/tests.log) | Exit101; two test compilation errors; no assertions |
| [runtime-build200](frozen200/runtime-build200/receipt.json.gz) · [log](frozen200/runtime-build200/tests.log) | Exit101; one test compilation error; no assertions |

## Retained source 201 commands

Twelve exact operations retain one complete immutable source inventory and `uniform_source: true`. The requested twelve-package library/test build exits101 after185.62 seconds: two DB `Module` initialisers omit `source_metadata_input`. It executes no assertions and does not complete the requested package build. Independent Runtime and Compiler-only builds pass after54.48 and173.70 seconds, respectively; each rebuilt (`fresh: false`) executable is pinned to its actual successful build receipt and source inventory. Compiler and Runtime listing commands enumerate8420 and1249 tests without executing assertions.

The Runtime finite union terminates with signal6 during the deep foreach assertion and supplies no final framework summary; all explicit pass/fail events and its incomplete assertion are retained. The independently executed union without deep controls completes37 passed and15 failed with1197 filtered. Compiler memo controls complete8 passed and1 failed with8411 filtered; the selected String-coordinate diagnostic control completes0 passed and1 failed with8419 filtered.

Two distinct trace invocations of the deep OO selector time out with exit124 after5.01 and15.01 seconds, preserving incomplete events and whole instrumentation output. The separate unbounded deep OO invocation finishes normally after1044.13 seconds:1 passed,0 failed,8419 filtered. It is not terminated, and its completed result does not convert either timed-out trace or any other command into a pass.

The exact commands, full original logs, framework summaries, receipt inventories and two executable associations remain independently reviewable. This group claims no complete workspace or Runtime-suite pass, Clippy/Python operation, other package executable or native provider observation. Source/API proof controls and current source contracts remain distinct from these actual source201 results.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build201](frozen201/compiler-build201/receipt.json.gz) · [log](frozen201/compiler-build201/tests.log) | Exit101; two DB Module initialisers omit source_metadata_input; requested twelve-package build incomplete, no assertions |
| [runtime-build201](frozen201/runtime-build201/receipt.json.gz) · [log](frozen201/runtime-build201/tests.log) | Compilation passed; no assertions executed |
| [compiler-only-build201](frozen201/compiler-only-build201/receipt.json.gz) · [log](frozen201/compiler-only-build201/tests.log) | Compilation passed; no assertions executed |
| [runtime-list201](frozen201/runtime-list201/receipt.json.gz) · [log](frozen201/runtime-list201/tests.log) | 1249 tests listed; no assertions executed |
| [runtime-finite201](frozen201/runtime-finite201/receipt.json.gz) · [log](frozen201/runtime-finite201/tests.log) | Signal 6; no final summary; 29 explicit pass, 12 explicit fail, 1 incomplete events |
| [runtime-finite-without-deep201](frozen201/runtime-finite-without-deep201/receipt.json.gz) · [log](frozen201/runtime-finite-without-deep201/tests.log) | 37 passed, 15 failed, 1197 filtered out |
| [compiler-list201](frozen201/compiler-list201/receipt.json.gz) · [log](frozen201/compiler-list201/tests.log) | 8420 tests listed; no assertions executed |
| [compiler-memo201](frozen201/compiler-memo201/receipt.json.gz) · [log](frozen201/compiler-memo201/tests.log) | 8 passed, 1 failed, 8411 filtered out |
| [compiler-string-equal-diagnostic201](frozen201/compiler-string-equal-diagnostic201/receipt.json.gz) · [log](frozen201/compiler-string-equal-diagnostic201/tests.log) | 0 passed, 1 failed, 8419 filtered out |
| [compiler-deep-oo-trace201](frozen201/compiler-deep-oo-trace201/receipt.json.gz) · [log](frozen201/compiler-deep-oo-trace201/tests.log) | Timeout exit124; no final summary; 0 explicit pass, 0 explicit fail, 1 incomplete events |
| [compiler-deep-oo-transfer-trace201](frozen201/compiler-deep-oo-transfer-trace201/receipt.json.gz) · [log](frozen201/compiler-deep-oo-transfer-trace201/tests.log) | Timeout exit124; no final summary; 0 explicit pass, 0 explicit fail, 1 incomplete events |
| [compiler-deep-oo201](frozen201/compiler-deep-oo201/receipt.json.gz) · [log](frozen201/compiler-deep-oo201/tests.log) | 1 passed, 0 failed, 8419 filtered out |

| Exact executable pin | Artifact reported fresh |
| --- | --- |
| [runtime](frozen201/pinned-runtime201.json) | `false` |
| [compiler](frozen201/pinned-compiler201.json) | `false` |

## Retained source 202 commands

Source202 retains two actual library/test build commands under the same complete immutable source inventory and `uniform_source: true`. The requested twelve-package build exits101 after71.32 seconds with E0308 for a VM precision-test byte value supplied to a String interface. The independent Runtime build exits101 after39.42 seconds with three E0433 test references to an unavailable `obj` module and E0624 for a test call to the private `transport_host_refusal_from` method.

Neither command executes assertions or supplies a completed requested test executable. No executable pin, Clippy result, Python result or Native provider observation is attributed to source202. The complete original receipts and whole logs retain the exact commands and inventory independently of current source contracts.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build202](frozen202/compiler-build202/receipt.json.gz) · [log](frozen202/compiler-build202/tests.log) | Exit101; one test compilation error; no assertions |
| [runtime-build202](frozen202/runtime-build202/receipt.json.gz) · [log](frozen202/runtime-build202/tests.log) | Exit101; four test compilation errors; no assertions |

## Retained source 203 commands

Seven exact operations retain one complete immutable source inventory and `uniform_source: true`. The requested twelve-package library/test build exits101 after275.47 seconds with three Registry test-reference errors: unavailable `SpecSurface`, `resolve_spec` and `available_subcommands` paths. It executes no assertions and does not complete the requested package build. Independent Runtime and Compiler-only builds pass after77.56 and140.72 seconds; each rebuilt (`fresh: false`) executable is pinned to its own actual successful build receipt and exact source inventory. The Runtime listing enumerates1266 tests without executing assertions.

The selected Runtime union completes38 passed and36 failed with1192 filtered; the selected Compiler metadata controls complete13 passed and0 failed with8414 filtered. These are exact selected results rather than complete package or workspace-suite claims. The separate source-phase trace times out with exit124 after20.03 seconds, retaining one incomplete deep OO event and no framework summary. Its instrumentation reports the source walk at4229 milliseconds and14276 lookup calls; those fields describe that bounded trace and supply no completed assertion, whole-test timing or speedup result.

Exact commands, complete original logs, framework summaries, inventories and the two successful independent executable associations remain reviewable. The incomplete requested twelve-package build supplies no successful full package build or other executable association here. No Clippy/Python operation, native provider observation or current source pass is inferred from this group. Source/API proof assertions retain their separate provider limits.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build203](frozen203/compiler-build203/receipt.json.gz) · [log](frozen203/compiler-build203/tests.log) | Exit101; three Registry test references fail (SpecSurface, resolve_spec and available_subcommands); requested twelve-package build incomplete, no assertions |
| [runtime-build203](frozen203/runtime-build203/receipt.json.gz) · [log](frozen203/runtime-build203/tests.log) | Compilation passed; no assertions executed |
| [compiler-only-build203](frozen203/compiler-only-build203/receipt.json.gz) · [log](frozen203/compiler-only-build203/tests.log) | Compilation passed; no assertions executed |
| [runtime-list203](frozen203/runtime-list203/receipt.json.gz) · [log](frozen203/runtime-list203/tests.log) | 1266 tests listed; no assertions executed |
| [runtime-finite203](frozen203/runtime-finite203/receipt.json.gz) · [log](frozen203/runtime-finite203/tests.log) | 38 passed, 36 failed, 1192 filtered out |
| [compiler-current-metadata203](frozen203/compiler-current-metadata203/receipt.json.gz) · [log](frozen203/compiler-current-metadata203/tests.log) | 13 passed, 0 failed, 8414 filtered out |
| [compiler-deep-phase-trace203](frozen203/compiler-deep-phase-trace203/receipt.json.gz) · [log](frozen203/compiler-deep-phase-trace203/tests.log) | Timeout exit124; no final summary; 0 explicit pass, 0 explicit fail, 1 incomplete events |

| Exact executable pin | Artifact reported fresh |
| --- | --- |
| [runtime](frozen203/pinned-runtime203.json) | `false` |
| [compiler](frozen203/pinned-compiler203.json) | `false` |

## Retained source 204 commands

Two actual library/test build commands retain the same complete immutable source inventory and `uniform_source: true`. The requested twelve-package build exits101 after105.36 seconds with four fixture compilation errors: E0433 for an unavailable `tcl_registry::SpecSurface` path, two E0308 type mismatches and E0507 for a move from a shared reference. The independent Runtime build exits101 after50.16 seconds with E0599 for the unavailable test method `resolve_original_command_key`.

Neither command executes assertions or supplies a successful requested executable pin. Clippy, Python, software tests and native provider observations are not attributed to these commands. Complete original receipts and whole logs preserve the exact requests and source inventory independently of current edited contracts.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build204](frozen204/compiler-build204/receipt.json.gz) · [log](frozen204/compiler-build204/tests.log) | Exit101; four fixture compilation errors; no assertions |
| [runtime-build204](frozen204/runtime-build204/receipt.json.gz) · [log](frozen204/runtime-build204/tests.log) | Exit101; one fixture compilation error; no assertions |

## Retained source 205 commands

All five actual commands retain the same complete immutable source inventory and `uniform_source: true`. The requested13-package library/test build exits101 after117.99 seconds with the dataflow `FnOnce` lifetime error repeated for library/test; it executes no assertions and supplies no completed aggregate executable pin. The independent Runtime build exits0 after27.94 seconds and reports its rebuilt test artifact (`fresh: false`). Its exact executable SHA4de8d30ba10481dd8d41e7e3e424775769aceb92fb2f60d6cdfc5efa37b0c279, successful build receipt and complete source inventory are verified together. The listing records1270 tests without executing assertions.

The default-stack selected invocation exits by SIGABRT/-6 after25.71 seconds at the deep-foreach test. Whole output retains57 explicit passes,7 explicit failures and one incomplete event, with no final framework result. The independently invoked identical filter vector under `RUST_MIN_STACK=67108864` completes79 selected assertions and exits101 after26.33 seconds:72 passed,7 failed,1191 filtered. The explicit environment change and repeated events remain separately recorded. Neither invocation establishes a full-suite pass, current-source success or native interpreter stack capacity.

Complete original gzip receipts and whole logs retain both commands, exact environment vectors, listing, partial events and final summary. Only the independently successful Runtime image is pinned; no Compiler/Registry/other consumer image, Clippy, Python or native execution is attributed to this group.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build205](frozen205/compiler-build205/receipt.json.gz) · [log](frozen205/compiler-build205/tests.log) | Requested13-package build exits101 after117.99s;one dataflow FnOnce lifetime error repeated for lib/test; no assertions or completed aggregate pin |
| [runtime-build205](frozen205/runtime-build205/receipt.json.gz) · [log](frozen205/runtime-build205/tests.log) | Independent Runtime build passed after27.94s; no assertions executed |
| [runtime-list205](frozen205/runtime-list205/receipt.json.gz) · [log](frozen205/runtime-list205/tests.log) | 1270 tests listed; no assertions executed |
| [runtime-finite205](frozen205/runtime-finite205/receipt.json.gz) · [log](frozen205/runtime-finite205/tests.log) | SIGABRT/-6 after25.71s;57 explicit passes,7 explicit failures,1 incomplete deep-foreach event; no final framework result |
| [runtime-finite-stack205](frozen205/runtime-finite-stack205/receipt.json.gz) · [log](frozen205/runtime-finite-stack205/tests.log) | Explicit RUST_MIN_STACK=67108864;72 passed,7 failed,1191 filtered after26.33s;79 selected assertions |

| Exact executable pin | Artifact reported fresh |
| --- | --- |
| [runtime](frozen205/pinned-runtime205.json) | `false` |

## Retained source 207 Runtime commands

All four actual commands retain the same complete immutable source inventory and `uniform_source: true`. The independent Runtime build exits0 after158.36 seconds and reports its rebuilt test artifact (`fresh: false`). Its executable SHAa88c2086671439243fed932e3210fa2ca3435bc7f394d5c1456291e191549538, successful build receipt and complete source inventory are verified together. The listing records1274 tests without executing assertions.

The exact array debug control exits101 after2.53 seconds with0 passed,1 failed and1273 filtered. Its whole panic output and the later repeated event remain separately recorded. The selected invocation explicitly uses `RUST_MIN_STACK=67108864` and completes82 assertions, exiting101 after40.43 seconds with69 passed,13 failed and1192 filtered. The retained [selection inventory](frozen207/runtime-selection207.json) matches all82 actual listed/observed selectors and the exact command filter vector, with no zero-match filter. Selection/listing records do not count as passed assertions; repeated debug/selected events do not supply distinct coverage. No full-suite/current-source success or native stack capacity follows.

Complete original gzip receipts and whole logs preserve exact commands, source image, environment vector, panic, final summaries and executable association. Only this independently successful Runtime image is pinned; no other consumer, Clippy, Python or native result is attributed to this group.

| Receipt and log | Actual result |
| --- | --- |
| [runtime-build207](frozen207/runtime-build207/receipt.json.gz) · [log](frozen207/runtime-build207/tests.log) | Independent Runtime build passed after158.36s; no assertions executed |
| [runtime-list207](frozen207/runtime-list207/receipt.json.gz) · [log](frozen207/runtime-list207/tests.log) | 1274 tests listed; no assertions executed |
| [runtime-array-debug207](frozen207/runtime-array-debug207/receipt.json.gz) · [log](frozen207/runtime-array-debug207/tests.log) | Exact array control completed0 passed/1 failed/1273 filtered after2.53s; its full panic/result and repeated selected outcome remain independent |
| [runtime-finite-stack207](frozen207/runtime-finite-stack207/receipt.json.gz) · [log](frozen207/runtime-finite-stack207/tests.log) | Explicit RUST_MIN_STACK=67108864;82 selected assertions completed69 passed/13 failed/1192 filtered after40.43s |

| Exact executable pin | Artifact reported fresh |
| --- | --- |
| [runtime](frozen207/pinned-runtime207.json) | `false` |

## Retained source 206 consumer commands

All 27 actual commands retain the same complete immutable source inventory and `uniform_source: true`: four requested compilations, eleven inventories, eleven selected batches and one bounded source-phase trace. Three compilations exit101 without assertions: the thirteen-package request reports a Registry `SpecSurface` reference error; the twelve-package request reports ten VM test diagnostics; the Compiler-only request reports two missing `oo_object_collision_message` methods in its VM dependency. Their entire Cargo messages and artifact records are retained without inferring a cause or successful requested build.

The eleven-package build exits0 after85.29 seconds. Each retained executable SHA is verified against its pin, successful build receipt and complete frozen source inventory. Cargo reports `fresh: true` for CLI support, Compiler and Syntax, and `fresh: false` for the other eight artifacts. These are the reported flags; the successful build does not imply that every artifact was rebuilt. Eleven inventories execute no assertions. The [actual selection record](frozen206/consumer-selection206.json) retains308 jobs with no zero-match job, its eleven planned selector sets and their exact filter vectors. The CmdCore vector contains an empty filter and therefore selects all175 listed tests; the Runtime API and test-support batches also select their entire62- and10-test inventories. Planned or listed names do not count as completed assertions.

Nine selected batches have final framework summaries: four pass and five fail. The Compiler batch receives SIGTERM after1687.92 seconds with166 explicit passes,28 explicit failures and one incomplete event; its690 planned selectors have no final result. The Core batch receives SIGTERM after1681.50 seconds with167 explicit passes,11 explicit failures and one incomplete event; its473 planned selectors have no final result. The complete [stop-purpose record](frozen206/root-superseded-consumer206-stop.json) retains the two owned process commands, PIDs and reasons, including the requirement for fresh exact-depth validation. Those partial events supply neither completed deep assertions nor complete batch tallies.

The separate source-phase trace times out with exit124 after60.06 seconds. It retains one incomplete deep OO event, no framework summary, and its actual instrumentation fields, including a source-walk field of4486 milliseconds. This bounded trace supplies no whole-test timing, completed assertion or speedup result. Its repeated deep selector remains a separate command observation.

Lossless gzip receipts, whole logs, pins and selection/stop records preserve the exact source image, command vectors, final or missing summaries and individual partial events. No complete workspace/current-source success, Registry or VM executable association, native outcome, Clippy or Python result is inferred from this group.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build206](frozen206/compiler-build206/receipt.json.gz) · [log](frozen206/compiler-build206/tests.log) | Compilation exit101 after205.80s; 1 coded diagnostics; no assertions or successful requested build |
| [consumer-build206](frozen206/consumer-build206/receipt.json.gz) · [log](frozen206/consumer-build206/tests.log) | Compilation exit101 after61.84s; 10 coded diagnostics; no assertions or successful requested build |
| [compiler-only-build206](frozen206/compiler-only-build206/receipt.json.gz) · [log](frozen206/compiler-only-build206/tests.log) | Compilation exit101 after38.01s; 2 coded diagnostics; no assertions or successful requested build |
| [valid-consumer-build206](frozen206/valid-consumer-build206/receipt.json.gz) · [log](frozen206/valid-consumer-build206/tests.log) | Eleven-package compilation passed after85.29s; exact artifact freshness recorded separately; no assertions |
| [cli-support-list206](frozen206/cli-support-list206/receipt.json.gz) · [log](frozen206/cli-support-list206/tests.log) | 26 tests listed; no assertions executed |
| [cmdcore-list206](frozen206/cmdcore-list206/receipt.json.gz) · [log](frozen206/cmdcore-list206/tests.log) | 175 tests listed; no assertions executed |
| [compiler-list206](frozen206/compiler-list206/receipt.json.gz) · [log](frozen206/compiler-list206/tests.log) | 8454 tests listed; no assertions executed |
| [core-list206](frozen206/core-list206/receipt.json.gz) · [log](frozen206/core-list206/tests.log) | 2730 tests listed; no assertions executed |
| [db-list206](frozen206/db-list206/receipt.json.gz) · [log](frozen206/db-list206/tests.log) | 114 tests listed; no assertions executed |
| [irules-list206](frozen206/irules-list206/receipt.json.gz) · [log](frozen206/irules-list206/tests.log) | 68 tests listed; no assertions executed |
| [lexer-list206](frozen206/lexer-list206/receipt.json.gz) · [log](frozen206/lexer-list206/tests.log) | 532 tests listed; no assertions executed |
| [runtimeapi-list206](frozen206/runtimeapi-list206/receipt.json.gz) · [log](frozen206/runtimeapi-list206/tests.log) | 62 tests listed; no assertions executed |
| [server-list206](frozen206/server-list206/receipt.json.gz) · [log](frozen206/server-list206/tests.log) | 658 tests listed; no assertions executed |
| [support-list206](frozen206/support-list206/receipt.json.gz) · [log](frozen206/support-list206/tests.log) | 10 tests listed; no assertions executed |
| [syntax-list206](frozen206/syntax-list206/receipt.json.gz) · [log](frozen206/syntax-list206/tests.log) | 610 tests listed; no assertions executed |
| [consumer-cli-support206](frozen206/consumer-cli-support206/receipt.json.gz) · [log](frozen206/consumer-cli-support206/tests.log) | 0 passed/5 failed/21 filtered after17.94s |
| [consumer-cmdcore206](frozen206/consumer-cmdcore206/receipt.json.gz) · [log](frozen206/consumer-cmdcore206/tests.log) | 173 passed/2 failed/0 filtered after0.03s |
| [consumer-compiler206](frozen206/consumer-compiler206/receipt.json.gz) · [log](frozen206/consumer-compiler206/tests.log) | SIGTERM after1687.92s; no final summary; 166 explicit pass/28 explicit fail/1 incomplete event, 690 planned selectors |
| [consumer-core206](frozen206/consumer-core206/receipt.json.gz) · [log](frozen206/consumer-core206/tests.log) | SIGTERM after1681.50s; no final summary; 167 explicit pass/11 explicit fail/1 incomplete event, 473 planned selectors |
| [consumer-db206](frozen206/consumer-db206/receipt.json.gz) · [log](frozen206/consumer-db206/tests.log) | 28 passed/5 failed/81 filtered after140.56s |
| [consumer-irules206](frozen206/consumer-irules206/receipt.json.gz) · [log](frozen206/consumer-irules206/tests.log) | 4 passed/1 failed/63 filtered after3.22s |
| [consumer-lexer206](frozen206/consumer-lexer206/receipt.json.gz) · [log](frozen206/consumer-lexer206/tests.log) | 7 passed/0 failed/525 filtered after0.01s |
| [consumer-runtimeapi206](frozen206/consumer-runtimeapi206/receipt.json.gz) · [log](frozen206/consumer-runtimeapi206/tests.log) | 62 passed/0 failed/0 filtered after0.01s |
| [consumer-server206](frozen206/consumer-server206/receipt.json.gz) · [log](frozen206/consumer-server206/tests.log) | 25 passed/6 failed/627 filtered after385.69s |
| [consumer-support206](frozen206/consumer-support206/receipt.json.gz) · [log](frozen206/consumer-support206/tests.log) | 10 passed/0 failed/0 filtered after0.10s |
| [consumer-syntax206](frozen206/consumer-syntax206/receipt.json.gz) · [log](frozen206/consumer-syntax206/tests.log) | 4 passed/0 failed/606 filtered after0.06s |
| [compiler-deep-phase-trace206](frozen206/compiler-deep-phase-trace206/receipt.json.gz) · [log](frozen206/compiler-deep-phase-trace206/tests.log) | Timeout exit124 after60.06s; one incomplete deep OO event; no framework summary |

| Exact executable pin | Artifact reported fresh | Listed | Planned selection |
| --- | --- | --- | --- |
| [cli-support](frozen206/pinned-cli-support206.json) | `true` | 26 | 5 |
| [cmdcore](frozen206/pinned-cmdcore206.json) | `false` | 175 | 175 |
| [compiler](frozen206/pinned-compiler206.json) | `true` | 8454 | 690 |
| [core](frozen206/pinned-core206.json) | `false` | 2730 | 473 |
| [db](frozen206/pinned-db206.json) | `false` | 114 | 33 |
| [irules](frozen206/pinned-irules206.json) | `false` | 68 | 5 |
| [lexer](frozen206/pinned-lexer206.json) | `false` | 532 | 7 |
| [runtimeapi](frozen206/pinned-runtimeapi206.json) | `false` | 62 | 62 |
| [server](frozen206/pinned-server206.json) | `false` | 658 | 31 |
| [support](frozen206/pinned-support206.json) | `false` | 10 | 10 |
| [syntax](frozen206/pinned-syntax206.json) | `true` | 610 | 4 |

## Retained source 208–211 build commands

Four actual `--lib --no-run` commands each retain their own complete immutable source inventory and `uniform_source: true`. The Runtime208 request exits101 after9.26 seconds with E0433 for `tcl_registry` in CmdCore. The thirteen-package209 request exits101 after81.94 seconds with twelve coded diagnostics: E0425, E0689, E0308, five E0277, two E0061 and two E0502. The thirteen-package210 request exits101 after2.65 seconds with E0433 for the CmdCore `tcl_registry` reference. The Runtime211 request exits101 after55.75 seconds with seven coded diagnostics: E0425, E0277, E0308 and four E0599.

No command executes assertions, produces a final libtest summary or supplies a successful requested executable pin. All four lossless gzip receipts and whole Cargo logs retain the exact command, elapsed time, source image and error locations. Reported dependency artifacts, current edited source, other images, native observations, Clippy and Python remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [runtime-build208](frozen208/runtime-build208/receipt.json.gz) · [log](frozen208/runtime-build208/tests.log) | Exit101 after9.26s; 1 coded diagnostics; no assertions |
| [all-consumer-build209](frozen209/all-consumer-build209/receipt.json.gz) · [log](frozen209/all-consumer-build209/tests.log) | Exit101 after81.94s; 12 coded diagnostics; no assertions |
| [all-consumer-build210](frozen210/all-consumer-build210/receipt.json.gz) · [log](frozen210/all-consumer-build210/tests.log) | Exit101 after2.65s; 1 coded diagnostics; no assertions |
| [runtime-build211](frozen211/runtime-build211/receipt.json.gz) · [log](frozen211/runtime-build211/tests.log) | Exit101 after55.75s; 7 coded diagnostics; no assertions |

## Retained source 213–217 commands

Eight actual commands retain complete immutable source inventories and `uniform_source: true`. Four requested builds exit101 without assertions: the thirteen-package213 and Runtime214 commands each report E0621; the thirteen-package215 command reports E0308 and E0425; the thirteen-package217 command reports six E0433 diagnostics for `tcl_registry`. Exact elapsed times, locations and whole Cargo output are retained per command. None supplies a successful requested executable pin.

The independent Runtime216 build exits0 after65.85 seconds and reports a rebuilt test artifact (`fresh: false`). Its executable SHA7c8117990ac985199fdf173b70a83de4503ab695307885746c8b0b0ce3f57fda, successful build receipt and full source inventory are verified together. The listing records1291 tests without assertions. The [selection inventory](frozen216/runtime-selection216.json) retains83 filters and91 exact listed names with no zero-match filter. One named deep host-limit control runs separately; the finite command passes its other90 names directly.

Both assertion commands explicitly remove `RUST_MIN_STACK` from the environment. The exact deep control completes1 passed,0 failed and1290 filtered after1.67 seconds. It tests the implemented host evaluation limit, separately from native nested-source observations. The finite command exits101 after42.52 seconds with84 passed,6 failed and1201 filtered. The six failures, full panic output and both final summaries remain intact. Neither command establishes a full-suite/current-source success or Native stack capacity.

All eight original receipts are losslessly compressed with their whole logs. Only the independently successful Runtime216 image is pinned; other requested consumer images, native observations, Clippy and Python results are not supplied by this group.

| Receipt and log | Actual result |
| --- | --- |
| [all-consumer-build213](frozen213/all-consumer-build213/receipt.json.gz) · [log](frozen213/all-consumer-build213/tests.log) | Exit101 after33.36s; 1 coded diagnostics; no assertions |
| [runtime-build214](frozen214/runtime-build214/receipt.json.gz) · [log](frozen214/runtime-build214/tests.log) | Exit101 after26.28s; 1 coded diagnostics; no assertions |
| [all-consumer-build215](frozen215/all-consumer-build215/receipt.json.gz) · [log](frozen215/all-consumer-build215/tests.log) | Exit101 after70.28s; 2 coded diagnostics; no assertions |
| [runtime-build216](frozen216/runtime-build216/receipt.json.gz) · [log](frozen216/runtime-build216/tests.log) | Independent Runtime build passes after65.85s; no assertions |
| [runtime-list216](frozen216/runtime-list216/receipt.json.gz) · [log](frozen216/runtime-list216/tests.log) | 1291 tests listed; no assertions |
| [runtime-default-stack216](frozen216/runtime-default-stack216/receipt.json.gz) · [log](frozen216/runtime-default-stack216/tests.log) | Default-stack exact host-limit control:1 passed,0 failed,1290 filtered after1.67s |
| [runtime-finite216](frozen216/runtime-finite216/receipt.json.gz) · [log](frozen216/runtime-finite216/tests.log) | Default-stack90 selected assertions:84 passed,6 failed,1201 filtered after42.52s |
| [all-consumer-build217](frozen217/all-consumer-build217/receipt.json.gz) · [log](frozen217/all-consumer-build217/tests.log) | Exit101 after4.09s; 6 coded diagnostics; no assertions |

| Exact executable pin | Artifact reported fresh |
| --- | --- |
| [runtime](frozen216/pinned-runtime216.json) | `false` |

## Retained source 218 build command

One actual thirteen-package `--lib --no-run` command retains its complete immutable source inventory and `uniform_source: true`. It exits101 after30.25 seconds with E0507: cannot move out of `*profile` behind a shared reference. The full Cargo log retains the exact test-fixture location and compiler explanation.

No assertion executes, no final libtest summary is emitted and the failed requested build supplies no successful requested executable pin. Its lossless original gzip receipt and whole log preserve the command, elapsed time, source image and error location. Current edited source, dependency artifacts, other images, native observations, Clippy and Python remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [all-consumer-build218](frozen218/all-consumer-build218/receipt.json.gz) · [log](frozen218/all-consumer-build218/tests.log) | Exit101 after30.25s; 1 coded diagnostics; no assertions |

## Retained source 219 Runtime commands

Five closed actual commands retain the same complete immutable source inventory and `uniform_source: true`. The independent Runtime build passes after 49.25 seconds and reports its rebuilt test artifact (`fresh: false`). Executable SHA 66adb54dd78edcaaacfdea25c5986fc5647b1c227e516c73b5a1454e003a7029, successful build receipt and complete source inventory are verified together. The listing records 1301 tests without executing assertions.

All three assertion commands explicitly remove `RUST_MIN_STACK`. The separate exact deep host-limit control completes 1 passed, 0 failed and 1300 filtered after 1.73 seconds. The [selection inventory](frozen219/runtime-selection219.json) retains 85 filters / 101 exact names with no zero-match filter; the finite command passes the other 100 names directly. It aborts with SIGABRT(-6) after 28.71 seconds and emits no aggregate libtest summary. Its whole log records 77 completed passed events, 1 completed failed event and 1 incomplete event; 21 selected names are not entered. These are partial events, not a completed 100-assertion result.

The completed failure retains the original C8.4 info -nons message mismatch. The final incomplete namespace control retains its literal-pool borrow panic, cleanup backtrace and non-unwinding abort text. A separate command enters those 21 names and completes 21 passed, 0 failed and 1280 filtered after 15.18 seconds. Its exact names are verified against the names absent from the aborted command. This separate result supplies no aggregate summary for the aborted command. Whole diagnostics and exact source/executable associations are preserved. The separate deep pass, inventory and successful build do not supply a finite/full-suite/current-source pass or native stack capacity.

All five lossless original gzip receipts, whole logs, executable pin and selection inventory remain independent of other consumer/source images, Native provider outcomes, Clippy and Python.

| Receipt and log | Actual result |
| --- | --- |
| [runtime-build219](frozen219/runtime-build219/receipt.json.gz) · [log](frozen219/runtime-build219/tests.log) | Independent rebuilt Runtime build0 after49.25s; no assertions |
| [runtime-list219](frozen219/runtime-list219/receipt.json.gz) · [log](frozen219/runtime-list219/tests.log) | 1301 tests listed; no assertions |
| [runtime-default-stack219](frozen219/runtime-default-stack219/receipt.json.gz) · [log](frozen219/runtime-default-stack219/tests.log) | Default-stack exact host-limit control1P/0F/1300filtered after1.73s |
| [runtime-finite219](frozen219/runtime-finite219/receipt.json.gz) · [log](frozen219/runtime-finite219/tests.log) | Default-stack100-name command SIGABRT(-6) after28.71s; partial77passed/1failed/1incomplete, no aggregate summary |
| [runtime-remaining219](frozen219/runtime-remaining219/receipt.json.gz) · [log](frozen219/runtime-remaining219/tests.log) | Separate remaining 21-name command: 21 passed / 0 failed / 1280 filtered after 15.18s |

| Exact executable pin | Artifact reported fresh |
| --- | --- |
| [runtime](frozen219/pinned-runtime219.json) | `false` |

## Retained source 220 build command

One actual thirteen-package `--lib --no-run` command retains its complete immutable source inventory and `uniform_source: true`. It exits 101 after 149.47 seconds. The whole Cargo output retains 15 coded diagnostic events: seven E0308, one E0433, two E0277, four E0599 and one E0505, including the repeated command-binding location as separate actual events. Exact locations, messages and explanations remain in the immutable log.

No assertion executes, no final libtest summary is emitted and the failed requested build supplies no successful requested executable pin. Its lossless original gzip receipt and whole log preserve the command, elapsed time, source image and error location. Current edited source, dependency artifacts, other images, native observations, Clippy and Python remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [all-consumer-build220](frozen220/all-consumer-build220/receipt.json.gz) · [log](frozen220/all-consumer-build220/tests.log) | Exit 101 after 149.47s; 15 coded diagnostics; no assertions |

## Retained sources 221 and 222 build commands

Two independent actual `--lib --no-run` commands retain their own complete immutable source inventories and `uniform_source: true`. Runtime221 exits 101 after 56.69 seconds with E0599: OptionTable has no index_of_original method. The thirteen-package Main222 command exits 101 after 136.41 seconds with two E0425 diagnostics for missing CommandRegistry type imports. Each whole Cargo log preserves its own exact locations, compiler explanations and warnings.

No assertion executes, no final libtest summary is emitted and neither failed requested build supplies a successful requested executable pin. Both lossless original gzip receipts and whole logs preserve each exact command, elapsed time, independent source image and diagnostic locations. Current edited source, dependency artifacts, other images, native observations, Clippy and Python remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [runtime-build221](frozen221/runtime-build221/receipt.json.gz) · [log](frozen221/runtime-build221/tests.log) | Exit 101 after 56.69s; 1 coded diagnostics; no assertions |
| [all-consumer-build222](frozen222/all-consumer-build222/receipt.json.gz) · [log](frozen222/all-consumer-build222/tests.log) | Exit 101 after 136.41s; 2 coded diagnostics; no assertions |

## Retained sources 223 and 224 commands

The three Runtime223 commands retain one complete immutable source inventory and `uniform_source: true`. Its independent build passes after 22.73 seconds and reports a rebuilt test artifact (`fresh: false`). Executable SHA 995bca81ccf44b031ca0b8cec4946bd3c2954f84a6db79ed561b0abc017670f0, successful build receipt and complete source inventory are verified together. The listing records 1304 tests without executing assertions.

The [selection inventory](frozen223/runtime-selection223.json) retains 88 filters / 104 names with no zero-match filter. The actual finite command supplies the other 103 names with `--exact`; its complete log records 101 passed, 2 failed, 0 ignored and 1201 filtered after 35.39 seconds. No selected name is incomplete. The two failures retain the C8.5 info missing-selector arity mismatch and the literal-retirement fixture's missing selected original Bytecode assertion. The namespace source comparator completes in this image. These are measured results for the exact Runtime223 test definitions; edited fixtures, the separately identified deep control, current source and full-suite results have no result from this command.

The independent thirteen-package Main224 `--lib --no-run` command exits 101 after 152.37 seconds and retains its own different source image. Its whole Cargo log contains E0425 for `tcl_lexer::whole_var_ref` in Core rename and an uncoded FnOnce lifetime diagnostic in Core document links. No assertions execute and the failed requested build supplies no successful requested executable pin or aggregate consumer result. Individual dependency artifacts cannot supply a completed requested build.

All four lossless original gzip receipts, whole logs, Runtime executable pin and exact selection remain independent of native provider outcomes, other source images, Clippy and Python.

| Receipt and log | Actual result |
| --- | --- |
| [runtime-build223](frozen223/runtime-build223/receipt.json.gz) · [log](frozen223/runtime-build223/tests.log) | Runtime build0 after22.73s; no assertions |
| [runtime-list223](frozen223/runtime-list223/receipt.json.gz) · [log](frozen223/runtime-list223/tests.log) | 1304 tests listed; no assertions |
| [runtime-finite223](frozen223/runtime-finite223/receipt.json.gz) · [log](frozen223/runtime-finite223/tests.log) | Exact103-name command101P/2F/1201filtered after35.39s; completed final summary |
| [all-consumer-build224](frozen224/all-consumer-build224/receipt.json.gz) · [log](frozen224/all-consumer-build224/tests.log) | Independent13-package buildFAIL101 after152.37s; two compiler diagnostics, no assertions |

| Exact executable pin | Artifact reported fresh |
| --- | --- |
| [runtime](frozen223/pinned-runtime223.json) | `false` |

## Independent compiler224 and runtime225 commands

The independent compiler-only224b build passes after 190.01 seconds and pins executable SHA bf4eacb0fdec7783289da44a91a762251b5870485642118465368ac2ad5eec9f to its successful build receipt and complete immutable source image. Its listing records 8516 tests without executing assertions. This build supplies no successful thirteen-package Main224 outcome.

The two exact compiler batches retain 29 names / 11 passed / 18 failed and 32 names / 11 passed / 21 failed after 24.15 and 21.60 seconds. Complete selection inventories, individual events, failed assertions and final summaries remain intact. The32-name inventory explicitly excludes optimiser::chain_fold::tests::deeply_nested_if_survives_fold_script; it supplies no result for that selector. The separate exact standalone-protocol selector is absent from this executable; exit0 with all8516 filtered executes zero assertions.

The unchanged depth1000 unknown-handler extraction control completes1P after710.74 seconds under its test's explicit64MiB stack. The exact log records62920 complete-world nested visits and1001 layouts, source completion547541ms and later lowering709311ms. This test asserts that the original source returns; it does not assert semantic output, provider equivalence or a current edited source performance improvement.

The command labeled Runtime225 without an explicit manifest selects the root Cargo workspace and exits101 after217.67 seconds with two ENOSPC diagnostics in rustls and Core. It is not Runtime verification and supplies no requested Runtime executable pin. The independent225b command explicitly selects runtime/rust/Cargo.toml and a clean target, passes after104.10 seconds and reports a rebuilt artifact (`fresh: false`). Executable SHA87e448c12aa30ca90f085365575564ff90796e0e73009a16329504c3cccd5302 is verified with that successful receipt and source inventory. Its listing records1304 tests without executing assertions.

The exact103-name Runtime225 command completes102P/1F/0ignored/1201filtered after38.14 seconds. The sole failed selector is the original Info-dispatch comparator, whose whole log retains the Jim bare-info arity difference. C-provider Info windows and the genuine Bytecode-retirement software control complete in this image. These measured results apply only to this exact executable and its selected definitions; no current-source, full-suite, generic native-dispatch or private provider identity result follows.

All ten lossless original gzip receipts, whole logs, two executable pins and three exact selection inventories retain their own commands and source associations. Other source images, native provider observations, Clippy and Python remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build224b](frozen224/compiler-build224b/receipt.json.gz) · [log](frozen224/compiler-build224b/tests.log) | Independent requested buildPASS0 after190.01s; no assertions |
| [compiler-list224](frozen224/compiler-list224/receipt.json.gz) · [log](frozen224/compiler-list224/tests.log) | 8516 tests listed; no assertions |
| [compiler-aot224](frozen224/compiler-aot224/receipt.json.gz) · [log](frozen224/compiler-aot224/tests.log) | 11P/18F/8487filtered after24.15s; complete final summary |
| [compiler-consumers224](frozen224/compiler-consumers224/receipt.json.gz) · [log](frozen224/compiler-consumers224/tests.log) | 11P/21F/8484filtered after21.60s; complete final summary |
| [compiler-standalone-protocol224](frozen224/compiler-standalone-protocol224/receipt.json.gz) · [log](frozen224/compiler-standalone-protocol224/tests.log) | 0P/0F/8516filtered after0.06s; zero assertions |
| [compiler-deep-phase-trace224](frozen224/compiler-deep-phase-trace224/receipt.json.gz) · [log](frozen224/compiler-deep-phase-trace224/tests.log) | 1P/0F/8515filtered after710.74s; complete final summary |
| [runtime-build225](frozen225/runtime-build225/receipt.json.gz) · [log](frozen225/runtime-build225/tests.log) | Root-workspace commandFAIL101 after217.67s; two ENOSPC diagnostics; not Runtime verification |
| [runtime-build225b](frozen225/runtime-build225b/receipt.json.gz) · [log](frozen225/runtime-build225b/tests.log) | Independent requested buildPASS0 after104.10s; no assertions |
| [runtime-list225](frozen225/runtime-list225/receipt.json.gz) · [log](frozen225/runtime-list225/tests.log) | 1304 tests listed; no assertions |
| [runtime-finite225](frozen225/runtime-finite225/receipt.json.gz) · [log](frozen225/runtime-finite225/tests.log) | 102P/1F/1201filtered after38.14s; complete final summary |

| Exact executable pin | Artifact reported fresh |
| --- | --- |
| [compiler](frozen224/pinned-compiler224.json) | `false` |
| [runtime](frozen225/pinned-runtime225.json) | `false` |

## Exact replay artifact storage

The [closed storage225 journals](replay-storage/storage225/README.md) preserve three pinned Rust executables, 591 inactive command receipts/logs and 85 archived artifact directories. Independent streaming checks verify compressed and original SHA-256 digests, byte lengths, every tar leaf and permission mode. Restore an exact pinned artifact before attempting its recorded command; storage verification or restoration executes no assertion and supplies no new result. Disposable Cargo cache removal is separate from replay artifact storage.

The [closed storage239 executable journal](replay-storage/storage239/README.md) preserves nine inactive pinned Rust executables. Independent streaming checks verify all compressed/decompressed digests and lengths against unchanged published pins. The complete original bytes remain restorable; verification or restoration supplies no assertion or new measured outcome.

The [closed duplicate artifact archive journal](replay-storage/artifact-archives/README.md) preserves363 approved archives and all8,805 original member inventories. Independent checks verify every archive/file digest and exact path/type/mode/uid/gid/mtime/size. These restoration records supply no new applied-state, Native behavior or assertion result.

## Independent aggregate226 and runtime227 commands

The actual Main226 command requests fourteen packages with --lib --no-run and exits101 after95.72 seconds. Its complete Cargo log retains E0603 for the private authored_policy helper and an uncoded FnOnce lifetime error, both in Compiler auto_path_eval/source_expression.rs. No assertion executes and this failed requested build supplies no successful aggregate executable pin. Its complete immutable source inventory remains separate from Runtime227.

The explicit Runtime227 build passes after63.13 seconds and reports a rebuilt artifact (fresh: false). Executable SHA4e528e880cec535c9e1dac91d7c7f10f8900266a64dd6878a44bb1d6c4033232 is verified with its successful build receipt and complete immutable source image. Its listing records1306 tests without executing assertions. The exact selection retains103 existing names plus two independently identified child-alias names; every selected name occurs in the listing.

The105-name finite command completes103 passed,2 failed,0 ignored and1201 filtered after34.04 seconds. The whole-source child-alias comparator fails at the Jim produced-NUL diagnostic bytes; the separate missing/foreign issuer control fails its expected child refusal-state assertion. All other103 selected names complete in this exact image, including the original Info comparator and genuine Bytecode-retirement control. These measured events establish no pass for edited child controls, current source, the full suite or additional native/private provider behavior.

All four lossless original gzip receipts, whole logs, exact selection and executable pin retain their commands and independent source associations. Storage restoration, other binaries, current implementation edits, native provider captures, Clippy and Python supply no result to these commands.

| Receipt and log | Actual result |
| --- | --- |
| [all-consumer-build226](frozen226/all-consumer-build226/receipt.json.gz) · [log](frozen226/all-consumer-build226/tests.log) | 14-package aggregate buildFAIL101 after95.72s; two compiler diagnostics, no assertions |
| [runtime-build227](frozen227/runtime-build227/receipt.json.gz) · [log](frozen227/runtime-build227/tests.log) | Independent Runtime buildPASS0 after63.13s; no assertions |
| [runtime-list227](frozen227/runtime-list227/receipt.json.gz) · [log](frozen227/runtime-list227/tests.log) | 1306 tests listed; no assertions |
| [runtime-finite227](frozen227/runtime-finite227/receipt.json.gz) · [log](frozen227/runtime-finite227/tests.log) | Exact105-name command103P/2F/1201filtered after34.04s; complete final summary |

| Exact executable pin | Artifact reported fresh |
| --- | --- |
| [runtime](frozen227/pinned-runtime227.json) | `false` |

## Independent compiler/Registry228 and Runtime229 commands

The compiler-only228 build exits101 after122.25 seconds with E0308 in common_aot_plan.rs:2536: its test assigns None to a CommandTokens value. The whole Cargo log retains the exact expected/found types and source location. No assertion executes and this failed requested compiler build supplies no successful compiler executable pin.

The independently requested Registry228 build passes after144.20 seconds and pins executable SHAcb73fa67611ff5803ff759507d515070c01f9b3b20a1a39f8ea3e48cad77ca64 to its successful build receipt and complete immutable source inventory. Its listing records1513 tests without executing assertions. Its exact two-name batch completes2P/0F/1511filtered after0.02 seconds: the typed authored path-operation control and registered Jim Info-arity recipe. These are source/API software assertions, independent of external native provider observations and the failed compiler-only build.

The independent Runtime229 build passes after20.46 seconds and pins executable SHA4dcbd78ee0c09a2c7a298a7271a0445d2617345ac6f6d114ee649eb852c41e88 to its own successful receipt and different complete source inventory. Its listing records1306 tests without executing assertions. Its exact105-name selection completes104P/1F/0ignored/1201filtered after35.36 seconds. The whole child-alias comparator fails at tcl8.4/original-unicode-child-alias; its full byte arrays and original assertion remain intact. The independently authored child refusal control and all other103 selected names complete in this exact image. This command supplies no successful whole36-window child comparison, edited-definition result or full-suite result.

Both successful build artifacts report fresh: false. All seven lossless original gzip receipts, whole logs, two exact selections and two executable pins retain their separate requested commands, images and outcomes. External native measurements, current edits, other binaries, Clippy and Python remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build228](frozen228/compiler-build228/receipt.json.gz) · [log](frozen228/compiler-build228/tests.log) | Compiler-only buildFAIL101 after122.25s; one E0308 test diagnostic, no assertions |
| [registry-build228](frozen228/registry-build228/receipt.json.gz) · [log](frozen228/registry-build228/tests.log) | Independent requested buildPASS0 after144.20s; no assertions |
| [registry-list228](frozen228/registry-list228/receipt.json.gz) · [log](frozen228/registry-list228/tests.log) | 1513 tests listed; no assertions |
| [registry-finite228](frozen228/registry-finite228/receipt.json.gz) · [log](frozen228/registry-finite228/tests.log) | Exact2-name command2P/0F/1511filtered after0.02s; complete final summary |
| [runtime-build229](frozen229/runtime-build229/receipt.json.gz) · [log](frozen229/runtime-build229/tests.log) | Independent requested buildPASS0 after20.46s; no assertions |
| [runtime-list229](frozen229/runtime-list229/receipt.json.gz) · [log](frozen229/runtime-list229/tests.log) | 1306 tests listed; no assertions |
| [runtime-finite229](frozen229/runtime-finite229/receipt.json.gz) · [log](frozen229/runtime-finite229/tests.log) | Exact105-name command104P/1F/1201filtered after35.36s; complete final summary |

| Exact executable pin | Artifact reported fresh |
| --- | --- |
| [registry](frozen228/pinned-registry228.json) | `false` |
| [runtime](frozen229/pinned-runtime229.json) | `false` |

## Compiler230 build command

The actual compiler-only230 --lib --no-run command exits101 after72.94 seconds with uniform_source: true. Its complete Cargo output retains one E0599 fixture diagnostic: Arc<[SourceInstalledProcedureBody]> has no clear method. The exact original receipt retains the whole immutable source inventory, command and elapsed time; the whole log retains the source location, compiler explanation and warnings.

No assertion executes, no libtest summary is emitted and the blocked requested compiler build supplies no successful executable pin. Current edited fixtures, other source images, dependency artifacts, native provider outcomes, Clippy and Python remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build230](frozen230/compiler-build230/receipt.json.gz) · [log](frozen230/compiler-build230/tests.log) | Compiler-only buildFAIL101 after72.94s; one E0599 fixture diagnostic; no assertions |

## Compiler231 build command

The actual compiler-only231 --lib --no-run command exits101 after138.64 seconds with uniform_source: true. Its complete Cargo output retains two fixture diagnostics: E0599 uses an absent NativeWord.source method, and E0597 retains a loop closure borrow beyond its valid lifetime. The exact original receipt retains the whole immutable source inventory, command and elapsed time; the whole log retains the source location, compiler explanation and warnings.

No assertion executes, no libtest summary is emitted and the blocked requested compiler build supplies no successful executable pin. Current edited fixtures, other source images, dependency artifacts, native provider outcomes, Clippy and Python remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build231](frozen231/compiler-build231/receipt.json.gz) · [log](frozen231/compiler-build231/tests.log) | Compiler-only buildFAIL101 after138.64s; E0599 and E0597 fixture diagnostics; no assertions |

## Runtime233 commands

The independently requested Runtime233 build passes after77.12 seconds with uniform_source: true. Its exact successful build receipt and complete immutable source inventory pin executable SHA52a23ee9409925987b604fb0789d32bac4cc212128a78245fef2c2b66f68ea8c. The listing records1306 tests without executing assertions.

The exact105-name command completes105P/0F/0ignored/1201filtered after41.08 seconds with its complete final libtest summary. The whole36 original child-alias comparison, independently authored refusal control and all other103 selected names pass in this precise executable. The complete selection has zero missing names and remains identical to the independently retained Runtime229 selection. These software comparisons add no native execution, private provider identity or full-suite result. This source image contains production308; this selection executes none of the new Controls30955-window encoding comparison definitions.

All three whole logs, exact original gzip receipts, original selection and executable pin retain their separate commands, source image and channels. Current edits, other binaries/images, external native provider outcomes, Clippy and Python remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [runtime-build233](frozen233/runtime-build233/receipt.json.gz) · [log](frozen233/runtime-build233/tests.log) | Requested Runtime buildPASS0 after77.12s; no assertions |
| [runtime-list233](frozen233/runtime-list233/receipt.json.gz) · [log](frozen233/runtime-list233/tests.log) | 1306 tests listed; no assertions |
| [runtime-finite233](frozen233/runtime-finite233/receipt.json.gz) · [log](frozen233/runtime-finite233/tests.log) | Exact105-name command105P/0F/1201filtered after41.08s; complete final summary |

[Exact executable pin](frozen233/pinned-runtime233.json) reports artifact fresh: `false`.

## Runtime234 commands

The independently requested Runtime234 build passes after43.44 seconds with uniform_source: true. Its exact successful build receipt and complete immutable source inventory pin executable SHA1901bff0ecbc8c1a1ab7cd1ac15bb188e00d13cf5206645f8a23c9260790257f. The listing records1308 tests without executing assertions.

The exact107-name command completes107P/0F/0ignored/1201filtered after51.13 seconds with its complete final libtest summary. The whole36 original child-alias comparison, independently authored refusal control and all other103 selected names pass in this precise executable. The complete selection has zero missing names and retains the independently recorded Runtime233105 names plus the original encoding comparator and its foreign/Jim API refusal control. These software comparisons add no native execution, private provider identity or full-suite result. The actual Runtime encoding comparator passes all55 original C constructor/storage/output/error windows against unchanged Native306/307 columns. Its separate foreign/Jim API refusal control also passes. The CmdCore pure codec and VM comparator are not selected by this Runtime command.

All three whole logs, exact original gzip receipts, original selection and executable pin retain their separate commands, source image and channels. Current edits, other binaries/images, external native provider outcomes, Clippy and Python remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [runtime-build234](frozen234/runtime-build234/receipt.json.gz) · [log](frozen234/runtime-build234/tests.log) | Requested Runtime buildPASS0 after43.44s; no assertions |
| [runtime-list234](frozen234/runtime-list234/receipt.json.gz) · [log](frozen234/runtime-list234/tests.log) | 1308 tests listed; no assertions |
| [runtime-finite234](frozen234/runtime-finite234/receipt.json.gz) · [log](frozen234/runtime-finite234/tests.log) | Exact107-name command107P/0F/1201filtered after51.13s; complete final summary |

[Exact executable pin](frozen234/pinned-runtime234.json) reports artifact fresh: `false`.

## Main232 commands

The independently requested Compiler-only232 build passes after198.85 seconds with uniform_source: true and pins executable SHAc90a6ab0224527fa64518b401872ece4149b386093b3c2ca01dd6efd8dd18d67. The listing records8543 tests without assertions. Its exact15-name structural command completes5P/10F after8.79 seconds; the separate exact61-name baseline command completes26P/35F after41.84 seconds. Both retain complete final summaries, every actual event and zero-missing original selections. Results belong only to this unchanged pinned image.

The separate all-consumer check fails101 after84.55 seconds with one Explorer private-field pattern error. It executes no assertions and supplies no successful requested consumer image; its failure remains independent of the successful Compiler-only build.

The unchanged original depth1000 selector completes1P/0F after497.52 seconds under its recorded command and environment. Its full trace records62920 layout visits,1000 inputs and1000 parsed commands; the source trace ends at388772 milliseconds with1001 layouts. This is one exact test outcome and its observed trace, not proof of semantic output, native-provider equivalence or a current-source performance improvement.

All six complete logs, exact original gzip receipts, separate selections and successful Compiler-only pin retain the complete immutable source inventory and original command channels. Current edits, other images, Runtime validation, external native outcomes, Clippy and Python remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build232](frozen232/compiler-build232/receipt.json.gz) · [log](frozen232/compiler-build232/tests.log) | Requested Compiler-only buildPASS0 after198.85s; no assertions |
| [compiler-list232](frozen232/compiler-list232/receipt.json.gz) · [log](frozen232/compiler-list232/tests.log) | 8543 tests listed; no assertions |
| [compiler-structural232](frozen232/compiler-structural232/receipt.json.gz) · [log](frozen232/compiler-structural232/tests.log) | Exact15-name command5P/10F/8528filtered after8.79s; complete final summary |
| [compiler-baseline232](frozen232/compiler-baseline232/receipt.json.gz) · [log](frozen232/compiler-baseline232/tests.log) | Exact61-name command26P/35F/8482filtered after41.84s; complete final summary |
| [all-consumer-check232](frozen232/all-consumer-check232/receipt.json.gz) · [log](frozen232/all-consumer-check232/tests.log) | All-consumer checkFAIL101 after84.55s; one Explorer private-field pattern error; no assertions or successful consumer pin |
| [compiler-deep-phase-trace232](frozen232/compiler-deep-phase-trace232/receipt.json.gz) · [log](frozen232/compiler-deep-phase-trace232/tests.log) | Exact1-name command1P/0F/8542filtered after497.52s; complete final summary |

[Exact Compiler-only executable pin](frozen232/pinned-compiler232.json) reports artifact fresh: `false`.

## Compiler235 nonuniform build

The exact Compiler235 build fails101 after119.01 seconds with uniform_source: false and no assertions or successful requested executable. Its whole log records three unavailable InvocationDialect paths and two lexer Result handling errors. Cargo also reconciled one test-support dependency in Cargo.lock during this command. The recorded pre-command source inventory, byte-exact original lockfile, Cargo-updated lockfile and source-owner receipt remain independent retained artifacts; this build supplies no uniform verification claim.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build235](frozen235/compiler-build235/receipt.json.gz) · [log](frozen235/compiler-build235/tests.log) | Failed101, nonuniform source, five compile errors; no assertions or successful pin |

[Lockfile source receipt](frozen235/lockfile-source235/receipt.json) retains the exact one-line dependency difference and both payload hashes: [original](frozen235/lockfile-source235/original-Cargo.lock) and [Cargo-updated](frozen235/lockfile-source235/cargo-updated-Cargo.lock).

## Additional Runtime234 command

Six exact additional tests pass after15.13 seconds with uniform_source: true under the same successful Runtime234 executable and complete unchanged source inventory. Every name occurs in the original1308-test inventory and differs from the107-name selection, covering113 unique selected names through two separate completed runs. The original receipt retains the exact six-name command and full6P/0F/1302filtered summary. This adds neither a full-suite result nor external native execution, another backend result or a pass for current edited source.

| Receipt and log | Actual result |
| --- | --- |
| [runtime-additional234](frozen234/runtime-additional234/receipt.json.gz) · [log](frozen234/runtime-additional234/tests.log) | Exact6P/0F; same unchanged Runtime234 pin; six names disjoint from107-name run |

## Main236 commands

The locked Compiler/VM/CmdCore236 build fails101 after188.57 seconds with five VM test type errors. It executes no assertions and supplies no successful combined pin. The independently requested Compiler-only build passes after0.47 seconds and pins executable SHAb7f66200012b401c2c3fbf638a1d9974275c4a72ab78febef752a0c42b040f97. Its listing records8557 tests without assertions. The exact102-name command completes42P/60F after81.21 seconds; an independent one-name original-command-table trace completes0P/1F after1.82 seconds. Both retain every actual event and their complete failing summaries.

The independently requested CmdCore236 build passes after7.52 seconds and pins executable SHA54efa3f39bfe01ba2454e51d579f2f08dd0542639c063cadb93a2d6fe02fc162. Its listing records177 tests without assertions. The exact selected encoding comparator passes1P/0F after0.004 seconds and checks all55 retained C byte/message/errorCode windows as a software API control. Runtime object-storage comparisons, VM execution and original external provider runs retain their separate receipts and scopes.

All eight commands record uniform_source: true under the same complete immutable source snapshot. Successful separate builds and inventories do not convert the combined build failure or compiler assertion failures into an aggregate pass. Whole logs, exact gzip receipts, zero-missing selections and both successful independent pins remain retained. Current edited definitions, other images, native outcomes, Clippy, Python and full suites remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [main-build236](frozen236/main-build236/receipt.json.gz) · [log](frozen236/main-build236/tests.log) | Compiler/VM/CmdCore buildFAIL101 after188.57s; five VM test type errors; no assertions or successful combined pin |
| [compiler-only-build236](frozen236/compiler-only-build236/receipt.json.gz) · [log](frozen236/compiler-only-build236/tests.log) | Independently requested tcl-compiler buildPASS0 after0.47s; no assertions |
| [compiler-list236](frozen236/compiler-list236/receipt.json.gz) · [log](frozen236/compiler-list236/tests.log) | 8557 tests listed; no assertions |
| [compiler-finite236](frozen236/compiler-finite236/receipt.json.gz) · [log](frozen236/compiler-finite236/tests.log) | Exact102-name command 42P/60F/8455filtered after81.21s; complete final summary |
| [backend-original-trace236](frozen236/backend-original-trace236/receipt.json.gz) · [log](frozen236/backend-original-trace236/tests.log) | Exact1-name command 0P/1F/8556filtered after1.82s; complete final summary |
| [cmdcore-build236](frozen236/cmdcore-build236/receipt.json.gz) · [log](frozen236/cmdcore-build236/tests.log) | Independently requested tcl-cmd-core buildPASS0 after7.52s; no assertions |
| [cmdcore-list236](frozen236/cmdcore-list236/receipt.json.gz) · [log](frozen236/cmdcore-list236/tests.log) | 177 tests listed; no assertions |
| [cmdcore-finite236](frozen236/cmdcore-finite236/receipt.json.gz) · [log](frozen236/cmdcore-finite236/tests.log) | Exact1-name command 1P/0F/176filtered after0.00s; complete final summary |

[Compiler-only pin](frozen236/pinned-compiler236.json) reports artifact fresh: `true`; [CmdCore pin](frozen236/pinned-cmdcore236.json) reports artifact fresh: `false`.

## Compiler237 build

The exact locked Compiler-only237 build fails101 after108.63 seconds with uniform_source: true. Its whole log records two unavailable source names and three private TaintSourceContext field accesses in the taint consumers. No assertions execute and no successful requested test image or pin is supplied. The exact gzip receipt preserves the complete unchanged source inventory and command; current edited definitions, other images and native outcomes remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build237](frozen237/compiler-build237/receipt.json.gz) · [log](frozen237/compiler-build237/tests.log) | Failed101; five taint integration errors; no assertions or successful pin |

## Runtime238 commands

The independently requested locked Runtime238 build passes after61.14 seconds with uniform_source: true. Its successful build receipt and complete immutable source inventory pin executable SHA711af5177269eec3631228d4219d29bda22e8e6bfd2d31e271180004a2c16cfa. The listing records1309 tests without executing assertions.

The exact114-name command completes114P/0F/0ignored/1195filtered after39.25 seconds with its complete final libtest summary. The selection has zero missing names and contains all107 names from the Runtime234 primary command, its six independently selected additional names, and the counted map-prefix comparator. Every selected name passes in this precise executable. The map comparator agrees with all20 unchanged original C8.5–9.1 public object-vector windows, including each stored counted head, whole map query and independent call result. The original whole36 child-alias comparison and Runtime55-window encoding constructor/storage/output/error comparison also pass. The CmdCore55-window codec has its independent Main236 receipt. VM map/storage comparisons require their own executable and actual receipts; this Runtime command supplies no VM result.

All three whole logs, exact original gzip receipts, original selection and executable pin retain their separate commands and one immutable source image. These software comparisons add no new native process, private provider identity, current edited-definition result or full-suite outcome. Other binaries/images, Clippy and Python remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [runtime-build238](frozen238/runtime-build238/receipt.json.gz) · [log](frozen238/runtime-build238/tests.log) | Requested Runtime buildPASS0 after61.14s; no assertions |
| [runtime-list238](frozen238/runtime-list238/receipt.json.gz) · [log](frozen238/runtime-list238/tests.log) | 1309 tests listed; no assertions |
| [runtime-finite238](frozen238/runtime-finite238/receipt.json.gz) · [log](frozen238/runtime-finite238/tests.log) | Exact114-name command114P/0F/1195filtered after39.25s; complete final summary |

[Exact executable pin](frozen238/pinned-runtime238.json) reports artifact fresh: `false`.

## Compiler239 build

The exact locked Compiler-only239 build fails101 after73.70 seconds with uniform_source: true. Its whole log records four unavailable CfgCommandClasses type names in the try-handler fixture and one incoming-argument fixture type mismatch. No assertions execute and no successful requested test image or pin is supplied. The exact gzip receipt preserves the complete unchanged source inventory and command; current edited definitions, other images and native outcomes remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-build239](frozen239/compiler-build239/receipt.json.gz) · [log](frozen239/compiler-build239/tests.log) | Failed101; five integration errors; no assertions or successful pin |

## Consumer240 check

The exact locked14-package all-targets check fails101 after187.53 seconds with uniform_source: true. Its whole log records one E0603 private source_structure import in the MCP datagroup consumer. The command executes no assertions and supplies no successful check or executable pin. Its exact gzip receipt preserves the complete unchanged source inventory and requested package list; current edited definitions, other images and native outcomes remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [consumer-check240](frozen240/consumer-check240/receipt.json.gz) · [log](frozen240/consumer-check240/tests.log) | Failed101; one MCP private import error; no assertions or successful check |

## Main241 commands

The locked Compiler/VM/Core build fails101 after379.68 seconds with five Core test compile errors. It executes no assertions and supplies no successful combined pin. The independently requested Compiler/VM build passes after151.30 seconds, with separate Compiler and VM executable/build/source associations. Their inventories list8571 and706 tests without assertions.

The exact28-name Compiler quick batch completes12P/16F after159.22 seconds. The independent234-name Compiler batch completes116P/118F after1066.62 seconds. The exact339-name VM batch completes273P/66F after1287.42 seconds. Each retains every actual event, the original zero-missing selection and a complete failing summary. Repeated selectors across commands remain repeated events; the archive supplies no aggregate pass or unique coverage count.

Five independent diagnostic commands also complete with failures: the two-name Logical declaration trace records0P/2F, while the VM coroutine, info-codegen, dictionary writeback and dictionary publication commands each record0P/1F. All twelve operations record uniform_source: true under the same complete immutable source snapshot. Successful separate builds and inventories do not convert assertion failures into a pass. Whole logs, exact gzip receipts, source inventories and original independent pins remain retained. Current edited definitions, other images, native provider outcomes, Clippy, Python and full suites retain their separate scopes.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-vm-core-build241](frozen241/compiler-vm-core-build241/receipt.json.gz) · [log](frozen241/compiler-vm-core-build241/tests.log) | Compiler/VM/Core buildFAIL101 after379.68s; five Core test compile errors; no assertions or successful combined pin |
| [compiler-vm-build241](frozen241/compiler-vm-build241/receipt.json.gz) · [log](frozen241/compiler-vm-build241/tests.log) | Independent Compiler/VM buildPASS0 after151.30s; no assertions |
| [compiler-list241](frozen241/compiler-list241/receipt.json.gz) · [log](frozen241/compiler-list241/tests.log) | 8571 tests listed; no assertions |
| [vm-list241](frozen241/vm-list241/receipt.json.gz) · [log](frozen241/vm-list241/tests.log) | 706 tests listed; no assertions |
| [compiler-quick241](frozen241/compiler-quick241/receipt.json.gz) · [log](frozen241/compiler-quick241/tests.log) | Exact28-name command 12P/16F/8543filtered after159.22s; complete failing summary |
| [logical-source-trace241](frozen241/logical-source-trace241/receipt.json.gz) · [log](frozen241/logical-source-trace241/tests.log) | Exact2-name command 0P/2F/8569filtered after0.38s; complete failing summary |
| [compiler-finite241](frozen241/compiler-finite241/receipt.json.gz) · [log](frozen241/compiler-finite241/tests.log) | Exact234-name command 116P/118F/8337filtered after1066.62s; complete failing summary |
| [vm-finite241](frozen241/vm-finite241/receipt.json.gz) · [log](frozen241/vm-finite241/tests.log) | Exact339-name command 273P/66F/367filtered after1287.42s; complete failing summary |
| [vm-native-codegen-trace241](frozen241/vm-native-codegen-trace241/receipt.json.gz) · [log](frozen241/vm-native-codegen-trace241/tests.log) | Exact1-name command 0P/1F/705filtered after4.54s; complete failing summary |
| [vm-info-commands-codegen241](frozen241/vm-info-commands-codegen241/receipt.json.gz) · [log](frozen241/vm-info-commands-codegen241/tests.log) | Exact1-name command 0P/1F/705filtered after9.21s; complete failing summary |
| [vm-dict-writeback-progress241](frozen241/vm-dict-writeback-progress241/receipt.json.gz) · [log](frozen241/vm-dict-writeback-progress241/tests.log) | Exact1-name command 0P/1F/705filtered after22.42s; complete failing summary |
| [vm-dict-publication-progress241](frozen241/vm-dict-publication-progress241/receipt.json.gz) · [log](frozen241/vm-dict-publication-progress241/tests.log) | Exact1-name command 0P/1F/705filtered after64.96s; complete failing summary |

[Compiler pin](frozen241/pinned-compiler241.json): SHA `4d5e04b249537c4b59fba2cd7f59b3b5d811582bdd7e4a9ffbb4171a5ab2f136`, artifact fresh: `false`. [VM pin](frozen241/pinned-vm241.json): SHA `d566a8e3ce05c2ec6834f1585470488df83f002530eb3e0d384f4f9aeb258c7b`, artifact fresh: `false`. Both belong to the independent successful Compiler/VM build.

## Consumer242 check

The exact locked14-package all-targets check fails101 after83.12 seconds with uniform_source: true. Its whole log records one E0716 temporary-context borrow in declaration_preview.rs and two E0308 SSA test-type errors. The command executes no assertions and supplies no successful check or executable pin. Its exact gzip receipt preserves the complete unchanged source inventory and requested package list; current edited definitions, other images and native outcomes remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [consumer-check242](frozen242/consumer-check242/receipt.json.gz) · [log](frozen242/consumer-check242/tests.log) | Failed101; three Compiler borrow/type errors; no assertions or successful check |

## Consumer244 check

The exact locked14-package all-targets check fails101 after86.95 seconds with uniform_source: true. Its whole log records three distinct Compiler errors: Option::flatten on a single metadata option in cfg_lookup_context.rs, a missing MAX_EXPR_NODE_DEPTH value in dataflow.rs and a ContextRegistry crate path in execution_regions.rs. The flatten failure is emitted for both library and library-test builds, preserving four diagnostic events. The command executes no assertions and supplies no successful check or executable pin. Its exact gzip receipt preserves the complete unchanged source inventory and requested package list; current edited definitions, other images and native outcomes remain independent.

| Receipt and log | Actual result |
| --- | --- |
| [consumer-check244](frozen244/consumer-check244/receipt.json.gz) · [log](frozen244/consumer-check244/tests.log) | Failed101; three distinct Compiler type/import errors; four diagnostic events; no assertions or successful check |
