package org.movo.app

import android.app.Application
import android.app.UiModeManager
import android.content.Context
import android.content.res.Configuration
import coil3.ImageLoader
import coil3.PlatformContext
import coil3.SingletonImageLoader
import coil3.request.crossfade

/**
 * Configures the image loader every screen shares. Posters arrive from the provider's CDN at
 * unpredictable speed, so they fade in rather than snapping over the placeholder tile the cards
 * draw behind them.
 *
 * Not on a television: the fade blends a full poster per frame for as long as it runs, and a rail
 * brings six or seven of them into view at once on hardware that has no headroom for it. The
 * cards already show a placeholder tile, so without the fade the poster simply replaces it.
 */
class MovoApplication : Application(), SingletonImageLoader.Factory {
    override fun newImageLoader(context: PlatformContext): ImageLoader =
        ImageLoader.Builder(context)
            .crossfade(!isTelevision())
            .build()

    private fun isTelevision() =
        (getSystemService(Context.UI_MODE_SERVICE) as UiModeManager).currentModeType ==
            Configuration.UI_MODE_TYPE_TELEVISION
}
