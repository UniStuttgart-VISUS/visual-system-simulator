import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import org.gradle.api.tasks.Sync

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

val catalogPresetAssets = layout.buildDirectory.dir("generated/catalogPresetAssets")
val syncCatalogPresetAssets by tasks.registering(Sync::class) {
    from(layout.projectDirectory.dir("../../vss-catalog/presets")) {
        include("**/*.png")
    }
    into(catalogPresetAssets.map { it.dir("presets") })
}

data class RustTarget(
    val abi: String,
    val triple: String,
    val linker: String,
    val linkerEnvironmentVariable: String,
)

val rustTargets = listOf(
    RustTarget(
        abi = "armeabi-v7a",
        triple = "armv7-linux-androideabi",
        linker = "armv7a-linux-androideabi31-clang",
        linkerEnvironmentVariable = "CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER",
    ),
    RustTarget(
        abi = "arm64-v8a",
        triple = "aarch64-linux-android",
        linker = "aarch64-linux-android31-clang",
        linkerEnvironmentVariable = "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER",
    ),
    RustTarget(
        abi = "x86",
        triple = "i686-linux-android",
        linker = "i686-linux-android31-clang",
        linkerEnvironmentVariable = "CARGO_TARGET_I686_LINUX_ANDROID_LINKER",
    ),
    RustTarget(
        abi = "x86_64",
        triple = "x86_64-linux-android",
        linker = "x86_64-linux-android31-clang",
        linkerEnvironmentVariable = "CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER",
    ),
)

val rustLibraryName = "libvss_android.so"
val androidNdkVersion = "28.2.13676358"
val debugRustAbi = providers.gradleProperty("rustDebugAbi").orElse("arm64-v8a").get()
val hostOs = System.getProperty("os.name").lowercase()
val ndkHostTag = when {
    hostOs.contains("windows") -> "windows-x86_64"
    hostOs.contains("mac") -> "darwin-x86_64"
    hostOs.contains("linux") -> "linux-x86_64"
    else -> error("Unsupported Android NDK host: ${System.getProperty("os.name")} ${System.getProperty("os.arch")}")
}
val ndkExecutableSuffix = if (hostOs.contains("windows")) ".cmd" else ""

require(rustTargets.any { it.abi == debugRustAbi }) {
    "Unsupported rustDebugAbi '$debugRustAbi'. Expected one of: ${rustTargets.joinToString { it.abi }}"
}

android {
    namespace = "com.vss"
    compileSdk = 37
    ndkVersion = androidNdkVersion

    defaultConfig {
        applicationId = "com.vss"
        minSdk = 31
        targetSdk = 37
        versionCode = 2
        versionName = "2.0"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        buildConfig = true
        compose = true
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
    }

    sourceSets {
        getByName("main").assets.directories.add(
            layout.projectDirectory.dir("../../vss-catalog/articles").asFile.absolutePath,
        )
        getByName("main").assets.directories.add(catalogPresetAssets.get().asFile.absolutePath)
        getByName("debug").jniLibs.directories.add(
            layout.buildDirectory.dir("rustJniLibs/debug-$debugRustAbi").get().asFile.absolutePath,
        )
        getByName("release").jniLibs.directories.add(
            layout.buildDirectory.dir("rustJniLibs/release").get().asFile.absolutePath,
        )
    }
}

kotlin {
    compilerOptions {
        jvmTarget = JvmTarget.JVM_17
    }
}

dependencies {
    implementation(platform("androidx.compose:compose-bom:2026.05.00"))
    implementation("androidx.activity:activity-compose:1.12.3")
    implementation("androidx.core:core-ktx:1.17.0")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.10.0")
    testImplementation("junit:junit:4.13.2")
}

val repositoryDirectory = layout.projectDirectory.dir("../..")
val androidSdkDirectory = providers.environmentVariable("ANDROID_HOME")
    .orElse(providers.environmentVariable("ANDROID_SDK_ROOT"))
    .orElse(providers.systemProperty("user.home").map { "$it\\AppData\\Local\\Android\\Sdk" })
val ndkDirectory = androidSdkDirectory.map { file("$it\\ndk\\$androidNdkVersion") }

androidComponents {
    onVariants(selector().all()) { variant ->
        val variantName = variant.name
        val capitalizedVariantName = variantName.replaceFirstChar(Char::uppercaseChar)
        val cargoProfile = if (variant.buildType == "release") "release" else "debug"
        val includedTargets = if (variant.buildType == "release") {
            rustTargets
        } else {
            rustTargets.filter { it.abi == debugRustAbi }
        }

        val copyTasksByTarget = rustTargets.associateWith { rustTarget ->
            val capitalizedAbi = rustTarget.abi
                .split('-', '_')
                .joinToString("") { it.replaceFirstChar(Char::uppercaseChar) }
            val cargoTask = tasks.register<Exec>("cargoBuild$capitalizedVariantName$capitalizedAbi") {
                group = "rust"
                description = "Builds ${rustTarget.triple} Rust code for $variantName."
                workingDir(repositoryDirectory)
                commandLine(
                    "cargo",
                    "build",
                    "-p",
                    "vss-android",
                    "--target",
                    rustTarget.triple,
                )
                if (cargoProfile == "release") {
                    args("--release")
                }

                val linker = ndkDirectory.map {
                    File(it, "toolchains/llvm/prebuilt/$ndkHostTag/bin/${rustTarget.linker}$ndkExecutableSuffix")
                }
                val linkerFile = linker.get()
                environment(rustTarget.linkerEnvironmentVariable, linkerFile.absolutePath)
                inputs.file(linkerFile).withPropertyName("androidNdkLinker")

                inputs.files(
                    repositoryDirectory.file("Cargo.toml"),
                    repositoryDirectory.file("Cargo.lock"),
                    repositoryDirectory.file("vss/Cargo.toml"),
                    repositoryDirectory.file("vss-android/Cargo.toml"),
                    repositoryDirectory.file("vss-catalog/Cargo.toml"),
                    repositoryDirectory.file("vss-catalog/build.rs"),
                )
                inputs.dir(repositoryDirectory.dir("vss/src"))
                inputs.dir(repositoryDirectory.dir("vss-android/src"))
                inputs.dir(repositoryDirectory.dir("vss-catalog/src"))
                inputs.dir(repositoryDirectory.dir("vss-catalog/articles"))
                inputs.dir(repositoryDirectory.dir("vss-catalog/presets"))
                outputs.file(
                    repositoryDirectory.file(
                        "target/${rustTarget.triple}/$cargoProfile/$rustLibraryName",
                    ),
                )
            }

            tasks.register<Copy>("copyRust$capitalizedVariantName${capitalizedAbi}JniLib") {
                group = "rust"
                description = "Copies the ${rustTarget.abi} Rust library into $variantName JNI libs."
                dependsOn(cargoTask)
                from(
                    repositoryDirectory.file(
                        "target/${rustTarget.triple}/$cargoProfile/$rustLibraryName",
                    ),
                )
                val outputRoot = if (variant.buildType == "release") {
                    "rustJniLibs/release"
                } else {
                    "rustJniLibs/debug-${rustTarget.abi}"
                }
                into(layout.buildDirectory.dir("$outputRoot/${rustTarget.abi}"))
            }
        }

        val aggregateTask = tasks.register("cargoBuild$capitalizedVariantName") {
            group = "rust"
            description = "Builds all Rust libraries included in $variantName."
            dependsOn(includedTargets.map { copyTasksByTarget.getValue(it) })
        }
        tasks.matching { it.name == "merge${capitalizedVariantName}Assets" }.configureEach {
            dependsOn(aggregateTask, syncCatalogPresetAssets)
        }
        tasks.matching { it.name == "merge${capitalizedVariantName}JniLibFolders" }.configureEach {
            dependsOn(aggregateTask)
        }
    }
}
