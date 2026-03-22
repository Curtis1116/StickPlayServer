import { useEffect, useRef, useState, memo } from "react";
import { Star, Play, RefreshCw, Scissors } from "lucide-react";
import { VideoEntry } from "../types";
import { openVideo, toggleFavorite, rescanSingleVideo, readImage } from "../api";
import EditVideoModal from "./EditVideoModal";
import ManualCropModal from "./ManualCropModal";

interface VideoCardProps {
    video: VideoEntry;
    onFavoriteToggled: (id: string, newState: boolean) => void;
    onVideoUpdated: (updated: VideoEntry) => void;
    onVideoRemoved?: (id: string) => void;
    onToast: (msg: string) => void;
    onModalStateChange: (open: boolean) => void;
}

const VideoCard = memo(({
    video,
    onFavoriteToggled,
    onVideoUpdated,
    onVideoRemoved,
    onToast,
    onModalStateChange,
}: VideoCardProps) => {
    const [showRating, setShowRating] = useState(false);
    const [showCropModal, setShowCropModal] = useState(false);
    const [rescanning, setRescanning] = useState(false);
    const [posterUrl, setPosterUrl] = useState<string | null>(null);
    const [updateTrigger, setUpdateTrigger] = useState(0);

    const cardRef = useRef<HTMLDivElement>(null);

    // 當 Modal 開啟或關閉時，通知 App 元件
    useEffect(() => {
        onModalStateChange(showRating || showCropModal);
    }, [showRating, showCropModal, onModalStateChange]);

    useEffect(() => {
        if (!video.poster_path) {
            setPosterUrl(null);
            return;
        }

        let cancelled = false;
        const observer = new IntersectionObserver(
            (entries) => {
                if (entries[0].isIntersecting) {
                    readImage(video.poster_path!, video.id)
                        .then((dataUrl) => {
                            if (!cancelled) setPosterUrl(dataUrl);
                        })
                        .catch(() => {
                            if (!cancelled) setPosterUrl(null);
                        });
                    if (cardRef.current) {
                        observer.unobserve(cardRef.current);
                    }
                }
            },
            { rootMargin: "200px" }
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

    const handlePlayInternal = async () => {
        try {
            await openVideo(video.video_path);
        } catch (e) {
            onToast(`播放失敗: ${e}`);
        }
    };


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

    const handleRescan = async (e: React.MouseEvent) => {
        e.stopPropagation();
        setRescanning(true);
        try {
            const updated = await rescanSingleVideo(video.folder_path);
            onVideoUpdated(updated);
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

    const subtitle = [
        video.release_date?.slice(0, 4),
        video.id,
    ].filter(Boolean).join(" • ");


    return (
        <div
            ref={cardRef}
            className="movie-card relative flex flex-col cursor-pointer group"
        >
            <div className="relative aspect-[2/3] rounded-xl overflow-hidden shadow-xl border border-zinc-800 bg-zinc-900">
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
                        <span className="text-xs text-zinc-600 font-medium">{video.id}</span>
                    </div>
                )}

                <button
                    onClick={handleToggleFav}
                    className="absolute top-2 left-2 z-20 w-8 h-8 flex items-center justify-center rounded-lg bg-black/40 backdrop-blur-md border border-white/10 hover:bg-black/60 transition-all"
                >
                    <Star
                        size={16}
                        className={`transition-all ${video.is_favorite ? "fill-amber-400 text-amber-400 scale-110" : "text-white/40"}`}
                    />
                </button>

                {video.level && (
                    <div className="absolute top-2 right-2 px-2 py-0.5 bg-black/50 backdrop-blur-md rounded border border-white/5 text-[10px] font-bold text-zinc-300">
                        {video.level}
                    </div>
                )}

                <div className="absolute inset-0 bg-gradient-to-t from-black/95 via-black/40 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300 flex flex-col justify-end p-4">
                    <button
                        onClick={handleRescan}
                        disabled={rescanning}
                        className="absolute top-2 right-2 z-20 w-7 h-7 flex items-center justify-center rounded-lg bg-black/40 backdrop-blur-md border border-white/10 hover:bg-black/60 transition-all disabled:opacity-50"
                        title="重新整理索引"
                    >
                        <RefreshCw size={13} className={`text-zinc-300 ${rescanning ? "animate-spin" : ""}`} />
                    </button>

                    <div className="flex flex-col gap-2 relative">
                        <button
                            onClick={(e) => { e.stopPropagation(); handlePlayInternal(); }}
                            className="w-full bg-white text-black py-2 rounded-lg text-xs font-bold hover:bg-indigo-50 flex items-center justify-center gap-1.5 transition-colors"
                        >
                            <Play size={12} fill="currentColor" /> 播放
                        </button>

                        <button
                            onClick={(e) => { e.stopPropagation(); setShowCropModal(true); }}
                            className="bg-zinc-800/80 border border-white/10 py-2 rounded-lg text-[10px] font-bold hover:bg-zinc-700 text-zinc-300 flex items-center justify-center gap-1.5"
                        >
                            <Scissors size={10} /> 海報裁切
                        </button>
                    </div>
                </div>
            </div>

            <div className="mt-3 px-1">
                <h3 className="text-sm font-bold truncate group-hover:text-indigo-400 transition-colors flex items-center gap-1.5">
                    {video.criticrating >= 0 && (
                         <span 
                            onClick={(e) => { e.stopPropagation(); setShowRating(true); }}
                            className="text-amber-400 font-black text-xs min-w-[1.4rem] text-center bg-amber-400/10 rounded px-1 py-0.5 cursor-pointer hover:bg-amber-400/20 active:scale-95 transition-all" 
                            title="點擊修改評分"
                         >
                             {video.criticrating}
                         </span>
                    )}
                    <span className="truncate flex-1 ml-0.5">{video.actors.join(", ") || video.id}</span>
                </h3>
                <div className="flex justify-between items-center mt-0.5">
                    <p className="text-[11px] text-zinc-500 truncate max-w-[80%]">{subtitle}</p>
                    {video.genres.length > 0 && (
                        <span className="text-[10px] text-zinc-600 truncate">{video.genres[0]}</span>
                    )}
                </div>
            </div>

            {showRating && (
                <EditVideoModal
                    video={video}
                    onClose={() => setShowRating(false)}
                    onSaved={(updatedVideo: VideoEntry) => {
                        onVideoUpdated(updatedVideo);
                        setShowRating(false);
                    }}
                    onToast={onToast}
                />
            )}

            {showCropModal && (
                <ManualCropModal
                    folderPath={video.folder_path}
                    videoId={video.id}
                    onClose={() => setShowCropModal(false)}
                    onSaved={(newPosterPath) => {
                        onVideoUpdated({ ...video, poster_path: newPosterPath });
                        setUpdateTrigger(prev => prev + 1);
                    }}
                    onToast={onToast}
                />
            )}
        </div>
    );
});

export default VideoCard;
