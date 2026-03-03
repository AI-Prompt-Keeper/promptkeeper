plugins {
    id("java-library")
    id("maven-publish")
    kotlin("jvm") version "1.9.22"
    kotlin("plugin.serialization") version "1.9.22"
}

group = "com.promptkeeper"
version = "1.0.0"

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
}

kotlin {
    jvmToolchain(11)
}

publishing {
    publications {
        create<MavenPublication>("release") {
            from(components["java"])
            groupId = "com.promptkeeper"
            artifactId = "android-sdk"
            version = project.version.toString()
            pom {
                name.set("PromptKeeper Android SDK")
                description.set("Kotlin/Android SDK for Prompt Keeper API — init, setKey, setPrompt, exec (streaming).")
                url.set("https://github.com/promptkeeper/promptkeeper")
            }
        }
    }
    repositories {
        mavenLocal()
    }
}

tasks.test {
    useJUnit()
}
