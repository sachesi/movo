@file:OptIn(ExperimentalTvMaterial3Api::class)

package org.movo.app

import androidx.compose.material3.MaterialTheme
import androidx.tv.material3.ExperimentalTvMaterial3Api
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.click
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.tv.material3.MaterialTheme as TvMaterialTheme
import androidx.tv.material3.Text as TvText
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.movo.app.ui.TvButton
import org.movo.app.ui.TvCard
import org.movo.app.ui.TvFilterChip
import org.robolectric.RobolectricTestRunner

/**
 * tv-material's surfaces answer the D-pad and the semantics click only. The wrappers add the
 * touch; this is what keeps a phone or tablet on the television layout from going dead.
 */
@RunWith(RobolectricTestRunner::class)
class TvTouchTest {
    @get:Rule val compose = createComposeRule()

    @Test
    fun theTelevisionControlsAnswerATouch() {
        var buttons = 0
        var chips = 0
        var cards = 0
        compose.setContent {
            MaterialTheme {
                TvMaterialTheme {
                    androidx.compose.foundation.layout.Column {
                        TvButton(onClick = { buttons++ }, Modifier.testTag("button")) { TvText("Button") }
                        TvFilterChip(selected = false, onClick = { chips++ }, Modifier.testTag("chip")) { TvText("Chip") }
                        TvCard(onClick = { cards++ }, Modifier.testTag("card")) { TvText("Card") }
                    }
                }
            }
        }

        compose.onNodeWithTag("button").performTouchInput { click() }
        compose.onNodeWithTag("chip").performTouchInput { click() }
        compose.onNodeWithTag("card").performTouchInput { click() }
        compose.runOnIdle {
            assertEquals(1, buttons)
            assertEquals(1, chips)
            assertEquals(1, cards)
        }
    }
}
