@file:OptIn(ExperimentalMaterial3Api::class, ExperimentalTvMaterial3Api::class)

package org.movo.app.settings

import androidx.compose.runtime.Stable
import androidx.datastore.preferences.core.Preferences
import org.movo.app.ui.sectionHeading
import androidx.compose.foundation.layout.PaddingValues
import org.movo.app.ui.TV_OVERSCAN_HORIZONTAL
import org.movo.app.ui.TV_OVERSCAN_VERTICAL
import org.movo.app.ui.MovoChoiceChip
import org.movo.app.ui.tvFocusMemory
import org.movo.app.R
import org.movo.app.core.Tab
import org.movo.app.home.label
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
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.surfaceColorAtElevation
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
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
import org.movo.app.ui.TvFilterChip
import org.movo.app.ui.TvListItem
import androidx.tv.material3.ListItemScale
import androidx.tv.material3.MaterialTheme as TvMaterialTheme
import androidx.tv.material3.SelectableChipScale
import androidx.tv.material3.SurfaceDefaults
import androidx.tv.material3.Surface as TvSurface
import androidx.tv.material3.Switch as TvSwitch
import androidx.tv.material3.Text as TvText

/** Where the settings editor writes. One method, because every setting is one key and one value. */
@Stable
interface SettingsActions {
    fun <T> save(key: Preferences.Key<T>, value: T)
}

/**
 * Settings editor (content only — the host `Scaffold` supplies the top bar + insets).
 *
 * Reads the current [settings] and writes through [actions]; the host observes the backing
 * DataStore and re-applies theme / layout-mode changes live.
 */
@Composable
fun SettingsContent(
    settings: AppSettings,
    isTv: Boolean,
    actions: SettingsActions,
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
                    choose = { actions.save(Keys.LAYOUT_MODE, it.name) },
                    isTv = isTv,
                )
                ChoiceSection(
                    title = stringResource(R.string.initial_screen),
                    values = Tab.entries,
                    selected = settings.initialTab,
                    label = { stringResource(it.label) },
                    choose = { actions.save(Keys.INITIAL_TAB, it.name) },
                    isTv = isTv,
                )
                SwitchItem(stringResource(R.string.sort_voices), settings.sortVoices, isTv) { actions.save(Keys.SORT_VOICES, it) }
                TextItem(
                    label = stringResource(R.string.hidden_countries),
                    hint = stringResource(R.string.hidden_countries_hint),
                    value = settings.hiddenCountries,
                ) { actions.save(Keys.HIDDEN_COUNTRIES, it) }
                }
            }

            item {
                SettingsGroup(title = stringResource(R.string.settings_group_appearance), isTv = isTv) {
                ChoiceSection(
                    title = stringResource(R.string.theme),
                    values = ThemePref.entries,
                    selected = settings.theme,
                    label = { stringResource(it.label) },
                    choose = { actions.save(Keys.THEME, it.name) },
                    isTv = isTv,
                )
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                    SwitchItem(
                        label = stringResource(R.string.dynamic_color),
                        checked = settings.useDynamicColor,
                        isTv = isTv,
                        onCheckedChange = { actions.save(Keys.USE_DYNAMIC_COLOR, it) },
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
                    choose = { actions.save(Keys.QUALITY_MODE, it.name) },
                    isTv = isTv,
                )
                SwitchItem(stringResource(R.string.auto_next), settings.autoNext, isTv) { actions.save(Keys.AUTO_NEXT, it) }
                if (!isTv) SwitchItem(stringResource(R.string.picture_in_picture), settings.pictureInPicture, isTv) { actions.save(Keys.PICTURE_IN_PICTURE, it) }
                ChoiceSection(
                    title = stringResource(R.string.seek_interval),
                    values = listOf(5, 10, 15, 30),
                    selected = settings.seekSeconds,
                    label = { stringResource(R.string.seconds_short, it) },
                    choose = { actions.save(Keys.SEEK_SECONDS, it) },
                    isTv = isTv,
                )
                ChoiceSection(
                    title = stringResource(R.string.default_speed),
                    values = listOf(.75f, 1f, 1.25f, 1.5f, 2f),
                    selected = settings.playbackSpeed,
                    label = { "${it}×" },
                    choose = { actions.save(Keys.PLAYBACK_SPEED, it) },
                    isTv = isTv,
                )
                ChoiceSection(
                    title = stringResource(R.string.video_fit),
                    values = VideoFit.entries,
                    selected = settings.videoFit,
                    label = { stringResource(it.label) },
                    choose = { actions.save(Keys.VIDEO_FIT, it.name) },
                    isTv = isTv,
                )
                SwitchItem(stringResource(R.string.ask_quality), settings.askQuality, isTv) { actions.save(Keys.ASK_QUALITY, it) }
                SwitchItem(stringResource(R.string.save_quality), settings.saveQuality, isTv) { actions.save(Keys.SAVE_QUALITY, it) }
                }
            }

            item {
                SettingsGroup(title = stringResource(R.string.settings_group_player_overlay), isTv = isTv) {
                SwitchItem(stringResource(R.string.show_buffer), settings.showBuffer, isTv) { actions.save(Keys.SHOW_BUFFER, it) }
                SwitchItem(stringResource(R.string.show_end_time), settings.showEndTime, isTv) { actions.save(Keys.SHOW_END_TIME, it) }
                ChoiceSection(
                    title = stringResource(R.string.buffer_window),
                    values = listOf(0, 15, 30, 60, 120, 180),
                    selected = settings.bufferSeconds,
                    label = { if (it == 0) stringResource(R.string.layout_auto) else stringResource(R.string.seconds_short, it) },
                    choose = { actions.save(Keys.BUFFER_SECONDS, it) },
                    isTv = isTv,
                )
                }
            }

            item {
                SettingsGroup(title = stringResource(R.string.settings_group_tv), isTv = isTv) {
                SwitchItem(stringResource(R.string.tv_center_pauses), settings.tvCenterPauses, isTv) { actions.save(Keys.TV_CENTER_PAUSES, it) }
                if (settings.tvCenterPauses) SwitchItem(stringResource(R.string.tv_pause_shows_controls), settings.tvPauseShowsControls, isTv) { actions.save(Keys.TV_PAUSE_SHOWS_CONTROLS, it) }
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
    val inset = if (isTv) TV_OVERSCAN_HORIZONTAL else 16.dp
    Text(
        text = title,
        style = MaterialTheme.typography.labelLarge,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(start = inset + 4.dp, top = 20.dp, bottom = 4.dp).sectionHeading(),
    )
    val modifier = Modifier
        .fillMaxWidth()
        .padding(horizontal = inset, vertical = 4.dp)
    if (isTv) {
        TvSurface(
            modifier = modifier,
            shape = RoundedCornerShape(16.dp),
            colors = SurfaceDefaults.colors(containerColor = TvMaterialTheme.colorScheme.surfaceVariant),
        ) {
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
        modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp).sectionHeading(),
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
        // Not `selected = checked`: that paints a switched-on row as a chosen list entry.
        TvListItem(
            selected = false,
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

/** A free-text setting, saved as it is typed. */
@Composable
private fun TextItem(
    label: String,
    hint: String,
    value: String,
    onValueChange: (String) -> Unit,
) {
    // Edits show at once and the store catches up; a field bound straight to the store lags a
    // write behind every keystroke.
    var text by remember(value) { mutableStateOf(value) }
    OutlinedTextField(
        value = text,
        onValueChange = { text = it; onValueChange(it) },
        label = { Text(label) },
        supportingText = { Text(hint) },
        singleLine = true,
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
    )
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
