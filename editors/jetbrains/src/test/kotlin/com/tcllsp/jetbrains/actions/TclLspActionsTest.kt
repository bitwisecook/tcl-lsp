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

    @Test
    fun conditionalBranchesAllJoinAndNoElseKeepsFalsePath() {
        val payload = JsonParser.parseString(
            """
            {"events":[{"name":"HTTP_REQUEST","flow":[{"kind":"if","branches":[
              {"condition":"a","body":[{"kind":"action","label":"pool a"}]},
              {"condition":"b","body":[{"kind":"action","label":"pool b"}]}
            ]},{"kind":"action","label":"after"}]}],"procedures":[]}
            """.trimIndent()
        )

        val mermaid = assertNotNull(renderDiagramMermaid(payload))
        assertContains(mermaid, "pool a")
        assertContains(mermaid, "pool b")
        assertContains(mermaid, "|false|")
        assertContains(mermaid, "after")
        assertContains(mermaid, "if join")
        assertContains(mermaid, "n1 -->|a| n2")
        assertContains(mermaid, "n1 -->|b| n4")
        assertContains(mermaid, "n3 --> n6")
        assertContains(mermaid, "n5 --> n6")
        assertContains(mermaid, "n1 -->|false| n6")
        assertContains(mermaid, "n6 --> n7")
    }

    @Test
    fun explicitElseDoesNotAddImplicitFalseEdge() {
        val payload = JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"if","branches":[
              {"condition":"a","body":[{"kind":"action","label":"then"}]},
              {"condition":"b","body":[{"kind":"action","label":"elseif"}]},
              {"condition":"else","body":[{"kind":"action","label":"else"}]}
            ]},{"kind":"action","label":"after"}]}],"procedures":[]}""".trimIndent()
        )
        val mermaid = assertNotNull(renderDiagramMermaid(payload))
        assertContains(mermaid, "n1 -->|a| n2")
        assertContains(mermaid, "n1 -->|b| n4")
        assertContains(mermaid, "n1 -->|else| n6")
        assertContains(mermaid, "n3 --> n8")
        assertContains(mermaid, "n5 --> n8")
        assertContains(mermaid, "n7 --> n8")
        assertContains(mermaid, "n8 --> n9")
        kotlin.test.assertFalse(mermaid.contains("|false|"))
    }

    @Test
    fun nestedConditionJoinsPropagateToOuterContinuation() {
        val payload = JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"if","branches":[
              {"condition":"outer","body":[{"kind":"if","branches":[
                {"condition":"inner","body":[{"kind":"action","label":"inner then"}]}
              ]}]},
              {"condition":"other","body":[{"kind":"action","label":"other"}]}
            ]},{"kind":"action","label":"after"}]}],"procedures":[]}""".trimIndent()
        )
        val mermaid = assertNotNull(renderDiagramMermaid(payload))
        assertContains(mermaid, "n5 --> n6")
        assertContains(mermaid, "n3 -->|false| n6")
        assertContains(mermaid, "n6 --> n9")
        assertContains(mermaid, "n8 --> n9")
        assertContains(mermaid, "n9 --> n10")
    }

    @Test
    fun switchArmsJoinBeforeTheContinuationAndKeepNoMatchWithoutDefault() {
        val payload = JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"switch","subject":"kind","arms":[
              {"pattern":"a","body":[{"kind":"action","label":"first"}]},
              {"pattern":"b","body":[{"kind":"action","label":"second"}]}
            ]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )
        val mermaid = assertNotNull(renderDiagramMermaid(payload))
        assertContains(mermaid, "n1 -->|a| n2")
        assertContains(mermaid, "n1 -->|b| n4")
        assertContains(mermaid, "n3 --> n6")
        assertContains(mermaid, "n5 --> n6")
        assertContains(mermaid, "n1 -->|no match| n6")
        assertContains(mermaid, "n6 --> n7")
    }

    @Test
    fun switchDefaultConsumesNoMatchAndNestedSwitchStillJoins() {
        val payload = JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"switch","subject":"kind","arms":[
              {"pattern":"a","body":[{"kind":"switch","subject":"inner","arms":[
                {"pattern":"x","body":[{"kind":"action","label":"inner"}]}
              ]}]},
              {"pattern":"default","body":[{"kind":"action","label":"fallback"}]}
            ]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )
        val mermaid = assertNotNull(renderDiagramMermaid(payload))
        assertContains(mermaid, "switch join")
        assertContains(mermaid, "after")
        // The inner switch has no default, but the outer one does.  This is
        // a mutation guard: deleting the per-switch default check adds a
        // second no-match edge from the outer decision.
        kotlin.test.assertEquals(1, Regex("\\|no match\\|").findAll(mermaid).count(), mermaid)
    }

    @Test
    fun edgeCaptionPipesAreHtmlEscaped() {
        val payload = JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"switch","subject":"kind","arms":[
              {"pattern":"a | b","body":[{"kind":"action","label":"first"}]},
              {"pattern":"foo|bar","body":[{"kind":"action","label":"second"}]}
            ]}]}],"procedures":[]}"""
        )
        val mermaid = assertNotNull(renderDiagramMermaid(payload))
        assertContains(mermaid, "|a &#124; b|")
        assertContains(mermaid, "|foo&#124;bar|")
        kotlin.test.assertFalse(mermaid.contains("|a | b|"), mermaid)
    }
}
