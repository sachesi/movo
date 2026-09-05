@file:OptIn(
    ExperimentalMaterial3Api::class,
    ExperimentalAnimationApi::class,
    ExperimentalMaterial3WindowSizeClassApi::class,
    ExperimentalComposeUiApi::class,
    ExperimentalTvMaterial3Api::class,
)

package org.movo.app.ui

import org.movo.app.R
import androidx.compose.animation.ExperimentalAnimationApi
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
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
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.material3.windowsizeclass.ExperimentalMaterial3WindowSizeClassApi
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.tv.material3.ExperimentalTvMaterial3Api
import androidx.tv.material3.Text as TvText

@Composable
internal fun Loading(label: String) = Box(
    Modifier.fillMaxSize(),
    contentAlignment = Alignment.Center,
) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        CircularProgressIndicator()
        Spacer(Modifier.height(12.dp))
        Text(label)
    }
}

@Composable
internal fun TvHomeSkeleton() {
    LazyColumn(
        modifier = Modifier.fillMaxSize().testTag("tv-home-placeholder-list"),
        contentPadding = PaddingValues(horizontal = TV_OVERSCAN_HORIZONTAL, vertical = TV_OVERSCAN_VERTICAL),
        verticalArrangement = Arrangement.spacedBy(20.dp),
    ) {
        items(5) { rail ->
            Column(
                Modifier.testTag("tv-home-placeholder-rail-$rail"),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Box(
                    Modifier
                        .width(180.dp)
                        .height(24.dp)
                        .clip(RoundedCornerShape(8.dp))
                        .background(MaterialTheme.colorScheme.surfaceVariant),
                )
                LazyRow(
                    horizontalArrangement = Arrangement.spacedBy(16.dp),
                    userScrollEnabled = false,
                    modifier = Modifier.testTag("tv-home-placeholder-row-$rail"),
                ) {
                    items(7) { card ->
                        Column(
                            Modifier.width(176.dp).testTag("tv-home-placeholder-card-$rail-$card"),
                        ) {
                            Box(
                                Modifier
                                    .fillMaxWidth()
                                    .aspectRatio(2f / 3f)
                                    .clip(MaterialTheme.shapes.medium)
                                    .background(MaterialTheme.colorScheme.surfaceVariant),
                            )
                            Spacer(Modifier.height(8.dp))
                            Box(
                                Modifier
                                    .fillMaxWidth(.8f)
                                    .height(14.dp)
                                    .clip(RoundedCornerShape(7.dp))
                                    .background(MaterialTheme.colorScheme.surfaceVariant),
                            )
                        }
                    }
                }
            }
        }
    }
}

/** Shimmering poster placeholders that mirror the real grid geometry. */
@Composable
internal fun MediaGridSkeleton(isTv: Boolean) {
    // A television draws the placeholders still: the pulse redraws the whole grid every frame
    // for as long as the load runs, on hardware that has no frames to spare.
    val alpha: () -> Float = if (isTv) {
        { 0.65f }
    } else {
        val transition = rememberInfiniteTransition(label = "skeleton")
        val pulse = transition.animateFloat(
            initialValue = 0.4f,
            targetValue = 0.9f,
            animationSpec = infiniteRepeatable(
                animation = tween(700, easing = LinearEasing),
                repeatMode = RepeatMode.Reverse,
            ),
            label = "skeleton alpha",
        )
        val read = { pulse.value }
        read
    }
    LazyVerticalGrid(
        // Same geometry as the grid it stands in for, so the content does not jump columns
        // the moment it arrives.
        columns = GridCells.Adaptive(140.dp),
        contentPadding = if (isTv) {
            PaddingValues(horizontal = TV_OVERSCAN_HORIZONTAL, vertical = TV_OVERSCAN_VERTICAL)
        } else {
            PaddingValues(12.dp)
        },
        horizontalArrangement = Arrangement.spacedBy(if (isTv) 16.dp else 12.dp),
        verticalArrangement = Arrangement.spacedBy(if (isTv) 20.dp else 16.dp),
        modifier = Modifier.fillMaxSize(),
        userScrollEnabled = false,
    ) {
        items(12) {
            Column {
                SkeletonBlock(alpha, Modifier.fillMaxWidth().aspectRatio(2f / 3f), 14.dp)
                Spacer(Modifier.height(8.dp))
                SkeletonBlock(alpha, Modifier.fillMaxWidth(0.8f).height(14.dp), 7.dp)
                Spacer(Modifier.height(4.dp))
                SkeletonBlock(alpha, Modifier.fillMaxWidth(0.5f).height(11.dp), 6.dp)
            }
        }
    }
}

/**
 * One shimmering placeholder bar. The pulse is passed as a lambda and applied in the draw phase,
 * so the animation never invalidates composition — it would otherwise recompose every placeholder
 * on every frame, for the whole time a load is in flight.
 */
@Composable
private fun SkeletonBlock(alpha: () -> Float, modifier: Modifier, corner: Dp) {
    val color = MaterialTheme.colorScheme.surfaceVariant
    val shape = RoundedCornerShape(corner)
    Box(
        modifier
            .clip(shape)
            .drawBehind { drawRect(color.copy(alpha = alpha())) },
    )
}

/**
 * Over the page while a title the user chose is on its way, so the press is seen to land: the
 * thin bar at the top of the page read as nothing happening, and a second press restarted the load.
 * Taps stop here; the remote still reaches the card, which the model ignores while it loads.
 */
@Composable
internal fun OpeningOverlay() = Box(
    Modifier
        .fillMaxSize()
        .background(MaterialTheme.colorScheme.scrim.copy(alpha = 0.4f))
        .pointerInput(Unit) { detectTapGestures { } },
    contentAlignment = Alignment.Center,
) { CircularProgressIndicator() }

/** Before a search runs the grid invites one; afterwards it names the query that found nothing. */
@Composable
internal fun searchEmptyTitle(query: String) =
    if (query.isBlank()) stringResource(R.string.search_start)
    else stringResource(R.string.search_no_results, query)

internal fun searchEmptyIcon(query: String) =
    if (query.isBlank()) Icons.Default.Search else Icons.Default.SearchOff

@Composable
internal fun searchEmptyHint(query: String) =
    if (query.isBlank()) stringResource(R.string.search_start_hint)
    else stringResource(R.string.search_no_results_hint)

@Composable
internal fun Empty(label: String, hint: String? = null, icon: ImageVector = Icons.Default.Inbox) = Box(
    Modifier.fillMaxSize(),
    contentAlignment = Alignment.Center,
) {
    Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.padding(32.dp)) {
        Icon(
            icon,
            contentDescription = null,
            modifier = Modifier.size(56.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f),
        )
        Spacer(Modifier.height(12.dp))
        Text(
            label,
            style = MaterialTheme.typography.titleLarge,
            textAlign = TextAlign.Center,
        )
        if (hint != null) {
            Spacer(Modifier.height(4.dp))
            Text(
                hint,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
        }
    }
}

@Composable
internal fun ErrorBanner(
    message: String,
    dismiss: () -> Unit,
    modifier: Modifier = Modifier,
    retry: (() -> Unit)? = null,
    isTv: Boolean = false,
) {
    // The banner floats over a focus group it is not part of, so on a television nothing would
    // carry the highlight to it; it claims focus on arrival instead.
    Snackbar(
        modifier = modifier
            .padding(
                horizontal = if (isTv) TV_OVERSCAN_HORIZONTAL else 12.dp,
                vertical = if (isTv) TV_OVERSCAN_VERTICAL else 12.dp,
            )
            // Read out on arrival; a snackbar host would do this, and the banner has none.
            .semantics { liveRegion = LiveRegionMode.Polite },
        containerColor = MaterialTheme.colorScheme.errorContainer,
        contentColor = MaterialTheme.colorScheme.onErrorContainer,
        action = {
            Row(horizontalArrangement = Arrangement.spacedBy(if (isTv) 8.dp else 4.dp)) {
                val retryLabel = stringResource(R.string.retry)
                val dismissLabel = stringResource(R.string.dismiss)
                if (isTv) {
                    if (retry != null) {
                        TvButton({ dismiss(); retry() }, Modifier.tvInitialFocus()) { TvText(retryLabel) }
                    }
                    TvButton(dismiss, Modifier.tvInitialFocus(retry == null)) { TvText(dismissLabel) }
                } else {
                    if (retry != null) {
                        TextButton({ dismiss(); retry() }) { Text(retryLabel) }
                    }
                    TextButton(dismiss) { Text(dismissLabel) }
                }
            }
        },
    ) { Text(message) }
}
