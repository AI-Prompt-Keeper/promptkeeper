import java.util.Base64

plugins {
    id("java-library")
    id("maven-publish")
    id("signing")
    id("org.jetbrains.kotlin.jvm") version "1.9.22"
    id("org.jetbrains.kotlin.plugin.serialization") version "1.9.22"
    id("com.vanniktech.maven.publish") version "0.34.0"
}

group = "ai.promptkeeper"
// Version delegated to VERSION_NAME in gradle.properties
version = (project.findProperty("VERSION_NAME") as String?) ?: "0.0.1"

repositories {
    mavenCentral()
}

dependencies {
    implementation("org.jetbrains.kotlin:kotlin-stdlib")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.7.3")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.2")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    testImplementation("org.jetbrains.kotlin:kotlin-test-junit:1.9.22")
}

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
    withSourcesJar()
    withJavadocJar()
}

kotlin {
    jvmToolchain(17)
}

val versionName = (project.findProperty("VERSION_NAME") as String?) ?: version.toString()

mavenPublishing {
    publishToMavenCentral(automaticRelease = true)
    coordinates("ai.promptkeeper", "android-sdk", versionName)
    pom {
        name.set("PromptKeeper Android SDK")
        description.set("Kotlin/Android SDK for PromptKeeper API — init, setKey, setPrompt, exec (streaming).")
        url.set("https://github.com/AI-Prompt-Keeper/promptkeeper")
        inceptionYear.set("2025")
        licenses {
            license {
                name.set("The Apache License, Version 2.0")
                url.set("http://www.apache.org/licenses/LICENSE-2.0.txt")
                distribution.set("http://www.apache.org/licenses/LICENSE-2.0.txt")
            }
        }
        developers {
            developer {
                id.set("promptkeeper")
                name.set("PromptKeeper")
                email.set("dev@promptkeeper.ai")
            }
        }
        scm {
            url.set("https://github.com/AI-Prompt-Keeper/promptkeeper")
            connection.set("scm:git:https://github.com/AI-Prompt-Keeper/promptkeeper.git")
            developerConnection.set("scm:git:ssh://git@github.com/AI-Prompt-Keeper/promptkeeper.git")
        }
    }
}

signing {
    val rawKey = System.getenv("GPG_PRIVATE_KEY")
    val passphrase = System.getenv("GPG_PASSPHRASE")

    if (!rawKey.isNullOrBlank() && !passphrase.isNullOrBlank()) {
        val key: String = try {
            val decoded = Base64.getDecoder().decode(rawKey)
            String(decoded, Charsets.UTF_8)
        } catch (_: IllegalArgumentException) {
            rawKey
        }

        useInMemoryPgpKeys(key, passphrase)
        sign(publishing.publications)
    } else {
        logger.warn("GPG_PRIVATE_KEY or GPG_PASSPHRASE not set; artifacts will not be signed.")
    }
}

tasks.test {
    useJUnit()
}
