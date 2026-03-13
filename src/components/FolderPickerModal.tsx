import React, { useState, useEffect } from "react";
import { createPortal } from "react-dom";
import { X, Folder, ChevronRight, ChevronLeft, Check, HardDrive } from "lucide-react";
import { listDirs } from "../api";

interface FolderPickerModalProps {
    onClose: () => void;
    onSelect: (path: string) => void;
}

export default function FolderPickerModal({ onClose, onSelect }: FolderPickerModalProps) {
    const [currentPath, setCurrentPath] = useState<string>("/media");
    const [dirs, setDirs] = useState<any[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const fetchDirs = async (path: string) => {
        setLoading(true);
        setError(null);
        try {
            const result = await listDirs(path);
            setDirs(result);
        } catch (err) {
            console.error("Failed to list dirs:", err);
            setError("無法讀取資料夾內容");
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchDirs(currentPath);
    }, [currentPath]);

    const handleNavigate = (path: string) => {
        setCurrentPath(path);
    };

    const handleBack = () => {
        if (currentPath === "/media" || currentPath === "/") return;
        
        // Simple parent path calculation
        const parts = currentPath.split("/").filter(Boolean);
        parts.pop();
        const parent = "/" + parts.join("/");
        setCurrentPath(parent === "" ? "/" : parent);
    };

    const handleConfirm = () => {
        onSelect(currentPath);
    };


    return createPortal(
        <div className="fixed inset-0 z-[110] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
            <div className="glass-panel w-full max-w-lg bg-zinc-900/90 border border-zinc-700/50 rounded-2xl shadow-2xl overflow-hidden flex flex-col h-[70vh]">
                {/* Header */}
                <div className="flex items-center justify-between p-4 border-b border-white/10 shrink-0">
                    <div className="flex items-center gap-2">
                        <Folder className="text-indigo-400" size={20} />
                        <h2 className="text-lg font-bold text-white">選擇資料夾</h2>
                    </div>
                    <button
                        onClick={onClose}
                        className="p-1 rounded-lg text-zinc-400 hover:text-white hover:bg-white/10 transition-colors"
                    >
                        <X size={20} />
                    </button>
                </div>

                {/* Path Breadcrumb */}
                <div className="px-4 py-2 bg-black/20 border-b border-white/5 flex items-center gap-2 overflow-x-auto no-scrollbar">
                    <button 
                        onClick={() => handleNavigate("/media")}
                        className="p-1.5 rounded hover:bg-white/5 text-zinc-400 hover:text-white transition-colors"
                    >
                        <HardDrive size={16} />
                    </button>
                    <ChevronRight size={14} className="text-zinc-600 shrink-0" />
                    <div className="flex items-center gap-1 text-sm font-medium text-zinc-300 whitespace-nowrap">
                        {currentPath === "/media" ? (
                            <span>媒體庫根目錄 (/media)</span>
                        ) : (
                            currentPath.replace("/media", "").split("/").filter(Boolean).map((part, i, arr) => (
                                <React.Fragment key={i}>
                                    <button 
                                        onClick={() => {
                                            const target = "/media/" + arr.slice(0, i + 1).join("/");
                                            handleNavigate(target);
                                        }}
                                        className="hover:text-indigo-400 transition-colors"
                                    >
                                        {part}
                                    </button>
                                    {i < arr.length - 1 && <ChevronRight size={12} className="text-zinc-600 mx-0.5" />}
                                </React.Fragment>
                            ))
                        )}
                    </div>
                </div>

                {/* Directory List */}
                <div className="flex-1 overflow-y-auto p-2 custom-scrollbar">
                    {currentPath !== "/media" && (
                        <button
                            onClick={handleBack}
                            className="w-full flex items-center gap-3 px-3 py-2.5 rounded-xl hover:bg-white/5 text-zinc-400 transition-colors group text-sm"
                        >
                            <ChevronLeft size={18} className="group-hover:-translate-x-0.5 transition-transform" />
                            <span>上一層</span>
                        </button>
                    )}

                    {loading ? (
                        <div className="flex flex-col items-center justify-center py-20 gap-3 text-zinc-500">
                            <div className="w-6 h-6 border-2 border-indigo-500 border-t-transparent rounded-full animate-spin"></div>
                            <span className="text-sm">讀取中...</span>
                        </div>
                    ) : error ? (
                        <div className="text-center py-20 text-rose-400 text-sm">
                            {error}
                        </div>
                    ) : dirs.length === 0 ? (
                        <div className="text-center py-20 text-zinc-600 text-sm italic">
                            此目錄下無子資料夾
                        </div>
                    ) : (
                        <div className="grid grid-cols-1 gap-1">
                            {dirs.map((entry) => (
                                <button
                                    key={entry.path}
                                    onClick={() => entry.is_dir && handleNavigate(entry.path)}
                                    className={`flex items-center justify-between px-3 py-2.5 rounded-xl transition-all group border border-transparent ${
                                        entry.is_dir 
                                        ? "hover:bg-indigo-500/10 text-zinc-300 hover:text-white hover:border-indigo-500/20" 
                                        : "opacity-40 cursor-default"
                                    }`}
                                >
                                    <div className="flex items-center gap-3 truncate">
                                        {entry.is_dir ? (
                                            <Folder size={18} className="text-indigo-400/70 group-hover:text-indigo-400 transition-colors shrink-0" />
                                        ) : (
                                            <div className="w-[18px] h-[18px] flex items-center justify-center">
                                                <div className="w-1.5 h-1.5 rounded-full bg-zinc-600" />
                                            </div>
                                        )}
                                        <span className="truncate text-sm">{entry.name}</span>
                                    </div>
                                    {entry.is_dir && <ChevronRight size={14} className="text-zinc-700 group-hover:text-indigo-400/50 transition-colors" />}
                                </button>
                            ))}
                        </div>
                    )}
                </div>

                {/* Footer */}
                <div className="p-4 border-t border-white/10 shrink-0 flex items-center justify-between bg-black/20">
                    <div className="text-[10px] text-zinc-500 bg-black/30 px-2 py-1 rounded truncate max-w-[200px]" title={currentPath}>
                        {currentPath}
                    </div>
                    <div className="flex gap-2">
                        <button
                            onClick={onClose}
                            className="px-4 py-2 rounded-xl text-xs font-bold text-zinc-400 hover:text-white hover:bg-white/10 transition-colors"
                        >
                            取消
                        </button>
                        <button
                            onClick={handleConfirm}
                            className="flex items-center gap-2 px-5 py-2 bg-indigo-500 hover:bg-indigo-600 text-white rounded-xl text-xs font-bold shadow-lg shadow-indigo-500/20 transition-all"
                        >
                            <Check size={14} />
                            選擇此資料夾
                        </button>
                    </div>
                </div>
            </div>
        </div>,
        document.body
    );
}
