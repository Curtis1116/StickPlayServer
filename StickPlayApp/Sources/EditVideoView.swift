import SwiftUI

struct EditVideoView: View {
    @Environment(\.dismiss) var dismiss
    let video: VideoEntry
    let onSave: () async -> Void
    
    @State private var criticRating: Int
    @State private var id: String
    @State private var level: String
    @State private var actorsStr: String
    @State private var releaseDate: String
    @State private var dateAdded: String
    @State private var isFavorite: Bool
    @State private var isUncensored: Bool
    
    @State private var isSaving = false
    @State private var errorMsg: String?
    
    init(video: VideoEntry, onSave: @escaping () async -> Void) {
        self.video = video
        self.onSave = onSave
        
        _criticRating = State(initialValue: video.criticrating)
        _id = State(initialValue: video.id)
        _level = State(initialValue: video.level)
        _actorsStr = State(initialValue: video.actors.joined(separator: ", "))
        _releaseDate = State(initialValue: video.releaseDate)
        _dateAdded = State(initialValue: video.dateAdded)
        _isFavorite = State(initialValue: video.isFavorite)
        
        let initialUncensored = video.genres.contains("無碼") || video.level.lowercased().hasSuffix("x")
        _isUncensored = State(initialValue: initialUncensored)
    }
    
    var body: some View {
        VStack(spacing: 0) {
            // Custom Header instead of NavigationView Toolbar to ensure Title is NOT a button
            HStack {
                Text("編輯影片資訊")
                    .font(.headline)
                    .fontWeight(.bold)
                
                Spacer()
                
                HStack(spacing: 12) {
                    Button("取消") {
                        dismiss()
                    }
                    .buttonStyle(.bordered)
                    
                    if isSaving {
                        ProgressView()
                    } else {
                        Button("儲存變更") {
                            Task { await saveChanges() }
                        }
                        .buttonStyle(.borderedProminent)
                        .fontWeight(.bold)
                    }
                }
            }
            .padding()
            .background(Color(UIColor.systemBackground))
            .overlay(Divider(), alignment: .bottom)
            
            ScrollView {
                VStack(spacing: 24) {
                    // Top Section: Critic Rating & Status Toggles
                    VStack(spacing: 16) {
                        // ... (rest of the content remains the same)
                        
                        // Critic Rating Card
                        GroupBox {
                            HStack(spacing: 12) {
                                HStack(spacing: 8) {
                                    Button(action: { criticRating = max(0, criticRating - 1) }) {
                                        Image(systemName: "minus")
                                    }
                                    .buttonStyle(.bordered)
                                    .clipShape(Circle())
                                    
                                    TextField("0", value: $criticRating, formatter: NumberFormatter())
                                        .keyboardType(.numberPad)
                                        .multilineTextAlignment(.center)
                                        .font(.system(size: 20, weight: .bold))
                                        .foregroundColor(.orange)
                                        .frame(width: 60)
                                    
                                    Button(action: { criticRating = min(100, criticRating + 1) }) {
                                        Image(systemName: "plus")
                                    }
                                    .buttonStyle(.bordered)
                                    .clipShape(Circle())
                                }
                                
                                HStack(spacing: 8) {
                                    Button("-5") { criticRating = max(0, criticRating - 5) }
                                        .buttonStyle(.bordered)
                                        
                                    Button("+5") { criticRating = min(100, criticRating + 5) }
                                        .buttonStyle(.bordered)
                                }
                                
                                Spacer()
                                
                                Button(action: { criticRating = 0 }) {
                                    Image(systemName: "trash")
                                }
                                .buttonStyle(.borderless)
                                .foregroundColor(.red)
                            }
                            .padding(.vertical, 4)
                        }
                        
                        // Status Toggles
                        HStack(spacing: 12) {
                            Button(action: { isFavorite.toggle() }) {
                                HStack(spacing: 6) {
                                    Image(systemName: isFavorite ? "star.fill" : "star.slash")
                                    Text("最愛")
                                        .fontWeight(.bold)
                                }
                                .frame(maxWidth: .infinity)
                                .padding(.vertical, 8)
                            }
                            .buttonStyle(.borderedProminent)
                            .tint(isFavorite ? .blue : .gray)
                            
                            Button(action: { isUncensored.toggle() }) {
                                HStack(spacing: 6) {
                                    Text("無碼模式")
                                        .fontWeight(.bold)
                                }
                                .frame(maxWidth: .infinity)
                                .padding(.vertical, 8)
                            }
                            .buttonStyle(.borderedProminent)
                            .tint(isUncensored ? .red : .gray)
                        }
                    }
                    
                    // Details Form Blocks
                    GroupBox {
                        VStack(spacing: 16) {
                            // Custom Row for ID
                            HStack(spacing: 12) {
                                Text("影片 ID")
                                    .font(.system(size: 14, weight: .bold))
                                    .foregroundColor(.secondary)
                                    .frame(width: 70, alignment: .trailing)
                                
                                TextField("ID", text: $id)
                                    .textFieldStyle(RoundedBorderTextFieldStyle())
                            }
                            
                            // Custom Row for Actors
                            HStack(spacing: 12) {
                                Text("演員清單")
                                    .font(.system(size: 14, weight: .bold))
                                    .foregroundColor(.secondary)
                                    .frame(width: 70, alignment: .trailing)
                                
                                TextField("以逗號分隔", text: $actorsStr)
                                    .textFieldStyle(RoundedBorderTextFieldStyle())
                            }
                            
                            // Custom Row for Release Date
                            HStack(spacing: 12) {
                                Text("發行日期")
                                    .font(.system(size: 14, weight: .bold))
                                    .foregroundColor(.secondary)
                                    .frame(width: 70, alignment: .trailing)
                                
                                TextField("發行日期", text: $releaseDate)
                                    .textFieldStyle(RoundedBorderTextFieldStyle())
                            }
                            
                            // Custom Row for Scan Date
                            HStack(spacing: 12) {
                                Text("掃描時間")
                                    .font(.system(size: 14, weight: .bold))
                                    .foregroundColor(.secondary)
                                    .frame(width: 70, alignment: .trailing)
                                
                                TextField("掃描時間", text: $dateAdded)
                                    .textFieldStyle(RoundedBorderTextFieldStyle())
                            }
                        }
                    }
                    
                    if let errorMsg = errorMsg {
                        Text(errorMsg)
                            .foregroundColor(.red)
                            .font(.system(size: 12))
                            .padding()
                    }
                }
                .padding()
            }
            .navigationTitle("")
            .navigationBarTitleDisplayMode(.inline)
            // Removed old toolbar items as they are now in the custom header
        }
        // For a complete native look:
        .background(Color(UIColor.systemGroupedBackground))
    }
    
    private func saveChanges() async {
        isSaving = true
        errorMsg = nil
        
        // Exact logic from React handleSave
        let actorsList = actorsStr
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }
            
        var newLevel = level
        if isUncensored && !newLevel.lowercased().hasSuffix("x") {
            newLevel += "X"
        } else if !isUncensored && newLevel.lowercased().hasSuffix("x") {
            newLevel = String(newLevel.dropLast())
        }
        
        var newGenres = video.genres.filter { $0 != "無碼" }
        if isUncensored {
            newGenres.append("無碼")
        }
        
        let payload = UpdateVideoInfoPayload(
            originalId: video.id,
            videoId: id, // user modified ID
            title: video.title, // untouched in UI
            level: newLevel,
            rating: Double(criticRating) / 10.0,
            criticrating: criticRating,
            actors: actorsList,
            genres: newGenres,
            releaseDate: releaseDate,
            dateAdded: dateAdded,
            isFavorite: isFavorite,
            isUncensored: isUncensored,
            videoPath: video.videoPath,
            folderPath: video.folderPath,
            posterPath: video.posterPath,
            nfoPath: video.nfoPath
        )
        
        do {
            try await APIService.shared.updateVideoInfo(payload: payload)
            await onSave()
            await MainActor.run {
                isSaving = false
                dismiss()
            }
        } catch {
            await MainActor.run {
                errorMsg = error.localizedDescription
                isSaving = false
            }
        }
    }
}
