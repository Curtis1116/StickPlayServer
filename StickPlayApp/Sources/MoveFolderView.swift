import SwiftUI

struct MoveFolderView: View {
    @Environment(\.dismiss) var dismiss
    @ObservedObject var api = APIService.shared
    
    let video: VideoEntry
    let onSaved: (VideoEntry) -> Void
    let onRemoved: (String) -> Void
    
    @State private var currentPath: String
    @State private var directories: [DirEntry] = []
    @State private var isLoading = false
    @State private var isMoving = false
    @State private var errorMessage: String?
    
    init(video: VideoEntry, onSaved: @escaping (VideoEntry) -> Void, onRemoved: @escaping (String) -> Void) {
        self.video = video
        self.onSaved = onSaved
        self.onRemoved = onRemoved
        
        let components = video.folderPath.split(separator: "/")
        if components.isEmpty {
            _currentPath = State(initialValue: "/media")
        } else {
            let parent = "/" + components.dropLast().joined(separator: "/")
            _currentPath = State(initialValue: parent == "/" ? "/media" : parent)
        }
    }
    
    var body: some View {
        NavigationView {
            ZStack {
                // Background gradient for a premium feel
                LinearGradient(
                    gradient: Gradient(colors: [Color(white: 0.98), Color(white: 0.94)]),
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                )
                .ignoresSafeArea()
                
                VStack(spacing: 16) {
                    // Header Info
                    VStack(alignment: .leading, spacing: 6) {
                        HStack(alignment: .top) {
                            Text("目前路徑：")
                                .font(.system(size: 12, weight: .bold))
                                .foregroundColor(.secondary)
                            Text(video.folderPath)
                                .font(.system(size: 12, design: .monospaced))
                                .foregroundColor(.primary)
                                .lineLimit(2)
                                .multilineTextAlignment(.leading)
                        }
                        
                        Text("請選擇要搬移至的目標資料夾：")
                            .font(.system(size: 13, weight: .medium))
                            .foregroundColor(.secondary)
                            .padding(.top, 4)
                    }
                    .padding(.horizontal)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    
                    // Folder Browser
                    VStack(spacing: 0) {
                        // Navigation Header (Breadcrumbs-ish)
                        HStack {
                            Image(systemName: "folder.circle.fill")
                                .foregroundColor(.indigo)
                            
                            Text(currentPath)
                                .font(.system(size: 11, weight: .semibold, design: .monospaced))
                                .foregroundColor(.indigo)
                                .lineLimit(1)
                            
                            Spacer()
                            
                            if currentPath != "/media" {
                                Button(action: goBack) {
                                    HStack(spacing: 4) {
                                        Image(systemName: "arrow.uturn.backward")
                                        Text("回上層")
                                    }
                                    .font(.system(size: 10, weight: .bold))
                                    .padding(.horizontal, 10)
                                    .padding(.vertical, 5)
                                    .background(Color.indigo.opacity(0.1))
                                    .foregroundColor(.indigo)
                                    .cornerRadius(8)
                                }
                            }
                        }
                        .padding(12)
                        .background(Color.indigo.opacity(0.05))
                        
                        Divider()
                            .opacity(0.5)
                        
                        // List Area
                        Group {
                            if isLoading {
                                Spacer()
                                ProgressView()
                                    .scaleEffect(1.2)
                                Spacer()
                            } else if directories.isEmpty {
                                Spacer()
                                VStack(spacing: 12) {
                                    Image(systemName: "folder.badge.questionmark")
                                        .font(.system(size: 40))
                                        .foregroundColor(.secondary.opacity(0.3))
                                    Text("此資料夾內無子資料夾")
                                        .font(.system(size: 13, weight: .medium))
                                        .foregroundColor(.secondary)
                                }
                                Spacer()
                            } else {
                                ScrollView {
                                    LazyVStack(spacing: 0) {
                                        ForEach(directories) { dir in
                                            Button(action: {
                                                currentPath = dir.path
                                                loadDirs()
                                            }) {
                                                HStack(spacing: 12) {
                                                    Image(systemName: "folder.fill")
                                                        .symbolRenderingMode(.hierarchical)
                                                        .foregroundColor(.indigo)
                                                        .font(.system(size: 18))
                                                    
                                                    Text(dir.name)
                                                        .font(.system(size: 14, weight: .medium))
                                                        .foregroundColor(.primary)
                                                    
                                                    Spacer()
                                                    
                                                    Image(systemName: "chevron.right")
                                                        .font(.system(size: 10, weight: .bold))
                                                        .foregroundColor(.secondary.opacity(0.4))
                                                }
                                                .padding(.horizontal, 16)
                                                .padding(.vertical, 12)
                                                .background(Color.primary.opacity(0.01))
                                            }
                                            .buttonStyle(.plain)
                                            
                                            Divider().padding(.leading, 46)
                                        }
                                    }
                                }
                            }
                        }
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(.ultraThinMaterial)
                    .cornerRadius(20)
                    .overlay(
                        RoundedRectangle(cornerRadius: 20)
                            .stroke(Color.primary.opacity(0.06), lineWidth: 1)
                    )
                    .padding(.horizontal)
                    .shadow(color: Color.black.opacity(0.03), radius: 10, x: 0, y: 5)
                    
                    // Action Buttons
                    HStack(spacing: 15) {
                        Button(action: { dismiss() }) {
                            Text("取消")
                                .font(.system(size: 15, weight: .bold))
                                .foregroundColor(.secondary)
                                .frame(maxWidth: .infinity)
                                .padding(.vertical, 14)
                                .background(Color.secondary.opacity(0.1))
                                .cornerRadius(15)
                        }
                        
                        Button(action: moveFolder) {
                            HStack(spacing: 8) {
                                if isMoving {
                                    ProgressView()
                                        .tint(.white)
                                } else {
                                    Image(systemName: "folder.badge.gearshape.fill")
                                }
                                Text("確定搬移至此")
                            }
                            .font(.system(size: 15, weight: .bold))
                            .foregroundColor(.white)
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 14)
                            .background(
                                LinearGradient(
                                    gradient: Gradient(colors: [Color.indigo, Color.indigo.opacity(0.8)]),
                                    startPoint: .top,
                                    endPoint: .bottom
                                )
                            )
                            .cornerRadius(15)
                            .shadow(color: Color.indigo.opacity(0.3), radius: 8, x: 0, y: 4)
                        }
                        .disabled(isMoving || isLoading)
                        .opacity(isMoving || isLoading ? 0.6 : 1.0)
                    }
                    .padding(.horizontal)
                    .padding(.bottom, 10)
                }
            }
            .navigationTitle("搬移影片資料夾")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: { dismiss() }) {
                        Image(systemName: "xmark.circle.fill")
                            .symbolRenderingMode(.hierarchical)
                            .foregroundColor(.secondary)
                            .font(.system(size: 20))
                    }
                }
            }
        }
        .onAppear {
            loadDirs()
        }
        .alert("搬移失敗", isPresented: .init(get: { errorMessage != nil }, set: { _ in errorMessage = nil })) {
            Button("確定", role: .cancel) { }
        } message: {
            Text(errorMessage ?? "")
        }
    }
    
    private func loadDirs() {
        isLoading = true
        Task {
            do {
                let dirs = try await api.listDirs(path: currentPath)
                await MainActor.run {
                    self.directories = dirs.filter { $0.isDir }
                    self.isLoading = false
                }
            } catch {
                await MainActor.run {
                    self.errorMessage = "讀取資料夾失敗: \(error.localizedDescription)"
                    self.isLoading = false
                }
            }
        }
    }
    
    private func goBack() {
        if currentPath == "/media" { return }
        let components = currentPath.split(separator: "/")
        if components.isEmpty { 
            currentPath = "/media"
        } else {
            let parent = "/" + components.dropLast().joined(separator: "/")
            currentPath = parent == "/" ? "/media" : parent
        }
        loadDirs()
    }
    
    private func moveFolder() {
        isMoving = true
        let payload = MoveFolderPayload(
            videoId: video.id,
            currentFolderPath: video.folderPath,
            targetParentFolder: currentPath
        )
        
        Task {
            do {
                let updated = try await api.moveVideoFolder(payload: payload)
                await MainActor.run {
                    onSaved(updated)
                    dismiss()
                }
            } catch {
                let msg = error.localizedDescription
                await MainActor.run {
                    if msg.contains("不在目前媒體庫的監控範圍內") {
                        onRemoved(video.id)
                        dismiss()
                    } else {
                        self.errorMessage = msg
                        self.isMoving = false
                    }
                }
            }
        }
    }
}
