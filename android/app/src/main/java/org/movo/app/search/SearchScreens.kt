@file:OptIn(
    ExperimentalMaterial3Api::class,
    ExperimentalAnimationApi::class,
    ExperimentalMaterial3WindowSizeClassApi::class,
    ExperimentalComposeUiApi::class,
    ExperimentalTvMaterial3Api::class,
)

package org.movo.app.search

import org.movo.app.ui.TV_OVERSCAN_HORIZONTAL
import org.movo.app.ui.TV_OVERSCAN_VERTICAL
import org.movo.app.catalog.MediaGrid
import org.movo.app.catalog.PathHeader
import org.movo.app.ui.ChoiceRow
import org.movo.app.ui.searchEmptyHint
import org.movo.app.ui.searchEmptyTitle
import org.movo.app.ui.tvFocusMemory
import org.movo.app.ui.tvFocusScale
import org.movo.app.R
import org.movo.app.core.AppState
import org.movo.app.core.MovoViewModel
import org.movo.app.core.SearchFilter
import android.content.Intent
import android.speech.RecognizerIntent
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.ExperimentalAnimationApi
import androidx.compose.foundation.focusGroup
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.material3.windowsizeclass.ExperimentalMaterial3WindowSizeClassApi
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.focusRestorer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import androidx.tv.material3.Button as TvButton
import androidx.tv.material3.ExperimentalTvMaterial3Api
import androidx.tv.material3.FilterChip as TvFilterChip
import androidx.tv.material3.IconButton as TvIconButton
import androidx.tv.material3.SelectableChipScale
import androidx.tv.material3.Icon as TvIcon
import androidx.tv.material3.Text as TvText

@Composable
internal fun SearchScreen(state: AppState, isTv: Boolean, model: MovoViewModel) {
    var query by rememberSaveable { mutableStateOf(state.query) }
    var showFilters by remember { mutableStateOf(false) }
    val context = LocalContext.current
    val voiceIntent = remember {
        Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH)
            .putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
    }
    val voiceAvailable = remember(context) { voiceIntent.resolveActivity(context.packageManager) != null }
    val voice = rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
        result.data?.getStringArrayListExtra(RecognizerIntent.EXTRA_RESULTS)?.firstOrNull()?.let {
            query = it
            model.search(it)
        }
    }
    LaunchedEffect(state.query) { query = state.query }
    LaunchedEffect(Unit) { model.loadSearchFilters() }
    if (state.collectionPath != null) {
        Column(Modifier.fillMaxSize()) {
            PathHeader(state.collectionTitle.orEmpty(), isTv, model::closeCollection)
            key(state.collectionPath) {
                MediaGrid(
                    state.items,
                    isTv,
                    state.loading,
                    state.focusedUrl,
                    { url -> model.openDetails(url, url) },
                    emptyTitle = stringResource(R.string.collection_empty),
                ) { model.loadPath(append = true) }
            }
        }
        BackHandler { model.closeCollection() }
        return
    }
    if (isTv) {
        TvSearchContent(
            state = state,
            query = query,
            updateQuery = { value -> query = value; model.suggest(value) },
            voiceAvailable = voiceAvailable,
            launchVoice = { voice.launch(voiceIntent) },
            search = { model.search(query) },
            chooseHistory = { value -> query = value; model.search(value) },
            clearHistory = model::clearSearchHistory,
            openFilters = { showFilters = true },
            open = { url -> model.openDetails(url, url) },
            loadMore = { model.search(query, true) },
        )
        if (showFilters) SearchFiltersDialog(state.searchFilters, true, { showFilters = false }) { title, path ->
            showFilters = false
            model.openPath(title, path)
        }
        return
    }
    Column(Modifier.fillMaxSize().padding(top = 12.dp)) {
        OutlinedTextField(
            value = query,
            onValueChange = { query = it; model.suggest(it) },
            modifier = Modifier
                .widthIn(max = if (isTv) 840.dp else 720.dp)
                .fillMaxWidth()
                .align(Alignment.CenterHorizontally)
                .padding(horizontal = 12.dp),
            label = { Text(stringResource(R.string.search_hint)) },
            trailingIcon = {
                Row {
                    IconButton({ voice.launch(voiceIntent) }, Modifier.tvFocusScale(isTv), enabled = voiceAvailable) {
                        Icon(Icons.Default.Mic, stringResource(R.string.voice_search))
                    }
                    IconButton({ model.search(query) }, Modifier.tvFocusScale(isTv), enabled = query.isNotBlank()) {
                        Icon(Icons.Default.Search, stringResource(R.string.search))
                    }
                }
            },
            singleLine = true,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
            keyboardActions = KeyboardActions(onSearch = { if (query.isNotBlank()) model.search(query) }),
        )
        if (state.suggestions.isNotEmpty()) {
            LazyRow(contentPadding = PaddingValues(horizontal = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                items(state.suggestions, key = { it }) { suggestion ->
                    SuggestionChip(onClick = { query = suggestion; model.search(suggestion) }, label = { Text(suggestion) }, modifier = Modifier.tvFocusScale(isTv))
                }
            }
        }
        if (query.isBlank() && state.searchHistory.isNotEmpty()) {
            Row(Modifier.fillMaxWidth().padding(horizontal = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                Text(stringResource(R.string.recent_searches), style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
                TextButton(model::clearSearchHistory, Modifier.tvFocusScale(isTv)) { Text(stringResource(R.string.clear)) }
            }
            LazyRow(contentPadding = PaddingValues(horizontal = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                items(state.searchHistory, key = { it }) { value ->
                    AssistChip(onClick = { query = value; model.search(value) }, label = { Text(value) }, modifier = Modifier.tvFocusScale(isTv), leadingIcon = { Icon(Icons.Default.History, null) })
                }
            }
        }
        if (state.searchFilters.isNotEmpty()) {
            LazyRow(contentPadding = PaddingValues(horizontal = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                item { FilledTonalButton({ showFilters = true }, Modifier.tvFocusScale(isTv)) { Icon(Icons.Default.Tune, null); Text(stringResource(R.string.advanced_search)) } }
            }
        }
        Spacer(Modifier.height(8.dp))
        key(state.query) {
            MediaGrid(
                state.items,
                isTv,
                state.loading,
                state.focusedUrl,
                { url -> model.openDetails(url, url) },
                emptyTitle = searchEmptyTitle(state.query),
                emptyHint = searchEmptyHint(state.query),
            ) { model.search(query, true) }
        }
    }
    if (showFilters) SearchFiltersDialog(state.searchFilters, isTv, { showFilters = false }) { title, path ->
        showFilters = false
        model.openPath(title, path)
    }
}

@Composable
internal fun TvSearchContent(
    state: AppState,
    query: String,
    updateQuery: (String) -> Unit,
    voiceAvailable: Boolean,
    launchVoice: () -> Unit,
    search: () -> Unit,
    chooseHistory: (String) -> Unit,
    clearHistory: () -> Unit,
    openFilters: () -> Unit,
    open: (String) -> Unit,
    loadMore: () -> Unit,
) {
    Column(
        Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(
            Modifier
                .fillMaxWidth()
                .padding(start = TV_OVERSCAN_HORIZONTAL, end = TV_OVERSCAN_HORIZONTAL, top = TV_OVERSCAN_VERTICAL)
                .focusRestorer()
                .focusGroup(),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            OutlinedTextField(
                value = query,
                onValueChange = updateQuery,
                modifier = Modifier
                    .weight(1f)
                    .tvFocusMemory("search:field")
                    .testTag("tv-search-field"),
                label = { Text(stringResource(R.string.search_hint)) },
                singleLine = true,
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                keyboardActions = KeyboardActions(onSearch = { if (query.isNotBlank()) search() }),
            )
            TvIconButton(
                onClick = launchVoice,
                modifier = Modifier.tvFocusMemory("search:voice").testTag("tv-search-voice"),
                enabled = voiceAvailable,
            ) {
                TvIcon(Icons.Default.Mic, stringResource(R.string.voice_search))
            }
            TvIconButton(
                onClick = search,
                modifier = Modifier.tvFocusMemory("search:submit").testTag("tv-search-submit"),
                enabled = query.isNotBlank(),
            ) {
                TvIcon(Icons.Default.Search, stringResource(R.string.search))
            }
        }
        if (state.suggestions.isNotEmpty()) {
            LazyRow(
                contentPadding = PaddingValues(horizontal = TV_OVERSCAN_HORIZONTAL),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.focusRestorer().focusGroup(),
            ) {
                items(state.suggestions, key = { it }) { suggestion ->
                    TvFilterChip(
                        selected = false,
                        onClick = { chooseHistory(suggestion) },
                        modifier = Modifier.tvFocusMemory("search:suggestion:$suggestion"),
                        scale = SelectableChipScale.None,
                    ) {
                        TvText(suggestion)
                    }
                }
            }
        }
        if (query.isBlank() && state.searchHistory.isNotEmpty()) {
            Row(
                Modifier.fillMaxWidth().padding(horizontal = TV_OVERSCAN_HORIZONTAL),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    stringResource(R.string.recent_searches),
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.weight(1f),
                )
                TvButton(
                    onClick = clearHistory,
                    modifier = Modifier.tvFocusMemory("search:clear"),
                ) { TvText(stringResource(R.string.clear)) }
            }
            LazyRow(
                contentPadding = PaddingValues(horizontal = TV_OVERSCAN_HORIZONTAL),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.focusRestorer().focusGroup(),
            ) {
                items(state.searchHistory, key = { it }) { value ->
                    TvFilterChip(
                        selected = false,
                        onClick = { chooseHistory(value) },
                        modifier = Modifier.tvFocusMemory("search:history:$value"),
                        scale = SelectableChipScale.None,
                    ) {
                        TvText(value)
                    }
                }
            }
        }
        if (state.searchFilters.isNotEmpty()) {
            Box(Modifier.padding(horizontal = TV_OVERSCAN_HORIZONTAL)) {
                TvButton(
                    onClick = openFilters,
                    modifier = Modifier.tvFocusMemory("search:filters").testTag("tv-search-filter-row"),
                ) {
                    TvIcon(Icons.Default.Tune, contentDescription = null)
                    TvText(stringResource(R.string.advanced_search))
                }
            }
        }
        Box(Modifier.weight(1f).fillMaxWidth()) {
            MediaGrid(
                state.items,
                true,
                state.loading,
                state.focusedUrl,
                open,
                emptyTitle = searchEmptyTitle(state.query),
                emptyHint = searchEmptyHint(state.query),
                loadMore = loadMore,
            )
        }
    }
}

internal fun searchFilterPath(genre: String, year: String) = when (year) {
    "-1" -> genre.removeSuffix("/best")
    "0", "" -> genre
    else -> genre + year
}

@Composable
private fun SearchFiltersDialog(filters: List<SearchFilter>, isTv: Boolean, dismiss: () -> Unit, apply: (String, String) -> Unit) {
    val recentLabel = stringResource(R.string.recent)
    var filter by remember(filters) { mutableStateOf(filters.first()) }
    var genre by remember(filter) { mutableStateOf(filter.genres.firstOrNull()) }
    var year by remember(filter) { mutableStateOf(filter.years.firstOrNull()) }
    AlertDialog(
        onDismissRequest = dismiss,
        title = { Text(stringResource(R.string.advanced_search)) },
        text = {
            Column(
                Modifier.then(if (isTv) Modifier.focusRestorer().focusGroup() else Modifier),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                if (isTv) {
                    TvChoiceRow(stringResource(R.string.category), filters, filter, { it.name }, { it.name }, {
                        filter = it; genre = it.genres.firstOrNull(); year = it.years.firstOrNull()
                    })
                    TvChoiceRow(stringResource(R.string.genre), filter.genres, genre, { if (it.url == "-1") recentLabel else it.name }, { it.url }, { genre = it })
                    TvChoiceRow(stringResource(R.string.year), filter.years, year, { if (it.url == "-1") recentLabel else it.name }, { it.url }, { year = it })
                } else {
                    ChoiceRow(stringResource(R.string.category), filters, filter, { it.name }, { it.name }, {
                        filter = it; genre = it.genres.firstOrNull(); year = it.years.firstOrNull()
                    })
                    ChoiceRow(stringResource(R.string.genre), filter.genres, genre, { if (it.url == "-1") recentLabel else it.name }, { it.url }, { genre = it })
                    ChoiceRow(stringResource(R.string.year), filter.years, year, { if (it.url == "-1") recentLabel else it.name }, { it.url }, { year = it })
                }
            }
        },
        confirmButton = {
            if (isTv) {
                TvButton(
                    onClick = { genre?.let { apply(filter.name, searchFilterPath(it.url, year?.url.orEmpty())) } },
                    enabled = genre != null,
                ) { TvText(stringResource(R.string.search)) }
            } else {
                TextButton(onClick = { genre?.let { apply(filter.name, searchFilterPath(it.url, year?.url.orEmpty())) } }, enabled = genre != null) { Text(stringResource(R.string.search)) }
            }
        },
        dismissButton = {
            if (isTv) TvButton(onClick = dismiss) { TvText(stringResource(R.string.cancel)) }
            else TextButton(dismiss) { Text(stringResource(R.string.cancel)) }
        },
    )
}

@Composable
private fun <T> TvChoiceRow(
    title: String,
    values: List<T>,
    selected: T?,
    label: (T) -> String,
    itemKey: (T) -> Any,
    choose: (T) -> Unit,
) {
    Column {
        Text(title, style = MaterialTheme.typography.titleMedium)
        LazyRow(
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier.focusRestorer().focusGroup(),
        ) {
            items(values, key = itemKey) { value ->
                TvFilterChip(
                    selected = value == selected,
                    onClick = { choose(value) },
                    scale = SelectableChipScale.None,
                ) {
                    TvText(label(value))
                }
            }
        }
    }
}
