//
//  AddPromptView.swift
//  PromptKeeperExample
//

import SwiftUI

struct AddPromptView: View {
    @ObservedObject var execService: ExecService
    @State private var promptTitle = ""
    @State private var promptText = ""
    @State private var provider: PromptProviderOption = .openai
    @State private var isStoring = false
    @State private var alertTitle = ""
    @State private var alertMessage = ""
    @State private var showAlert = false

    var body: some View {
        Form {
            Section {
                TextField("Prompt title", text: $promptTitle)
                    .textContentType(.none)
                    .autocapitalization(.none)
                ZStack(alignment: .topLeading) {
                    if promptText.isEmpty {
                        Text("Prompt text")
                            .foregroundStyle(.secondary)
                            .padding(.top, 8)
                            .padding(.leading, 4)
                    }
                    TextEditor(text: $promptText)
                        .frame(minHeight: 140)
                }
            } header: {
                Text("Prompt")
            } footer: {
                Text("Title and body are required. The title becomes the function id used when executing.")
            }

            Section("Provider") {
                Picker("Provider", selection: $provider) {
                    ForEach(PromptProviderOption.allCases) { option in
                        Text(option.label).tag(option)
                    }
                }
            }

            Section {
                Button {
                    storePrompt()
                } label: {
                    HStack {
                        if isStoring { ProgressView().padding(.trailing, 8) }
                        Text(isStoring ? "Storing…" : "Store prompt")
                    }
                }
                .disabled(!canStore || isStoring)
            }

            if !execService.hasManagementKey {
                Section {
                    Text("Set Keys.managementAPIKey in Keys.swift to a management key (`pk_mgt_live_…`).")
                        .foregroundStyle(.secondary)
                }
            }
        }
        .navigationTitle("Add prompt")
        .alert(alertTitle, isPresented: $showAlert) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(alertMessage)
        }
    }

    private var canStore: Bool {
        execService.hasManagementKey &&
            !promptTitle.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty &&
            !promptText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private func storePrompt() {
        let trimmedTitle = promptTitle.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedBody = promptText.trimmingCharacters(in: .whitespacesAndNewlines)
        let providerRaw = provider.rawValue
        isStoring = true
        Task {
            defer { Task { @MainActor in isStoring = false } }
            do {
                _ = try await execService.storePrompt(
                    title: trimmedTitle,
                    promptText: trimmedBody,
                    provider: providerRaw
                )

                // MARK: - Wrong-key demonstration (read before uncommenting)
                // The line below calls `setPrompt` with the **execution** API key instead of the management key.
                // **Uncommenting is expected to fail** with HTTP **403 Forbidden** — execution keys cannot store prompts.
                // try await PromptKeeper(apiKey: Keys.executionAPIKey).setPrompt(
                //     name: trimmedTitle,
                //     rawSecret: trimmedBody,
                //     provider: providerRaw,
                //     preferredModel: nil
                // )

                await MainActor.run {
                    alertTitle = "Stored"
                    alertMessage = "Prompt “\(trimmedTitle)” was saved. You can run it from the Execute prompt tab."
                    showAlert = true
                }
            } catch {
                await MainActor.run {
                    alertTitle = "Couldn’t store prompt"
                    alertMessage = UserFacingError.message(for: error)
                    showAlert = true
                }
            }
        }
    }
}

private enum PromptProviderOption: String, CaseIterable, Identifiable {
    case openai
    case anthropic
    case gemini

    var id: String { rawValue }

    var label: String {
        switch self {
        case .openai: return "OpenAI"
        case .anthropic: return "Anthropic"
        case .gemini: return "Gemini"
        }
    }
}
