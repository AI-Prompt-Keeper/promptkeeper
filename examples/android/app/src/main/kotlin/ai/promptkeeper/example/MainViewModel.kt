package ai.promptkeeper.example

import ai.promptkeeper.example.api.ListPromptsClient
import ai.promptkeeper.example.exec.parseOpenAITextChunk
import ai.promptkeeper.example.ui.ExecUiState
import ai.promptkeeper.sdk.PromptKeeper
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.catch
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

class MainViewModel : ViewModel() {

    private val managementSdk: PromptKeeper by lazy {
        PromptKeeper(apiKey = ApiKeys.PROMPTKEEPER_MANAGEMENT_KEY)
    }

    private val executionSdk: PromptKeeper by lazy {
        PromptKeeper(apiKey = ApiKeys.PROMPTKEEPER_EXECUTION_KEY)
    }

    private val _dialogMessage = MutableStateFlow<String?>(null)
    val dialogMessage: StateFlow<String?> = _dialogMessage.asStateFlow()

    private val _promptTitles = MutableStateFlow<List<String>>(emptyList())
    val promptTitles: StateFlow<List<String>> = _promptTitles.asStateFlow()

    private val _listLoading = MutableStateFlow(false)
    val listLoading: StateFlow<Boolean> = _listLoading.asStateFlow()

    private val _executeState = MutableStateFlow<ExecUiState>(ExecUiState.Idle)
    val executeState: StateFlow<ExecUiState> = _executeState.asStateFlow()

    private val _storeLoading = MutableStateFlow(false)
    val storeLoading: StateFlow<Boolean> = _storeLoading.asStateFlow()

    fun dismissDialog() {
        _dialogMessage.value = null
    }

    fun showMessage(message: String) {
        _dialogMessage.value = message
    }

    /**
     * Stores a new prompt using the **management** key (`POST /v1/prompts`).
     */
    fun storePrompt(title: String, text: String, provider: String) {
        val t = title.trim()
        val body = text.trim()
        if (t.isEmpty() || body.isEmpty()) {
            _dialogMessage.value = "Please enter both a prompt title and prompt text."
            return
        }
        viewModelScope.launch {
            _storeLoading.value = true
            try {
                managementSdk.setPrompt(
                    name = t,
                    rawSecret = body,
                    provider = provider.trim().lowercase(),
                    preferredModel = null
                )
                _dialogMessage.value = "Prompt stored successfully. It may take a moment to appear in the list after deployment."
            } catch (e: Exception) {
                _dialogMessage.value = e.toUserMessage()
            } finally {
                _storeLoading.value = false
            }
        }

        /*
         * Demo: storing with an **execution** key is expected to fail with **403 Forbidden**
         * (execution keys may only call `POST /v1/execute` and `GET /v1/list-prompts`).
         * Uncomment the block below to verify error handling in the dialog:
         *
         * viewModelScope.launch {
         *     try {
         *         executionSdk.setPrompt(
         *             name = t,
         *             rawSecret = body,
         *             provider = provider.trim().lowercase(),
         *             preferredModel = null
         *         )
         *     } catch (e: Exception) {
         *         _dialogMessage.value = e.toUserMessage()
         *     }
         * }
         */
    }

    fun refreshPromptList() {
        viewModelScope.launch {
            _listLoading.value = true
            try {
                val result = ListPromptsClient.fetchTitles(ApiKeys.PROMPTKEEPER_EXECUTION_KEY)
                result.fold(
                    onSuccess = { _promptTitles.value = it },
                    onFailure = { e -> _dialogMessage.value = e.toUserMessage() }
                )
            } finally {
                _listLoading.value = false
            }
        }
    }

    /**
     * Runs a stored prompt by title using the **execution** key (`POST /v1/execute`).
     */
    fun executePrompt(title: String, provider: String) {
        val fn = title.trim()
        if (fn.isEmpty()) return
        val p = provider.trim().lowercase()
        viewModelScope.launch {
            _executeState.value = ExecUiState.Loading
            val sb = StringBuilder()
            try {
                val model: String? = when (p) {
                    "anthropic" -> "claude-sonnet-4-6"
                    "gemini" -> "gemini-3-flash-preview"
                    else -> null
                }
                executionSdk.exec(
                    functionId = fn,
                    variables = emptyMap(),
                    provider = p,
                    model = model
                )
                    .catch { e: Throwable ->
                        _dialogMessage.value = e.toUserMessage()
                        _executeState.value = ExecUiState.Idle
                    }
                    .collect { chunk: String ->
                        val part = parseOpenAITextChunk(chunk)
                        if (part != null) sb.append(part)
                        _executeState.update { ExecUiState.Content(text = sb.toString()) }
                    }
            } finally {
                if (_executeState.value is ExecUiState.Loading) {
                    val finalText = sb.toString()
                    _executeState.update { ExecUiState.Content(text = finalText) }
                }
            }
        }
    }

    fun clearExecuteState() {
        _executeState.value = ExecUiState.Idle
    }
}
