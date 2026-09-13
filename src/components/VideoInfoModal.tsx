import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { ChevronLeft, ChevronRight, ImageIcon, Pencil, Play, Scissors, Star, X } from "lucide-react";
import { getFolderImages, readImage } from "../api";
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
    const [images, setImages] = useState<string[]>([]);
    const [selectedIndex, setSelectedIndex] = useState(0);
    const [previewUrl, setPreviewUrl] = useState("");
    const [loadingImages, setLoadingImages] = useState(true);
    const regularGenres = video.genres.filter((genre) => genre !== "無碼");
    const isUncensored = video.genres.includes("無碼") || video.level.toLowerCase().endsWith("x");

    useEffect(() => {
        let cancelled = false;
        setLoadingImages(true);
        setImages([]);
        setSelectedIndex(0);
        getFolderImages(video.folder_path)
            .then((list) => { if (!cancelled) setImages(list); })
            .catch(() => { if (!cancelled) setImages([]); })
            .finally(() => { if (!cancelled) setLoadingImages(false); });
        return () => { cancelled = true; };
    }, [video.folder_path]);

    useEffect(() => {
        let cancelled = false;
        const path = images[selectedIndex];
        if (!path) {
            setPreviewUrl("");
            return;
        }
        readImage(path, undefined, false)
            .then((url) => { if (!cancelled) setPreviewUrl(url); })
            .catch(() => { if (!cancelled) setPreviewUrl(""); });
        return () => { cancelled = true; };
    }, [images, selectedIndex]);

    const selectedPath = images[selectedIndex] || "";
    const selectedName = selectedPath.split(/[\\/]/).pop() || selectedPath;
    const selectPrevious = () => setSelectedIndex((index) => (index - 1 + images.length) % images.length);
    const selectNext = () => setSelectedIndex((index) => (index + 1) % images.length);

    return createPortal(
        <div ref={dialogRef} role="dialog" aria-modal="true" aria-label="影片資訊" className="fixed inset-0 z-[1000] flex items-center justify-center bg-black/70 lg:p-4" onPointerDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
            <section className="flex h-[100dvh] w-full flex-col overflow-hidden border border-zinc-700 bg-[#15161b] shadow-2xl lg:h-auto lg:max-h-[90dvh] lg:max-w-3xl lg:rounded-2xl">
                <header className="flex shrink-0 items-center gap-2 border-b border-zinc-800 px-4 py-2">
                    <h2 className="flex-1 text-lg font-bold">影片資訊 <span className="ml-1 text-xs font-normal text-zinc-500">唯讀</span></h2>
                    <button type="button" onClick={onEdit} className="flex min-h-11 items-center gap-2 rounded-lg px-3 text-sm text-indigo-300 hover:bg-zinc-800"><Pencil size={17} />編輯</button>
                    <button type="button" onClick={onClose} aria-label="關閉影片資訊" className="flex h-11 w-11 items-center justify-center rounded-lg text-zinc-400 hover:bg-zinc-800"><X size={20} /></button>
                </header>

                <div className="min-h-0 flex-1 overflow-y-auto p-4 lg:p-6">
                    <div className="flex items-start gap-4 lg:gap-6">
                        <div className="w-28 shrink-0 lg:w-36">
                            {posterUrl ? <img src={posterUrl} alt={`${video.title || video.id} 封面`} className="aspect-[2/3] w-full rounded-lg object-cover" /> : <div className="poster-placeholder aspect-[2/3] w-full rounded-lg"><span className="text-2xl text-zinc-700">🎬</span></div>}
                            <button type="button" onClick={onCrop} className="flex min-h-11 items-center gap-2 text-sm text-indigo-300 hover:text-indigo-200"><Scissors size={17} />裁切封面</button>
                        </div>

                        <div className="min-w-0 flex-1 pt-0.5">
                            <p className="text-xs font-bold tracking-[0.16em] text-indigo-400">{video.id}</p>
                            <h3 className="mt-1 break-words text-lg font-bold leading-snug lg:text-2xl">{video.title || video.id}</h3>
                            <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-2 text-sm">
                                <span className="flex items-center gap-1.5 font-bold text-amber-400"><Star size={18} fill="currentColor" />{video.rating} / 10</span>
                                <span className="border-l border-zinc-700 pl-3 text-zinc-400">{video.is_favorite ? "★ 已收藏" : "未收藏"}</span>
                                {isUncensored && <span className="rounded-md border border-rose-500/30 bg-rose-500/10 px-2 py-1 text-xs text-rose-300">無碼模式</span>}
                            </div>
                            {regularGenres.length > 0 && <div className="mt-3 flex flex-wrap gap-1.5">{regularGenres.map((genre) => <span key={genre} className="rounded-full border border-zinc-700 bg-zinc-800/70 px-2.5 py-1 text-xs text-zinc-300">{genre}</span>)}</div>}
                            <dl className="mt-4 space-y-2 text-sm">
                                <div className="flex gap-3"><dt className="w-12 shrink-0 text-zinc-500">演員</dt><dd className="min-w-0 break-words text-zinc-200">{video.actors.join("、") || "—"}</dd></div>
                                <div className="flex gap-3"><dt className="w-12 shrink-0 text-zinc-500">分級</dt><dd className="text-zinc-200">{video.level || "未分級"}</dd></div>
                            </dl>
                        </div>
                    </div>

                    <section className="mt-5 lg:mt-6" aria-label="資料夾預覽">
                        <div className="mb-2 flex items-center gap-3">
                            <h4 className="shrink-0 text-xs font-bold tracking-wider text-zinc-300">資料夾預覽</h4>
                            {selectedName && <span className="min-w-0 truncate font-mono text-[10px] text-zinc-600">{selectedName}</span>}
                            <span className="h-px flex-1 bg-zinc-800" />
                        </div>
                        <div className="relative flex aspect-video w-full items-center justify-center overflow-hidden rounded-xl border border-zinc-700 bg-black">
                            {previewUrl ? <img src={previewUrl} alt={selectedName || "資料夾圖片"} className="h-full w-full object-contain" /> : <div className="flex flex-col items-center gap-2 text-sm text-zinc-600"><ImageIcon size={30} />{loadingImages ? "正在讀取圖片…" : "資料夾內沒有可預覽的圖片"}</div>}
                            {images.length > 0 && <span className="absolute right-3 top-3 rounded-full border border-white/10 bg-black/70 px-2.5 py-1 font-mono text-[10px] text-white">{selectedIndex + 1} / {images.length}</span>}
                            {images.length > 1 && <><button type="button" onClick={selectPrevious} aria-label="上一張圖片" className="absolute left-3 top-1/2 flex h-11 w-11 -translate-y-1/2 items-center justify-center rounded-full border border-white/15 bg-black/70 text-white hover:bg-black/90"><ChevronLeft size={22} /></button><button type="button" onClick={selectNext} aria-label="下一張圖片" className="absolute right-3 top-1/2 flex h-11 w-11 -translate-y-1/2 items-center justify-center rounded-full border border-white/15 bg-black/70 text-white hover:bg-black/90"><ChevronRight size={22} /></button></>}
                        </div>
                    </section>

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
