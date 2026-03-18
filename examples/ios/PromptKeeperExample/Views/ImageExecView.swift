//
//  ImageExecView.swift
//  PromptKeeperExample
//

import SwiftUI

struct ImageExecView: View {
    @ObservedObject var execService: ExecService
    @State private var functionId = "image_gen"
    @State private var promptVariable = ""
    @State private var imageData: Data?
    @State private var isRunning = false
    @State private var errorMessage: String?

    var body: some View {
        Form {
            Section("Image exec (Gemini)") {
                TextField("Function ID", text: $functionId)
                    .textContentType(.none)
                    .autocapitalization(.none)
                TextField("Prompt (variable value)", text: $promptVariable)
                    .textContentType(.none)
            }
            Section {
                Button {
                    runImageExec()
                } label: {
                    HStack {
                        if isRunning { ProgressView().padding(.trailing, 8) }
                        Text(isRunning ? "Running…" : "Run image exec")
                    }
                }
                .disabled(!execService.isConfigured || isRunning)
            }
            if let err = errorMessage {
                Section {
                    Text(err).foregroundStyle(.red)
                }
            }
            if let data = imageData, let uiImage = UIImage(data: data) {
                Section("Generated image") {
                    Image(uiImage: uiImage)
                        .resizable()
                        .scaledToFit()
                        .frame(maxHeight: 300)
                }
            }
        }
        .navigationTitle("Image (Gemini)")
    }

    private func runImageExec() {
        imageData = nil
        errorMessage = nil
        isRunning = true
        Task {
            defer { Task { @MainActor in isRunning = false } }
            do {
                let data = try await execService.runImageExec(
                    functionId: functionId,
                    variables: ["prompt": promptVariable],
                    provider: "gemini",
                    model: nil
                )
                Task { @MainActor in imageData = data }
            } catch {
                Task { @MainActor in errorMessage = error.localizedDescription }
            }
        }
    }
}
