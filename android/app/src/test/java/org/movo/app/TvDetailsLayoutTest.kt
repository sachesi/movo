package org.movo.app

import androidx.activity.ComponentActivity
import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.isFocused
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performKeyInput
import androidx.compose.ui.test.pressKey
import androidx.test.core.app.ApplicationProvider
import androidx.tv.material3.MaterialTheme as TvMaterialTheme
import org.junit.After
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.movo.app.core.AppState
import org.movo.app.core.CoreTransport
import org.movo.app.core.Episode
import org.movo.app.core.MediaDetails
import org.movo.app.core.MediaItem
import org.movo.app.core.MovoViewModel
import org.movo.app.core.NativeBridge
import org.movo.app.core.Person
import org.movo.app.core.Season
import org.movo.app.core.Translator
import org.movo.app.core.UserProfile
import org.movo.app.details.DetailsScreen
import org.movo.app.settings.AppSettings
import org.movo.app.ui.TV_OVERSCAN_VERTICAL
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

private object OfflineDetailsCore : CoreTransport {
    override fun send(request: String) = """{"error":"offline"}"""
}

/**
 * The details page on TV used to pin a Material top app bar over a LazyColumn with no
 * bring-into-view rule of its own: one D-pad Down from Play scrolled the poster and headline
 * clean under the bar, and Up back never brought them into view again. This exercises the fix
 * end to end: no app bar on TV, the header pulls the list back to the top when it takes focus,
 * and the focus pivot keeps the highlight clear of the overscan band while scrolling.
 */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], qualifiers = "w960dp-h540dp-land-television-xhdpi")
class TvDetailsLayoutTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    @After fun tearDown() { NativeBridge.transport = null }

    private fun model(): MovoViewModel {
        NativeBridge.transport = OfflineDetailsCore
        return MovoViewModel(ApplicationProvider.getApplicationContext())
    }

    // A series, so the translator list and episodes are exercised the way a real title would be.
    private val details = MediaDetails(
        id = 1,
        title = "A Long Running Show With A Title Wide Enough To Wrap",
        url = "/series/long-show",
        description = "Long enough to overflow the collapsed description. ".repeat(20),
        mediaType = "TVSeries",
        genres = listOf("drama"),
        countries = listOf("US"),
        directors = emptyList(),
        actors = emptyList(),
        translators = listOf(Translator(id = 1, name = "Original", premium = false)),
        seasons = listOf(
            Season(id = 1, title = "Season 1", episodes = listOf(Episode(id = 1, seasonId = 1, title = "Episode 1"))),
        ),
        franchises = emptyList(),
        actorDetails = (1..8).map { Person(name = "Actor $it", url = "/actors/actor-$it") },
        related = (1..8).map { MediaItem(id = it.toLong(), title = "Related $it", url = "/films/related-$it") },
    )

    private val state = AppState(
        user = UserProfile(userId = "1", username = "u", loggedIn = true, vip = false),
        details = details,
    )

    private fun render(m: MovoViewModel) {
        compose.setContent {
            MaterialTheme {
                TvMaterialTheme {
                    DetailsScreen(state, isTv = true, wideContent = true, settings = AppSettings(), model = m)
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun theAppBarIsGoneAndTheHeadlineStartsInsideTheOverscan() {
        render(model())
        // Exactly the inline back button in the header row: a second one would mean the pinned
        // app bar is still there. onNodeWithText below fails the same way if it is, since the
        // app bar used to carry a second copy of the title.
        compose.onAllNodesWithContentDescription("Back").assertCountEquals(1)
        val headline = compose.onNodeWithText(details.title).fetchSemanticsNode()
        val overscanTopPx = with(compose.density) { TV_OVERSCAN_VERTICAL.toPx() }
        assertTrue(headline.boundsInRoot.top >= overscanTopPx - 1f)
    }

    @Test
    fun downThenUpLeavesTheHeadlineVisibleAgain() {
        render(model())
        compose.onRoot().performKeyInput { pressKey(Key.DirectionDown) }
        compose.waitForIdle()
        compose.onRoot().performKeyInput { pressKey(Key.DirectionUp) }
        compose.waitForIdle()
        val headline = compose.onNodeWithText(details.title).fetchSemanticsNode()
        // Back at the top of the list, not merely inside the screen: the header pulls the list
        // back to item 0 when it takes focus, so the poster is fully visible too.
        val overscanTopPx = with(compose.density) { TV_OVERSCAN_VERTICAL.toPx() }
        assertTrue(headline.boundsInRoot.top >= overscanTopPx - 1f)
    }

    @Test
    fun theFocusedRowStaysClearOfTheBottomEdgeAfterScrollingDown() {
        render(model())
        val screenBottom = compose.onRoot().fetchSemanticsNode().boundsInRoot.bottom
        compose.onRoot().performKeyInput { pressKey(Key.DirectionDown) }
        compose.waitForIdle()
        val focused = compose.onAllNodes(isFocused()).fetchSemanticsNodes().first()
        assertTrue(focused.boundsInRoot.bottom < screenBottom - 1f)
    }
}
