"""Import all code definition modules to populate the registry.

Importing this module guarantees all diagnostic and optimisation codes
are registered and queryable via :mod:`core.common.codes`.
"""

from core.common import (  # noqa: F401
    codes_bigip,
    codes_error,
    codes_hint,
    codes_iapp,
    codes_irules,
    codes_optimiser,
    codes_shimmer,
    codes_taint,
    codes_tk,
    codes_warning,
    codes_xc,
)
