@file:OptIn(
    ExperimentalMaterial3Api::class,
    ExperimentalAnimationApi::class,
    ExperimentalMaterial3WindowSizeClassApi::class,
    ExperimentalComposeUiApi::class,
    ExperimentalTvMaterial3Api::class,
)

package org.movo.app.account

import org.movo.app.ui.tvInitialFocus
import org.movo.app.ui.TV_OVERSCAN_HORIZONTAL
import org.movo.app.ui.TV_OVERSCAN_VERTICAL
import org.movo.app.catalog.MediaGrid
import org.movo.app.home.ConfirmLogoutDialog
import org.movo.app.ui.Empty
import org.movo.app.ui.Loading
import org.movo.app.ui.MovoChoiceChip
import org.movo.app.ui.tvFocusMemory
import org.movo.app.ui.tvFocusScale
import org.movo.app.R
import org.movo.app.core.AppState
import org.movo.app.core.FavoriteGroup
import org.movo.app.core.HistoryEntry
import org.movo.app.core.MovoViewModel
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.ExperimentalAnimationApi
import androidx.compose.foundation.clickable
import androidx.compose.foundation.background
import androidx.compose.foundation.focusGroup
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Logout
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.material3.windowsizeclass.ExperimentalMaterial3WindowSizeClassApi
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.focus.focusRestorer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.shape.RoundedCornerShape
import coil3.compose.AsyncImage
import androidx.tv.material3.Button as TvButton
import androidx.tv.material3.ExperimentalTvMaterial3Api
import androidx.tv.material3.FilterChip as TvFilterChip
import androidx.tv.material3.IconButton as TvIconButton
import androidx.tv.material3.ListItem as TvListItem
import androidx.tv.material3.ListItemScale
import androidx.tv.material3.SelectableChipScale
import androidx.tv.material3.Surface as TvSurface
import androidx.tv.material3.Icon as TvIcon
import androidx.tv.material3.Text as TvText

@Composable
internal fun LoginScreen(
    loading: Boolean,
    error: String?,
    isTv: Boolean,
    login: (String, String) -> Unit,
    clearError: () -> Unit,
) {
    var name by rememberSaveable { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var passwordVisible by remember { mutableStateOf(false) }
    Box(
        Modifier
            .fillMaxSize()
            .imePadding()
            .verticalScroll(rememberScrollState()),
        contentAlignment = Alignment.Center,
    ) {
        Card(Modifier.widthIn(max = 420.dp).padding(24.dp)) {
            Column(
                Modifier.padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                Text(stringResource(R.string.app_name), style = MaterialTheme.typography.headlineLarge)
                Text(stringResource(R.string.sign_in_account))
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it; clearError() },
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text(stringResource(R.string.login_hint)) },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(
                        keyboardType = KeyboardType.Email,
                        imeAction = ImeAction.Next,
                    ),
                )
                OutlinedTextField(
                    value = password,
                    onValueChange = { password = it; clearError() },
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text(stringResource(R.string.password_hint)) },
                    singleLine = true,
                    visualTransformation = if (passwordVisible) VisualTransformation.None else PasswordVisualTransformation(),
                    trailingIcon = {
                        IconButton({ passwordVisible = !passwordVisible }, Modifier.tvFocusScale(isTv)) {
                            Icon(
                                if (passwordVisible) Icons.Default.VisibilityOff else Icons.Default.Visibility,
                                contentDescription = stringResource(
                                    if (passwordVisible) R.string.hide_password else R.string.show_password,
                                ),
                            )
                        }
                    },
                    keyboardOptions = KeyboardOptions(
                        keyboardType = KeyboardType.Password,
                        imeAction = ImeAction.Done,
                    ),
                    keyboardActions = KeyboardActions(
                        onDone = { if (name.isNotBlank() && password.isNotBlank()) login(name, password) },
                    ),
                )
                error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                Button(
                    onClick = { login(name, password) },
                    modifier = Modifier.fillMaxWidth().tvFocusScale(isTv),
                    enabled = !loading && name.isNotBlank() && password.isNotBlank(),
                ) {
                    Text(if (loading) stringResource(R.string.signing_in) else stringResource(R.string.sign_in))
                }
                Text(
                    stringResource(R.string.password_disclaimer),
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }
    }
}

@Composable
internal fun NotificationsScreen(state: AppState, isTv: Boolean, model: MovoViewModel) {
    if (state.notifications.isEmpty()) {
        if (state.loading) Loading(stringResource(R.string.loading_content)) else Empty(stringResource(R.string.notifications_empty))
        return
    }
    val focusedIndex = remember(state.notifications, state.focusedUrl) {
        state.notifications
            .flatMap { group -> listOf(group.date) + group.items.map { "${group.date}:${it.url}:${it.info}" } }
            .indexOf(state.focusedUrl)
            .coerceAtLeast(0)
    }
    val listState = rememberLazyListState(initialFirstVisibleItemIndex = focusedIndex)
    LazyColumn(
        state = listState,
        contentPadding = if (isTv) PaddingValues(horizontal = TV_OVERSCAN_HORIZONTAL, vertical = TV_OVERSCAN_VERTICAL) else PaddingValues(12.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        modifier = if (isTv) Modifier.focusRestorer().focusGroup() else Modifier.focusGroup(),
    ) {
        state.notifications.forEach { group ->
            item(group.date) {
                Text(
                    group.date,
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.padding(start = 4.dp, top = 8.dp),
                )
            }
            items(group.items, key = { "${group.date}:${it.url}:${it.info}" }) { item ->
                val focusKey = "${group.date}:${item.url}:${item.info}"
                val modifier = Modifier
                    .fillMaxWidth()
                    .then(if (isTv) Modifier.tvFocusMemory(focusKey) else Modifier)
                if (isTv) {
                    TvListItem(
                        selected = false,
                        onClick = { model.openDetails(item.url, focusKey) },
                        headlineContent = { TvText(item.title) },
                        supportingContent = { TvText(item.info) },
                        leadingContent = { TvIcon(Icons.Default.NewReleases, null) },
                        modifier = modifier,
                        scale = ListItemScale.None,
                    )
                } else {
                    ElevatedCard(
                        onClick = { model.openDetails(item.url, focusKey) },
                        modifier = modifier,
                    ) {
                    ListItem(
                        headlineContent = { Text(item.title) },
                        supportingContent = { Text(item.info) },
                        leadingContent = { Icon(Icons.Default.NewReleases, null) },
                        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                    )
                    }
                }
            }
        }
    }
}

@Composable
internal fun AccountScreen(state: AppState, isTv: Boolean, model: MovoViewModel) {
    val user = state.user ?: return
    var confirmLogout by remember { mutableStateOf(false) }
    if (isTv) {
        TvAccountScreen(state) { confirmLogout = true }
        if (confirmLogout) ConfirmLogoutDialog(true, { confirmLogout = false }) {
            confirmLogout = false
            model.logout()
        }
        return
    }
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.TopCenter) {
        ElevatedCard(Modifier.widthIn(max = 720.dp).padding(20.dp)) {
            Column(
                Modifier.padding(24.dp).verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                    AsyncImage(user.avatarUrl, null, Modifier.size(80.dp).clip(RoundedCornerShape(18.dp)), contentScale = ContentScale.Crop)
                    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Text(user.username, style = MaterialTheme.typography.headlineSmall)
                        user.email?.let { Text(it, color = MaterialTheme.colorScheme.onSurfaceVariant) }
                        val days = state.premiumDays ?: user.premiumDays
                        if (user.vip || days != null) {
                            AssistChip(
                                onClick = {},
                                enabled = false,
                                label = {
                                    Text(
                                        days?.let { pluralStringResource(R.plurals.premium_days, it, it) }
                                            ?: stringResource(R.string.premium_active),
                                    )
                                },
                                leadingIcon = { Icon(Icons.Default.WorkspacePremium, null, Modifier.size(16.dp)) },
                            )
                        }
                    }
                }
                HorizontalDivider()
                Text(stringResource(R.string.official_account), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                OutlinedButton({ confirmLogout = true }) {
                    Icon(Icons.AutoMirrored.Filled.Logout, null); Text(stringResource(R.string.menu_sign_out))
                }
                HorizontalDivider()
                AboutSection()
            }
        }
    }
    if (confirmLogout) ConfirmLogoutDialog(isTv = false, dismiss = { confirmLogout = false }) {
        confirmLogout = false
        model.logout()
    }
}

@Composable
private fun TvAccountScreen(state: AppState, requestLogout: () -> Unit) {
    val user = state.user ?: return
    val context = LocalContext.current
    val version = remember(context) {
        runCatching { context.packageManager.getPackageInfo(context.packageName, 0).versionName }
            .getOrNull()
            .orEmpty()
    }
    var aboutExpanded by rememberSaveable { mutableStateOf(false) }
    Box(
        Modifier.fillMaxSize().padding(horizontal = TV_OVERSCAN_HORIZONTAL, vertical = TV_OVERSCAN_VERTICAL),
        contentAlignment = Alignment.TopCenter,
    ) {
        TvSurface(Modifier.widthIn(max = 720.dp).fillMaxHeight()) {
            LazyColumn(
                Modifier.fillMaxSize().focusRestorer().focusGroup(),
                contentPadding = PaddingValues(24.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                item("profile") {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(16.dp),
                    ) {
                        AsyncImage(
                            user.avatarUrl,
                            null,
                            Modifier.size(112.dp).clip(RoundedCornerShape(18.dp)),
                            contentScale = ContentScale.Crop,
                        )
                        Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(user.username, style = MaterialTheme.typography.headlineSmall)
                            user.email?.let { Text(it, color = MaterialTheme.colorScheme.onSurfaceVariant) }
                            val days = state.premiumDays ?: user.premiumDays
                            if (user.vip || days != null) {
                                Text(
                                    days?.let { pluralStringResource(R.plurals.premium_days, it, it) }
                                        ?: stringResource(R.string.premium_active),
                                    color = MaterialTheme.colorScheme.primary,
                                )
                            }
                        }
                    }
                }
                item("official") {
                    Text(
                        stringResource(R.string.official_account),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                item("logout") {
                    TvButton(
                        onClick = requestLogout,
                        modifier = Modifier.tvFocusMemory("account:logout"),
                    ) {
                        TvIcon(Icons.AutoMirrored.Filled.Logout, contentDescription = null)
                        TvText(stringResource(R.string.menu_sign_out))
                    }
                }
                item("about") {
                    TvListItem(
                        selected = aboutExpanded,
                        onClick = { aboutExpanded = !aboutExpanded },
                        modifier = Modifier.tvFocusMemory("account:about"),
                        headlineContent = { TvText(stringResource(R.string.about_title)) },
                        leadingContent = { TvIcon(Icons.Default.Info, contentDescription = null) },
                        trailingContent = {
                            TvIcon(
                                if (aboutExpanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                                contentDescription = stringResource(if (aboutExpanded) R.string.collapse else R.string.more),
                            )
                        },
                        scale = ListItemScale.None,
                    )
                    if (aboutExpanded) {
                        Column(
                            Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            Text(stringResource(R.string.about_body), style = MaterialTheme.typography.bodyMedium)
                            Text(
                                stringResource(R.string.about_disclaimer),
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            Text(
                                stringResource(R.string.about_version, version),
                                style = MaterialTheme.typography.labelMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun AboutSection() {
    val context = LocalContext.current
    val version = remember(context) {
        runCatching {
            context.packageManager.getPackageInfo(context.packageName, 0).versionName
        }.getOrNull().orEmpty()
    }
    var expanded by rememberSaveable { mutableStateOf(false) }
    Column(
        Modifier
            .fillMaxWidth()
            .clickable { expanded = !expanded },
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Icon(
                Icons.Default.Info,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.primary,
                modifier = Modifier.size(20.dp),
            )
            Text(
                stringResource(R.string.about_title),
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.weight(1f),
            )
            Icon(
                if (expanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                contentDescription = stringResource(if (expanded) R.string.collapse else R.string.more),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        AnimatedVisibility(visible = expanded) {
            Column(Modifier.padding(top = 8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(stringResource(R.string.about_body), style = MaterialTheme.typography.bodyMedium)
                Text(
                    stringResource(R.string.about_disclaimer),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    stringResource(R.string.about_version, version),
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
internal fun FavoritesScreen(state: AppState, isTv: Boolean, model: MovoViewModel) {
    val gridFocusRequester = remember { FocusRequester() }
    Column(Modifier.fillMaxSize()) {
        if (isTv) {
            TvFavoriteFilters(
                state.favoriteGroups,
                state.favoriteGroup,
                gridFocusRequester,
                model::loadFavorites,
            )
        } else {
            LazyRow(
                contentPadding = PaddingValues(12.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(state.favoriteGroups, key = { it.id ?: it.name }) { group ->
                    MovoChoiceChip(
                        selected = state.favoriteGroup == group.id,
                        onClick = { model.loadFavorites(group.id) },
                        label = { Text("${group.name} (${group.count})") },
                        isTv = false,
                    )
                }
            }
        }
        key(state.favoriteGroup) {
            MediaGrid(
                state.items,
                isTv,
                state.loading,
                state.focusedUrl,
                { url -> model.openDetails(url, url) },
                emptyTitle = stringResource(R.string.favorites_empty),
                emptyHint = stringResource(R.string.favorites_empty_hint),
                entryFocusRequester = if (isTv) gridFocusRequester else null,
            ) { model.loadFavorites(append = true) }
        }
    }
}

@Composable
internal fun TvFavoriteFilters(
    groups: List<FavoriteGroup>,
    selectedGroup: Long?,
    gridFocusRequester: FocusRequester,
    select: (Long?) -> Unit,
) {
    LazyRow(
        contentPadding = PaddingValues(horizontal = TV_OVERSCAN_HORIZONTAL, vertical = 12.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        modifier = Modifier
            .focusProperties { down = gridFocusRequester }
            .focusRestorer()
            .focusGroup(),
    ) {
        items(groups, key = { it.id ?: it.name }) { group ->
            TvFilterChip(
                selected = selectedGroup == group.id,
                onClick = { select(group.id) },
                modifier = Modifier
                    .tvFocusMemory("favorites:group:${group.id}")
                    .testTag("tv-favorite-group-${group.id ?: group.name}"),
                scale = SelectableChipScale.None,
            ) {
                TvText("${group.name} (${group.count})")
            }
        }
    }
}

@Composable
internal fun HistoryScreen(state: AppState, isTv: Boolean, model: MovoViewModel) {
    if (state.history.isEmpty()) {
        if (state.loading) Loading(stringResource(R.string.loading_content))
        else Empty(stringResource(R.string.history_empty))
        return
    }
    val initialIndex = remember(state.history, state.focusedUrl) {
        state.history.indexOfFirst { state.focusedUrl?.startsWith(it.id) == true }.coerceAtLeast(0)
    }
    val listState = rememberLazyListState(initialFirstVisibleItemIndex = initialIndex)
    Box(Modifier.fillMaxSize()) {
        LazyColumn(
            state = listState,
            contentPadding = if (isTv) PaddingValues(horizontal = TV_OVERSCAN_HORIZONTAL, vertical = TV_OVERSCAN_VERTICAL) else PaddingValues(12.dp),
            verticalArrangement = Arrangement.spacedBy(if (isTv) 16.dp else 8.dp),
            modifier = Modifier
                .widthIn(max = 1100.dp)
                .fillMaxSize()
                .align(Alignment.TopCenter)
                .then(if (isTv) Modifier.focusRestorer().focusGroup() else Modifier.focusGroup()),
        ) {
            items(state.history, key = { it.id }) { entry ->
                HistoryCard(entry, isTv, model)
            }
        }
    }
}

@Composable
private fun HistoryCard(entry: HistoryEntry, isTv: Boolean, model: MovoViewModel) {
    if (isTv) {
        TvHistoryCard(
            entry,
            open = { model.openDetails(entry.url, entry.id) },
            toggle = { model.toggleHistory(entry) },
            remove = { model.removeHistory(entry) },
        )
        return
    }
    var confirmRemoval by remember { mutableStateOf(false) }
    ElevatedCard(
        onClick = { model.openDetails(entry.url, entry.id) },
        modifier = Modifier
            .fillMaxWidth()
            .alpha(if (entry.watched) .72f else 1f),
        shape = RoundedCornerShape(16.dp),
    ) {
        ListItem(
            headlineContent = { Text(entry.title.ifEmpty { stringResource(R.string.unknown) }) },
            supportingContent = {
                Text(listOfNotNull(entry.info, entry.additionalInfo, entry.date).joinToString(" • "))
            },
            leadingContent = {
                AsyncImage(
                    model = entry.posterUrl,
                    contentDescription = null,
                    modifier = Modifier
                        .size(64.dp, 92.dp)
                        .clip(RoundedCornerShape(10.dp))
                        .background(MaterialTheme.colorScheme.surfaceVariant),
                    contentScale = ContentScale.Crop,
                )
            },
            trailingContent = {
                val watched = entry.watched
                Row {
                    IconButton({ model.toggleHistory(entry) }) {
                        Icon(
                            if (watched) Icons.Default.CheckCircle else Icons.Default.RadioButtonUnchecked,
                            contentDescription = stringResource(
                                if (watched) R.string.mark_unwatched else R.string.mark_watched,
                            ),
                        )
                    }
                    IconButton({ confirmRemoval = true }) {
                        Icon(Icons.Default.Delete, contentDescription = stringResource(R.string.remove_history))
                    }
                }
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
        )
    }
    if (confirmRemoval) {
        AlertDialog(
            onDismissRequest = { confirmRemoval = false },
            title = { Text(stringResource(R.string.remove_history_title)) },
            text = { Text(stringResource(R.string.remove_history_message, entry.title)) },
            confirmButton = {
                TextButton({ confirmRemoval = false; model.removeHistory(entry) }) {
                    Text(stringResource(R.string.remove))
                }
            },
            dismissButton = {
                TextButton({ confirmRemoval = false }) { Text(stringResource(R.string.cancel)) }
            },
        )
    }
}

@Composable
internal fun TvHistoryCard(
    entry: HistoryEntry,
    open: () -> Unit,
    toggle: () -> Unit,
    remove: () -> Unit,
) {
    var confirmRemoval by remember { mutableStateOf(false) }

    Row(
        Modifier.fillMaxWidth().focusRestorer().focusGroup(),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TvListItem(
            selected = entry.watched,
            onClick = open,
            headlineContent = { TvText(entry.title.ifEmpty { stringResource(R.string.unknown) }) },
            supportingContent = {
                TvText(listOfNotNull(entry.info, entry.additionalInfo, entry.date).joinToString(" • "))
            },
            leadingContent = {
                AsyncImage(
                    model = entry.posterUrl,
                    contentDescription = null,
                    modifier = Modifier
                        .size(88.dp, 132.dp)
                        .clip(RoundedCornerShape(10.dp))
                        .background(MaterialTheme.colorScheme.surfaceVariant),
                    contentScale = ContentScale.Crop,
                )
            },
            modifier = Modifier
                .weight(1f)
                .tvFocusMemory(entry.id)
                .testTag("tv-history-primary"),
            scale = ListItemScale.None,
        )
        TvIconButton(
            onClick = toggle,
            modifier = Modifier.tvFocusMemory("${entry.id}:toggle").testTag("tv-history-toggle"),
        ) {
            TvIcon(
                if (entry.watched) Icons.Default.CheckCircle else Icons.Default.RadioButtonUnchecked,
                contentDescription = stringResource(
                    if (entry.watched) R.string.mark_unwatched else R.string.mark_watched,
                ),
            )
        }
        TvIconButton(
            onClick = { confirmRemoval = true },
            modifier = Modifier.tvFocusMemory("${entry.id}:delete").testTag("tv-history-delete"),
        ) {
            TvIcon(Icons.Default.Delete, contentDescription = stringResource(R.string.remove_history))
        }
    }

    if (confirmRemoval) {
        AlertDialog(
            onDismissRequest = { confirmRemoval = false },
            title = { Text(stringResource(R.string.remove_history_title)) },
            text = { Text(stringResource(R.string.remove_history_message, entry.title)) },
            confirmButton = {
                TvButton(onClick = { confirmRemoval = false; remove() }) {
                    TvText(stringResource(R.string.remove))
                }
            },
            dismissButton = {
                TvButton(
                    onClick = { confirmRemoval = false },
                    modifier = Modifier
                        .tvInitialFocus()
                        .testTag("tv-history-cancel"),
                ) {
                    TvText(stringResource(R.string.cancel))
                }
            },
        )
    }
}
