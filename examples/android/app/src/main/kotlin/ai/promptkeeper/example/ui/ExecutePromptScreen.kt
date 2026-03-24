package ai.promptkeeper.example.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.collectAsState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import ai.promptkeeper.example.MainViewModel

@Composable
fun ExecutePromptScreen(
    modifier: Modifier = Modifier,
    viewModel: MainViewModel,
    onNavigateToAdd: () -> Unit
) {
    val titles by viewModel.promptTitles.collectAsState()
    val listLoading by viewModel.listLoading.collectAsState()
    val execState by viewModel.executeState.collectAsState()
    var provider by remember { mutableStateOf("openai") }

    LaunchedEffect(Unit) {
        viewModel.refreshPromptList()
    }

    Column(
        modifier = modifier.verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text(
            "Choose a stored prompt (execution key). Response streams below.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )

        Text("Provider for this run", style = MaterialTheme.typography.titleSmall)
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceEvenly
        ) {
            listOf("openai" to "openai", "gemini" to "gemini", "anthropic" to "Anthropic").forEach { (id, label) ->
                Row(verticalAlignment = Alignment.CenterVertically) {
                    RadioButton(
                        selected = provider == id,
                        onClick = { provider = id }
                    )
                    Text(label)
                }
            }
        }

        HorizontalDivider()

        when {
            listLoading -> {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.Center
                ) {
                    CircularProgressIndicator()
                }
            }
            titles.isEmpty() -> {
                Column(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    Text("No prompts yet.", style = MaterialTheme.typography.bodyLarge)
                    Button(onClick = onNavigateToAdd) {
                        Text("No prompts yet! Add one?")
                    }
                }
            }
            else -> {
                Text("Stored prompts", style = MaterialTheme.typography.titleSmall)
                Column(modifier = Modifier.fillMaxWidth()) {
                    titles.forEach { title ->
                        Text(
                            text = title,
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable {
                                    viewModel.executePrompt(title, provider)
                                }
                                .padding(vertical = 12.dp, horizontal = 8.dp),
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.primary
                        )
                        HorizontalDivider()
                    }
                }
            }
        }

        Button(
            onClick = { viewModel.refreshPromptList() },
            modifier = Modifier.align(Alignment.CenterHorizontally)
        ) {
            Text("Refresh list")
        }

        when (val state = execState) {
            ExecUiState.Idle -> { }
            ExecUiState.Loading -> {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.Center
                ) {
                    CircularProgressIndicator()
                    Text(
                        "Streaming…",
                        modifier = Modifier.padding(8.dp),
                        color = MaterialTheme.colorScheme.onSurface
                    )
                }
            }
            is ExecUiState.Content -> {
                Text("Response:", style = MaterialTheme.typography.titleSmall)
                SelectionContainer {
                    Text(
                        text = state.text,
                        modifier = Modifier
                            .fillMaxWidth()
                            .heightIn(min = 120.dp),
                        color = MaterialTheme.colorScheme.onSurface
                    )
                }
            }
        }

        Button(
            onClick = { viewModel.clearExecuteState() },
            modifier = Modifier.align(Alignment.CenterHorizontally)
        ) {
            Text("Clear output")
        }
    }
}
