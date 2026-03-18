package ai.promptkeeper.example.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.RadioButton
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.collectAsState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.foundation.layout.padding
import androidx.compose.ui.unit.dp
import ai.promptkeeper.example.MainViewModel

@Composable
fun TextScreen(
    modifier: Modifier = Modifier,
    viewModel: MainViewModel
) {
    var promptInput by remember { mutableStateOf("") }
    var selectedProvider by remember { mutableStateOf("openai") }
    val uiState by viewModel.textState.collectAsState()

    Column(
        modifier = modifier,
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        OutlinedTextField(
            value = promptInput,
            onValueChange = { promptInput = it },
            label = { Text("Prompt") },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true
        )

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceEvenly
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                RadioButton(
                    selected = selectedProvider == "openai",
                    onClick = { selectedProvider = "openai" }
                )
                Text("openai")
            }
            Row(verticalAlignment = Alignment.CenterVertically) {
                RadioButton(
                    selected = selectedProvider == "gemini",
                    onClick = { selectedProvider = "gemini" }
                )
                Text("gemini")
            }
            Row(verticalAlignment = Alignment.CenterVertically) {
                RadioButton(
                    selected = selectedProvider == "anthropic",
                    onClick = { selectedProvider = "anthropic" }
                )
                Text("Anthropic")
            }
        }

        Button(
            onClick = { viewModel.runTextExec(promptInput, selectedProvider) },
            modifier = Modifier.align(Alignment.CenterHorizontally)
        ) {
            Text("Send request")
        }
        Button(
            onClick = { viewModel.clearTextState() },
            modifier = Modifier.align(Alignment.CenterHorizontally)
        ) {
            Text("Clear")
        }

        when (uiState) {
            is ExecUiState.Loading -> {
                Column(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalAlignment = Alignment.CenterHorizontally
                ) {
                    CircularProgressIndicator()
                    Text("Streaming…", modifier = Modifier.padding(8.dp), color = MaterialTheme.colorScheme.onSurface)
                }
            }
            is ExecUiState.Content -> {
                val contentState = uiState as ExecUiState.Content
                Text(
                    "Response:",
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.onSurface
                )
                SelectionContainer {
                    Text(
                        text = contentState.text,
                        modifier = Modifier
                            .fillMaxWidth()
                            .heightIn(min = 100.dp)
                            .verticalScroll(rememberScrollState()),
                        color = MaterialTheme.colorScheme.onSurface
                    )
                }
            }
            is ExecUiState.Error -> {
                val errorState = uiState as ExecUiState.Error
                Text(
                    text = "Error: ${errorState.message}",
                    color = MaterialTheme.colorScheme.error
                )
            }
            ExecUiState.Idle -> {
                Text(
                    "Response will appear here after you run.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}
