// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains.settings

import com.intellij.ui.components.JBCheckBox
import java.awt.Rectangle
import javax.swing.JComponent
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

/**
 * The settings page's checkbox groups used `GridLayout(0, n)`, whose width is
 * the widest label times `n` regardless of the space available — about 2000px
 * for the optimiser's four columns. An IntelliJ settings pane never scrolls
 * horizontally, so those columns ran off the right-hand edge unreachably.
 */
class ReflowingGridTest {

    private fun grid(count: Int = 12): ReflowingGrid =
        ReflowingGrid((1..count).map { JBCheckBox("IRULE400$it: a label of realistic length") })

    /** Width of one cell, which is what the grid may never go below. */
    private fun cellWidth(grid: ReflowingGrid): Int =
        grid.components.maxOf { it.preferredSize.width }

    @Test
    fun theGridNeverAsksForMoreThanOneColumnOfWidth() {
        val grid = grid()
        val cell = cellWidth(grid)

        assertTrue(
            grid.minimumSize.width <= cell + grid.insets.left + grid.insets.right,
            "minimum width ${grid.minimumSize.width} exceeds one ${cell}px cell — " +
                "the page cannot be squeezed into a narrow settings pane",
        )
    }

    @Test
    fun aWiderPaneGetsMoreColumnsAndFewerRows() {
        val grid = grid()
        val cell = cellWidth(grid)

        val narrow = layoutAt(grid, cell + 4)
        val wide = layoutAt(grid, cell * 3 + 40)

        assertEquals(1, columnsIn(narrow), "one cell of width must yield one column")
        assertEquals(3, columnsIn(wide), "three cells of width must yield three columns")
        assertTrue(rowsIn(wide) < rowsIn(narrow))
    }

    @Test
    fun everyCheckboxStaysInsideTheWidthTheGridWasGiven() {
        val grid = grid(31)
        val cell = cellWidth(grid)

        for (width in listOf(cell, cell * 2 + 8, cell * 4 + 40, cell * 7)) {
            val bounds = layoutAt(grid, width)
            val overflow = bounds.filter { it.x + it.width > width }
            assertTrue(
                overflow.isEmpty(),
                "${overflow.size} of ${bounds.size} cells overflow a ${width}px grid",
            )
            assertTrue(grid.preferredSize.width <= width || columnsIn(bounds) == 1)
        }
    }

    /** Lay the grid out at [width] and report where each child landed. */
    private fun layoutAt(grid: ReflowingGrid, width: Int): List<Rectangle> {
        grid.setSize(width, Int.MAX_VALUE / 2)
        grid.doLayout()
        return grid.components.map { (it as JComponent).bounds }
    }

    private fun columnsIn(bounds: List<Rectangle>): Int = bounds.map { it.x }.distinct().size

    private fun rowsIn(bounds: List<Rectangle>): Int = bounds.map { it.y }.distinct().size
}
