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
            "ltm_persistence_sip",
            module="ltm",
            object_types=("persistence sip",),
        ),
        header_types=(("ltm", "persistence sip"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("ltm_persistence_sip",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="match-across-pools", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="match-across-services", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="match-across-virtuals", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="mirror", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="override-connection-limit",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="sip-info",
                value_type="enum",
                allow_none=True,
                enum_values=("call-id", "from", "none", "sip-etag", "subject", "to"),
            ),
            BigipPropertySpec(name="timeout", value_type="integer"),
        ),
    )
