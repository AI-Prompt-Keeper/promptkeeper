package ai.promptkeeper.example

import ai.promptkeeper.sdk.PromptKeeper
import ai.promptkeeper.sdk.PromptKeeperException
import ai.promptkeeper.example.exec.parseImageChunk
import ai.promptkeeper.example.exec.parseOpenAITextChunk
import ai.promptkeeper.example.ui.ExecUiState
import ai.promptkeeper.example.ui.ImageData
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.catch
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

class MainViewModel : ViewModel() {

    private val sdk: PromptKeeper by lazy {
        PromptKeeper(apiKey = ApiKeys.PROMPTKEEPER_API_KEY)
    }

    private val _textState = MutableStateFlow<ExecUiState>(ExecUiState.Idle)
    val textState: StateFlow<ExecUiState> = _textState.asStateFlow()

    private val _imageState = MutableStateFlow<ExecUiState>(ExecUiState.Idle)
    val imageState: StateFlow<ExecUiState> = _imageState.asStateFlow()

    /** One-time setup: register stored prompts (`text`, `image`) if needed. Execution uses POST /v1/execute by title. */
    fun ensureConfigured() {
        viewModelScope.launch {
            try {
                sdk.setPrompt(
                    name = "text",
                    rawSecret = "{{prompt}}",
                    provider = null,
                    preferredModel = null
                )
                sdk.setPrompt(
                    name = "image",
                    rawSecret = "{{prompt}}",
                    provider = "gemini",
                    preferredModel = "gemini-3.1-flash-image-preview"
                )
            } catch (_: Exception) {
                // Keys/prompts may already be set or keys are placeholders
            }
        }
    }

    /** Runs stored prompt `text` with variable `prompt` (POST /v1/execute). */
    fun runTextExec(prompt: String, provider: String) {
        if (prompt.isBlank()) return
        val p = provider.trim().lowercase()
        if (p.isBlank()) return
        viewModelScope.launch {
            _textState.value = ExecUiState.Loading
            val sb = StringBuilder()
            try {
                try {
                    sdk.setPrompt(
                        name = "text",
                        rawSecret = "{{prompt}}",
                        provider = null,
                        preferredModel = null
                    )
                } catch (_: Exception) {
                    // prompt may already exist
                }
                val model: String? = when (p) {
                    "anthropic" -> "claude-sonnet-4-6"
                    // Text-only default; avoid `*-image-*` model names so Gemini uses text streaming.
                    "gemini" -> "gemini-3-flash-preview"
                    else -> null
                }
                sdk.exec(
                    functionId = "text",
                    variables = mapOf("prompt" to prompt),
                    provider = p,
                    model = model
                )
                    .catch { e: Throwable ->
                        _textState.update { _: ExecUiState ->
                            ExecUiState.Error(
                                (e as? PromptKeeperException.Server)?.message
                                    ?: (e as? PromptKeeperException.Http)?.let { "HTTP ${it.statusCode}" }
                                    ?: e.message ?: "Unknown error"
                            )
                        }
                    }
                    .collect { chunk: String ->
                        val part = parseOpenAITextChunk(chunk)
                        if (part != null) {
                            sb.append(part)
                        }
                        _textState.update { _: ExecUiState -> ExecUiState.Content(text = sb.toString()) }
                    }
            } finally {
                if (_textState.value == ExecUiState.Loading) {
                    val finalText = sb.toString()
                    _textState.update { ExecUiState.Content(text = finalText) }
                }
            }
        }
    }

    fun runImageExec(prompt: String) {
        if (prompt.isBlank()) return
        viewModelScope.launch {
            _imageState.value = ExecUiState.Loading
            var imageData: ImageData? = null
            val textSb = StringBuilder()
            try {
                sdk.exec(
                    functionId = "image",
                    variables = mapOf("prompt" to prompt),
                    provider = "gemini",
                    model = "gemini-3.1-flash-image-preview"
                )
                    .catch { e ->
                        _imageState.update {
                            ExecUiState.Error(
                                (e as? PromptKeeperException.Server)?.message
                                    ?: (e as? PromptKeeperException.Http)?.let { "HTTP ${it.statusCode}" }
                                    ?: e.message ?: "Unknown error"
                            )
                        }
                    }
                    .collect { chunk ->
                        val img = parseImageChunk(chunk)
                        if (img != null) {
                            imageData = ImageData(base64 = img.b64, url = img.url)
                        }
                        parseOpenAITextChunk(chunk)?.let { part ->
                            textSb.append(part)
                        }
                        if (img == null && textSb.isEmpty()) {
                            // ignore unparsed chunks
                        }
                        _imageState.update {
                            ExecUiState.Content(text = textSb.toString(), imageData = imageData)
                        }
                    }
            } finally {
                if (_imageState.value == ExecUiState.Loading) {
                    _imageState.update {
                        ExecUiState.Content(text = textSb.toString(), imageData = imageData)
                    }
                }
            }
        }
    }

    fun clearTextState() {
        _textState.value = ExecUiState.Idle
    }

    fun clearImageState() {
        _imageState.value = ExecUiState.Idle
    }
}
