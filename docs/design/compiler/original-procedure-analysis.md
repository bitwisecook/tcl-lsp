# Original procedure analysis owners

Procedure names are source data and reporting labels. `ProcedureId` identifies a body issued by one actual `Module`; consumers retain that module, its current source image, complete analysis input, configuration and declaration receipts.

`Module::original_source_procedure_bodies` owns the source inventory. It includes primary procedure bodies and independently validated original declaration bodies. The Native allocation-backed declaration inventory, Native-name source cards, Logical declaration cards and Hosted declaration cards keep separate purposes. Anonymous and installed-only bodies do not become original source declarations. A source declaration supplies no installed command, executing body, Native formal ABI or physical frame.

| Query | Current owner requirements |
| --- | --- |
| `CompilationUnit::function_for_body` | Same Module issuer, one actual body store and a matching FunctionUnit body stamp. `None` selects the actual root unit. |
| `CompilationUnit::original_procedure_for_body` | Membership in the shared current original source inventory. |
| `CompilationUnit::original_procedure_cfg_for_body` | Original source inventory plus the independently prepared CFG retained before lattice analysis. |
| `LatticeRequest::original_source_procedure` | Current complete source, issued body, original body coordinates, formal vector and offset-zero original IR agree. |
| `CompilationUnit::accepts_original_procedure_lattice` | Current retained header, source/input/store and prepared CFG agree with the already-computed candidate after rebasing. A refusal withdraws its downstream baseline key. |

`OriginalNativeProcedureBody` retains a unanimous original Native-name declaration header and its `OriginalProcedureReadFlowEntry`. The complete input, source/configuration/Registry, name operand, formal topology, original body and Lowerer-issued Script must agree with the actual current Module entry and body row. The separate Native source-card inventory supplies readonly `OriginalDeclaration` units without a successful implementation allocation or publication; those units are excluded from executable roots. Missing Native source ownership cannot borrow Logical or Hosted cards.

Original declaration lattices retain their actual body ID and formals under the declaration's conditional source context. Their physical frame remains unknown. The source context independently retained at each CFG site owns any more precise source facts. Taint summaries, non-taint checks and optimisation slices join this same inventory by actual body ID; methods retain their separate instance-state overlay.

`FunctionUnit::original_hosted_procedure_body` holds the exact original Hosted card from the current Module after the card validates the source, declaration, body, input and Registry. Consumers compare the held Arc with the Module card and revalidate its current inputs. Event receipts stay separate; a hosted source procedure is not an executing iRules event or a Native procedure allocation.

Raw memo inputs retain actual identities and complete dependencies. Private correspondence certificates can permit reuse only for their supported source and dependency cases. Cached facts rebind to the current original owner, and the current prepared CFG must still agree. Neither equality nor an unchanged display name supplies callable, variable-cell, compiler, object, frame or execution authority.
