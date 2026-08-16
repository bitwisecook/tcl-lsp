package com.tcllsp.jetbrains.actions

import com.google.gson.JsonParser
import com.google.gson.JsonPrimitive
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
    fun exhaustiveAbruptConditionalDoesNotReachItsContinuation() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"if","branches":[
              {"condition":"a","body":[{"kind":"action","label":"then return","completion":"return"}]},
              {"condition":"else","body":[{"kind":"action","label":"else return","completion":"return"}]}
            ]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )))
        assertContains(mermaid, "then return")
        assertContains(mermaid, "else return")
        kotlin.test.assertFalse(mermaid.contains("if join"), mermaid)
        kotlin.test.assertFalse(mermaid.contains("after"), mermaid)
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
    fun exhaustiveAbruptSwitchDoesNotReachItsContinuation() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"switch","subject":"kind","arms":[
              {"pattern":"a","body":[{"kind":"action","label":"first return","completion":"return"}]},
              {"pattern":"default","body":[{"kind":"action","label":"default return","completion":"return"}]}
            ]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )))
        assertContains(mermaid, "first return")
        assertContains(mermaid, "default return")
        kotlin.test.assertFalse(mermaid.contains("switch join"), mermaid)
        kotlin.test.assertFalse(mermaid.contains("after"), mermaid)
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
              {"kind_handler":"on","match":"error","completion_code":1,"fallthrough":false,"body":[{"kind":"action","label":"recover"}]}
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
    fun dynamicExitRoutesOnlyItsErrorPathThroughTryFinallyAndAfter() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"exit ${'$'}status","completion":"exit_or_error"}],"handlers":[
              {"kind_handler":"on","match":"error","completion_code":1,"fallthrough":false,"body":[{"kind":"action","label":"handled status error"}]}],
              "finally":[{"kind":"action","label":"cleanup"}]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )))
        // The typed union supplies exactly one catchable branch. Its sibling
        // process-exit branch is deliberately not connected to finally or
        // after, while the on-error branch remains visible through both.
        assertEquals(1, mermaid.lines().count { it.contains("-->|on|") }, mermaid)
        assertContains(mermaid, "n2 -->|on| n3")
        assertContains(mermaid, "n4 --> n5")
        kotlin.test.assertFalse(mermaid.contains("n2 --> n5"), mermaid)
        assertContains(mermaid, "handled status error")
        assertContains(mermaid, "finally")
        assertContains(mermaid, "cleanup")
        assertContains(mermaid, "after")
    }

    @Test
    fun catchAbsorbsEveryTclCompletionButNotProcessExit() {
        fun render(completion: String, code: Int? = null): String {
            val completionCode = code?.let { ",\"completion_code\":$it" } ?: ""
            return assertNotNull(renderDiagramMermaid(JsonParser.parseString(
                """{"events":[{"name":"E","flow":[{"kind":"catch","body":[
                  {"kind":"action","label":"caught","completion":"$completion"$completionCode}]},
                  {"kind":"action","label":"after"}]}],"procedures":[]}"""
            )))
        }

        for ((completion, code) in listOf(
            "error" to null,
            "return" to null,
            "break" to null,
            "custom" to 42,
        )) {
            val mermaid = render(completion, code)
            assertContains(mermaid, "caught", message = completion)
            assertContains(mermaid, "after", message = completion)
        }

        val processExit = render("process_exit")
        assertContains(processExit, "caught")
        kotlin.test.assertFalse(processExit.contains("after"), processExit)
    }

    @Test
    fun emptyTrapExhaustivelyShadowsLaterExactErrorHandlers() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"fail","completion":"error"}],"handlers":[
              {"kind_handler":"trap","match":"","trap_pattern":[],"fallthrough":false,"body":[{"kind":"action","label":"empty trap body"}]},
              {"kind_handler":"trap","match":"X","trap_pattern":["X"],"fallthrough":false,"body":[{"kind":"action","label":"shadowed trap body"}]},
              {"kind_handler":"on","match":"error","completion_code":1,"fallthrough":false,"body":[{"kind":"action","label":"shadowed on body"}]}
            ]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )))
        assertContains(mermaid, "empty trap body")
        assertContains(mermaid, "after")
        kotlin.test.assertFalse(mermaid.contains("shadowed trap body"), mermaid)
        kotlin.test.assertFalse(mermaid.contains("shadowed on body"), mermaid)
        assertEquals(1, mermaid.lines().count { it.contains("-->|trap|") }, mermaid)
        assertEquals(0, mermaid.lines().count { it.contains("-->|on|") }, mermaid)
    }

    @Test
    fun emptyTrapExhaustivelyShadowsLaterDynamicErrorHandlers() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"dynamic result","completion":"dynamic_return_or_error"}],"handlers":[
              {"kind_handler":"trap","match":"","trap_pattern":[],"fallthrough":false,"body":[{"kind":"action","label":"empty trap body"}]},
              {"kind_handler":"trap","match":"X","trap_pattern":["X"],"fallthrough":false,"body":[{"kind":"action","label":"shadowed trap body"}]},
              {"kind_handler":"on","match":"error","completion_code":1,"fallthrough":false,"body":[{"kind":"action","label":"shadowed on body"}]},
              {"kind_handler":"on","match":"return","completion_code":2,"fallthrough":false,"body":[{"kind":"action","label":"return body"}]}
            ]}]}],"procedures":[]}"""
        )))
        assertContains(mermaid, "empty trap body")
        assertContains(mermaid, "return body")
        kotlin.test.assertFalse(mermaid.contains("shadowed trap body"), mermaid)
        kotlin.test.assertFalse(mermaid.contains("shadowed on body"), mermaid)
        assertEquals(1, mermaid.lines().count { it.contains("-->|trap|") }, mermaid)
        assertEquals(1, mermaid.lines().count { it.contains("-->|on|") }, mermaid)
    }

    @Test
    fun returnCompletionSelectsOnReturnRatherThanOnError() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"return -code error","completion":"return"}],"handlers":[
              {"kind_handler":"on","match":"error","completion_code":1,"fallthrough":false,"body":[{"kind":"action","label":"wrong"}]},
              {"kind_handler":"on","match":"return","completion_code":2,"fallthrough":false,"body":[{"kind":"action","label":"handled"}]}
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
              {"kind_handler":"on","match":"ok","completion_code":0,"fallthrough":false,"body":[{"kind":"action","label":"ok path"}]},
              {"kind_handler":"on","match":"error","completion_code":1,"fallthrough":false,"body":[{"kind":"action","label":"error path"}]},
              {"kind_handler":"on","match":"return","completion_code":2,"fallthrough":false,"body":[{"kind":"action","label":"return path"}]},
              {"kind_handler":"on","match":"42","completion_code":42,"fallthrough":false,"body":[{"kind":"action","label":"other path"}]}
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
    fun dynamicNormalOutcomeRoutesOnceThroughOnOkAndFinally() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"return -options ${'$'}opts","completion":"dynamic"}],"handlers":[
              {"kind_handler":"on","match":"ok","completion_code":0,"fallthrough":false,"body":[{"kind":"action","label":"ok path"}]}
            ],"finally":[{"kind":"action","label":"cleanup"}]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )))
        assertEquals(1, mermaid.lines().count { it == "n2 -->|on| n3" }, mermaid)
        assertEquals(1, mermaid.lines().count { it == "n4 --> n5" }, mermaid)
        assertEquals(1, mermaid.lines().count { it == "n5 --> n6" }, mermaid)
        assertEquals(1, mermaid.lines().count { it == "n6 --> n7" }, mermaid)
    }

    @Test
    fun dynamicNormalOutcomeRoutesOnceThroughFinallyWithoutAHandler() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"return -options ${'$'}opts","completion":"dynamic"}],"finally":[
              {"kind":"action","label":"cleanup"}]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )))
        assertEquals(1, mermaid.lines().count { it == "n2 --> n3" }, mermaid)
        assertEquals(1, mermaid.lines().count { it == "n3 --> n4" }, mermaid)
        assertEquals(1, mermaid.lines().count { it == "n4 --> n5" }, mermaid)
    }

    @Test
    fun dynamicCompletionOutsideTryRetainsItsNormalContinuation() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[
              {"kind":"action","label":"return -options ${'$'}options","completion":"dynamic"},
              {"kind":"action","label":"after"}
            ]}],"procedures":[]}"""
        )))
        assertContains(mermaid, "return -options ${'$'}options")
        assertContains(mermaid, "after")
        assertContains(mermaid, "n1 --> n2")
    }

    @Test
    fun dynamicDefaultCodeIsOnlyReturnOrErrorAndDoesNotInventOk() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"return -code ${'$'}code","completion":"dynamic_return_or_error"}],"handlers":[
              {"kind_handler":"on","match":"ok","completion_code":0,"fallthrough":false,"body":[{"kind":"action","label":"wrong ok"}]},
              {"kind_handler":"on","match":"error","completion_code":1,"fallthrough":false,"body":[{"kind":"action","label":"error path"}]},
              {"kind_handler":"on","match":"return","completion_code":2,"fallthrough":false,"body":[{"kind":"action","label":"return path"}]}
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
              {"kind_handler":"on","match":"error","completion_code":1,"fallthrough":false,"body":[{"kind":"action","label":"first error"}]},
              {"kind_handler":"on","match":"error","completion_code":1,"fallthrough":false,"body":[{"kind":"action","label":"duplicate error"}]},
              {"kind_handler":"on","match":"return","completion_code":2,"fallthrough":false,"body":[{"kind":"action","label":"first return"}]},
              {"kind_handler":"on","match":"return","completion_code":2,"fallthrough":false,"body":[{"kind":"action","label":"duplicate return"}]}
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
              {"kind_handler":"on","match":"42","completion_code":42,"fallthrough":false,"body":[{"kind":"action","label":"first 42"}]},
              {"kind_handler":"on","match":"42","completion_code":42,"fallthrough":false,"body":[{"kind":"action","label":"duplicate 42"}]},
              {"kind_handler":"on","match":"43","completion_code":43,"fallthrough":false,"body":[{"kind":"action","label":"43 path"}]},
              {"kind_handler":"on","match":"return","completion_code":2,"fallthrough":false,"body":[{"kind":"action","label":"return path"}]}
            ]}]}],"procedures":[]}"""
        )))
        // 42, 43, and custom are independently possible; only duplicate 42
        // is shadowed by Tcl's first matching on-clause rule.
        assertEquals(3, mermaid.lines().count { it.startsWith("n2 -->|on|") }, mermaid)
    }

    @Test
    fun canonicalWrappedCompletionCodesShadowByTheirSignedCode() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"return -options ${'$'}options","completion":"dynamic"}],"handlers":[
              {"kind_handler":"on","match":"4294967295","completion_code":-1,"fallthrough":false,"body":[{"kind":"action","label":"wrapped"}]},
              {"kind_handler":"on","match":"-1","completion_code":-1,"fallthrough":false,"body":[{"kind":"action","label":"duplicate"}]},
              {"kind_handler":"on","match":"2147483648","completion_code":-2147483648,"fallthrough":false,"body":[{"kind":"action","label":"minimum"}]}
            ]}]}],"procedures":[]}"""
        )))
        assertEquals(2, mermaid.lines().count { it.startsWith("n2 -->|on|") }, mermaid)
    }

    @Test
    fun exactCustomCompletionMatchesItsCanonicalOnCode() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"return -code 42","completion":"custom","completion_code":42}],"handlers":[
              {"kind_handler":"on","match":"42","completion_code":42,"fallthrough":false,"body":[{"kind":"action","label":"handled"}]},
              {"kind_handler":"on","match":"43","completion_code":43,"fallthrough":false,"body":[{"kind":"action","label":"wrong"}]}
            ],"finally":[{"kind":"action","label":"cleanup"}]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )))
        assertContains(mermaid, "handled")
        kotlin.test.assertFalse(mermaid.contains("wrong"), mermaid)
        assertContains(mermaid, "after")
    }

    @Test
    fun wrappedMinusOneCustomCompletionMatchesOnMinusOne() {
        val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"try","body":[
              {"kind":"action","label":"return -code 4294967295","completion":"custom","completion_code":-1}],"handlers":[
              {"kind_handler":"on","match":"-1","completion_code":-1,"fallthrough":false,"body":[{"kind":"action","label":"handled minus one"}]}
            ],"finally":[{"kind":"action","label":"cleanup"}]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )))
        assertContains(mermaid, "handled minus one")
        assertContains(mermaid, "after")
    }

    @Test
    fun exactStandardCompletionsUseCanonicalHandlerCodesNotSelectorSpellings() {
        for ((completion, selector, code) in listOf(
            Triple(null, "00", 0),
            Triple("error", "+1", 1),
            Triple("return", "02", 2),
            Triple("break", "0x3", 3),
            Triple("continue", "04", 4),
        )) {
            val completionField = completion?.let { ",\"completion\":\"$it\"" } ?: ""
            val mermaid = assertNotNull(renderDiagramMermaid(JsonParser.parseString(
                """{"events":[{"name":"E","flow":[{"kind":"try","body":[
                  {"kind":"action","label":"body"$completionField}],"handlers":[
                  {"kind_handler":"on","match":"$selector","completion_code":$code,"fallthrough":false,"body":[{"kind":"action","label":"handled"}]}
                ],"finally":[{"kind":"action","label":"cleanup"}]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
            )))
            assertContains(mermaid, "after", message = selector)
        }
    }

    @Test
    fun trapSourceOrderUsesProjectedTclListElements() {
        fun render(firstMatch: String, secondMatch: String, pattern: List<String>?): String {
            val projected = pattern?.joinToString(prefix = "[", postfix = "]") { JsonPrimitive(it).toString() } ?: "null"
            return assertNotNull(renderDiagramMermaid(JsonParser.parseString(
                """{"events":[{"name":"E","flow":[{"kind":"try","body":[
                  {"kind":"action","label":"return -code ${'$'}code","completion":"dynamic_return_or_error"}],"handlers":[
                  {"kind_handler":"trap","match":${JsonPrimitive(firstMatch)},"trap_pattern":$projected,"fallthrough":false,"body":[{"kind":"action","label":"first trap"}]},
                  {"kind_handler":"trap","match":${JsonPrimitive(secondMatch)},"trap_pattern":$projected,"fallthrough":false,"body":[{"kind":"action","label":"second trap"}]},
                  {"kind_handler":"on","match":"error","completion_code":1,"fallthrough":false,"body":[{"kind":"action","label":"error"}]}
                ]}]}],"procedures":[]}"""
            )))
        }

        for ((first, second, pattern) in listOf(
            Triple("A {B C}", "A \"B C\"", listOf("A", "B C")),
            Triple("A \\${'$'}B", "A {${'$'}B}", listOf("A", "${'$'}B")),
            Triple("A \\[B\\]", "A {[B]}", listOf("A", "[B]")),
            Triple("A C:\\\\tmp", "A {C:\\tmp}", listOf("A", "C:\\tmp")),
        )) {
            val mermaid = render(first, second, pattern)
            assertEquals(1, mermaid.lines().count { it.contains("-->|trap|") }, mermaid)
            assertContains(mermaid, "first trap")
            kotlin.test.assertFalse(mermaid.contains("second trap"), mermaid)
        }

        for ((first, second) in listOf(
            "A {" to "A }",
            "${'$'}pattern" to "A ${'$'}pattern",
        )) {
            val mermaid = render(first, second, null)
            assertEquals(2, mermaid.lines().count { it.contains("-->|trap|") }, mermaid)
            assertContains(mermaid, "first trap")
            assertContains(mermaid, "second trap")
        }
    }

    @Test
    fun exactErrorTrapPrefixesRespectSourceOrder() {
        fun render(first: List<String>, second: List<String>): String {
            fun pattern(values: List<String>) =
                values.joinToString(prefix = "[", postfix = "]") { JsonPrimitive(it).toString() }
            return assertNotNull(renderDiagramMermaid(JsonParser.parseString(
                """{"events":[{"name":"E","flow":[{"kind":"try","body":[
                  {"kind":"action","label":"bad return","completion":"error"}],"handlers":[
                  {"kind_handler":"trap","match":"first","trap_pattern":${pattern(first)},"fallthrough":false,"body":[{"kind":"action","label":"first trap"}]},
                  {"kind_handler":"trap","match":"second","trap_pattern":${pattern(second)},"fallthrough":false,"body":[{"kind":"action","label":"second trap"}]},
                  {"kind_handler":"on","match":"error","completion_code":1,"fallthrough":false,"body":[{"kind":"action","label":"on error"}]}
                ]}]}],"procedures":[]}"""
            )))
        }

        val shadowed = render(listOf("A"), listOf("A", "B"))
        assertEquals(1, shadowed.lines().count { it.contains("-->|trap|") }, shadowed)
        assertContains(shadowed, "first trap")
        kotlin.test.assertFalse(shadowed.contains("second trap"), shadowed)
        assertContains(shadowed, "on error")

        val nonOverlapping = render(listOf("A", "B"), listOf("A"))
        assertEquals(2, nonOverlapping.lines().count { it.contains("-->|trap|") }, nonOverlapping)
        assertContains(nonOverlapping, "first trap")
        assertContains(nonOverlapping, "second trap")
        assertContains(nonOverlapping, "on error")
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
              {"kind_handler":"on","match":"error","completion_code":1,"fallthrough":true,"body":[]},
              {"kind_handler":"on","match":"return","completion_code":2,"fallthrough":false,"body":[{"kind":"action","label":"shared body"}]}
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

    @Test
    fun loopsConsumeBreakAndContinueCompletions() {
        val payload = JsonParser.parseString(
            """{"events":[{"name":"E","flow":[{"kind":"loop","label":"while ready","exit":"false","body":[{"kind":"switch","label":"mode","arms":[{"pattern":"skip","body":[{"kind":"action","label":"continue","completion":"continue"}]},{"pattern":"stop","body":[{"kind":"action","label":"break","completion":"break"}]}]}]},{"kind":"action","label":"after"}]}],"procedures":[]}"""
        )
        val mermaid = assertNotNull(renderDiagramMermaid(payload))
        assertContains(mermaid, "-->|continue|")
        assertContains(mermaid, "-->|break|")
        assertContains(mermaid, "loop exit")
        assertContains(mermaid, "after")
    }
}
