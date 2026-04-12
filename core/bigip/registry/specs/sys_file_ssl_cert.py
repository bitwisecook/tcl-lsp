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
            "sys_file_ssl_cert",
            module="sys",
            object_types=("file ssl-cert",),
        ),
        header_types=(("sys", "file ssl-cert"),),
        properties=(
            BigipPropertySpec(name="source-path", value_type="string"),
            BigipPropertySpec(
                name="cert-validation-options",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "ocsp"),
            ),
            BigipPropertySpec(name="cert-validators", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="issuer-cert", value_type="boolean", allow_none=True),
        ),
    )
