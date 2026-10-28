plugins {
    kotlin("multiplatform")
    kotlin("plugin.serialization")
    id("com.android.library")
    id("org.jetbrains.compose")
    id("org.jetbrains.kotlin.plugin.compose")
}
kotlin {
    androidTarget { compilerOptions { jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17) } }
    jvm("desktop")
    jvmToolchain(21)
    sourceSets {
        commonMain.dependencies {
            implementation(compose.runtime)
            api(compose.foundation)
            implementation(compose.material3)
            api(compose.ui)
            implementation(compose.components.resources)
            implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")
            implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.9.0")
        }
        commonTest.dependencies { implementation(kotlin("test")) }
        androidMain { kotlin.srcDir("src/jvmMain/kotlin"); dependencies { implementation("org.mozilla:rhino:1.8.0"); implementation(project(":solver")) } }
        val desktopMain by getting { kotlin.srcDir("src/jvmMain/kotlin"); dependencies { implementation("org.mozilla:rhino:1.8.0"); implementation(project(":solver")) } }
        val desktopTest by getting { dependencies { implementation(kotlin("test-junit")) } }
    }
}
android {
    namespace = "lab.cube.shared"
    compileSdk = 36
    buildToolsVersion = "36.0.0"
    defaultConfig { minSdk = 26 }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
}
compose.resources { publicResClass = true; packageOfResClass = "lab.cube.resources" }
