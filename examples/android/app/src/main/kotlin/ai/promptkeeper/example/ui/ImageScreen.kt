package ai.promptkeeper.example.ui

import android.util.Base64
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import coil.compose.rememberAsyncImagePainter
import coil.request.ImageRequest
import ai.promptkeeper.example.MainViewModel

@Composable
fun ImageScreen(
    modifier: Modifier = Modifier,
    viewModel: MainViewModel
) {
    var promptInput by remember { mutableStateOf("A cute cat on a windowsill") }
    val uiState by viewModel.imageState.collectAsState()

    Column(
        modifier = modifier,
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        OutlinedTextField(
            value = promptInput,
            onValueChange = { promptInput = it },
            label = { Text("Image prompt") },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true
        )
        Button(
            onClick = { viewModel.runImageExec(promptInput) },
            modifier = Modifier.align(Alignment.CenterHorizontally)
        ) {
            Text("Run image exec (Gemini)")
        }
        Button(
            onClick = { viewModel.clearImageState() },
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
                    Text("Generating image…", modifier = Modifier.padding(8.dp))
                }
            }
            is ExecUiState.Content -> {
                val contentState = uiState as ExecUiState.Content
                contentState.imageData?.let { data ->
                    val context = LocalContext.current
                    val model = when {
                        data.base64 != null -> {
                            val bytes = Base64.decode(data.base64, Base64.DEFAULT)
                            ImageRequest.Builder(context)
                                .data(bytes)
                                .build()
                        }
                        data.url != null -> {
                            ImageRequest.Builder(context)
                                .data(data.url)
                                .build()
                        }
                        else -> null
                    }
                    if (model != null) {
                        Image(
                            painter = rememberAsyncImagePainter(model),
                            contentDescription = "Generated image",
                            modifier = Modifier
                                .fillMaxWidth()
                                .size(320.dp),
                            contentScale = ContentScale.Fit
                        )
                    }
                }
                if (contentState.imageData == null && contentState.text.isNotEmpty()) {
                    Text(
                        "Response (text):",
                        style = MaterialTheme.typography.titleSmall,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                    SelectionContainer {
                        Text(
                            text = contentState.text,
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(vertical = 8.dp)
                                .verticalScroll(rememberScrollState()),
                            color = MaterialTheme.colorScheme.onSurface
                        )
                    }
                } else if (contentState.imageData == null && contentState.text.isEmpty()) {
                    Text(
                        "No image or text received. Backend may only support text streaming for this function.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
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
            ExecUiState.Idle -> { }
        }
    }
}
