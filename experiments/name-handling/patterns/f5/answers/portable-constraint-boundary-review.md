# Earlier portable failures and stock Wiki constraints

This independent review covers fourteen remaining portable matrix rows and eight stock Wiki matrix rows. They retain thirty-two actual measurement identities because each aggregate stock row has separate stock Tcl 8.4 and 8.5 captures. Every row remains unreached; no canonical final-operation or body result is promoted. This bounded review does not finish the broader outstanding question task.

| Matrix case | Matrix context | Suggested terminal boundary | Actual capture pointers |
| --- | --- | --- | --- |
| `ensemble-prefix-map` | `tmsh-cli-script` | `original-setup-subcommand-rejected` | `tmsh-cli-script` [nh20261012exact1/records.json](nh20261012exact1/records.json) `/93` |
| `ensemble-prefix-map` | `iapp-implementation` | `original-setup-subcommand-rejected` | `iapp-implementation` [nh20261012exact1/records.json](nh20261012exact1/records.json) `/3` |
| `ensemble-prefix-map` | `icall` | `original-setup-subcommand-rejected` | `icall` [nh20261012exact1/records.json](nh20261012exact1/records.json) `/48` |
| `ensemble-prefix-map` | `iapp-apl` | `original-setup-subcommand-rejected` | `iapp-apl` [nh20261012aplexact1/records.json](nh20261012aplexact1/records.json) `/3` |
| `oo-prefix-deleted-near-miss` | `tmsh-cli-script` | `original-setup-command-missing` | `tmsh-cli-script` [nh20261012exact1/records.json](nh20261012exact1/records.json) `/101` |
| `oo-prefix-deleted-near-miss` | `iapp-implementation` | `original-setup-command-missing` | `iapp-implementation` [nh20261012exact1/records.json](nh20261012exact1/records.json) `/11` |
| `oo-prefix-deleted-near-miss` | `appliance-stock-tclsh` | `original-setup-command-missing` | `stock-8.4` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/11`; `stock-8.5` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/56` |
| `oo-prefix-deleted-near-miss` | `icall` | `original-setup-command-missing` | `icall` [nh20261012exact1/records.json](nh20261012exact1/records.json) `/56` |
| `oo-prefix-deleted-near-miss` | `iapp-apl` | `original-setup-command-missing` | `iapp-apl` [nh20261012aplexact1/records.json](nh20261012aplexact1/records.json) `/11` |
| `oo-self-prefix` | `tmsh-cli-script` | `original-setup-command-missing` | `tmsh-cli-script` [nh20261012exact1/records.json](nh20261012exact1/records.json) `/102` |
| `oo-self-prefix` | `iapp-implementation` | `original-setup-command-missing` | `iapp-implementation` [nh20261012exact1/records.json](nh20261012exact1/records.json) `/12` |
| `oo-self-prefix` | `appliance-stock-tclsh` | `original-setup-command-missing` | `stock-8.4` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/12`; `stock-8.5` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/57` |
| `oo-self-prefix` | `icall` | `original-setup-command-missing` | `icall` [nh20261012exact1/records.json](nh20261012exact1/records.json) `/57` |
| `oo-self-prefix` | `iapp-apl` | `original-setup-command-missing` | `iapp-apl` [nh20261012aplexact1/records.json](nh20261012aplexact1/records.json) `/12` |
| `wiki-code-7` | `appliance-stock-tclsh` | `harness-constraint-false-version` | `stock-8.4` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/24`; `stock-8.5` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/69` |
| `wiki-oo-1` | `appliance-stock-tclsh` | `harness-constraint-false-command-surface` | `stock-8.4` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/38`; `stock-8.5` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/83` |
| `wiki-oo-2` | `appliance-stock-tclsh` | `harness-constraint-false-command-surface` | `stock-8.4` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/39`; `stock-8.5` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/84` |
| `wiki-oo-3` | `appliance-stock-tclsh` | `harness-constraint-false-command-surface` | `stock-8.4` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/40`; `stock-8.5` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/85` |
| `wiki-oo-4` | `appliance-stock-tclsh` | `harness-constraint-false-command-surface` | `stock-8.4` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/41`; `stock-8.5` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/86` |
| `wiki-oo-5` | `appliance-stock-tclsh` | `harness-constraint-false-command-surface` | `stock-8.4` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/42`; `stock-8.5` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/87` |
| `wiki-oo-6` | `appliance-stock-tclsh` | `harness-constraint-false-command-surface` | `stock-8.4` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/43`; `stock-8.5` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/88` |
| `wiki-oo-7` | `appliance-stock-tclsh` | `harness-constraint-false-command-surface` | `stock-8.4` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/44`; `stock-8.5` [nh20261012stock2/records.json](nh20261012stock2/records.json) `/89` |

The ensemble captures fail on the original `namespace ensemble create` during the first namespace-eval command, before the final facade invocation. The TclOO captures fail on original `oo::class create` at outer eval line 1, before either final prefix operation. Successful wrapper setup does not establish successful original setup. These are exact setup-command failure boundaries; they do not answer the later final lookup or whether another runtime/optional package could provide the command.

The eight Wiki rows have matching real tcltest selection and named constraint skips on each stock interpreter. `wiki-code-7` requires the authored `noAunt` version condition Tcl>=8.6; each capture’s own oracle header records a lower version. `wiki-oo-1` through `wiki-oo-7` require the authored `tcloo` test of ordinary `::oo::class` command presence. Its false constraint proves that particular suite interpreter's command-surface test failed at that point. The selected body was not entered, no body-return receipt exists, and the successful whole-suite wrapper completion is not a body result. A skip alone supplies no semantic unsupported answer or optional-package capability conclusion.

| Stock Wiki captured interpreter | Own real tcltest version | Selected body applicability |
| --- | --- | --- |
| Tcl 8.4.13 | 2.2.8 | `noAunt` false; `tcloo` false |
| Tcl 8.5.13 | 2.3.4 | `noAunt` false; `tcloo` false |

BIG-IP version/build are retained independently as 21.1.0.1/0.0.26. Hosted APL records retain their own run's 8.4.6 callback version. Other portable per-case captures do not record a Tcl patchlevel; those fields remain null. The Wiki patchlevels above are not donated to the separate portable captures.

[reviewed-boundaries.json](nh20261012unreached2/reviewed-boundaries.json) hashes original source, exact operation materialisation, source ranges, complete error-stack chunks and real harness transcripts. It retains each context/unit/invocation separately and excludes authentication/security metadata. Existing sealed captures, matrix and central manifest were not changed.
