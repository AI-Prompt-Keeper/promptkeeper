package ai.promptkeeper.example.ui

/**
 * UI state for exec screens (text and image).
 */
sealed class ExecUiState {
    data object Idle : ExecUiState()
    data object Loading : ExecUiState()
    data class Content(val text: String = "", val imageData: ImageData? = null) : ExecUiState()
    data class Error(val message: String) : ExecUiState()
}

data class ImageData(
    val base64: String? = null,
    val url: String? = null
)
