import { useEffect, useRef, useState } from "react";
import { Star, Play, FolderOpen, RefreshCw } from "lucide-react";
import { VideoEntry } from "../types";
import { openVideo, openFolder, toggleFavorite, rescanSingleVideo, readImage } from "../api";
import EditVideoModal from "./EditVideoModal";

interface VideoCardProps {
    video: VideoEntry;
    onFavoriteToggled: (id: string, newState: boolean) => void;
    onRatingUpdated: (id: string, newRating: number, nfosPath: string) => void;
    onVideoUpdated: (updated: VideoEntry) => void;
    onVideoRemoved?: (id: string) => void;
    onToast: (msg: string) => void;
}

export default function VideoCard({
    video,
    onFavoriteToggled,
    onRatingUpdated,
    onVideoUpdated,
    onVideoRemoved,
    onToast,
}: VideoCardProps) {
    const [showRating, setShowRating] = useState(false);
    const [rescanning, setRescanning] = useState(false);
    const [posterUrl, setPosterUrl] = useState<string | null>(null);
    const [updateTrigger, setUpdateTrigger] = useState(0);

    const cardRef = useRef<HTMLDivElement>(null);

    // 透過 Tauri IPC 讀取海報圖片（支援 NAS/任意路徑），並加入 Lazy Loading 機制
    useEffect(() => {
        if (!video.poster_path) {
            setPosterUrl(null);
            return;
        }

        let cancelled = false;

        const observer = new IntersectionObserver(
            (entries) => {
                if (entries[0].isIntersecting) {
                    // 當卡片進入畫面，才去讀取圖片
                    readImage(video.poster_path!)
                        .then((dataUrl) => {
                            if (!cancelled) setPosterUrl(dataUrl);
                        })
                        .catch(() => {
                            if (!cancelled) setPosterUrl(null);
                        });
                    // 讀取過後就可以解除觀察
                    if (cardRef.current) {
                        observer.unobserve(cardRef.current);
                    }
                }
            },
            { rootMargin: "200px" } // 提早 200px 預先載入
        );

        if (cardRef.current) {
            observer.observe(cardRef.current);
        }

        return () => {
            cancelled = true;
            if (cardRef.current) {
                observer.unobserve(cardRef.current);
            }
        };
    }, [video.poster_path, updateTrigger]);

    // 播放影片
    const handlePlay = async () => {
        try {
            await openVideo(video.video_path);
        } catch (e) {
            onToast(`播放失敗: ${e}`);
        }
    };

    // 開啟資料夾
    const handleOpenFolder = async () => {
        try {
            await openFolder(video.folder_path);
        } catch (e) {
            onToast(`開啟資料夾失敗: ${e}`);
        }
    };

    // 切換最愛
    const handleToggleFav = async (e: React.MouseEvent) => {
        e.stopPropagation();
        try {
            const newState = await toggleFavorite(video.id);
            onFavoriteToggled(video.id, newState);
            onToast(newState ? "★ 已加入我的最愛" : "☆ 已取消收藏");
        } catch (err) {
            onToast(`操作失敗: ${err}`);
        }
    };

    // 重新整理單一影片
    const handleRescan = async (e: React.MouseEvent) => {
        e.stopPropagation();
        setRescanning(true);
        try {
            const updated = await rescanSingleVideo(video.folder_path);
            onVideoUpdated(updated);

            // 強制觸發重新讀取海報圖
            if (updated.poster_path) {
                setUpdateTrigger((prev) => prev + 1);
            } else {
                setPosterUrl(null);
            }
            onToast(`✅ ${video.id} 已重新索引`);
        } catch (err: unknown) {
            const msg = String(err);
            if (msg.includes("已從資料庫移除") && onVideoRemoved) {
                onVideoRemoved(video.id);
            }
            onToast(`重新索引失敗: ${msg}`);
        } finally {
            setRescanning(false);
        }
    };

    // 雙擊播放
    const handleDoubleClick = () => {
        handlePlay();
    };

    // 顯示的副標題
    const subtitle = [
        video.release_date?.slice(0, 4),
        //video.actors.length > 0 ? video.actors.slice(0, 2).join(", ") : null,
        video.id,
    ]
        .filter(Boolean)
        .join(" • ");

    return (
        <div
            ref={cardRef}
            className={`movie-card relative flex flex-col cursor-pointer ${showRating ? "" : "group"}`}
            onDoubleClick={handleDoubleClick}
        >
            <div className="relative aspect-[2/3] rounded-xl overflow-hidden shadow-xl border border-zinc-800 bg-zinc-900">
                {/* 海報圖 */}
                {posterUrl ? (
                    <img
                        src={posterUrl}
                        alt={video.title}
                        className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-500"
                        loading="lazy"
                    />
                ) : (
                    <div className="poster-placeholder w-full h-full flex flex-col items-center justify-center gap-2">
                        <span className="text-3xl text-zinc-700">🎬</span>
                        <span className="text-xs text-zinc-600 font-medium">
                            {video.id}
                        </span>
                    </div>
                )}

                {/* 左上角：最愛星號 */}
                <button
                    onClick={handleToggleFav}
                    className="absolute top-2 left-2 z-20 w-8 h-8 flex items-center justify-center rounded-lg bg-black/40 backdrop-blur-md border border-white/10 hover:bg-black/60 transition-all"
                >
                    <Star
                        size={16}
                        className={`transition-all ${video.is_favorite
                            ? "fill-amber-400 text-amber-400 scale-110"
                            : "text-white/40"
                            }`}
                    />
                </button>

                {/* 右上角：等級標籤 */}
                {video.level && (
                    <div className="absolute top-2 right-2 px-2 py-0.5 bg-black/50 backdrop-blur-md rounded border border-white/5 text-[10px] font-bold text-zinc-300">
                        {video.level}
                    </div>
                )}

                {/* 懸停資訊層 */}
                <div className="absolute inset-0 bg-gradient-to-t from-black/95 via-black/40 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300 flex flex-col justify-end p-4">
                    {/* 右上角：重新整理按鈕 */}
                    <button
                        onClick={handleRescan}
                        disabled={rescanning}
                        className="absolute top-2 right-2 z-20 w-7 h-7 flex items-center justify-center rounded-lg bg-black/40 backdrop-blur-md border border-white/10 hover:bg-black/60 transition-all disabled:opacity-50"
                        title="重新整理索引及圖片"
                    >
                        <RefreshCw
                            size={13}
                            className={`text-zinc-300 ${rescanning ? "animate-spin" : ""}`}
                        />
                    </button>
                    {/* 評分（點擊可編輯） */}
                    <button
                        onClick={(e) => {
                            e.stopPropagation();
                            setShowRating(true);
                        }}
                        className="flex items-center gap-2 text-amber-400 text-sm font-bold mb-3 hover:text-amber-300 transition-colors cursor-pointer bg-transparent border-none p-0 text-left"
                        title="點擊編輯評分"
                    >
                        <span>
                            {video.rating > 0
                                ? video.rating.toFixed(1)
                                : "—"}
                        </span>
                        <span className="text-[10px] text-zinc-500 font-normal">
                            / 10
                        </span>
                    </button>

                    {/* 功能按鈕 */}
                    <div className="flex flex-col gap-2">
                        <button
                            onClick={(e) => {
                                e.stopPropagation();
                                handlePlay();
                            }}
                            className="btn-hover-effect w-full bg-white text-black py-2 rounded-lg text-xs font-bold hover:bg-indigo-50 flex items-center justify-center gap-1.5"
                        >
                            <Play size={12} fill="currentColor" /> 播放影片
                        </button>
                        <button
                            onClick={(e) => {
                                e.stopPropagation();
                                handleOpenFolder();
                            }}
                            className="btn-hover-effect w-full bg-zinc-800/80 border border-white/10 py-2 rounded-lg text-[10px] font-bold hover:bg-zinc-700 text-zinc-300 flex items-center justify-center gap-1"
                        >
                            <FolderOpen size={10} /> 開啟資料夾
                        </button>
                    </div>
                </div>
            </div>

            {/* 卡片下方文字 */}
            <div className="mt-3 px-1">
                <h3 className="text-sm font-bold truncate group-hover:text-indigo-400 transition-colors">
                    {video.actors.join(", ") || video.id}
                </h3>
                <div className="flex justify-between items-center mt-0.5">
                    <p className="text-[11px] text-zinc-500 truncate max-w-[80%]">
                        {subtitle || video.id}
                    </p>
                    {video.genres.length > 0 && (
                        <span className="text-[10px] text-zinc-600 truncate">
                            {video.genres[0]}
                        </span>
                    )}
                </div>
            </div>

            {/* 編輯影片資訊 */}
            {showRating && (
                <EditVideoModal
                    video={video}
                    onClose={() => setShowRating(false)}
                    onSaved={(updatedVideo: VideoEntry, newNfosPath: string) => {
                        onVideoUpdated({ ...updatedVideo, nfos_path: newNfosPath });
                        onRatingUpdated(updatedVideo.id, updatedVideo.rating, newNfosPath);
                        setShowRating(false);
                    }}
                    onToast={onToast}
                />
            )}
        </div>
    );
}
