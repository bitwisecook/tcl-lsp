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
            "ltm_dns_cache_transparent",
            module="ltm",
            object_types=("dns cache transparent",),
        ),
        header_types=(("ltm", "dns cache transparent"),),
        properties=(
            BigipPropertySpec(
                name="answer-default-zones",
                value_type="enum",
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="local-zones",
                value_type="list",
                list_operators=frozenset(("add",)),
            ),
            BigipPropertySpec(name="msg-cache-size", value_type="integer"),
            BigipPropertySpec(
                name="response-policy-zones",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify")),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("response-policy-zones",),
                enum_values=("nxdomain", "walled-garden"),
            ),
            BigipPropertySpec(
                name="walled-garden",
                value_type="unknown",
                in_sections=("response-policy-zones",),
            ),
            BigipPropertySpec(name="rrset-cache-size", value_type="integer"),
            BigipPropertySpec(
                name="rrset-rotate",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "query-id"),
            ),
        ),
    )
