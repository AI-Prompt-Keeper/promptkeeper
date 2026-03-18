//
//  ContentView.swift
//  PromptKeeperExample
//

import SwiftUI

struct ContentView: View {
    @StateObject private var execService = ExecService()
    @State private var promptKeeperKey = ""
    @State private var selectedTab = 0

    var body: some View {
        TabView(selection: $selectedTab) {
            configView
                .tabItem { Label("Config", systemImage: "key.fill") }
                .tag(0)
            textExecView
                .tabItem { Label("Text Query", systemImage: "text.bubble.fill") }
                .tag(1)
        }
        .onAppear {
            if !Keys.promptKeeperAPIKey.isEmpty {
                execService.configure(apiKey: Keys.promptKeeperAPIKey)
            }
        }
    }

    private var configView: some View {
        Form {
            Section {
                SecureField("Prompt Keeper API key", text: $promptKeeperKey)
                    .textContentType(.password)
                    .autocapitalization(.none)
                Button("Use this key (in-memory only)") {
                    if !promptKeeperKey.isEmpty {
                        execService.configure(apiKey: promptKeeperKey)
                    }
                }
                .disabled(promptKeeperKey.isEmpty)
            } header: {
                Text("API key")
            } footer: {
                Text("Obtain via CLI (e.g. prke login) or your backend. Not persisted.")
            }
            if execService.isConfigured {
                Label("Key is set for this session", systemImage: "checkmark.circle.fill")
                    .foregroundStyle(.green)
            }
        }
        .navigationTitle("Config")
    }

    private var textExecView: some View {
        TextExecView(execService: execService)
    }
}

#Preview {
    ContentView()
}
