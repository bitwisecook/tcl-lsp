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
            "security_ip_intelligence_policy",
            module="security",
            object_types=("ip-intelligence policy",),
        ),
        header_types=(("security", "ip-intelligence policy"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="reference", default="none"),
            BigipPropertySpec(
                name="blacklist-categories",
                value_type="list",
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("blacklist-categories",),
                        enum_values=("accept", "drop", "use-policy-setting"),
                    ),
                    BigipPropertySpec(
                        name="app-service",
                        value_type="unknown",
                        in_sections=("blacklist-categories",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="description",
                        value_type="unknown",
                        in_sections=("blacklist-categories",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="log-blacklist-hit-only",
                        value_type="enum",
                        in_sections=("blacklist-categories",),
                        enum_values=("no", "use-policy-setting", "yes"),
                    ),
                    BigipPropertySpec(
                        name="log-blacklist-whitelist-hit",
                        value_type="enum",
                        in_sections=("blacklist-categories",),
                        enum_values=("no", "use-policy-setting", "yes"),
                    ),
                    BigipPropertySpec(
                        name="match-direction-override",
                        value_type="enum",
                        in_sections=("blacklist-categories",),
                        enum_values=(
                            "match-destination",
                            "match-source",
                            "match-source-and-destination",
                        ),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("blacklist-categories",),
                enum_values=("accept", "drop", "use-policy-setting"),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="unknown",
                in_sections=("blacklist-categories",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="description",
                value_type="unknown",
                in_sections=("blacklist-categories",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="log-blacklist-hit-only",
                value_type="enum",
                in_sections=("blacklist-categories",),
                enum_values=("no", "use-policy-setting", "yes"),
            ),
            BigipPropertySpec(
                name="log-blacklist-whitelist-hit",
                value_type="enum",
                in_sections=("blacklist-categories",),
                enum_values=("no", "use-policy-setting", "yes"),
            ),
            BigipPropertySpec(
                name="match-direction-override",
                value_type="enum",
                in_sections=("blacklist-categories",),
                enum_values=("match-destination", "match-source", "match-source-and-destination"),
            ),
            BigipPropertySpec(
                name="default-action",
                value_type="enum",
                enum_values=("accept", "drop"),
            ),
            BigipPropertySpec(
                name="default-log-blacklist-hit-only",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="default-log-blacklist-whitelist-hit",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="feed-lists",
                value_type="list",
                references=("security_ip_intelligence_feed_list",),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
        ),
    )
