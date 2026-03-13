/// 影片資料結構
export interface VideoEntry {
    id: string;
    title: string;
    actors: string[];
    genres: string[];
    level: string;
    rating: number;
    release_date: string;
    date_added: string;
    video_path: string;
    folder_path: string;
    poster_path: string | null;
    nfo_path: string | null;
    nfos_path: string | null;
    is_favorite: boolean;
}

/// 篩選參數
export interface VideoFilter {
    genres?: string[];
    levels?: string[];
    search?: string;
    sort_by?: string;
    sort_order?: string;
    favorites_only?: boolean;
}

/// 媒體庫設定
export interface Library {
    id: string;
    name: string;
    paths: string[];
    db_name: string;
}
