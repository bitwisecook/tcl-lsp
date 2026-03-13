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
            "sys_internal_proxy",
            module="sys",
            object_types=("internal-proxy",),
        ),
        header_types=(("sys", "internal-proxy"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="dns-resolver", value_type="string"),
            BigipPropertySpec(
                name="proxy-server-pool", value_type="reference", references=("ltm_pool",)
            ),
            BigipPropertySpec(
                name="route-domain", value_type="reference", references=("net_route_domain",)
            ),
        ),
    )
