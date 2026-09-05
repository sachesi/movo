package org.movo.app

import org.movo.app.core.CoreTransport
import org.movo.app.core.MovoViewModel
import org.movo.app.core.NativeBridge
import org.movo.app.core.Tab
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/** Answers bridge requests from a canned table and records what was asked. */
private class FakeCore(private val replies: Map<String, String>) : CoreTransport {
    val requested = mutableListOf<String>()

    override fun send(request: String): String {
        val type = NativeBridge.json.parseToJsonElement(request)
            .let { it as kotlinx.serialization.json.JsonObject }
            .getValue("type")
            .let { (it as kotlinx.serialization.json.JsonPrimitive).content }
        requested += type
        return replies[type] ?: """{"error":"no reply for $type"}"""
    }
}

private const val CATALOG_REPLY =
    """{"data":[{"id":1,"title":"Dune","url":"/films/dune"}]}"""
private const val SUGGESTIONS_REPLY = """{"data":["dune","dune two"]}"""
private const val DETAILS_REPLY =
    """{"data":{"id":1,"title":"Dune","url":"/films/dune","description":"","media_type":"Film","genres":[],"countries":[],"directors":[],"actors":[],"translators":[],"seasons":[],"franchises":[]}}"""

/** Comfortably shorter than the debounce, so four keystrokes still make one request. */
private const val SUGGEST_KEYSTROKE_GAP_MS = 50L

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class MovoViewModelTest {
    private val core = FakeCore(
        mapOf("catalog" to CATALOG_REPLY, "search_suggestions" to SUGGESTIONS_REPLY, "details" to DETAILS_REPLY),
    )

    private fun model(scheduler: kotlinx.coroutines.test.TestCoroutineScheduler): MovoViewModel {
        val dispatcher = StandardTestDispatcher(scheduler)
        Dispatchers.setMain(dispatcher)
        NativeBridge.transport = core
        NativeBridge.dispatcher = dispatcher
        return MovoViewModel(ApplicationProvider.getApplicationContext())
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
        NativeBridge.transport = null
        NativeBridge.dispatcher = Dispatchers.IO
    }

    @Test
    fun typingASuggestionDoesNotCancelTheLoadBesideIt() = runTest {
        val model = model(testScheduler)
        model.loadCatalog()
        model.suggest("du")
        advanceUntilIdle()

        assertEquals(listOf("Dune"), model.state.value.items.map { it.title })
    }

    @Test
    fun typingSendsOneSuggestionRequestPerPause() = runTest {
        val model = model(testScheduler)
        model.selectTab(Tab.Search, isTv = false)
        // Time has to pass between the keystrokes: without it every launch is cancelled before it
        // is ever dispatched, and the test would pass whether or not the typing is debounced.
        "dune".forEach {
            model.suggest("query$it")
            advanceTimeBy(SUGGEST_KEYSTROKE_GAP_MS)
        }
        advanceUntilIdle()

        assertEquals(1, core.requested.count { it == "search_suggestions" })
    }

    @Test
    fun aSecondPressOnTheSameCardDoesNotRestartTheLoad() = runTest {
        val model = model(testScheduler)
        model.openDetails("/films/dune")
        assertEquals("/films/dune", model.state.value.openingUrl)
        model.openDetails("/films/dune")
        advanceUntilIdle()

        assertEquals(1, core.requested.count { it == "details" })
        assertEquals(null, model.state.value.openingUrl)
    }

    @Test
    fun returningToATabShowsItsListingWithoutFetchingAgain() = runTest {
        val model = model(testScheduler)
        model.selectTab(Tab.Catalog, isTv = false)
        advanceUntilIdle()
        assertEquals(listOf("Dune"), model.state.value.items.map { it.title })

        model.selectTab(Tab.History, isTv = false)
        assertEquals(emptyList<String>(), model.state.value.items.map { it.title })
        model.selectTab(Tab.Catalog, isTv = false)
        assertEquals(listOf("Dune"), model.state.value.items.map { it.title })
        advanceUntilIdle()

        assertEquals(1, core.requested.count { it == "catalog" })
    }

    @Test
    fun closingAListingOpenedFromATitleReopensTheTitle() = runTest {
        val model = model(testScheduler)
        model.openDetails("/films/dune")
        advanceUntilIdle()
        model.openDiscoveryPath(Tab.Search, "Drama", "/drama")
        advanceUntilIdle()
        assertEquals(null, model.state.value.details)

        model.closeCollection()
        advanceUntilIdle()

        assertEquals("/films/dune", model.state.value.details?.url)
        assertEquals(null, model.state.value.collectionPath)
    }
}
