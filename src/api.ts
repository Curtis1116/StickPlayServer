import { VideoEntry, VideoFilter } from "./types";

const API_BASE = "/api";

async function post<T>(endpoint: string, payload?: any): Promise<T> {
    const res = await fetch(`${API_BASE}/${endpoint}`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: payload ? JSON.stringify(payload) : undefined
    });
    if (!res.ok) {
        throw new Error(await res.text() || res.statusText);
    }
    return res.json();
}

/// 掃描媒體庫
export async function scanLibrary(paths: string[]): Promise<number> {
    return post<number>("scan_library", { paths });
}

/// 重新掃描單一影片（更新索引 + 重新生成海報）
export async function rescanSingleVideo(folderPath: string): Promise<VideoEntry> {
    return post<VideoEntry>("rescan_single_video", { folderPath });
}

/// 從 DB 查詢影片列表
export async function queryVideos(filter: VideoFilter): Promise<VideoEntry[]> {
    return post<VideoEntry[]>("query_videos", { filter });
}

/// 取得影片縮圖 (同檔名或 fanart)
export async function getFanartPath(folderPath: string, videoPath: string): Promise<string> {
    return post<string>("get_fanart_path", { folderPath, videoPath });
}

/// 更新影片完整資訊（DB + .nfos 雙寫）
export async function updateVideoInfo(
    originalId: string,
    videoId: string,
    title: string,
    level: string,
    rating: number,
    actors: string[],
    releaseDate: string,
    dateAdded: string,
    isFavorite: boolean,
    isUncensored: boolean,
    videoPath: string,
    folderPath: string,
    posterPath: string | null,
    nfoPath: string | null,
    nfosPath: string | null
): Promise<string> {
    return post<string>("update_video_info", {
        originalId,
        videoId,
        title,
        level,
        rating,
        actors,
        releaseDate,
        dateAdded,
        isFavorite,
        isUncensored,
        videoPath,
        folderPath,
        posterPath,
        nfoPath,
        nfosPath,
    });
}

/// 更新評分（DB + .nfos 雙寫，永不修改原始 .nfo）
export async function updateRating(
    videoId: string,
    rating: number,
    nfoPath: string | null,
    nfosPath: string | null,
    folderPath: string | null
): Promise<string> {
    return post<string>("update_rating", { videoId, rating, nfoPath, nfosPath, folderPath });
}

/// 切換我的最愛
export async function toggleFavorite(videoId: string): Promise<boolean> {
    return post<boolean>("toggle_favorite", { videoId });
}

/// 取得所有類型
export async function getAllGenres(): Promise<string[]> {
    return post<string[]>("get_all_genres");
}

/// 取得所有等級
export async function getAllLevels(): Promise<string[]> {
    return post<string[]>("get_all_levels");
}

/// 取得統計資訊 (total, favorites)
export async function getStats(): Promise<[number, number]> {
    return post<[number, number]>("get_stats");
}

/// 在網頁直接開啟影片
export async function openVideo(path: string): Promise<void> {
    window.open(`/api/video?path=${encodeURIComponent(path)}`, '_blank');
}

/// 在網頁無法直接打開檔案管理員，發出提醒
export async function openFolder(path: string): Promise<void> {
    console.warn("網頁版不支援直接開啟本地資料夾", path);
    alert("網頁版無法直接開啟本地資料夾");
}

export async function switchDatabase(dbName: string): Promise<void> {
    return post<void>("switch_database", { dbName });
}

export async function deleteDatabase(dbName: string): Promise<void> {
    return post<void>("delete_database", { dbName });
}

/// 回傳圖片伺服器網址
export async function readImage(path: string): Promise<string> {
    return `/api/image?path=${encodeURIComponent(path)}`;
}

/// 列出伺服器資料夾
export async function listDirs(path?: string): Promise<any[]> {
    return post<any[]>("list_dirs", { path });
}

/// 同步監控路徑
export async function syncWatchPaths(paths: string[]): Promise<void> {
    return post<void>("sync_watch_paths", { paths });
}

/// 取得伺服器儲存的媒體庫清單
export async function getLibraries(): Promise<any[]> {
    return post<any[]>("get_libraries", {});
}

/// 儲存媒體庫清單至伺服器
export async function saveLibraries(libs: any[]): Promise<void> {
    return post<void>("save_libraries", libs);
}
