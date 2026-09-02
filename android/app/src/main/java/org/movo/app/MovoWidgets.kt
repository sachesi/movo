@file:OptIn(
    ExperimentalMaterial3Api::class,
    ExperimentalAnimationApi::class,
    ExperimentalMaterial3WindowSizeClassApi::class,
    ExperimentalComposeUiApi::class,
    ExperimentalTvMaterial3Api::class,
)

package org.movo.app

import androidx.compose.animation.ExperimentalAnimationApi
import androidx.compose.animation.core.LinearOutSlowInEasing
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsFocusedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.material3.windowsizeclass.ExperimentalMaterial3WindowSizeClassApi
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.Stable
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.mapSaver
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.onPlaced
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.tv.material3.ExperimentalTvMaterial3Api

internal val MovoBlue = Color(0xFF9CCAFF)

/**
 * Per-destination record of the last focused element on the TV surface, so returning to a
 * destination puts the highlight back where it was.
 *
 * Deliberately *not* snapshot state: every D-pad move records a new key, and observable state
 * here would invalidate whichever composition provides it — the whole destination — on every
 * single focus change. The entries are write-mostly and read only when a node first composes,
 * so a plain map behind a stable holder is both correct and free.
 */
@Stable
internal class TvFocusMemory(entries: Map<String, String> = emptyMap()) {
    private val entries = HashMap(entries)

    /** Destination currently on screen. Assigned by the TV shell before its content composes. */
    var destination: String = ""

    /** Key to fall back to when the destination has no recorded focus yet. */
    var fallback: String? = null

    fun record(key: String) {
        if (destination.isNotEmpty()) entries[destination] = key
    }

    fun restoreKey(): String? = entries[destination] ?: fallback

    companion object {
        val Saver: Saver<TvFocusMemory, Any> = mapSaver(
            save = { HashMap<String, Any?>(it.entries) },
            restore = { saved -> TvFocusMemory(saved.mapValues { (_, value) -> value as String }) },
        )
    }
}

internal val LocalTvFocusMemory = staticCompositionLocalOf<TvFocusMemory?> { null }

/** Records focus gains for [key] and, on the destination's first composition, restores it. */
@Composable
internal fun Modifier.tvFocusMemory(key: String): Modifier {
    val memory = LocalTvFocusMemory.current ?: return this
    val focusRequester = remember { FocusRequester() }
    val restoreOnEntry = remember { memory.restoreKey() == key }
    val tracked = this
        .focusRequester(focusRequester)
        .onFocusChanged { if (it.isFocused) memory.record(key) }
    if (!restoreOnEntry) return tracked
    // Requested from onPlaced rather than a launched effect: a focus request against a node that
    // has not been placed yet is dropped, which is how restoration silently lands on the wrong
    // element. The guard keeps it to the first placement so scrolling cannot steal focus back.
    val restored = remember { RestoreOnce() }
    return tracked.onPlaced {
        if (!restored.done) {
            restored.done = true
            runCatching { focusRequester.requestFocus() }
        }
    }
}

@Composable
internal fun AdaptiveModal(
    isTv: Boolean,
    dismiss: () -> Unit,
    content: @Composable () -> Unit,
) {
    if (isTv) {
        BasicAlertDialog(onDismissRequest = dismiss) {
            Surface(
                modifier = Modifier
                    .widthIn(max = 760.dp)
                    .fillMaxHeight(0.88f),
                shape = RoundedCornerShape(24.dp),
                tonalElevation = 6.dp,
            ) { content() }
        }
    } else {
        ModalBottomSheet(
            onDismissRequest = dismiss,
            sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true),
        ) { content() }
    }
}

@Composable
fun <T> ChoiceRow(
    title: String,
    values: List<T>,
    selected: T?,
    label: (T) -> String,
    itemKey: (T) -> Any,
    choose: (T) -> Unit,
    modifier: Modifier = Modifier,
    isTv: Boolean = false,
) {
    Column(modifier) {
        Text(title, style = MaterialTheme.typography.titleMedium)
        LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            items(values, key = itemKey) { value ->
                MovoChoiceChip(
                    selected = value == selected,
                    onClick = { choose(value) },
                    label = { Text(label(value)) },
                    isTv = isTv,
                )
            }
        }
    }
}

@Composable
internal fun MovoChoiceChip(
    selected: Boolean,
    onClick: () -> Unit,
    label: @Composable () -> Unit,
    isTv: Boolean,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    trailingIcon: (@Composable () -> Unit)? = null,
) {
    val interactionSource = remember { MutableInteractionSource() }
    val focused by interactionSource.collectIsFocusedAsState()
    val tvFocused = isTv && focused
    val scale by animateFloatAsState(if (tvFocused) 1.06f else 1f, label = "choice focus")
    val focusContainer = MaterialTheme.colorScheme.primary
    val focusContent = MaterialTheme.colorScheme.onPrimary
    FilterChip(
        selected = selected,
        onClick = onClick,
        label = label,
        modifier = modifier
            .zIndex(if (tvFocused) 1f else 0f)
            .graphicsLayer { scaleX = scale; scaleY = scale },
        enabled = enabled,
        leadingIcon = {
            Icon(
                Icons.Default.Check,
                contentDescription = null,
                modifier = Modifier.alpha(if (selected) 1f else 0f),
            )
        },
        trailingIcon = trailingIcon,
        colors = FilterChipDefaults.filterChipColors(
            containerColor = if (tvFocused) focusContainer else Color.Transparent,
            labelColor = if (tvFocused) focusContent else MaterialTheme.colorScheme.onSurfaceVariant,
            iconColor = if (tvFocused) focusContent else MaterialTheme.colorScheme.onSurfaceVariant,
            selectedContainerColor = if (tvFocused) focusContainer else MaterialTheme.colorScheme.secondaryContainer,
            selectedLabelColor = if (tvFocused) focusContent else MaterialTheme.colorScheme.onSecondaryContainer,
            selectedLeadingIconColor = if (tvFocused) focusContent else MaterialTheme.colorScheme.onSecondaryContainer,
            selectedTrailingIconColor = if (tvFocused) focusContent else MaterialTheme.colorScheme.onSecondaryContainer,
        ),
        interactionSource = interactionSource,
    )
}

/**
 * TV focus treatment: gentle scale-up while focused.
 *
 * Smoothness notes:
 * - Only [graphicsLayer] reads the animated value, so focus changes never trigger
 *   recomposition — the layer just re-draws.
 * - A symmetric pivot (center) plus snap (no spring overshoot) keeps neighboring
 *   items from being clipped mid-animation.
 */
@Composable
internal fun Modifier.tvFocusScale(isTv: Boolean, focusedScale: Float = 1.08f): Modifier {
    var focused by remember { mutableStateOf(false) }
    val scale by animateFloatAsState(
        targetValue = if (isTv && focused) focusedScale else 1f,
        animationSpec = tween(durationMillis = 150, easing = LinearOutSlowInEasing),
        label = "TV focus",
    )
    return this
        .onFocusChanged { focused = it.isFocused }
        .graphicsLayer {
            scaleX = scale
            scaleY = scale
            transformOrigin = TransformOrigin.Center
        }
        .zIndex(if (focused) 1f else 0f)
}
