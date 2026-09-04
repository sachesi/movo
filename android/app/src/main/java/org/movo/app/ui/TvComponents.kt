@file:OptIn(ExperimentalTvMaterial3Api::class)

package org.movo.app.ui

import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.RowScope
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.input.pointer.pointerInput
import androidx.tv.material3.ButtonColors
import androidx.tv.material3.ButtonDefaults
import androidx.tv.material3.CardDefaults
import androidx.tv.material3.CardScale
import androidx.tv.material3.ClickableChipScale
import androidx.tv.material3.ExperimentalTvMaterial3Api
import androidx.tv.material3.FilterChipDefaults
import androidx.tv.material3.ListItemDefaults
import androidx.tv.material3.ListItemScale
import androidx.tv.material3.SelectableChipScale
import androidx.tv.material3.AssistChip as TvMaterialAssistChip
import androidx.tv.material3.Button as TvMaterialButton
import androidx.tv.material3.Card as TvMaterialCard
import androidx.tv.material3.FilterChip as TvMaterialFilterChip
import androidx.tv.material3.IconButton as TvMaterialIconButton
import androidx.tv.material3.ListItem as TvMaterialListItem

/**
 * The tv-material controls the television layout is built from, answering a touch as well as
 * the remote.
 *
 * tv-material's clickable surface handles exactly two things: the D-pad centre key and the
 * semantics click action. It reads no pointer input at all, so on a phone or tablet showing the
 * television layout nothing tapped responded, and a phone switched to that layout could not even
 * reach the setting to switch back. Every use in the app goes through these wrappers, which add
 * the tap; a new tv-material control belongs here too rather than at a call site.
 */
@Composable
internal fun TvButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    colors: ButtonColors = ButtonDefaults.colors(),
    content: @Composable RowScope.() -> Unit,
) = TvMaterialButton(onClick, modifier.tapToClick(enabled, onClick), enabled = enabled, colors = colors, content = content)

@Composable
internal fun TvIconButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    content: @Composable BoxScope.() -> Unit,
) = TvMaterialIconButton(onClick, modifier.tapToClick(enabled, onClick), enabled = enabled, content = content)

@Composable
internal fun TvFilterChip(
    selected: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    scale: SelectableChipScale = FilterChipDefaults.scale(),
    content: @Composable () -> Unit,
) = TvMaterialFilterChip(selected, onClick, modifier.tapToClick(true, onClick), scale = scale, content = content)

@Composable
internal fun TvAssistChip(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    leadingIcon: (@Composable () -> Unit)? = null,
    trailingIcon: (@Composable () -> Unit)? = null,
    content: @Composable () -> Unit,
) = TvMaterialAssistChip(
    onClick,
    modifier.tapToClick(enabled, onClick),
    enabled = enabled,
    leadingIcon = leadingIcon,
    trailingIcon = trailingIcon,
    // No scale: these sit in rows that clip to their own height, which cut the focused chip off.
    scale = ClickableChipScale.None,
    content = content,
)

@Composable
internal fun TvListItem(
    selected: Boolean,
    onClick: () -> Unit,
    headlineContent: @Composable () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    supportingContent: (@Composable () -> Unit)? = null,
    leadingContent: (@Composable BoxScope.() -> Unit)? = null,
    trailingContent: (@Composable () -> Unit)? = null,
    scale: ListItemScale = ListItemDefaults.scale(),
) = TvMaterialListItem(
    selected = selected,
    onClick = onClick,
    headlineContent = headlineContent,
    modifier = modifier.tapToClick(enabled, onClick),
    enabled = enabled,
    supportingContent = supportingContent,
    leadingContent = leadingContent,
    trailingContent = trailingContent,
    scale = scale,
)

@Composable
internal fun TvCard(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    scale: CardScale = CardDefaults.scale(),
    content: @Composable ColumnScope.() -> Unit,
) = TvMaterialCard(onClick, modifier.tapToClick(true, onClick), scale = scale, content = content)

/**
 * A tap on a tv-material surface takes its focus and runs its click. Focus first, so the
 * highlight follows the finger the way it follows the remote and the focus memory records the
 * row the user left from. Pure pointer input: adding `clickable` would put a second focus target
 * and a second click action on the same node.
 */
@Composable
private fun Modifier.tapToClick(enabled: Boolean, onClick: () -> Unit): Modifier {
    val focusRequester = remember { FocusRequester() }
    val latestEnabled by rememberUpdatedState(enabled)
    val latestOnClick by rememberUpdatedState(onClick)
    return this
        .focusRequester(focusRequester)
        .pointerInput(Unit) {
            detectTapGestures {
                if (!latestEnabled) return@detectTapGestures
                runCatching { focusRequester.requestFocus() }
                latestOnClick()
            }
        }
}
