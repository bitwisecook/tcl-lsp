"""Import all modules that register diagnostic and optimisation codes.

Importing this module guarantees all codes are registered and queryable
via :mod:`shared.codes`.  Each module registers its codes via
``@diag(...)`` / ``@opt(...)`` decorators at import time.
"""

# analysis
# compiler
import compiler.compiler_checks  # noqa: F401
import compiler.gvn  # noqa: F401
import compiler.irules_flow  # noqa: F401
import compiler.optimiser._branch_folding  # noqa: F401
import compiler.optimiser._code_sinking  # noqa: F401
import compiler.optimiser._elimination  # noqa: F401
import compiler.optimiser._pattern_recognition  # noqa: F401
import compiler.optimiser._propagation  # noqa: F401
import compiler.optimiser._structure_elimination  # noqa: F401
import compiler.optimiser._tail_call  # noqa: F401
import compiler.optimiser._unused_procs  # noqa: F401
import compiler.shimmer  # noqa: F401
import compiler.taint._sinks  # noqa: F401
import compiler.taint._uri_split  # noqa: F401
import core.analysis  # noqa: F401
import core.analysis._analyser._utils  # noqa: F401
import core.analysis.checks._domain  # noqa: F401
import core.analysis.checks._security  # noqa: F401
import core.analysis.checks._style  # noqa: F401
import core.analysis.checks._syntax  # noqa: F401
import core.analysis.irules_checks  # noqa: F401
import core.bigip.iapp_diagnostics  # noqa: F401
import core.bigip.validator  # noqa: F401

# extensions
import core.tk.diagnostics  # noqa: F401
import core.xc.translator  # noqa: F401
