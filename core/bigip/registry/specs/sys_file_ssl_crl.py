from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "sys_file_ssl_crl",
            module="sys",
            object_types=("file ssl-crl",),
        ),
        header_types=(("sys", "file ssl-crl"),),
        properties=(BigipPropertySpec(name="source-path", value_type="string"),),
    )
