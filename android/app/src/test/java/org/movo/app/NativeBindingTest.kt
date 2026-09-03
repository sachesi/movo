package org.movo.app

import org.movo.app.core.NativeBridge
import org.junit.Assert.assertTrue
import org.junit.Assert.assertEquals
import org.junit.Test
import java.io.File
import java.lang.reflect.Modifier

/**
 * The Kotlin side and the Rust side of the JNI boundary agree on one name, and nothing else
 * checks it: a native declaration compiles against no library, so moving the class to another
 * package or the method to another class builds cleanly and then dies with an
 * UnsatisfiedLinkError on the first request the app makes. That is what this holds down.
 */
class NativeBindingTest {

    /** `Java_` + the class name with dots as underscores, + the method name. */
    private fun jniSymbol(owner: Class<*>, method: String): String {
        fun escape(part: String) = part.replace("_", "_1")
        return "Java_" + owner.name.split(".").joinToString("_") { escape(it) } + "_" + escape(method)
    }

    private fun repoRoot(): File {
        var dir = File(System.getProperty("user.dir")!!).absoluteFile
        while (!File(dir, "crates/movo-android/src/lib.rs").isFile) {
            dir = dir.parentFile ?: error("no repository root above ${System.getProperty("user.dir")}")
        }
        return dir
    }

    @Test
    fun theCoreExportsTheSymbolTheBridgeDeclares() {
        val natives = NativeBridge::class.java.declaredMethods.filter { Modifier.isNative(it.modifiers) }
        assertEquals("the bridge declares exactly one native method", 1, natives.size)

        val expected = jniSymbol(NativeBridge::class.java, natives.single().name)
        val rust = File(repoRoot(), "crates/movo-android/src/lib.rs").readText()

        assertTrue(
            "crates/movo-android/src/lib.rs exports no `$expected`. The Kotlin declaration and " +
                "the Rust symbol have to name the same class, and one of them moved.",
            rust.contains("fn $expected("),
        )
    }
}
