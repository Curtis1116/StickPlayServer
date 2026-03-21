import Foundation

// MARK: - VideoEntry
struct VideoEntry: Codable, Identifiable {
    let id: String
    let title: String
    let actors: [String]
    let genres: [String]
    let level: String
    let rating: Double
    let criticrating: Int
    let releaseDate: String
    let dateAdded: String
    let videoPath: String
    let folderPath: String
    let posterPath: String?
    let nfoPath: String?
    let nfosPath: String?
    let isFavorite: Bool
    
    enum CodingKeys: String, CodingKey {
        case id, title, actors, genres, level, rating, criticrating
        case releaseDate = "release_date"
        case dateAdded = "date_added"
        case videoPath = "video_path"
        case folderPath = "folder_path"
        case posterPath = "poster_path"
        case nfoPath = "nfo_path"
        case nfosPath = "nfos_path"
        case isFavorite = "is_favorite"
    }
}

// MARK: - VideoFilter
struct VideoFilter: Codable {
    var search: String?
    var genres: [String]?
    var levels: [String]?
    var sortBy: String?
    var sortOrder: String?
    var favoritesOnly: Bool?
    
    enum CodingKeys: String, CodingKey {
        case search, genres, levels
        case sortBy = "sort_by"
        case sortOrder = "sort_order"
        case favoritesOnly = "favorites_only"
    }
}

// MARK: - QueryPayload
struct QueryPayload: Codable {
    let filter: VideoFilter
}

// MARK: - UpdateVideoInfoPayload
struct UpdateVideoInfoPayload: Codable {
    let originalId: String
    let videoId: String
    let title: String
    let level: String
    let rating: Double
    let criticrating: Int
    let actors: [String]
    let genres: [String]
    let releaseDate: String
    let dateAdded: String
    let isFavorite: Bool
    let isUncensored: Bool
    let videoPath: String
    let folderPath: String
    let posterPath: String?
    let nfoPath: String?
}

// MARK: - Library & Rescan
struct Library: Codable, Identifiable {
    let id: String
    let name: String
    let dbName: String
    let paths: [String]
    
    enum CodingKeys: String, CodingKey {
        case id, name, paths
        case dbName = "db_name"
    }
}


struct RescanPayload: Codable {
    let folderPath: String
}

struct ScanPathsPayload: Codable {
    let paths: [String]
}

struct SwitchDbPayload: Codable {
    let dbName: String
}
