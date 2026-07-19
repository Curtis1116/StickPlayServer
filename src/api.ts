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
    criticrating: number
): Promise<string> {
    return post<string>("update_video_info", {
        originalId,
        videoId,
        title,
        level,
        rating,
        criticrating,
        actors,
        releaseDate,
        dateAdded,
        isFavorite,
        isUncensored,
        videoPath,
        folderPath,
        posterPath,
        nfoPath,
    });
}

/// 更新評分（DB + .nfos 雙寫，永不修改原始 .nfo）
export async function updateRating(
    videoId: string,
    rating: number,
    criticrating: number,
    nfoPath: string | null,
    folderPath: string | null
): Promise<string> {
    return post<string>("update_rating", { videoId, rating, criticrating, nfoPath, folderPath });
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

export type PlayerChoice = "browser" | "potplayer" | "vlc" | "infuse";

const PLAYER_STORE_KEY = "stickplay_player";

/// 判斷是否為 iOS / iPadOS（iPadOS 13+ 的 Safari 會偽裝成 Mac UA，需搭配觸控點數輔助判斷）
export function isIOSDevice(): boolean {
    const ua = navigator.userAgent;
    if (/iPhone|iPad|iPod/.test(ua)) return true;
    return navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1;
}

/// 依平台回傳此裝置可選的播放器清單（PotPlayer 僅 Windows 有，Infuse 僅 iOS/iPadOS/macOS 有）
export function getAvailablePlayers(): PlayerChoice[] {
    return isIOSDevice() ? ["browser", "vlc", "infuse"] : ["browser", "potplayer", "vlc"];
}

/// 讀取使用者選擇的播放器（存於本機瀏覽器 localStorage，每台裝置各自獨立）
export function getPlayerPreference(): PlayerChoice {
    const stored = localStorage.getItem(PLAYER_STORE_KEY) as PlayerChoice | null;
    const available = getAvailablePlayers();
    return stored && available.includes(stored) ? stored : "browser";
}

/// 儲存播放器選擇
export function setPlayerPreference(player: PlayerChoice): void {
    localStorage.setItem(PLAYER_STORE_KEY, player);
}

/// 依設定開啟影片：瀏覽器分頁播放，或喚起本機播放器並帶入影片網址
export async function openVideo(path: string): Promise<void> {
    const absoluteUrl = `${window.location.origin}/api/video?path=${encodeURIComponent(path)}`;
    const player = getPlayerPreference();

    switch (player) {
        case "potplayer":
            window.location.href = `stickplay-potplayer:${encodeURIComponent(absoluteUrl)}`;
            break;
        case "vlc":
            window.location.href = isIOSDevice()
                ? `vlc-x-callback://x-callback-url/stream?url=${encodeURIComponent(absoluteUrl)}`
                : `stickplay-vlc:${encodeURIComponent(absoluteUrl)}`;
            break;
        case "infuse":
            window.location.href = `infuse://x-callback-url/play?url=${encodeURIComponent(absoluteUrl)}`;
            break;
        default:
            window.open(absoluteUrl, "_blank");
    }
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
export async function readImage(path: string, id?: string, thumb: boolean = true): Promise<string> {
    let url = `/api/image?path=${encodeURIComponent(path)}`;
    if (id) url += `&id=${encodeURIComponent(id)}`;
    if (thumb) url += `&thumb=true`;
    return url;
}

/// 列出伺服器資料夾
export async function listDirs(path?: string): Promise<any[]> {
    return post<any[]>("list_dirs", { path });
}

/// 同步監控路徑
export async function syncWatchPaths(paths: string[]): Promise<void> {
    return post<void>("sync_watch_paths", { paths });
}

/// 訂閱 SSE 即時通知（媒體庫變更時自動觸發 onUpdate）
export function subscribeToEvents(onUpdate: () => void): () => void {
    const es = new EventSource('/api/events');
    es.onmessage = (e) => {
        if (e.data === 'library_updated') {
            console.log('[SSE] 收到媒體庫更新通知');
            onUpdate();
        }
    };
    es.onerror = () => {
        console.warn('[SSE] 連線中斷，將自動重連');
    };
    return () => es.close();
}

/// 取得伺服器儲存的媒體庫清單
export async function getLibraries(): Promise<any[]> {
    return post<any[]>("get_libraries", {});
}

/// 儲存媒體庫清單至伺服器
export async function saveLibraries(libs: any[]): Promise<void> {
    return post<void>("save_libraries", libs);
}

/// 取得資料夾內所有圖片
export async function getFolderImages(folderPath: string): Promise<string[]> {
    return post<string[]>("get_folder_images", { folderPath });
}

/// 裁切並儲存海報
export async function cropAndSavePoster(
    imagePath: string,
    x: number,
    y: number,
    width: number,
    height: number,
    outputFolder: string,
    videoId?: string
): Promise<string> {
    return post<string>("crop_and_save_poster", {
        imagePath,
        x,
        y,
        width,
        height,
        outputFolder,
        videoId,
    });
}

/// 搬移影片資料夾
export async function moveVideoFolder(
    videoId: string,
    currentFolderPath: string,
    targetParentFolder: string
): Promise<VideoEntry> {
    return post<VideoEntry>("move_video_folder", {
        videoId,
        currentFolderPath,
        targetParentFolder
    });
}

