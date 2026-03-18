import { useState } from "react";
import { createPortal } from "react-dom";
import { X, Save, Star, StarOff, Trash2, Plus, Minus } from "lucide-react";
import { VideoEntry } from "../types";
import { updateVideoInfo } from "../api";

interface EditVideoModalProps {
    video: VideoEntry;
    onClose: () => void;
    onSaved: (updatedVideo: VideoEntry) => void;
    onToast: (msg: string) => void;
}

export default function EditVideoModal({
    video,
    onClose,
    onSaved,
    onToast,
}: EditVideoModalProps) {
    const [rating, setRating] = useState(video.rating);
    const [criticRating, setCriticRating] = useState(video.criticrating || Math.round(video.rating * 10));
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

    const [saving, setSaving] = useState(false);

    // 評分同步邏輯

    const updateCriticRating = (newCriticRating: number) => {
        setCriticRating(newCriticRating);
        setRating(newCriticRating / 10);
    };

    const handleSave = async () => {
        setSaving(true);
        try {
            const actorsList = actorsStr
                .split(",")
                .map((s: string) => s.trim())
                .filter(Boolean);

            let newLevel = level;
            if (isUncensored && !newLevel.toLowerCase().endsWith("x")) {
                newLevel += "X";
            } else if (!isUncensored && newLevel.toLowerCase().endsWith("x")) {
                newLevel = newLevel.slice(0, -1);
            }

            const nfoPath = await updateVideoInfo(
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
                video.nfo_path,
                criticRating
            );

            const newGenres = video.genres.filter(g => g !== "無碼");
            if (isUncensored) newGenres.push("無碼");

            const updatedVideo: VideoEntry = {
                ...video,
                id,
                rating,
                criticrating: criticRating,
                actors: actorsList,
                release_date: releaseDate,
                date_added: dateAdded,
                is_favorite: isFavorite,
                genres: newGenres,
                level: newLevel,
                nfo_path: nfoPath,
                nfos_path: null, // 確保拋棄舊的 nfos
            };

            onSaved(updatedVideo);
            onToast(`影片資料已更新`);
        } catch (e) {
            onToast(`更新失敗: ${e}`);
        } finally {
            setSaving(false);
        }
    };

    return createPortal(
        <div className="fixed inset-0 z-[1000] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4" onPointerDown={(e) => e.target === e.currentTarget && onClose()}>
            <div className="glass-panel w-full max-w-2xl bg-zinc-900/90 border border-zinc-700/50 rounded-2xl shadow-2xl overflow-hidden flex flex-col h-[85vh]">
                <div className="flex items-center justify-between p-4 border-b border-white/10 shrink-0">
                    <h2 className="text-lg font-bold text-white">編輯影片資訊</h2>
                    <button onClick={onClose} className="p-1 rounded-lg text-zinc-400 hover:text-white hover:bg-white/10 transition-colors">
                        <X size={20} />
                    </button>
                </div>

                <div className="p-4 sm:p-6 overflow-y-auto flex-grow custom-scrollbar">
                    <div className="flex flex-col md:flex-row gap-4 sm:gap-6 mb-6">
                        {/* Simplified Critic Rating Section */}
                        <div className="flex-1">
                            <div className="bg-black/20 p-4 rounded-xl border border-white/5 flex items-center justify-between">
                                <div className="flex items-center gap-3">
                                    <div className="flex items-center gap-1 bg-zinc-800 border border-white/10 rounded-lg p-1">
                                        <button 
                                            onClick={() => updateCriticRating(Math.max(0, criticRating - 1))}
                                            className="w-8 h-8 flex items-center justify-center hover:bg-white/10 rounded text-zinc-400 transition-colors"
                                        >
                                            <Minus size={14} />
                                        </button>
                                        <input 
                                            type="number" 
                                            value={criticRating} 
                                            onChange={(e) => updateCriticRating(parseInt(e.target.value) || 0)}
                                            className="w-16 bg-transparent text-xl font-bold text-amber-400 text-center focus:outline-none appearance-none"
                                        />
                                        <button 
                                            onClick={() => updateCriticRating(Math.min(100, criticRating + 1))}
                                            className="w-8 h-8 flex items-center justify-center hover:bg-white/10 rounded text-zinc-400 transition-colors"
                                        >
                                            <Plus size={14} />
                                        </button>
                                    </div>
                                    <div className="flex gap-1">
                                        {[-5, +5].map(v => (
                                            <button
                                                key={v}
                                                onClick={() => updateCriticRating(Math.max(0, Math.min(100, criticRating + v)))}
                                                className="px-2 py-1 bg-zinc-800/50 hover:bg-zinc-700 text-[10px] font-bold text-zinc-500 rounded transition-colors"
                                            >
                                                {v > 0 ? `+${v}` : v}
                                            </button>
                                        ))}
                                    </div>
                                </div>

                                <button
                                    onClick={() => updateCriticRating(0)}
                                    className="p-2.5 text-zinc-500 hover:text-red-400 hover:bg-red-400/10 rounded-xl transition-all"
                                    title="清除分數"
                                >
                                    <Trash2 size={20} />
                                </button>
                            </div>
                        </div>

                        {/* Status Section */}
                        <div className="w-full md:w-32 shrink-0 flex flex-row md:flex-col gap-2 sm:gap-3">
                            <button
                                onClick={() => setIsFavorite(!isFavorite)}
                                className={`flex items-center justify-center gap-2 flex-1 md:w-full py-2.5 sm:py-3 rounded-xl border transition-all ${isFavorite ? "bg-indigo-500/20 border-indigo-500/50 text-indigo-400" : "bg-zinc-800/50 border-white/5 text-zinc-400"}`}
                            >
                                {isFavorite ? <Star size={16} fill="currentColor" /> : <StarOff size={16} />}
                                <span className="text-[10px] sm:text-xs font-bold">最愛</span>
                            </button>
                            <button
                                onClick={() => setIsUncensored(!isUncensored)}
                                className={`flex items-center justify-center gap-2 flex-1 md:w-full py-2.5 sm:py-3 rounded-xl border transition-all ${isUncensored ? "bg-rose-500/20 border-rose-500/50 text-rose-400" : "bg-zinc-800/50 border-white/5 text-zinc-400"}`}
                            >
                                <span className="text-[10px] sm:text-xs font-bold">無碼模式</span>
                            </button>
                        </div>
                    </div>

                    <div className="space-y-3 sm:space-y-4">
                        <div className="flex flex-row items-center gap-2 sm:gap-3">
                            <label className="w-16 sm:w-20 shrink-0 text-right text-[9px] sm:text-[10px] font-bold text-zinc-500 uppercase">影片 ID</label>
                            <input type="text" value={id} onChange={(e) => setId(e.target.value)} className="flex-1 bg-zinc-800/50 border border-white/10 rounded-xl px-3 sm:px-4 py-2 sm:py-2.5 text-xs sm:text-sm text-white focus:outline-none focus:border-indigo-500 transition-colors" />
                            <label className="shrink-0 text-[9px] sm:text-[10px] font-bold text-zinc-500 uppercase">分級</label>
                            <input type="text" value={level} onChange={(e) => setLevel(e.target.value)} className="w-16 sm:w-20 bg-zinc-800/50 border border-white/10 rounded-xl px-2 sm:px-4 py-2 sm:py-2.5 text-xs sm:text-sm text-white focus:outline-none focus:border-indigo-500 text-center" />
                        </div>

                        <div className="flex flex-row items-center gap-2 sm:gap-3">
                            <label className="w-16 sm:w-20 shrink-0 text-right text-[9px] sm:text-[10px] font-bold text-zinc-500 uppercase">演員清單</label>
                            <input type="text" value={actorsStr} onChange={(e) => setActorsStr(e.target.value)} placeholder="以逗號分隔" className="flex-1 bg-zinc-800/50 border border-white/10 rounded-xl px-3 sm:px-4 py-2 sm:py-2.5 text-xs sm:text-sm text-white focus:outline-none focus:border-indigo-500" />
                        </div>

                        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 sm:gap-4">
                            <div className="flex flex-row items-center gap-2 sm:gap-3">
                                <label className="w-16 sm:w-20 shrink-0 text-right text-[9px] sm:text-[10px] font-bold text-zinc-500 uppercase">發行日期</label>
                                <input type="text" value={releaseDate} onChange={(e) => setReleaseDate(e.target.value)} className="flex-1 bg-zinc-800/50 border border-white/10 rounded-xl px-3 sm:px-4 py-2 sm:py-2.5 text-xs sm:text-sm text-white focus:outline-none" />
                            </div>
                            <div className="flex flex-row items-center gap-2 sm:gap-3">
                                <label className="w-16 sm:w-20 shrink-0 text-right text-[9px] sm:text-[10px] font-bold text-zinc-500 uppercase">掃描時間</label>
                                <input type="text" value={dateAdded} onChange={(e) => setDateAdded(e.target.value)} className="flex-1 bg-zinc-800/50 border border-white/10 rounded-xl px-3 sm:px-4 py-2 sm:py-2.5 text-xs sm:text-sm text-white focus:outline-none" />
                            </div>
                        </div>
                    </div>
                </div>

                <div className="p-4 border-t border-white/10 shrink-0 flex justify-end gap-3 bg-zinc-950/50">
                    <button onClick={onClose} className="px-6 py-2 rounded-xl text-sm font-bold text-zinc-400 hover:text-white transition-colors">取消</button>
                    <button onClick={handleSave} disabled={saving} className="px-8 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-xl text-sm font-bold shadow-lg shadow-indigo-600/20 transition-all disabled:opacity-50 flex items-center gap-2">
                        <Save size={18} />
                        {saving ? "處理中..." : "儲存變更"}
                    </button>
                </div>
            </div>
        </div>,
        document.body
    );
}
