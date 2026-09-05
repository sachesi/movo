package org.movo.app

import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.movo.app.core.Tab
import org.movo.app.home.OVERFLOW_TABS
import org.movo.app.home.PRIMARY_TABS
import org.movo.app.catalog.homeStartPosition
import org.movo.app.core.HomeSection
import org.movo.app.core.MediaItem
import org.movo.app.ui.FocusPivotSpec
import org.movo.app.ui.toTvColorScheme
import org.junit.Test

class NavigationLayoutTest {

    @Test
    fun everyTabButTheAccountIsReachableFromTheBottomBar() {
        // The account page opens from the user's name in the top bar.
        assertEquals(Tab.entries.toSet() - Tab.Account, (PRIMARY_TABS + OVERFLOW_TABS).toSet())
        assertEquals(Tab.entries.size - 1, PRIMARY_TABS.size + OVERFLOW_TABS.size)
    }

    @Test
    fun theBottomBarStaysWithinFiveItems() {
        // The primary tabs plus the one that opens the rest.
        assertTrue(PRIMARY_TABS.size + 1 <= 5)
    }

    @Test
    fun theTvSchemeIsTheAppScheme() {
        val dark = darkColorScheme()
        val tvDark = dark.toTvColorScheme(isDark = true)
        assertEquals(dark.primary, tvDark.primary)
        assertEquals(dark.onPrimary, tvDark.onPrimary)
        assertEquals(dark.surface, tvDark.surface)
        assertEquals(dark.onSurfaceVariant, tvDark.onSurfaceVariant)
        assertEquals(dark.errorContainer, tvDark.errorContainer)

        val light = lightColorScheme()
        assertEquals(light.primary, light.toTvColorScheme(isDark = false).primary)
    }

    @Test
    fun theFocusedCardIsHeldThreeTenthsIntoTheViewport() {
        // A card at the far edge scrolls back to the pivot; one already there does not move.
        assertEquals(700f, FocusPivotSpec.calculateScrollDistance(offset = 1000f, size = 176f, containerSize = 1000f))
        assertEquals(0f, FocusPivotSpec.calculateScrollDistance(offset = 300f, size = 176f, containerSize = 1000f))
        assertEquals(-300f, FocusPivotSpec.calculateScrollDistance(offset = 0f, size = 176f, containerSize = 1000f))
    }

    @Test
    fun theHomePageReopensOnTheRailAndCardTheUserLeftFrom() {
        val sections = listOf(
            HomeSection("hot", listOf(MediaItem(id = 1, title = "A", url = "/a"))),
            HomeSection("new", listOf(MediaItem(id = 2, title = "B", url = "/b"), MediaItem(id = 3, title = "C", url = "/c:d"))),
        )
        assertEquals(1 to 1, homeStartPosition(sections, "new:/c:d"))
        assertEquals(0 to 0, homeStartPosition(sections, "popular:/zzz"))
        assertEquals(0 to 0, homeStartPosition(sections, null))
    }
}
