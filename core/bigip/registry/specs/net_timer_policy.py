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
            "net_timer_policy",
            module="net",
            object_types=("timer-policy",),
        ),
        header_types=(("net", "timer-policy"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="rules",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="destination-ports",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="ip-protocol", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="timers",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="value", value_type="string", in_sections=("timers",)),
            BigipPropertySpec(name="timers", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="net", value_type="string"),
            BigipPropertySpec(name="idle-flow-policy", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="rules", value_type="string", in_sections=("idle-flow-policy",)),
            BigipPropertySpec(name="r1", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(name="ip-protocol", value_type="string", in_sections=("r1",)),
            BigipPropertySpec(name="destination-ports", value_type="string", in_sections=("r1",)),
            BigipPropertySpec(
                name="http", value_type="list", in_sections=("destination-ports",), repeated=True
            ),
            BigipPropertySpec(
                name="webcache",
                value_type="list",
                in_sections=("destination-ports",),
                repeated=True,
            ),
            BigipPropertySpec(name="timers", value_type="string", in_sections=("r1",)),
            BigipPropertySpec(
                name="flow-idle-timeout", value_type="string", in_sections=("timers",)
            ),
            BigipPropertySpec(
                name="value", value_type="string", in_sections=("flow-idle-timeout",)
            ),
            BigipPropertySpec(name="r2", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(name="ip-protocol", value_type="string", in_sections=("r2",)),
            BigipPropertySpec(name="destination-ports", value_type="string", in_sections=("r2",)),
            BigipPropertySpec(name="flow-idle-policy", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="rules", value_type="string", in_sections=("flow-idle-policy",)),
            BigipPropertySpec(
                name="all-other",
                value_type="list",
                in_sections=("destination-ports",),
                repeated=True,
            ),
            BigipPropertySpec(name="r3", value_type="string", in_sections=("flow-idle-policy",)),
            BigipPropertySpec(name="ip-protocol", value_type="string", in_sections=("r3",)),
            BigipPropertySpec(name="destination-ports", value_type="string", in_sections=("r3",)),
            BigipPropertySpec(name="timers", value_type="string", in_sections=("r3",)),
            BigipPropertySpec(name="r4", value_type="string"),
            BigipPropertySpec(name="ip-protocol", value_type="string", in_sections=("r4",)),
            BigipPropertySpec(name="destination-ports", value_type="string", in_sections=("r4",)),
            BigipPropertySpec(name="timers", value_type="string", in_sections=("r4",)),
        ),
    )
