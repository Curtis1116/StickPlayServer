import SwiftUI

struct ContentView: View {
    @State private var videos: [VideoEntry] = []
    @State private var libraries: [Library] = []
    @State private var currentLibrary: String = UserDefaults.standard.string(forKey: "currentLibrary") ?? "Default"
    
    @State private var isLoading = true
    @State private var errorMessage: String?
    @State private var lastDebugLog: String = "App Started"
    
    @State private var isSearchVisible = false
    @State private var searchText = ""
    @State private var selectedSort = UserDefaults.standard.string(forKey: "selectedSort") ?? "date_added"
    @State private var isAscending = UserDefaults.standard.bool(forKey: "isAscending")
    @State private var favoritesOnly = UserDefaults.standard.bool(forKey: "favoritesOnly")
    @State private var uncensoredOnly = UserDefaults.standard.bool(forKey: "uncensoredOnly")
    @State private var selectedLevel = UserDefaults.standard.string(forKey: "selectedLevel") ?? "All"
    @State private var allLevels: [String] = []
    
    @State private var showingSettings = false
    @State private var editingVideo: VideoEntry?
    @State private var showingInfuseAlert = false
    
    private let api = APIService.shared
    
    // Grid Setup
    private let columns = [
        GridItem(.adaptive(minimum: 160), spacing: 16)
    ]
    
    var filteredVideos: [VideoEntry] {
        var result = videos
        if !searchText.isEmpty {
            result = result.filter { $0.title.localizedCaseInsensitiveContains(searchText) || $0.id.localizedCaseInsensitiveContains(searchText) }
        }
        return result
    }
    
    var body: some View {
        NavigationView {
            ZStack {
                Color(UIColor.systemGroupedBackground).ignoresSafeArea()
                
                VStack(spacing: 0) {
                    if isSearchVisible {
                        SearchBar(text: $searchText)
                            .padding(.horizontal)
                            .padding(.vertical, 8)
                    }
                    
                    if isLoading {
                        Spacer()
                        ProgressView("Loading videos...")
                        Text(lastDebugLog).font(.caption2).foregroundColor(.secondary).padding()
                        Spacer()
                    } else if let error = errorMessage {
                        VStack(spacing: 20) {
                            Image(systemName: "exclamationmark.triangle")
                                .font(.system(size: 50))
                                .foregroundColor(.red)
                            Text("Fetch Error")
                                .font(.headline)
                            Text(error)
                                .font(.subheadline)
                                .multilineTextAlignment(.center)
                            Button("Retry") { reloadData() }
                                .buttonStyle(.borderedProminent)
                        }
                        .padding()
                    } else {
                        ScrollView {
                            LazyVGrid(columns: columns, spacing: 16) {
                                ForEach(filteredVideos) { video in
                                    VideoCard(
                                        video: video,
                                        onPlay: { playVideo(video: video) },
                                        onEdit: { editingVideo = video },
                                        onRescan: { rescanVideo(video: video) }
                                    )
                                }
                            }
                            .padding(.horizontal)
                        }
                        .padding(.top, -30)
                        .refreshable { await loadData() }
                    }
                }
            }
            .navigationTitle("")
            .toolbar {
                ToolbarItem(placement: .principal) {
                    HStack {
                        Text("\(filteredVideos.count) 影片")
                            .font(.system(size: 11))
                            .foregroundColor(.secondary)
                        Spacer()
                    }
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    HStack(spacing: 14) {
                        Button(action: { withAnimation { isSearchVisible.toggle() } }) {
                            Image(systemName: "magnifyingglass.circle")
                        }
                        
                        Menu {
                            Section("Libraries") {
                                ForEach(libraries) { lib in
                                    Button(action: { switchLibrary(lib) }) {
                                        HStack {
                                            Text(lib.name)
                                            if currentLibrary == lib.name { Image(systemName: "checkmark") }
                                        }
                                    }
                                }
                            }
                        } label: {
                            HStack(spacing: 2) {
                                Image(systemName: "server.rack")
                                if !libraries.isEmpty {
                                    Text("\(libraries.count)").font(.caption2)
                                }
                            }
                        }
                        
                        Button(action: { scanAll() }) {
                            Image(systemName: "arrow.clockwise.icloud")
                        }
                        
                        Menu {
                            Section("Filter") {
                                Button(action: { favoritesOnly.toggle(); reloadData() }) {
                                    Label("Favorites Only", systemImage: favoritesOnly ? "heart.fill" : "heart")
                                }
                                Button(action: { uncensoredOnly.toggle(); reloadData() }) {
                                    Label("Uncensored (無碼)", systemImage: uncensoredOnly ? "checkmark.circle.fill" : "circle")
                                }
                            }
                        } label: {
                            Image(systemName: (favoritesOnly || uncensoredOnly) ? "line.3.horizontal.decrease.circle.fill" : "line.3.horizontal.decrease.circle")
                        }

                        Menu {
                            Section("Sort By") {
                                sortButton(title: "ID", field: "id")
                                sortButton(title: "Title", field: "title")
                                sortButton(title: "Date Added", field: "date_added")
                                sortButton(title: "Release Date", field: "release_date")
                                sortButton(title: "Rating", field: "rating")
                                sortButton(title: "Actors", field: "actors")
                            }
                        } label: {
                            Image(systemName: "arrow.up.arrow.down")
                        }
                        
                        Button(action: { showingSettings = true }) {
                            Image(systemName: "gearshape.fill")
                        }
                    }
                }
            }
            .sheet(isPresented: $showingSettings, onDismiss: { reloadData() }) { SettingsView() }
            .sheet(item: $editingVideo) { video in EditVideoView(video: video, onSave: { reloadData() }) }
            .alert("未安裝 Infuse", isPresented: $showingInfuseAlert) { Button("OK", role: .cancel) { } }
            .task {
                do {
                    let libs = try await api.fetchLibraries()
                    await MainActor.run {
                        self.libraries = libs
                        let target = libs.first(where: { $0.name == currentLibrary }) ?? libs.first
                        if let target = target {
                            switchLibrary(target)
                        } else {
                            isLoading = false
                        }
                    }
                } catch {
                    await MainActor.run {
                        self.errorMessage = error.localizedDescription
                        self.isLoading = false
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func sortButton(title: String, field: String) -> some View {
        Button(action: {
            if selectedSort == field { isAscending.toggle() }
            else { selectedSort = field; isAscending = true }
            reloadData()
        }) {
            HStack {
                Text(title)
                if selectedSort == field { Image(systemName: isAscending ? "arrow.up" : "arrow.down") }
            }
        }
    }
    
    private func reloadData() { Task { await loadData() } }
    
    private func loadData() async {
        await MainActor.run { isLoading = true; errorMessage = nil }
        UserDefaults.standard.set(favoritesOnly, forKey: "favoritesOnly")
        UserDefaults.standard.set(uncensoredOnly, forKey: "uncensoredOnly")
        
        do {
            let filter = VideoFilter(
                search: searchText,
                genres: uncensoredOnly ? ["無碼"] : nil,
                levels: selectedLevel == "All" ? nil : [selectedLevel],
                sortBy: selectedSort,
                sortOrder: isAscending ? "asc" : "desc",
                favoritesOnly: favoritesOnly
            )
            let result = try await api.fetchVideos(filter: filter)
            await MainActor.run {
                self.videos = result
                self.isLoading = false
            }
        } catch {
            await MainActor.run {
                self.errorMessage = error.localizedDescription
                self.isLoading = false
            }
        }
    }
    
    private func switchLibrary(_ lib: Library) {
        Task {
            @MainActor in
            isLoading = true
            do {
                try await api.switchLibrary(name: lib.dbName)
                currentLibrary = lib.name
                UserDefaults.standard.set(lib.name, forKey: "currentLibrary")
                await loadData()
            } catch {
                self.errorMessage = "Switch failed: \(error.localizedDescription)"
                self.isLoading = false
            }
        }
    }
    
    private func scanAll() {
        guard let currentLib = libraries.first(where: { $0.name == currentLibrary }) else { return }
        Task {
            do {
                _ = try await api.scanLibrary(paths: currentLib.paths)
                reloadData()
            } catch {
                await MainActor.run { self.errorMessage = "Scan failed: \(error.localizedDescription)" }
            }
        }
    }

    private func rescanVideo(video: VideoEntry) {
        Task {
            do {
                _ = try await api.rescanVideo(folderPath: video.folderPath)
                reloadData()
            } catch {
                await MainActor.run { self.errorMessage = "Rescan failed: \(error.localizedDescription)" }
            }
        }
    }
    
    private func playVideo(video: VideoEntry) {
        guard let serverVideoUrl = APIService.shared.getVideoUrl(path: video.videoPath) else { return }
        UIApplication.shared.open(serverVideoUrl, options: [:]) { success in
            if !success { self.showingInfuseAlert = true }
        }
    }
}

struct SearchBar: View {
    @Binding var text: String
    var body: some View {
        HStack {
            Image(systemName: "magnifyingglass").foregroundColor(.secondary)
            TextField("Search...", text: $text)
                .textFieldStyle(.plain)
                .autocapitalization(.none)
            if !text.isEmpty {
                Button(action: { text = "" }) {
                    Image(systemName: "xmark.circle.fill").foregroundColor(.secondary)
                }
            }
        }
        .padding(8)
        .background(Color(.secondarySystemBackground))
        .cornerRadius(10)
    }
}
