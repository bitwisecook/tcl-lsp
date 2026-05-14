"""``cm ha-group`` — hand-maintained alias of ``sys ha-group``.

The 21.0 manpage corpus only generates ``sys_ha_group.json``; the
device, however, also emits stanzas under the ``cm ha-group``
header on some 13.x / 14.x releases (and ``cm traffic-group``
references its HA group via this kind in :mod:`cm_traffic_group`).

The spec stays hand-maintained until the corpus carries a canonical
``cm_ha_group.json`` (or the device emission is confirmed gone).
A spec-regeneration audit lists every kind without a matching
manpage source and trips on hand-maintained kinds NOT in this
allowlist so dead entries can't slip in unnoticed.
"""

from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "cm_ha_group",
            module="cm",
            object_types=("ha-group",),
        ),
        header_types=(("cm", "ha-group"),),
        properties=(),
    )
