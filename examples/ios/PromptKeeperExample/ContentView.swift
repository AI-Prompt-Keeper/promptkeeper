//
//  ContentView.swift
//  PromptKeeperExample
//

import SwiftUI

struct ContentView: View {
    @StateObject private var execService = ExecService()
    @State private var selectedTab = 0

    var body: some View {
        TabView(selection: $selectedTab) {
            NavigationView {
                AddPromptView(execService: execService)
            }
            .tabItem { Label("Add prompt", systemImage: "plus.square.on.square") }
            .tag(0)
            NavigationView {
                ExecutePromptView(execService: execService, selectedTab: $selectedTab)
            }
            .tabItem { Label("Execute prompt", systemImage: "play.circle") }
            .tag(1)
        }
        .onAppear {
            execService.reloadClientsFromKeys()
        }
    }
}

#Preview {
    ContentView()
}
