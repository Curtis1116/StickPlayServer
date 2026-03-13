import React, { useState, useEffect } from "react";
import { createPortal } from "react-dom";
import { X, Save, FolderOpen, Image as ImageIcon, Trash2 } from "lucide-react";
import { VideoEntry } from "../types";
import { updateVideoInfo, getFanartPath, readImage, openFolder } from "../api";

interface EditVideoModalProps {
    video: VideoEntry;
    onClose: () => void;
    onSaved: (updatedVideo: VideoEntry, newNfosPath: string) => void;
    onToast: (msg: string) => void;
}

export default function EditVideoModal({
    video,
    onClose,
    onSaved,
    onToast,
}: EditVideoModalProps) {
    const [rating, setRating] = useState(video.rating);
    const [id, setId] = useState(video.id);
    const [level, setLevel] = useState(video.level);
    const [actorsStr, setActorsStr] = useState(video.actors.join(", "));
    const [releaseDate, setReleaseDate] = useState(video.release_date);
    const [dateAdded, setDateAdded] = useState(video.date_added);
    const [isFavorite, setIsFavorite] = useState(video.is_favorite);
    const [isUncensored, setIsUncensored] = useState(
        video.genres.includes("無碼") ||
        video.level.toLowerCase().endsWith("x")
    );

    const [fanartDataUrl, setFanartDataUrl] = useState<string | null>(null);
    const [loadingImage, setLoadingImage] = useState(true);
    const [saving, setSaving] = useState(false);

    useEffect(() => {
        let isMounted = true;
        const fetchImage = async () => {
            try {
                const path = await getFanartPath(video.folder_path, video.video_path);
                const dataUrl = await readImage(path);
                if (isMounted) {
                    setFanartDataUrl(dataUrl);
                }
            } catch (err) {
                console.error("fetchImage Error:", err, "folder:", video.folder_path);
            } finally {
                if (isMounted) setLoadingImage(false);
            }
        };
        fetchImage();
        return () => {
            isMounted = false;
        };
    }, [video.folder_path, video.video_path]);

    const handleSave = async () => {
        setSaving(true);
        try {
            const actorsList = actorsStr
                .split(",")
                .map((s) => s.trim())
                .filter(Boolean);

            // Check if level should be updated (add/remove X suffix)
            let newLevel = level;
            if (isUncensored && !newLevel.toLowerCase().endsWith("x")) {
                newLevel += "X";
            } else if (!isUncensored && newLevel.toLowerCase().endsWith("x")) {
                newLevel = newLevel.slice(0, -1);
            }

            const newNfosPath = await updateVideoInfo(
                video.id,
                id,
                video.title,
                newLevel,
                rating,
                actorsList,
                releaseDate,
                dateAdded,
                isFavorite,
                isUncensored,
                video.video_path,
                video.folder_path,
                video.poster_path || null,
                video.nfo_path || null,
                video.nfos_path || null
            );

            const newGenres = video.genres.filter(g => g !== "無碼");
            if (isUncensored) newGenres.push("無碼");

            const updatedVideo: VideoEntry = {
                ...video,
                id,
                rating,
                actors: actorsList,
                release_date: releaseDate,
                date_added: dateAdded,
                is_favorite: isFavorite,
                genres: newGenres,
                level: newLevel,
                nfos_path: newNfosPath,
            };

            onSaved(updatedVideo, newNfosPath);
            onToast(`影片資料已更新`);
        } catch (e) {
            onToast(`更新失敗: ${e}`);
        } finally {
            setSaving(false);
        }
    };

    const handleOpenFolder = async (e: React.MouseEvent) => {
        e.preventDefault();
        try {
            await openFolder(video.folder_path);
        } catch (err) {
            onToast(`開啟資料夾失敗: ${err}`);
        }
    };

    return createPortal(
        <div
            className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
            onPointerDown={(e) => {
                if (e.target === e.currentTarget) onClose();
            }}
        >
            <div className="glass-panel w-full max-w-2xl bg-zinc-900/90 border border-zinc-700/50 rounded-2xl shadow-2xl overflow-hidden flex flex-col h-[85vh]">
                {/* Header */}
                <div className="flex items-center justify-between p-4 border-b border-white/10 shrink-0">
                    <h2 className="text-lg font-bold text-white">編輯影片資訊</h2>
                    <button
                        onClick={onClose}
                        className="p-1 rounded-lg text-zinc-400 hover:text-white hover:bg-white/10 transition-colors"
                    >
                        <X size={20} />
                    </button>
                </div>

                {/* Content */}
                <div className="p-6 overflow-y-auto flex-grow custom-scrollbar">

                    <div className="flex flex-col md:flex-row gap-6 mb-4">
                        {/* Rating Section (Left Size shrunk) */}
                        <div className="flex-1 flex flex-col items-center relative bg-black/20 p-4 rounded-xl border border-white/5 justify-center">
                            {rating > 0 && (
                                <button
                                    onClick={() => setRating(0)}
                                    className="absolute top-2 right-2 p-2 text-zinc-500 hover:text-rose-400 bg-zinc-800/80 hover:bg-zinc-700 rounded-lg transition-colors border border-white/5"
                                    title="刪除評分"
                                >
                                    <Trash2 size={16} />
                                </button>
                            )}
                            <div className="text-3xl font-black text-amber-400 mb-1">
                                {rating > 0 ? rating.toFixed(1) : "—"}
                            </div>
                            <input
                                type="range"
                                min="0"
                                max="100"
                                step="1"
                                value={rating * 10}
                                onChange={(e) => setRating(Number(e.target.value) / 10)}
                                className="w-full h-1.5 bg-zinc-800 rounded-lg appearance-none cursor-pointer mb-2"
                            />
                            <div className="flex gap-1 flex-wrap justify-center">
                                {[1, 2, 3, 4, 5, 6, 7, 8, 9, 10].map((v) => (
                                    <button
                                        key={v}
                                        onClick={() => setRating(v)}
                                        className={`w-7 h-7 rounded-md text-xs font-bold transition-all ${Math.floor(rating) === v
                                            ? "bg-indigo-500 text-white shadow-lg shadow-indigo-500/30"
                                            : "bg-zinc-800 text-zinc-400 hover:bg-zinc-700"
                                            }`}
                                    >
                                        {v}
                                    </button>
                                ))}
                            </div>
                        </div>

                        {/* Checkboxes Section (Right Side of Rating) */}
                        <div className="w-full md:w-32 shrink-0 flex flex-col justify-center gap-4 bg-black/20 p-4 rounded-xl border border-white/5">
                            <label className="flex items-center gap-3 cursor-pointer group">
                                <div className="relative flex items-center justify-center w-6 h-6 rounded border border-white/20 bg-zinc-800 group-hover:bg-zinc-700 transition-colors">
                                    <input
                                        type="checkbox"
                                        className="sr-only"
                                        checked={isFavorite}
                                        onChange={(e) => setIsFavorite(e.target.checked)}
                                    />
                                    {isFavorite && (
                                        <div className="w-3.5 h-3.5 bg-indigo-500 rounded-sm" />
                                    )}
                                </div>
                                <span className="text-sm font-bold text-zinc-300 group-hover:text-white transition-colors">
                                    我的最愛
                                </span>
                            </label>

                            <label className="flex items-center gap-3 cursor-pointer group">
                                <div className="relative flex items-center justify-center w-6 h-6 rounded border border-white/20 bg-zinc-800 group-hover:bg-zinc-700 transition-colors">
                                    <input
                                        type="checkbox"
                                        className="sr-only"
                                        checked={isUncensored}
                                        onChange={(e) => setIsUncensored(e.target.checked)}
                                    />
                                    {isUncensored && (
                                        <div className="w-3.5 h-3.5 bg-rose-500 rounded-sm" />
                                    )}
                                </div>
                                <span className="text-sm font-bold text-zinc-300 group-hover:text-white transition-colors">
                                    無碼
                                </span>
                            </label>
                        </div>
                    </div>

                    <div className="flex flex-col gap-3">
                        {/* Form Fields - Now Full Width */}
                        <div className="space-y-3">
                            {/* Folder Path (Readonly) */}
                            <div className="flex flex-row items-center gap-3">
                                <label className="w-20 shrink-0 text-right text-xs font-semibold text-zinc-400">
                                    資料夾路徑
                                </label>
                                <div className="flex flex-1 gap-2">
                                    <button
                                        onClick={handleOpenFolder}
                                        className="flex shrink-0 items-center justify-center w-10 h-10 bg-zinc-800 hover:bg-zinc-700 rounded-xl transition-colors text-zinc-300 border border-white/5"
                                        title="開啟資料夾"
                                    >
                                        <FolderOpen size={18} />
                                    </button>
                                    <input
                                        type="text"
                                        value={video.folder_path}
                                        readOnly
                                        className="w-full bg-black/40 border border-white/5 rounded-xl px-3 py-2 text-sm text-zinc-500 focus:outline-none"
                                    />
                                </div>
                            </div>

                            {/* ID and Level */}
                            <div className="flex flex-row gap-4">
                                <div className="flex flex-row items-center gap-3 basis-2/3">
                                    <label className="w-20 shrink-0 text-right text-xs font-semibold text-zinc-400">
                                        影片 ID
                                    </label>
                                    <input
                                        type="text"
                                        value={id}
                                        onChange={(e) => setId(e.target.value)}
                                        className="w-full flex-1 bg-zinc-800/50 border border-white/10 rounded-xl px-3 py-2 text-sm text-white focus:outline-none focus:border-indigo-500 transition-colors"
                                    />
                                </div>
                                <div className="flex flex-row items-center gap-3 basis-1/3">
                                    <label className="shrink-0 text-right text-xs font-semibold text-zinc-400">
                                        分級
                                    </label>
                                    <input
                                        type="text"
                                        value={level}
                                        onChange={(e) => setLevel(e.target.value)}
                                        className="w-full flex-1 bg-zinc-800/50 border border-white/10 rounded-xl px-3 py-2 text-sm text-white focus:outline-none focus:border-indigo-500 transition-colors"
                                    />
                                </div>
                            </div>

                            {/* Actors */}
                            <div className="flex flex-row items-center gap-3">
                                <label className="w-20 shrink-0 text-right text-xs font-semibold text-zinc-400">
                                    演員
                                </label>
                                <input
                                    type="text"
                                    value={actorsStr}
                                    onChange={(e) => setActorsStr(e.target.value)}
                                    placeholder="例如: 演員A, 演員B"
                                    className="w-full flex-1 bg-zinc-800/50 border border-white/10 rounded-xl px-3 py-2 text-sm text-white focus:outline-none focus:border-indigo-500 transition-colors"
                                />
                            </div>

                            {/* Dates */}
                            <div className="grid grid-cols-2 gap-4">
                                <div className="flex flex-row items-center gap-3">
                                    <label className="w-20 shrink-0 text-right text-xs font-semibold text-zinc-400">
                                        發行日期
                                    </label>
                                    <input
                                        type="text"
                                        value={releaseDate}
                                        onChange={(e) => setReleaseDate(e.target.value)}
                                        placeholder="YYYY-MM-DD"
                                        className="w-full flex-1 bg-zinc-800/50 border border-white/10 rounded-xl px-3 py-2 text-sm text-white focus:outline-none focus:border-indigo-500 transition-colors"
                                    />
                                </div>
                                <div className="flex flex-row items-center gap-3">
                                    <label className="w-20 shrink-0 text-right text-xs font-semibold text-zinc-400">
                                        加入時間
                                    </label>
                                    <input
                                        type="text"
                                        value={dateAdded}
                                        onChange={(e) => setDateAdded(e.target.value)}
                                        placeholder="YYYY-MM-DD HH:MM:SS"
                                        className="w-full flex-1 bg-zinc-800/50 border border-white/10 rounded-xl px-3 py-2 text-sm text-white focus:outline-none focus:border-indigo-500 transition-colors"
                                    />
                                </div>
                            </div>
                        </div>
                    </div>

                    {/* Image Preview - Moved below form fields */}
                    <div className="w-full flex flex-col mt-2 items-center">
                        <label className="block text-xs font-medium text-zinc-400 mb-1 w-full text-center">
                            影片縮圖
                        </label>
                        <div className="w-full max-w-xs md:max-w-sm aspect-video bg-black/40 rounded-xl border border-white/5 overflow-hidden flex items-center justify-center relative">
                            {loadingImage ? (
                                <div className="w-6 h-6 border-2 border-indigo-500 border-t-transparent rounded-full animate-spin"></div>
                            ) : fanartDataUrl ? (
                                <img
                                    src={fanartDataUrl}
                                    alt="Thumbnail Preview"
                                    className="w-full h-full object-cover"
                                />
                            ) : (
                                <div className="flex flex-col items-center text-zinc-600">
                                    <ImageIcon size={32} className="mb-2 opacity-50" />
                                    <span className="text-xs font-medium">無專屬縮圖</span>
                                </div>
                            )}
                        </div>
                    </div>
                </div>

                {/* Footer */}
                <div className="p-4 border-t border-white/10 shrink-0 flex justify-end gap-3 bg-black/20">
                    <button
                        onClick={onClose}
                        className="px-5 py-2 rounded-xl text-sm font-bold text-zinc-400 hover:text-white hover:bg-white/10 transition-colors"
                    >
                        取消
                    </button>
                    <button
                        onClick={handleSave}
                        disabled={saving}
                        className="px-6 py-2 bg-indigo-500 hover:bg-indigo-600 text-white rounded-xl text-sm font-bold shadow-lg shadow-indigo-500/20 transition-all disabled:opacity-50 flex items-center gap-2"
                    >
                        <Save size={16} />
                        {saving ? "儲存中..." : "儲存設定"}
                    </button>
                </div>
            </div>
        </div>,
        document.body
    );
}
