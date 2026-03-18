package ai.promptkeeper.example

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import ai.promptkeeper.example.ui.TextScreen
import ai.promptkeeper.example.ui.Theme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            Theme {
                val viewModel: MainViewModel = viewModel()

                Scaffold(
                    modifier = Modifier.fillMaxSize(),
                    topBar = {
                        Surface(
                            modifier = Modifier.windowInsetsPadding(WindowInsets.statusBars),
                            color = MaterialTheme.colorScheme.primaryContainer,
                            shadowElevation = 4.dp
                        ) {
                            Text(
                                text = "Text",
                                modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
                                style = MaterialTheme.typography.titleLarge,
                                color = MaterialTheme.colorScheme.onPrimaryContainer
                            )
                        }
                    }
                ) { padding ->
                    TextScreen(
                        modifier = Modifier.fillMaxSize().padding(padding).padding(16.dp),
                        viewModel = viewModel
                    )
                }
            }
        }
    }
}

