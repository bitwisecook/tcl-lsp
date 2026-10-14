# Core consumer API build

The Core test library fails to compile because the already-public LogicalSourceObjectMembers facade is not re-exported from analyser for its sibling consumer. Zero semantic tests execute; captured source-subset drift is zero. The subsequent API export adds no constructor or ownership grant.
