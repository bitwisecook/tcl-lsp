package com.tcllsp.jetbrains.actions

import com.google.gson.JsonParser
import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertNotNull

class TclLspActionsTest {
    @Test
    fun renderDiagramMermaidProjectsStructuredPayload() {
        val payload = JsonParser.parseString(
            """
            {
              "events": [{
                "name": "HTTP_REQUEST",
                "priority": 400,
                "multiplicity": "once",
                "flow": [{
                  "kind": "if",
                  "branches": [{
                    "condition": "path == /",
                    "body": [{"kind": "action", "label": "pool web"}]
                  }]
                }]
              }],
              "procedures": [{
                "name": "helper",
                "params": ["request"],
                "flow": [{"kind": "return", "value": "request"}]
              }]
            }
            """.trimIndent()
        )

        val mermaid = assertNotNull(renderDiagramMermaid(payload))
        assertContains(mermaid, "flowchart TD")
        assertContains(mermaid, "when HTTP_REQUEST (priority 400, once)")
        assertContains(mermaid, "proc helper(request)")
        assertContains(mermaid, "path == /")
        assertContains(mermaid, "pool web")
        assertContains(mermaid, "-->|")
    }

    @Test
    fun renderDiagramMermaidRejectsMissingStructuredFields() {
        val payload = JsonParser.parseString("{\"events\": []}")
        kotlin.test.assertNull(renderDiagramMermaid(payload))
    }
}
