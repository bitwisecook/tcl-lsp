"""F5-specific CLI verbs for the ``f5`` command.

Verbs here use the verb registry in :mod:`explorer.verbs.f5._registry`,
which is independent of the ``tcl`` / ``irule`` registry under
:mod:`explorer.verbs._registry`.
"""


def load_verbs() -> None:
    """Import all ``@verb``-decorated f5 modules, triggering their registrations."""
    from . import (  # noqa: F401
        cleanup,
        completion,
        convert,
        diff,
        enrich_pcapng,
        enrich_wireshark,
        explain,
        explain_flow,
        extract,
        fetch,
        graph,
        grep,
        merge,
        pcap_remap,
        pull,
        push,
        query,
        redact,
        rename,
        split,
        stats,
        tmsh,
        unredact,
        validate,
    )
