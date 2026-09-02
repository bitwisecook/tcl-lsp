package com.tcllsp.jetbrains.packs

import com.google.gson.JsonParser
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class PackAssociationReconcilerTest {

    private val tcl = PackAssociationReconciler.TCL_FILE_TYPE
    private val irule = PackAssociationReconciler.IRULE_FILE_TYPE

    @Test
    fun advertisedRowsResolveOntoTheTwoContributedFileTypes() {
        val payload = JsonParser.parseString(
            """
            {"pack_file_extensions": [
              {"extension": "IRULEX", "dialect": "f5-irules", "language_id": "tcl-irule", "pack": "extlib"},
              {"extension": "packplain", "dialect": null, "language_id": "tcl", "pack": "extlib"},
              {"extension": "pshx", "dialect": "probe-shell-tcl", "language_id": "tcl", "pack": "probe"}
            ]}
            """.trimIndent(),
        )

        assertEquals(
            mapOf("irulex" to irule, "packplain" to tcl, "pshx" to tcl),
            PackAssociationReconciler.claimsFrom(payload),
        )
    }

    @Test
    fun aBareArrayAndAnAbsentPayloadAreBothAccepted() {
        val rows = JsonParser.parseString("""[{"extension": "foo", "language_id": "tcl"}]""")
        assertEquals(mapOf("foo" to tcl), PackAssociationReconciler.claimsFrom(rows))
        assertEquals(emptyMap(), PackAssociationReconciler.claimsFrom(null))
        assertEquals(emptyMap(), PackAssociationReconciler.claimsFrom(JsonParser.parseString("{}")))
    }

    @Test
    fun malformedRowsAreDroppedRatherThanRegistered() {
        val payload = JsonParser.parseString(
            """
            [{"extension": ""}, {"extension": ".foo"}, {"extension": "two words"},
             {"language_id": "tcl"}, "not an object", {"extension": "good"}]
            """.trimIndent(),
        )

        assertEquals(mapOf("good" to tcl), PackAssociationReconciler.claimsFrom(payload))
    }

    @Test
    fun anUnclaimedExtensionIsAssociatedAndRecorded() {
        val plan = PackAssociationReconciler.plan(
            claimed = mapOf("irulex" to irule),
            owned = emptyMap(),
            associatedWith = { null },
        )

        assertEquals(listOf(PackExtensionClaim("irulex", irule)), plan.associate)
        assertEquals(emptyList(), plan.disassociate)
        assertEquals(mapOf("irulex" to irule), plan.owned)
    }

    @Test
    fun anAssociationWeInstalledIsRetiredOnceNoPackClaimsIt() {
        val plan = PackAssociationReconciler.plan(
            claimed = emptyMap(),
            owned = mapOf("irulex" to irule),
            associatedWith = { irule },
        )

        assertEquals(listOf(PackExtensionClaim("irulex", irule)), plan.disassociate)
        assertEquals(emptyMap(), plan.owned)
    }

    @Test
    fun anAssociationTheUserHasSinceChangedIsForgottenNotRemoved() {
        // Recorded as ours, but the IDE now says something else: the user
        // retargeted it. Retiring it here would delete their choice.
        val plan = PackAssociationReconciler.plan(
            claimed = emptyMap(),
            owned = mapOf("irulex" to irule),
            associatedWith = { "JSON" },
        )

        assertEquals(emptyList(), plan.disassociate)
        assertEquals(emptyList(), plan.associate)
        assertEquals(emptyMap(), plan.owned)
    }

    @Test
    fun anExtensionSomebodyElseOwnsIsNeitherClaimedNorRecorded() {
        val plan = PackAssociationReconciler.plan(
            claimed = mapOf("irulex" to irule),
            owned = emptyMap(),
            associatedWith = { "JSON" },
        )

        assertEquals(emptyList(), plan.associate)
        assertEquals(emptyList(), plan.disassociate)
        assertEquals(emptyMap(), plan.owned)
        assertEquals(listOf(PackExtensionClaim("irulex", "JSON")), plan.deferred)
        assertTrue(plan.isEmpty)
    }

    @Test
    fun ourOwnFileTypeAssociatedByHandIsRespectedAndNeverRetired() {
        // The user pointed `.irulex` at the plugin's own Tcl type themselves.
        // We did not install it, so we neither re-claim it nor remove it when
        // the pack that claimed it goes away.
        val claimed = PackAssociationReconciler.plan(
            claimed = mapOf("irulex" to irule),
            owned = emptyMap(),
            associatedWith = { tcl },
        )
        assertEquals(emptyMap(), claimed.owned)
        assertTrue(claimed.isEmpty)

        val retired = PackAssociationReconciler.plan(
            claimed = emptyMap(),
            owned = claimed.owned,
            associatedWith = { tcl },
        )
        assertTrue(retired.isEmpty)
    }

    @Test
    fun aClaimThatMovesBetweenOurFileTypesIsRetargeted() {
        val plan = PackAssociationReconciler.plan(
            claimed = mapOf("irulex" to irule),
            owned = mapOf("irulex" to tcl),
            associatedWith = { tcl },
        )

        assertEquals(listOf(PackExtensionClaim("irulex", tcl)), plan.disassociate)
        assertEquals(listOf(PackExtensionClaim("irulex", irule)), plan.associate)
        assertEquals(mapOf("irulex" to irule), plan.owned)
    }

    @Test
    fun aSteadyStateReconciliationTouchesNothing() {
        val plan = PackAssociationReconciler.plan(
            claimed = mapOf("irulex" to irule),
            owned = mapOf("irulex" to irule),
            associatedWith = { irule },
        )

        assertTrue(plan.isEmpty)
        assertEquals(mapOf("irulex" to irule), plan.owned)
    }

    @Test
    fun projectsUnionTheirClaimsAndDisagreementFallsBackToPlainTcl() {
        val first = mapOf("aaa" to tcl, "shared" to irule)
        val second = mapOf("bbb" to irule, "shared" to tcl)

        val forward = PackAssociationReconciler.union(listOf(first, second))
        val backward = PackAssociationReconciler.union(listOf(second, first))

        assertEquals(mapOf("aaa" to tcl, "bbb" to irule, "shared" to tcl), forward)
        assertEquals(forward, backward)
    }

    @Test
    fun anExtensionClaimedByOneOfTwoProjectsSurvivesTheOtherReporting() {
        // The multi-project rule: registration is the union, so a project that
        // does not claim `irulex` must not retire it while another still does.
        val claimed = PackAssociationReconciler.union(
            listOf(mapOf("irulex" to irule), emptyMap()),
        )
        val plan = PackAssociationReconciler.plan(
            claimed = claimed,
            owned = mapOf("irulex" to irule),
            associatedWith = { irule },
        )

        assertTrue(plan.isEmpty)
        assertEquals(mapOf("irulex" to irule), plan.owned)
    }
}
