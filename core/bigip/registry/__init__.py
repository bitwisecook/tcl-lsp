"""BIG-IP object registry catalogue."""

from .data import (
    HEADER_KIND_MAP,
    KIND_SPECS,
    OBJECT_KIND_SPECS,
    PROPERTY_NAMES_BY_TYPE,
    PROPERTY_REFERENCE_SPECS,
    PROPERTY_SPECS_BY_TYPE,
    property_names_for,
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
    "PROPERTY_NAMES_BY_TYPE",
    "PROPERTY_SPECS_BY_TYPE",
    "HEADER_KIND_MAP",
    "property_names_for",
]
