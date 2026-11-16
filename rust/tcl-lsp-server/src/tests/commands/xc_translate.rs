// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Document translation to F5 Distributed Cloud.

use super::*;

/// `tcl-lsp.xcTranslateDocument` translates the open document under its
/// own analysis and names what it describes; a stale, unknown, non-iRules
/// or source-only request is refused rather than reparsed standalone.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn xc_translate_document_uses_the_current_open_document() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///xc-document.irul").unwrap();
    let src = "when HTTP_REQUEST {\n    pool my_pool\n}\n";
    let mut state = DocumentState::new(src.to_owned(), "f5-irules".to_owned());
    state.version = Some(4);
    backend
        .documents
        .lock("test")
        .await
        .insert(uri.clone(), state);
    backend
        .db_set_source(&uri, src, "f5-irules".to_owned())
        .await
        .expect("the fixture retains its actual checked SourceFile owner");
    let request = |uri: &Uri, version: i32, source: &str| {
        vec![
            serde_json::json!({ "uri": uri.as_str(), "version": version, "source": source }),
            serde_json::json!("both"),
        ]
    };

    let document = backend
        .xc_translate_document_command(&request(&uri, 4, src))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(document["source"]["purpose"], "document", "{document}");
    assert_eq!(document["source"]["uri"], uri.as_str(), "{document}");
    assert_eq!(document["source"]["version"], 4, "{document}");
    assert_eq!(document["source"]["dialect"], "f5-irules", "{document}");
    let standalone = backend
        .xc_translate_command(&[serde_json::json!(src), serde_json::json!("both")])
        .await
        .unwrap();
    assert_eq!(document["items"], standalone["items"]);
    assert_eq!(document["coverage_pct"], standalone["coverage_pct"]);

    for (args, reason) in [
        (request(&uri, 3, src), "stale"),
        (request(&uri, 4, "when HTTP_REQUEST {}\n"), "stale"),
        (
            request(&Uri::from_str("file:///missing.irul").unwrap(), 4, src),
            "notOpen",
        ),
        (
            vec![serde_json::json!(src), serde_json::json!("both")],
            "request",
        ),
    ] {
        let refused = backend
            .xc_translate_document_command(&args)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(refused, serde_json::json!({ "unavailable": reason }));
    }

    let tcl_uri = Uri::from_str("file:///xc-document.tcl").unwrap();
    let mut tcl = DocumentState::new(src.to_owned(), "tcl8.6".to_owned());
    tcl.version = Some(1);
    backend
        .documents
        .lock("test")
        .await
        .insert(tcl_uri.clone(), tcl);
    backend
        .db_set_source(&tcl_uri, src, "tcl8.6".to_owned())
        .await
        .expect("the fixture retains its actual checked SourceFile owner");
    let refused = backend
        .xc_translate_document_command(&request(&tcl_uri, 1, src))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(refused, serde_json::json!({ "unavailable": "notIrules" }));
}
