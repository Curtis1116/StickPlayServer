import SwiftUI

struct VideoCard: View {
    let video: VideoEntry
    let onPlay: () -> Void
    let onEdit: () -> Void
    let onRescan: () -> Void
    let onUpdate: () -> Void
    let onDelete: () -> Void
    
    @State private var posterImage: UIImage?
    @State private var hasFailedToLoadImage = false
    @State private var debugMessage: String = ""
    @State private var showingMoveSheet = false
    
    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Poster Image
            Button(action: onPlay) {
                ZStack(alignment: .topTrailing) {
                    Color.gray.opacity(0.1)
                    
                    if let posterImage = posterImage {
                        Image(uiImage: posterImage)
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                    } else if hasFailedToLoadImage {
                        VStack(spacing: 4) {
                            Image(systemName: "film")
                                .font(.largeTitle)
                            if !debugMessage.isEmpty {
                                Text(debugMessage)
                                    .font(.system(size: 8))
                                    .foregroundColor(.red)
                                    .multilineTextAlignment(.center)
                            }
                        }
                        .padding(20)
                        .foregroundColor(.gray.opacity(0.4))
                    } else {
                        ProgressView()
                    }
                    
                    if video.isFavorite {
                        Image(systemName: "heart.fill")
                            .foregroundColor(.red)
                            .padding(8)
                            .background(.ultraThinMaterial)
                            .clipShape(Circle())
                            .padding(8)
                    }
                    
                    Text(video.id)
                        .font(.system(size: 10, weight: .bold))
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(.black.opacity(0.6))
                        .foregroundColor(.white)
                        .cornerRadius(4)
                        .padding(8)
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomLeading)
                }
                .aspectRatio(2/3, contentMode: .fit)
                .contentShape(Rectangle())
                .clipped()
            }
            .buttonStyle(PlainButtonStyle())
            
            // Metadata
            Button(action: onEdit) {
                HStack(alignment: .center, spacing: 6) {
                    HStack(spacing: 2) {
                        Image(systemName: "star.fill")
                            .foregroundColor(.yellow)
                        Text(String(format: "%.1f", video.rating))
                            .fontWeight(.medium)
                            .foregroundColor(.black)
                    }
                    
                    if !video.actors.isEmpty {
                        Text(video.actors.joined(separator: ", "))
                            .lineLimit(1)
                            .foregroundColor(.black)
                    }
                    
                    Spacer(minLength: 0)
                }
                .font(.system(size: 11))
                .padding(10)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(.regularMaterial)
                .contentShape(Rectangle())
            }
            .buttonStyle(PlainButtonStyle())
        }
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.1), radius: 5, x: 0, y: 2)
        .contextMenu {
            Button(action: onRescan) {
                Label("重整此影片", systemImage: "arrow.clockwise")
            }
            Button(action: { showingMoveSheet = true }) {
                Label("搬移資料夾", systemImage: "folder.badge.gearshape")
            }
            Button(action: onEdit) {
                Label("編輯資訊", systemImage: "pencil")
            }
        }
        .sheet(isPresented: $showingMoveSheet) {
            MoveFolderView(
                video: video,
                onSaved: { _ in onUpdate() },
                onRemoved: { _ in onDelete() }
            )
        }
        .task(id: video.id) {
            await loadPoster()
        }
    }
    
    private func loadPoster() async {
        await MainActor.run { 
            self.posterImage = nil 
            self.hasFailedToLoadImage = false 
            self.debugMessage = ""
        }
        
        let path = video.posterPath ?? video.folderPath
        if path.isEmpty {
            await MainActor.run { 
                self.hasFailedToLoadImage = true
                self.debugMessage = "No Path" 
            }
            return
        }
        
        guard let url = APIService.shared.getImageUrl(path: path, id: video.id) else {
            await MainActor.run { 
                self.hasFailedToLoadImage = true
                self.debugMessage = "URL Err" 
            }
            return
        }
        
        do {
            let (data, response) = try await URLSession.shared.data(from: url)
            if let httpRes = response as? HTTPURLResponse {
                if httpRes.statusCode == 200, let image = UIImage(data: data) {
                    await MainActor.run {
                        self.posterImage = image
                        self.hasFailedToLoadImage = false
                    }
                } else {
                    await MainActor.run { 
                        self.hasFailedToLoadImage = true
                        self.debugMessage = "HTTP \(httpRes.statusCode)" 
                    }
                }
            }
        } catch {
            await MainActor.run { 
                self.hasFailedToLoadImage = true
                self.debugMessage = "Err: \(error.localizedDescription)" 
            }
        }
    }
}
