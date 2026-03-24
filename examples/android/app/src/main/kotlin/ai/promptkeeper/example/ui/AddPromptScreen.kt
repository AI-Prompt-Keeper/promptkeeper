package ai.promptkeeper.example.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
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
fun AddPromptScreen(
    modifier: Modifier = Modifier,
    viewModel: MainViewModel
) {
    var title by remember { mutableStateOf("") }
    var promptText by remember { mutableStateOf("") }
    var provider by remember { mutableStateOf("openai") }
    val storeLoading by viewModel.storeLoading.collectAsState()

    Column(
        modifier = modifier.verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text(
            "Register a prompt on the server (management key). Variables can use Handlebars, e.g. {{name}}.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )

        OutlinedTextField(
            value = title,
            onValueChange = { title = it },
            label = { Text("Prompt title *") },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
            enabled = !storeLoading
        )

        OutlinedTextField(
            value = promptText,
            onValueChange = { promptText = it },
            label = { Text("Prompt text *") },
            modifier = Modifier.fillMaxWidth(),
            minLines = 3,
            enabled = !storeLoading
        )

        Text("Provider", style = MaterialTheme.typography.titleSmall)
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceEvenly
        ) {
            listOf("openai" to "openai", "gemini" to "gemini", "anthropic" to "Anthropic").forEach { (id, label) ->
                Row(verticalAlignment = Alignment.CenterVertically) {
                    RadioButton(
                        selected = provider == id,
                        onClick = { provider = id },
                        enabled = !storeLoading
                    )
                    Text(label)
                }
            }
        }

        Button(
            onClick = { viewModel.storePrompt(title, promptText, provider) },
            modifier = Modifier.align(Alignment.CenterHorizontally),
            enabled = !storeLoading
        ) {
            if (storeLoading) {
                CircularProgressIndicator(modifier = Modifier.size(20.dp))
            } else {
                Text("Store prompt")
            }
        }
    }
}
