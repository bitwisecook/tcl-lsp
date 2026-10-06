# Native RegExp cache and range controls

`probe.c` uses the actual five canonical Tcl headers and static libraries named in `manifest.json`. The manifest retains source, header, library and executable SHA256 identities, compile commands, exit statuses and original output.

Cases 0 and 9 invalidate only the resident pattern string and reach the same compiled RegExp cache. Case 1 changes flags with a resident pattern; case 2 fails compilation without installing RegExp. Cases 3–5 match raw FF, modified NUL and four-byte astral input and inspect original subject/range storage before asking for result bytes. Cases 6–8 distinguish general RegExp substitution, literal mapping and same-subject no-match results.

Case 10 asks for different compile flags after invalidating a pattern whose RegExp primary has no string updater. Each native engine aborts; these attempted captures establish no successful completion. The checked adapters report unavailable string access at that boundary.
