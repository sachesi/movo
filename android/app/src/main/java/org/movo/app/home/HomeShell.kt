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
import org.movo.app.ui.tvInitialFocus
import org.movo.app.ui.LocalReducedMotion
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
import org.movo.app.ui.TV_OVERSCAN_VERTICAL
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
import androidx.compose.animation.core.tween
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.foundation.background
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.requiredWidth
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.layout
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.unit.Dp
import androidx.compose.animation.core.VectorConverter
import org.movo.app.ui.motionSpec
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.focusGroup
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.windowsizeclass.ExperimentalMaterial3WindowSizeClassApi
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
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
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.focusRestorer
import androidx.compose.runtime.withFrameNanos
import android.content.res.Configuration
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import org.movo.app.ui.TvButton
import androidx.tv.material3.ExperimentalTvMaterial3Api
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
) {
    if (isTv) {
        TvHomeFlow(state, compactHeight, model, settings)
        return
    }
    var section by rememberSaveable { mutableStateOf(Section.Home) }
    val scrollBehavior = TopAppBarDefaults.enterAlwaysScrollBehavior()
    // A pull re-runs the visible tab's load. Tracked apart from `loading`, which every
    // operation raises, so the indicator only shows for a refresh the user asked for.
    var pulled by remember { mutableStateOf(false) }
    LaunchedEffect(state.loading) { if (!state.loading) pulled = false }

    Scaffold(
        // Only while the tabs are up: the settings bar does not follow the scroll, and a bar left
        // collapsed under Settings would come back hidden.
        modifier = if (section == Section.Home) Modifier.nestedScroll(scrollBehavior.nestedScrollConnection) else Modifier,
        topBar = {
            if (section == Section.Home) {
                HomeTopAppBar(state, model, scrollBehavior) { section = Section.Settings }
            } else {
                CenterAlignedTopAppBar(
                    title = { Text(stringResource(R.string.settings_title)) },
                    navigationIcon = {
                        IconButton({ section = Section.Home }) {
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
            if (section == Section.Home && useRail) HomeNavigationRail(state, model) { section = Section.Settings }
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
                        Section.Home -> PullToRefreshBox(
                            isRefreshing = pulled && state.loading,
                            onRefresh = { pulled = true; model.retry(false) },
                        ) {
                            HomeTabs(state, compactHeight, model)
                        }
                        Section.Settings -> SettingsDestination(settings, false)
                    }
                }
                if (state.loading) {
                    LinearProgressIndicator(Modifier.fillMaxWidth().align(Alignment.TopCenter))
                }
                state.error?.let {
                    ErrorBanner(it, model::clearError, Modifier.align(Alignment.BottomCenter), retry = { model.retry(false) })
                }
            }
        }
    }

    // Back returns to the initial tab before it leaves the app. Composed after the tabs, so it
    // must stand aside while a collection has its own way back.
    BackHandler(enabled = section == Section.Home && state.collectionPath == null && state.tab != settings.initialTab) {
        model.selectTab(settings.initialTab, false)
    }
    BackHandler(enabled = section == Section.Settings) { section = Section.Home }
}

@Composable
private fun TvHomeFlow(
    state: AppState,
    compactHeight: Boolean,
    model: MovoViewModel,
    settings: AppSettings,
) {
    var section by rememberSaveable { mutableStateOf(Section.Home) }
    val focusMemory = rememberSaveable(saver = TvFocusMemory.Saver) { TvFocusMemory() }
    val stateHolder = rememberSaveableStateHolder()
    // A collection opened from a tab is its own destination: its cards must not overwrite the
    // record of where the user left the tab's own grid.
    val destinationKey = when {
        section == Section.Settings -> "settings"
        state.collectionPath != null -> "${state.tab.name}:${state.collectionPath}"
        else -> state.tab.name
    }
    focusMemory.destination = destinationKey
    focusMemory.fallback = state.focusedUrl
    // The content's focus group. Entered by hand in two cases the memory does not cover: a
    // destination nothing restores itself on (the first visit, or the first launch), which
    // otherwise left the highlight in the drawer or nowhere; and the error banner going away,
    // which otherwise dropped it. The search page is left alone: its first element is the text
    // field, and landing there would open the keyboard on every visit.
    val contentFocus = remember { FocusRequester() }
    val contentFocused = remember { mutableStateOf(false) }
    LaunchedEffect(destinationKey) {
        // One frame: the page lays out and restores its own focus first.
        withFrameNanos {}
        if (!contentFocused.value && state.tab != Tab.Search) runCatching { contentFocus.requestFocus() }
    }
    val clearError = { model.clearError(); runCatching { contentFocus.requestFocus() }; Unit }

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
                .focusRequester(contentFocus)
                .onFocusChanged { contentFocused.value = it.hasFocus }
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
                    clearError,
                    Modifier.align(Alignment.BottomCenter),
                    retry = { model.retry(true) },
                    isTv = true,
                )
            }
        }
    }

    BackHandler(enabled = section == Section.Home && state.collectionPath == null && state.tab != settings.initialTab) {
        model.selectTab(settings.initialTab, true)
    }
    BackHandler(enabled = section == Section.Settings) { section = Section.Home }
}

private val Configuration.isTelevision
    get() = uiMode and Configuration.UI_MODE_TYPE_MASK == Configuration.UI_MODE_TYPE_TELEVISION

// Icon rail: safe-area padding, item padding, icon, item padding, safe-area padding.
private val RAIL_PADDING = 24.dp
private val RAIL_WIDTH = RAIL_PADDING * 2 + 14.dp * 2 + 24.dp
private val LABELLED_RAIL_PADDING = 6.dp
private val LABELLED_RAIL_WIDTH = 84.dp
private val PANEL_WIDTH = 260.dp
private val DRAWER_ITEM_HEIGHT = 48.dp
private val LABELLED_ITEM_HEIGHT = 64.dp

/**
 * The television navigation sheet: a rail of icons that widens over the content, on focus, into
 * icons with labels.
 *
 * Written out rather than taken from tv-material, whose two drawers each cost more than what they
 * save here. The side-by-side one sizes its sheet with `animateContentSize`, which hands the
 * content beside it a new width on every frame of the widen and re-measures the whole destination
 * — the rails, the grid, all of it — a dozen-odd times for each move into and out of the drawer,
 * which is the most frequent thing a remote does. The modal one lays the sheet over the content as
 * this does, but paints nothing behind it, so over a wall of posters the labels have no ground to
 * sit on.
 *
 * What is left measures the content once, at a width that never changes, and animates the sheet
 * alone. The column inside the sheet is held at [PANEL_WIDTH] whatever the sheet is currently
 * showing, so the icons never move and the labels never re-wrap while it opens: the widening only
 * uncovers what was already laid out. The width is read in the layout phase, so the animation
 * re-measures the sheet without recomposing any of it.
 *
 * [labelled] is the form for a tablet or phone on this layout: there is no moment at which focus
 * "enters" the rail, so it would never widen and the labels would never show. Instead the rail is
 * a little wider, never widens, and stacks each label under its icon, the way a navigation rail
 * does. Decided by the device's mode rather than its touch screen: a television box that reports
 * a touch screen it does not have was getting the stacked rail, with Settings below the fold.
 */
@Composable
internal fun TvNavigationDrawer(
    selectedTab: Tab,
    settingsSelected: Boolean,
    notificationCount: Int,
    selectTab: (Tab) -> Unit,
    openSettings: () -> Unit,
    labelled: Boolean = !LocalConfiguration.current.isTelevision,
    content: @Composable () -> Unit,
) {
    val railWidth = if (labelled) LABELLED_RAIL_WIDTH else RAIL_WIDTH
    // The content starts under the rail's own inner padding: every page adds the overscan margin
    // itself, and measured from the rail's edge that put 72dp of blank between the icons and
    // the first card against 48dp on the right.
    val contentStart = railWidth - if (labelled) LABELLED_RAIL_PADDING else RAIL_PADDING
    // Inside the safe area: a television reports none, but a phone or tablet on this layout has
    // a status bar and a camera cutout, and without this the clock sat over the rail.
    Box(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing).testTag("tv-navigation-drawer")) {
        Box(Modifier.fillMaxSize().padding(start = contentStart)) { content() }
        TvDrawerSheet(selectedTab, settingsSelected, notificationCount, labelled, selectTab, openSettings)
    }
}

@Composable
private fun TvDrawerSheet(
    selectedTab: Tab,
    settingsSelected: Boolean,
    notificationCount: Int,
    labelled: Boolean,
    selectTab: (Tab) -> Unit,
    openSettings: () -> Unit,
) {
    val railWidth = if (labelled) LABELLED_RAIL_WIDTH else RAIL_WIDTH
    var expanded by remember { mutableStateOf(false) }
    val width = remember { Animatable(railWidth, Dp.VectorConverter) }
    val motion = motionSpec<Dp>(tween(durationMillis = 200, easing = FastOutSlowInEasing))
    LaunchedEffect(expanded) { width.animateTo(if (expanded) PANEL_WIDTH else railWidth, motion) }

    Box(
        Modifier
            .fillMaxHeight()
            .onFocusChanged { if (!labelled) expanded = it.hasFocus }
            .focusGroup()
            // Opaque: the sheet is laid over the content, so whatever it covers has to stop
            // showing through it.
            .background(MaterialTheme.colorScheme.surface)
            .clipToBounds()
            .layout { measurable, constraints ->
                val sheet = measurable.measure(constraints)
                val visible = width.value.roundToPx().coerceIn(0, sheet.width)
                layout(visible, sheet.height) { sheet.place(0, 0) }
            },
    ) {
        Column(
            Modifier
                .requiredWidth(if (labelled) railWidth else PANEL_WIDTH)
                .fillMaxHeight()
                // Taller items than a phone in landscape has room for, so the labelled rail scrolls.
                .then(if (labelled) Modifier.verticalScroll(rememberScrollState()) else Modifier)
                // Inside the television safe area, as the content beside it is: a set that crops
                // the outer five percent was cutting into the first icon and the selection block.
                .padding(horizontal = if (labelled) LABELLED_RAIL_PADDING else RAIL_PADDING, vertical = if (labelled) 20.dp else TV_OVERSCAN_VERTICAL)
                .selectableGroup(),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Tab.entries.filter { it != Tab.Account }.forEach { tab ->
                TvDrawerItem(
                    tab = tab,
                    expanded = expanded,
                    labelled = labelled,
                    selected = !settingsSelected && selectedTab == tab,
                    badge = if (tab == Tab.Notifications) notificationCount else 0,
                    testTag = "tv-drawer-${tab.name.lowercase()}",
                ) { selectTab(tab) }
            }
            if (labelled) Spacer(Modifier.height(12.dp)) else Spacer(Modifier.weight(1f))
            TvDrawerItem(
                tab = Tab.Account,
                expanded = expanded,
                labelled = labelled,
                selected = !settingsSelected && selectedTab == Tab.Account,
                testTag = "tv-drawer-account",
            ) { selectTab(Tab.Account) }
            TvDrawerItem(
                icon = Icons.Default.Settings,
                label = stringResource(R.string.settings_title),
                expanded = expanded,
                labelled = labelled,
                selected = settingsSelected,
                testTag = "tv-drawer-settings",
                onClick = openSettings,
            )
        }
    }
}

@Composable
private fun TvDrawerItem(
    tab: Tab,
    expanded: Boolean,
    labelled: Boolean,
    selected: Boolean,
    testTag: String,
    badge: Int = 0,
    onClick: () -> Unit,
) = TvDrawerItem(tab.icon, stringResource(tab.label), expanded, labelled, selected, testTag, onClick, badge)

@Composable
private fun TvDrawerItem(
    icon: ImageVector,
    label: String,
    expanded: Boolean,
    labelled: Boolean,
    selected: Boolean,
    testTag: String,
    onClick: () -> Unit,
    badge: Int = 0,
) {
    var focused by remember { mutableStateOf(false) }
    // A tap takes focus as well, so the highlight follows the finger as it follows the remote.
    val focusRequester = remember { FocusRequester() }
    val colors = MaterialTheme.colorScheme
    val container = when {
        focused -> colors.onSurface
        selected -> colors.surfaceVariant
        else -> Color.Transparent
    }
    val item = Modifier
        .fillMaxWidth()
        .clip(MaterialTheme.shapes.small)
        .background(container)
        .focusRequester(focusRequester)
        .onFocusChanged { focused = it.isFocused }
        .selectable(selected = selected) {
            runCatching { focusRequester.requestFocus() }
            onClick()
        }
        // The label is not composed while the rail is narrow, so the row names itself for a
        // screen reader until it is.
        .then(if (expanded || labelled) Modifier else Modifier.semantics { contentDescription = label })
        .testTag(testTag)
    CompositionLocalProvider(
        LocalContentColor provides if (focused) colors.surface else colors.onSurface,
    ) {
        if (labelled) {
            Column(
                item.height(LABELLED_ITEM_HEIGHT),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center,
            ) {
                BadgedIcon(icon, badge)
                Spacer(Modifier.height(2.dp))
                Text(
                    label,
                    style = MaterialTheme.typography.labelSmall,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    textAlign = TextAlign.Center,
                )
            }
        } else {
            Row(
                item.height(DRAWER_ITEM_HEIGHT).padding(horizontal = 14.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                BadgedIcon(icon, badge)
                // Composed only once the sheet is open: a label the rail is too narrow to show is
                // still a label a screen reader would read out and a test would find.
                if (expanded) {
                    Text(
                        label,
                        style = MaterialTheme.typography.titleSmall,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
        }
    }
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
private fun HomeTopAppBar(
    state: AppState,
    model: MovoViewModel,
    scrollBehavior: TopAppBarScrollBehavior,
    onOpenSettings: () -> Unit,
) {
    TopAppBar(
        title = { Text(stringResource(R.string.app_name)) },
        scrollBehavior = scrollBehavior,
        actions = {
            // The name is the way to the account page, which also holds sign-out.
            state.user?.let {
                TextButton(
                    onClick = { if (state.tab != Tab.Account) model.selectTab(Tab.Account, false) },
                    modifier = Modifier.widthIn(max = 200.dp),
                ) {
                    Icon(Icons.Default.AccountCircle, contentDescription = null)
                    Spacer(Modifier.width(6.dp))
                    Text(
                        text = it.username,
                        style = MaterialTheme.typography.titleMedium,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
            IconButton(onClick = onOpenSettings) {
                Icon(Icons.Default.Settings, contentDescription = stringResource(R.string.settings_title))
            }
        },
    )
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
 * labels stop fitting a compact width, so the rest sit behind the fifth. The account page is
 * reached from the user's name in the top bar instead.
 */
internal val PRIMARY_TABS = listOf(Tab.Catalog, Tab.Search, Tab.Favorites, Tab.History)
internal val OVERFLOW_TABS = Tab.entries - PRIMARY_TABS.toSet() - Tab.Account

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
                    trailingContent = if (state.tab == tab) {
                        { Icon(Icons.Default.Check, contentDescription = null) }
                    } else {
                        null
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
    openSettings: () -> Unit,
) {
    // Scrolls: seven tabs and Settings are taller than a landscape phone, which uses the rail
    // because its width is not compact, and a fixed column clipped the lower items off.
    NavigationRail(Modifier.fillMaxHeight().verticalScroll(rememberScrollState())) {
        Spacer(Modifier.height(12.dp))
        Tab.entries.forEach { tab ->
            NavigationRailItem(
                selected = state.tab == tab,
                onClick = { if (state.tab != tab) model.selectTab(tab, false) },
                icon = { TabIcon(tab, state.notificationCount) },
                label = { Text(stringResource(tab.label)) },
                modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
            )
        }
        NavigationRailItem(
            selected = false,
            onClick = openSettings,
            icon = { Icon(Icons.Default.Settings, contentDescription = null) },
            label = { Text(stringResource(R.string.settings_title)) },
            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
        )
    }
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
    compactHeight: Boolean,
    model: MovoViewModel,
) {
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

internal val Tab.label: Int get() = when (this) {
    Tab.Catalog -> R.string.nav_catalog
    Tab.Search -> R.string.nav_search
    Tab.Collections -> R.string.nav_collections
    Tab.Favorites -> R.string.nav_favorites
    Tab.History -> R.string.nav_history
    Tab.Notifications -> R.string.nav_notifications
    Tab.Account -> R.string.nav_account
}
