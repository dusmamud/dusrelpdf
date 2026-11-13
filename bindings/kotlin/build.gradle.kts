// Root build file. Plugin versions live here (apply false); the :dusrelpdf
// module applies them without repeating versions.
//
// Pinned versions (known-good combination, Nov 2026):
//   Android Gradle Plugin 8.5.2  — requires Gradle >= 8.7 and JDK 17.
//   Kotlin 2.0.20               — compatible with AGP 8.5.x.
//   Gradle wrapper              — 8.7 (see gradle/wrapper/gradle-wrapper.properties).

plugins {
    id("com.android.library") version "8.5.2" apply false
    id("org.jetbrains.kotlin.android") version "2.0.20" apply false
}
