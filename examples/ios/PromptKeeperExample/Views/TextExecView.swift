//
//  TextExecView.swift
//  PromptKeeperExample
//
//  Uses execPrompt (inline prompt) so you can run text generation without storing a function first.
//

import SwiftUI

struct TextExecView: View {
    @ObservedObject var execService: ExecService
    @State private var promptText = "You are a helpful assistant. Reply briefly to: What is 2+2?"
    @State private var streamedText = ""
    @State private var isRunning = false
    @State private var errorMessage: String?
    @State private var selectedProvider: ProviderOption = .openai

    var body: some View {
        Form {
            Section("Text query") {
                TextField("Prompt", text: $promptText)
                    .textContentType(.none)

                providerPickerRow
            }

            Section {
                Button {
                    runTextExecPrompt()
                } label: {
                    HStack {
                        if isRunning { ProgressView().padding(.trailing, 8) }
                        Text(isRunning ? "Running…" : "Run exec (prompt)")
                    }
                }
                .disabled(
                    !execService.isConfigured ||
                    isRunning ||
                    promptText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                )
            }

            if let err = errorMessage {
                Section {
                    Text(err).foregroundStyle(.red)
                }
            }

            if !streamedText.isEmpty {
                Section("Streamed output") {
                    ScrollView {
                        Text(streamedText)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .textSelection(.enabled)
                    }
                    .frame(minHeight: 120)
                }
            }
        }
        .navigationTitle("Text Query")
    }

    private func runTextExecPrompt() {
        streamedText = ""
        errorMessage = nil
        isRunning = true
        Task {
            defer { Task { @MainActor in isRunning = false } }
            do {
                try await execService.runTextExecPrompt(
                    prompt: promptText.trimmingCharacters(in: .whitespacesAndNewlines),
                    variables: [:],
                    provider: selectedProvider.rawValue,
                    model: selectedProvider.defaultModel
                ) { chunk in
                    Task { @MainActor in streamedText += chunk }
                }
            } catch {
                Task { @MainActor in errorMessage = error.localizedDescription }
            }
        }
    }

    private var providerPickerRow: some View {
        HStack(spacing: 14) {
            ForEach(ProviderOption.allCases) { option in
                ProviderRadioButton(
                    option: option,
                    selection: $selectedProvider
                )
            }
        }
        .padding(.top, 8)
    }
}

private enum ProviderOption: String, CaseIterable, Identifiable {
    case openai
    case gemini
    case anthropic

    var id: String { rawValue }

    var label: String {
        switch self {
        case .openai: return "OpenAI"
        case .gemini: return "Gemini"
        case .anthropic: return "Anthropic"
        }
    }

    /// Default models requested by the example requirements.
    var defaultModel: String? {
        switch self {
        case .gemini: return "gemini-3-flash-preview"
        case .anthropic: return "claude-sonnet-4-6"
        case .openai: return nil
        }
    }
}

private struct ProviderRadioButton: View {
    let option: ProviderOption
    @Binding var selection: ProviderOption

    var body: some View {
        Button {
            selection = option
        } label: {
            HStack(spacing: 8) {
                radioCircle
                Text(option.label)
                    .font(.subheadline)
                    .foregroundColor(.primary)
            }
        }
        .buttonStyle(PlainButtonStyle())
    }

    private var radioCircle: some View {
        let isSelected = selection == option
        return ZStack {
            Circle()
                .strokeBorder(isSelected ? Color.accentColor : Color.secondary, lineWidth: 2)
                .frame(width: 18, height: 18)
            if isSelected {
                Circle()
                    .fill(Color.accentColor)
                    .frame(width: 10, height: 10)
            }
        }
    }
}
