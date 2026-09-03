@file:OptIn(
    ExperimentalMaterial3Api::class,
    ExperimentalAnimationApi::class,
    ExperimentalMaterial3WindowSizeClassApi::class,
    ExperimentalComposeUiApi::class,
    ExperimentalTvMaterial3Api::class,
)

package org.movo.app

import android.content.Intent
import androidx.activity.compose.BackHandler
import androidx.compose.animation.ExperimentalAnimationApi
import androidx.compose.foundation.clickable
import androidx.compose.foundation.background
import androidx.compose.foundation.focusGroup
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Comment
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.material3.windowsizeclass.ExperimentalMaterial3WindowSizeClassApi
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.shape.RoundedCornerShape
import coil3.compose.AsyncImage
import kotlinx.coroutines.launch
import androidx.tv.material3.ExperimentalTvMaterial3Api

@Composable
internal fun DetailsScreen(
    state: AppState,
    isTv: Boolean,
    wideContent: Boolean,
    settings: AppSettings,
    model: MovoViewModel,
) {
    val details = state.details ?: return
    val shareLabel = stringResource(R.string.share)
    val scope = rememberCoroutineScope()
    val translators = remember(details.translators, details.voiceRatings, settings.sortVoices) {
        if (!settings.sortVoices) details.translators else details.translators.sortedByDescending { voice ->
            details.voiceRatings.firstOrNull { it.title == voice.name }?.rating ?: 0f
        }
    }
    val playbackDetails = remember(details, translators) {
        if (translators === details.translators) details else details.copy(translators = translators)
    }
    val series = details.mediaType == "TVSeries" || details.seasons.isNotEmpty()
    var translator by remember(details.url) {
        mutableStateOf(preferredTranslator(translators, state.resumeTranslatorId))
    }
    var selectedSeasonId by remember(details.url) {
        mutableStateOf(state.resumeSeasonId ?: details.seasons.firstOrNull()?.id)
    }
    LaunchedEffect(details.seasons, state.episodesTranslatorId, translator?.id) {
        if (state.episodesTranslatorId == translator?.id &&
            (selectedSeasonId == null || details.seasons.none { it.id == selectedSeasonId })
        ) {
            selectedSeasonId = state.resumeSeasonId
                ?.takeIf { translator?.id == state.resumeTranslatorId }
                ?: details.seasons.firstOrNull()?.id
        }
    }
    var descExpanded by remember(details.url) { mutableStateOf(false) }
    var showFavorites by remember(details.url) { mutableStateOf(false) }
    var showPlayback by remember(details.url) { mutableStateOf(false) }
    var automaticPlayback by remember(details.url) { mutableStateOf(false) }
    var showRating by remember(details.url) { mutableStateOf(false) }
    val context = LocalContext.current
    LaunchedEffect(state.detailAction) {
        when (state.detailAction) {
            DetailAction.Favorites -> showFavorites = true
            DetailAction.Comments -> model.loadComments()
            DetailAction.Rating -> showRating = true
            null -> return@LaunchedEffect
        }
        model.consumeDetailAction()
    }
    var selectedQuality by remember(state.preparedStream, settings.qualityMode, settings.lastQuality) {
        mutableStateOf(state.preparedStream?.let { selectStream(it, settings.qualityMode, settings.lastQuality.takeIf { _ -> settings.saveQuality }) })
    }
    val selectedSeason = details.seasons.firstOrNull { it.id == selectedSeasonId }
        ?: details.seasons.firstOrNull()
    val previewEpisode = selectedSeason?.episodes?.firstOrNull {
        it.id == state.resumeEpisodeId &&
            translator?.id == state.resumeTranslatorId &&
            selectedSeason.id == state.resumeSeasonId
    } ?: selectedSeason?.episodes?.firstOrNull()
    LaunchedEffect(showPlayback) {
        if (showPlayback && state.episodesTranslatorId != translator?.id) {
            translator?.let(model::loadEpisodes)
        }
    }
    LaunchedEffect(
        showPlayback,
        translator?.id,
        state.episodesTranslatorId,
        selectedSeason?.id,
        previewEpisode?.id,
    ) {
        val voice = translator ?: return@LaunchedEffect
        if (!showPlayback) return@LaunchedEffect
        if (!series) {
            model.preparePlayback(voice)
        } else if (state.episodesTranslatorId == voice.id && selectedSeason != null && previewEpisode != null) {
            model.preparePlayback(voice, selectedSeason.id, previewEpisode.id)
        }
    }
    LaunchedEffect(automaticPlayback, state.preparedStream, selectedSeason?.id, previewEpisode?.id) {
        if (!automaticPlayback) return@LaunchedEffect
        val voice = translator ?: return@LaunchedEffect
        val prepared = state.preparedStream ?: return@LaunchedEffect
        val quality = selectStream(prepared, settings.qualityMode, settings.lastQuality.takeIf { settings.saveQuality }) ?: return@LaunchedEffect
        val season = if (series) selectedSeason?.id ?: return@LaunchedEffect else null
        val episode = if (series) previewEpisode?.id ?: return@LaunchedEffect else null
        automaticPlayback = false
        showPlayback = false
        if (settings.saveQuality) scope.launch { context.save(Keys.LAST_QUALITY, quality.quality) }
        model.startPlayback(voice, quality.quality, settings.qualityMode, season, episode)
    }
    BackHandler { model.closeDetails() }
    Scaffold(
        topBar = {
            CenterAlignedTopAppBar(
                title = { Text(details.title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                navigationIcon = {
                    IconButton(model::closeDetails, Modifier.tvFocusScale(isTv)) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = stringResource(R.string.back),
                        )
                    }
                },
            )
        },
    ) { padding ->
        Box(Modifier.padding(padding).fillMaxSize()) {
            key(details.url) {
                LazyColumn(
                    Modifier
                        .widthIn(max = 1200.dp)
                        .fillMaxSize()
                        .align(Alignment.TopCenter)
                        .focusGroup(),
                    contentPadding = PaddingValues(if (isTv) 32.dp else if (wideContent) 24.dp else 16.dp),
                    verticalArrangement = Arrangement.spacedBy(16.dp),
                ) {
            item {
                Row(horizontalArrangement = Arrangement.spacedBy(18.dp)) {
                    Box {
                        AsyncImage(
                            model = details.posterHqUrl ?: details.posterUrl,
                            contentDescription = null,
                            modifier = Modifier
                                .width(if (isTv) 260.dp else if (wideContent) 200.dp else 120.dp)
                                .aspectRatio(2f / 3f)
                                .clip(RoundedCornerShape(if (isTv) 16.dp else 12.dp))
                                .background(MaterialTheme.colorScheme.surfaceVariant),
                            contentScale = ContentScale.Crop,
                        )
                        if (details.trailerAvailable) {
                            // Small affordance: trailer is also reachable from the action row,
                            // the badge just makes it discoverable on the poster.
                            FilledTonalIconButton(
                                onClick = model::loadTrailer,
                                modifier = Modifier
                                    .align(Alignment.BottomEnd)
                                    .padding(6.dp)
                                    .size(32.dp)
                                    .tvFocusScale(isTv),
                            ) {
                                Icon(
                                    Icons.Default.Movie,
                                    contentDescription = stringResource(R.string.trailer),
                                    modifier = Modifier.size(18.dp),
                                )
                            }
                        }
                    }
                    Column(
                        Modifier.weight(1f),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        Text(
                            details.title,
                            style = if (wideContent || isTv) {
                                MaterialTheme.typography.headlineMedium
                            } else {
                                MaterialTheme.typography.titleLarge
                            },
                            maxLines = 3,
                            overflow = TextOverflow.Ellipsis,
                        )
                        details.originalTitle?.let {
                            Text(
                                it,
                                maxLines = 2,
                                overflow = TextOverflow.Ellipsis,
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        val metadata = listOfNotNull(
                            details.year?.toString(),
                            details.duration,
                            details.genres.joinToString().ifBlank { null },
                        ).joinToString(" • ")
                        if (metadata.isNotEmpty()) {
                            Text(
                                metadata,
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        RatingRow(details)
                        FlowRow(
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            FilledTonalIconButton(onClick = { showFavorites = true }, modifier = Modifier.tvFocusScale(isTv)) {
                                Icon(
                                    if (details.favoriteCategoryIds.isEmpty()) Icons.Default.FavoriteBorder else Icons.Default.Favorite,
                                    contentDescription = stringResource(R.string.favorite),
                                )
                            }
                            Button(
                                onClick = { automaticPlayback = !settings.askQuality; showPlayback = true },
                                enabled = translators.isNotEmpty() && !state.loading,
                                modifier = Modifier.tvFocusScale(isTv),
                            ) {
                                if (state.loading) {
                                    CircularProgressIndicator(
                                        Modifier.size(18.dp),
                                        strokeWidth = 2.dp,
                                        color = LocalContentColor.current,
                                    )
                                } else {
                                    Icon(Icons.Default.PlayArrow, contentDescription = null)
                                }
                                Text(stringResource(R.string.play))
                            }
                            if (details.trailerAvailable && (wideContent || isTv)) {
                                FilledTonalIconButton(model::loadTrailer, Modifier.tvFocusScale(isTv)) { Icon(Icons.Default.Movie, stringResource(R.string.trailer)) }
                            }
                            FilledTonalIconButton(onClick = { showRating = true }, modifier = Modifier.tvFocusScale(isTv), enabled = !details.ratingPosted) {
                                Icon(Icons.Default.StarRate, stringResource(R.string.rate_title))
                            }
                            FilledTonalIconButton(onClick = { model.loadComments() }, modifier = Modifier.tvFocusScale(isTv)) {
                                Icon(Icons.AutoMirrored.Filled.Comment, stringResource(R.string.comments))
                            }
                            FilledTonalIconButton(onClick = {
                                context.startActivity(Intent.createChooser(Intent(Intent.ACTION_SEND).apply {
                                    type = "text/plain"
                                    putExtra(Intent.EXTRA_TEXT, details.url)
                                }, shareLabel))
                            }, modifier = Modifier.tvFocusScale(isTv)) { Icon(Icons.Default.Share, stringResource(R.string.share)) }
                        }
                    }
                }
            }
            item {
                Text(
                    text = details.description,
                    maxLines = if (descExpanded) Int.MAX_VALUE else 8,
                    overflow = TextOverflow.Ellipsis,
                )
                if (details.description.length > 400) {
                    TextButton(onClick = { descExpanded = !descExpanded }, modifier = Modifier.tvFocusScale(isTv)) {
                        Text(if (descExpanded) stringResource(R.string.collapse) else stringResource(R.string.more))
                    }
                }
            }
            if (details.genreLinks.isNotEmpty() || details.countryLinks.isNotEmpty()) {
                item {
                    LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        items(details.genreLinks + details.countryLinks, key = { "${it.url}:${it.name}" }) { link ->
                            AssistChip(onClick = { model.openDiscoveryPath(Tab.Search, link.name, link.url) }, label = { Text(link.name) }, modifier = Modifier.tvFocusScale(isTv))
                        }
                    }
                }
            }
            if (details.franchises.size > 1) {
                item {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(
                            stringResource(R.string.franchise),
                            style = if (isTv) {
                                MaterialTheme.typography.titleLarge
                            } else {
                                MaterialTheme.typography.titleMedium
                            },
                        )
                        LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            items(
                                details.franchises,
                                key = { "${it.url}:${it.title}" },
                            ) { part ->
                                MovoChoiceChip(
                                    selected = part.current,
                                    onClick = {
                                        if (!part.current && part.url.isNotBlank()) {
                                            model.openDetails(part.url)
                                        }
                                    },
                                    enabled = part.current || part.url.isNotBlank(),
                                    label = {
                                        Text(
                                            part.title,
                                            maxLines = 1,
                                            overflow = TextOverflow.Ellipsis,
                                            style = if (isTv) {
                                                MaterialTheme.typography.titleSmall
                                            } else {
                                                MaterialTheme.typography.labelLarge
                                            },
                                        )
                                    },
                                    trailingIcon = if (!part.current) {
                                        { Icon(Icons.Default.ChevronRight, contentDescription = null) }
                                    } else null,
                                    modifier = Modifier
                                        .widthIn(min = if (isTv) 200.dp else 150.dp, max = 300.dp)
                                        .height(if (isTv) 52.dp else 40.dp),
                                    isTv = isTv,
                                )
                            }
                        }
                    }
                }
            }
            if (details.actorDetails.isNotEmpty() || details.directorDetails.isNotEmpty()) {
                item {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(stringResource(R.string.people), style = MaterialTheme.typography.titleLarge)
                        LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            items(details.directorDetails + details.actorDetails, key = { "${it.url}:${it.name}" }) { person ->
                                AssistChip(
                                    onClick = { if (person.url.isNotBlank()) model.openActor(person.url) },
                                    label = { Text(person.name) },
                                    modifier = Modifier.tvFocusScale(isTv),
                                    leadingIcon = { Icon(Icons.Default.Person, null) },
                                    enabled = person.url.isNotBlank(),
                                )
                            }
                        }
                    }
                }
            }
            if (details.schedules.isNotEmpty()) {
                item {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(stringResource(R.string.schedule), style = MaterialTheme.typography.titleLarge)
                        details.schedules.forEach { group ->
                            if (group.name.isNotBlank()) Text(group.name, style = MaterialTheme.typography.titleMedium)
                            group.items.forEach { episode ->
                                ListItem(
                                    headlineContent = { Text("${episode.episode}  ${episode.title}") },
                                    supportingContent = { Text(listOfNotNull(episode.originalTitle, episode.date).joinToString(" • ")) },
                                    trailingContent = {
                                        IconButton({ model.toggleSchedule(episode) }, Modifier.tvFocusScale(isTv), enabled = episode.id.isNotBlank()) {
                                            Icon(if (episode.watched) Icons.Default.CheckCircle else Icons.Default.RadioButtonUnchecked, stringResource(if (episode.watched) R.string.mark_unwatched else R.string.mark_watched))
                                        }
                                    },
                                    colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                                )
                            }
                        }
                    }
                }
            }
            if (details.includedIn.isNotEmpty() || details.fromCollections.isNotEmpty()) {
                item {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(stringResource(R.string.from_collections), style = MaterialTheme.typography.titleLarge)
                        LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            items(details.includedIn + details.fromCollections, key = { "${it.url}:${it.name}" }) { link ->
                                AssistChip(onClick = { model.openDiscoveryPath(Tab.Collections, link.name, link.url) }, label = { Text(link.name) }, modifier = Modifier.tvFocusScale(isTv))
                            }
                        }
                    }
                }
            }
            if (details.related.isNotEmpty()) {
                item {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(stringResource(R.string.related), style = MaterialTheme.typography.titleLarge)
                        LazyRow(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                            items(details.related, key = { it.url }) { item ->
                                Box(Modifier.width(if (isTv) 180.dp else 150.dp)) { MediaCard(item, isTv) { model.openDetails(item.url) } }
                            }
                        }
                    }
                }
            }
            state.error?.let { item { ErrorBanner(it, model::clearError) } }
                }
            }
        }
    }

    if (showFavorites) {
        FavoriteFoldersSheet(
            isTv = isTv,
            groups = state.favoriteGroups,
            selectedIds = details.favoriteCategoryIds,
            dismiss = { showFavorites = false },
            toggle = model::toggleFavorite,
        )
    }
    if (showPlayback) {
        PlaybackSheet(
            isTv = isTv,
            details = playbackDetails,
            series = series,
            translator = translator,
            selectedSeasonId = selectedSeasonId,
            preparedStream = state.preparedStream,
            selectedQuality = selectedQuality,
            loading = state.loading,
            episodesLoading = series && state.episodesTranslatorId != translator?.id,
            error = state.error,
            dismiss = {
                automaticPlayback = false
                showPlayback = false
                model.cancelPlaybackPreparation()
            },
            chooseTranslator = {
                model.cancelPlaybackPreparation()
                translator = it
                selectedSeasonId = null
                if (series) model.loadEpisodes(it)
            },
            chooseSeason = {
                model.cancelPlaybackPreparation()
                selectedSeasonId = it
            },
            chooseQuality = { selectedQuality = it },
            play = { season, episode ->
                automaticPlayback = false
                val voice = translator
                val quality = selectedQuality
                if (voice != null && quality != null) {
                    if (settings.saveQuality) scope.launch { context.save(Keys.LAST_QUALITY, quality.quality) }
                    model.startPlayback(voice, quality.quality, settings.qualityMode, season, episode)
                }
            },
        )
    }
    if (showRating) {
        AlertDialog(
            onDismissRequest = { showRating = false },
            title = { Text(stringResource(R.string.rate_title)) },
            text = {
                LazyRow(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    items((1..10).toList()) { rating ->
                        FilledTonalButton(
                            onClick = { showRating = false; model.rate(rating) },
                            modifier = Modifier.tvFocusScale(isTv),
                            contentPadding = PaddingValues(horizontal = 12.dp),
                        ) { Text(rating.toString()) }
                    }
                }
            },
            confirmButton = {},
            dismissButton = { TextButton({ showRating = false }, Modifier.tvFocusScale(isTv)) { Text(stringResource(R.string.cancel)) } },
        )
    }
    state.actor?.let { ActorDialog(it, isTv, model) }
    state.comments?.let { CommentsDialog(it, isTv, model) }
}

@Composable
private fun ActorDialog(actor: ActorDetails, isTv: Boolean, model: MovoViewModel) {
    AdaptiveModal(isTv, model::closeActor) {
        LazyColumn(Modifier.fillMaxHeight(.9f), contentPadding = PaddingValues(24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            item {
                Row(horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                    AsyncImage(actor.photoUrl, null, Modifier.width(150.dp).aspectRatio(2f / 3f), contentScale = ContentScale.Crop)
                    Column {
                        Text(actor.name, style = MaterialTheme.typography.headlineSmall)
                        actor.originalName?.let { Text(it) }
                        actor.birthDate?.let { Text(stringResource(R.string.born, it)) }
                        actor.birthPlace?.let { Text(it) }
                        actor.height?.let { Text(it) }
                        if (actor.careers.isNotEmpty()) Text(actor.careers.joinToString())
                    }
                }
            }
            if (actor.roles.isEmpty()) {
                items(actor.films, key = { it.url }) { film -> ActorFilmRow(film, isTv, model) }
            } else actor.roles.forEach { role ->
                item("role:${role.name}") { Column { Text(role.name, style = MaterialTheme.typography.titleLarge); if (role.info.isNotBlank()) Text(role.info) } }
                items(role.films, key = { "${role.name}:${it.url}" }) { film -> ActorFilmRow(film, isTv, model) }
            }
        }
    }
}

@Composable
private fun ActorFilmRow(film: MediaItem, isTv: Boolean, model: MovoViewModel) {
    ListItem(
        headlineContent = { Text(film.title) },
        supportingContent = { Text(listOfNotNull(film.year?.toString(), film.info).joinToString(" • ")) },
        modifier = Modifier.tvFocusScale(isTv, 1.03f).clickable { model.closeActor(); model.openDetails(film.url) },
    )
}

@Composable
private fun CommentsDialog(page: CommentsPage, isTv: Boolean, model: MovoViewModel) {
    var revealedSpoilers by remember(page.page) { mutableStateOf(emptySet<String>()) }
    AdaptiveModal(isTv, model::closeComments) {
        LazyColumn(Modifier.fillMaxHeight(.9f), contentPadding = PaddingValues(24.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            item { Text(stringResource(R.string.comments), style = MaterialTheme.typography.headlineSmall) }
            items(page.items, key = { it.id }) { comment ->
                ListItem(
                    headlineContent = { Text(comment.username) },
                    supportingContent = { Column {
                        if (comment.hasSpoiler && comment.id !in revealedSpoilers) {
                            TextButton({ revealedSpoilers = revealedSpoilers + comment.id }, Modifier.tvFocusScale(isTv)) { Icon(Icons.Default.Visibility, null); Text(stringResource(R.string.show_spoiler)) }
                        } else Text(comment.text)
                        Text(comment.date, style = MaterialTheme.typography.labelSmall)
                    } },
                    leadingContent = { AsyncImage(comment.avatarUrl, null, Modifier.size(48.dp).clip(RoundedCornerShape(24.dp)), contentScale = ContentScale.Crop) },
                    trailingContent = { TextButton({ model.likeComment(comment.id) }, Modifier.tvFocusScale(isTv), colors = ButtonDefaults.textButtonColors(contentColor = if (comment.liked) MaterialTheme.colorScheme.primary else LocalContentColor.current)) { Icon(Icons.Default.ThumbUp, null); Text(comment.likes.toString()) } },
                    modifier = Modifier.padding(start = (comment.indent.coerceAtMost(4) * 12).dp),
                )
            }
            if (page.totalPages > 1) {
                item {
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedButton({ model.loadComments(page.page - 1) }, Modifier.tvFocusScale(isTv), enabled = page.page > 1) { Text(stringResource(R.string.previous)) }
                        OutlinedButton({ model.loadComments(page.page + 1) }, Modifier.tvFocusScale(isTv), enabled = page.page < page.totalPages) { Text(stringResource(R.string.next)) }
                    }
                }
            }
        }
    }
}

internal fun preferredTranslator(translators: List<Translator>, translatorId: Long?) =
    translators.firstOrNull { it.id == translatorId } ?: translators.firstOrNull()

@Composable
private fun RatingRow(details: MediaDetails) {
    val ratings = (listOfNotNull(
        details.ratingRezka?.let { stringResource(R.string.rating_rezka, it) },
        details.ratingImdb?.let { stringResource(R.string.rating_imdb, it) },
        details.ratingKp?.let { stringResource(R.string.rating_kp, it) },
    ) + details.ratings.map { rating ->
        buildString {
            append(rating.name)
            append(' ')
            append(rating.value)
            rating.votes?.let { append(" ($it)") }
        }
    }).distinct()
    if (ratings.isNotEmpty()) Text(ratings.joinToString(" • "), style = MaterialTheme.typography.labelLarge)
}

@Composable
private fun FavoriteFoldersSheet(
    isTv: Boolean,
    groups: List<FavoriteGroup>,
    selectedIds: List<Long>,
    dismiss: () -> Unit,
    toggle: (Long, Boolean) -> Unit,
) {
    AdaptiveModal(isTv, dismiss) {
        LazyColumn {
            item {
                Text(
                    stringResource(R.string.favorite_folders),
                    style = MaterialTheme.typography.headlineSmall,
                    modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
                )
            }
            items(groups, key = { it.id ?: it.name }) { group ->
                group.id?.let { id ->
                    val selected = id in selectedIds
                    ListItem(
                        headlineContent = { Text(group.name) },
                        trailingContent = { Checkbox(selected, null) },
                        modifier = Modifier
                            .tvFocusScale(isTv, 1.02f)
                            .toggleable(
                                value = selected,
                                role = Role.Checkbox,
                                onValueChange = { toggle(id, it) },
                            ),
                    )
                }
            }
            item { Spacer(Modifier.height(24.dp)) }
        }
    }
}

@Composable
private fun PlaybackSheet(
    isTv: Boolean,
    details: MediaDetails,
    series: Boolean,
    translator: Translator?,
    selectedSeasonId: Long?,
    preparedStream: StreamBundle?,
    selectedQuality: StreamEntry?,
    loading: Boolean,
    episodesLoading: Boolean,
    error: String?,
    dismiss: () -> Unit,
    chooseTranslator: (Translator) -> Unit,
    chooseSeason: (Long) -> Unit,
    chooseQuality: (StreamEntry) -> Unit,
    play: (Long?, Long?) -> Unit,
) {
    val season = details.seasons.firstOrNull { it.id == selectedSeasonId } ?: details.seasons.firstOrNull()
    val qualities = preparedStream?.streams?.filter { it.urls.isNotEmpty() }.orEmpty()
    var startingEpisodeId by remember(translator?.id, season?.id) { mutableStateOf<Long?>(null) }
    LaunchedEffect(loading) {
        if (!loading) startingEpisodeId = null
    }
    AdaptiveModal(isTv, dismiss) {
        Column(
            modifier = Modifier.fillMaxHeight(0.85f),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text(
                stringResource(R.string.choose_playback),
                style = MaterialTheme.typography.headlineSmall,
                modifier = Modifier.padding(horizontal = 24.dp),
            )
            error?.let {
                Text(
                    it,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(horizontal = 24.dp),
                )
            }
            ChoiceRow(
                title = stringResource(R.string.translation),
                values = details.translators,
                selected = translator,
                label = { it.name },
                itemKey = { it.id },
                choose = chooseTranslator,
                modifier = Modifier.padding(horizontal = 24.dp),
                isTv = isTv,
            )
            if (qualities.isNotEmpty()) {
                ChoiceRow(
                    title = stringResource(R.string.quality),
                    values = qualities,
                    selected = selectedQuality,
                    label = { it.quality },
                    itemKey = { it.quality },
                    choose = chooseQuality,
                    modifier = Modifier.padding(horizontal = 24.dp),
                    isTv = isTv,
                )
            } else {
                Column(Modifier.padding(horizontal = 24.dp)) {
                    Text(stringResource(R.string.quality), style = MaterialTheme.typography.titleMedium)
                    if (loading) {
                        CircularProgressIndicator(Modifier.size(32.dp), strokeWidth = 3.dp)
                    } else {
                        Text(stringResource(R.string.no_streams))
                    }
                }
            }
            if (!series) {
                Button(
                    onClick = { play(null, null) },
                    enabled = translator != null && selectedQuality != null && !loading,
                    modifier = Modifier.padding(horizontal = 24.dp).tvFocusScale(isTv),
                ) {
                    if (loading) {
                        CircularProgressIndicator(Modifier.size(18.dp), strokeWidth = 2.dp)
                    } else {
                        Icon(Icons.Default.PlayArrow, contentDescription = null)
                    }
                    Text(stringResource(R.string.start_watching))
                }
            } else if (episodesLoading) {
                Box(Modifier.fillMaxWidth().weight(1f), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator()
                }
            } else if (season != null) {
                ChoiceRow(
                    title = stringResource(R.string.season),
                    values = details.seasons,
                    selected = season,
                    label = { it.title },
                    itemKey = { it.id },
                    choose = { chooseSeason(it.id) },
                    modifier = Modifier.padding(horizontal = 24.dp),
                    isTv = isTv,
                )
                HorizontalDivider()
                Text(
                    stringResource(R.string.episode),
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(horizontal = 24.dp),
                )
                key(season.id) {
                    LazyColumn(
                        modifier = Modifier.weight(1f),
                        contentPadding = PaddingValues(bottom = 24.dp),
                    ) {
                        items(season.episodes, key = { "${season.id}:${it.id}" }) { episode ->
                            val contentAlpha = if (episode.watched) .6f else 1f
                            ListItem(
                                headlineContent = { Text(episode.title, modifier = Modifier.alpha(contentAlpha)) },
                                supportingContent = episode.watchId?.let { id -> { Text(id, Modifier.alpha(contentAlpha), maxLines = 1) } },
                                trailingContent = {
                                    FilledTonalIconButton(
                                        onClick = {
                                            startingEpisodeId = episode.id
                                            play(season.id, episode.id)
                                        },
                                        modifier = Modifier.tvFocusScale(isTv),
                                        enabled = selectedQuality != null && !loading,
                                    ) {
                                        if (loading && startingEpisodeId == episode.id) {
                                            CircularProgressIndicator(Modifier.size(18.dp), strokeWidth = 2.dp)
                                        } else {
                                            Icon(
                                                Icons.Default.PlayArrow,
                                                contentDescription = stringResource(
                                                    R.string.play_episode,
                                                    episode.title,
                                                ),
                                            )
                                        }
                                    }
                                },
                            )
                        }
                    }
                }
            }
        }
    }
}
