# The Rust core is reached over JNI, so the bridge class, its native method names and the
# JNI entry point signature must survive shrinking and obfuscation.
-keep class org.movo.app.core.NativeBridge { *; }
-keepclasseswithmembernames class * {
    native <methods>;
}

# kotlinx.serialization resolves serializers reflectively through the generated companions.
-keepattributes *Annotation*, InnerClasses
-keepclassmembers class org.movo.app.** {
    *** Companion;
}
-keepclasseswithmembers class org.movo.app.** {
    kotlinx.serialization.KSerializer serializer(...);
}
-keep,includedescriptorclasses class org.movo.app.**$$serializer { *; }
