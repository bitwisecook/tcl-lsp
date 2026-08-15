package com.tcllsp.jetbrains.actions

import com.google.gson.JsonParser
import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertEquals
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
    fun processExitBypassesFinallyAndTheContinuation() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"exit 0","completion":"process_exit"}],
              "finally":[{"kind":"action","label":"must not run"}]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )))
        assertContains(mermaid, "n1 --> n2")
        kotlin.test.assertFalse(mermaid.contains("finally"), mermaid)
        kotlin.test.assertFalse(mermaid.contains("must not run"), mermaid)
        kotlin.test.assertFalse(mermaid.contains("after"), mermaid)
    }

    @Test
    fun returnCompletionSelectsOnReturnRatherThanOnError() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"return -code error","completion":"return"}],"handlers":[
              {"kind_handler":"on","match":"error","fallthrough":false,"body":[{"kind":"action","label":"wrong"}]},
              {"kind_handler":"on","match":"return","fallthrough":false,"body":[{"kind":"action","label":"handled"}]}
            ],"finally":[{"kind":"action","label":"cleanup"}]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )))
        kotlin.test.assertFalse(mermaid.contains("wrong"), mermaid)
        assertContains(mermaid, "n2 -->|on| n4")
        assertContains(mermaid, "handled")
        assertContains(mermaid, "after")
    }

    @Test
    fun dynamicOptionsRetainEveryPossibleOnHandlerAndNormalPath() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"return -options ${'$'}opts","completion":"dynamic"}],"handlers":[
              {"kind_handler":"on","match":"ok","fallthrough":false,"body":[{"kind":"action","label":"ok path"}]},
              {"kind_handler":"on","match":"error","fallthrough":false,"body":[{"kind":"action","label":"error path"}]},
              {"kind_handler":"on","match":"return","fallthrough":false,"body":[{"kind":"action","label":"return path"}]},
              {"kind_handler":"on","match":"42","fallthrough":false,"body":[{"kind":"action","label":"other path"}]}
            ],"finally":[{"kind":"action","label":"cleanup"}]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )))
        // Dynamic -options can be ok, error, return, break, continue, or a
        // custom code. All source-order `on` candidates remain represented.
        for (handler in listOf("ok path", "error path", "return path", "other path")) {
            assertContains(mermaid, handler)
        }
        assertContains(mermaid, "after")
    }

    @Test
    fun dynamicDefaultCodeIsOnlyReturnOrErrorAndDoesNotInventOk() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"return -code ${'$'}code","completion":"dynamic_return_or_error"}],"handlers":[
              {"kind_handler":"on","match":"ok","fallthrough":false,"body":[{"kind":"action","label":"wrong ok"}]},
              {"kind_handler":"on","match":"error","fallthrough":false,"body":[{"kind":"action","label":"error path"}]},
              {"kind_handler":"on","match":"return","fallthrough":false,"body":[{"kind":"action","label":"return path"}]}
            ],"finally":[{"kind":"action","label":"cleanup"}]}]}],"procedures":[]}"""
        )))
        kotlin.test.assertFalse(mermaid.contains("wrong ok"), mermaid)
        assertContains(mermaid, "error path")
        assertContains(mermaid, "return path")
    }

    @Test
    fun dynamicCompletionUsesOnlyTheFirstOnHandlerForEachConcreteCode() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"return -code ${'$'}code","completion":"dynamic_return_or_error"}],"handlers":[
              {"kind_handler":"on","match":"error","fallthrough":false,"body":[{"kind":"action","label":"first error"}]},
              {"kind_handler":"on","match":"error","fallthrough":false,"body":[{"kind":"action","label":"duplicate error"}]},
              {"kind_handler":"on","match":"return","fallthrough":false,"body":[{"kind":"action","label":"first return"}]},
              {"kind_handler":"on","match":"return","fallthrough":false,"body":[{"kind":"action","label":"duplicate return"}]}
            ]}]}],"procedures":[]}"""
        )))
        // Handler nodes are retained for source context, but only the first
        // matching on-clause for error and return receives a body edge.
        assertEquals(2, mermaid.lines().count { it.startsWith("n2 -->|on|") }, mermaid)
    }

    @Test
    fun broadDynamicCompletionKeepsDistinctCustomOnCodes() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"return -options ${'$'}options","completion":"dynamic"}],"handlers":[
              {"kind_handler":"on","match":"42","fallthrough":false,"body":[{"kind":"action","label":"first 42"}]},
              {"kind_handler":"on","match":"42","fallthrough":false,"body":[{"kind":"action","label":"duplicate 42"}]},
              {"kind_handler":"on","match":"43","fallthrough":false,"body":[{"kind":"action","label":"43 path"}]},
              {"kind_handler":"on","match":"custom","fallthrough":false,"body":[{"kind":"action","label":"custom path"}]}
            ]}]}],"procedures":[]}"""
        )))
        // 42, 43, and custom are independently possible; only duplicate 42
        // is shadowed by Tcl's first matching on-clause rule.
        assertEquals(3, mermaid.lines().count { it.startsWith("n2 -->|on|") }, mermaid)
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
