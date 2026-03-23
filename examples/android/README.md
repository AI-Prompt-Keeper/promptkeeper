# PromptKeeper Android Example

Example Android app that integrates the [PromptKeeper Android SDK](https://github.com/AI-Prompt-Keeper/promptkeeper) from Maven Central. It demonstrates:

- **Text (OpenAI / Gemini / Anthropic)**: Registers stored prompt `text` (`{{prompt}}`), then `exec(functionId = "text", ...)` and streams the response.
- **Image (Gemini)**: Run `exec()` with an image-generation prompt and display the result (base64 or URL).

## Requirements

- Android Studio Ladybug (2024.2) or newer (or use CLI with JDK 17).
- Min SDK 24, target SDK 34.

## Setup

1. **Open the project**: Open the `examples/android` folder in Android Studio (or run from the repo root with `./gradlew :app:assembleDebug` if you have a Gradle wrapper at repo root).

2. **API keys** (see `app/src/main/kotlin/ai/promptkeeper/example/ApiKeys.kt`):
   - **PromptKeeper API key** (`pk_...`): Obtain via your PromptKeeper registration or CLI.
   - **OpenAI** and **Gemini** keys: Create in the provider consoles; register them with PromptKeeper (e.g. via CLI) so the backend can use them. The app uses stub placeholders — **do not store real keys in code or commit them**; use BuildConfig, env, or a secure store in production.

3. Replace the placeholders in `ApiKeys.kt` only for local testing; never commit real keys.

## Architecture

- **UI**: Jetpack Compose, Material 3, two tabs (Text / Image).
- **Architecture**: MVVM (single `MainViewModel`; UI state via `StateFlow`).
- **Networking**: Handled by the PromptKeeper SDK (OkHttp); no manual HTTP in the app.
- **Dependencies**: `ai.promptkeeper:android-sdk` from Maven Central, Coil for image loading, Kotlin Serialization for parsing SSE chunks.

## Build & run

From Android Studio: Run the `app` configuration on a device or emulator.

From CLI (with Gradle wrapper in this directory or from repo root):

```bash
cd examples/android
./gradlew :app:assembleDebug
# or from repo root, if you have a root build that includes this project:
# ./gradlew :examples:android:app:assembleDebug
```

If the wrapper is missing, generate it with `gradle wrapper` or open the project in Android Studio once to create it.
