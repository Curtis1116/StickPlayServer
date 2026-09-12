import { createPortal } from "react-dom";
import { Pencil, Play, Scissors, Star, X } from "lucide-react";
import { VideoEntry } from "../types";
import { useDialogFocus } from "../useDialogFocus";

interface VideoInfoModalProps {
    video: VideoEntry;
    posterUrl: string | null;
    onClose: () => void;
    onPlay: () => void;
    onEdit: () => void;
    onCrop: () => void;
}

export default function VideoInfoModal({ video, posterUrl, onClose, onPlay, onEdit, onCrop }: VideoInfoModalProps) {
    const dialogRef = useDialogFocus(onClose);
    const fields = [
        ["影片 ID", video.id], ["分級", video.level || "未分級"],
        ["演員", video.actors.join("、") || "—"], ["類型", video.genres.join("、") || "—"],
        ["發行日期", video.release_date || "—"], ["加入日期", video.date_added || "—"],
        ["評分", video.criticrating >= 0 ? `${video.criticrating} / 100` : "—"],
        ["收藏", video.is_favorite ? "已收藏" : "未收藏"],
    ];
    return createPortal(
        <div ref={dialogRef} role="dialog" aria-modal="true" aria-label="影片資訊" className="fixed inset-0 z-[1000] flex items-center justify-center bg-black/70 lg:p-4" onPointerDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
            <section className="flex h-[100dvh] w-full flex-col overflow-hidden border border-zinc-700 bg-[#15161b] shadow-2xl lg:h-auto lg:max-h-[90dvh] lg:max-w-2xl lg:rounded-2xl">
                <header className="flex shrink-0 items-center gap-2 border-b border-zinc-800 px-4 py-2">
                    <h2 className="flex-1 text-lg font-bold">影片資訊 <span className="ml-1 text-xs font-normal text-zinc-500">唯讀</span></h2>
                    <button type="button" onClick={onEdit} className="flex min-h-11 items-center gap-2 rounded-lg px-3 text-sm text-indigo-300 hover:bg-zinc-800"><Pencil size={17} />編輯</button>
                    <button type="button" onClick={onClose} aria-label="關閉影片資訊" className="flex h-11 w-11 items-center justify-center rounded-lg text-zinc-400 hover:bg-zinc-800"><X size={20} /></button>
                </header>
                <div className="min-h-0 flex-1 overflow-y-auto p-4 lg:p-6">
                    <div className="mb-4 flex items-start gap-4">
                        <div className="w-28 shrink-0 lg:w-36">
                            {posterUrl && <img src={posterUrl} alt={`${video.title || video.id} 封面`} className="aspect-[2/3] w-full rounded-lg object-cover" />}
                            <button type="button" onClick={onCrop} className="flex min-h-11 items-center gap-2 text-sm text-indigo-300 hover:text-indigo-200"><Scissors size={17} />裁切封面</button>
                        </div>
                        <div className="min-w-0">
                            <h3 className="break-words text-lg font-bold lg:text-xl">{video.title || video.id}</h3>
                            <p className="mt-2 break-all text-sm text-zinc-400">{video.id}</p>
                            <p className="mt-3 flex items-center gap-2 text-amber-400"><Star size={20} fill="currentColor" />{video.rating} / 10</p>
                            <p className="mt-3 text-sm text-zinc-400">{video.is_favorite ? "已收藏" : "未收藏"}</p>
                        </div>
                    </div>
                    <dl className="space-y-3 border-t border-zinc-800 pt-4 text-sm">
                        {fields.map(([label, value]) => <div key={label} className="flex gap-4"><dt className="w-20 shrink-0 text-zinc-400">{label}</dt><dd className="min-w-0 break-words text-zinc-100">{value}</dd></div>)}
                    </dl>
                    <details className="mt-5 border-t border-zinc-800 pt-4 text-sm text-zinc-400">
                        <summary className="cursor-pointer">檔案資訊</summary>
                        <p className="mt-3 break-all">影片：{video.video_path}</p>
                        <p className="mt-2 break-all">資料夾：{video.folder_path}</p>
                    </details>
                </div>
                <footer className="flex shrink-0 justify-end gap-3 border-t border-zinc-800 bg-[#15161b] px-4 pb-[calc(1rem+env(safe-area-inset-bottom))] pt-3">
                    <button type="button" onClick={onClose} className="hidden rounded-lg border border-zinc-700 px-5 text-sm text-zinc-300 lg:block">關閉</button>
                    <button type="button" onClick={onPlay} className="flex min-h-12 w-full items-center justify-center gap-2 rounded-lg bg-indigo-600 px-6 font-bold text-white hover:bg-indigo-500 lg:w-auto"><Play size={18} fill="currentColor" />播放影片</button>
                </footer>
            </section>
        </div>, document.body,
    );
}
