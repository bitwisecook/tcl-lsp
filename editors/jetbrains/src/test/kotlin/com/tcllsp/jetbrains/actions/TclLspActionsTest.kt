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
    fun onlyExactLowerCaseDefaultConsumesTheNoMatchPath() {
        fun render(pattern: String): String = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"switch","subject":"kind","arms":[
              {"pattern":"$pattern","body":[{"kind":"action","label":"arm"}]}
            ]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )))

        val defaultArm = render("default")
        kotlin.test.assertFalse(defaultArm.contains("|no match|"), defaultArm)

        // Mutation guard: a case-insensitive default check wrongly removes
        // this edge for normal Tcl patterns such as `Default` and `DEFAULT`.
        for (ordinaryPattern in listOf("Default", "DEFAULT")) {
            val mermaid = render(ordinaryPattern)
            assertContains(mermaid, "|no match|")
            assertContains(mermaid, "switch join")
        }
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

    @Test
    fun tryCompletionContractRoutesOnlyHandledPathsPastFinally() {
        val payload = JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"fail","completion":"error"}],"handlers":[
              {"kind_handler":"on","match":"error","fallthrough":false,"body":[{"kind":"action","label":"recover"}]}
            ],"finally":[{"kind":"action","label":"cleanup"}]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )

        val mermaid = assertNotNull(renderDiagramMermaid(payload))
        assertContains(mermaid, "n1 --> n2")
        assertContains(mermaid, "n2 -->|on| n3")
        assertContains(mermaid, "n3 --> n4")
        kotlin.test.assertFalse(mermaid.contains("n2 --> n5"), mermaid)
        assertContains(mermaid, "n4 --> n5")
        assertContains(mermaid, "n5 --> n6")
        assertContains(mermaid, "n6 --> n7")
        kotlin.test.assertFalse(mermaid.contains("try join"), mermaid)
    }

    @Test
    fun returnAndUnhandledErrorRunFinallyButDoNotReachContinuation() {
        fun render(completion: String): String = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"leave","completion":"$completion"}],
              "finally":[{"kind":"action","label":"cleanup"}]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )))

        for (completion in listOf("return", "error", "break", "continue", "terminal")) {
            val mermaid = render(completion)
            assertContains(mermaid, "n2 --> n3")
            assertContains(mermaid, "n3 --> n4")
            kotlin.test.assertFalse(mermaid.contains("after"), mermaid)
        }
    }

    @Test
    fun normalFinallyPathContinuesAndFinallyCompletionOverridesIt() {
        fun render(finallyCompletion: String? = null): String {
            val completion = finallyCompletion?.let { ",\"completion\":\"$it\"" } ?: ""
            return assertNotNull(renderDiagramMermaid(JsonParser.parseString(
                """{"events":[{"name":"E","flow":[{"kind":"try","body":[{"kind":"action","label":"work"}],
                  "finally":[{"kind":"action","label":"cleanup"$completion}]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
            )))
        }

        val normal = render()
        assertContains(normal, "n5[\"after\"]")
        assertContains(normal, "n4 --> n5")

        val override = render("return")
        kotlin.test.assertFalse(override.contains("after"), override)
    }

    @Test
    fun tryHandlerFallthroughTargetsTheNextHandlerBodyBeforeFinally() {
        val payload = JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"fail","completion":"error"}],"handlers":[
              {"kind_handler":"on","match":"error","fallthrough":true,"body":[]},
              {"kind_handler":"on","match":"return","fallthrough":false,"body":[{"kind":"action","label":"shared body"}]}
            ],"finally":[{"kind":"action","label":"cleanup"}]}]}],"procedures":[]}"""
        )

        val mermaid = assertNotNull(renderDiagramMermaid(payload))
        assertContains(mermaid, "n2 -->|on| n3")
        assertContains(mermaid, "n3 -->|fall through| n4")
        assertContains(mermaid, "n4 --> n5")
        assertContains(mermaid, "n5 --> n6")
        kotlin.test.assertFalse(mermaid.contains("n3 --> n6"), mermaid)
    }

    @Test
    fun loopsHaveBackEdgesAndDistinctExitTails() {
        fun render(label: String, exit: String): String = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"loop","label":"$label","exit":"$exit","body":[{"kind":"action","label":"body"}]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )))

        for ((label, exit) in listOf(
            "while ready" to "false",
            "for" to "false",
            "foreach item" to "exhausted",
        )) {
            val mermaid = render(label, exit)
            assertContains(mermaid, "n0 --> n1")
            assertContains(mermaid, "n1 --> n2")
            assertContains(mermaid, "n2 -->|repeat| n1")
            assertContains(mermaid, "n1 -->|$exit| n3")
            assertContains(mermaid, "n3 --> n4")
        }
    }
}
