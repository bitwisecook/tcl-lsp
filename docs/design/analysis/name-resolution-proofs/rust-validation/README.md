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

Twelve exact operations retain one complete immutable source inventory and `uniform_source: true`. The requested twelve-package library/test build exits101 after185.62 seconds: two DB `Module` initialisers omit `source_metadata_input`. It executes no assertions and does not complete the requested package build. Independent Runtime and Compiler-only builds pass after 54.48 and173.70 seconds, respectively; each rebuilt (`fresh: false`) executable is pinned to its actual successful build receipt and source inventory. Compiler and Runtime listing commands enumerate8420 and1249 tests without executing assertions.

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

## Main245 commands

The independent locked Compiler/VM build passes after 193.46 seconds and supplies two copied sealed executable/build/source associations. Their inventories list 8594 and 710 tests without assertions. The separately requested Core/consumer-library no-run build fails with exit 101 after 112.19 seconds with ENOSPC while creating compiler temporary directories; it supplies no Core pin, assertion or aggregate successful build.

The exact 63-name Compiler focus batch completes 21P/42F after 139.72 seconds. Five independent one-name diagnostic commands also complete with failures: original declaration-world traversal, original CFG binding replay, VM mathop codegen, dictionary writeback and dictionary publication each record 0P/1F. Whole logs preserve every actual event and the complete failing summaries. The declaration trace's old 7-count assertion fails with both retained traversals reporting 28; distinct original world observations are retained as software diagnostics, without a passing replacement assertion or elapsed-performance conclusion.

All ten operations record uniform_source: true under the same immutable source snapshot. The Compiler 63-name and VM 343-name selections retain exact zero-missing inventory associations; the VM 343-name selection has no finite batch execution receipt or coverage outcome. Repeated focus/trace selectors stay repeated events. Builds and inventories do not promote failures, current edited definitions or incoming integration to a pass. Native provider observations, other images, Clippy, Python and full suites keep their independent scopes.

| Receipt and log | Actual result |
| --- | --- |
| [compiler-vm-build245](frozen245/compiler-vm-build245/receipt.json.gz) · [log](frozen245/compiler-vm-build245/tests.log) | Independent Compiler/VM build PASS0 after 193.46s; no assertions |
| [compiler-list245](frozen245/compiler-list245/receipt.json.gz) · [log](frozen245/compiler-list245/tests.log) | 8594 tests listed; no assertions |
| [vm-list245](frozen245/vm-list245/receipt.json.gz) · [log](frozen245/vm-list245/tests.log) | 710 tests listed; no assertions |
| [compiler-focus245](frozen245/compiler-focus245/receipt.json.gz) · [log](frozen245/compiler-focus245/tests.log) | Exact 63-name command 21P/42F/8531filtered after 139.72s; complete failing summary |
| [compiler-declaration-world-trace245](frozen245/compiler-declaration-world-trace245/receipt.json.gz) · [log](frozen245/compiler-declaration-world-trace245/tests.log) | Exact 1-name command 0P/1F/8593filtered after 3.54s; complete failing summary |
| [vm-mathop-codegen245](frozen245/vm-mathop-codegen245/receipt.json.gz) · [log](frozen245/vm-mathop-codegen245/tests.log) | Exact 1-name command 0P/1F/709filtered after 8.05s; complete failing summary |
| [vm-dict-writeback-progress245](frozen245/vm-dict-writeback-progress245/receipt.json.gz) · [log](frozen245/vm-dict-writeback-progress245/tests.log) | Exact 1-name command 0P/1F/709filtered after 28.94s; complete failing summary |
| [vm-dict-publication-progress245](frozen245/vm-dict-publication-progress245/receipt.json.gz) · [log](frozen245/vm-dict-publication-progress245/tests.log) | Exact 1-name command 0P/1F/709filtered after 73.12s; complete failing summary |
| [consumer-libs-build245](frozen245/consumer-libs-build245/receipt.json.gz) · [log](frozen245/consumer-libs-build245/tests.log) | Core/consumer-library no-run build FAIL101 after 112.19s; ENOSPC; no assertions or Core pin |
| [compiler-cfg-replay-trace245](frozen245/compiler-cfg-replay-trace245/receipt.json.gz) · [log](frozen245/compiler-cfg-replay-trace245/tests.log) | Exact 1-name command 0P/1F/8593filtered after 2.60s; complete failing summary |

[Compiler pin](frozen245/pinned-compiler245.json): SHA `445956a54f2a27d1e0ab65c6a44d737459f8381ffdd906dfd47b3e1a851da622`, artifact fresh: `false`. [VM pin](frozen245/pinned-vm245.json): SHA `66c9668ecf0d1dfac418ece49da8e917fcbe5aa7fb0ad19843a29e75f7f4440b`, artifact fresh: `false`. Both belong solely to the independent successful Compiler/VM build.

## Registry library checks246–254

Nine exact `cargo check --locked --offline -p tcl-registry --lib` commands retain their own immutable source inventories and full logs. Each closes with exit101 and `uniform_source: true`. The recorded failures are manifest loading, locked dependency resolution or Rust compilation refusals. These library checks execute no test assertions and supply no test executable pin; they are neither Native provider failures nor a successful validation gate.

Every original receipt is retained as a lossless gzip payload with both compressed and uncompressed SHA256. Each immutable archive leaf is independently checked against that receipt's source map. Command paths, original source roots, timings and complete diagnostics remain unchanged. Ledger entries retain no invented test result, successful binary or aggregate pass.

| Receipt and log | Actual result |
| --- | --- |
| [integration-registry-check246](frozen246/integration-registry-check246/receipt.json.gz) · [log](frozen246/integration-registry-check246/tests.log) | Exit101 after 0.074932s; Duplicate workspace manifest key; manifest loading refused. No assertions or executable pin. |
| [integration-registry-check247](frozen247/integration-registry-check247/receipt.json.gz) · [log](frozen247/integration-registry-check247/tests.log) | Exit101 after 0.262853s; Locked dependency resolution refuses the required lockfile update. No assertions or executable pin. |
| [integration-registry-check248](frozen248/integration-registry-check248/receipt.json.gz) · [log](frozen248/integration-registry-check248/tests.log) | Exit101 after 4.177036s; Four tcl-dialect compile errors. No assertions or executable pin. |
| [integration-registry-check249](frozen249/integration-registry-check249/receipt.json.gz) · [log](frozen249/integration-registry-check249/tests.log) | Exit101 after 0.985796s; PackageFloor lacks the required Hash implementation. No assertions or executable pin. |
| [integration-registry-check250](frozen250/integration-registry-check250/receipt.json.gz) · [log](frozen250/integration-registry-check250/tests.log) | Exit101 after 6.075160s; Eleven tcl-cmd-core compile errors. No assertions or executable pin. |
| [integration-registry-check251](frozen251/integration-registry-check251/receipt.json.gz) · [log](frozen251/integration-registry-check251/tests.log) | Exit101 after 4.935033s; Two tcl-regex NO_MATCH symbol errors. No assertions or executable pin. |
| [integration-registry-check252](frozen252/integration-registry-check252/receipt.json.gz) · [log](frozen252/integration-registry-check252/tests.log) | Exit101 after 21.496689s; Sixty-five tcl-registry compile errors. No assertions or executable pin. |
| [integration-registry-check253](frozen253/integration-registry-check253/receipt.json.gz) · [log](frozen253/integration-registry-check253/tests.log) | Exit101 after 33.230326s; Eleven tcl-registry compile errors. No assertions or executable pin. |
| [integration-registry-check254](frozen254/integration-registry-check254/receipt.json.gz) · [log](frozen254/integration-registry-check254/tests.log) | Exit101 after 18.352784s; One non-exhaustive TemplateWordPlan source match for WordPart::Expression. No assertions or executable pin. |

## Compiler library check255

The exact `cargo check --locked --offline -p tcl-compiler --lib` command closes with exit101 after 29.333715s and `uniform_source: true`. The full log records 82 Compiler compilation errors and 14 warnings. Dependency compilation provides no separate assertion result. This library check executes no test assertions and supplies no test executable pin or aggregate pass; it is not a Native provider failure.

The [original receipt](frozen255/integration-compiler-check255/receipt.json.gz) is retained as a lossless gzip payload with compressed and uncompressed SHA256. The [complete log](frozen255/integration-compiler-check255/tests.log) preserves all diagnostics. Every immutable source archive leaf is independently checked against the original receipt map. Original command, paths, source association and timing remain unchanged.

## Compiler library check256

The exact `cargo check --locked --offline -p tcl-compiler --lib` command closes with exit101 after 31.101201s and `uniform_source: true`. The full log records 71 Compiler compilation errors and 12 warnings. Dependency compilation provides no separate assertion result. This library check executes no test assertions and supplies no test executable pin or aggregate pass; it is not a Native provider failure.

The [original receipt](frozen256/integration-compiler-check256/receipt.json.gz) is retained as a lossless gzip payload with compressed and uncompressed SHA256. The [complete log](frozen256/integration-compiler-check256/tests.log) preserves all diagnostics. Every immutable source archive leaf is independently checked against the original receipt map. Original command, paths, source association and timing remain unchanged.

## Compiler and VM library checks257 and258

These three closed commands retain separate outcomes with `uniform_source: true`. The first257 command repeats `--lib`; Cargo rejects it with exit1 after 0.039263s before compilation. The corrected257 command closes with exit101 after 15.280502s at one Registry dependency type error. Check258 closes with exit101 after 42.642274s, recording 100 Compiler errors and 12 warnings and 29 VM errors and 4 warnings. None executes test assertions, supplies a test executable pin or establishes an aggregate pass or Native provider failure.

| Exact closed command | Original receipt | Complete log | Outcome |
| --- | --- | --- | --- |
| Rejected257 argument vector | [receipt](frozen257/integration-compiler-check257/receipt.json.gz) | [log](frozen257/integration-compiler-check257/tests.log) | Command blocked before compilation |
| Compiler and VM check257 | [receipt](frozen257/integration-compiler-vm-check257/receipt.json.gz) | [log](frozen257/integration-compiler-vm-check257/tests.log) | Registry dependency compilation blocked |
| Compiler and VM check258 | [receipt](frozen258/integration-compiler-vm-check258/receipt.json.gz) | [log](frozen258/integration-compiler-vm-check258/tests.log) | Compiler and VM compilation blocked |

Each exact receipt is retained as a lossless gzip payload with compressed and uncompressed SHA256. Every immutable source archive leaf is independently checked against the original receipt map. Original commands, paths, source associations, diagnostic streams and timings remain unchanged.

## Registry test-build dependency check259

The exact `cargo test --locked --offline -p tcl-registry --lib --no-run --message-format=json` command closes with exit101 after 67.030690s and `uniform_source: true`. The full log records 59 Compiler compilation errors and 12 warnings, plus one VM compilation error and four warnings. Development-dependency compilation blocks Registry libtest construction. This no-run build executes no test assertions and supplies no test executable pin or aggregate pass; it is not a Native provider failure.

The [original receipt](frozen259/integration-registry-test-build259/receipt.json.gz) is retained as a lossless gzip payload with compressed and uncompressed SHA256. The [complete log](frozen259/integration-registry-test-build259/tests.log) preserves all diagnostics. Every immutable source archive leaf is independently checked against the original receipt map. Original command, paths, source association and timing remain unchanged.

## Compiler and VM library check260

The exact `cargo check --locked --offline --target-dir /workspace/.targets/naming-upstream-clean -p tcl-compiler -p tcl-vm --lib --message-format=json` command closes with exit101 after 53.884576s and `uniform_source: true`. The full log records eight Compiler compilation errors and 12 warnings. The VM library compiles with nine warnings. The independently compiled VM library supplies no test assertion or sealed test executable pin. This combined library check executes no test assertions and supplies no test executable pin or aggregate pass; it is not a Native provider failure.

The [original receipt](frozen260/integration-compiler-vm-check260/receipt.json.gz) is retained as a lossless gzip payload with compressed and uncompressed SHA256. The [complete log](frozen260/integration-compiler-vm-check260/tests.log) preserves all diagnostics. Every immutable source archive leaf is independently checked against the original receipt map. Original command, paths, source association and timing remain unchanged.

## Library compilation and no-run test build261

The exact `cargo check --locked --offline --target-dir /workspace/.targets/naming-upstream-clean -p tcl-compiler -p tcl-vm --lib --message-format=json` command closes with exit0 after 40.344003s and `uniform_source: true`. The Compiler and VM libraries compile with warnings. Both library artifact records have executable:null and test:false. This library check executes no test assertions and supplies no test executable pin, full validation-gate pass or Native provider result.

The [original receipt](frozen261/integration-compiler-vm-check261/receipt.json.gz) is retained as a lossless gzip payload with compressed and uncompressed SHA256. The [complete log](frozen261/integration-compiler-vm-check261/tests.log) preserves all diagnostics. Every immutable source archive leaf is independently checked against the original receipt map. Original command, paths, source association and timing remain unchanged.

The separate `cargo test --locked --offline --target-dir /workspace/.targets/naming-upstream-clean -p tcl-registry -p tcl-compiler -p tcl-vm --lib --no-run --message-format=json` command closes with exit101 after 68.049132s and `uniform_source: true`. Six TclError field errors in the tcl-engine-tclvm development dependency block test construction. Ordinary Compiler library compilation supplies no unit-test result; test inventory is not reached. Its [exact original receipt](frozen261/integration-registry-compiler-vm-test-build261/receipt.json.gz) and [complete log](frozen261/integration-registry-compiler-vm-test-build261/tests.log) retain this independent outcome with no test pin or Native claim.

## Native-consumer builds262/263 and CmdCore263 tests

The locked, offline no-run builds for Registry, Compiler, VM, TclVM engine and CmdCore both close with exit101 and `uniform_source: true`. Build262 fails at three CmdCore test-fixture compile errors after 32.923354s and produces no test executable. Build263 fails at two SpecHooks dependency compile errors after 88.003873s. Its complete log independently contains one successful CmdCore libtest artifact with `fresh: false`; the failed aggregate build supplies no successful executable association for another crate.

The [copied original partial-artifact pin record](frozen263/pinned-cmdcore263-partial.json) retains its exact build-receipt and source-snapshot hashes. Its sealed executable SHA256 is `2726d7b241fe8968561791ec9df14daea5a3b1d2a76d9e027943e3c29034bde5`; the original executable bytes are independently checked against that pin. The separate inventory lists 190 tests without executing assertions. The [exact full selection](frozen263/cmdcore-full-selection263.json) matches every inventory name, the pin and the list hash, with zero missing selectors.

The complete single-threaded CmdCore263 test command closes after 0.209579s with 189 passes and one failure, zero ignored and zero filtered tests. The failing selector is `native_info_level::tests::native_level_integer_error_retains_primitive_fields_and_string_producer`. Its original panic and summary remain in the full log. The current tab-delimiter repair has no execution outcome under these receipts and does not change their failure. A valid partial artifact and 189 passing assertions do not imply aggregate build success, successful VM comparators or a Native provider outcome.

All four original receipts retain their commands, timings and complete immutable source maps as lossless gzip payloads with compressed and uncompressed SHA256. Both source images are independently checked byte for byte; the three 263 operations share the exact same full source association. Other images, current definitions, external Native observations and full validation gates retain their own evidence.

| Receipt and log | Actual result |
| --- | --- |
| [No-run build262](frozen262/integration-native-consumer-test-build262/receipt.json.gz) · [log](frozen262/integration-native-consumer-test-build262/tests.log) | Exit101; three CmdCore fixture compile errors; no test artifact |
| [No-run build263](frozen263/integration-native-consumer-test-build263/receipt.json.gz) · [log](frozen263/integration-native-consumer-test-build263/tests.log) | Exit101; two SpecHooks dependency compile errors; one independently successful CmdCore libtest artifact |
| [CmdCore263 inventory](frozen263/integration-cmdcore-inventory263/receipt.json.gz) · [log](frozen263/integration-cmdcore-inventory263/tests.log) | Exit0; 190 tests listed; no assertions |
| [CmdCore263 full test run](frozen263/integration-cmdcore-full-tests263/receipt.json.gz) · [log](frozen263/integration-cmdcore-full-tests263/tests.log) | Exit101; 189 passed; one failed; original complete 190-test outcome |

## Native-consumer build264 and independent CmdCore264 tests

The locked, offline aggregate no-run build for Registry, Compiler, VM, TclVM engine and CmdCore closes with exit101 after 54.854262s and `uniform_source: true`. Two ordinary Compiler errors at word_subst.rs:177 and 262 concern retained SourceImage transport. The full log also records one CmdCore libtest artifact with `fresh: false`; that aggregate artifact has no sealed pin or assertion outcome assigned here. The failed aggregate build establishes no successful engine/spec construction or unit-test result.

The separate locked, offline CmdCore-only no-run build succeeds after 6.858042s and emits a different CmdCore libtest artifact with `fresh: false`. Its [exact original pin record](frozen264/pinned-cmdcore264.json) retains the independent successful build-receipt and immutable source-snapshot association. The sealed executable bytes independently match SHA256 `06476c17b6ca7d90b7e76818cf19119098e322e965f4d61e1e72b2ebdc44535e`; the pin belongs solely to this CmdCore-only build.

The separate current inventory lists 190 deterministic tests without assertions. The [unchanged full selection](frozen264/cmdcore-full-selection264.json) retains its exact inventory receipt/hash and all 190 selectors, matching every actual test event. The full single-threaded CmdCore264 run closes with exit0 after 0.247871s: 190 passed, zero failed, ignored or filtered. This outcome applies only to that original pin and frozen source. Earlier 263 failures remain unchanged, and this CmdCore run establishes no aggregate build pass, VM comparator result or external Native provider outcome.

All four operations have `uniform_source: true` and retain the same full immutable source association. The original receipts are lossless gzip payloads with compressed and uncompressed SHA256; every archived source leaf is independently byte-checked. Commands, timings, summaries and complete logs retain their original bytes.

| Receipt and log | Actual result |
| --- | --- |
| [Aggregate no-run build264](frozen264/integration-native-consumer-test-build264/receipt.json.gz) · [log](frozen264/integration-native-consumer-test-build264/tests.log) | Exit101; two ordinary Compiler SourceImage errors; one unpinned CmdCore artifact; no assertions |
| [Independent CmdCore-only build264](frozen264/integration-cmdcore-test-build264/receipt.json.gz) · [log](frozen264/integration-cmdcore-test-build264/tests.log) | Exit0; independently sealed CmdCore libtest artifact; no assertions |
| [Current CmdCore264 inventory](frozen264/integration-cmdcore-current-inventory264/receipt.json.gz) · [log](frozen264/integration-cmdcore-current-inventory264/tests.log) | Exit0; 190 tests listed; no assertions |
| [Full CmdCore264 test run](frozen264/integration-cmdcore-full-tests264/receipt.json.gz) · [log](frozen264/integration-cmdcore-full-tests264/tests.log) | Exit0; 190 passed; zero failed, ignored or filtered |

The separately retained [zero-byte legacy inventory log](frozen264/unclassified-original-log/integration-cmdcore-inventory264.log) has no original JSON receipt. It contains no inventory, command association, status or assertion evidence and is not a fifth validation operation.

## Five-crate no-run unit build265

The exact locked, offline Registry/Compiler/VM/Engine/CmdCore no-run unit build closes with exit101 after 73.926729s and `uniform_source: true`. Two ordinary Compiler library errors block construction: the inlining command-substitution helper call and the Registry semantic-key comparison. The full original log retains one CmdCore libtest executable artifact, marked `fresh: false`; this operation never runs or seals that artifact as a validation pin. There is no test inventory, assertion result, aggregate pass, VM/Engine execution or Native provider result.

Its [exact original receipt](frozen265/integration-native-consumer-test-build265/receipt.json.gz) is retained as lossless gzip with compressed and uncompressed SHA256. The [complete log](frozen265/integration-native-consumer-test-build265/tests.log) preserves all diagnostics and the unrun artifact record. Every archived source leaf is independently checked against the original receipt inventory. Commands, paths, source association, timing and failure remain exact.

## No-run consumer builds266 and267

The three independent locked, offline no-run unit builds close with exit101 and `uniform_source: true`. The broader eight-crate naming build266 takes 107.134101s; the separate five-crate native-consumer build266 takes 64.327595s. Both preserve twenty ordinary Core library errors. The five-crate build267 takes 139.981644s and preserves one ordinary Core E0451 error in an AnalysisResult test fixture constructor. Each log retains ordinary Compiler/VM library artifacts and one distinct CmdCore libtest artifact; none of these operations runs or seals that test artifact as a validation pin. There is no test inventory, unit assertion, aggregate pass, tested Compiler/VM/Engine artifact or Native provider outcome.

- `integration-naming-consumer-test-build266`: [exact receipt](frozen266/integration-naming-consumer-test-build266/receipt.json.gz), [complete log](frozen266/integration-naming-consumer-test-build266/tests.log). Exit101; 107.134101s; 20 ordinary Core errors; one unrun CmdCore libtest artifact marked `fresh: false`.
- `integration-native-consumer-test-build266`: [exact receipt](frozen266/integration-native-consumer-test-build266/receipt.json.gz), [complete log](frozen266/integration-native-consumer-test-build266/tests.log). Exit101; 64.327595s; 20 ordinary Core errors; one unrun CmdCore libtest artifact marked `fresh: true`.
- `integration-native-consumer-test-build267`: [exact receipt](frozen267/integration-native-consumer-test-build267/receipt.json.gz), [complete log](frozen267/integration-native-consumer-test-build267/tests.log). Exit101; 139.981644s; 1 ordinary Core errors; one unrun CmdCore libtest artifact marked `fresh: false`.

Each receipt is a lossless gzip payload with compressed and uncompressed SHA256. Every immutable archived source leaf is independently byte-checked; both266 commands retain the exact same source inventory. Original commands, source association, paths, timing, diagnostics and unrun artifact records remain unchanged.

## VM no-run unit build268

The exact locked, offline VM library no-run unit build closes with exit101 after 144.900876s and `uniform_source: true`. Two libtest compilation errors block construction: stale `brace_safe`/`quote_for_script` imports in exec.rs and the old InterpState `dialect_profile` field in interp.rs. It emits no libtest executable and never reaches test inventory or assertions. There is no sealed pin, compiled test pass, aggregate gate or Native provider result.

Its [exact original receipt](frozen268/integration-vm-native-test-build268/receipt.json.gz) is retained as lossless gzip with compressed and uncompressed SHA256. The [complete log](frozen268/integration-vm-native-test-build268/tests.log) preserves every diagnostic. Every immutable source archive leaf is independently checked against the receipt inventory; original commands, paths, source association and timing remain exact.

## Sealed VM269 and compiler dependency checks270–271

The independent VM-only269 no-run build succeeds and produces the [sealed VM269 image](frozen269/sealed-vm-image/pinned-vm269.elf), with its [exact producer/source pin](frozen269/sealed-vm-image/pinned-vm269.json). Its inventory lists728 tests. The separate six-selector focus run records2 passed/4 failed; its five software owner controls pass. The Compiler-only269 build fails at a Core libtest dependency error and supplies no Compiler image. These are per-command outcomes, without an aggregate pass or current-source/provider promotion.

Both270 and271 aggregate no-run builds fail during dependency/libtest compilation and produce no sealed test image. Their logs retain explicit successful tcl_syntax and tcl_engine_tclvm test artifacts, which remain unpinned and unexecuted here. The separate270 three-selector and271 one-selector diagnostics execute the older sealed VM269 ELF against the explicitly recorded current270/271 source association and fail0/3 and0/1 respectively. Their `uniform_source: true` belongs to each recorded current source; it does not make the older ELF a270/271 build. Every original receipt, command, full log, signal/error/trace detail and frozen source association remains exact.

| Closed operation | Status | Recorded result | Exact receipt | Complete log |
| --- | --- | --- | --- | --- |
| `integration-vm-native-test-build269` | `compile-passed`, exit0, 107.085306s | no assertions | [Original lossless receipt](frozen269/integration-vm-native-test-build269/receipt.json.gz) | [Whole log](frozen269/integration-vm-native-test-build269/tests.log) |
| `integration-compiler-native-test-build269` | `compile-blocked`, exit101, 114.857339s | no assertions | [Original lossless receipt](frozen269/integration-compiler-native-test-build269/receipt.json.gz) | [Whole log](frozen269/integration-compiler-native-test-build269/tests.log) |
| `integration-vm-current-inventory269` | `listed`, exit0, 0.007214s | 728 listed, no assertions | [Original lossless receipt](frozen269/integration-vm-current-inventory269/receipt.json.gz) | [Whole log](frozen269/integration-vm-current-inventory269/tests.log) |
| `integration-vm-native-focus-tests269` | `failed`, exit101, 150.905162s | 2 passed/4 failed | [Original lossless receipt](frozen269/integration-vm-native-focus-tests269/receipt.json.gz) | [Whole log](frozen269/integration-vm-native-focus-tests269/tests.log) |
| `integration-vm-current-owner-tests269` | `passed`, exit0, 5.586860s | 5 passed/0 failed | [Original lossless receipt](frozen269/integration-vm-current-owner-tests269/receipt.json.gz) | [Whole log](frozen269/integration-vm-current-owner-tests269/tests.log) |
| `integration-compiler-consumer-test-build270` | `compile-blocked`, exit101, 109.046702s | no assertions | [Original lossless receipt](frozen270/integration-compiler-consumer-test-build270/receipt.json.gz) | [Whole log](frozen270/integration-compiler-consumer-test-build270/tests.log) |
| `diagnostic-pinned269-native-frontiers-on270` | `failed`, exit101, 61.761671s | 0 passed/3 failed | [Original lossless receipt](frozen270/diagnostic-pinned269-native-frontiers-on270/receipt.json.gz) | [Whole log](frozen270/diagnostic-pinned269-native-frontiers-on270/tests.log) |
| `integration-compiler-consumer-test-build271` | `compile-blocked`, exit101, 202.927960s | no assertions | [Original lossless receipt](frozen271/integration-compiler-consumer-test-build271/receipt.json.gz) | [Whole log](frozen271/integration-compiler-consumer-test-build271/tests.log) |
| `diagnostic-pinned269-dictionary345-backtrace-on271` | `failed`, exit101, 70.441658s | 0 passed/1 failed | [Original lossless receipt](frozen271/diagnostic-pinned269-dictionary345-backtrace-on271/receipt.json.gz) | [Whole log](frozen271/diagnostic-pinned269-dictionary345-backtrace-on271/tests.log) |

The [six-selector focus selection](frozen269/selections/vm-native-focus-selection269.json) and [five-selector owner selection](frozen269/selections/vm-owner-selection269.json) are byte-identical originals. Every selected name is verified against the728-test inventory and actual libtest events. Compressed receipts retain both compressed and uncompressed digests; every distinct archived source leaf is byte-checked. No new Rust build, test, Native replay or mutable source-anchor refresh is performed by this archive.

## Naming272 command selection and workspace compilation

The all-consumer command is blocked before compilation by a Runtime feature that belongs outside the root workspace selection. The separate workspace-only no-run command reaches one ordinary Compiler E0308 producer error. It records a successful Syntax libtest artifact, which remains unsealed and unexecuted; neither command reaches assertions or issues a Compiler/VM image or aggregate pass. Each original receipt retains its complete unchanged naming272 source association, command and elapsed time.

| Closed operation | Status | Exact receipt | Complete log |
| --- | --- | --- | --- |
| `integration-all-naming-consumer-test-build272` | `command-blocked`, exit101, 0.065593s; no assertions | [Lossless original](frozen272/integration-all-naming-consumer-test-build272/receipt.json.gz) | [Whole log](frozen272/integration-all-naming-consumer-test-build272/tests.log) |
| `integration-workspace-naming-consumer-test-build272` | `compile-blocked`, exit101, 140.167522s; no assertions | [Lossless original](frozen272/integration-workspace-naming-consumer-test-build272/receipt.json.gz) | [Whole log](frozen272/integration-workspace-naming-consumer-test-build272/tests.log) |

Compressed receipts round-trip byte-for-byte and retain both compressed and original digests. Every recorded archived source leaf is checked independently. These exact per-command outcomes do not refresh mutable implementation anchors or change Native provider evidence.

## Sealed VM273 and independent dependency builds

The independent VM-only no-run273 build succeeds and yields the [lossless storage of the sealed actual VM273 image](frozen273/sealed-vm-image/pinned-vm273.elf.gz) with its [exact producer/source metadata](frozen273/sealed-vm-image/pinned-vm273.json). Its inventory lists735 tests. The list-gated23-selector frontier run records19 passed/4 failed; the separate dictionary-publication backtrace records0 passed/1 failed. These are actual assertions from this exact source/image, without an aggregate pass, other-backend result or Native provider promotion.

The separate workspace-only build reaches two ordinary DB tuple Eq/Hash macro errors; the independent Compiler/Registry/Syntax/Engine build reaches five Compiler libtest fixture errors. Both retain successful but unsealed/unexecuted test-artifact events in their full logs. Their failure does not invalidate the separately produced VM image or issue another executable/pin/assertion result. Each exact original command and frozen naming273 source association remains unchanged.

| Closed operation | Status | Recorded result | Exact receipt | Complete log |
| --- | --- | --- | --- | --- |
| `integration-vm-native-test-build273` | `compile-passed`, exit0, 113.245286s | no assertions | [Original lossless receipt](frozen273/integration-vm-native-test-build273/receipt.json.gz) | [Whole log](frozen273/integration-vm-native-test-build273/tests.log) |
| `integration-vm-current-inventory273` | `listed`, exit0, 0.012205s | 735 listed, no assertions | [Original lossless receipt](frozen273/integration-vm-current-inventory273/receipt.json.gz) | [Whole log](frozen273/integration-vm-current-inventory273/tests.log) |
| `integration-vm-native-frontier-tests273` | `failed`, exit101, 140.928231s | 19 passed/4 failed | [Original lossless receipt](frozen273/integration-vm-native-frontier-tests273/receipt.json.gz) | [Whole log](frozen273/integration-vm-native-frontier-tests273/tests.log) |
| `integration-workspace-naming-consumer-test-build273` | `compile-blocked`, exit101, 102.644712s | no assertions | [Original lossless receipt](frozen273/integration-workspace-naming-consumer-test-build273/receipt.json.gz) | [Whole log](frozen273/integration-workspace-naming-consumer-test-build273/tests.log) |
| `integration-compiler-registry-consumer-test-build273` | `compile-blocked`, exit101, 263.125773s | no assertions | [Original lossless receipt](frozen273/integration-compiler-registry-consumer-test-build273/receipt.json.gz) | [Whole log](frozen273/integration-compiler-registry-consumer-test-build273/tests.log) |
| `diagnostic-native273-dictionary345-backtrace` | `failed`, exit101, 85.483665s | 0 passed/1 failed | [Original lossless receipt](frozen273/diagnostic-native273-dictionary345-backtrace/receipt.json.gz) | [Whole log](frozen273/diagnostic-native273-dictionary345-backtrace/tests.log) |

The [23-selector frontier selection](frozen273/selections/vm-native-frontier-selection273.json) is the byte-identical original; every requested name is verified against the735-test inventory and actual libtest events. The exact four failed selectors remain visible, including both mathop source controls and the329/345 dictionary comparisons. Passing owned-storage, byte-carrier, reached error capture/refusal, typed first-cause, active-manifest and other software controls do not promote external Native evidence. Every receipt compresses losslessly with both digests retained; every frozen source leaf and the sealed image producer/hash association are independently checked. No build, test or Native replay is launched by this archive.


The [image storage receipt](frozen273/sealed-vm-image/lossless-image-storage.json) retains both stored gzip and original measured ELF digests/lengths. The original producer metadata is unchanged. Restore a local executable from the repository root only after checking both identities:

```python
from pathlib import Path
import gzip, hashlib, json
base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen273/sealed-vm-image")
record = json.loads((base / "lossless-image-storage.json").read_text())
packed = (base / "pinned-vm273.elf.gz").read_bytes()
assert len(packed) == record["stored_bytes"]
assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
original = gzip.decompress(packed)
assert len(original) == record["original_bytes"]
assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
restored = Path("/tmp/pinned-vm273.elf")
restored.write_bytes(original)
restored.chmod(0o755)
```

Decompression restores the measured executable byte-for-byte; it is neither a new build nor an assertion or Native replay.

## Independent naming274 unit construction

The VM-only no-run build closes with two E0599 fixture errors caused by the missing ValueOps import. The independent Compiler/Registry/Syntax/Engine no-run build closes with an expr_simplify E0308 fixture ownership mismatch and a manager E0609 nonexistent Optimisation.source_context field. Neither invocation executes assertions, reaches test inventory or supplies a VM/Compiler pin. The second log retains actual successful Syntax/Registry/Engine libtest artifacts, which remain unsealed and unexecuted. Complete original command/source associations and failures remain separate from later source repairs, other binaries and Native provider evidence.

| Closed operation | Status | Exact receipt | Complete log |
| --- | --- | --- | --- |
| `integration-vm-native-test-build274` | `compile-blocked`, exit101, 183.554294s; no assertions | [Lossless original](frozen274/integration-vm-native-test-build274/receipt.json.gz) | [Whole log](frozen274/integration-vm-native-test-build274/tests.log) |
| `integration-compiler-registry-consumer-test-build274` | `compile-blocked`, exit101, 323.043956s; no assertions | [Lossless original](frozen274/integration-compiler-registry-consumer-test-build274/receipt.json.gz) | [Whole log](frozen274/integration-compiler-registry-consumer-test-build274/tests.log) |

Both original receipts have uniform_source=true, lossless gzip round trips and independently checked complete frozen source inventories. Stored and original receipt digests are retained; no mutable implementation pin, actual Native capture or provider answer is changed by this archive.

## Sealed VM275 and independent unit construction

The independent VM-only no-run275 build succeeds and yields the [lossless storage of the sealed actual VM275 image](frozen275/sealed-vm-image/pinned-vm275.elf.gz), [exact producer metadata](frozen275/sealed-vm-image/pinned-vm275.json) and [unchanged original source companion](frozen275/sealed-vm-image/source-snapshot.json.gz). Its inventory lists 737 tests. The list-gated 24-selector frontier records 21 passed/3 failed. The separate dictionary ownership diagnostic records 0 passed/1 failed. These actual software outcomes apply to this exact source/image; they supply no aggregate gate pass, other-backend result or external Native provider promotion.

The four-crate Compiler/Registry/Syntax/Engine no-run build stops at four Registry libtest E0599 calls to native_command_admission. Its successful Syntax/Engine/Compiler artifact events remain in the complete log, without a sealed image or executed assertion from that failed invocation. The independent Compiler-only no-run build succeeds and emits its actual Compiler test artifact; this archive records that successful producer without a Compiler pin, inventory or assertion result. Each original command and full frozen naming275 source association remains unchanged.

| Closed operation | Status | Recorded result | Exact receipt | Complete log |
| --- | --- | --- | --- | --- |
| `integration-vm-native-test-build275` | `compile-passed`, exit 0, 106.916094s | no assertions | [Original lossless receipt](frozen275/integration-vm-native-test-build275/receipt.json.gz) | [Whole log](frozen275/integration-vm-native-test-build275/tests.log) |
| `integration-vm-current-inventory275` | `listed`, exit 0, 0.006932s | 737 listed, no assertions | [Original lossless receipt](frozen275/integration-vm-current-inventory275/receipt.json.gz) | [Whole log](frozen275/integration-vm-current-inventory275/tests.log) |
| `integration-vm-native-frontier-tests275` | `failed`, exit 101, 165.038087s | 21 passed/3 failed | [Original lossless receipt](frozen275/integration-vm-native-frontier-tests275/receipt.json.gz) | [Whole log](frozen275/integration-vm-native-frontier-tests275/tests.log) |
| `integration-compiler-registry-consumer-test-build275` | `compile-blocked`, exit 101, 266.883495s | no assertions | [Original lossless receipt](frozen275/integration-compiler-registry-consumer-test-build275/receipt.json.gz) | [Whole log](frozen275/integration-compiler-registry-consumer-test-build275/tests.log) |
| `integration-compiler-own-test-build275` | `compile-passed`, exit 0, 156.843618s | no assertions | [Original lossless receipt](frozen275/integration-compiler-own-test-build275/receipt.json.gz) | [Whole log](frozen275/integration-compiler-own-test-build275/tests.log) |
| `diagnostic-native275-dictionary345-ownership` | `failed`, exit 101, 67.485009s | 0 passed/1 failed | [Original lossless receipt](frozen275/diagnostic-native275-dictionary345-ownership/receipt.json.gz) | [Whole log](frozen275/diagnostic-native275-dictionary345-ownership/tests.log) |

The [24-selector frontier selection](frozen275/selections/vm-native-frontier-selection275.json) remains the byte-identical original, checked against the 737-test inventory and actual libtest events. Its three failures remain both mathop controls at their C8.4 preflight refusal and the 186-window InfoCommands comparison at the original C8.4 qualified child String birth; that failure is separate from outer List storage. Passing dictionary observation windows, independent live host-storage, reached error capture/refusal, typed first-cause, active-manifest and other software controls do not grant defined native post-free contents, external provider equivalence, another image or an aggregate success. The dictionary ownership diagnostic retains the genuine retired CALL-child refusal in its whole log; its failure is not converted to a process comparison pass. Every receipt compresses losslessly with both digests retained. Every frozen source leaf and the sealed VM producer/hash association are independently checked. This archive launches no build, assertion or Native replay.

The [image storage receipt](frozen275/sealed-vm-image/lossless-image-storage.json) retains stored gzip and original measured ELF digests/lengths. The original producer and source metadata remain unchanged. From the repository root, restore only after checking both identities:

```python
from pathlib import Path
import gzip, hashlib, json
base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen275/sealed-vm-image")
record = json.loads((base / "lossless-image-storage.json").read_text())
packed = (base / "pinned-vm275.elf.gz").read_bytes()
assert len(packed) == record["stored_bytes"]
assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
original = gzip.decompress(packed)
assert len(original) == record["original_bytes"]
assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
restored = Path("/tmp/pinned-vm275.elf")
restored.write_bytes(original)
restored.chmod(0o755)
```

Decompression restores the measured executable byte-for-byte. It is neither a new build nor an assertion or Native replay.

## Sealed Compiler275 source and assertion receipts

The [original pin metadata](frozen275/sealed-compiler-image/pinned-compiler275.json), [lossless measured Compiler ELF](frozen275/sealed-compiler-image/pinned-compiler275.elf.gz) and [storage identity receipt](frozen275/sealed-compiler-image/lossless-image-storage.json) retain the independently successful Compiler-only275 producer already recorded in the ledger. This image is not attributed to the failed four-crate invocation. Its [source companion](frozen275/sealed-vm-image/source-snapshot.json.gz) is byte-identical to the unchanged source association independently checked for the successful Compiler-only producer. Sharing that exact companion does not make Compiler and VM executable identities equivalent.

The inventory lists9067 tests. The [original 114-selector selection](frozen275/selections/compiler-focus-selection275.json) records47 passed/67 failed. Every failure remains in the whole log. The declaration trace requests two selectors, but the requested source_command_world Unicode-transfer name is absent from that inventory: only the declaration test actually executes and fails. The corrected logical_operation Unicode-transfer selector executes independently and fails in its own command. These actual outcomes provide neither aggregate success nor another image/backend or Native provider result.

| Closed operation | Status | Recorded result | Exact receipt | Complete log |
| --- | --- | --- | --- | --- |
| `integration-compiler-current-inventory275` | `listed`, exit 0, 0.056978s | 9067 listed, no assertions | [Original lossless receipt](frozen275/integration-compiler-current-inventory275/receipt.json.gz) | [Whole log](frozen275/integration-compiler-current-inventory275/tests.log) |
| `integration-compiler-focused-consumers275` | `failed`, exit 101, 156.230215s | 47 passed/67 failed | [Original lossless receipt](frozen275/integration-compiler-focused-consumers275/receipt.json.gz) | [Whole log](frozen275/integration-compiler-focused-consumers275/tests.log) |
| `diagnostic-compiler-original-declarations275` | `failed`, exit 101, 0.253940s | 0 passed/1 failed | [Original lossless receipt](frozen275/diagnostic-compiler-original-declarations275/receipt.json.gz) | [Whole log](frozen275/diagnostic-compiler-original-declarations275/tests.log) |
| `diagnostic-compiler-original-transfer275` | `failed`, exit 101, 0.670688s | 0 passed/1 failed | [Original lossless receipt](frozen275/diagnostic-compiler-original-transfer275/receipt.json.gz) | [Whole log](frozen275/diagnostic-compiler-original-transfer275/tests.log) |

Every exact source association, pin digest, complete producer log and original selection is checked independently. The two trace requests retain their original commands and actual test counts. Receipt compression preserves the original bytes and digests. This record launches no build, assertion or Native replay. Restore the measured executable from the repository root only after checking both stored and original identities:

```python
from pathlib import Path
import gzip, hashlib, json
base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen275/sealed-compiler-image")
record = json.loads((base / "lossless-image-storage.json").read_text())
packed = (base / "pinned-compiler275.elf.gz").read_bytes()
assert len(packed) == record["stored_bytes"]
assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
original = gzip.decompress(packed)
assert len(original) == record["original_bytes"]
assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
restored = Path("/tmp/pinned-compiler275.elf")
restored.write_bytes(original)
restored.chmod(0o755)
```

Decompression restores the original measured ELF byte-for-byte; it supplies no new build or assertion outcome.

## Sealed VM278 and independent compilation outcomes

The [original VM278 pin metadata](frozen278/sealed-vm-image/pinned-vm278.json), [lossless measured image](frozen278/sealed-vm-image/pinned-vm278.elf.gz), [storage identity receipt](frozen278/sealed-vm-image/lossless-image-storage.json) and [exact source companion](frozen278/sealed-vm-image/source-snapshot.json.gz) retain the successful independent VM-only278 no-run producer. Its inventory lists739 tests. The separate276 and277 VM construction attempts fail respectively at an ordinary Compiler DiagnosticSubject import and a typed VM adapter mismatch, supplying no assertion or image. The independent Core/DB/Server278 no-run attempt fails at a private AnalysisResult fixture constructor and supplies no image or assertion.

The sealed VM278 runs retain0 passed/2 failed for mathop admission,2 passed/1 failed for InfoCommands/options, and1 passed/0 failed for the original345 complete dictionary process comparator. The latter passes after reached-capture411 and before the independent Tick-vector412 ownership change; diagnostic-only409 topology still retains retired CALL-head views. It is neither a remaining345 comparator failure nor a new Native private-lifetime/frame guarantee. InfoCommands fails at the C8.4 escaped-nul-prefix primary-before-getter window (actual list, captured none); this exact failure is separate from VM275’s qualified-child String birth. No whole186-window software success is claimed.

| Closed operation | Status | Recorded result | Exact receipt | Complete log |
| --- | --- | --- | --- | --- |
| `integration-vm-native-test-build276` | `compile-blocked`, exit 101, 58.262503s | no assertions | [Original lossless receipt](frozen276/integration-vm-native-test-build276/receipt.json.gz) | [Whole log](frozen276/integration-vm-native-test-build276/tests.log) |
| `integration-vm-native-test-build277` | `compile-blocked`, exit 101, 73.779185s | no assertions | [Original lossless receipt](frozen277/integration-vm-native-test-build277/receipt.json.gz) | [Whole log](frozen277/integration-vm-native-test-build277/tests.log) |
| `integration-vm-native-test-build278` | `compile-passed`, exit 0, 104.570689s | no assertions | [Original lossless receipt](frozen278/integration-vm-native-test-build278/receipt.json.gz) | [Whole log](frozen278/integration-vm-native-test-build278/tests.log) |
| `integration-vm-current-inventory278` | `listed`, exit 0, 0.006270s | 739 listed, no assertions | [Original lossless receipt](frozen278/integration-vm-current-inventory278/receipt.json.gz) | [Whole log](frozen278/integration-vm-current-inventory278/tests.log) |
| `diagnostic-native278-mathop-admission` | `failed`, exit 101, 4.645936s | 0 passed/2 failed | [Original lossless receipt](frozen278/diagnostic-native278-mathop-admission/receipt.json.gz) | [Whole log](frozen278/diagnostic-native278-mathop-admission/tests.log) |
| `integration-vm-info-options-frontier278` | `failed`, exit 101, 3.200391s | 2 passed/1 failed | [Original lossless receipt](frozen278/integration-vm-info-options-frontier278/receipt.json.gz) | [Whole log](frozen278/integration-vm-info-options-frontier278/tests.log) |
| `diagnostic-native278-dictionary345-ownership` | `passed`, exit 0, 280.646146s | 1 passed/0 failed | [Original lossless receipt](frozen278/diagnostic-native278-dictionary345-ownership/receipt.json.gz) | [Whole log](frozen278/diagnostic-native278-dictionary345-ownership/tests.log) |
| `integration-core-db-server-test-build278` | `compile-blocked`, exit 101, 231.498612s | no assertions | [Original lossless receipt](frozen278/integration-core-db-server-test-build278/receipt.json.gz) | [Whole log](frozen278/integration-core-db-server-test-build278/tests.log) |

Every command, whole log, exact source association and libtest event is retained. The739-test inventory independently gates the actual selected assertion names. Native provider evidence and original expected windows are unchanged; these pinned-image outcomes do not promote an aggregate gate or another backend. Restore from the repository root after checking both stored and measured identities:

```python
from pathlib import Path
import gzip, hashlib, json
base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen278/sealed-vm-image")
record = json.loads((base / "lossless-image-storage.json").read_text())
packed = (base / "pinned-vm278.elf.gz").read_bytes()
assert len(packed) == record["stored_bytes"]
assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
original = gzip.decompress(packed)
assert len(original) == record["original_bytes"]
assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
restored = Path("/tmp/pinned-vm278.elf")
restored.write_bytes(original)
restored.chmod(0o755)
```

Lossless storage restores the exact measured executable; decompression supplies no new build or assertion.

## Sealed VM280 and independent compilation outcomes

The [unchanged VM280 pin metadata](frozen280/sealed-vm-image/pinned-vm280.json), [lossless measured image](frozen280/sealed-vm-image/pinned-vm280.elf.gz), [storage identity receipt](frozen280/sealed-vm-image/lossless-image-storage.json) and [exact source companion](frozen280/sealed-vm-image/source-snapshot.json.gz) retain the successful independent VM-only no-run producer. The earlier combined Compiler/VM invocation fails at three Compiler fixture errors; its VM artifact event is not treated as a successful producer. The separate Core/DB/Server build fails at a DB fixture Arc borrow and produces no test image.

The sealed seven-selector VM280 run retains4 passed/3 failed: two activation controls and two binary-carrier controls pass; both mathop comparators fail at the Jim namespace-info fixture, and InfoCommands fails at the C8.6 escaped-NUL primary-before-getter window (actual list, captured none). This is neither whole84/60/186 success nor an aggregate gate. The full log, exact selected names and all libtest events are retained. No absent standalone inventory receipt is invented.

| Closed operation | Status | Recorded result | Exact receipt | Complete log |
| --- | --- | --- | --- | --- |
| `integration-compiler-vm-test-build280` | `compile-blocked`, exit 101, 221.625077s | no assertions | [Original lossless receipt](frozen280/integration-compiler-vm-test-build280/receipt.json.gz) | [Whole log](frozen280/integration-compiler-vm-test-build280/tests.log) |
| `integration-vm-own-test-build280` | `compile-passed`, exit 0, 103.896400s | no assertions | [Original lossless receipt](frozen280/integration-vm-own-test-build280/receipt.json.gz) | [Whole log](frozen280/integration-vm-own-test-build280/tests.log) |
| `integration-vm-native-frontier-tests280` | `failed`, exit 101, 52.828099s | 4 passed/3 failed | [Original lossless receipt](frozen280/integration-vm-native-frontier-tests280/receipt.json.gz) | [Whole log](frozen280/integration-vm-native-frontier-tests280/tests.log) |
| `integration-core-db-server-test-build280` | `compile-blocked`, exit 101, 130.342107s | no assertions | [Original lossless receipt](frozen280/integration-core-db-server-test-build280/receipt.json.gz) | [Whole log](frozen280/integration-core-db-server-test-build280/tests.log) |

Every original source association is independently byte-checked. Native provider evidence and original expected windows are unchanged. Restore the exact measured image from the repository root after checking both stored and measured identities:

```python
from pathlib import Path
import gzip, hashlib, json
base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen280/sealed-vm-image")
record = json.loads((base / "lossless-image-storage.json").read_text())
packed = (base / "pinned-vm280.elf.gz").read_bytes()
assert len(packed) == record["stored_bytes"]
assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
original = gzip.decompress(packed)
assert len(original) == record["original_bytes"]
assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
restored = Path("/tmp/pinned-vm280.elf")
restored.write_bytes(original)
restored.chmod(0o755)
```

Lossless storage restores the exact executable; decompression supplies no new build or assertion.

## Compiler, Core, DB and Server no-run build282

The exact four-package `cargo test --lib --no-run --message-format=json --offline` command closes with exit101 after 68.490786s and `uniform_source: true`. One ordinary Compiler library E0308 mismatched-type diagnostic at value_transfer.rs:6665 blocks this no-run command before test executable creation. No test assertion or sealed executable pin is supplied. This command executes no test assertions and establishes no aggregate pass or Native provider failure.

The [original receipt](frozen282/integration-compiler-core-db-server-test-build282/receipt.json.gz) is retained as lossless gzip with compressed and uncompressed SHA256. The [complete log](frozen282/integration-compiler-core-db-server-test-build282/tests.log) preserves every original diagnostic and Cargo event. Every immutable source archive leaf is independently checked against the original receipt map. Original command, paths, source association and timing remain unchanged.

## Compiler, Core, DB and Server no-run build283

The exact four-package `cargo test --lib --no-run --message-format=json --offline` command closes with exit101 after 181.207771s and `uniform_source: true`. Two Compiler libtest E0716 lifetime diagnostics and 49 Server compilation diagnostics block this no-run command. The Server diagnostics include missing imports and diagnostic/test API mismatches, plus cancellation-closure trait safety. No test executable event, assertion or sealed executable pin is supplied. This command executes no test assertions and establishes no aggregate pass or Native provider failure.

The [original receipt](frozen283/integration-compiler-core-db-server-test-build283/receipt.json.gz) is retained as lossless gzip with compressed and uncompressed SHA256. The [complete log](frozen283/integration-compiler-core-db-server-test-build283/tests.log) preserves every original diagnostic and Cargo event. Every immutable source archive leaf is independently checked against the original receipt map. Original command, paths, source association and timing remain unchanged.

## Sealed Source284 compilation, inventories and assertions

Two independent successful no-run commands produce the actual VM284 and Compiler/Registry284 images. Their exact pin metadata, measured ELF identities and shared source companion are retained losslessly. The fresh inventories list 744 VM, 9111 Compiler and 1717 Registry tests. The three-selector VM run closes 0 passed/3 failed: both mathop comparisons fail Jim case 0 with typed Host retired-header refusal after the explicit helper; Info reaches Jim after its C rows and fails the fixture provider-profile lookup. These are software outcomes, independent of original Native observations, guarded Jim UNAVAILABLE source results and later fixture changes. The full Registry suite retains 1679 passed/38 failed; the exact original 114 Compiler selection closes 53 passed/61 failed. The separate authentic structural trace closes 0 passed/1 failed. No aggregate gate passes.

| Closed operation | Status | Recorded result | Exact receipt | Complete log |
| --- | --- | --- | --- | --- |
| `integration-vm-own-test-build284` | `compile-passed`, exit 0, 164.029556s | no assertions | [Original receipt](frozen284/integration-vm-own-test-build284/receipt.json.gz) | [Whole log](frozen284/integration-vm-own-test-build284/tests.log) |
| `integration-compiler-registry-own-test-build284` | `compile-passed`, exit 0, 280.474679s | no assertions | [Original receipt](frozen284/integration-compiler-registry-own-test-build284/receipt.json.gz) | [Whole log](frozen284/integration-compiler-registry-own-test-build284/tests.log) |
| `integration-vm-inventory284` | `listed`, exit 0, 0.006704s | no assertions | [Original receipt](frozen284/integration-vm-inventory284/receipt.json.gz) | [Whole log](frozen284/integration-vm-inventory284/tests.log) |
| `integration-compiler-inventory284` | `listed`, exit 0, 0.019214s | no assertions | [Original receipt](frozen284/integration-compiler-inventory284/receipt.json.gz) | [Whole log](frozen284/integration-compiler-inventory284/tests.log) |
| `integration-registry-inventory284` | `listed`, exit 0, 0.008491s | no assertions | [Original receipt](frozen284/integration-registry-inventory284/receipt.json.gz) | [Whole log](frozen284/integration-registry-inventory284/tests.log) |
| `integration-vm-native-frontier-tests284` | `failed`, exit 101, 91.776460s | 0 passed/3 failed | [Original receipt](frozen284/integration-vm-native-frontier-tests284/receipt.json.gz) | [Whole log](frozen284/integration-vm-native-frontier-tests284/tests.log) |
| `integration-compiler-original-focus-tests284` | `failed`, exit 101, 226.270322s | 53 passed/61 failed | [Original receipt](frozen284/integration-compiler-original-focus-tests284/receipt.json.gz) | [Whole log](frozen284/integration-compiler-original-focus-tests284/tests.log) |
| `integration-registry-complete-lib-tests284` | `failed`, exit 101, 116.243573s | 1679 passed/38 failed | [Original receipt](frozen284/integration-registry-complete-lib-tests284/receipt.json.gz) | [Whole log](frozen284/integration-registry-complete-lib-tests284/tests.log) |
| `integration-compiler-structural-declaration-trace284` | `failed`, exit 101, 1.695621s | 0 passed/1 failed | [Original receipt](frozen284/integration-compiler-structural-declaration-trace284/receipt.json.gz) | [Whole log](frozen284/integration-compiler-structural-declaration-trace284/tests.log) |

The [unchanged 114-selector receipt](frozen284/compiler-focus-selection284.json) retains its independent original selection digest, actual Compiler pin and current inventory check. Every original source association is byte-checked. Question links on assertion rows require the current complete source file to match that measured image; changed current definitions are recorded separately without borrowing the old result. Original Native provider evidence and expected windows remain unchanged.

Restore a sealed image from the repository root after verifying both stored and measured identities:

```python
from pathlib import Path
import gzip, hashlib, json
name = "vm"  # also "compiler" or "registry"
base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen284") / ("sealed-" + name + "-image")
record = json.loads((base / "lossless-image-storage.json").read_text())
packed = (base / ("pinned-" + name + "284.elf.gz")).read_bytes()
assert len(packed) == record["stored_bytes"]
assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
original = gzip.decompress(packed)
assert len(original) == record["original_bytes"]
assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
restored = Path("/tmp/pinned-" + name + "284.elf")
restored.write_bytes(original)
restored.chmod(0o755)
```

Lossless restoration preserves executable identity and supplies no new build or assertion.

## VM no-run build285: compilation only

The exact `cargo test -p tcl-vm --lib --no-run --message-format=json --offline` command closes with exit 101 after 53.847967s and `uniform_source: true`. Two Compiler dependency missing_docs errors at dynamic_names.rs:289 and :290 block the VM no-run command before test executable creation. No test assertion or strict executable pin is supplied. These compilation records supply no Native provider failure or aggregate gate pass.

The [unchanged original receipt](frozen285/integration-vm-own-test-build285/receipt.json.gz) is retained as lossless gzip with compressed and uncompressed SHA256. The [whole original log](frozen285/integration-vm-own-test-build285/tests.log) retains every diagnostic and Cargo event. Every immutable source archive leaf is independently byte-checked. Original command, paths, source association and timing remain unchanged.


## VM no-run build286: compilation only

The exact `cargo test -p tcl-vm --lib --no-run --message-format=json --offline` command closes with exit 0 after 131.275487s and `uniform_source: true`. The exact successful VM no-run command records one tcl_vm libtest executable event. This record establishes compilation only; strict copied executable pinning, inventory listing and any later assertion runs require their own original receipts. No pin or assertion outcome is attached to this build-only record. These compilation records supply no Native provider failure or aggregate gate pass.

The [unchanged original receipt](frozen286/integration-vm-own-test-build286/receipt.json.gz) is retained as lossless gzip with compressed and uncompressed SHA256. The [whole original log](frozen286/integration-vm-own-test-build286/tests.log) retains every diagnostic and Cargo event. Every immutable source archive leaf is independently byte-checked. Original command, paths, source association and timing remain unchanged.

## Sealed Source286 VM inventory and Jim guard diagnostic

The strict copied VM286 image belongs to its independently archived successful no-run producer. Its [original pin metadata](frozen286/sealed-vm-image/pinned-vm286.json), [lossless image record](frozen286/sealed-vm-image/lossless-image-storage.json), gzip executable and [source companion](frozen286/source-snapshot.json.gz) preserve exact original identities. Every source association is byte-checked. The actual inventory lists 745 tests; the separate opt-in Jim namespace-info guard diagnostic closes 0 passed/1 failed with typed Host retired-header refusal. This is a software ownership outcome, independent of the original current-Jim guarded code0/UNAVAILABLE measurements and helper declaration. It does not rerun the whole 60/84 comparators or unrelated control suites. No aggregate gate passes.

| Closed operation | Recorded outcome | Original receipt | Whole log |
| --- | --- | --- | --- |
| `integration-vm-inventory286` | `listed`, exit 0, 0.006250s; 745 listed; no assertions | [Original receipt](frozen286/integration-vm-inventory286/receipt.json.gz) | [Complete log](frozen286/integration-vm-inventory286/tests.log) |
| `integration-vm-jim-original-header-diagnostic286` | `failed`, exit 101, 0.146721s; 0 passed/1 failed | [Original receipt](frozen286/integration-vm-jim-original-header-diagnostic286/receipt.json.gz) | [Complete log](frozen286/integration-vm-jim-original-header-diagnostic286/tests.log) |

Restore the exact measured image after checking stored and original identities:

```python
from pathlib import Path
import gzip, hashlib, json
base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen286/sealed-vm-image")
record = json.loads((base / "lossless-image-storage.json").read_text())
packed = (base / "pinned-vm286.elf.gz").read_bytes()
assert len(packed) == record["stored_bytes"]
assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
original = gzip.decompress(packed)
assert len(original) == record["original_bytes"]
assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
restored = Path("/tmp/pinned-vm286.elf")
restored.write_bytes(original)
restored.chmod(0o755)
```

Restoration preserves measured executable identity and supplies no new build, run, Native provider answer or private-header proof.

## Compiler, Registry, Core, DB and Server no-run build287: compilation only

The original `cargo test` no-run command selects the five named packages and closes with exit101 after 202.570912s and `uniform_source: true`. Four Compiler fixture/import errors and three Server integration errors block this exact five-package no-run command. The whole Cargo log records no executable event; no test inventory, strict executable pin or unit assertion is produced by this invocation. This record supplies no Native provider failure, assertion outcome or aggregate gate result.

The [unchanged original receipt](frozen287/integration-compiler-registry-core-db-server-test-build287/receipt.json.gz) is retained as lossless gzip with compressed and uncompressed SHA256. The [whole original log](frozen287/integration-compiler-registry-core-db-server-test-build287/tests.log) preserves all seven errors and Cargo events. All 31917 immutable source archive leaves are independently byte-checked. Original command, paths, source association and timing remain unchanged.

## Sealed Source288 VM producer, inventory and independent software outcomes

The exact successful no-run producer and strict copied VM288 image retain their original pin, complete source companion and measured executable identity. The actual inventory lists 752 tests. The paired Jim guard diagnostic fails0 passed/1 failed with Value::drop under eval_original_uplevel before later NativeJimScriptState::new retired access. The independent seven-control software run closes6 passed/1 failed: all five checked completion controls and supported inventory pass. The old negative fails its pre-getter expectation because genuine C8.6 core policy remains present after only the source profile/pin changes to Jim0.79. The later standalone-negative fixture and distinct core/source-positive control are separate unexecuted definitions for this image. No original external provider result, aggregate gate pass or deferred418 assertion outcome follows.

| Closed operation | Recorded outcome | Original receipt | Whole log |
| --- | --- | --- | --- |
| `integration-vm-own-test-build288` | `compile-passed`, exit0, 140.920935s; one successful artifact event; no assertions | [Original receipt](frozen288/integration-vm-own-test-build288/receipt.json.gz) | [Whole log](frozen288/integration-vm-own-test-build288/tests.log) |
| `integration-vm-inventory288` | `listed`, exit0, 0.028622s; 752 listed; no assertions | [Original receipt](frozen288/integration-vm-inventory288/receipt.json.gz) | [Whole log](frozen288/integration-vm-inventory288/tests.log) |
| `integration-vm-jim-retirement-diagnostic288` | `failed`, exit101, 0.274198s; 0 passed/1 failed | [Original receipt](frozen288/integration-vm-jim-retirement-diagnostic288/receipt.json.gz) | [Whole log](frozen288/integration-vm-jim-retirement-diagnostic288/tests.log) |
| `integration-vm-completion-inventory-tests288` | `failed`, exit101, 2.229806s; 6 passed/1 failed | [Original receipt](frozen288/integration-vm-completion-inventory-tests288/receipt.json.gz) | [Whole log](frozen288/integration-vm-completion-inventory-tests288/tests.log) |

The [exact selection](frozen288/selection/vm-completion-inventory-selection288.json), [original pin metadata](frozen288/sealed-vm-image/pinned-vm288.json), [lossless image record](frozen288/sealed-vm-image/lossless-image-storage.json) and [source companion](frozen288/source-snapshot.json.gz) retain their original associations. All 127688 source leaf associations are independently checked. Current proof IDs attach only where the entire current source leaf matches this frozen image; changed definitions borrow no outcome.

Restore the exact measured executable with both compressed and original integrity checks:

```python
from pathlib import Path
import gzip, hashlib, json
base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen288/sealed-vm-image")
record = json.loads((base / "lossless-image-storage.json").read_text())
packed = (base / "pinned-vm288.elf.gz").read_bytes()
assert len(packed) == record["stored_bytes"]
assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
original = gzip.decompress(packed)
assert len(original) == record["original_bytes"]
assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
restored = Path("/tmp/pinned-vm288.elf")
restored.write_bytes(original)
restored.chmod(0o755)
```

Restoration proves payload identity only; it supplies no new build, run, Native private/header/frame observation or current assertion pass.

## Exact Source289 Compiler and Registry outcomes

Six closed operations retain the same immutable source and exact original command receipts. The five-package no-run invocation fails 15 DB fixture/API errors and supplies no pin. Its independent Compiler/Registry no-run producer succeeds and yields the two separately sealed images. Actual inventories list 9,135 Compiler and 1,719 Registry tests. The focused Compiler command runs 210 tests and closes 135 passed/75 failed; the complete Registry command closes 1,700 passed/19 failed with no ignored or filtered tests. No aggregate gate or external Native provider pass follows.

| Closed operation | Original outcome | Receipt | Whole log |
| --- | --- | --- | --- |
| `integration-compiler-registry-core-db-server-test-build289` | `compile-failed`, exit 101, 127.474772s; no assertions | [Receipt](frozen289/integration-compiler-registry-core-db-server-test-build289/receipt.json.gz) | [Log](frozen289/integration-compiler-registry-core-db-server-test-build289/tests.log) |
| `integration-compiler-registry-own-test-build289` | `compile-passed`, exit 0, 226.707102s; no assertions | [Receipt](frozen289/integration-compiler-registry-own-test-build289/receipt.json.gz) | [Log](frozen289/integration-compiler-registry-own-test-build289/tests.log) |
| `integration-compiler-inventory289` | `listed`, exit 0, 0.017769s; no assertions | [Receipt](frozen289/integration-compiler-inventory289/receipt.json.gz) | [Log](frozen289/integration-compiler-inventory289/tests.log) |
| `integration-registry-inventory289` | `listed`, exit 0, 0.015690s; no assertions | [Receipt](frozen289/integration-registry-inventory289/receipt.json.gz) | [Log](frozen289/integration-registry-inventory289/tests.log) |
| `integration-compiler-comprehensive-focus289` | `failed`, exit 101, 343.473291s; 135 passed/75 failed | [Receipt](frozen289/integration-compiler-comprehensive-focus289/receipt.json.gz) | [Log](frozen289/integration-compiler-comprehensive-focus289/tests.log) |
| `integration-registry-complete-lib289` | `failed`, exit 101, 84.124573s; 1700 passed/19 failed | [Receipt](frozen289/integration-registry-complete-lib289/receipt.json.gz) | [Log](frozen289/integration-registry-complete-lib289/tests.log) |

The [exact 210 selection](frozen289/selections/compiler-focus-selection289.json) retains the actual listed-image gate. The [original 114 subset record](frozen289/selections/original114-derived-outcome.json) preserves the unchanged original selection and derives 72 passed/42 failed from this same focus command; it is not another operation. The [complete source companion](frozen289/source-snapshot.json.gz) and each decompressed receipt retain the exact source associations. Current proof IDs attach only where the entire current source leaf matches the frozen producer.

Both measured executable identities are preserved as lossless gzip: [Compiler storage](frozen289/sealed-compiler-image/lossless-image-storage.json) and [Registry storage](frozen289/sealed-registry-image/lossless-image-storage.json). Their original strict pins and successful producer remain distinct from the failed aggregate invocation. Full decompressed payload bytes, size and SHA256 were compared with each original measured ELF. Restore either image with both integrity checks:

```python
from pathlib import Path
import gzip, hashlib, json
for kind in ("compiler", "registry"):
    base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen289") / f"sealed-{kind}-image"
    record = json.loads((base / "lossless-image-storage.json").read_text())
    packed = (base / f"pinned-{kind}289.elf.gz").read_bytes()
    assert len(packed) == record["stored_bytes"]
    assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
    original = gzip.decompress(packed)
    assert len(original) == record["original_bytes"]
    assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
    restored = Path(f"/tmp/pinned-{kind}289.elf")
    restored.write_bytes(original)
    restored.chmod(0o755)
```

Restoration establishes exact measured image identity only; it does not perform another build/test or grant Native private object/frame/handler authority.

## Sealed Source290 VM producer and two independent assertion runs

The exact successful VM producer and copied strict image retain their complete source companion and measured executable identity. The fresh list contains 756 tests. The independent 12-control software run closes 11 passed/1 failed: the original Jim helper guard, three deferred-script ownership controls, five checked completion controls and two previous inventory controls pass. The genuine-core/unknown-source positive fails downstream native object list conversion; it supplies no successful inventory result. The separate 25-control run closes 24 passed/1 failed. Both complete 84/60 mathop comparators and the complete 345 dictionary comparator pass. Info186 fails at Jim absolute-existing Guest completion 1 versus original 0; prior C loop advancement is not a completed 186 or independent provider pass. All other selected observation-window, live-host-storage, error-capture, manifest, byte and Host-boundary assertions pass only in their exact purpose. There is no aggregate gate PASS.

| Closed operation | Recorded outcome | Original receipt | Whole log |
| --- | --- | --- | --- |
| `integration-vm-own-test-build290` | `compile-passed`, exit 0, 160.890518s; one successful artifact event; no assertions | [Original receipt](frozen290/integration-vm-own-test-build290/receipt.json.gz) | [Whole log](frozen290/integration-vm-own-test-build290/tests.log) |
| `integration-vm-inventory290` | `listed`, exit 0, 0.014115s; 756 listed; no assertions | [Original receipt](frozen290/integration-vm-inventory290/receipt.json.gz) | [Whole log](frozen290/integration-vm-inventory290/tests.log) |
| `integration-vm-ownership-completion-inventory-tests290` | `failed`, exit 101, 3.928031s; 11 passed/1 failed | [Original receipt](frozen290/integration-vm-ownership-completion-inventory-tests290/receipt.json.gz) | [Whole log](frozen290/integration-vm-ownership-completion-inventory-tests290/tests.log) |
| `integration-vm-native-frontier-tests290` | `failed`, exit 101, 351.653479s; 24 passed/1 failed | [Original receipt](frozen290/integration-vm-native-frontier-tests290/receipt.json.gz) | [Whole log](frozen290/integration-vm-native-frontier-tests290/tests.log) |

The exact [12-control selection](frozen290/selections/vm-ownership-completion-inventory-selection290.json) and [25-control selection](frozen290/selections/vm-native-frontier-selection290.json) retain the strict image/list gates. The [original pin](frozen290/sealed-vm-image/pinned-vm290.json), [lossless storage record](frozen290/sealed-vm-image/lossless-image-storage.json) and [complete source companion](frozen290/source-snapshot.json.gz) retain all associations; 127,832 source leaves are independently checked across the four receipts. Current proof IDs attach only where the entire source leaf matches the frozen producer; changed definitions borrow no outcome.

Restore the exact measured image with both compressed and original integrity checks:

```python
from pathlib import Path
import gzip, hashlib, json
base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen290/sealed-vm-image")
record = json.loads((base / "lossless-image-storage.json").read_text())
packed = (base / "pinned-vm290.elf.gz").read_bytes()
assert len(packed) == record["stored_bytes"]
assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
original = gzip.decompress(packed)
assert len(original) == record["original_bytes"]
assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
restored = Path("/tmp/pinned-vm290.elf")
restored.write_bytes(original)
restored.chmod(0o755)
```

Restoration establishes measured payload identity only; it performs no new build/run and grants no Native header/frame/handler authority. The 326 complete/3 bounded dictionary-window assertions preserve post-free boundaries; live host storage is independent. Original public measurement archives and failed Info/inventory assertions remain unchanged.

## Compiler, Registry, Core, DB and Server no-run build291: compilation only

The original `cargo test` no-run command selects the five named packages and closes with exit101 after 351.410758s and `uniform_source: true`. Two Compiler fixture errors block this exact five-package no-run command. The whole log also records three test executable creation events for Server, DB and Registry. Those events are retained as unsealed artifacts from a failed invocation; no independent successful producer receipt, strict executable pin, test inventory or unit assertion is claimed. This record supplies no Native provider failure, assertion outcome or aggregate gate result.

The [unchanged original receipt](frozen291/integration-compiler-registry-core-db-server-test-build291/receipt.json.gz) is retained as lossless gzip with compressed and uncompressed SHA256. The [whole original log](frozen291/integration-compiler-registry-core-db-server-test-build291/tests.log) preserves both Compiler errors and all three executable events. All 31980 immutable source archive leaves are independently byte-checked. Original command, paths, source association and timing remain unchanged.

## VM no-run build292: dependency compilation only

The original `cargo test` no-run command selects VM and closes with exit101 after 24.978411s and `uniform_source: true`. One Registry library type error in value_transfer/const_ops.rs blocks this exact VM no-run command. The log records no test executable creation event. No successful producer receipt, strict executable pin, test inventory or unit assertion is claimed. This record supplies no Native provider failure, assertion outcome or aggregate gate result.

The [unchanged original receipt](frozen292/integration-vm-own-test-build292/receipt.json.gz) is retained as lossless gzip with compressed and uncompressed SHA256. The [whole original log](frozen292/integration-vm-own-test-build292/tests.log) preserves the Registry type error and unused-import warning. All 31997 immutable source archive leaves are independently byte-checked. Original command, paths, source association and timing remain unchanged.

## VM no-run build293: compilation only

The exact original `cargo test -p tcl-vm --lib --no-run` command closes with exit101 after 28.933597s and `uniform_source: true`. Six Registry library errors for the missing NativeExecutionError type block this exact VM no-run command. No test executable creation event is recorded. The retained unused-import warning remains in the whole log. No successful producer pin, test inventory, unit assertion, Native provider outcome or aggregate gate result is claimed.

The [unchanged original receipt](frozen293/integration-vm-own-test-build293/receipt.json.gz) is retained as lossless gzip with compressed and uncompressed SHA256. The [whole original log](frozen293/integration-vm-own-test-build293/tests.log) preserves every diagnostic. All 31997 immutable source archive leaves are independently byte-checked. Original command, paths, source association and timing remain unchanged.


## VM no-run build294: compilation only

The exact original `cargo test -p tcl-vm --lib --no-run` command closes with exit101 after 129.602939s and `uniform_source: true`. One VM test fixture type error blocks this exact no-run command: the inherent dictionary convenience query returns a String key, while this control requires the original Value key identity. No test executable creation event is recorded. The retained unused-import warning remains in the whole log. No successful producer pin, test inventory, unit assertion, Native provider outcome or aggregate gate result is claimed.

The [unchanged original receipt](frozen294/integration-vm-own-test-build294/receipt.json.gz) is retained as lossless gzip with compressed and uncompressed SHA256. The [whole original log](frozen294/integration-vm-own-test-build294/tests.log) preserves every diagnostic. All 31999 immutable source archive leaves are independently byte-checked. Original command, paths, source association and timing remain unchanged.

## Sealed Source295 VM image and separate dependency-blocked builds

The successful independent VM no-run producer and its strict copied executable retain the exact source/pin association. The actual inventory lists 762 tests. The twenty-control software focus closes19 passed/1 failed: the foreign dictionary getter refusal fails while the other nineteen assertions pass, including genuine unknown-source actual-engine inventory and the three deferred Jim ownership controls. The unchanged Info186 comparator closes 0 passed/1 failed at Jim absolute-existing with Guest1 versus observed0 and live/readable result headers; it establishes no whole comparator pass. Both independent Compiler dependency commands fail on the same two ordinary Core errors, each with one unsealed partial Syntax artifact event and no Compiler pin or assertions. Original provider observations remain independent.

| Exact closed operation | Recorded outcome | Original receipt | Whole log |
| --- | --- | --- | --- |
| `integration-vm-own-test-build295` | `compile-passed`, exit0, 118.143700s; one sealed VM artifact; no assertions | [Original receipt](frozen295/integration-vm-own-test-build295/receipt.json.gz) | [Whole log](frozen295/integration-vm-own-test-build295/tests.log) |
| `integration-vm-inventory295` | `listed`, exit0, 0.011224s; 762 listed; no assertions | [Original receipt](frozen295/integration-vm-inventory295/receipt.json.gz) | [Whole log](frozen295/integration-vm-inventory295/tests.log) |
| `integration-vm-software-tests295` | `failed`, exit101, 7.457860s; 19 passed/1 failed | [Original receipt](frozen295/integration-vm-software-tests295/receipt.json.gz) | [Whole log](frozen295/integration-vm-software-tests295/tests.log) |
| `integration-vm-info-diagnostic-tests295` | `failed`, exit101, 73.346140s; 0 passed/1 failed | [Original receipt](frozen295/integration-vm-info-diagnostic-tests295/receipt.json.gz) | [Whole log](frozen295/integration-vm-info-diagnostic-tests295/tests.log) |
| `integration-compiler-registry-syntax-core-db-server-test-build295` | `compile-blocked`, exit101, 191.643445s; two Core errors; partial Syntax event unsealed; no assertions | [Original receipt](frozen295/integration-compiler-registry-syntax-core-db-server-test-build295/receipt.json.gz) | [Whole log](frozen295/integration-compiler-registry-syntax-core-db-server-test-build295/tests.log) |
| `integration-compiler-registry-syntax-own-test-build295` | `compile-blocked`, exit101, 110.221930s; two Core errors; partial Syntax event unsealed; no assertions | [Original receipt](frozen295/integration-compiler-registry-syntax-own-test-build295/receipt.json.gz) | [Whole log](frozen295/integration-compiler-registry-syntax-own-test-build295/tests.log) |

The [software selection](frozen295/selection/vm-software-selection295.json), [Info diagnostic selection](frozen295/selection/vm-info-diagnostic-selection295.json), [original pin](frozen295/sealed-vm-image/pinned-vm295.json), [lossless image record](frozen295/sealed-vm-image/lossless-image-storage.json) and [complete source companion](frozen295/source-snapshot.json.gz) retain their original command associations. All 192234 immutable source-leaf associations are independently checked. Current question IDs attach only when the entire current definition leaf matches the frozen image. Definition links and later source fixes do not borrow outcomes.

Restore the exact measured VM executable with compressed and original integrity checks:

```python
from pathlib import Path
import gzip, hashlib, json
base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen295/sealed-vm-image")
record = json.loads((base / "lossless-image-storage.json").read_text())
packed = (base / "pinned-vm295.elf.gz").read_bytes()
assert len(packed) == record["stored_bytes"]
assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
original = gzip.decompress(packed)
assert len(original) == record["original_bytes"]
assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
restored = Path("/tmp/pinned-vm295.elf")
restored.write_bytes(original)
restored.chmod(0o755)
```

Restoration proves payload identity, not a new build, replay, Native observation or current assertion pass.

## Compiler, Registry, Syntax, Core, Database and Server no-run build296: compilation only

The exact original six-package library no-run command closes with exit `101` after 362.937164s and `uniform_source: true`. Four Compiler libtest fixture errors refer to source_registry_words_at through the wrong helper namespace. The failed invocation records four partial executable events: Syntax is marked fresh; Server, Database and Registry are marked newly built. These events are retained without a sealed pin or an assertion result. No Rust unit assertion is launched. The partial artifact events supply no successful aggregate producer, Compiler pin, test inventory, Native provider answer or overall PASS.

The [unchanged original receipt](frozen296/integration-compiler-registry-syntax-core-db-server-own-test-build296/receipt.json.gz) is retained as lossless gzip with compressed and uncompressed SHA256. The [whole original log](frozen296/integration-compiler-registry-syntax-core-db-server-own-test-build296/tests.log) preserves all four errors and artifact events, including their actual fresh flags. All 32150 immutable source associations are byte-checked. Original command, source/archive paths and timing remain unchanged; separate later fixture corrections are outside this frozen operation.

## Source297 independent build and assertion records

The Compiler/Registry/Syntax no-run command fails one Compiler libtest fixture type error: expected owned CommandRegistry, found a borrowed registry in store_advice.rs. Its partial Syntax/Registry artifact events remain unsealed. The independent VM no-run producer succeeds; its strict copied image and complete source companion support a fresh inventory of 763 tests. The separate 21-control software run passes all 21. The separate Info186 run fails 0 passed/1 failed at Jim absolute-existing primary before the getter: actual none versus original string. This actual receipt does not report the earlier namespace-info availability failure. No partial loop advancement is a completed 186 assertion or Native provider pass, and there is no aggregate gate PASS.

| Closed operation | Recorded outcome | Original receipt | Whole log |
| --- | --- | --- | --- |
| `integration-compiler-registry-syntax-own-test-build297` | `compile-blocked`, exit 101, 207.628163s; compilation only; no assertions | [Original receipt](frozen297/integration-compiler-registry-syntax-own-test-build297/receipt.json.gz) | [Whole log](frozen297/integration-compiler-registry-syntax-own-test-build297/tests.log) |
| `integration-vm-own-test-build297` | `compile-passed`, exit 0, 92.213905s; compilation only; no assertions | [Original receipt](frozen297/integration-vm-own-test-build297/receipt.json.gz) | [Whole log](frozen297/integration-vm-own-test-build297/tests.log) |
| `integration-vm-inventory297` | `listed`, exit 0, 0.006714s; 763 listed; no assertions | [Original receipt](frozen297/integration-vm-inventory297/receipt.json.gz) | [Whole log](frozen297/integration-vm-inventory297/tests.log) |
| `integration-vm-software-tests297` | `passed`, exit 0, 6.482244s; 21 passed/0 failed | [Original receipt](frozen297/integration-vm-software-tests297/receipt.json.gz) | [Whole log](frozen297/integration-vm-software-tests297/tests.log) |
| `integration-vm-info-diagnostic-tests297` | `failed`, exit 101, 42.423379s; 0 passed/1 failed | [Original receipt](frozen297/integration-vm-info-diagnostic-tests297/receipt.json.gz) | [Whole log](frozen297/integration-vm-info-diagnostic-tests297/tests.log) |

The [21-control selection](frozen297/selections/vm-software-selection297.json) and [Info diagnostic selection](frozen297/selections/vm-info-diagnostic-selection297.json) preserve exact strict image/inventory gates. The [unchanged pin](frozen297/sealed-vm-image/pinned-vm297.json), [lossless image storage record](frozen297/sealed-vm-image/lossless-image-storage.json) and [complete source companion](frozen297/source-snapshot.json.gz) preserve every original association. All 160,850 source associations across five receipts are independently byte-checked. Question IDs attach only where the entire current source leaf equals the frozen producer; later changed definitions borrow no result.

Restore the exact measured image with both compressed and original integrity checks:

```python
from pathlib import Path
import gzip, hashlib, json
base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen297/sealed-vm-image")
record = json.loads((base / "lossless-image-storage.json").read_text())
packed = (base / "pinned-vm297.elf.gz").read_bytes()
assert len(packed) == record["stored_bytes"]
assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
original = gzip.decompress(packed)
assert len(original) == record["original_bytes"]
assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
restored = Path("/tmp/pinned-vm297.elf")
restored.write_bytes(original)
restored.chmod(0o755)
```

Restoration proves payload identity only and launches no assertion. Software materialisation/foreign seal/cache, deferred script ownership, completion options, binary origin and inventory-purpose assertions retain their own exact scopes; they do not produce an original Native header/cache/frame observation or application admission.

## Source298 exact producer and closed attempts

The independent Compiler/Registry/Syntax no-run producer succeeds and seals three authentic copied libtest images. Their complete inventories contain 9172,1722 and 635 tests respectively. The complete Registry run closes 1720 passed/2 failed; the complete Syntax run closes 634 passed/1 failed. The independent source-path trace closes 0 passed/1 failed. Both Compiler selections are interrupted without a final libtest summary: the 344-control request stops in an alias control, and the independently selected 342 controls stop in the large regexp fixture. Two separate 45-second alias traces are also killed without an assertion outcome. No timeout establishes an infinite loop or convergence property, and no partial Compiler log is an aggregate pass/fail result.

| Closed operation | Exact outcome | Original receipt | Whole log |
|---|---|---|---|
| `integration-compiler-registry-syntax-own-test-build298` | `compile-passed`, exit 0, 310.564886s; compilation only | [Original receipt](frozen298/integration-compiler-registry-syntax-own-test-build298/receipt.json.gz) | [Whole log](frozen298/integration-compiler-registry-syntax-own-test-build298/tests.log) |
| `integration-compiler-inventory298` | `listed`, exit 0, 0.014154s; listing only | [Original receipt](frozen298/integration-compiler-inventory298/receipt.json.gz) | [Whole log](frozen298/integration-compiler-inventory298/tests.log) |
| `integration-registry-inventory298` | `listed`, exit 0, 0.008303s; listing only | [Original receipt](frozen298/integration-registry-inventory298/receipt.json.gz) | [Whole log](frozen298/integration-registry-inventory298/tests.log) |
| `integration-syntax-inventory298` | `listed`, exit 0, 0.003648s; listing only | [Original receipt](frozen298/integration-syntax-inventory298/receipt.json.gz) | [Whole log](frozen298/integration-syntax-inventory298/tests.log) |
| `integration-compiler-consumer-tests298` | `aborted`, exit -15, 463.911617s; no final assertion summary | [Original receipt](frozen298/integration-compiler-consumer-tests298/receipt.json.gz) | [Whole log](frozen298/integration-compiler-consumer-tests298/tests.log) |
| `integration-compiler-other-consumer-tests298` | `aborted`, exit -15, 1374.014597s; no final assertion summary | [Original receipt](frozen298/integration-compiler-other-consumer-tests298/receipt.json.gz) | [Whole log](frozen298/integration-compiler-other-consumer-tests298/tests.log) |
| `integration-compiler-alias-bounded-trace298` | `aborted`, exit -9, 45.022633s; no final assertion summary | [Original receipt](frozen298/integration-compiler-alias-bounded-trace298/receipt.json.gz) | [Whole log](frozen298/integration-compiler-alias-bounded-trace298/tests.log) |
| `integration-compiler-alias-bounded-phase-trace298` | `aborted`, exit -9, 45.013868s; no final assertion summary | [Original receipt](frozen298/integration-compiler-alias-bounded-phase-trace298/receipt.json.gz) | [Whole log](frozen298/integration-compiler-alias-bounded-phase-trace298/tests.log) |
| `integration-compiler-source-path-trace298` | `failed`, exit 101, 0.841776s; 0 passed/1 failed | [Original receipt](frozen298/integration-compiler-source-path-trace298/receipt.json.gz) | [Whole log](frozen298/integration-compiler-source-path-trace298/tests.log) |
| `integration-registry-all-tests298` | `failed`, exit 101, 111.948334s; 1720 passed/2 failed | [Original receipt](frozen298/integration-registry-all-tests298/receipt.json.gz) | [Whole log](frozen298/integration-registry-all-tests298/tests.log) |
| `integration-syntax-all-tests298` | `failed`, exit 101, 1.035825s; 634 passed/1 failed | [Original receipt](frozen298/integration-syntax-all-tests298/receipt.json.gz) | [Whole log](frozen298/integration-syntax-all-tests298/tests.log) |

The [344-control selection](frozen298/selections/compiler-focus-selection298.json), [342-control selection](frozen298/selections/compiler-other-consumer-selection298.json) and [bounded-close record](frozen298/selections/compiler-consumer-bounded-close298.json) preserve distinct exact request and interruption purposes. The [Registry selection](frozen298/selections/registry-full-selection298.json) and [Syntax selection](frozen298/selections/syntax-full-selection298.json) preserve the complete admitted inventories. Every original receipt retains the same [full source companion](frozen298/source-snapshot.json.gz); all 356015 source associations across eleven receipts are independently verified. Current question/result links require whole source-leaf equality with that original producer; later changed controls receive no outcome.

Restore each exact measured image with compressed and original integrity checks:

```python
from pathlib import Path
import gzip, hashlib, json
for kind in ("compiler", "registry", "syntax"):
    base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen298") / ("sealed-" + kind + "-image")
    record = json.loads((base / "lossless-image-storage.json").read_text())
    name = "pinned-" + kind + "298.elf"
    packed = (base / (name + ".gz")).read_bytes()
    assert len(packed) == record["stored_bytes"]
    assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
    original = gzip.decompress(packed)
    assert len(original) == record["original_bytes"]
    assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
    restored = Path("/tmp") / name
    restored.write_bytes(original)
    restored.chmod(0o755)
```

Restoration checks payload identity and launches no test. These images retain every original byte and source association; no original image is removed by publication. Exact software assertion outcomes do not create external Native observations, private header/frame/cache proof, compiler or application admission, or an overall green gate.

## Source299 independent failed no-run invocation

The separate five-crate Source299 no-run command fails one CmdCore test import: DialectPoint and Release are imported from the crate root instead of their model owner. Its newly-created Syntax executable event remains unsealed. The invocation provides no successful aggregate producer, strict 299 pin, inventory or assertions; Source298 images and later corrected imports confer no 299 success. The [unchanged compressed receipt](frozen299/integration-shared-owner-own-test-build299/receipt.json.gz), [whole log](frozen299/integration-shared-owner-own-test-build299/tests.log) and [complete Source299 companion](frozen299/source-snapshot.json.gz) preserve the exact command, exit 101, 40.193293303018436s and all 32710 independently verified source leaves.

## Five-crate no-run build300: fixture compilation only

The original `cargo test` no-run command selects Compiler, Registry, Syntax, CmdCore and VM and closes with exit 101 after 7.267246s and `uniform_source: true`. One Syntax test fixture type error at scalar_getter/target_tests.rs:457 blocks this exact five-crate no-run command: the expected cached String uses Vec<u8> while the input supplies Cow<[u8]>. The log records no test executable creation event. No successful producer receipt, strict executable pin, test inventory or unit assertion is claimed. This record supplies no Native provider failure, assertion outcome or aggregate gate result.

The [unchanged original receipt](frozen300/integration-shared-owner-own-test-build300/receipt.json.gz) is retained as lossless gzip with compressed and uncompressed SHA256. The [whole original log](frozen300/integration-shared-owner-own-test-build300/tests.log) preserves the Syntax fixture type error and complete build diagnostics. All 32821 immutable source archive leaves are independently byte-checked. Original command, paths, source association and timing remain unchanged.

## Six-crate no-run build301: independent fixture compilation only

Source301 selects Compiler, Registry, Syntax, CmdCore, VM and RuntimeAPI and closes exit 101 after 46.44751959500718s with uniform source. Its CmdCore cache fixture has a Cow/Vec type mismatch. The log records one newly-created Syntax executable event, which remains unsealed; no successful producer, strict 301 executable pin, inventory or assertion is claimed. The subsequent fixture correction supplies no outcome for this invocation.

The [unchanged original301 receipt](frozen301/integration-shared-owner-own-test-build301/receipt.json.gz), [whole301 log](frozen301/integration-shared-owner-own-test-build301/tests.log) and [full301 source companion](frozen301/source-snapshot.json.gz) preserve all 32869 independently verified source associations and exact original timing/path/command. The [independent full300 source companion](frozen300/source-snapshot.json.gz) retains its separate 32821-leaf image. Receipts/source companions use verified lossless gzip; all original bytes and compressed/uncompressed hashes remain restorable. Neither failed command supplies an aggregate gate or Native result.

## Source301 independent Syntax producer and full assertions

The separate Syntax `num-bigint` no-run command succeeds with its own strict image and source companion. Its fresh inventory lists 643 tests. The exact full selection closes 642 passed/1 failed: `scalar_getter::target_tests::original_long64_public_value_cache_and_live_failure_fields_match_captures`. Compilation and listing execute no assertions. The earlier six-crate Source301 command remains independently failed; an unsealed partial Syntax event from that command is not this successful producer. No overall gate or Native primitive/provider PASS is claimed.

| Closed operation | Exact outcome | Original receipt | Whole log |
| --- | --- | --- | --- |
| `integration-syntax-own-test-build301` | `compile-passed`, exit 0, 7.668721s; compilation only; no assertions | [Original receipt](frozen301/integration-syntax-own-test-build301/receipt.json.gz) | [Whole log](frozen301/integration-syntax-own-test-build301/tests.log) |
| `integration-syntax-inventory301` | `listed`, exit 0, 0.003427s; 643 listed; no assertions | [Original receipt](frozen301/integration-syntax-inventory301/receipt.json.gz) | [Whole log](frozen301/integration-syntax-inventory301/tests.log) |
| `integration-syntax-all-tests301` | `failed`, exit 101, 1.416152s; 642 passed/1 failed | [Original receipt](frozen301/integration-syntax-all-tests301/receipt.json.gz) | [Whole log](frozen301/integration-syntax-all-tests301/tests.log) |

The [exact selection](frozen301/selections/syntax-all-selection301.json) and [original request](frozen301/selections/syntax-all-request301.json) retain the authentic 643-test inventory/pin gate. The [unchanged strict pin](frozen301/sealed-syntax-image/pinned-syntax301.json), [lossless storage record](frozen301/sealed-syntax-image/lossless-image-storage.json) and [full source companion](frozen301/sealed-syntax-image/source-snapshot.json.gz) preserve the independently successful image. All 98,607 source associations across three operations are byte-verified; question links attach only when the entire current source leaf equals the frozen producer.

Restore the measured payload with both compressed and original integrity checks:

```python
from pathlib import Path
import gzip, hashlib, json
base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen301/sealed-syntax-image")
record = json.loads((base / "lossless-image-storage.json").read_text())
packed = (base / "pinned-syntax301.elf.gz").read_bytes()
assert len(packed) == record["stored_bytes"]
assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
original = gzip.decompress(packed)
assert len(original) == record["original_bytes"]
assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
restored = Path("/tmp/pinned-syntax301.elf")
restored.write_bytes(original)
restored.chmod(0o755)
```

Restoration proves exact bytes and launches no assertion. Primitive getter/host stages, mathematical expression advice, source geometry and original Native issuer/result purposes remain independent.

## Source301 scoped Compiler, Registry and RuntimeAPI compilation

The separate scoped no-run command selects Compiler, Registry and RuntimeAPI and closes with exit 101 after 73.112772s and `uniform_source: true`. Six VM diagnostics report the missing `NativeStringUnavailable` to `ValueError` conversion at three original getter sites; the Compiler durable-inventory witness also reports an inaccessible field-pattern error. No test executable creation event is recorded. No successful producer, strict pin, inventory or assertion is claimed.

The [unchanged full original receipt](frozen301/integration-compiler-registry-api-own-test-build301/receipt.json.gz) preserves the complete immutable source image and command as lossless gzip. The [whole original log](frozen301/integration-compiler-registry-api-own-test-build301/tests.log) retains all seven compile diagnostics. All 32,869 source leaves are independently byte-verified. The independent Syntax `num-bigint` producer, list and full assertions keep their separate measured executable and outcomes. No Native provider failure, broader PASS or aggregate gate result is inferred.

## Source302 scoped Compiler, Registry and RuntimeAPI compilation

The scoped no-run command selects Compiler, Registry and RuntimeAPI and closes with exit 101 after 119.103976s and `uniform_source: true`. Core reports one E0624 error at `original_invocation.rs:412`: the selected metadata getter `permits_logical_source_names` is private. The original log also records one newly-created RuntimeAPI libtest artifact event. The overall failed operation supplies no successful producer, admitted strict pin, test inventory or assertion outcome for that partial artifact.

The [unchanged full original receipt](frozen302/integration-compiler-registry-api-own-test-build302/receipt.json.gz) preserves the complete immutable source image and command as lossless gzip. The [whole original log](frozen302/integration-compiler-registry-api-own-test-build302/tests.log) retains the visibility diagnostic and partial creation event. All 32,878 source leaves are independently byte-verified. The independent Syntax302 producer and any separately closed list/assertions keep their own source/image scope; no success is transferred to this failed command. No Native provider outcome or aggregate gate result is inferred.

## Source302 independent Syntax producer and full assertions

The separate Syntax `num-bigint` no-run command succeeds with its own strict image and source companion. Its fresh inventory lists 643 tests. The exact full selection closes 642 passed/1 failed: `scalar_getter::target_tests::original_long64_public_value_cache_and_live_failure_fields_match_captures`. Compilation and listing execute no assertions. The separate scoped Compiler/Registry/RuntimeAPI Source302 command remains independently failed; the unsealed RuntimeAPI creation event from that failed command is independent of this successful Syntax producer. No overall gate or Native primitive/provider PASS is claimed.

| Closed operation | Exact outcome | Original receipt | Whole log |
| --- | --- | --- | --- |
| `integration-syntax-own-test-build302` | `compile-passed`, exit 0, 6.613966s; compilation only; no assertions | [Original receipt](frozen302/integration-syntax-own-test-build302/receipt.json.gz) | [Whole log](frozen302/integration-syntax-own-test-build302/tests.log) |
| `integration-syntax-inventory302` | `listed`, exit 0, 0.002959s; 643 listed; no assertions | [Original receipt](frozen302/integration-syntax-inventory302/receipt.json.gz) | [Whole log](frozen302/integration-syntax-inventory302/tests.log) |
| `integration-syntax-all-tests302` | `failed`, exit 101, 0.769700s; 642 passed/1 failed | [Original receipt](frozen302/integration-syntax-all-tests302/receipt.json.gz) | [Whole log](frozen302/integration-syntax-all-tests302/tests.log) |

The [exact selection](frozen302/selections/syntax-all-selection302.json) and [original request](frozen302/selections/syntax-all-request302.json) retain the authentic 643-test inventory/pin gate. The [unchanged strict pin](frozen302/sealed-syntax-image/pinned-syntax302.json), [lossless storage record](frozen302/sealed-syntax-image/lossless-image-storage.json) and [full source companion](frozen302/sealed-syntax-image/source-snapshot.json.gz) preserve the independently successful image. All 98,634 source associations across three operations are byte-verified; question links attach only when the entire current source leaf equals the frozen producer.

Restore the measured payload with both compressed and original integrity checks:

```python
from pathlib import Path
import gzip, hashlib, json
base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen302/sealed-syntax-image")
record = json.loads((base / "lossless-image-storage.json").read_text())
packed = (base / "pinned-syntax302.elf.gz").read_bytes()
assert len(packed) == record["stored_bytes"]
assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
original = gzip.decompress(packed)
assert len(original) == record["original_bytes"]
assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
restored = Path("/tmp/pinned-syntax302.elf")
restored.write_bytes(original)
restored.chmod(0o755)
```

Restoration proves exact bytes and launches no assertion. Primitive getter/host stages, mathematical expression advice, source geometry and original Native issuer/result purposes remain independent.

## Source303 scoped Compiler, Registry and RuntimeAPI compilation

The scoped no-run command selects Compiler, Registry and RuntimeAPI and closes with exit 101 after 228.489463s and `uniform_source: true`. Compiler reports one E0308 fixture type error at `native_lowering/tests.rs:74`: the actual profile is passed directly where `LexerConfig::for_profile` requires `Option<&DialectProfile>`. The original log records newly-created RuntimeAPI and Registry libtest artifact events. Those events supply no successful producer, admitted strict pin, inventory or assertion from the overall failed command.

The [unchanged full original receipt](frozen303/integration-compiler-registry-api-own-test-build303/receipt.json.gz) preserves the complete immutable source image and command as lossless gzip. The [whole original log](frozen303/integration-compiler-registry-api-own-test-build303/tests.log) retains the fixture diagnostic and both partial creation events. All 32,894 source leaves are independently byte-verified. Any independent successful Registry/RuntimeAPI or Syntax producer and assertions retain their own command/source/image scope; no success is transferred to this failed invocation. No Native provider outcome or aggregate gate result is inferred.

## Source303 independent Registry/API producer and full assertions

The Registry/RuntimeAPI-only no-run command succeeds independently of the failed scoped Compiler/Registry/API command. Its actual Cargo events are fresh reused artifacts; the subsequent strict pins bind both exact executable byte streams to this successful command and immutable source. Registry lists and runs 1,723 tests, all passed. RuntimeAPI lists 85 tests and closes 84 passed/1 failed: `codegen_abi::tests::every_layout_constant_is_in_the_abi_fingerprint` reports that `NATIVE_PROC_STATUS_HOST_REFUSED` is missing from the ABI fingerprint. Build and listing execute no assertions. These separate crate results supply no combined gate, Compiler or new Native provider PASS.

| Closed operation | Exact outcome | Original receipt | Whole log |
| --- | --- | --- | --- |
| `integration-registry-api-own-test-build303` | `compile-passed`, exit 0, 0.396362s; no assertions | [Original receipt](frozen303/integration-registry-api-own-test-build303/receipt.json.gz) | [Whole log](frozen303/integration-registry-api-own-test-build303/tests.log) |
| `integration-registry-inventory303` | `listed`, exit 0, 0.011929s; no assertions | [Original receipt](frozen303/integration-registry-inventory303/receipt.json.gz) | [Whole log](frozen303/integration-registry-inventory303/tests.log) |
| `integration-registry-all-tests303` | `passed`, exit 0, 96.011552s; 1723 passed/0 failed | [Original receipt](frozen303/integration-registry-all-tests303/receipt.json.gz) | [Whole log](frozen303/integration-registry-all-tests303/tests.log) |
| `integration-api-inventory303` | `listed`, exit 0, 0.004371s; no assertions | [Original receipt](frozen303/integration-api-inventory303/receipt.json.gz) | [Whole log](frozen303/integration-api-inventory303/tests.log) |
| `integration-api-all-tests303` | `failed`, exit 101, 0.008227s; 84 passed/1 failed | [Original receipt](frozen303/integration-api-all-tests303/receipt.json.gz) | [Whole log](frozen303/integration-api-all-tests303/tests.log) |

The exact [Registry selection](frozen303/selections/registry-all-selection303.json), [Registry request](frozen303/selections/registry-all-request303.json), [API selection](frozen303/selections/api-all-selection303.json) and [API request](frozen303/selections/api-all-request303.json) retain both authentic pin/list gates. The [complete source companion](frozen303/sealed-registry-api-images/source-snapshot.json.gz) preserves all original32894 hashes and compile paths. All164470 source associations across the five operations are byte-verified. Current question links require the entire current leaf to equal the frozen producer, so later source controls and VM464 outcomes are not inferred.

Restore either measured executable with its own compressed and original integrity checks:

```python
from pathlib import Path
import gzip, hashlib, json
base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen303/sealed-registry-api-images")
for kind in ("registry", "api"):
    record = json.loads((base / f"lossless-{kind}-image-storage.json").read_text())
    packed = (base / f"pinned-{kind}303.elf.gz").read_bytes()
    assert len(packed) == record["stored_bytes"]
    assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
    original = gzip.decompress(packed)
    assert len(original) == record["original_bytes"]
    assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
    restored = Path(f"/tmp/pinned-{kind}303.elf")
    restored.write_bytes(original)
    restored.chmod(0o755)
```

Restoration proves exact payload bytes and launches no assertion. The unchanged [Registry pin](frozen303/sealed-registry-api-images/pinned-registry303.json) and [API pin](frozen303/sealed-registry-api-images/pinned-api303.json), with their [Registry storage](frozen303/sealed-registry-api-images/lossless-registry-image-storage.json) and [API storage](frozen303/sealed-registry-api-images/lossless-api-image-storage.json) records, preserve the original measured identities, including their recorded workspace executable paths.

## Source304 independent failed library-test builds

Both exact no-run commands use the immutable Source304 image and close with
exit101 and `uniform_source: true`. Each original log retains nine VM library
diagnostics: eight E0277 bounds for `ValueError: From<NativeStringUnavailable>`
and one E0609 access to nonexistent `Vm.result`. Compiler's VM development
dependency also makes the separately selected four-package command fail.

| Exact command scope | Seconds | Original full receipt | Original log | Test executable events |
| --- | --- | --- | --- | --- |
| Compiler, Registry, Syntax, CmdCore, VM, RuntimeAPI | 115.67499489701004 | [lossless gzip](frozen304/integration-shared-owner-own-test-build304/receipt.json.gz) | [whole log](frozen304/integration-shared-owner-own-test-build304/tests.log) | newly created Syntax, CmdCore and RuntimeAPI, all unsealed |
| Compiler, Syntax, CmdCore, RuntimeAPI | 11.142933130002348 | [lossless gzip](frozen304/integration-compiler-syntax-core-api-own-test-build304/receipt.json.gz) | [whole log](frozen304/integration-compiler-syntax-core-api-own-test-build304/tests.log) | none recorded |

Each decompressed receipt preserves the unchanged command, head, archive and
complete 32,931-leaf source inventory. Every archived source byte association is
independently verified; gzip restoration returns the full original receipt
bytes and SHA. Neither failed operation admits a whole successful producer,
strict pin, executable inventory or assertion. The partial creation events do
not establish assertion availability or success. Independent producer and
assertion records retain their own exact source/command/image scope. No Native
provider outcome, aggregate pass or mutable anchor refresh is inferred.

## Source305 independent Syntax/CmdCore/RuntimeAPI producer and assertions

The separate three-package no-run producer succeeds with actual empty Cargo
features and newly created Syntax, CmdCore and RuntimeAPI test executables.
Strict pins, exact inventories and complete crate selections retain each
original byte stream. Syntax runs its actual641 tests and closes640P1F; only
the new C84 primitive/callback join fixture panics on a missing parsed row.
CmdCore runs196P0F and RuntimeAPI85P0F. Build and listing run no assertions.
The three num-bigint-gated controls absent from this image are not ignored or
passed; no644 Syntax selection/result or combined gate pass is claimed.

| Closed operation | Exact outcome | Original receipt | Whole log |
| --- | --- | --- | --- |
| `integration-syntax-core-api-own-test-build305` | `compile-passed`, exit0, 18.003217028s; no assertions | [lossless original](frozen305/integration-syntax-core-api-own-test-build305/receipt.json.gz) | [whole log](frozen305/integration-syntax-core-api-own-test-build305/tests.log) |
| `integration-syntax-inventory305` | `listed`, exit0, 0.005740959s; no assertions | [lossless original](frozen305/integration-syntax-inventory305/receipt.json.gz) | [whole log](frozen305/integration-syntax-inventory305/tests.log) |
| `integration-syntax-all-tests305` | `failed`, exit101, 2.222358694s; 640 passed/1 failed | [lossless original](frozen305/integration-syntax-all-tests305/receipt.json.gz) | [whole log](frozen305/integration-syntax-all-tests305/tests.log) |
| `integration-cmd-core-inventory305` | `listed`, exit0, 0.002647606s; no assertions | [lossless original](frozen305/integration-cmd-core-inventory305/receipt.json.gz) | [whole log](frozen305/integration-cmd-core-inventory305/tests.log) |
| `integration-cmd-core-all-tests305` | `passed`, exit0, 0.447388498s; 196 passed/0 failed | [lossless original](frozen305/integration-cmd-core-all-tests305/receipt.json.gz) | [whole log](frozen305/integration-cmd-core-all-tests305/tests.log) |
| `integration-api-inventory305` | `listed`, exit0, 0.005490063s; no assertions | [lossless original](frozen305/integration-api-inventory305/receipt.json.gz) | [whole log](frozen305/integration-api-inventory305/tests.log) |
| `integration-api-all-tests305` | `passed`, exit0, 0.022546707s; 85 passed/0 failed | [lossless original](frozen305/integration-api-all-tests305/receipt.json.gz) | [whole log](frozen305/integration-api-all-tests305/tests.log) |

The [complete original source companion](frozen305/sealed-syntax-cmdcore-api-images/source-snapshot.json.gz) preserves all33,002 source hashes and source/head/compile/archive associations. All231,014 associations across these seven operations are byte-verified. The [Syntax selection](frozen305/selections/syntax-all-selection305.json), [CmdCore selection](frozen305/selections/cmd-core-all-selection305.json) and [API selection](frozen305/selections/api-all-selection305.json) preserve exact owned pins, inventory receipts and selection requests. Question links require whole-current-leaf equality with the frozen source, so changed fixtures and later controls receive no transferred assertion outcome. Independent Compiler/Registry/VM build failures remain failed, and original Native provider answers and mutable owner anchors remain unchanged.

Restore the three exact measured executable payloads with their own compressed/original integrity checks:

```python
from pathlib import Path
import gzip, hashlib, json
base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen305/sealed-syntax-cmdcore-api-images")
for kind in ("syntax", "cmd-core", "api"):
    record = json.loads((base / f"lossless-{kind}-image-storage.json").read_text())
    packed = (base / f"pinned-{kind}305.elf.gz").read_bytes()
    assert len(packed) == record["stored_bytes"]
    assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
    original = gzip.decompress(packed)
    assert len(original) == record["original_bytes"]
    assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
    restored = Path(f"/tmp/pinned-{kind}305.elf")
    restored.write_bytes(original)
    restored.chmod(0o755)
```

Restoration proves complete executable bytes and runs no assertion. Each unchanged strict pin and its lossless-storage record retains the successful own-producer/source/image identity.

## Source305 independent failed Compiler/Registry/VM library-test builds

Both exact no-run commands close exit101 with uniform_source true on the
unchanged 33002-leaf image. Each original log retains the same Compiler E0308
at registry_invocation/source_structure.rs:1730: state-transition descriptors
are compared where original CommandSpec/member descriptor identity is required.
The Registry/VM-only command also reaches this Compiler dependency. Neither
log records a test executable creation event; no successful producer, strict
Compiler/Registry/VM pin, inventory or assertion is admitted.

| Exact command scope | Seconds | Original full receipt | Original log |
| --- | --- | --- | --- |
| Compiler, Registry, VM | 63.459958148014266 | [lossless original](frozen305/integration-compiler-registry-vm-own-test-build305/receipt.json.gz) | [whole log](frozen305/integration-compiler-registry-vm-own-test-build305/tests.log) |
| Registry, VM | 29.331218568986515 | [lossless original](frozen305/integration-registry-vm-own-test-build305/receipt.json.gz) | [whole log](frozen305/integration-registry-vm-own-test-build305/tests.log) |

Each decompressed original receipt retains complete source/head/command and
compile/archive associations. Every source byte association is independently
verified, and gzip restoration returns the full original bytes and SHA. The
separate Syntax/CmdCore/RuntimeAPI producer, strict pins and full assertions
retain their own measured scope; no success is transferred to these failed
commands. Original Native provider answers and mutable owner anchors remain
unchanged, with no combined gate or later-source result inferred.

## Source306 failed six-owner library-test build

The exact Compiler/Registry/Syntax/CmdCore/VM/RuntimeAPI no-run command closes
exit101 after 97.97169901599409 seconds with uniform_source true on the unchanged
33006-leaf image, HEAD56331a02b691a085ded93416ce9dfe642f2070b0. The whole log
retains two Compiler errors: E0609 for NativeImports.completion_release at
native_emit.rs:1543 and E0369 for Option<StateTransitionDescriptor> inequality
at source_structure.rs:1743. This is a compile-only operation with no Rust
assertion, Native provider answer or overall gate result.

The three Syntax/CmdCore/RuntimeAPI creation events are explicitly unsealed.
No successful producer, strict image, inventory or assertion is admitted from
this failed command. Source305's independently successful Syntax/Core/API
operations and any other fresh producer retain their own measured scope.

The [complete original receipt](frozen306/integration-shared-owner-own-test-build306/receipt.json.gz)
is losslessly retained; decompression returns SHA-256 `c62946d0f659060066169f67f04a2e0d6f0123eac52bb02b2f4bcaa6bb7c35e4`. Its complete
source/head/command/archive associations are unchanged and every source leaf
is byte-verified. The [whole original log](frozen306/integration-shared-owner-own-test-build306/tests.log)
has SHA-256 `cd3398370de3952b02110cb565f2e045dd0aba0584128e712d0d1bb29e0f5c80`. Original sources, outcomes and mutable proof anchors
are preserved; no later-source result is donated to this operation.

## Source307 failed six-owner library-test build

The exact Compiler/Registry/Syntax/CmdCore/VM/RuntimeAPI no-run command closes
exit101 after 161.01232246900327 seconds with uniform_source true on the unchanged
33037-leaf image, HEAD56331a02b691a085ded93416ce9dfe642f2070b0. Four VM fixture errors are retained: undeclared tcl_host_c_abi at value_scalar_tests.rs:321; Rc<[u8]> conversion from &[u8;11] and &[u8;3] at command.rs:3977–3978; and E0308 at command.rs:3983.

The 4 actual executable artifact events remain unsealed. Their original
Cargo fresh/features fields are preserved; the failed command admits no
successful producer, strict pin, inventory or assertion for any package.
The [complete original receipt](frozen307/integration-shared-owner-own-test-build307/receipt.json.gz)
restores to SHA-256 `37004170fb7b9daa29d382bf0826d9451ca29992d11b534bd5048cd574060c68`. The [whole log](frozen307/integration-shared-owner-own-test-build307/tests.log)
has SHA-256 `52aae3f4a90dbe4e831923ca1cd53fdbdafa0168765b87208dcf14c0243fce36`. Every source association is byte-verified.
No later-source assertion, original Native answer or combined gate result is
transferred to this compile-only operation.


## Source308 failed six-owner library-test build

The exact Compiler/Registry/Syntax/CmdCore/VM/RuntimeAPI no-run command closes
exit101 after 154.49609750299715 seconds with uniform_source true on the unchanged
33041-leaf image, HEAD8177220bf85568de21dc1db5553d6d3d83450332. The Compiler fixture E0308 at analyser/oo.rs:6003 compares a selected CommandSpec pointer with an Option<StateTransitionDescriptor>; the complete compiler message and expected/actual types remain retained.

The 5 actual executable artifact events remain unsealed. Their original
Cargo fresh/features fields are preserved; the failed command admits no
successful producer, strict pin, inventory or assertion for any package.
The [complete original receipt](frozen308/integration-shared-owner-own-test-build308/receipt.json.gz)
restores to SHA-256 `8e99434b522d1c67ecf2a8e1a7809d12f0726464c0e546a9a64663a8ffbb761f`. The [whole log](frozen308/integration-shared-owner-own-test-build308/tests.log)
has SHA-256 `19d791683518a0229fe20485a2c914142a3e6d41437f2f87d3a54f1d9c9f97b7`. Every source association is byte-verified.
No later-source assertion, original Native answer or combined gate result is
transferred to this compile-only operation.

## Source309 six-owner producer, inventories and closed selections

The exact Compiler/Registry/Syntax/CmdCore/VM/RuntimeAPI own no-run command
succeeds on the unchanged33,097-leaf image. Its six exact executable payloads,
strict metadata and complete source companion are losslessly retained.
Actual Cargo fresh/features fields distinguish three reused outputs from
three produced outputs; compilation executes no assertion and compiles no
separate Runtime crate. The six actual inventories are Compiler9200, VM778,
Registry1724, Syntax645, CmdCore196 and RuntimeAPI85.

The complete Syntax645, CmdCore196 and RuntimeAPI85 selections each pass all
actual tests with zero failures, ignored, measured or filtered tests. The
separate Compiler primary31 selection closes8P23F with9169 filtered; every
failure remains in the original full log and ledger. Active VM55 and Compiler
alias30 selections are excluded from this archive. No complete Compiler,
VM, Registry, separate Runtime, Native provider or combined gate result is
inferred from these closed scopes.

| Closed operation | Exact outcome | Original receipt | Whole log |
| --- | --- | --- | --- |
| `integration-shared-owner-own-test-build309` | `compile-passed`, exit0, 327.232875651s; no assertions | [lossless original](frozen309/integration-shared-owner-own-test-build309/receipt.json.gz) | [whole log](frozen309/integration-shared-owner-own-test-build309/tests.log) |
| `integration-compiler-inventory309` | `listed`, exit0, 0.019228478s; no assertions | [lossless original](frozen309/integration-compiler-inventory309/receipt.json.gz) | [whole log](frozen309/integration-compiler-inventory309/tests.log) |
| `integration-vm-inventory309` | `listed`, exit0, 0.142167157s; no assertions | [lossless original](frozen309/integration-vm-inventory309/receipt.json.gz) | [whole log](frozen309/integration-vm-inventory309/tests.log) |
| `integration-registry-inventory309` | `listed`, exit0, 0.194193872s; no assertions | [lossless original](frozen309/integration-registry-inventory309/receipt.json.gz) | [whole log](frozen309/integration-registry-inventory309/tests.log) |
| `integration-syntax-inventory309` | `listed`, exit0, 0.003253155s; no assertions | [lossless original](frozen309/integration-syntax-inventory309/receipt.json.gz) | [whole log](frozen309/integration-syntax-inventory309/tests.log) |
| `integration-cmd-core-inventory309` | `listed`, exit0, 0.006875844s; no assertions | [lossless original](frozen309/integration-cmd-core-inventory309/receipt.json.gz) | [whole log](frozen309/integration-cmd-core-inventory309/tests.log) |
| `integration-api-inventory309` | `listed`, exit0, 0.004130934s; no assertions | [lossless original](frozen309/integration-api-inventory309/receipt.json.gz) | [whole log](frozen309/integration-api-inventory309/tests.log) |
| `integration-syntax-all-tests309` | `passed`, exit0, 1.701034346s; 645 passed/0 failed/0 filtered | [lossless original](frozen309/integration-syntax-all-tests309/receipt.json.gz) | [whole log](frozen309/integration-syntax-all-tests309/tests.log) |
| `integration-cmd-core-all-tests309` | `passed`, exit0, 0.361128833s; 196 passed/0 failed/0 filtered | [lossless original](frozen309/integration-cmd-core-all-tests309/receipt.json.gz) | [whole log](frozen309/integration-cmd-core-all-tests309/tests.log) |
| `integration-api-all-tests309` | `passed`, exit0, 0.018188826s; 85 passed/0 failed/0 filtered | [lossless original](frozen309/integration-api-all-tests309/receipt.json.gz) | [whole log](frozen309/integration-api-all-tests309/tests.log) |
| `integration-compiler-primary-tests309` | `failed`, exit101, 23.757505962s; 8 passed/23 failed/9169 filtered | [lossless original](frozen309/integration-compiler-primary-tests309/receipt.json.gz) | [whole log](frozen309/integration-compiler-primary-tests309/tests.log) |

The [complete original source companion](frozen309/sealed-six-owner-images/source-snapshot.json.gz) and exact [Syntax](frozen309/selections/syntax-all-selection309.json), [CmdCore](frozen309/selections/cmd-core-all-selection309.json), [RuntimeAPI](frozen309/selections/api-all-selection309.json) and [Compiler primary](frozen309/selections/compiler-primary-selection309.json) selections preserve original requests, strict pins and inventory associations. All364,067 source associations across these eleven operations are byte-verified. Current question links require exact whole-current-leaf equality with the immutable source; later controls receive no transferred assertion result. Original Native answers and mutable proof contexts remain unchanged.

Restore each complete measured executable with its recorded integrity checks:

```python
from pathlib import Path
import gzip, hashlib, json
base = Path("docs/design/analysis/name-resolution-proofs/rust-validation/frozen309/sealed-six-owner-images")
for kind in ("compiler", "vm", "registry", "syntax", "cmd-core", "api"):
    record = json.loads((base / f"lossless-{kind}-image-storage.json").read_text())
    packed = (base / f"pinned-{kind}309.elf.gz").read_bytes()
    assert len(packed) == record["stored_bytes"]
    assert hashlib.sha256(packed).hexdigest() == record["stored_sha256"]
    original = gzip.decompress(packed)
    assert len(original) == record["original_bytes"]
    assert hashlib.sha256(original).hexdigest() == record["original_sha256"]
    restored = Path(f"/tmp/pinned-{kind}309.elf")
    restored.write_bytes(original)
    restored.chmod(0o755)
```

Restoration runs no assertion and does not replace an active original executable. The unchanged strict pin and successful producer receipt retain each exact measured source/image identity.

## Source309 VM55 and independent diagnostic requests

The original VM55 selection closes53P2F with723 filtered. The Info186 control
fails on Jim imported-command resident bytes before its getter; the primitive
Boolean/storage control fails its Jim case5 follow-up numeric input. All55
individual outcomes and both failure bodies remain in the complete log. The
passing dictionary and mathop controls retain their independent original
comparison scopes; they cannot supply a complete VM, Info or scalar pass.

Two independent one-selector source-value requests each close0P1F. The first
uses TCL_LSP_TRACE_DIAGNOSTIC_SOURCE_VALUES, which the frozen source does not
consume. The second uses the genuine TCL_LSP_TRACE_DIAGNOSTIC_VALUES flag and
records both execution and diagnostic Overdefined values before the same
source-path assertion fails. Both original argument vectors and whole outputs
are retained independently. A separate single-selector alias diagnostic
request closes1P0F; its686.145391555-second duration supplies no performance
threshold, complete Alias30 result or convergence claim.

| Closed operation | Exact outcome | Original receipt | Whole log |
| --- | --- | --- | --- |
| `integration-vm-current-tests309` | 53 passed/2 failed/723 filtered; 410.938394467s | [lossless original](frozen309/integration-vm-current-tests309/receipt.json.gz) | [whole log](frozen309/integration-vm-current-tests309/tests.log) |
| `integration-compiler-source-value-trace309` | 0 passed/1 failed/9199 filtered; 0.797109190s | [lossless original](frozen309/integration-compiler-source-value-trace309/receipt.json.gz) | [whole log](frozen309/integration-compiler-source-value-trace309/tests.log) |
| `integration-compiler-source-value-correct-trace309` | 0 passed/1 failed/9199 filtered; 0.704667110s | [lossless original](frozen309/integration-compiler-source-value-correct-trace309/receipt.json.gz) | [whole log](frozen309/integration-compiler-source-value-correct-trace309/tests.log) |
| `integration-compiler-alias-performance309` | 1 passed/0 failed/9199 filtered; 686.145391555s | [lossless original](frozen309/integration-compiler-alias-performance309/receipt.json.gz) | [whole log](frozen309/integration-compiler-alias-performance309/tests.log) |
| `integration-runtime-own-test-build310` | compile-failed before compilation; no assertions; 0.102690900s | [lossless original](frozen310/integration-runtime-own-test-build310/receipt.json.gz) | [whole log](frozen310/integration-runtime-own-test-build310/tests.log) |

The exact [VM55 selection](frozen309/selections/vm-current-selection309.json) retains all778-test inventory and strict-image joins. These operations reuse the already retained lossless [VM](frozen309/sealed-six-owner-images/pinned-vm309.elf.gz) and [Compiler](frozen309/sealed-six-owner-images/pinned-compiler309.elf.gz) originals and [complete source companion](frozen309/sealed-six-owner-images/source-snapshot.json.gz). All165,541 source associations across the four309 operations and separate310 lock failure are independently byte-verified. No source/image payload is replaced, and no Native/provider answer or changed implementation context is refreshed. The single alias assertion completes all thirty source-analysis cases in its unchanged original fixture; those analyses are not separate test/process receipts.

The independent Runtime engine310 --locked command fails before compilation because its frozen lock requires resolution. Its full original lock-error output, exact command and33,153-leaf source associations remain independent of later lock maintenance and successful workspace309 images. No Runtime artifact, inventory or assertion is admitted from that command.

## Source311 proof controls and independent failed builds

The maintained Python boundary suite passes all17 fixed controls. The separate
full catalogue command fails with16 errors:15 changed current implementation
digests and the absent old completion-owner declaration. Its1075 questions,
4599 candidates and21300 corpus files have no pending candidates or unlinked
tests; those counts do not supply a successful gate. Each operation retains
its exact command, complete output and33278-leaf original source associations.

The Runtime engine no-run command fails on36 compiler errors; its50 dependency
artifact events contain no executable and remain unsealed. The independent
Compiler --locked no-run request fails before compilation and emits no
artifact event. Neither command admits an image, inventory or assertion.

| Closed operation | Exact outcome | Original receipt | Whole log |
| --- | --- | --- | --- |
| `integration-maintained-proof-boundaries311` | 17 Python controls passed; 0.239527417s | [lossless original](frozen311/integration-maintained-proof-boundaries311/receipt.json.gz) | [whole log](frozen311/integration-maintained-proof-boundaries311/tests.log) |
| `integration-proof-catalog-check311` | full catalogue failed:16 errors; 89.997229661s | [lossless original](frozen311/integration-proof-catalog-check311/receipt.json.gz) | [whole log](frozen311/integration-proof-catalog-check311/tests.log) |
| `integration-runtime-own-test-build311` | Runtime compilation failed:36 errors; no image; 61.457755113s | [lossless original](frozen311/integration-runtime-own-test-build311/receipt.json.gz) | [whole log](frozen311/integration-runtime-own-test-build311/tests.log) |
| `integration-compiler-own-test-build311` | Compiler lock failure before compilation; no image; 0.280501996s | [lossless original](frozen311/integration-compiler-own-test-build311/receipt.json.gz) | [whole log](frozen311/integration-compiler-own-test-build311/tests.log) |

The two [lock-maintenance provenance](frozen311/lock-provenance/runtime-lock-maintained311/receipt.json) retain exact before/after lock bytes, original cargo metadata receipts, empty original stderr and lossless whole metadata output. All package versions, package sources and checksums remain equal within each pair. Runtime lock maintenance is already captured by Source311; workspace maintenance312 independently adds the actual `tcl-host-c-abi` dependency roster entry to `tcl-runtime`. Neither maintenance command is a compiler, test-image, Runtime libc or assertion measurement. Original failed build receipts and provider answers remain unchanged.

- `runtime-lock-maintained311`: [original command/receipt](frozen311/lock-provenance/runtime-lock-maintained311/receipt.json), [before](frozen311/lock-provenance/runtime-lock-maintained311/before.lock), [after](frozen311/lock-provenance/runtime-lock-maintained311/after.lock), [whole metadata](frozen311/lock-provenance/runtime-lock-maintained311/metadata.json.gz), [stderr](frozen311/lock-provenance/runtime-lock-maintained311/stderr.log). The independently maintained Runtime lock is already the exact after bytes in Source311; the later Runtime compilation still fails.

- `workspace-lock-maintained312`: [original command/receipt](frozen311/lock-provenance/workspace-lock-maintained312/receipt.json), [before](frozen311/lock-provenance/workspace-lock-maintained312/before.lock), [after](frozen311/lock-provenance/workspace-lock-maintained312/after.lock), [whole metadata](frozen311/lock-provenance/workspace-lock-maintained312/metadata.json.gz), [stderr](frozen311/lock-provenance/workspace-lock-maintained312/stderr.log). The Compiler311 failure uses the exact before bytes. Workspace maintenance312 supplies separate after bytes and no Source311 build/image success.
