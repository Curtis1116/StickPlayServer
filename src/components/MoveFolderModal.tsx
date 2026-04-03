import { useState, useEffect } from "react";
import { createPortal } from "react-dom";
import { X, Folder, ChevronRight, FolderOutput, Loader2 } from "lucide-react";
import { listDirs, moveVideoFolder } from "../api";
import { VideoEntry } from "../types";

interface MoveFolderModalProps {
    video: VideoEntry;
    onClose: () => void;
    onSaved: (updatedVideo: VideoEntry) => void;
    onRemoved: (id: string) => void;
    onToast: (msg: string) => void;
}

export default function MoveFolderModal({
    video,
    onClose,
    onSaved,
    onRemoved,
    onToast,
}: MoveFolderModalProps) {
    const [currentPath, setCurrentPath] = useState<string>("/media");
    const [directories, setDirectories] = useState<any[]>([]);
    const [loading, setLoading] = useState(false);
    const [moving, setMoving] = useState(false);

    useEffect(() => {
        loadDirs(currentPath);
    }, [currentPath]);

    const loadDirs = async (path: string) => {
        setLoading(true);
        try {
            const dirs = await listDirs(path);
            setDirectories(dirs.filter(d => d.is_dir));
        } catch (err) {
            onToast(`載入資料夾失敗: ${err}`);
        } finally {
            setLoading(false);
        }
    };

    const handleBack = () => {
        if (currentPath === "/media") return;
        const parts = currentPath.split("/");
        parts.pop();
        let parent = parts.join("/");
        if (!parent || parent === "") parent = "/media";
        setCurrentPath(parent);
    };

    const handleConfirm = async () => {
        setMoving(true);
        try {
            const updated = await moveVideoFolder(video.id, video.folder_path, currentPath);
            onToast("✅ 資料夾搬移成功");
            onSaved(updated);
            onClose();
        } catch (err: unknown) {
            const msg = String(err);
            if (msg.includes("不在目前媒體庫的監控範圍內")) {
                onToast("✅ 搬移成功 (影片超出目前媒體庫監控，將於清單隱藏)");
                onRemoved(video.id);
            } else {
                onToast(`❌ 搬移失敗: ${msg}`);
            }
            onClose();
        } finally {
            setMoving(false);
        }
    };

    return createPortal(
        <div 
            className="fixed inset-0 z-[1000] flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-200"
            onClick={onClose}
        >
            <div 
                className="bg-zinc-900 border border-white/10 rounded-2xl w-full max-w-md overflow-hidden shadow-2xl flex flex-col animate-in zoom-in-95 duration-200"
                onClick={(e) => e.stopPropagation()}
            >
                <div className="flex items-center justify-between p-4 border-b border-white/5 bg-white/5">
                    <h2 className="text-sm font-bold flex items-center gap-2">
                        <FolderOutput size={16} className="text-zinc-400" />
                        搬移影片資料夾
                    </h2>
                    <button 
                        onClick={onClose}
                        className="p-1 rounded-lg hover:bg-white/10 text-zinc-400 transition-colors"
                        disabled={moving}
                    >
                        <X size={16} />
                    </button>
                </div>

                <div className="p-4 flex-grow flex flex-col gap-4 max-h-[65vh]">
                    <div className="text-xs text-zinc-400 flex flex-col gap-1">
                        <div className="truncate"><span className="font-bold text-zinc-300">目前路徑：</span>{video.folder_path}</div>
                        <div>請選擇要搬移至的目標資料夾：</div>
                    </div>

                    <div className="flex-grow flex flex-col bg-black/40 border border-white/5 rounded-xl overflow-hidden min-h-[320px]">
                        <div className="p-2 border-b border-white/5 bg-white/5 flex items-center gap-2 text-xs text-zinc-300">
                            <span className="truncate flex-1 font-mono text-[10px]">{currentPath}</span>
                            {currentPath !== "/media" && (
                                <button 
                                    onClick={handleBack}
                                    className="px-2 py-1 text-[10px] bg-white/10 rounded hover:bg-white/20 transition-colors font-bold shrink-0"
                                >
                                    回上層
                                </button>
                            )}
                        </div>

                        <div className="flex-grow overflow-y-auto p-2 flex flex-col gap-1 custom-scrollbar">
                            {loading ? (
                                <div className="flex-grow flex items-center justify-center">
                                    <Loader2 className="animate-spin text-zinc-500" size={24} />
                                </div>
                            ) : directories.length === 0 ? (
                                <div className="flex-grow flex items-center justify-center text-xs text-zinc-600 font-bold p-4 text-center">
                                    此資料夾為空
                                </div>
                            ) : (
                                directories.map((dir) => (
                                    <button
                                        key={dir.path}
                                        onClick={() => setCurrentPath(dir.path)}
                                        className="flex items-center justify-between p-2 rounded-lg hover:bg-white/5 transition-colors text-left group"
                                    >
                                        <div className="flex items-center gap-3 truncate">
                                            <Folder size={14} fill="currentColor" className="text-zinc-500 group-hover:text-indigo-400 transition-colors" />
                                            <span className="text-xs font-mono truncate">{dir.name}</span>
                                        </div>
                                        <ChevronRight size={14} className="text-zinc-700 group-hover:text-zinc-400 transition-colors" />
                                    </button>
                                ))
                            )}
                        </div>
                    </div>
                </div>

                <div className="p-4 border-t border-white/5 bg-black/20 flex justify-end gap-2">
                    <button
                        onClick={onClose}
                        className="px-4 py-2 text-xs font-bold text-zinc-400 hover:text-zinc-300 hover:bg-white/5 rounded-lg transition-colors"
                        disabled={moving}
                    >
                        取消
                    </button>
                    <button
                        onClick={handleConfirm}
                        disabled={moving}
                        className="px-6 py-2 text-xs font-bold bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg shadow disabled:opacity-50 transition-colors flex items-center gap-2"
                    >
                        {moving ? <Loader2 size={14} className="animate-spin" /> : <FolderOutput size={14} />}
                        確定搬移
                    </button>
                </div>
            </div>
        </div>,
        document.body
    );
}
