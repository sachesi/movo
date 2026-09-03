@file:OptIn(
    ExperimentalMaterial3Api::class,
    ExperimentalAnimationApi::class,
    ExperimentalMaterial3WindowSizeClassApi::class,
    ExperimentalComposeUiApi::class,
    ExperimentalTvMaterial3Api::class,
)

package org.movo.app.catalog

import org.movo.app.ui.TV_OVERSCAN_HORIZONTAL
import org.movo.app.ui.TV_OVERSCAN_VERTICAL
import org.movo.app.ui.Empty
import org.movo.app.ui.Loading
import org.movo.app.ui.MediaGridSkeleton
import org.movo.app.ui.MovoChoiceChip
import org.movo.app.ui.TvHomeSkeleton
import org.movo.app.ui.tvFocusMemory
import org.movo.app.R
import org.movo.app.core.AppState
import org.movo.app.core.CatalogCategory
import org.movo.app.core.CollectionItem
import org.movo.app.core.HomeSection
import org.movo.app.core.MediaItem
import org.movo.app.core.MovoViewModel
import androidx.activity.compose.BackHandler
import androidx.compose.animation.ExperimentalAnimationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.focusGroup
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.material3.windowsizeclass.ExperimentalMaterial3WindowSizeClassApi
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.focusRestorer
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.shape.RoundedCornerShape
import coil3.compose.AsyncImage
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.flow.distinctUntilChanged
import androidx.tv.material3.Card as TvCard
import androidx.tv.material3.CardScale
import androidx.tv.material3.Button as TvButton
import androidx.tv.material3.ExperimentalTvMaterial3Api
import androidx.tv.material3.IconButton as TvIconButton
import androidx.tv.material3.Icon as TvIcon
import androidx.tv.material3.Text as TvText

@Composable
internal fun CatalogScreen(
    state: AppState,
    isTv: Boolean,
    compactHeight: Boolean,
    model: MovoViewModel,
) {
    if (isTv) {
        if (state.homeSections.isNotEmpty()) {
            TvHomeScreen(state.homeSections, state.loading) { url, key ->
                model.openDetails(url, key)
            }
        } else {
            // TV home is the primary surface — never flash the flat catalog grid while
            // the sections load (the "page loads then swaps to hot news" symptom).
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                if (state.loading || state.error == null) TvHomeSkeleton()
                else Empty(stringResource(R.string.home_failed), hint = stringResource(R.string.home_failed_hint))
            }
        }
        return
    }
    Column(Modifier.fillMaxSize()) {
        if (compactHeight) {
            LazyRow(
                contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                item { Text(stringResource(R.string.category), style = MaterialTheme.typography.labelLarge) }
                items(CatalogCategory.entries) { category ->
                    MovoChoiceChip(
                        selected = state.category == category,
                        onClick = { model.setCatalog(category = category) },
                        label = { Text(stringResource(category.label)) },
                        isTv = isTv,
                    )
                }
                item { VerticalDivider(Modifier.height(32.dp)) }
                item { Text(stringResource(R.string.sort_by), style = MaterialTheme.typography.labelLarge) }
                items(SORT_OPTIONS) { (value, label) ->
                    MovoChoiceChip(
                        selected = state.sort == value,
                        onClick = { model.setCatalog(sort = value) },
                        label = { Text(stringResource(label)) },
                        isTv = isTv,
                    )
                }
            }
        } else {
            Text(
                stringResource(R.string.category),
                style = MaterialTheme.typography.labelLarge,
                modifier = Modifier.padding(start = 12.dp, top = 12.dp),
            )
            LazyRow(
                contentPadding = PaddingValues(horizontal = 12.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(CatalogCategory.entries) { category ->
                    MovoChoiceChip(
                        selected = state.category == category,
                        onClick = { model.setCatalog(category = category) },
                        label = { Text(stringResource(category.label)) },
                        isTv = isTv,
                    )
                }
            }
            Text(
                stringResource(R.string.sort_by),
                style = MaterialTheme.typography.labelLarge,
                modifier = Modifier.padding(start = 12.dp, top = 8.dp),
            )
            LazyRow(
                contentPadding = PaddingValues(horizontal = 12.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(SORT_OPTIONS) { (value, label) ->
                    MovoChoiceChip(
                        selected = state.sort == value,
                        onClick = { model.setCatalog(sort = value) },
                        label = { Text(stringResource(label)) },
                        isTv = isTv,
                    )
                }
            }
        }
        key(state.category, state.sort) {
            MediaGrid(
                state.items,
                isTv,
                state.loading,
                state.focusedUrl,
                { url -> model.openDetails(url, url) },
                emptyTitle = stringResource(R.string.catalog_empty),
                emptyHint = stringResource(R.string.catalog_empty_hint),
            ) { model.loadCatalog(true) }
        }
    }
}

@Composable
internal fun TvHomeScreen(
    sections: List<HomeSection>,
    loading: Boolean,
    open: (String, String) -> Unit,
) {
    LazyColumn(
        Modifier.fillMaxSize().testTag("tv-home-list").focusRestorer().focusGroup(),
        contentPadding = PaddingValues(horizontal = TV_OVERSCAN_HORIZONTAL, vertical = TV_OVERSCAN_VERTICAL),
        verticalArrangement = Arrangement.spacedBy(20.dp),
    ) {
        sections.filter { it.items.isNotEmpty() }.forEach { section ->
            item(section.id) {
                Column(
                    Modifier.testTag("tv-home-rail-${section.id}"),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    Text(
                        stringResource(section.title),
                        style = MaterialTheme.typography.titleLarge,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    LazyRow(
                        horizontalArrangement = Arrangement.spacedBy(16.dp),
                        modifier = Modifier.focusRestorer().focusGroup(),
                    ) {
                        items(section.items, key = { media -> media.url }) { media ->
                            val key = "${section.id}:${media.url}"
                            Box(Modifier.width(176.dp).testTag("tv-home-card-$key")) {
                                MediaCard(
                                    media,
                                    true,
                                    focusKey = key,
                                ) { open(media.url, key) }
                            }
                        }
                    }
                }
            }
        }
        if (loading) item { LinearProgressIndicator(Modifier.fillMaxWidth()) }
    }
}

private val HomeSection.title: Int get() = when (id) {
    "hot" -> R.string.home_hot
    "new" -> R.string.sort_new
    "watching" -> R.string.sort_watching
    "popular" -> R.string.sort_popular
    "awaiting" -> R.string.home_awaiting
    else -> R.string.nav_catalog
}

private val SORT_OPTIONS = listOf(
    "popular" to R.string.sort_popular,
    "watching" to R.string.sort_watching,
    "new" to R.string.sort_new,
    "last" to R.string.sort_last,
)

private val CatalogCategory.label: Int get() = when (this) {
    CatalogCategory.All -> R.string.category_all
    CatalogCategory.Films -> R.string.category_films
    CatalogCategory.Series -> R.string.category_series
    CatalogCategory.Cartoons -> R.string.category_cartoons
    CatalogCategory.Animation -> R.string.category_animation
}

@Composable
internal fun CollectionsScreen(state: AppState, isTv: Boolean, model: MovoViewModel) {
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
    if (state.collections.isEmpty()) {
        if (state.loading) Loading(stringResource(R.string.loading_content))
        else Empty(stringResource(R.string.collections_empty))
        return
    }
    val initialIndex = remember(state.collections, state.focusedUrl) {
        state.collections.indexOfFirst { it.url == state.focusedUrl }.coerceAtLeast(0)
    }
    val gridState = rememberLazyGridState(initialFirstVisibleItemIndex = initialIndex)
    LazyVerticalGrid(
        state = gridState,
        columns = if (isTv) GridCells.Fixed(5) else GridCells.Adaptive(220.dp),
        contentPadding = if (isTv) PaddingValues(horizontal = TV_OVERSCAN_HORIZONTAL, vertical = TV_OVERSCAN_VERTICAL) else PaddingValues(12.dp),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
        modifier = if (isTv) Modifier.focusRestorer().focusGroup() else Modifier.focusGroup(),
    ) {
        items(state.collections, key = { it.url }) { collection ->
            val modifier = if (isTv) Modifier.tvFocusMemory(collection.url) else Modifier
            if (isTv) {
                TvCard(
                    onClick = { model.openCollection(collection) },
                    modifier = modifier,
                    scale = CardScale.None,
                ) {
                    CollectionCardContent(collection)
                }
            } else {
                Card(onClick = { model.openCollection(collection) }, modifier = modifier) {
                    CollectionCardContent(collection)
                }
            }
        }
        item(span = { GridItemSpan(maxLineSpan) }) {
            if (isTv) {
                TvButton(
                    onClick = { model.loadCollections(true) },
                    modifier = Modifier.fillMaxWidth().tvFocusMemory("collections:more"),
                ) { TvText(stringResource(R.string.load_more)) }
            } else {
                TextButton({ model.loadCollections(true) }, Modifier.fillMaxWidth()) {
                    Text(stringResource(R.string.load_more))
                }
            }
        }
    }
}

@Composable
internal fun PathHeader(title: String, isTv: Boolean, back: () -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .padding(horizontal = if (isTv) TV_OVERSCAN_HORIZONTAL else 12.dp, vertical = if (isTv) TV_OVERSCAN_VERTICAL else 0.dp)
            .then(if (isTv) Modifier.focusRestorer().focusGroup() else Modifier),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        if (isTv) {
            TvIconButton(onClick = back, modifier = Modifier.tvFocusMemory("path:back")) {
                TvIcon(Icons.AutoMirrored.Filled.ArrowBack, stringResource(R.string.back))
            }
        } else {
            IconButton(back) { Icon(Icons.AutoMirrored.Filled.ArrowBack, stringResource(R.string.back)) }
        }
        Text(title, style = MaterialTheme.typography.titleLarge)
    }
}

@Composable
private fun CollectionCardContent(collection: CollectionItem) {
                AsyncImage(collection.imageUrl, null, Modifier.fillMaxWidth().aspectRatio(16f / 9f), contentScale = ContentScale.Crop)
                ListItem(
                    headlineContent = { Text(collection.title, maxLines = 2, overflow = TextOverflow.Ellipsis) },
                    supportingContent = { Text(pluralStringResource(R.plurals.collection_items, collection.count, collection.count)) },
                    colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                )
}

@Composable
internal fun MediaGrid(
    items: List<MediaItem>,
    isTv: Boolean,
    loading: Boolean,
    focusedUrl: String?,
    open: (String) -> Unit,
    emptyTitle: String,
    emptyHint: String? = null,
    entryFocusRequester: FocusRequester? = null,
    loadMore: (() -> Unit)? = null,
) {
    if (items.isEmpty()) {
        if (loading) MediaGridSkeleton(isTv)
        else Empty(emptyTitle, hint = emptyHint)
        return
    }
    val initialIndex = remember(items, focusedUrl) { items.indexOfFirst { it.url == focusedUrl }.coerceAtLeast(0) }
    val gridState = rememberLazyGridState(initialFirstVisibleItemIndex = initialIndex)
    var requestedItemCount by remember(items.first().url) { mutableStateOf(-1) }
    // `loading` is read through rememberUpdatedState so the collector keeps the live value
    // without being torn down and re-armed every time it flips (which previously missed
    // trigger frames and stalled autoload during fast scrolls).
    val isLoading by rememberUpdatedState(loading)
    LaunchedEffect(gridState, items.size, loadMore) {
        val request = loadMore ?: return@LaunchedEffect
        snapshotFlow { gridState.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: -1 }
            .distinctUntilChanged()
            .collect { lastVisibleIndex ->
                if (shouldLoadMore(lastVisibleIndex, items.size, isLoading, requestedItemCount)) {
                    requestedItemCount = items.size
                    request()
                }
            }
    }
    LazyVerticalGrid(
        state = gridState,
        columns = if (isTv) GridCells.Fixed(5) else GridCells.Adaptive(140.dp),
        contentPadding = if (isTv) {
            PaddingValues(horizontal = TV_OVERSCAN_HORIZONTAL, vertical = TV_OVERSCAN_VERTICAL)
        } else {
            PaddingValues(12.dp)
        },
        horizontalArrangement = Arrangement.spacedBy(if (isTv) 16.dp else 12.dp),
        verticalArrangement = Arrangement.spacedBy(if (isTv) 20.dp else 16.dp),
        modifier = if (isTv) {
            Modifier
                .then(entryFocusRequester?.let { Modifier.focusRequester(it) } ?: Modifier)
                .focusRestorer(entryFocusRequester ?: FocusRequester.Default)
                .focusGroup()
        } else {
            Modifier.focusGroup()
        },
    ) {
        items(items, key = { it.url }) { item ->
            MediaCard(item, isTv) { open(item.url) }
        }
    }
}

internal fun shouldLoadMore(lastVisibleIndex: Int, itemCount: Int, loading: Boolean, requestedItemCount: Int) =
    itemCount > 0 && !loading && requestedItemCount != itemCount && lastVisibleIndex >= itemCount - 12

@Composable
internal fun MediaCard(
    item: MediaItem,
    isTv: Boolean,
    focusKey: String = item.url,
    open: () -> Unit,
) {
    val shape = RoundedCornerShape(14.dp)
    val modifier = Modifier
        .fillMaxWidth()
        .then(if (isTv) Modifier.tvFocusMemory(focusKey) else Modifier)
    if (isTv) {
        TvCard(onClick = open, modifier = modifier, scale = CardScale.None) {
            MediaCardContent(item, true, shape)
        }
    } else {
        Card(onClick = open, modifier = modifier, shape = shape) {
            MediaCardContent(item, false, shape)
        }
    }
}

@Composable
private fun MediaCardContent(item: MediaItem, isTv: Boolean, shape: RoundedCornerShape) {
        AsyncImage(
            model = item.posterUrl,
            contentDescription = null,
            modifier = Modifier
                .fillMaxWidth()
                .aspectRatio(2f / 3f)
                .clip(shape)
                .background(MaterialTheme.colorScheme.surfaceVariant),
            contentScale = ContentScale.Crop,
        )
        Column(Modifier.padding(if (isTv) 12.dp else 10.dp)) {
            Text(
                item.title,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
                style = if (isTv) MaterialTheme.typography.titleSmall else MaterialTheme.typography.bodyLarge,
            )
            val metadata = listOfNotNull(item.year?.toString(), item.category).joinToString(" • ")
            if (metadata.isNotEmpty()) {
                Text(
                    metadata,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
}
