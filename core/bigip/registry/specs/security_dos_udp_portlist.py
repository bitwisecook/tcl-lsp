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
            "security_dos_udp_portlist",
            module="security",
            object_types=("dos udp-portlist",),
        ),
        header_types=(("security", "dos udp-portlist"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="list-type",
                value_type="enum",
                enum_values=("exclude-listed-ports", "include-listed-ports"),
            ),
            BigipPropertySpec(
                name="entries", value_type="enum", enum_values=("modify", "replace-all-with")
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("entries",)),
            BigipPropertySpec(
                name="match-direction",
                value_type="enum",
                in_sections=("entries",),
                allow_none=True,
                enum_values=("both", "dst", "none", "src"),
            ),
            BigipPropertySpec(name="port-number", value_type="string", in_sections=("entries",)),
            BigipPropertySpec(name="security", value_type="string"),
            BigipPropertySpec(name="entries", value_type="string", in_sections=("security",)),
            BigipPropertySpec(name="entry1", value_type="string", in_sections=("entries",)),
            BigipPropertySpec(name="match-direction", value_type="string", in_sections=("entry1",)),
            BigipPropertySpec(name="port-number", value_type="string", in_sections=("entry1",)),
            BigipPropertySpec(
                name="entry2", value_type="list", in_sections=("entries",), repeated=True
            ),
            BigipPropertySpec(
                name="entry3", value_type="list", in_sections=("entries",), repeated=True
            ),
            BigipPropertySpec(
                name="entry4", value_type="list", in_sections=("entries",), repeated=True
            ),
        ),
    )
