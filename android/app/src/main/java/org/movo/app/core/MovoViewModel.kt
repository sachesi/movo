package org.movo.app.core

import org.movo.app.R
import org.movo.app.settings.QualityMode
import org.movo.app.settings.settings
import org.movo.app.player.selectStream
import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.debounce
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.encodeToJsonElement
import kotlinx.serialization.json.put

data class AppState(
    val restoring: Boolean = true,
    val user: UserProfile? = null,
    val loading: Boolean = false,
    val error: String? = null,
    val tab: Tab = Tab.Catalog,
    val items: List<MediaItem> = emptyList(),
    val homeSections: List<HomeSection> = emptyList(),
    val category: CatalogCategory = CatalogCategory.All,
    val sort: String = "popular",
    val page: Int = 1,
    val query: String = "",
    val suggestions: List<String> = emptyList(),
    val searchHistory: List<String> = emptyList(),
    val searchFilters: List<SearchFilter> = emptyList(),
    val collections: List<CollectionItem> = emptyList(),
    val collectionPath: String? = null,
    val collectionTitle: String? = null,
    val favoriteGroups: List<FavoriteGroup> = emptyList(),
    val favoriteGroup: Long? = null,
    val history: List<HistoryEntry> = emptyList(),
    val notifications: List<NotificationGroup> = emptyList(),
    val notificationCount: Int = 0,
    val premiumDays: Int? = null,
    val details: MediaDetails? = null,
    val actor: ActorDetails? = null,
    val comments: CommentsPage? = null,
    val trailerUrl: String? = null,
    val resumeSeasonId: Long? = null,
    val resumeEpisodeId: Long? = null,
    val resumeTranslatorId: Long? = null,
    val episodesTranslatorId: Long? = null,
    val preparedStream: StreamBundle? = null,
    val stream: StreamBundle? = null,
    val playbackQuality: String? = null,
    val playbackPositionMs: Long = 0,
    val focusedUrl: String? = null,
    val detailAction: DetailAction? = null,
    /** The title a press is fetching, so the page can show the press landed. */
    val openingUrl: String? = null,
    /** Countries the settings offer to hide, from the core. */
    val countries: List<Country> = emptyList(),
)

/**
 * Something the app should say once and forget. Anything still true after the screen is rebuilt
 * belongs on [AppState] instead: this is only for what the UI shows and moves past.
 */
sealed interface AppEffect {
    data class Message(val text: String) : AppEffect
}

/** How long typing has to settle before suggestions are fetched. */
private const val SUGGEST_DEBOUNCE_MS = 300L

enum class Tab { Catalog, Search, Collections, Favorites, History, Notifications, Account }
enum class Screen { Restoring, Login, Player, Trailer, Details, Home }
enum class DetailAction { Favorites, Comments, Rating }

@OptIn(FlowPreview::class)
class MovoViewModel(application: Application) : AndroidViewModel(application) {
    private val store = SessionStore(application)
    private val _state = MutableStateFlow(AppState())
    val state = _state.asStateFlow()
    val screen = state.map {
        when {
            it.restoring -> Screen.Restoring
            it.user == null -> Screen.Login
            it.stream != null -> Screen.Player
            it.trailerUrl != null -> Screen.Trailer
            it.details != null -> Screen.Details
            else -> Screen.Home
        }
    }.distinctUntilChanged()
    // Buffered rather than a SharedFlow: a message raised while the app is backgrounded waits
    // there and is shown on resume instead of being dropped.
    private val _effects = Channel<AppEffect>(Channel.BUFFERED)
    val effects = _effects.receiveAsFlow()
    private var activeOperations = 0
    private var contentJob: Job? = null
    private var suggestJob: Job? = null
    private var detailsJob: Job? = null
    private var episodesJob: Job? = null
    private var playbackJob: Job? = null
    private var progressJob: Job? = null
    private var syncJob: Job? = null
    private var completionJob: Job? = null
    private var markedWatchedKey: String? = null
    private val detailsBackStack = ArrayDeque<String>()
    private var pendingDetailsUrl: String? = null
    private var detailsBackLoading = false

    /** The title a genre or collection listing was opened from, reopened when the listing closes. */
    private var discoveryReturnUrl: String? = null
    private var pathReturnFocus: String? = null

    /**
     * Each tab's listing as the user left it. Shown again on return rather than fetched again, so
     * the pages already loaded and the place in them survive a visit to another tab; pull-to-refresh,
     * Retry and an action that changed the listing fetch it afresh.
     */
    private val listings = HashMap<Tab, Listing>()
    private var loginJob: Job? = null

    /** What the core currently filters by, so only a real change reloads the listings. */
    private var appliedHiddenCountries: String? = null

    init {
        restore()
        viewModelScope.launch {
            getApplication<Application>().settings.map { it.hiddenCountries }.distinctUntilChanged().debounce(HIDDEN_COUNTRIES_SETTLE_MS).collect { hidden ->
                if (hidden == appliedHiddenCountries) return@collect
                applyHiddenCountries(hidden)
                // Every kept listing was filtered by the old names.
                listings.clear()
                reloadListings()
            }
        }
    }

    private suspend fun applyHiddenCountries(hidden: String) {
        // Nothing to hide and nothing hidden yet: the core starts out that way.
        if (hidden.isNotBlank() || appliedHiddenCountries != null) {
            // Best-effort: a filter that did not reach the core costs a few extra rows, not the
            // sign-in or the listing behind it.
            runCatching { NativeBridge.call("set_hidden_countries", buildJsonObject { put("countries", hidden) }) }
        }
        appliedHiddenCountries = hidden
    }

    /** Fetch again whatever listing is on screen, now that the filter differs. */
    private fun reloadListings() {
        val current = state.value
        when {
            current.details != null || current.stream != null -> return
            current.tab == Tab.Catalog && current.homeSections.isNotEmpty() -> {
                _state.update { it.copy(homeSections = emptyList()) }
                loadHome()
            }
            current.tab == Tab.Catalog && current.items.isNotEmpty() -> loadCatalog()
            current.tab == Tab.Search && current.query.isNotBlank() -> search(current.query)
        }
    }

    private fun run(block: suspend () -> Unit) = viewModelScope.launch {
        activeOperations++
        _state.update { it.copy(loading = true, error = null) }
        try {
            block()
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (error: Exception) {
            _state.update { it.copy(error = describe(error)) }
        } finally {
            activeOperations--
            _state.update { it.copy(
                loading = activeOperations > 0,
                restoring = false,
            ) }
        }
    }

    /** What the banner says: the kind of failure in the user's words, or the message when it has no kind. */
    private fun describe(error: Exception): String {
        val code = (error as? BridgeException)?.code
        val id = when (code) {
            "timeout" -> R.string.error_timeout
            "network" -> R.string.error_network
            "provider" -> R.string.error_provider
            "session" -> R.string.error_session
            "malformed" -> R.string.error_malformed
            else -> return error.message ?: text(R.string.operation_failed)
        }
        return text(id)
    }

    private fun restore() = run {
        warmUpNativeBridge()
        // Before the first listing, so nothing hidden is shown on the way in.
        applyHiddenCountries(getApplication<Application>().settings.first().hiddenCountries)
        try {
            val secret = store.secret() ?: return@run
            // The provider may be slow or away; a bounded wait, then the app opens on the account
            // as last seen with a banner, rather than a splash for as long as the network takes.
            val user = withTimeoutOrNull(RESTORE_TIMEOUT_MS) {
                NativeBridge.decode<UserProfile>("restore", buildJsonObject { put("secret", secret) })
            }
            if (user == null) {
                val known = store.profile() ?: error(text(R.string.error_timeout))
                _state.update { it.copy(user = known, error = text(R.string.error_timeout)) }
                return@run
            }
            store.saveProfile(user)
            _state.update { it.copy(user = user) }
            // Notifications and premium days are decoration: they load beside the restored
            // session, not ahead of it, so the splash does not wait on them.
            viewModelScope.launch { attempt { refreshAccountData(false) } }
        } catch (error: BridgeException) {
            // Only forget the session the provider actually turned down. Clearing it on any
            // failure signed the account out whenever restore happened to hit a dead network.
            if (error.sessionRejected) store.saveSecret(null)
            throw error
        }
    }

    fun login(login: String, password: String) {
        // One at a time: the keyboard's Done submits as well as the button, and a repeat while
        // the first is pending queued another round trip behind it.
        if (loginJob?.isActive == true) return
        loginJob = run {
            require(login.isNotBlank() && password.isNotBlank()) { text(R.string.enter_credentials) }
            val result = NativeBridge.decode<LoginResult>("login", buildJsonObject { put("login", login.trim()); put("password", password) })
            store.saveSecret(result.secret)
            store.saveProfile(result.user)
            _state.update { it.copy(user = result.user) }
            attempt { refreshAccountData(false) }
        }
    }

    fun logout() = run {
        cancelRequests()
        try { NativeBridge.call("logout") } finally {
            store.saveSecret(null)
            listings.clear()
            _state.value = AppState(restoring = false)
        }
    }

    fun selectTab(tab: Tab, isTv: Boolean) {
        cancelRequests()
        detailsBackStack.clear()
        discoveryReturnUrl = null
        val leaving = state.value
        listings[leaving.tab] = Listing(leaving.items, leaving.page, leaving.collectionPath, leaving.collectionTitle, pathReturnFocus)
        val kept = listings[tab]?.takeIf { it.items.isNotEmpty() }
        pathReturnFocus = kept?.returnFocus
        // Items are one slot shared by every grid, so without its own kept listing the previous
        // tab's posters would sit under the new tab's chips until its own page arrives.
        _state.update { it.copy(
            tab = tab,
            details = null,
            collectionPath = kept?.collectionPath,
            collectionTitle = kept?.collectionTitle,
            items = kept?.items.orEmpty(),
            page = kept?.page ?: 1,
            focusedUrl = null,
            error = null,
        ) }
        when (tab) {
            Tab.Catalog -> if (isTv) loadHome() else if (kept == null) loadCatalog()
            Tab.Search -> {
                state.value.user?.userId?.let { userId -> viewModelScope.launch { _state.update { it.copy(searchHistory = store.searchHistory(userId)) } } }
                if (kept == null) state.value.query.takeIf { it.isNotBlank() }?.let(::search)
            }
            Tab.Collections -> if (kept == null && state.value.collections.isEmpty()) loadCollections()
            Tab.Favorites -> if (kept == null) loadFavorites()
            Tab.History -> loadHistory()
            Tab.Notifications -> loadAccountData(true)
            Tab.Account -> loadAccountData(false)
        }
    }

    /** The Catalog tab is the home rails on a television and the flat grid elsewhere; a layout change swaps the source. */
    fun layoutChanged(isTv: Boolean) {
        if (state.value.tab != Tab.Catalog) return
        if (isTv) loadHome() else if (state.value.items.isEmpty()) loadCatalog()
    }

    fun setCatalog(category: CatalogCategory = state.value.category, sort: String = state.value.sort) {
        _state.update { it.copy(category = category, sort = sort, focusedUrl = null) }
        loadCatalog()
    }

    fun loadHome() {
        // Only a complete home is final: one that came back with a rail missing is fetched again
        // on the next visit rather than kept for the life of the session.
        if (state.value.homeSections.size >= HOME_RAIL_COUNT) return
        contentJob?.cancel()
        contentJob = run {
            val sections = NativeBridge.decode<List<HomeSection>>("home")
                .map { section -> section.copy(items = section.items.distinctBy { it.url }) }
                .filter { it.items.isNotEmpty() }
            // An empty home would otherwise leave the skeleton up for good: no rows, no error,
            // and nothing to retry from.
            if (sections.isEmpty()) error(text(R.string.home_failed))
            if (state.value.tab == Tab.Catalog) _state.update { it.copy(homeSections = sections) }
        }
    }

    fun loadCatalog(append: Boolean = false) {
        contentJob?.cancel()
        contentJob = run {
            val category = state.value.category
            val sort = state.value.sort
            val page = if (append) state.value.page + 1 else 1
            val items = NativeBridge.decode<List<MediaItem>>("catalog", buildJsonObject {
                put("category", category.name); put("filter", sort); put("page", page)
            })
            if (state.value.tab == Tab.Catalog && state.value.category == category && state.value.sort == sort) {
                _state.update { it.copy(items = merged(state.value.items, items, append), page = page) }
            }
        }
    }

    /**
     * The next page of a listing, on top of what is already shown. De-duplicated because the
     * grids key their rows by URL and the provider repeats a title across pages whenever its
     * ordering shifts between two requests; a repeated key is a crash, not a duplicate row.
     */
    private fun <T> merged(current: List<T>, page: List<T>, append: Boolean, key: (T) -> String): List<T> =
        (if (append) current + page else page).distinctBy(key)

    private fun merged(current: List<MediaItem>, page: List<MediaItem>, append: Boolean) =
        merged(current, page, append) { it.url }

    fun search(query: String, append: Boolean = false) {
        contentJob?.cancel()
        suggestJob?.cancel()
        if (query.isBlank()) {
            _state.update { it.copy(query = "", items = emptyList()) }
            return
        }
        val normalized = query.trim()
        if (!append) {
            _state.update { it.copy(
                query = normalized,
                suggestions = emptyList(),
                items = emptyList(),
                page = 1,
                focusedUrl = null,
            ) }
        }
        contentJob = run {
            val page = if (append) state.value.page + 1 else 1
            val items = NativeBridge.decode<List<MediaItem>>("search", buildJsonObject { put("query", normalized); put("page", page) })
            if (state.value.tab == Tab.Search && state.value.query == normalized) {
                val history = if (append) state.value.searchHistory else store.saveSearch(state.value.user?.userId ?: return@run, normalized)
                _state.update { it.copy(query = normalized, searchHistory = history, suggestions = emptyList(), items = merged(state.value.items, items, append), page = page) }
            }
        }
    }

    fun clearSearchHistory() = viewModelScope.launch {
        state.value.user?.userId?.let { store.clearSearchHistory(it) }
        _state.update { it.copy(searchHistory = emptyList()) }
    }

    /**
     * Suggestions for what the user is typing. Deliberately not on [contentJob] and not routed
     * through [run]: sharing the content job made every keystroke cancel the search whose results
     * were on screen, and a suggestion lookup has no business raising the global spinner or
     * banner. Cancelling the previous job also cancels its pending delay, which is what keeps
     * this to one request per pause rather than one per character.
     */
    fun suggest(query: String) {
        suggestJob?.cancel()
        if (query.length < 2) {
            _state.update { it.copy(suggestions = emptyList()) }
            return
        }
        suggestJob = viewModelScope.launch {
            delay(SUGGEST_DEBOUNCE_MS)
            val values = runCatching {
                NativeBridge.decode<List<String>>("search_suggestions", buildJsonObject { put("query", query.trim()) })
            }.getOrNull()?.distinct() ?: return@launch
            if (state.value.tab == Tab.Search) _state.update { it.copy(suggestions = values) }
        }
    }

    fun loadCountries() {
        if (state.value.countries.isNotEmpty()) return
        viewModelScope.launch {
            runCatching { NativeBridge.decode<List<Country>>("countries") }
                .onSuccess { countries -> _state.update { it.copy(countries = countries) } }
        }
    }

    fun loadSearchFilters() {
        if (state.value.searchFilters.isNotEmpty()) return
        run {
            val filters = NativeBridge.decode<List<SearchFilter>>("search_filters")
            _state.update { it.copy(searchFilters = filters) }
        }
    }

    fun loadCollections(append: Boolean = false) {
        contentJob?.cancel()
        contentJob = run {
            val page = if (append) state.value.page + 1 else 1
            val values = NativeBridge.decode<List<CollectionItem>>("collections", buildJsonObject { put("page", page) })
            if (state.value.tab == Tab.Collections && state.value.collectionPath == null) {
                _state.update { it.copy(collections = merged(state.value.collections, values, append) { c -> c.url }, page = page) }
            }
        }
    }

    fun openCollection(collection: CollectionItem) {
        openPath(collection.title, collection.url, returnFocus = collection.url)
    }

    fun openPath(title: String, path: String, returnFocus: String? = null) {
        contentJob?.cancel()
        pathReturnFocus = returnFocus
        discoveryReturnUrl = null
        _state.update { it.copy(collectionPath = path, collectionTitle = title, items = emptyList(), page = 1, focusedUrl = null) }
        loadPath(path)
    }

    fun openDiscoveryPath(tab: Tab, title: String, path: String) {
        // The listing replaces the title, so closing it reopens the title rather than the tab.
        discoveryReturnUrl = state.value.details?.url
        discardDetails()
        pathReturnFocus = null
        _state.update { it.copy(tab = tab, collectionPath = path, collectionTitle = title, items = emptyList(), page = 1, focusedUrl = null) }
        loadPath(path)
    }

    fun closeCollection() {
        contentJob?.cancel()
        val returnFocus = pathReturnFocus
        pathReturnFocus = null
        _state.update { it.copy(
            collectionPath = null,
            collectionTitle = null,
            items = emptyList(),
            page = 1,
            focusedUrl = returnFocus,
        ) }
        when (state.value.tab) {
            Tab.Collections -> loadCollections()
            Tab.Search -> state.value.query.takeIf { it.isNotBlank() }?.let(::search)
            else -> Unit
        }
        discoveryReturnUrl?.let { url ->
            discoveryReturnUrl = null
            openDetails(url)
        }
    }

    fun loadPath(path: String? = state.value.collectionPath, append: Boolean = false) {
        val selectedPath = path ?: return
        contentJob?.cancel()
        contentJob = run {
            val page = if (append) state.value.page + 1 else 1
            val values = NativeBridge.decode<List<MediaItem>>("path", buildJsonObject { put("path", selectedPath); put("page", page) })
            if (state.value.collectionPath == selectedPath) _state.update { it.copy(items = merged(state.value.items, values, append), page = page) }
        }
    }

    fun loadAccountData(markRead: Boolean = state.value.tab == Tab.Notifications) {
        contentJob?.cancel()
        contentJob = run {
            refreshAccountData(markRead)
        }
    }

    private suspend fun refreshAccountData(markRead: Boolean) {
        val userId = state.value.user?.userId ?: return
        val data = NativeBridge.decode<AccountData>("account_data")
        if (state.value.user?.userId != userId) return
        val keys = data.notifications.flatMap { group -> group.items.map { "${it.url}|${it.info}" } }.toSet()
        val seen = store.seenNotifications(userId)
        if (markRead) store.markNotificationsSeen(userId, keys)
        _state.update { it.copy(
            notifications = data.notifications,
            premiumDays = data.premiumDays,
            notificationCount = if (markRead) 0 else keys.count { it !in seen },
        ) }
    }

    fun loadFavorites(group: Long? = state.value.favoriteGroup, append: Boolean = false) {
        contentJob?.cancel()
        contentJob = run {
            val groups = if (append) state.value.favoriteGroups else NativeBridge.decode<List<FavoriteGroup>>("favorite_categories")
            val selected = group ?: groups.firstOrNull()?.id
            val page = if (append) state.value.page + 1 else 1
            val items = NativeBridge.decode<List<MediaItem>>("favorites", buildJsonObject { selected?.let { put("category_id", it) }; put("page", page) })
            if (state.value.tab == Tab.Favorites) {
                _state.update { it.copy(favoriteGroups = groups, favoriteGroup = selected, items = merged(state.value.items, items, append), page = page) }
            }
        }
    }

    fun loadHistory() {
        contentJob?.cancel()
        val userId = state.value.user?.userId
        contentJob = run {
            val history = NativeBridge.decode<HistoryResult>("history")
            if (state.value.user?.userId != userId) return@run
            if (state.value.tab == Tab.History) {
                _state.update { it.copy(history = history.entries) }
            }
        }
    }

    fun openDetails(url: String, returnFocus: String? = null) {
        // A second press on the same card while its page loads restarted the load.
        if (state.value.openingUrl == url) return
        // The content job is left running: every load checks its tab is still current before it
        // applies, and cancelling it here left a deep link that arrived mid-load with a home
        // that never finished and nothing to retry.
        detailsJob?.cancel()
        val previousUrl = state.value.details?.url
        if (previousUrl == null) {
            detailsBackStack.clear()
            _state.update { it.copy(focusedUrl = returnFocus) }
        }
        _state.update { it.copy(openingUrl = url) }
        pendingDetailsUrl = url.takeIf { previousUrl != null && previousUrl != url }
        detailsJob = run {
            try {
                loadDetails(url)
                if (previousUrl != null && previousUrl != url) detailsBackStack.addLast(previousUrl)
            } finally {
                if (pendingDetailsUrl == url) pendingDetailsUrl = null
                _state.update { if (it.openingUrl == url) it.copy(openingUrl = null) else it }
            }
        }
    }

    private suspend fun loadDetails(url: String) = coroutineScope {
        // Fetched beside the title, and optional: a slow or failed folder list must not hold the
        // page back or turn a title that loaded into an error.
        val groups = async { attempt { NativeBridge.decode<List<FavoriteGroup>>("favorite_categories") } }
        val details = NativeBridge.decode<MediaDetails>("details", buildJsonObject { put("url", url) })
        val resume = state.value.user?.userId?.let { store.lastWatchedEpisode(it, details.id) }
        _state.update { it.copy(
            details = details,
            favoriteGroups = groups.await() ?: it.favoriteGroups,
            resumeSeasonId = resume?.first,
            resumeEpisodeId = resume?.second,
            resumeTranslatorId = resume?.third,
            episodesTranslatorId = details.translators.firstOrNull()?.id,
        ) }
    }

    fun closeDetails() {
        if (detailsBackLoading) return
        if (pendingDetailsUrl != null) {
            pendingDetailsUrl = null
            detailsJob?.cancel()
            return
        }
        val previousUrl = detailsBackStack.removeLastOrNull()
        if (previousUrl != null) {
            detailsJob?.cancel()
            episodesJob?.cancel()
            playbackJob?.cancel()
            progressJob?.cancel()
            syncJob?.cancel()
            _state.update { it.copy(
                actor = null,
                comments = null,
                trailerUrl = null,
                detailAction = null,
                preparedStream = null,
                stream = null,
                playbackQuality = null,
            ) }
            detailsBackLoading = true
            detailsJob = run {
                try {
                    loadDetails(previousUrl)
                } catch (cancelled: CancellationException) {
                    throw cancelled
                } catch (error: Exception) {
                    detailsBackStack.addLast(previousUrl)
                    throw error
                } finally {
                    detailsBackLoading = false
                }
            }
            return
        }
        discardDetails()
    }

    private fun discardDetails() {
        pendingDetailsUrl = null
        detailsBackLoading = false
        detailsBackStack.clear()
        detailsJob?.cancel()
        episodesJob?.cancel()
        playbackJob?.cancel()
        progressJob?.cancel()
        syncJob?.cancel()
        _state.update { it.copy(
            details = null,
            error = null,
            actor = null,
            comments = null,
            trailerUrl = null,
            detailAction = null,
            resumeSeasonId = null,
            resumeEpisodeId = null,
            resumeTranslatorId = null,
            episodesTranslatorId = null,
            preparedStream = null,
            stream = null,
            playbackQuality = null,
        ) }
    }

    fun openPlayerAction(action: DetailAction, positionMs: Long) {
        saveProgress(positionMs)
        _state.update { it.copy(stream = null, playbackQuality = null, detailAction = action) }
    }

    fun consumeDetailAction() { _state.update { it.copy(detailAction = null) } }

    fun openActor(url: String) {
        detailsJob?.cancel()
        detailsJob = run {
            val actor = NativeBridge.decode<ActorDetails>("actor", buildJsonObject { put("url", url) })
            if (state.value.details != null) _state.update { it.copy(actor = actor) }
        }
    }

    fun closeActor() { _state.update { it.copy(actor = null) } }

    fun loadComments(page: Int = 1) {
        val details = state.value.details ?: return
        detailsJob?.cancel()
        detailsJob = run {
            val comments = NativeBridge.decode<CommentsPage>("comments", buildJsonObject { put("post_id", details.id); put("page", page) })
            if (state.value.details?.id == details.id) _state.update { it.copy(comments = comments) }
        }
    }

    fun closeComments() {
        // The page still on its way would otherwise reopen the dialog just dismissed.
        detailsJob?.cancel()
        _state.update { it.copy(comments = null) }
    }

    fun likeComment(id: String) {
        val details = state.value.details ?: return
        val page = state.value.comments?.page ?: return
        detailsJob?.cancel()
        detailsJob = run {
            NativeBridge.call("like_comment", buildJsonObject { put("id", id) })
            val comments = NativeBridge.decode<CommentsPage>("comments", buildJsonObject {
                put("post_id", details.id)
                put("page", page)
            })
            if (state.value.details?.url == details.url && state.value.comments != null) {
                _state.update { it.copy(comments = comments) }
            }
        }
    }

    fun rate(rating: Int) {
        val details = state.value.details ?: return
        detailsJob?.cancel()
        detailsJob = run {
            NativeBridge.call("rate", buildJsonObject { put("post_id", details.id); put("rating", rating) })
            loadDetails(details.url)
        }
    }

    fun loadTrailer() {
        val details = state.value.details ?: return
        detailsJob?.cancel()
        detailsJob = run {
            val url = NativeBridge.decode<String?>("trailer", buildJsonObject { put("post_id", details.id) })
            _state.update { it.copy(trailerUrl = url) }
        }
    }

    fun clearTrailer() { _state.update { it.copy(trailerUrl = null) } }

    fun toggleSchedule(item: ScheduleItem) {
        if (item.id.isBlank()) return
        val userId = state.value.user?.userId ?: return
        val details = state.value.details ?: return
        detailsJob?.cancel()
        detailsJob = run {
            NativeBridge.call("toggle_schedule_watched", buildJsonObject { put("id", item.id) })
            if (state.value.user?.userId == userId) loadDetails(details.url)
        }
    }

    fun loadEpisodes(translator: Translator) {
        val userId = state.value.user?.userId ?: return
        val details = state.value.details ?: return
        episodesJob?.cancel()
        episodesJob = run {
            val seasons = NativeBridge.decode<List<Season>>("episodes", buildJsonObject {
                put("post_id", details.id)
                put("translator_id", translator.id)
                // The watched state of the episodes lives on the title's schedule.
                put("schedules", NativeBridge.json.encodeToJsonElement(details.schedules))
            })
            if (state.value.user?.userId == userId && state.value.details?.url == details.url) {
                _state.update { it.copy(
                    details = state.value.details?.copy(seasons = seasons),
                    episodesTranslatorId = translator.id,
                ) }
            }
        }
    }

    fun toggleFavorite(groupId: Long, favorite: Boolean) {
        val details = state.value.details ?: return
        detailsJob?.cancel()
        // Ticked at once; the reload below confirms it or puts it back.
        _state.update { state ->
            state.copy(details = state.details?.let { current ->
                current.copy(favoriteCategoryIds = if (favorite) current.favoriteCategoryIds + groupId else current.favoriteCategoryIds - groupId)
            })
        }
        detailsJob = run {
            NativeBridge.call("set_favorite", buildJsonObject { put("url", details.url); put("post_id", details.id); put("category_id", groupId); put("favorite", favorite) })
            // The favourites listing no longer matches: fetched again on the next visit, or now
            // if it is the page under this one.
            listings.remove(Tab.Favorites)
            if (state.value.tab == Tab.Favorites) loadFavorites()
            if (state.value.details?.url == details.url) loadDetails(details.url)
        }
    }

    private suspend fun fetchStream(
        details: MediaDetails,
        translator: Translator,
        season: Long?,
        episode: Long?,
    ): StreamBundle {
        val fields = buildJsonObject {
            put("post_id", details.id)
            if (season == null || episode == null) {
                put("translator", NativeBridge.json.encodeToJsonElement(Translator.serializer(), translator))
            } else {
                put("translator_id", translator.id)
                put("season", season)
                put("episode", episode)
            }
        }
        val type = if (season == null || episode == null) "movie_stream" else "episode_stream"
        return NativeBridge.decode(type, fields)
    }

    fun preparePlayback(translator: Translator, season: Long? = null, episode: Long? = null) {
        val userId = state.value.user?.userId ?: return
        playbackJob?.cancel()
        _state.update { it.copy(preparedStream = null, playbackQuality = null) }
        playbackJob = run {
            val details = state.value.details ?: return@run
            val stream = fetchStream(details, translator, season, episode)
            if (state.value.user?.userId == userId && state.value.details?.url == details.url) {
                _state.update { it.copy(preparedStream = stream) }
            }
        }
    }

    fun startPlayback(
        translator: Translator,
        preferredQuality: String,
        qualityMode: QualityMode,
        season: Long? = null,
        episode: Long? = null,
    ) {
        val userId = state.value.user?.userId ?: return
        playbackJob?.cancel()
        playbackJob = run {
            val details = state.value.details ?: return@run
            val prepared = state.value.preparedStream?.takeIf {
                it.translatorId == translator.id && it.season == season && it.episode == episode
            }
            val stream = prepared ?: fetchStream(details, translator, season, episode)
            val selected = selectStream(stream, qualityMode, preferredQuality)
            if (selected == null) {
                notify(text(R.string.quality_unavailable))
                return@run
            }
            if (state.value.user?.userId == userId && state.value.details?.url == details.url) {
                val positionMs = store.progress(progressKey(userId, details.id, stream))
                _state.update { it.copy(
                    preparedStream = null,
                    stream = stream,
                    playbackQuality = selected.quality,
                    playbackPositionMs = positionMs,
                ) }
            }
        }
    }

    fun cancelPlaybackPreparation() {
        playbackJob?.cancel()
        _state.update { it.copy(preparedStream = null, playbackQuality = null) }
    }

    fun playbackStarted(onResult: (Boolean) -> Unit) {
        val userId = state.value.user?.userId ?: return
        val details = state.value.details ?: return
        val stream = state.value.stream ?: return
        // A rewatch of the same episode has to reach the provider again.
        markedWatchedKey = null
        syncJob?.cancel()
        syncJob = run {
            try {
                rememberLastPlayed(userId, details.id, stream.season, stream.episode, stream.translatorId)
                NativeBridge.call("save_watch", buildJsonObject {
                    put("post_id", details.id)
                    put("translator_id", stream.translatorId)
                    stream.season?.let { put("season", it) }
                    stream.episode?.let { put("episode", it) }
                })
                if (state.value.user?.userId == userId) onResult(true)
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (error: Exception) {
                // Retried when playback resumes. Banner-ing it on every resume would
                // say nothing the user can act on while the title is already playing.
                onResult(false)
            }
        }
    }

    fun saveProgress(positionMs: Long) {
        val userId = state.value.user?.userId ?: return
        val details = state.value.details ?: return
        val stream = state.value.stream ?: return
        val key = progressKey(userId, details.id, stream)
        // Kept on the state as well as on disk: the player screen is rebuilt from scratch on a
        // configuration change while this view model survives it, so a stale value here is what
        // the rebuilt screen resumes from.
        _state.update { it.copy(playbackPositionMs = positionMs) }
        progressJob?.cancel()
        progressJob = viewModelScope.launch { store.saveProgress(key, positionMs) }
    }

    fun playbackCompleted() {
        val userId = state.value.user?.userId ?: return
        val details = state.value.details ?: return
        val stream = state.value.stream ?: return
        completionJob?.cancel()
        completionJob = run {
            if (state.value.user?.userId != userId) return@run
            markWatched(userId, details, stream)
            if (state.value.user?.userId != userId) return@run
            store.clearProgress(progressKey(userId, details.id, stream))
            if (state.value.tab == Tab.History) refreshHistory()
        }
    }

    /** Keep what the details page reopens on, in the store and on the state it is already showing. */
    private suspend fun rememberLastPlayed(userId: String, postId: Long, season: Long?, episode: Long?, translatorId: Long) {
        store.saveLastEpisode(userId, postId, season, episode, translatorId)
        if (state.value.details?.id == postId) {
            _state.update { it.copy(resumeSeasonId = season, resumeEpisodeId = episode, resumeTranslatorId = translatorId) }
        }
    }

    /** The (season, episode) `offset` steps from the playing one across all seasons, if any. */
    private fun adjacentEpisode(details: MediaDetails, stream: StreamBundle, offset: Int): Pair<Long, Long>? {
        val episodes = details.seasons.flatMap { season -> season.episodes.map { season.id to it.id } }
        val index = episodes.indexOfFirst { (seasonId, episode) -> seasonId == stream.season && episode == stream.episode }
        return if (index >= 0) episodes.getOrNull(index + offset) else null
    }

    fun hasAdjacentEpisode(offset: Int): Boolean {
        val details = state.value.details ?: return false
        val stream = state.value.stream ?: return false
        return adjacentEpisode(details, stream, offset) != null
    }

    fun previousEpisode(completed: Boolean) = playAdjacentEpisode(-1, completed)
    fun nextEpisode(completed: Boolean) = playAdjacentEpisode(1, completed)

    /** [fallback] is set when the player dropped to [quality] on its own after a failure. */
    fun selectPlaybackQuality(quality: String, fallback: Boolean = false) {
        _state.update { it.copy(playbackQuality = quality) }
        if (fallback) notify(getApplication<Application>().getString(R.string.quality_fallback, quality))
    }

    fun playEpisode(season: Long, episode: Long, completed: Boolean = false) {
        val userId = state.value.user?.userId ?: return
        val details = state.value.details ?: return
        val current = state.value.stream ?: return
        val translator = details.translators.firstOrNull { it.id == current.translatorId } ?: return
        playbackJob?.cancel()
        playbackJob = run {
            if (completed) {
                markWatched(userId, details, current)
                store.clearProgress(progressKey(userId, details.id, current))
            }
            val next = fetchStream(details, translator, season, episode)
            if (state.value.user?.userId != userId || state.value.details?.url != details.url || state.value.stream != current) return@run
            val selected = selectStream(next, QualityMode.Max, state.value.playbackQuality)
            if (selected == null) {
                notify(text(R.string.quality_unavailable))
            } else {
                val positionMs = store.progress(progressKey(userId, details.id, next))
                _state.update { it.copy(stream = next, playbackQuality = selected.quality, playbackPositionMs = positionMs) }
            }
        }
    }

    private fun playAdjacentEpisode(offset: Int, completed: Boolean) {
        val userId = state.value.user?.userId ?: return
        val details = state.value.details ?: return
        val current = state.value.stream ?: return
        val (season, episode) = adjacentEpisode(details, current, offset) ?: return
        val translator = details.translators.firstOrNull { it.id == current.translatorId } ?: return
        playbackJob?.cancel()
        playbackJob = run {
            if (completed) {
                markWatched(userId, details, current)
                store.clearProgress(progressKey(userId, details.id, current))
            }
            val next = fetchStream(details, translator, season, episode)
            if (state.value.user?.userId != userId || state.value.details?.url != details.url || state.value.stream != current) return@run
            val selected = selectStream(next, QualityMode.Max, state.value.playbackQuality)
            if (selected == null) {
                notify(text(R.string.quality_unavailable))
            } else {
                val positionMs = store.progress(progressKey(userId, details.id, next))
                _state.update { it.copy(
                    stream = next,
                    playbackQuality = selected.quality,
                    playbackPositionMs = positionMs,
                ) }
            }
        }
    }

    fun closePlayer(completed: Boolean, positionMs: Long) {
        playbackJob?.cancel()
        val details = state.value.details
        val stream = state.value.stream
        val userId = state.value.user?.userId
        progressJob?.cancel()
        // The error slot is shared by every operation, so a playback message left in it would be
        // banner-ed over whatever screen the user lands on next.
        _state.update { it.copy(stream = null, playbackQuality = null, error = null) }
        if (details == null || stream == null || userId == null) return
        // Shown as watched at once where the list is already up: the account's history lags the
        // sync by a moment, and the list fetched right after it still said otherwise.
        if (completed) _state.update { state ->
            state.copy(history = state.history.map { if (it.url == details.url) it.copy(watched = true) else it })
        }
        run {
            if (state.value.user?.userId != userId) return@run
            if (completed) {
                markWatched(userId, details, stream)
                if (state.value.user?.userId != userId) return@run
                store.clearProgress(progressKey(userId, details.id, stream))
                // A finished episode is not the one to reopen on; offer the next one instead.
                adjacentEpisode(details, stream, 1)?.let { (season, episode) ->
                    rememberLastPlayed(userId, details.id, season, episode, stream.translatorId)
                }
            } else {
                store.saveProgress(progressKey(userId, details.id, stream), positionMs)
            }
            // The account's history moved either way, watched or not: the entry carries where
            // the title was left. Reflected in an open History list without the global spinner.
            if (state.value.tab == Tab.History) refreshHistory()
        }
    }

    /**
     * Best-effort: marking watched depends on the server surfacing the save in time.
     * Never let a transient "history item not found yet" / network hiccup wipe the
     * completed state, surface an error banner, or drop the cleared progress. A
     * finished playback is finalized locally first.
     *
     * Playback that ends and is then closed reports the same title twice; the key
     * of the last accepted call keeps the second one off the network.
     */
    private suspend fun markWatched(userId: String, details: MediaDetails, stream: StreamBundle) {
        val key = progressKey(userId, details.id, stream)
        if (markedWatchedKey == key) return
        runCatching {
            NativeBridge.call("mark_watched", buildJsonObject {
                put("url", details.url)
                put("post_id", details.id)
                stream.season?.let { put("season", it) }
                stream.episode?.let { put("episode", it) }
            })
        }.onSuccess { markedWatchedKey = key }
    }

    /** Quiet re-fetch of history (no loading/error mutation) so a freshly watched item
     *  shows its synced state immediately when the History tab is already in view. */
    private fun refreshHistory() {
        contentJob?.cancel()
        contentJob = viewModelScope.launch {
            runCatching {
                val userId = state.value.user?.userId ?: return@launch
                val result = NativeBridge.decode<HistoryResult>("history")
                if (state.value.user?.userId != userId) return@launch
                if (state.value.tab == Tab.History) {
                    _state.update { it.copy(history = result.entries) }
                }
            }
        }
    }

    private fun progressKey(userId: String, mediaId: Long, stream: StreamBundle) = "$userId:$mediaId:${stream.season ?: 0}:${stream.episode ?: 0}"

    private fun cancelRequests() {
        contentJob?.cancel()
        suggestJob?.cancel()
        detailsJob?.cancel()
        episodesJob?.cancel()
        playbackJob?.cancel()
        progressJob?.cancel()
        syncJob?.cancel()
        completionJob?.cancel()
    }

    fun toggleHistory(entry: HistoryEntry) {
        contentJob?.cancel()
        val userId = state.value.user?.userId ?: return
        contentJob = run {
            NativeBridge.call("set_history_watched", buildJsonObject { put("id", entry.id); put("watched", !entry.watched) })
            val history = NativeBridge.decode<HistoryResult>("history")
            if (state.value.user?.userId == userId && state.value.tab == Tab.History) {
                _state.update { it.copy(history = history.entries) }
            }
        }
    }

    fun removeHistory(entry: HistoryEntry) {
        contentJob?.cancel()
        val userId = state.value.user?.userId ?: return
        contentJob = run {
            val removed = NativeBridge.decode<RemovedHistory>("remove_history", buildJsonObject { put("id", entry.id) })
            if (state.value.user?.userId != userId) return@run
            store.clearProgressForMedia(userId, removed.mediaId)
            val history = NativeBridge.decode<HistoryResult>("history")
            if (state.value.user?.userId == userId && state.value.tab == Tab.History) {
                _state.update { it.copy(history = history.entries) }
            }
        }
    }

    private fun notify(text: String) {
        _effects.trySend(AppEffect.Message(text))
    }

    private fun text(id: Int): String = getApplication<Application>().getString(id)

    fun clearError() { _state.update { it.copy(error = null) } }

    /** Re-runs the load for whatever tab is currently visible — used by the error banner's Retry. */
    fun retry(isTv: Boolean) {
        clearError()
        state.value.collectionPath?.let {
            loadPath(it)
            return
        }
        when (state.value.tab) {
            Tab.Catalog -> if (isTv) loadHome() else loadCatalog()
            Tab.Collections -> loadCollections()
            Tab.Favorites -> loadFavorites()
            Tab.History -> loadHistory()
            Tab.Notifications, Tab.Account -> loadAccountData(markRead = state.value.tab == Tab.Notifications)
            Tab.Search -> {
                state.value.query.takeIf { it.isNotBlank() }?.let(::search)
                if (state.value.searchFilters.isEmpty()) loadSearchFilters()
            }
        }
    }
}

/**
 * Runs a best-effort suspend call, returning null on failure. `runCatching` around a suspend
 * body would also catch [CancellationException], which a coroutine's own cancellation relies on
 * propagating rather than being swallowed as an ordinary failure.
 */
private suspend inline fun <T> attempt(block: () -> T): T? = try {
    block()
} catch (cancelled: CancellationException) {
    throw cancelled
} catch (_: Exception) {
    null
}

/** Rails the core assembles for the home page; fewer means one of them failed. */
private const val HOME_RAIL_COUNT = 5
private const val RESTORE_TIMEOUT_MS = 10_000L

/** Pause after the last keystroke in the hidden-countries field before the filter is applied. */
private const val HIDDEN_COUNTRIES_SETTLE_MS = 600L

/** A tab's listing as the user left it: its items, the page they reach, and an open collection. */
private class Listing(
    val items: List<MediaItem>,
    val page: Int,
    val collectionPath: String?,
    val collectionTitle: String?,
    val returnFocus: String?,
)
