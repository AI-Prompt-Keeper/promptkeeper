package ai.promptkeeper.example.ui

/**
 * UI state for streaming execute output.
 */
sealed class ExecUiState {
    data object Idle : ExecUiState()
    data object Loading : ExecUiState()
    data class Content(val text: String = "") : ExecUiState()
}
