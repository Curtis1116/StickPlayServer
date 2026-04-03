import Foundation
import Network

class APIService: ObservableObject {
    static let shared = APIService()
    
    @Published var serverHost: String {
        didSet {
            UserDefaults.standard.set(serverHost, forKey: "serverHost")
            UserDefaults.standard.synchronize()
        }
    }
    
    @Published var serverPort: String {
        didSet {
            UserDefaults.standard.set(serverPort, forKey: "serverPort")
            UserDefaults.standard.synchronize()
        }
    }
    
    @Published var isVPNConnected: Bool = false
    private let monitor = NWPathMonitor()
    
    init() {
        self.serverHost = UserDefaults.standard.string(forKey: "serverHost") ?? "192.168.1.100"
        self.serverPort = UserDefaults.standard.string(forKey: "serverPort") ?? "8099"
        startVPNMonitor()
    }
    
    var baseURL: String {
        let host = serverHost.trimmingCharacters(in: .whitespacesAndNewlines)
        let port = serverPort.trimmingCharacters(in: .whitespacesAndNewlines)
        
        let protocolStr = (host.hasPrefix("http://") || host.hasPrefix("https://")) ? "" : "http://"
        let portStr = port.isEmpty ? "" : ":\(port)"
        let finalStr = "\(protocolStr)\(host)\(portStr)"
        
        // Remove any unintentional spaces or newlines that trimming missed (e.g. inside the string)
        let sanitized = finalStr.replacingOccurrences(of: " ", with: "").replacingOccurrences(of: "\n", with: "").replacingOccurrences(of: "\r", with: "")
        
        print("BASE_URL: [\(sanitized)]") // Debug log for Xcode
        return sanitized
    }
    
    private func startVPNMonitor() {
        self.isVPNConnected = self.isVPNActive()
        monitor.pathUpdateHandler = { [weak self] _ in
            Task { @MainActor in
                self?.isVPNConnected = self?.isVPNActive() ?? false
            }
        }
        let queue = DispatchQueue(label: "NetworkMonitor")
        monitor.start(queue: queue)
    }
    
    private func isVPNActive() -> Bool {
        guard let settings = CFNetworkCopySystemProxySettings()?.takeRetainedValue() as? [String: Any],
              let scoped = settings["__SCOPED__"] as? [String: Any] else {
            return false
        }
        for key in scoped.keys {
            let lowerKey = key.lowercased()
            if lowerKey.contains("tap") || lowerKey.contains("tun") || lowerKey.contains("ppp") || lowerKey.contains("ipsec") || lowerKey.contains("utun") {
                return true
            }
        }
        return false
    }
    
    func fetchVideos(filter: VideoFilter) async throws -> [VideoEntry] {
        guard let url = URL(string: "\(baseURL)/api/query_videos") else {
            throw URLError(.badURL)
        }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(QueryPayload(filter: filter))
        
        let (data, response) = try await URLSession.shared.data(for: request)
        guard let httpRes = response as? HTTPURLResponse, httpRes.statusCode == 200 else {
            throw URLError(.badServerResponse)
        }
        
        // CRITICAL FIX: Since Models.swift ALREADY defines explicit CodingKeys for snake_case mapping,
        // we MUST NOT use .convertFromSnakeCase here, as it would try to double-convert or fail.
        let decoder = JSONDecoder()
        do {
            return try decoder.decode([VideoEntry].self, from: data)
        } catch {
            print("DECODE ERROR: \(error)")
            throw error
        }
    }
    
    func fetchAllLevels() async throws -> [String] {
        guard let url = URL(string: "\(baseURL)/api/get_all_levels") else {
            throw URLError(.badURL)
        }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        
        do {
            let (data, response) = try await URLSession.shared.data(for: request)
            guard let httpRes = response as? HTTPURLResponse else { return [] }
            if httpRes.statusCode != 200 {
                let msg = String(data: data, encoding: .utf8) ?? "No body"
                print("FETCH_ALL_LEVELS Error [\(httpRes.statusCode)]: \(msg)")
                return []
            }
            return try JSONDecoder().decode([String].self, from: data)
        } catch {
            print("FETCH_ALL_LEVELS Failure: \(error.localizedDescription)")
            return []
        }
    }
    
    func updateVideoInfo(payload: UpdateVideoInfoPayload) async throws {
        guard let url = URL(string: "\(baseURL)/api/update_video_info") else {
            throw URLError(.badURL)
        }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.httpBody = try JSONEncoder().encode(payload)
        
        do {
            let (data, response) = try await URLSession.shared.data(for: request)
            if let httpRes = response as? HTTPURLResponse, httpRes.statusCode != 200 {
                let msg = String(data: data, encoding: .utf8) ?? "No body"
                print("UPDATE_VIDEO_INFO Error [\(httpRes.statusCode)]: \(msg)")
                throw NSError(domain: "APIService", code: httpRes.statusCode, userInfo: [NSLocalizedDescriptionKey: "Server [\(httpRes.statusCode)]: \(msg)"])
            }
        } catch {
            print("UPDATE_VIDEO_INFO Failure: \(error.localizedDescription)")
            throw error
        }
    }
    
    // MARK: - Library & Management
    
    func fetchLibraries() async throws -> [Library] {
        guard let url = URL(string: "\(baseURL)/api/get_libraries") else { 
            print("FETCH_LIBRARIES ERROR: Invalid URL \(baseURL)/api/get_libraries")
            throw URLError(.badURL) 
        }
        
        print("FETCH_LIBRARIES: Requesting \(url.absoluteString)")
        
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        
        do {
            let (data, response) = try await URLSession.shared.data(for: request)
            
            if let httpRes = response as? HTTPURLResponse {
                print("FETCH_LIBRARIES Response Status: \(httpRes.statusCode)")
                if httpRes.statusCode != 200 {
                    let msg = String(data: data, encoding: .utf8) ?? "No body"
                    print("FETCH_LIBRARIES Server Error: \(msg)")
                    throw URLError(.badServerResponse)
                }
            }
            
            let libs = try JSONDecoder().decode([Library].self, from: data)
            print("FETCH_LIBRARIES Success: Found \(libs.count) libraries")
            return libs
        } catch {
            print("FETCH_LIBRARIES Failure: \(error.localizedDescription)")
            throw error
        }
    }
    
    func switchLibrary(name: String) async throws {
        guard let url = URL(string: "\(baseURL)/api/switch_database") else { throw URLError(.badURL) }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        let payload = SwitchDbPayload(dbName: name)
        request.httpBody = try JSONEncoder().encode(payload)
        
        let (data, response) = try await URLSession.shared.data(for: request)
        if let httpRes = response as? HTTPURLResponse, httpRes.statusCode != 200 {
            let serverMsg = String(data: data, encoding: .utf8) ?? "Unknown error"
            print("SWITCH_DATABASE ERROR [\(httpRes.statusCode)]: \(serverMsg)")
            throw NSError(domain: "APIService", code: httpRes.statusCode, userInfo: [NSLocalizedDescriptionKey: "Server [\(httpRes.statusCode)]: \(serverMsg)"])
        }
    }

    
    func scanLibrary(paths: [String]) async throws -> Int {
        guard let url = URL(string: "\(baseURL)/api/scan_library") else { throw URLError(.badURL) }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(ScanPathsPayload(paths: paths))
        let (data, _) = try await URLSession.shared.data(for: request)
        return try JSONDecoder().decode(Int.self, from: data)
    }
    
    func syncWatchPaths(paths: [String]) async throws {
        guard let url = URL(string: "\(baseURL)/api/sync_watch_paths") else { throw URLError(.badURL) }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(ScanPathsPayload(paths: paths))
        
        let (data, response) = try await URLSession.shared.data(for: request)
        if let httpRes = response as? HTTPURLResponse, httpRes.statusCode != 200 {
            let serverMsg = String(data: data, encoding: .utf8) ?? "Unknown error"
            throw NSError(domain: "APIService", code: httpRes.statusCode, userInfo: [NSLocalizedDescriptionKey: "Server [\(httpRes.statusCode)]: \(serverMsg)"])
        }
    }
    
    func rescanVideo(folderPath: String) async throws -> VideoEntry {
        guard let url = URL(string: "\(baseURL)/api/rescan_single_video") else { throw URLError(.badURL) }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(RescanPayload(folderPath: folderPath))
        let (data, _) = try await URLSession.shared.data(for: request)
        return try JSONDecoder().decode(VideoEntry.self, from: data)
    }
    
    func listDirs(path: String?) async throws -> [DirEntry] {
        guard let url = URL(string: "\(baseURL)/api/list_dirs") else { throw URLError(.badURL) }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(ListDirsPayload(path: path))
        
        let (data, response) = try await URLSession.shared.data(for: request)
        if let httpRes = response as? HTTPURLResponse, httpRes.statusCode != 200 {
            let serverMsg = String(data: data, encoding: .utf8) ?? "Unknown error"
            print("LIST_DIRS Error [\(httpRes.statusCode)]: \(serverMsg)")
            throw URLError(.badServerResponse)
        }
        
        do {
            return try JSONDecoder().decode([DirEntry].self, from: data)
        } catch {
            print("DECODE ERROR IN LIST_DIRS: \(error)")
            throw error
        }
    }
    
    func moveVideoFolder(payload: MoveFolderPayload) async throws -> VideoEntry {
        guard let url = URL(string: "\(baseURL)/api/move_video_folder") else { throw URLError(.badURL) }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(payload)
        
        let (data, response) = try await URLSession.shared.data(for: request)
        if let httpRes = response as? HTTPURLResponse, httpRes.statusCode != 200 {
            let msg = String(data: data, encoding: .utf8) ?? "搬移失敗"
            throw NSError(domain: "APIService", code: httpRes.statusCode, userInfo: [NSLocalizedDescriptionKey: msg])
        }
        return try JSONDecoder().decode(VideoEntry.self, from: data)
    }

    
    func getImageUrl(path: String?, id: String?) -> URL? {
        guard let path = path, !path.isEmpty else { return nil }
        
        // Use standard encoding but more permissive.
        let allowed = CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "-._~:/")) 
        guard let escapedPath = path.addingPercentEncoding(withAllowedCharacters: allowed) else { return nil }
        
        var urlStr = "\(baseURL)/api/image?path=\(escapedPath)"
        if let id = id {
            urlStr += "&id=\(id)&thumb=true"
        }
        return URL(string: urlStr)
    }
    
    func getVideoUrl(path: String?) -> URL? {
        guard let path = path, !path.isEmpty else { return nil }
        let allowed = CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "-._~:/"))
        guard let escapedPath = path.addingPercentEncoding(withAllowedCharacters: allowed) else { return nil }
        return URL(string: "\(baseURL)/api/video?path=\(escapedPath)")
    }
}
