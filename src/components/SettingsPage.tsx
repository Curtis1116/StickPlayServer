import { FolderPlus, Trash2, FolderOpen, ArrowLeft, Plus } from "lucide-react";

import { Library } from "../types";
import { deleteDatabase } from "../api";
import { useState, useEffect } from "react";
import FolderPickerModal from "./FolderPickerModal";

interface SettingsPageProps {
    libraries: Library[];
    activeLibraryId: string;
    onBack: () => void;
    onLibrariesChanged: (libs: Library[]) => void;
    onLibraryChange: (id: string) => Promise<void>;
}

const STORE_KEY = "libraries";

function LibraryNameInput({ initialName, onRename }: { initialName: string, onRename: (name: string) => void }) {
    const [localName, setLocalName] = useState(initialName);

    // Sync with external state if it changes unexpectedly
    useEffect(() => {
        setLocalName(initialName);
    }, [initialName]);

    const handleBlur = () => {
        if (localName.trim() !== "" && localName !== initialName) {
            onRename(localName);
        } else if (localName.trim() === "") {
            setLocalName(initialName);
        }
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === "Enter") {
            handleBlur();
        }
    };

    return (
        <input
            value={localName}
            onChange={(e) => setLocalName(e.target.value)}
            onBlur={handleBlur}
            onKeyDown={handleKeyDown}
            className="bg-transparent text-sm font-bold text-zinc-200 outline-none hover:bg-zinc-800 px-2 py-1 rounded transition-colors"
        />
    );
}

export default function SettingsPage({
    libraries,
    activeLibraryId,
    onBack,
    onLibrariesChanged,
    onLibraryChange,
}: SettingsPageProps) {
    const [showFolderPicker, setShowFolderPicker] = useState(false);
    const [activeLibIndex, setActiveLibIndex] = useState<number | null>(null);

    const saveLibraries = async (newLibs: Library[]) => {
        try {
            localStorage.setItem(`stickplay_${STORE_KEY}`, JSON.stringify(newLibs));
            onLibrariesChanged(newLibs);
        } catch (e) {
            console.error("儲存媒體庫失敗:", e);
        }
    };

    const handleAddLibrary = () => {
        const name = `媒體庫 ${libraries.length + 1}`;
        const newLib: Library = {
            id: Date.now().toString(),
            name,
            paths: [],
            db_name: `lib_${Date.now()}`
        };
        saveLibraries([...libraries, newLib]);
    };

    const handleRemoveLibrary = async (index: number) => {
        const libToDelete = libraries[index];
        const newLibs = libraries.filter((_, i) => i !== index);

        if (libToDelete.id === activeLibraryId && newLibs.length > 0) {
            await onLibraryChange(newLibs[0].id);
        }

        try {
            await deleteDatabase(libToDelete.db_name);
        } catch (e) {
            console.error("刪除資料庫檔案失敗:", e);
        }

        saveLibraries(newLibs);
    };

    const handleRenameLibrary = (index: number, newName: string) => {
        const newLibs = [...libraries];
        newLibs[index].name = newName;
        saveLibraries(newLibs);
    };

    const handleAddPath = async (libraryIndex: number) => {
        setActiveLibIndex(libraryIndex);
        setShowFolderPicker(true);
    };

    const handleFolderSelect = (selected: string) => {
        if (activeLibIndex !== null && selected.trim() !== "") {
            const newLibs = [...libraries];
            if (!newLibs[activeLibIndex].paths.includes(selected.trim())) {
                newLibs[activeLibIndex].paths.push(selected.trim());
                saveLibraries(newLibs);
            }
        }
        setShowFolderPicker(false);
        setActiveLibIndex(null);
    };

    const handleRemovePath = (libraryIndex: number, pathIndex: number) => {
        const newLibs = [...libraries];
        newLibs[libraryIndex].paths = newLibs[libraryIndex].paths.filter((_, i) => i !== pathIndex);
        saveLibraries(newLibs);
    };

    return (
        <div className="page-transition-enter max-w-2xl mx-auto py-12 px-6">
            {/* 返回按鈕 + 標題 */}
            <div className="flex items-center gap-4 mb-8">
                <button
                    onClick={onBack}
                    className="w-10 h-10 flex items-center justify-center rounded-xl bg-zinc-900 border border-zinc-800 hover:bg-zinc-800 transition-colors"
                >
                    <ArrowLeft size={18} />
                </button>
                <div>
                    <h1 className="text-2xl font-bold">設定</h1>
                    <p className="text-sm text-zinc-500 mt-0.5">
                        管理媒體庫與資料夾路徑
                    </p>
                </div>
            </div>

            {/* 媒體庫路徑 */}
            <div className="glass-panel rounded-2xl p-6">
                <div className="flex items-center justify-between mb-4">
                    <div className="flex items-center gap-2">
                        <FolderOpen size={18} className="text-indigo-400" />
                        <h2 className="text-base font-bold">媒體庫列表</h2>
                    </div>
                    <button
                        onClick={handleAddLibrary}
                        className="flex items-center gap-1.5 px-4 py-2 bg-indigo-500 hover:bg-indigo-600 text-white text-sm font-bold rounded-xl transition-colors"
                    >
                        <FolderPlus size={14} />
                        新增媒體庫
                    </button>
                </div>

                {libraries.length === 0 ? (
                    <div className="text-center py-12 text-zinc-600">
                        <FolderOpen
                            size={40}
                            className="mx-auto mb-3 text-zinc-700"
                        />
                        <p className="text-sm">尚未設定任何媒體庫</p>
                        <p className="text-xs text-zinc-700 mt-1">
                            點擊「新增媒體庫」開始新增
                        </p>
                    </div>
                ) : (
                    <div className="flex flex-col gap-4">
                        {libraries.map((lib, index) => (
                            <div
                                key={lib.id}
                                className="flex flex-col bg-zinc-900/80 border border-zinc-800 rounded-xl px-5 py-4 group"
                            >
                                <div className="flex items-center justify-between border-b border-zinc-800/50 pb-3 mb-3">
                                    <div className="flex items-center gap-2">
                                        <LibraryNameInput
                                            initialName={lib.name}
                                            onRename={(newName) => handleRenameLibrary(index, newName)}
                                        />
                                    </div>
                                    <div className="flex items-center gap-2">
                                        <button
                                            onClick={() => handleAddPath(index)}
                                            className="flex items-center gap-1.5 px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 text-xs font-bold rounded-lg transition-colors"
                                        >
                                            <Plus size={14} />
                                            新增路徑
                                        </button>
                                        {libraries.length > 1 && (
                                            <button
                                                onClick={() => handleRemoveLibrary(index)}
                                                className="text-zinc-600 hover:text-red-400 transition-colors flex-shrink-0 ml-2 p-1.5 hover:bg-red-400/10 rounded-lg"
                                                title="刪除此媒體庫"
                                            >
                                                <Trash2 size={16} />
                                            </button>
                                        )}
                                    </div>
                                </div>

                                {lib.paths.length === 0 ? (
                                    <div className="text-center py-6 text-zinc-600 bg-zinc-950/30 rounded-lg border border-zinc-800/50 border-dashed">
                                        <p className="text-sm">尚未加入任何路徑</p>
                                    </div>
                                ) : (
                                    <div className="flex flex-col gap-2">
                                        {lib.paths.map((path, pIndex) => (
                                            <div
                                                key={pIndex}
                                                className="flex items-center justify-between bg-zinc-950/50 border border-zinc-800/50 rounded-lg px-3 py-2 group/path hover:border-zinc-700 transition-colors"
                                            >
                                                <div className="flex items-center gap-3 min-w-0">
                                                    <FolderOpen
                                                        size={14}
                                                        className="text-zinc-500 flex-shrink-0"
                                                    />
                                                    <span className="text-sm text-zinc-400 truncate">
                                                        {path}
                                                    </span>
                                                </div>
                                                <button
                                                    onClick={() => handleRemovePath(index, pIndex)}
                                                    className="text-zinc-600 hover:text-red-400 transition-colors ml-3 flex-shrink-0 opacity-0 group-hover/path:opacity-100 p-1"
                                                >
                                                    <Trash2 size={14} />
                                                </button>
                                            </div>
                                        ))}
                                    </div>
                                )}
                            </div>
                        ))}
                    </div>
                )}
            </div>

            {/* 提示 */}
            <p className="text-xs text-zinc-600 mt-4 px-2">
                修改媒體庫路徑後，切換回主畫面並點擊右上角的重新掃描按鈕即可索引新影片。
            </p>

            {showFolderPicker && (
                <FolderPickerModal
                    onClose={() => setShowFolderPicker(false)}
                    onSelect={handleFolderSelect}
                />
            )}
        </div>
    );
}
