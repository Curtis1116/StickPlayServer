import { useEffect, useState } from "react";
import {
    ArrowLeft,
    ChevronDown,
    Download,
    FolderOpen,
    FolderPlus,
    PlayCircle,
    Plus,
    Trash2,
} from "lucide-react";
import { Library } from "../types";
import {
    deleteDatabase,
    getAvailablePlayers,
    getPlayerPreference,
    PlayerChoice,
    setPlayerPreference,
} from "../api";
import { detectDevicePlatform } from "../player";
import AccountSettings from "./AccountSettings";
import FolderPickerModal from "./FolderPickerModal";

const PLAYER_LABELS: Record<PlayerChoice, string> = {
    browser: "瀏覽器",
    potplayer: "PotPlayer",
    vlc: "VLC",
    infuse: "Infuse",
    justplayer: "Just Player",
};

interface SettingsPageProps {
    libraries: Library[];
    activeLibraryId: string;
    onBack: () => void;
    onLibrariesChanged: (libs: Library[], persisted?: boolean) => Promise<void>;
}

function LibraryNameInput({ initialName, onRename }: { initialName: string; onRename: (name: string) => void }) {
    const [localName, setLocalName] = useState(initialName);
    useEffect(() => setLocalName(initialName), [initialName]);

    const save = () => {
        const next = localName.trim();
        if (!next) setLocalName(initialName);
        else if (next !== initialName) onRename(next);
    };

    return (
        <input
            aria-label="媒體庫名稱"
            value={localName}
            onClick={(event) => event.stopPropagation()}
            onChange={(event) => setLocalName(event.target.value)}
            onBlur={save}
            onKeyDown={(event) => { if (event.key === "Enter") event.currentTarget.blur(); }}
            className="min-w-0 max-w-52 flex-1 rounded-md bg-transparent px-1 py-1 text-sm font-bold text-zinc-100 outline-none hover:bg-zinc-800 focus:bg-zinc-800"
        />
    );
}

export default function SettingsPage({ libraries, activeLibraryId, onBack, onLibrariesChanged }: SettingsPageProps) {
    const [error, setError] = useState("");
    const [deleting, setDeleting] = useState(false);
    const [showFolderPicker, setShowFolderPicker] = useState(false);
    const [activeLibIndex, setActiveLibIndex] = useState<number | null>(null);
    const [expandedLibraryId, setExpandedLibraryId] = useState(activeLibraryId || libraries[0]?.id || "");
    const [player, setPlayer] = useState<PlayerChoice>(getPlayerPreference());
    const availablePlayers = getAvailablePlayers();
    const devicePlatform = detectDevicePlatform();
    const playerTool = devicePlatform === "windows"
        ? {
            href: "/api/player-tools/windows",
            label: "下載 Windows 播放器工具",
            help: "解壓縮後執行 install.cmd，為目前帳號設定 PotPlayer 與 VLC。",
        }
        : devicePlatform === "macos"
            ? {
                href: "/api/player-tools/macos",
                label: "下載 macOS VLC 工具",
                help: "請先安裝 VLC；解壓縮後執行 Install StickPlay VLC.command。",
            }
            : null;

    useEffect(() => {
        if (activeLibraryId) setExpandedLibraryId(activeLibraryId);
    }, [activeLibraryId]);

    const saveLibraries = async (next: Library[]) => {
        try {
            await onLibrariesChanged(next);
            localStorage.setItem("stickplay_libraries", JSON.stringify(next));
        } catch (reason) {
            setError(`儲存失敗：${reason}`);
        }
    };

    const handleAddLibrary = async () => {
        const id = Date.now().toString();
        const next = [...libraries, { id, name: `媒體庫 ${libraries.length + 1}`, paths: [], db_name: `lib_${id}` }];
        setExpandedLibraryId(id);
        await saveLibraries(next);
    };

    const handleRemoveLibrary = async (index: number) => {
        const library = libraries[index];
        if (deleting || !window.confirm(`刪除「${library.name}」？\n影片檔案會保留；索引與收藏會從清單移除，伺服器會先備份資料庫。`)) return;
        setDeleting(true);
        setError("");
        try {
            const remaining = await deleteDatabase(library.db_name);
            await onLibrariesChanged(remaining, true);
        } catch (reason) {
            setError(`刪除失敗：${reason}`);
        } finally {
            setDeleting(false);
        }
    };

    const handleRenameLibrary = (index: number, name: string) => {
        const next = [...libraries];
        next[index] = { ...next[index], name };
        void saveLibraries(next);
    };

    const handleAddPath = (libraryIndex: number) => {
        setActiveLibIndex(libraryIndex);
        setShowFolderPicker(true);
    };

    const handleFolderSelect = (selected: string) => {
        if (activeLibIndex !== null && selected.trim()) {
            const next = [...libraries];
            const path = selected.trim();
            if (!next[activeLibIndex].paths.includes(path)) {
                next[activeLibIndex] = { ...next[activeLibIndex], paths: [...next[activeLibIndex].paths, path] };
                void saveLibraries(next);
            }
        }
        setShowFolderPicker(false);
        setActiveLibIndex(null);
    };

    const handleRemovePath = (libraryIndex: number, pathIndex: number) => {
        const next = [...libraries];
        next[libraryIndex] = {
            ...next[libraryIndex],
            paths: next[libraryIndex].paths.filter((_, index) => index !== pathIndex),
        };
        void saveLibraries(next);
    };

    const choosePlayer = (choice: PlayerChoice) => {
        setPlayer(choice);
        setPlayerPreference(choice);
    };

    return (
        <main className="page-transition-enter mx-auto w-full max-w-[1400px] px-4 pb-24 pt-4 sm:px-6 lg:px-8 lg:pb-10 lg:pt-7">
            <button type="button" onClick={onBack} className="mb-4 flex min-h-11 items-center gap-2 rounded-lg text-sm font-medium text-zinc-400 hover:text-zinc-100">
                <ArrowLeft size={18} />
                返回影片庫
            </button>
            <div className="mb-6">
                <h1 className="text-2xl font-bold tracking-tight text-zinc-100 lg:text-3xl">設定</h1>
                <p className="mt-1 text-sm text-zinc-500">管理媒體庫、播放偏好與登入裝置</p>
            </div>
            {error && <p role="alert" className="mb-4 rounded-lg border border-red-500/30 bg-red-500/10 p-3 text-sm text-red-300">{error}</p>}

            <div className="grid items-start gap-5 lg:grid-cols-[minmax(0,2fr)_minmax(280px,1fr)]">
                <section className="rounded-xl border border-zinc-800 bg-zinc-900/35 p-4 lg:p-5">
                    <div className="mb-4 flex items-start justify-between gap-3">
                        <div>
                            <div className="flex items-center gap-2">
                                <FolderOpen size={20} className="text-indigo-400" />
                                <h2 className="text-lg font-bold text-zinc-100">媒體庫</h2>
                            </div>
                            <p className="ml-7 mt-1 text-xs text-zinc-500">所有裝置共用</p>
                        </div>
                        <button type="button" onClick={handleAddLibrary} className="flex min-h-11 shrink-0 items-center gap-2 rounded-lg bg-indigo-500 px-3 text-sm font-bold text-white hover:bg-indigo-400">
                            <FolderPlus size={17} />
                            <span className="hidden sm:inline">新增媒體庫</span>
                            <span className="sm:hidden">新增</span>
                        </button>
                    </div>

                    {libraries.length === 0 ? (
                        <div className="rounded-lg border border-dashed border-zinc-700 py-12 text-center text-sm text-zinc-500">尚未建立媒體庫</div>
                    ) : (
                        <div className="space-y-2">
                            {libraries.map((library, libraryIndex) => {
                                const expanded = expandedLibraryId === library.id;
                                const active = activeLibraryId === library.id;
                                return (
                                    <div key={library.id} className="overflow-hidden rounded-xl border border-zinc-700 bg-zinc-950/35">
                                        <div className="flex min-h-14 items-center gap-2 px-3">
                                            <button type="button" onClick={() => setExpandedLibraryId(expanded ? "" : library.id)} aria-label={expanded ? "收合媒體庫" : "展開媒體庫"} aria-expanded={expanded} className="flex h-11 w-8 shrink-0 items-center justify-center text-zinc-400">
                                                <ChevronDown size={18} className={`transition-transform ${expanded ? "" : "-rotate-90"}`} />
                                            </button>
                                            <FolderOpen size={18} className="shrink-0 text-zinc-500" />
                                            <LibraryNameInput initialName={library.name} onRename={(name) => handleRenameLibrary(libraryIndex, name)} />
                                            {active && <span className="hidden shrink-0 rounded-md bg-indigo-500/15 px-2 py-1 text-xs text-indigo-300 sm:inline">目前使用</span>}
                                            <span className="ml-auto shrink-0 text-xs text-zinc-600">{library.paths.length} 個路徑</span>
                                            {libraries.length > 1 && (
                                                <button type="button" disabled={deleting} onClick={() => handleRemoveLibrary(libraryIndex)} aria-label={`刪除 ${library.name}`} className="flex h-11 w-10 shrink-0 items-center justify-center rounded-lg text-zinc-500 hover:bg-red-500/10 hover:text-red-400 disabled:opacity-50">
                                                    <Trash2 size={16} />
                                                </button>
                                            )}
                                        </div>
                                        {expanded && (
                                            <div className="border-t border-zinc-800 p-3">
                                                <div className="space-y-2">
                                                    {library.paths.map((path, pathIndex) => (
                                                        <div key={path} className="flex min-h-11 items-center gap-3 rounded-lg border border-zinc-800 bg-zinc-950/60 px-3">
                                                            <FolderOpen size={15} className="shrink-0 text-zinc-600" />
                                                            <span className="min-w-0 flex-1 truncate text-sm text-zinc-400">{path}</span>
                                                            <button type="button" onClick={() => handleRemovePath(libraryIndex, pathIndex)} aria-label={`移除路徑 ${path}`} className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg text-zinc-500 hover:bg-red-500/10 hover:text-red-400">
                                                                <Trash2 size={15} />
                                                            </button>
                                                        </div>
                                                    ))}
                                                    {library.paths.length === 0 && <p className="py-3 text-center text-sm text-zinc-600">尚未加入任何路徑</p>}
                                                </div>
                                                <button type="button" onClick={() => handleAddPath(libraryIndex)} className="mt-3 flex min-h-11 w-full items-center justify-center gap-2 rounded-lg border border-zinc-700 text-sm font-medium text-zinc-300 hover:border-indigo-500 hover:text-indigo-300">
                                                    <Plus size={17} />新增路徑
                                                </button>
                                            </div>
                                        )}
                                    </div>
                                );
                            })}
                        </div>
                    )}
                    <p className="mt-4 text-xs text-zinc-600">修改路徑後，請返回影片庫重新掃描。</p>
                </section>

                <section className="rounded-xl border border-zinc-800 bg-zinc-900/35 p-4 lg:p-5">
                    <div className="flex items-center gap-2">
                        <PlayCircle size={20} className="text-indigo-400" />
                        <h2 className="text-lg font-bold text-zinc-100">播放器</h2>
                    </div>
                    <p className="ml-7 mt-1 text-xs text-zinc-500">僅套用此瀏覽器</p>
                    <p className="mb-2 mt-5 text-sm font-medium text-zinc-300">預設播放器</p>
                    <div className="grid grid-cols-3 gap-2 lg:grid-cols-1">
                        {availablePlayers.map((choice) => (
                            <button type="button" key={choice} onClick={() => choosePlayer(choice)} className={`flex min-h-11 items-center justify-center gap-2 rounded-lg border px-2 text-sm font-medium lg:justify-start lg:px-3 ${player === choice ? "border-indigo-500 bg-indigo-500/15 text-indigo-300" : "border-zinc-700 text-zinc-400 hover:border-zinc-600"}`}>
                                <span className={`h-4 w-4 shrink-0 rounded-full border-2 ${player === choice ? "border-[5px] border-indigo-400" : "border-zinc-600"}`} />
                                <span className="truncate">{PLAYER_LABELS[choice]}</span>
                            </button>
                        ))}
                    </div>
                    <p className="mt-4 text-xs leading-5 text-zinc-600">外部播放器需先安裝並完成設定。</p>
                    {playerTool && (
                        <div className="mt-4 border-t border-zinc-800 pt-4">
                            <a
                                href={playerTool.href}
                                download
                                className="flex min-h-11 w-full items-center justify-center gap-2 rounded-lg border border-indigo-500/50 bg-indigo-500/10 px-3 text-sm font-bold text-indigo-300 hover:border-indigo-400 hover:bg-indigo-500/20"
                            >
                                <Download size={17} />
                                {playerTool.label}
                            </a>
                            <p className="mt-2 text-xs leading-5 text-zinc-500">{playerTool.help}</p>
                        </div>
                    )}
                </section>

                <div className="lg:col-span-2">
                    <AccountSettings />
                </div>
            </div>

            {showFolderPicker && <FolderPickerModal onClose={() => { setShowFolderPicker(false); setActiveLibIndex(null); }} onSelect={handleFolderSelect} />}
        </main>
    );
}
