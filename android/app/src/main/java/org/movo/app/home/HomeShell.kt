@file:OptIn(
    ExperimentalMaterial3Api::class,
    ExperimentalAnimationApi::class,
    ExperimentalMaterial3WindowSizeClassApi::class,
    ExperimentalComposeUiApi::class,
    ExperimentalTvMaterial3Api::class,
)

package org.movo.app.home

import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.core.spring
import org.movo.app.ui.tvInitialFocus
import org.movo.app.ui.LocalReducedMotion
import org.movo.app.ui.motionSpec
import org.movo.app.Section
import org.movo.app.account.AccountScreen
import org.movo.app.account.FavoritesScreen
import org.movo.app.account.HistoryScreen
import org.movo.app.account.NotificationsScreen
import org.movo.app.catalog.CatalogScreen
import org.movo.app.catalog.CollectionsScreen
import org.movo.app.search.SearchScreen
import org.movo.app.settings.SettingsActions
import org.movo.app.settings.SettingsContent
import org.movo.app.ui.ErrorBanner
import org.movo.app.ui.LocalTvFocusMemory
import org.movo.app.ui.TvFocusMemory
import org.movo.app.ui.toTvColorScheme
import org.movo.app.ui.tvFocusScale
import org.movo.app.settings.settings
import androidx.datastore.preferences.core.Preferences
import org.movo.app.R
import org.movo.app.core.AppState
import org.movo.app.core.MovoViewModel
import org.movo.app.core.Tab
import org.movo.app.settings.AppSettings
import org.movo.app.settings.save
import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.ExperimentalAnimationApi
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.focusGroup
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsFocusedAsState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.material3.windowsizeclass.ExperimentalMaterial3WindowSizeClassApi
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.saveable.rememberSaveableStateHolder
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.focusRestorer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.shape.RoundedCornerShape
import kotlinx.coroutines.launch
import androidx.tv.material3.DrawerValue
import androidx.tv.material3.Button as TvButton
import androidx.tv.material3.ExperimentalTvMaterial3Api
import androidx.tv.material3.Icon as TvIcon
import androidx.tv.material3.MaterialTheme as TvMaterialTheme
import androidx.tv.material3.NavigationDrawer
import androidx.tv.material3.NavigationDrawerItem
import androidx.tv.material3.NavigationDrawerItemScale
import androidx.tv.material3.Text as TvText

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun HomeFlow(
    state: AppState,
    isTv: Boolean,
    useRail: Boolean,
    compactHeight: Boolean,
    model: MovoViewModel,
    settings: AppSettings,
    isDark: Boolean,
) {
    if (isTv) {
        TvHomeFlow(state, compactHeight, model, settings, isDark)
        return
    }
    var section by rememberSaveable { mutableStateOf(Section.Home) }

    Scaffold(
        topBar = {
            if (section == Section.Home) {
                HomeTopAppBar(state, model, isTv) { section = Section.Settings }
            } else {
                CenterAlignedTopAppBar(
                    title = { Text(stringResource(R.string.settings_title)) },
                    navigationIcon = {
                        IconButton({ section = Section.Home }, Modifier.tvFocusScale(isTv)) {
                            Icon(
                                imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                                contentDescription = stringResource(R.string.back),
                            )
                        }
                    },
                )
            }
        },
        bottomBar = {
            if (section == Section.Home && !useRail) HomeBottomBar(state, model)
        },
    ) { padding ->
        Row(Modifier.padding(padding).fillMaxSize()) {
            if (section == Section.Home && useRail) HomeNavigationRail(state, model, isTv) { section = Section.Settings }
            Box(
                Modifier
                    .weight(1f)
                    .fillMaxHeight()
                    .focusGroup(),
            ) {
                val reducedMotion = LocalReducedMotion.current
                AnimatedContent(
                    targetState = section,
                    transitionSpec = {
                        if (reducedMotion) EnterTransition.None togetherWith ExitTransition.None
                        else fadeIn() togetherWith fadeOut()
                    },
                ) { s ->
                    when (s) {
                        Section.Home -> HomeTabs(state, false, compactHeight, model)
                        Section.Settings -> SettingsDestination(settings, false)
                    }
                }
                if (state.loading) {
                    LinearProgressIndicator(Modifier.fillMaxWidth().align(Alignment.TopCenter))
                }
                state.error?.let { ErrorBanner(it, model::clearError, Modifier.align(Alignment.BottomCenter), retry = { model.retry(isTv) }, isTv = isTv) }
            }
        }
    }

    BackHandler(enabled = section == Section.Settings) { section = Section.Home }
}

@Composable
private fun TvHomeFlow(
    state: AppState,
    compactHeight: Boolean,
    model: MovoViewModel,
    settings: AppSettings,
    isDark: Boolean,
) {
    var section by rememberSaveable { mutableStateOf(Section.Home) }
    val focusMemory = rememberSaveable(saver = TvFocusMemory.Saver) { TvFocusMemory() }
    val stateHolder = rememberSaveableStateHolder()
    val destinationKey = if (section == Section.Settings) "settings" else state.tab.name
    focusMemory.destination = destinationKey
    focusMemory.fallback = state.focusedUrl

    val tvColorScheme = MaterialTheme.colorScheme.toTvColorScheme(isDark)
    TvMaterialTheme(colorScheme = tvColorScheme) {
        TvNavigationDrawer(
            selectedTab = state.tab,
            settingsSelected = section == Section.Settings,
            notificationCount = state.notificationCount,
            selectTab = { tab ->
                section = Section.Home
                if (state.tab != tab) model.selectTab(tab, true)
            },
            openSettings = {
                section = Section.Settings
            },
        ) {
            Box(
                Modifier
                    .fillMaxSize()
                    .focusRestorer()
                    .focusGroup(),
            ) {
                stateHolder.SaveableStateProvider(destinationKey) {
                    CompositionLocalProvider(LocalTvFocusMemory provides focusMemory) {
                        when (section) {
                            Section.Home -> HomeTabContent(state.tab, state, true, compactHeight, model)
                            Section.Settings -> SettingsDestination(settings, true)
                        }
                    }
                }
                if (state.loading) {
                    LinearProgressIndicator(Modifier.fillMaxWidth().align(Alignment.TopCenter))
                }
                state.error?.let {
                    ErrorBanner(
                        it,
                        model::clearError,
                        Modifier.align(Alignment.BottomCenter),
                        retry = { model.retry(true) },
                        isTv = true,
                    )
                }
            }
        }
    }

    BackHandler(enabled = section == Section.Settings) { section = Section.Home }
}

@Composable
internal fun TvNavigationDrawer(
    selectedTab: Tab,
    settingsSelected: Boolean,
    notificationCount: Int,
    selectTab: (Tab) -> Unit,
    openSettings: () -> Unit,
    content: @Composable () -> Unit,
) {
    NavigationDrawer(
        modifier = Modifier.testTag("tv-navigation-drawer"),
        drawerContent = { drawerValue ->
            val expanded = drawerValue == DrawerValue.Open
            Column(
                Modifier
                    .fillMaxHeight()
                    .padding(horizontal = 12.dp, vertical = 20.dp)
                    .selectableGroup(),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                Tab.entries.filter { it != Tab.Account }.forEach { tab ->
                    NavigationDrawerItem(
                        selected = !settingsSelected && selectedTab == tab,
                        onClick = { selectTab(tab) },
                        leadingContent = { TvIcon(tab.icon, contentDescription = stringResource(tab.label)) },
                        modifier = Modifier.testTag("tv-drawer-${tab.name.lowercase()}"),
                        trailingContent = if (
                            expanded && tab == Tab.Notifications && notificationCount > 0
                        ) {
                            { Badge { Text(notificationCount.coerceAtMost(99).toString()) } }
                        } else {
                            null
                        },
                        scale = NavigationDrawerItemScale.None,
                    ) {
                        if (expanded) TvText(stringResource(tab.label))
                    }
                }
                Spacer(Modifier.weight(1f))
                NavigationDrawerItem(
                    selected = !settingsSelected && selectedTab == Tab.Account,
                    onClick = { selectTab(Tab.Account) },
                    leadingContent = { TvIcon(Tab.Account.icon, contentDescription = stringResource(Tab.Account.label)) },
                    modifier = Modifier.testTag("tv-drawer-account"),
                    scale = NavigationDrawerItemScale.None,
                ) {
                    if (expanded) TvText(stringResource(Tab.Account.label))
                }
                NavigationDrawerItem(
                    selected = settingsSelected,
                    onClick = openSettings,
                    leadingContent = { TvIcon(Icons.Default.Settings, contentDescription = stringResource(R.string.settings_title)) },
                    modifier = Modifier.testTag("tv-drawer-settings"),
                    scale = NavigationDrawerItemScale.None,
                ) {
                    if (expanded) TvText(stringResource(R.string.settings_title))
                }
            }
        },
        content = content,
    )
}

@Composable
private fun SettingsDestination(settings: AppSettings, isTv: Boolean) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val actions = remember(context, scope) {
        object : SettingsActions {
            override fun <T> save(key: Preferences.Key<T>, value: T) {
                scope.launch { context.save(key, value) }
            }
        }
    }
    SettingsContent(settings = settings, isTv = isTv, actions = actions)
}

@Composable
private fun HomeTopAppBar(state: AppState, model: MovoViewModel, isTv: Boolean, onOpenSettings: () -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    var confirmLogout by remember { mutableStateOf(false) }
    TopAppBar(
        title = { Text(stringResource(R.string.app_name)) },
        actions = {
            state.user?.let {
                Text(
                    text = it.username,
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(end = 8.dp),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            IconButton(onClick = onOpenSettings, modifier = Modifier.tvFocusScale(isTv)) {
                Icon(Icons.Default.Settings, contentDescription = stringResource(R.string.settings_title))
            }
            IconButton(onClick = { expanded = true }, modifier = Modifier.tvFocusScale(isTv)) {
                Icon(imageVector = Icons.Default.MoreVert, contentDescription = stringResource(R.string.menu))
            }
            DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
                DropdownMenuItem(
                    onClick = { expanded = false; confirmLogout = true },
                    text = { Text(stringResource(R.string.menu_sign_out)) },
                )
            }
        },
    )
    if (confirmLogout) ConfirmLogoutDialog(isTv, { confirmLogout = false }) {
        confirmLogout = false
        model.logout()
    }
}

@Composable
internal fun ConfirmLogoutDialog(isTv: Boolean, dismiss: () -> Unit, confirm: () -> Unit) {
    AlertDialog(
        onDismissRequest = dismiss,
        title = { Text(stringResource(R.string.sign_out_title)) },
        text = { Text(stringResource(R.string.sign_out_message)) },
        confirmButton = {
            if (isTv) TvButton(onClick = confirm) { TvText(stringResource(R.string.menu_sign_out)) }
            else TextButton(confirm) { Text(stringResource(R.string.menu_sign_out)) }
        },
        dismissButton = {
            if (isTv) {
                TvButton(
                    onClick = dismiss,
                    modifier = Modifier.tvInitialFocus().testTag("tv-dialog-cancel"),
                ) {
                    TvText(stringResource(R.string.cancel))
                }
            } else {
                TextButton(dismiss) { Text(stringResource(R.string.cancel)) }
            }
        },
    )
}

/**
 * The four destinations the bottom bar shows outright. The bar holds five items before the
 * labels stop fitting a compact width, so the rest sit behind the fifth.
 */
private val PRIMARY_TABS = listOf(Tab.Catalog, Tab.Search, Tab.Favorites, Tab.Account)
private val OVERFLOW_TABS = Tab.entries - PRIMARY_TABS.toSet()

@Composable
private fun HomeBottomBar(state: AppState, model: MovoViewModel) {
    var showOverflow by remember { mutableStateOf(false) }
    // Only counts while Notifications is out of sight; on the bar it carries its own badge.
    val hiddenNotifications = if (Tab.Notifications in OVERFLOW_TABS) state.notificationCount else 0
    NavigationBar {
        PRIMARY_TABS.forEach { tab ->
            NavigationBarItem(
                selected = state.tab == tab,
                onClick = { if (state.tab != tab) model.selectTab(tab, false) },
                icon = { TabIcon(tab, state.notificationCount) },
                label = { Text(stringResource(tab.label)) },
            )
        }
        NavigationBarItem(
            selected = state.tab in OVERFLOW_TABS,
            onClick = { showOverflow = true },
            icon = { BadgedIcon(Icons.Default.MoreHoriz, hiddenNotifications) },
            label = { Text(stringResource(R.string.nav_more)) },
        )
    }
    if (showOverflow) {
        ModalBottomSheet(onDismissRequest = { showOverflow = false }) {
            OVERFLOW_TABS.forEach { tab ->
                ListItem(
                    headlineContent = { Text(stringResource(tab.label)) },
                    leadingContent = {
                        BadgedIcon(tab.icon, if (tab == Tab.Notifications) state.notificationCount else 0)
                    },
                    modifier = Modifier.selectable(
                        selected = state.tab == tab,
                        onClick = {
                            showOverflow = false
                            if (state.tab != tab) model.selectTab(tab, false)
                        },
                    ),
                )
            }
            Spacer(Modifier.height(24.dp))
        }
    }
}

@Composable
private fun HomeNavigationRail(
    state: AppState,
    model: MovoViewModel,
    isTv: Boolean,
    openSettings: () -> Unit,
) {
    val tabs = Tab.entries
    NavigationRail {
        Spacer(Modifier.height(12.dp))
        tabs.forEach { tab ->
            MovoNavigationRailItem(
                selected = state.tab == tab,
                onClick = { if (state.tab != tab) model.selectTab(tab, isTv) },
                icon = { TabIcon(tab, state.notificationCount) },
                label = { Text(stringResource(tab.label)) },
                isTv = isTv,
            )
        }
        MovoNavigationRailItem(
            selected = false,
            onClick = openSettings,
            icon = { Icon(Icons.Default.Settings, contentDescription = null) },
            label = { Text(stringResource(R.string.settings_title)) },
            isTv = isTv,
        )
    }
}

@Composable
private fun MovoNavigationRailItem(
    selected: Boolean,
    onClick: () -> Unit,
    icon: @Composable () -> Unit,
    label: @Composable () -> Unit,
    isTv: Boolean,
    modifier: Modifier = Modifier,
) {
    val interactionSource = remember { MutableInteractionSource() }
    val focused by interactionSource.collectIsFocusedAsState()
    val tvFocused = isTv && focused
    val scale by animateFloatAsState(
        targetValue = if (tvFocused) 1.06f else 1f,
        animationSpec = motionSpec(spring()),
        label = "navigation focus",
    )
    NavigationRailItem(
        selected = selected,
        onClick = onClick,
        icon = icon,
        label = label,
        modifier = modifier
            .padding(horizontal = 6.dp, vertical = 2.dp)
            .background(
                if (tvFocused) MaterialTheme.colorScheme.primaryContainer else Color.Transparent,
                RoundedCornerShape(20.dp),
            )
            .graphicsLayer { scaleX = scale; scaleY = scale },
        colors = NavigationRailItemDefaults.colors(
            selectedIconColor = if (tvFocused) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSecondaryContainer,
            selectedTextColor = if (tvFocused) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurface,
            indicatorColor = if (tvFocused) Color.Transparent else MaterialTheme.colorScheme.secondaryContainer,
            unselectedIconColor = if (tvFocused) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurfaceVariant,
            unselectedTextColor = if (tvFocused) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurfaceVariant,
        ),
        interactionSource = interactionSource,
    )
}

@Composable
private fun TabIcon(tab: Tab, notificationCount: Int) =
    BadgedIcon(tab.icon, if (tab == Tab.Notifications) notificationCount else 0)

@Composable
private fun BadgedIcon(icon: ImageVector, count: Int) {
    if (count > 0) {
        BadgedBox(badge = { Badge { Text(count.coerceAtMost(99).toString()) } }) {
            Icon(icon, contentDescription = null)
        }
    } else Icon(icon, contentDescription = null)
}

@Composable
private fun HomeTabs(
    state: AppState,
    isTv: Boolean,
    compactHeight: Boolean,
    model: MovoViewModel,
) {
    if (isTv) {
        HomeTabContent(state.tab, state, true, compactHeight, model)
        return
    }
    // Directional slide: forward for tabs later in the list, back for earlier ones, so
    // navigation reads spatially instead of a flat cross-fade.
    val reducedMotion = LocalReducedMotion.current
    AnimatedContent(
        targetState = state.tab,
        transitionSpec = {
            if (reducedMotion) return@AnimatedContent EnterTransition.None togetherWith ExitTransition.None
            val forward = targetState.ordinal >= initialState.ordinal
            val enter = fadeIn(tween(220)) +
                slideInHorizontally(tween(260)) { if (forward) it / 8 else -it / 8 }
            val exit = fadeOut(tween(180)) +
                slideOutHorizontally(tween(260)) { if (forward) -it / 8 else it / 8 }
            enter togetherWith exit
        },
        label = "home tabs",
    ) { tab ->
        HomeTabContent(tab, state, false, compactHeight, model)
    }
}

@Composable
private fun HomeTabContent(
    tab: Tab,
    state: AppState,
    isTv: Boolean,
    compactHeight: Boolean,
    model: MovoViewModel,
) {
    when (tab) {
        Tab.Catalog -> CatalogScreen(state, isTv, compactHeight, model)
        Tab.Search -> SearchScreen(state, isTv, model)
        Tab.Collections -> CollectionsScreen(state, isTv, model)
        Tab.Favorites -> FavoritesScreen(state, isTv, model)
        Tab.History -> HistoryScreen(state, isTv, model)
        Tab.Notifications -> NotificationsScreen(state, isTv, model)
        Tab.Account -> AccountScreen(state, isTv, model)
    }
}

private val Tab.icon get() = when (this) {
    Tab.Catalog -> Icons.Default.GridView
    Tab.Search -> Icons.Default.Search
    Tab.Collections -> Icons.Default.CollectionsBookmark
    Tab.Favorites -> Icons.Default.Favorite
    Tab.History -> Icons.Default.History
    Tab.Notifications -> Icons.Default.Notifications
    Tab.Account -> Icons.Default.AccountCircle
}

private val Tab.label: Int get() = when (this) {
    Tab.Catalog -> R.string.nav_catalog
    Tab.Search -> R.string.nav_search
    Tab.Collections -> R.string.nav_collections
    Tab.Favorites -> R.string.nav_favorites
    Tab.History -> R.string.nav_history
    Tab.Notifications -> R.string.nav_notifications
    Tab.Account -> R.string.nav_account
}
