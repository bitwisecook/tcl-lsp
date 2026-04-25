"""BIG-IP object registry catalogue."""

from .data import (
    HEADER_KIND_MAP,
    KIND_SPECS,
    OBJECT_KIND_SPECS,
    PROPERTY_REFERENCE_SPECS,
)
from .models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)

__all__ = [
    "BigipObjectSpec",
    "BigipObjectKindSpec",
    "BigipPropertySpec",
    "KIND_SPECS",
    "OBJECT_KIND_SPECS",
    "PROPERTY_REFERENCE_SPECS",
    "HEADER_KIND_MAP",
]
