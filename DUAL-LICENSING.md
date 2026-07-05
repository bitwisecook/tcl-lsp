# Dual Licensing

`tcl-lsp` is offered under **two** licensing options. You may choose whichever
fits your use.

## 1. Open-source license (default): GNU AGPL-3.0-or-later

Unless you have a separate written agreement with the copyright holder, this
software is licensed to you under the **GNU Affero General Public License,
version 3 or (at your option) any later version** (AGPL-3.0-or-later). The full
text is in [`LICENSE`](LICENSE).

The AGPL is a strong copyleft license. In particular, if you modify this
software and make it available to users over a network — for example as part of
a hosted or SaaS product — the AGPL requires you to offer those users the
**complete corresponding source code** of your modified version under the same
terms.

You are free to use, study, modify, and redistribute the software — including
**commercially and for paid products** — provided you comply with the AGPL. For
network and hosted use, "comply" means publishing your corresponding source. If
that copyleft obligation is compatible with your product, no further permission
is required.

## 2. Commercial license (on request)

If you want to use `tcl-lsp` in a product **without** the AGPL's copyleft
obligations — for example, to build a proprietary or closed-source product, or a
hosted/paid service whose source you do not wish to publish — a separate
**commercial license** is available.

To obtain one, contact the copyright holder:

- **James Deucker** (**bitwisecook**)
- GitHub: <https://github.com/bitwisecook>

Commercial licensing terms (scope, pricing, support) are agreed individually.
A commercial license removes the AGPL source-disclosure requirement for the
covered use; it does not change the terms for anyone using the software under
the AGPL.

> Note: The AGPL does not — and legally cannot — forbid commercial or paid use;
> its condition is source disclosure, not payment. The commercial license
> exists so that organisations who cannot or prefer not to meet that
> source-disclosure obligation have a lawful alternative.

## 3. Third-party components

This repository incorporates third-party code that is **not** covered by either
option above. Such components remain under **their own original licenses and
belong to their original authors**, and nothing in this project's licensing
grants or restricts rights in them beyond what those licenses provide. Notable
examples include:

- **Tcl runtime headers** (`runtime/zig/regex_include/`) — the vendored Tcl
  internal headers used to build the regex shim (for example `tclInt.h` and
  `regcustom.h`) are derived from the Tcl source and remain under the standard
  Tcl/BSD-style license.
- **Tcllib packages used by the test suite** (`tests/external/tcllib/`) — the
  bundled `counter` and `tcltest` packages are retained verbatim under their
  original Tcl/BSD-style license; see the accompanying `license.terms` files
  ([`tests/external/tcllib/counter/license.terms`](tests/external/tcllib/counter/license.terms)
  and
  [`tests/external/tcllib/tcltest/license.terms`](tests/external/tcllib/tcltest/license.terms)).

The Zig Tcl runtime under `runtime/zig/` (outside the vendored
`regex_include/` headers) is an independent, from-scratch implementation of Tcl
semantics; its source is original work under the licensing options above.

When a third-party component's license imposes obligations (such as retaining a
copyright notice), those obligations continue to apply regardless of which of
the two options above you choose.
