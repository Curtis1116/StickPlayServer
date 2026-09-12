import { memo, useEffect, useRef, useState } from "react";
import {
    FolderOutput,
    MoreHorizontal,
    Pencil,
    Play,
    RefreshCw,
    Scissors,
    Star,
} from "lucide-react";
import { VideoEntry } from "../types";
import { openVideo, readImage, rescanSingleVideo, toggleFavorite } from "../api";
import EditVideoModal from "./EditVideoModal";
import ManualCropModal from "./ManualCropModal";
import MoveFolderModal from "./MoveFolderModal";

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
    const [showMoveModal, setShowMoveModal] = useState(false);
    const [showMenu, setShowMenu] = useState(false);
    const [rescanning, setRescanning] = useState(false);
    const [posterUrl, setPosterUrl] = useState<string | null>(null);
    const [updateTrigger, setUpdateTrigger] = useState(0);
    const cardRef = useRef<HTMLDivElement>(null);
    const menuRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        onModalStateChange(showRating || showCropModal || showMoveModal);
    }, [showRating, showCropModal, showMoveModal, onModalStateChange]);

    useEffect(() => {
        if (!showMenu) return;
        const close = (event: MouseEvent) => {
            if (menuRef.current && !menuRef.current.contains(event.target as Node)) setShowMenu(false);
        };
        document.addEventListener("mousedown", close);
        return () => document.removeEventListener("mousedown", close);
    }, [showMenu]);

    useEffect(() => {
        if (!video.poster_path) {
            setPosterUrl(null);
            return;
        }
        let cancelled = false;
        const observer = new IntersectionObserver((entries) => {
            if (!entries[0].isIntersecting) return;
            readImage(video.poster_path!, video.id, true, updateTrigger || undefined)
                .then((dataUrl) => { if (!cancelled) setPosterUrl(dataUrl); })
                .catch(() => { if (!cancelled) setPosterUrl(null); });
            if (cardRef.current) observer.unobserve(cardRef.current);
        }, { rootMargin: "200px" });
        if (cardRef.current) observer.observe(cardRef.current);
        return () => {
            cancelled = true;
            observer.disconnect();
        };
    }, [video.poster_path, video.id, updateTrigger]);

    const handlePlay = async () => {
        try {
            await openVideo(video.video_path, video.id);
        } catch (error) {
            onToast(`播放失敗: ${error}`);
        }
    };

    const handleToggleFavorite = async (event: React.MouseEvent) => {
        event.stopPropagation();
        try {
            const newState = await toggleFavorite(video.id);
            onFavoriteToggled(video.id, newState);
            onToast(newState ? "★ 已加入我的最愛" : "☆ 已取消收藏");
        } catch (error) {
            onToast(`操作失敗: ${error}`);
        }
    };

    const handleRescan = async () => {
        setShowMenu(false);
        setRescanning(true);
        try {
            const updated = await rescanSingleVideo(video.folder_path);
            onVideoUpdated(updated);
            if (updated.poster_path) setUpdateTrigger((previous) => previous + 1);
            else setPosterUrl(null);
            onToast(`✅ ${video.id} 已重新索引`);
        } catch (error) {
            const message = String(error);
            if (message.includes("已從資料庫移除") && onVideoRemoved) onVideoRemoved(video.id);
            onToast(`重新索引失敗: ${message}`);
        } finally {
            setRescanning(false);
        }
    };

    const actorText = video.actors.join(", ") || video.id;
    const score = video.criticrating >= 0 ? video.criticrating : "—";
    const year = video.release_date?.slice(0, 4);

    return (
        <div ref={cardRef} className="movie-card group relative min-w-0">
            <div
                role="button"
                tabIndex={0}
                aria-label={`播放 ${actorText} ${video.id}`}
                onClick={handlePlay}
                onKeyDown={(event) => {
                    if (event.target === event.currentTarget && (event.key === "Enter" || event.key === " ")) {
                        event.preventDefault();
                        void handlePlay();
                    }
                }}
                className="relative aspect-[2/3] cursor-pointer overflow-hidden rounded-lg border border-zinc-800 bg-zinc-900 shadow-lg outline-none focus-visible:ring-2 focus-visible:ring-indigo-400 lg:rounded-xl"
            >
                {posterUrl ? (
                    <img src={posterUrl} alt="" className="h-full w-full object-cover transition-transform duration-500 group-hover:scale-[1.03]" loading="lazy" />
                ) : (
                    <div className="poster-placeholder flex h-full w-full flex-col items-center justify-center gap-2">
                        <span className="text-2xl text-zinc-700 lg:text-3xl">🎬</span>
                        <span className="max-w-full truncate px-2 text-[10px] font-medium text-zinc-600 lg:text-xs">{video.id}</span>
                    </div>
                )}

                <button type="button" onClick={handleToggleFavorite} aria-label={video.is_favorite ? "取消收藏" : "加入最愛"} className="absolute left-1.5 top-1.5 z-10 flex h-8 w-8 items-center justify-center rounded-lg border border-white/10 bg-black/55 backdrop-blur-md lg:left-2 lg:top-2">
                    <Star size={15} className={video.is_favorite ? "fill-amber-400 text-amber-400" : "text-white/70"} />
                </button>

                {video.level && <span className="absolute right-2 top-2 hidden rounded bg-black/60 px-2 py-1 text-[10px] font-medium text-zinc-300 backdrop-blur-sm lg:block">{video.level}</span>}

                <span className="absolute bottom-2 right-2 flex h-9 w-9 items-center justify-center rounded-full border border-white/30 bg-black/65 text-white backdrop-blur-sm lg:hidden">
                    <Play size={15} fill="currentColor" className="ml-0.5" />
                </span>
            </div>

            <div className="mt-1.5 min-w-0 px-0.5 lg:mt-3 lg:px-0">
                <div className="flex min-w-0 items-center gap-1">
                    <p className="min-w-0 flex-1 truncate text-[12px] font-bold text-zinc-100 sm:text-sm lg:text-sm">{actorText}</p>
                    <span className="shrink-0 text-[11px] font-bold tabular-nums text-amber-400 sm:text-xs lg:text-xs">★ {score}</span>
                </div>

                <p className="mt-1 hidden truncate text-sm text-zinc-300 lg:block">{video.title || video.id}</p>

                <div className="relative mt-0.5 flex min-w-0 items-center gap-1" ref={menuRef}>
                    <p className="min-w-0 flex-1 truncate text-[11px] text-zinc-500 lg:text-xs">
                        <span className="lg:hidden">{video.id}</span>
                        <span className="hidden lg:inline">{[year, video.id].filter(Boolean).join(" · ")}</span>
                    </p>
                    <button type="button" onClick={() => setShowMenu((value) => !value)} aria-label="更多操作" aria-expanded={showMenu} className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-zinc-500 hover:bg-zinc-800 hover:text-white lg:hidden">
                        <MoreHorizontal size={17} />
                    </button>
                    {showMenu && (
                        <div className="absolute bottom-9 right-0 z-30 w-40 overflow-hidden rounded-xl border border-zinc-700 bg-zinc-900 p-1 shadow-2xl">
                            <button type="button" onClick={() => { setShowMenu(false); setShowRating(true); }} className="flex min-h-10 w-full items-center gap-2 rounded-lg px-3 text-left text-sm text-zinc-300 hover:bg-zinc-800"><Pencil size={15} />編輯資訊</button>
                            <button type="button" onClick={() => { setShowMenu(false); setShowCropModal(true); }} className="flex min-h-10 w-full items-center gap-2 rounded-lg px-3 text-left text-sm text-zinc-300 hover:bg-zinc-800"><Scissors size={15} />裁切海報</button>
                            <button type="button" onClick={() => { setShowMenu(false); setShowMoveModal(true); }} className="flex min-h-10 w-full items-center gap-2 rounded-lg px-3 text-left text-sm text-zinc-300 hover:bg-zinc-800"><FolderOutput size={15} />搬移資料夾</button>
                            <button type="button" onClick={handleRescan} disabled={rescanning} className="flex min-h-10 w-full items-center gap-2 rounded-lg px-3 text-left text-sm text-zinc-300 hover:bg-zinc-800 disabled:opacity-50"><RefreshCw size={15} className={rescanning ? "animate-spin" : ""} />重新索引</button>
                        </div>
                    )}
                </div>

                <div className="mt-2 hidden grid-cols-[1fr_auto] gap-2 lg:grid">
                    <button type="button" onClick={handlePlay} className="flex h-10 items-center justify-center gap-2 rounded-lg border border-zinc-700 bg-zinc-900 text-sm font-medium text-zinc-200 hover:border-zinc-600 hover:bg-zinc-800">
                        <Play size={14} fill="currentColor" />播放
                    </button>
                    <button type="button" onClick={() => setShowMenu((value) => !value)} aria-label="更多操作" className="flex h-10 w-11 items-center justify-center rounded-lg border border-zinc-700 text-zinc-400 hover:border-zinc-600 hover:text-white">
                        <MoreHorizontal size={17} />
                    </button>
                </div>
            </div>

            {showRating && <EditVideoModal video={video} onClose={() => setShowRating(false)} onSaved={(updated) => { onVideoUpdated(updated); setShowRating(false); }} onToast={onToast} />}
            {showCropModal && <ManualCropModal folderPath={video.folder_path} videoId={video.id} onClose={() => setShowCropModal(false)} onSaved={(posterPath) => { onVideoUpdated({ ...video, poster_path: posterPath }); setUpdateTrigger((previous) => previous + 1); }} onToast={onToast} />}
            {showMoveModal && <MoveFolderModal video={video} onClose={() => setShowMoveModal(false)} onSaved={onVideoUpdated} onRemoved={(id) => onVideoRemoved?.(id)} onToast={onToast} />}
        </div>
    );
});

export default VideoCard;
