import java.util.Base64

plugins {
    id("java-library")
    id("maven-publish")
    id("signing")
    id("org.jetbrains.kotlin.jvm") version "1.9.22"
    id("org.jetbrains.kotlin.plugin.serialization") version "1.9.22"
    id("io.github.gradle-nexus.publish-plugin") version "1.3.0"
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
    sourceCompatibility = JavaVersion.VERSION_11
    targetCompatibility = JavaVersion.VERSION_11
    withSourcesJar()
    withJavadocJar()
}

kotlin {
    jvmToolchain(11)
}

publishing {
    publications {
        create<MavenPublication>("release") {
            from(components["java"])

            groupId = "ai.promptkeeper"
            artifactId = "android-sdk"
            version = (project.findProperty("VERSION_NAME") as String?) ?: version.toString()

            pom {
                name.set("PromptKeeper Android SDK")
                description.set("Kotlin/Android SDK for PromptKeeper API — init, setKey, setPrompt, exec (streaming).")
                url.set("https://github.com/promptkeeper/promptkeeper")

                licenses {
                    license {
                        name.set("The Apache License, Version 2.0")
                        url.set("http://www.apache.org/licenses/LICENSE-2.0.txt")
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
                    url.set("https://github.com/promptkeeper/promptkeeper")
                    connection.set("scm:git:https://github.com/promptkeeper/promptkeeper.git")
                    developerConnection.set("scm:git:ssh://git@github.com/promptkeeper/promptkeeper.git")
                }
            }
        }
    }
}

nexusPublishing {
    repositories {
        sonatype {
            nexusUrl.set(uri("https://s01.oss.sonatype.org/service/local/"))
            snapshotRepositoryUrl.set(uri("https://s01.oss.sonatype.org/content/repositories/snapshots/"))
            username.set(System.getenv("OSSRH_USERNAME"))
            password.set(System.getenv("OSSRH_TOKEN"))
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
        sign(publishing.publications["release"])
    } else {
        logger.warn("GPG_PRIVATE_KEY or GPG_PASSPHRASE not set; artifacts will not be signed.")
    }
}

tasks.test {
    useJUnit()
}
