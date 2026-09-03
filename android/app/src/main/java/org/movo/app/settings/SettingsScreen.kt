@file:OptIn(ExperimentalMaterial3Api::class, ExperimentalTvMaterial3Api::class)

package org.movo.app.settings

import androidx.compose.foundation.layout.PaddingValues
import org.movo.app.ui.TV_OVERSCAN_HORIZONTAL
import org.movo.app.ui.TV_OVERSCAN_VERTICAL
import org.movo.app.ui.MovoChoiceChip
import org.movo.app.ui.tvFocusMemory
import org.movo.app.settings.AppSettings
import org.movo.app.settings.LayoutMode
import org.movo.app.settings.QualityMode
import org.movo.app.settings.ThemePref
import org.movo.app.settings.VideoFit
import org.movo.app.settings.settings
import org.movo.app.R
import org.movo.app.core.Tab
import android.os.Build
import androidx.compose.foundation.focusGroup
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.surfaceColorAtElevation
import androidx.compose.runtime.Composable
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.focusRestorer
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.tv.material3.ExperimentalTvMaterial3Api
import androidx.tv.material3.FilterChip as TvFilterChip
import androidx.tv.material3.ListItem as TvListItem
import androidx.tv.material3.ListItemScale
import androidx.tv.material3.SelectableChipScale
import androidx.tv.material3.Surface as TvSurface
import androidx.tv.material3.Switch as TvSwitch
import androidx.tv.material3.Text as TvText

/**
 * Settings editor (content only — the host `Scaffold` supplies the top bar + insets).
 *
 * Reads the current [settings] and reports changes through the callbacks; the host observes the
 * backing DataStore and re-applies theme / layout-mode changes live.
 */
@Composable
fun SettingsContent(
    settings: AppSettings,
    isTv: Boolean,
    onLayoutModeChange: (LayoutMode) -> Unit,
    onThemeChange: (ThemePref) -> Unit,
    onDynamicColorChange: (Boolean) -> Unit,
    onQualityModeChange: (QualityMode) -> Unit,
    onAutoNextChange: (Boolean) -> Unit,
    onSeekSecondsChange: (Int) -> Unit,
    onPlaybackSpeedChange: (Float) -> Unit,
    onVideoFitChange: (VideoFit) -> Unit,
    onShowBufferChange: (Boolean) -> Unit,
    onShowEndTimeChange: (Boolean) -> Unit,
    onBufferSecondsChange: (Int) -> Unit,
    onTvCenterPausesChange: (Boolean) -> Unit,
    onTvPauseShowsControlsChange: (Boolean) -> Unit,
    onAskQualityChange: (Boolean) -> Unit,
    onSaveQualityChange: (Boolean) -> Unit,
    onSortVoicesChange: (Boolean) -> Unit,
    onInitialTabChange: (Tab) -> Unit,
    modifier: Modifier = Modifier,
) {
    Box(modifier.fillMaxSize()) {
        LazyColumn(
            Modifier
                .widthIn(max = 720.dp)
                .fillMaxWidth()
                .align(Alignment.TopCenter)
                .then(if (isTv) Modifier.testTag("tv-settings-list") else Modifier)
                .then(if (isTv) Modifier.focusRestorer().focusGroup() else Modifier),
            contentPadding = if (isTv) PaddingValues(vertical = TV_OVERSCAN_VERTICAL) else PaddingValues(),
        ) {
            item {
                SettingsGroup(title = stringResource(R.string.settings_group_general), isTv = isTv) {
                ChoiceSection(
                    title = stringResource(R.string.layout_mode),
                    values = LayoutMode.entries,
                    selected = settings.layoutMode,
                    label = { stringResource(it.label) },
                    choose = onLayoutModeChange,
                    isTv = isTv,
                )
                ChoiceSection(
                    title = stringResource(R.string.initial_screen),
                    values = Tab.entries,
                    selected = settings.initialTab,
                    label = { stringResource(it.settingsLabel) },
                    choose = onInitialTabChange,
                    isTv = isTv,
                )
                SwitchItem(stringResource(R.string.sort_voices), settings.sortVoices, isTv, onSortVoicesChange)
                }
            }

            item {
                SettingsGroup(title = stringResource(R.string.settings_group_appearance), isTv = isTv) {
                ChoiceSection(
                    title = stringResource(R.string.theme),
                    values = ThemePref.entries,
                    selected = settings.theme,
                    label = { stringResource(it.label) },
                    choose = onThemeChange,
                    isTv = isTv,
                )
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                    SwitchItem(
                        label = stringResource(R.string.dynamic_color),
                        checked = settings.useDynamicColor,
                        isTv = isTv,
                        onCheckedChange = onDynamicColorChange,
                    )
                }
                }
            }

            item {
                SettingsGroup(title = stringResource(R.string.settings_group_playback), isTv = isTv) {
                ChoiceSection(
                    title = stringResource(R.string.quality_mode),
                    values = QualityMode.entries,
                    selected = settings.qualityMode,
                    label = { stringResource(it.label) },
                    choose = onQualityModeChange,
                    isTv = isTv,
                )
                SwitchItem(stringResource(R.string.auto_next), settings.autoNext, isTv, onAutoNextChange)
                ChoiceSection(
                    title = stringResource(R.string.seek_interval),
                    values = listOf(5, 10, 15, 30),
                    selected = settings.seekSeconds,
                    label = { stringResource(R.string.seconds_short, it) },
                    choose = onSeekSecondsChange,
                    isTv = isTv,
                )
                ChoiceSection(
                    title = stringResource(R.string.default_speed),
                    values = listOf(.75f, 1f, 1.25f, 1.5f, 2f),
                    selected = settings.playbackSpeed,
                    label = { "${it}×" },
                    choose = onPlaybackSpeedChange,
                    isTv = isTv,
                )
                ChoiceSection(
                    title = stringResource(R.string.video_fit),
                    values = VideoFit.entries,
                    selected = settings.videoFit,
                    label = { stringResource(it.label) },
                    choose = onVideoFitChange,
                    isTv = isTv,
                )
                SwitchItem(stringResource(R.string.ask_quality), settings.askQuality, isTv, onAskQualityChange)
                SwitchItem(stringResource(R.string.save_quality), settings.saveQuality, isTv, onSaveQualityChange)
                }
            }

            item {
                SettingsGroup(title = stringResource(R.string.settings_group_player_overlay), isTv = isTv) {
                SwitchItem(stringResource(R.string.show_buffer), settings.showBuffer, isTv, onShowBufferChange)
                SwitchItem(stringResource(R.string.show_end_time), settings.showEndTime, isTv, onShowEndTimeChange)
                ChoiceSection(
                    title = stringResource(R.string.buffer_window),
                    values = listOf(0, 15, 30, 60, 120, 180),
                    selected = settings.bufferSeconds,
                    label = { if (it == 0) stringResource(R.string.layout_auto) else stringResource(R.string.seconds_short, it) },
                    choose = onBufferSecondsChange,
                    isTv = isTv,
                )
                }
            }

            item {
                SettingsGroup(title = stringResource(R.string.settings_group_tv), isTv = isTv) {
                SwitchItem(stringResource(R.string.tv_center_pauses), settings.tvCenterPauses, isTv, onTvCenterPausesChange)
                if (settings.tvCenterPauses) SwitchItem(stringResource(R.string.tv_pause_shows_controls), settings.tvPauseShowsControls, isTv, onTvPauseShowsControlsChange)
                }
            }

            item { Spacer(Modifier.height(if (isTv) TV_OVERSCAN_VERTICAL else 24.dp)) }
        }
    }
}

/** A titled card grouping related settings, replacing a flat list of dividers. */
@Composable
private fun SettingsGroup(
    title: String,
    isTv: Boolean,
    content: @Composable ColumnScope.() -> Unit,
) {
    Text(
        text = title,
        style = MaterialTheme.typography.labelLarge,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(start = 20.dp, top = 20.dp, bottom = 4.dp),
    )
    val modifier = Modifier
        .fillMaxWidth()
        .padding(horizontal = if (isTv) TV_OVERSCAN_HORIZONTAL else 16.dp, vertical = 4.dp)
    if (isTv) {
        TvSurface(modifier = modifier) {
            Column(Modifier.padding(vertical = 4.dp).focusRestorer().focusGroup(), content = content)
        }
    } else {
        Card(
            modifier = modifier,
            shape = RoundedCornerShape(16.dp),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceColorAtElevation(3.dp)),
        ) {
            Column(Modifier.padding(vertical = 4.dp), content = content)
        }
    }
}

@Composable
private fun SectionHeader(title: String) {
    Text(
        text = title,
        modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
        style = MaterialTheme.typography.titleMedium,
    )
}

@Composable
private fun <T> ChoiceSection(
    title: String,
    values: Iterable<T>,
    selected: T,
    label: @Composable (T) -> String,
    choose: (T) -> Unit,
    isTv: Boolean,
) {
    SectionHeader(title)
    FlowRow(
        modifier = Modifier
            .padding(horizontal = 16.dp, vertical = 4.dp)
            .selectableGroup()
            .then(if (isTv) Modifier.focusRestorer().focusGroup() else Modifier),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        values.forEach { value ->
            if (isTv) {
                TvFilterChip(
                    selected = selected == value,
                    onClick = { choose(value) },
                    modifier = Modifier
                        .tvFocusMemory("settings:$title:${label(value)}")
                        .semantics { role = Role.RadioButton },
                    scale = SelectableChipScale.None,
                ) {
                    TvText(label(value))
                }
            } else {
                MovoChoiceChip(
                    selected = selected == value,
                    onClick = { choose(value) },
                    label = { Text(label(value)) },
                    isTv = false,
                    modifier = Modifier.semantics { role = Role.RadioButton },
                )
            }
        }
    }
}

@Composable
private fun SwitchItem(
    label: String,
    checked: Boolean,
    isTv: Boolean,
    onCheckedChange: (Boolean) -> Unit,
) {
    if (isTv) {
        TvListItem(
            selected = checked,
            onClick = { onCheckedChange(!checked) },
            headlineContent = { TvText(label) },
            trailingContent = {
                TvSwitch(checked = checked, onCheckedChange = null)
            },
            modifier = Modifier.fillMaxWidth().tvFocusMemory("settings:$label"),
            scale = ListItemScale.None,
        )
        return
    }
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .toggleable(
                value = checked,
                role = Role.Switch,
                onValueChange = onCheckedChange,
            )
            .padding(vertical = 14.dp, horizontal = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(label, Modifier.weight(1f), style = MaterialTheme.typography.bodyLarge)
        Switch(
            checked = checked,
            onCheckedChange = null,
        )
    }
}

private val LayoutMode.label: Int get() = when (this) {
    LayoutMode.Auto -> R.string.layout_auto
    LayoutMode.Phone -> R.string.layout_phone
    LayoutMode.Tv -> R.string.layout_tv
}

private val ThemePref.label: Int get() = when (this) {
    ThemePref.System -> R.string.theme_system
    ThemePref.Light -> R.string.theme_light
    ThemePref.Dark -> R.string.theme_dark
}

private val VideoFit.label: Int get() = when (this) {
    VideoFit.Contain -> R.string.video_contain
    VideoFit.Cover -> R.string.video_cover
    VideoFit.Fill -> R.string.video_fill
}

private val Tab.settingsLabel: Int get() = when (this) {
    Tab.Catalog -> R.string.nav_catalog
    Tab.Search -> R.string.nav_search
    Tab.Collections -> R.string.nav_collections
    Tab.Favorites -> R.string.nav_favorites
    Tab.History -> R.string.nav_history
    Tab.Notifications -> R.string.nav_notifications
    Tab.Account -> R.string.nav_account
}
