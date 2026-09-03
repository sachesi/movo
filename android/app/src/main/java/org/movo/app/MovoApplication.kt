package org.movo.app

import android.app.Application
import coil3.ImageLoader
import coil3.PlatformContext
import coil3.SingletonImageLoader
import coil3.request.crossfade

/**
 * Configures the image loader every screen shares. Posters arrive from the provider's CDN at
 * unpredictable speed, so they fade in rather than snapping over the placeholder tile the cards
 * draw behind them.
 */
class MovoApplication : Application(), SingletonImageLoader.Factory {
    override fun newImageLoader(context: PlatformContext): ImageLoader =
        ImageLoader.Builder(context)
            .crossfade(true)
            .build()
}
