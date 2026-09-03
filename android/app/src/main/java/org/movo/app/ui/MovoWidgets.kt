@file:OptIn(
    ExperimentalMaterial3Api::class,
    ExperimentalAnimationApi::class,
    ExperimentalMaterial3WindowSizeClassApi::class,
    ExperimentalComposeUiApi::class,
    ExperimentalTvMaterial3Api::class,
)

package org.movo.app.ui

import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.window.layout.FoldingFeature
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.foundation.focusGroup
import androidx.compose.ui.focus.focusRestorer
import android.content.Context
import android.provider.Settings
import androidx.compose.animation.core.FiniteAnimationSpec
import androidx.compose.animation.core.snap
import androidx.compose.animation.core.spring
import androidx.tv.material3.ColorScheme as TvColorScheme
import androidx.tv.material3.darkColorScheme as tvDarkColorScheme
import androidx.tv.material3.lightColorScheme as tvLightColorScheme
import org.movo.app.settings.save
import androidx.compose.animation.ExperimentalAnimationApi
import androidx.compose.animation.core.LinearOutSlowInEasing
import androidx.compose.animation.core.Animatable
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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.Stable
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.mapSaver
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.onPlaced
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import kotlinx.coroutines.launch
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.tv.material3.ExperimentalTvMaterial3Api

internal val MovoBlue = Color(0xFF9CCAFF)

/**
 * Whether the user has turned system animations off. Compose has no built-in for this, so it is
 * read once per activity from the global animator scale and handed down; every animation in the
 * app checks it and snaps instead of moving.
 */
internal val LocalReducedMotion = staticCompositionLocalOf { false }

/**
 * The hinge of a foldable held half-open, or null on any device or posture without one.
 *
 * Nothing the user reads or touches may straddle it: on a book-posture fold the seam runs down
 * the middle of the window, and on a tabletop fold it runs across it.
 */
internal val LocalFold = staticCompositionLocalOf<FoldingFeature?> { null }

/** Height of the window above a tabletop fold, which is where the video belongs. */
@Composable
internal fun FoldingFeature.topHeight(): Dp = with(LocalDensity.current) { bounds.top.toDp() }

/** Width of the seam itself, which a layout has to leave empty. */
@Composable
internal fun FoldingFeature.seamWidth(): Dp = with(LocalDensity.current) { bounds.width().toDp() }

/** Marks a section title, so a screen reader can jump between sections instead of reading through. */
internal fun Modifier.sectionHeading() = semantics { heading() }

/** Reads [Settings.Global.ANIMATOR_DURATION_SCALE], which is 0 when animations are off. */
internal fun reducedMotionEnabled(context: Context) =
    Settings.Global.getFloat(context.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) == 0f

/** [spec] normally, an instant jump when the user has asked for no animation. */
@Composable
internal fun <T> motionSpec(spec: FiniteAnimationSpec<T>): FiniteAnimationSpec<T> =
    if (LocalReducedMotion.current) snap() else spec

/**
 * TV safe area: 5% of each edge, the margin a television may crop. Everything the user has to
 * read or aim at on the TV surface stays inside it.
 */
internal val TV_OVERSCAN_HORIZONTAL = 48.dp
internal val TV_OVERSCAN_VERTICAL = 27.dp

/**
 * Restates the resolved Material 3 scheme in the shape tv-material wants.
 *
 * The TV components carry their own theme, so without this the drawer and the content beside it
 * are coloured by two unrelated schemes — visibly so under dynamic colour, where the content
 * follows the wallpaper and a hand-written TV palette does not. [isDark] only picks which set of
 * defaults fills in `border` and `scrim`, which Material 3 has no counterpart for.
 */
internal fun ColorScheme.toTvColorScheme(isDark: Boolean): TvColorScheme {
    val build = if (isDark) ::tvDarkColorScheme else ::tvLightColorScheme
    return build(
        primary, onPrimary, primaryContainer, onPrimaryContainer, inversePrimary,
        secondary, onSecondary, secondaryContainer, onSecondaryContainer,
        tertiary, onTertiary, tertiaryContainer, onTertiaryContainer,
        background, onBackground, surface, onSurface, surfaceVariant, onSurfaceVariant,
        surfaceTint, inverseSurface, inverseOnSurface,
        error, onError, errorContainer, onErrorContainer,
        outline, outlineVariant, scrim,
    )
}

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

/** One-shot latch for a focus request that must not fire again on a later placement. */
private class RestoreOnce {
    var done = false
}

/**
 * Takes focus the first time the node is placed.
 *
 * Not a launched effect: a request against a node that has not been placed yet is dropped, and
 * the caller is then left with a dialog or a banner nothing on the remote can reach.
 */
@Composable
internal fun Modifier.tvInitialFocus(enabled: Boolean = true): Modifier {
    if (!enabled) return this
    val focusRequester = remember { FocusRequester() }
    val requested = remember { RestoreOnce() }
    return this.focusRequester(focusRequester).onPlaced {
        if (!requested.done) {
            requested.done = true
            runCatching { focusRequester.requestFocus() }
        }
    }
}

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
                    .fillMaxHeight(0.88f)
                    // Without a group to enter, the first press of the D-pad after the sheet
                    // opens lands nowhere.
                    .focusRestorer()
                    .focusGroup(),
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
    val scale by animateFloatAsState(
        targetValue = if (tvFocused) 1.06f else 1f,
        animationSpec = motionSpec(spring()),
        label = "choice focus",
    )
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
 * Nothing here is read from composition. The modifier returns before allocating anything off the
 * television, the target is set from the focus callback rather than from snapshot state, and the
 * only read of the animation is inside [graphicsLayer], which re-runs the layer instead of the
 * composable. The earlier version drove the target from a `mutableStateOf` and set `zIndex` from
 * the same flag: because this function returns a value rather than emitting, both reads landed in
 * the *caller's* restart scope, so every D-pad move recomposed the whole card or list row that
 * carried the modifier and re-measured its parent for the changed parent data. The scales here run
 * from 1.02 to 1.1 against rows that already space their children, so dropping the z-order lift
 * costs a few pixels of overlap at the trailing edge and nothing else.
 */
@Composable
internal fun Modifier.tvFocusScale(isTv: Boolean, focusedScale: Float = 1.08f): Modifier {
    if (!isTv) return this
    val reducedMotion = LocalReducedMotion.current
    val scale = remember { Animatable(1f) }
    val scope = rememberCoroutineScope()
    return this
        .onFocusChanged { focus ->
            val target = if (focus.isFocused) focusedScale else 1f
            scope.launch {
                if (reducedMotion) scale.snapTo(target)
                else scale.animateTo(target, tween(durationMillis = 150, easing = LinearOutSlowInEasing))
            }
        }
        .graphicsLayer {
            scaleX = scale.value
            scaleY = scale.value
        }
}
