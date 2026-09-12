package org.movo.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/** The activity is exported, so any app can hand it a link; only a title page is opened. */
class DeepLinkTest {

    @Test
    fun aTitlePageOpens() {
        val path = "/series/thriller/646-vo-vse-tyazhkie-2008.html"

        assertEquals("https://hdrzk.org$path", titlePageUrl("https", "hdrzk.org", path))
    }

    @Test
    fun anythingElseOnTheProviderIsTurnedAway() {
        assertNull(titlePageUrl("https", "hdrzk.org", "/index.php"))
        assertNull(titlePageUrl("https", "hdrzk.org", "/engine/ajax/rating.php"))
        assertNull(titlePageUrl("https", "hdrzk.org", "/films/../engine/ajax/1-rating.html"))
        assertNull(titlePageUrl("https", "hdrzk.org", "/films/drama/1-a.html/"))
        assertNull(titlePageUrl("https", "hdrzk.org", null))
    }

    @Test
    fun aTitlePageElsewhereIsTurnedAway() {
        assertNull(titlePageUrl("http", "hdrzk.org", "/films/drama/1-a.html"))
        assertNull(titlePageUrl("https", "example.com", "/films/drama/1-a.html"))
    }
}
