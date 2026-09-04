package org.movo.app

import org.movo.app.core.FavoriteGroup
import org.movo.app.core.HistoryEntry
import org.movo.app.core.HomeSection
import org.movo.app.core.MediaItem
import org.movo.app.core.SearchFilter
import org.movo.app.account.TvFavoriteFilters
import org.movo.app.account.TvHistoryCard
import org.movo.app.catalog.TvHomeScreen
import org.movo.app.core.AppState
import org.movo.app.core.Tab
import org.movo.app.home.ConfirmLogoutDialog
import org.movo.app.home.TvNavigationDrawer
import org.movo.app.search.TvSearchContent
import org.movo.app.settings.AppSettings
import androidx.datastore.preferences.core.Preferences
import org.movo.app.settings.SettingsActions
import org.movo.app.settings.SettingsContent
import org.movo.app.settings.settings
import org.movo.app.ui.TvHomeSkeleton
import androidx.compose.foundation.focusGroup
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.focusRestorer
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.assert
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.hasScrollAction
import androidx.compose.ui.test.isFocused
import androidx.compose.ui.test.junit4.ComposeContentTestRule
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performKeyInput
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performScrollToIndex
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.test.pressKey
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.tv.material3.Button as TvButton
import androidx.tv.material3.MaterialTheme as TvMaterialTheme
import androidx.tv.material3.Text as TvText
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class TvNavigationTest {
    @get:Rule
    val compose = createComposeRule()

    @Test
    fun drawerExpandsOnFocusAndActivatesOnlyOnCenter() {
        var selected by mutableStateOf(Tab.Catalog)
        val searchLabel = InstrumentationRegistry.getInstrumentation().targetContext
            .getString(R.string.nav_search)
        compose.setTvContent {
            TvNavigationDrawer(
                selectedTab = selected,
                settingsSelected = false,
                notificationCount = 3,
                selectTab = { selected = it },
                openSettings = {},
                labelled = false,
            ) {
                TvText("selected:${selected.name}")
            }
        }

        compose.onNodeWithText(searchLabel).assertDoesNotExist()
        compose.onNodeWithTag("tv-drawer-catalog")
            .performSemanticsAction(SemanticsActions.RequestFocus)
        compose.waitForIdle()
        compose.onNodeWithText(searchLabel).assertExists()
        compose.onNodeWithTag("tv-drawer-catalog").performKeyInput { pressKey(Key.DirectionDown) }
        compose.onNodeWithTag("tv-drawer-search").assertIsFocused()
        compose.onNodeWithText("selected:Catalog").assertExists()
        compose.onNodeWithTag("tv-drawer-search").performKeyInput { pressKey(Key.DirectionCenter) }
        compose.onNodeWithText("selected:Search").assertExists()
        compose.onAllNodes(isFocused()).assertCountEquals(1)
    }

    @Test
    fun rightReturnsFromDrawerToRememberedContent() {
        compose.setTvContent {
            TvNavigationDrawer(Tab.Catalog, false, 0, {}, {}, labelled = false) {
                LazyRow(
                    Modifier.padding(48.dp).focusRestorer().focusGroup(),
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    item {
                        TvButton(onClick = {}, modifier = Modifier.testTag("content-first")) {
                            TvText("First")
                        }
                    }
                    item {
                        TvButton(onClick = {}, modifier = Modifier.testTag("content-second")) {
                            TvText("Second")
                        }
                    }
                }
            }
        }

        compose.onNodeWithTag("content-second").performSemanticsAction(SemanticsActions.RequestFocus)
        compose.onNodeWithTag("content-second").performKeyInput { pressKey(Key.DirectionLeft) }
        compose.onNodeWithTag("content-first").performKeyInput { pressKey(Key.DirectionLeft) }
        compose.onNodeWithTag("tv-drawer-catalog").assertIsFocused()
        compose.onNodeWithTag("tv-drawer-catalog").performKeyInput { pressKey(Key.DirectionRight) }
        compose.onNodeWithTag("content-first").assertIsFocused()
    }

    @Test
    fun searchActionsAreSeparateFocusTargets() {
        var searches = 0
        var filters = 0
        compose.setTvContent {
            TvSearchContent(
                state = AppState(
                    suggestions = listOf("Dune"),
                    searchHistory = listOf("Alien"),
                    searchFilters = listOf(SearchFilter("Films", "/films", emptyList(), emptyList())),
                ),
                query = "test",
                updateQuery = {},
                voiceAvailable = true,
                launchVoice = {},
                search = { searches++ },
                chooseHistory = {},
                clearHistory = {},
                openFilters = { filters++ },
                open = {},
                loadMore = {},
            )
        }

        compose.onNodeWithTag("tv-search-field").assertExists()
        compose.onNodeWithTag("tv-search-voice").assertExists()
        compose.onNodeWithTag("tv-search-submit").performClick()
        compose.onNodeWithTag("tv-search-filter-row").performClick()
        compose.runOnIdle {
            assertEquals(1, searches)
            assertEquals(1, filters)
        }
    }

    @Test
    fun settingsScrollAndDestructiveDialogsStartOnCancel() {
        compose.setTvContent { TestSettings() }
        compose.onNodeWithTag("tv-settings-list").assert(hasScrollAction())
        val pauseLabel = InstrumentationRegistry.getInstrumentation().targetContext
            .getString(R.string.tv_pause_shows_controls)
        compose.onNodeWithText(pauseLabel).performScrollTo().assertIsDisplayed()

        var dismissed = false
        compose.setTvContent {
            ConfirmLogoutDialog(true, { dismissed = true }, {})
        }
        compose.onNodeWithTag("tv-dialog-cancel").assertIsFocused().performClick()
        compose.runOnIdle { assertTrue(dismissed) }
    }

    @Test
    fun historyActionsAreSiblingTargetsAndRemovalStartsOnCancel() {
        var opened = 0
        var toggled = 0
        var removed = 0
        val entry = HistoryEntry("7", "Film", "/7", watched = false)
        compose.setTvContent {
            TvHistoryCard(
                entry,
                open = { opened++ },
                toggle = { toggled++ },
                remove = { removed++ },
            )
        }

        compose.onNodeWithTag("tv-history-primary").performClick()
        compose.onNodeWithTag("tv-history-toggle").performClick()
        compose.onNodeWithTag("tv-history-delete").performClick()
        compose.onNodeWithTag("tv-history-cancel").assertIsFocused().performClick()
        compose.runOnIdle {
            assertEquals(1, opened)
            assertEquals(1, toggled)
            assertEquals(0, removed)
        }
    }

    @Test
    fun favoriteFiltersEnterGridDeterministically() {
        var selected by mutableStateOf<Long?>(1)
        compose.setTvContent {
            val grid = remember { FocusRequester() }
            Column {
                TvFavoriteFilters(
                    groups = listOf(
                        FavoriteGroup(1, "First", "/first", 2),
                        FavoriteGroup(2, "Second", "/second", 3),
                    ),
                    selectedGroup = selected,
                    gridFocusRequester = grid,
                    select = { selected = it },
                )
                TvButton(
                    onClick = {},
                    modifier = Modifier.focusRequester(grid).testTag("tv-favorite-grid-entry"),
                ) { TvText("Grid") }
            }
        }

        compose.onNodeWithTag("tv-favorite-group-1")
            .performSemanticsAction(SemanticsActions.RequestFocus)
        compose.onNodeWithTag("tv-favorite-group-1").performKeyInput { pressKey(Key.DirectionDown) }
        compose.onNodeWithTag("tv-favorite-grid-entry").assertIsFocused()
        compose.onNodeWithTag("tv-favorite-group-2").performClick()
        compose.runOnIdle { assertEquals(2L, selected) }
    }

    @Test
    fun homePlaceholderHasFiveStableRails() {
        var loading by mutableStateOf(true)
        val sections = listOf("hot", "new", "watching", "popular", "awaiting").map { id ->
            HomeSection(id, listOf(MediaItem(1, "Film", url = "/$id")))
        }
        compose.setTvContent {
            if (loading) TvHomeSkeleton() else TvHomeScreen(sections, false) { _, _ -> }
        }
        val placeholders = compose.onNodeWithTag("tv-home-placeholder-list")
        repeat(5) { rail ->
            placeholders.performScrollToIndex(rail)
            compose.onNodeWithTag("tv-home-placeholder-rail-$rail").assertExists()
            compose.onNodeWithTag("tv-home-placeholder-card-$rail-0").assertExists()
        }
        compose.runOnIdle { loading = false }
        val home = compose.onNodeWithTag("tv-home-list")
        repeat(5) { rail ->
            home.performScrollToIndex(rail)
            compose.onNodeWithTag("tv-home-rail-${sections[rail].id}").assertExists()
        }
    }

    private fun ComposeContentTestRule.setTvContent(content: @androidx.compose.runtime.Composable () -> Unit) {
        setContent {
            MaterialTheme {
                TvMaterialTheme {
                    androidx.compose.foundation.layout.Box(Modifier.fillMaxSize()) { content() }
                }
            }
        }
    }
}

@androidx.compose.runtime.Composable
private fun TestSettings() {
    SettingsContent(
        settings = AppSettings(),
        isTv = true,
        actions = object : SettingsActions {
            override fun <T> save(key: Preferences.Key<T>, value: T) = Unit
        },
    )
}
