import { VideoEntry, VideoFilter } from "./types";
import {
    androidIntentUrl,
    detectDevicePlatform,
    getPlayerPreference,
} from "./player";
export {
    getAvailablePlayers,
    getPlayerPreference,
    setPlayerPreference,
    type PlayerChoice,
} from "./player";

import { csrfToken, expired, refreshSession } from "./auth";
let libraryId = "";
let switchSequence = 0;
export function selectedLibrary() { return libraryId; }
export function clearLibrary() { libraryId = ""; }
export function scopedUrl(path: string) { return `${path}${path.includes("?") ? "&" : "?"}libraryId=${encodeURIComponent(libraryId)}`; }

const API_BASE = "/api";

async function post<T>(endpoint: string, payload?: any): Promise<T> {
    const res = await fetch(`${API_BASE}/${endpoint}`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'X-CSRF-Token': csrfToken(),
            'X-Library-Id': libraryId
        },
        body: payload ? JSON.stringify(payload) : undefined
    });
    if (res.status === 401) expired();
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

export function isIOSDevice(): boolean {
    return detectDevicePlatform() === "ios";
}

/// 判斷是否為 Windows 桌面瀏覽器（用於放大海報卡片的顯示尺寸；300x450 縮圖本身
/// 解析度已足夠，純粹是卡片版面在 Windows 下顯示更大張）
export function isWindowsDevice(): boolean {
    return detectDevicePlatform() === "windows";
}

/// 依設定開啟影片：瀏覽器分頁播放，或喚起本機播放器並帶入影片網址
export async function openVideo(path: string, videoId: string): Promise<void> {
    const player = getPlayerPreference();
    const popup = player === 'browser' ? window.open('about:blank', '_blank') : null;
    if (popup) popup.opener = null;
    let absoluteUrl: string;
    try {
        absoluteUrl = window.location.origin + (player === 'browser' ? scopedUrl(`/api/video?path=${encodeURIComponent(path)}`) : await post<string>('playback-tickets', { videoId }));
    } catch (e) { popup?.close(); throw e; }

    switch (player) {
        case "potplayer":
            window.location.href = `stickplay-potplayer:${encodeURIComponent(absoluteUrl)}`;
            break;
        case "vlc":
            switch (detectDevicePlatform()) {
                case "ios":
                    window.location.href = `vlc-x-callback://x-callback-url/stream?url=${encodeURIComponent(absoluteUrl)}`;
                    break;
                case "android":
                    window.location.href = androidIntentUrl(absoluteUrl, "org.videolan.vlc");
                    break;
                default:
                    window.location.href = `stickplay-vlc:${encodeURIComponent(absoluteUrl)}`;
            }
            break;
        case "infuse":
            window.location.href = `infuse://x-callback-url/play?url=${encodeURIComponent(absoluteUrl)}`;
            break;
        case "justplayer":
            window.location.href = androidIntentUrl(absoluteUrl, "com.brouken.player");
            break;
        default:
            if (popup) popup.location.href = absoluteUrl;
            else throw new Error("請允許彈出視窗以播放影片");
    }
}

/// 在網頁無法直接打開檔案管理員，發出提醒
export async function openFolder(path: string): Promise<void> {
    console.warn("網頁版不支援直接開啟本地資料夾", path);
    alert("網頁版無法直接開啟本地資料夾");
}

export async function switchDatabase(dbName: string): Promise<void> {
    const sequence = ++switchSequence;
    const id = await post<string>("switch_database", { dbName });
    if (sequence !== switchSequence) throw new Error("已改為切換至其他媒體庫");
    libraryId = id;
}

export async function deleteDatabase(dbName: string): Promise<import("./types").Library[]> {
    return post<import("./types").Library[]>("delete_database", { dbName });
}

/// 回傳圖片伺服器網址
/// version 用於強制瀏覽器在圖片內容變更後（例如手動裁切、重新索引）重新抓取，
/// 而非沿用同一組 path/id 對應到的舊快取內容
export async function readImage(path: string, id?: string, thumb: boolean = true, version?: number): Promise<string> {
    let url = `/api/image?path=${encodeURIComponent(path)}`;
    if (id) url += `&id=${encodeURIComponent(id)}`;
    if (thumb) url += `&thumb=true`;
    if (version) url += `&v=${version}`;
    return scopedUrl(url);
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
    if (!libraryId) return () => {};
    const es = new EventSource(scopedUrl('/api/events'));
    const close = () => es.close();
    window.addEventListener('stickplay-auth-expired', close);
    es.onmessage = (e) => {
        if (e.data === 'library_updated') {
            console.log('[SSE] 收到媒體庫更新通知');
            onUpdate();
        }
    };
    es.onerror = () => {
        refreshSession().catch(() => { es.close(); expired(); });
    };
    return () => { es.close(); window.removeEventListener('stickplay-auth-expired', close); };
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
