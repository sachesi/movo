package org.movo.app

import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.movo.app.core.Episode
import org.movo.app.core.Season
import org.movo.app.player.EpisodeSelector
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/**
 * The episode chooser lists hundreds of rows lazily. The dropdown it used to live in measured
 * its content by intrinsic width, which a lazy list refuses, so opening it threw.
 */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34])
class EpisodeMenuTest {
    @get:Rule val compose = createComposeRule()

    @Test
    fun theEpisodeMenuOpensOverALongSeries() {
        val seasons = (1..8).map { season ->
            Season(season.toLong(), "Season $season", (1..24).map { Episode(it.toLong(), season.toLong(), "Episode $it") })
        }
        var played: Pair<Long, Long>? = null
        compose.setContent {
            MaterialTheme {
                EpisodeSelector(seasons, 3, 5, { s, e -> played = s to e }, isTv = false, onMenuOpenChange = {})
            }
        }
        compose.onNodeWithContentDescription("Episode").performClick()
        compose.onNodeWithText("Season 3 • Episode 5").assertIsDisplayed()
        compose.onNodeWithText("Season 3 • Episode 6").performClick()
        compose.runOnIdle { assertEquals(3L to 6L, played) }
    }
}
