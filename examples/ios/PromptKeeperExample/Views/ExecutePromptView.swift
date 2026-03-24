//
//  ExecutePromptView.swift
//  PromptKeeperExample
//

import SwiftUI

struct ExecutePromptView: View {
    @ObservedObject var execService: ExecService
    @Binding var selectedTab: Int

    @State private var titles: [String] = []
    /// Full-screen spinner only while the first load (or reload with an empty list) is in flight.
    @State private var isLoadingList = true
    @State private var listErrorMessage: String?
    @State private var selectedTitle: String?
    @State private var streamedText = ""
    @State private var isExecuting = false
    @State private var execErrorMessage: String?
    @State private var alertTitle = ""
    @State private var alertMessage = ""
    @State private var showAlert = false

    var body: some View {
        Group {
            if !execService.hasExecutionKey {
                Form {
                    Section {
                        Text("Set Keys.executionAPIKey in Keys.swift to an execution key (`pk_exe_live_…`).")
                            .foregroundStyle(.secondary)
                    }
                }
            } else if isLoadingList {
                ProgressView("Loading prompts…")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let err = listErrorMessage, titles.isEmpty {
                loadFailedView(message: err)
            } else if titles.isEmpty {
                emptyState
            } else {
                listAndOutput
            }
        }
        .navigationTitle("Execute prompt")
        .task(id: execService.hasExecutionKey) {
            guard execService.hasExecutionKey else {
                await MainActor.run {
                    titles = []
                    listErrorMessage = nil
                    isLoadingList = false
                }
                return
            }
            await refreshList()
        }
        .refreshable {
            await refreshList()
        }
        .alert(alertTitle, isPresented: $showAlert) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(alertMessage)
        }
    }

    private func loadFailedView(message: String) -> some View {
        Form {
            Section {
                Text(message)
                    .foregroundStyle(.red)
            }
            Section {
                Button("Try again") {
                    Task { await refreshList() }
                }
            }
        }
    }

    private var emptyState: some View {
        VStack(spacing: 20) {
            Text("No prompts yet! Add one?")
                .font(.headline)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
            Button {
                selectedTab = 0
            } label: {
                Text("Go to Add prompt")
            }
            .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var listAndOutput: some View {
        List {
            if let err = listErrorMessage {
                Section {
                    Text(err)
                        .foregroundStyle(.red)
                }
            }
            Section("Stored prompts") {
                ForEach(titles, id: \.self) { title in
                    Button {
                        runExec(for: title)
                    } label: {
                        HStack {
                            Text(title)
                                .foregroundStyle(.primary)
                            Spacer()
                            if selectedTitle == title, isExecuting {
                                ProgressView()
                            }
                        }
                    }
                    .disabled(isExecuting)
                }
            }
            if let err = execErrorMessage {
                Section {
                    Text(err).foregroundStyle(.red)
                }
            }
            if !streamedText.isEmpty {
                Section("Output") {
                    ScrollView {
                        Text(streamedText)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .textSelection(.enabled)
                    }
                    .frame(minHeight: 120)
                }
            }
        }
    }

    private func refreshList() async {
        guard execService.hasExecutionKey else { return }
        await MainActor.run {
            if titles.isEmpty {
                isLoadingList = true
            }
            listErrorMessage = nil
        }
        do {
            let list = try await execService.listPromptTitles()
            await MainActor.run {
                titles = list
                listErrorMessage = nil
            }
        } catch {
            await MainActor.run {
                listErrorMessage = UserFacingError.message(for: error)
            }
        }
        await MainActor.run { isLoadingList = false }
    }

    private func runExec(for title: String) {
        streamedText = ""
        execErrorMessage = nil
        selectedTitle = title
        isExecuting = true
        Task {
            defer { Task { @MainActor in
                isExecuting = false
                selectedTitle = nil
            } }
            do {
                try await execService.runStreamingExec(functionId: title, provider: nil, model: nil) { chunk in
                    Task { @MainActor in streamedText += chunk }
                }
            } catch {
                await MainActor.run {
                    alertTitle = "Couldn’t run prompt"
                    alertMessage = UserFacingError.message(for: error)
                    showAlert = true
                }
            }
        }
    }
}
