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
            "gtm_region",
            module="gtm",
            object_types=("region",),
        ),
        header_types=(("gtm", "region"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="continent", value_type="string"),
            BigipPropertySpec(name="country", value_type="string"),
            BigipPropertySpec(name="datacenter", value_type="string"),
            BigipPropertySpec(name="geoip-isp", value_type="string"),
            BigipPropertySpec(name="isp", value_type="string"),
            BigipPropertySpec(name="not", value_type="string"),
            BigipPropertySpec(name="pool", value_type="reference", references=("ltm_pool",)),
            BigipPropertySpec(name="region-name", value_type="string"),
            BigipPropertySpec(name="state", value_type="string"),
        ),
    )
