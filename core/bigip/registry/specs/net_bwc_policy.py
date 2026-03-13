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
            "net_bwc_policy",
            module="net",
            object_types=("bwc policy",),
        ),
        header_types=(("net", "bwc policy"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="dynamic", value_type="boolean"),
            BigipPropertySpec(name="max-rate", value_type="integer"),
            BigipPropertySpec(name="max-user-rate", value_type="integer"),
            BigipPropertySpec(name="max-user-rate-pps", value_type="integer"),
            BigipPropertySpec(name="ip-tos", value_type="integer"),
            BigipPropertySpec(name="link-qos", value_type="integer"),
            BigipPropertySpec(name="measure", value_type="boolean"),
            BigipPropertySpec(name="log-publisher", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="log-period", value_type="integer"),
            BigipPropertySpec(name="categories", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="max-cat-rate", value_type="integer", in_sections=("categories",)
            ),
            BigipPropertySpec(
                name="max-cat-rate-percentage", value_type="integer", in_sections=("categories",)
            ),
            BigipPropertySpec(name="ip-tos", value_type="integer", in_sections=("categories",)),
            BigipPropertySpec(name="link-qos", value_type="integer", in_sections=("categories",)),
            BigipPropertySpec(
                name="traffic-priority-map", value_type="string", in_sections=("categories",)
            ),
            BigipPropertySpec(name="traffic-priority-map", value_type="string"),
            BigipPropertySpec(name="net", value_type="string"),
            BigipPropertySpec(name="max-rate", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="categories", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="web", value_type="string", in_sections=("categories",)),
            BigipPropertySpec(name="description", value_type="string", in_sections=("web",)),
            BigipPropertySpec(name="max-cat-rate", value_type="string", in_sections=("web",)),
            BigipPropertySpec(name="ip-tos", value_type="string", in_sections=("web",)),
            BigipPropertySpec(name="link-qos", value_type="string", in_sections=("web",)),
            BigipPropertySpec(name="description", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="dynamic", value_type="boolean", in_sections=("net",)),
            BigipPropertySpec(name="max-user-rate", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="measure", value_type="boolean", in_sections=("net",)),
            BigipPropertySpec(name="log-period", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="action", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="bwc", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="logging", value_type="boolean", in_sections=("net",)),
            BigipPropertySpec(name="order", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="rule", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="ltm", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string", in_sections=("ltm",)),
            BigipPropertySpec(name="mask", value_type="string", in_sections=("ltm",)),
            BigipPropertySpec(name="profiles", value_type="string", in_sections=("ltm",)),
            BigipPropertySpec(name="rules", value_type="string"),
            BigipPropertySpec(
                name="translate-address",
                value_type="boolean",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="translate-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(name="vlans", value_type="reference", references=("net_vlan",)),
            BigipPropertySpec(name="ip-protocol", value_type="string", in_sections=("ltm",)),
            BigipPropertySpec(
                name="tcp", value_type="list", in_sections=("profiles",), repeated=True
            ),
            BigipPropertySpec(name="rules", value_type="string", in_sections=("ltm",)),
            BigipPropertySpec(
                name="translate-address",
                value_type="boolean",
                in_sections=("ltm",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="cat1", value_type="string", in_sections=("categories",)),
            BigipPropertySpec(name="max-cat-rate", value_type="string", in_sections=("cat1",)),
            BigipPropertySpec(
                name="traffic-priority-map", value_type="string", in_sections=("cat1",)
            ),
            BigipPropertySpec(name="cat2", value_type="string", in_sections=("categories",)),
            BigipPropertySpec(name="max-cat-rate", value_type="string", in_sections=("cat2",)),
            BigipPropertySpec(
                name="traffic-priority-map", value_type="string", in_sections=("cat2",)
            ),
            BigipPropertySpec(
                name="max-cat-rate-percentage", value_type="string", in_sections=("web",)
            ),
            BigipPropertySpec(name="ip-tos", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="link-qos", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="log-publisher", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="when", value_type="string"),
            BigipPropertySpec(name="set", value_type="string", in_sections=("when",)),
            BigipPropertySpec(name="if", value_type="list", in_sections=("when",), repeated=True),
            BigipPropertySpec(name="log", value_type="string", in_sections=("when",)),
            BigipPropertySpec(name="incr", value_type="string"),
        ),
    )
