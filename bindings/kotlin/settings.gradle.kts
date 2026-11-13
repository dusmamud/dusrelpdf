// dusrelpdf — Android Gradle project.
//
// One module, :dusrelpdf, an Android library (AAR) that wraps the
// UniFFI-generated Kotlin bindings in ../uniffi/ (the single source of
// truth — see bindings/kotlin/README.md). Build with ./gradlew from this
// directory; requires JDK 17 and an Android SDK (API 34).

pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "dusrelpdf"
include(":dusrelpdf")
