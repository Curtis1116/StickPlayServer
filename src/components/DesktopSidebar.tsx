import { Film, FolderOpen, Grid2X2, Settings, Star } from "lucide-react";
import { Library, VideoFilter } from "../types";

interface DesktopSidebarProps {
    page: "main" | "settings";
    libraries: Library[];
    activeLibraryId: string;
    genres: string[];
    filter: VideoFilter;
    totalCount: number;
    favoriteCount: number;
    isScanning: boolean;
    onLibraryChange: (id: string) => void;
    onFilterChange: (filter: VideoFilter) => void;
    onOpenMain: () => void;
    onOpenSettings: () => void;
}

export default function DesktopSidebar({
    page,
    libraries,
    activeLibraryId,
    genres,
    filter,
    totalCount,
    favoriteCount,
    isScanning,
    onLibraryChange,
    onFilterChange,
    onOpenMain,
    onOpenSettings,
}: DesktopSidebarProps) {
    const clearCatalogFilters = () => {
        onOpenMain();
        onFilterChange({
            ...filter,
            search: undefined,
            genres: undefined,
            levels: undefined,
            favorites_only: undefined,
        });
    };

    const showFavorites = () => {
        onOpenMain();
        onFilterChange({ ...filter, search: undefined, favorites_only: true, genres: undefined, levels: undefined });
    };

    const selectGenre = (genre?: string) => {
        onOpenMain();
        onFilterChange({
            ...filter,
            genres: genre ? [genre] : undefined,
            favorites_only: undefined,
        });
    };

    const navClass = (active: boolean) =>
        `flex min-h-11 w-full items-center gap-3 rounded-lg px-3 text-left text-sm transition-colors ${
            active
                ? "bg-indigo-500/20 text-indigo-200"
                : "text-zinc-400 hover:bg-zinc-800/70 hover:text-zinc-100"
        }`;

    return (
        <aside className="sticky top-0 hidden h-[100dvh] w-72 shrink-0 flex-col border-r border-zinc-800 bg-[#121318] px-4 py-5 lg:flex">
            <div className="mb-7 flex items-center gap-3 px-2">
                <span className="flex h-8 w-8 items-center justify-center rounded-lg bg-indigo-500 text-white">
                    <Film size={17} fill="currentColor" />
                </span>
                <span className="text-xl font-bold tracking-tight text-zinc-100">
                    StickPlay<span className="text-indigo-400">Server</span>
                </span>
            </div>

            <label className="relative mb-5 block">
                <span className="sr-only">切換媒體庫</span>
                <FolderOpen className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-zinc-400" size={17} />
                <select
                    value={activeLibraryId}
                    onChange={(event) => onLibraryChange(event.target.value)}
                    className="h-12 w-full appearance-none rounded-lg border border-zinc-700 bg-zinc-900 pl-10 pr-9 text-sm font-medium text-zinc-100 outline-none transition-colors hover:border-zinc-600 focus:border-indigo-500"
                >
                    {libraries.length === 0 && <option value="">尚未建立媒體庫</option>}
                    {libraries.map((library) => (
                        <option key={library.id} value={library.id}>{library.name}</option>
                    ))}
                </select>
                <span className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-zinc-500">⌄</span>
            </label>

            <nav className="space-y-1" aria-label="主要導覽">
                <button
                    type="button"
                    onClick={clearCatalogFilters}
                    className={navClass(page === "main" && !filter.favorites_only && !filter.genres?.length)}
                >
                    <Film size={18} />
                    <span className="flex-1">所有影片</span>
                    <span className="tabular-nums text-xs text-zinc-500">{totalCount}</span>
                </button>
                <button
                    type="button"
                    onClick={showFavorites}
                    className={navClass(page === "main" && Boolean(filter.favorites_only))}
                >
                    <Star size={18} />
                    <span className="flex-1">我的最愛</span>
                    <span className="tabular-nums text-xs text-zinc-500">{favoriteCount}</span>
                </button>
            </nav>

            <div className="my-5 h-px bg-zinc-800" />
            <p className="mb-2 px-3 text-xs font-medium text-zinc-600">分類</p>
            <nav className="space-y-1" aria-label="影片分類">
                <button
                    type="button"
                    onClick={() => selectGenre()}
                    className={navClass(false)}
                >
                    <Grid2X2 size={18} />
                    <span>全部類型</span>
                </button>
                {genres.slice(0, 6).map((genre) => (
                    <button
                        key={genre}
                        type="button"
                        onClick={() => selectGenre(genre)}
                        className={navClass(page === "main" && filter.genres?.length === 1 && filter.genres[0] === genre)}
                    >
                        <span className="ml-1 h-1.5 w-1.5 rounded-full bg-zinc-600" />
                        <span className="truncate">{genre}</span>
                    </button>
                ))}
            </nav>

            <div className="mt-auto space-y-3">
                <button type="button" onClick={onOpenSettings} className={navClass(page === "settings")}>
                    <Settings size={18} />
                    <span>設定</span>
                </button>
                <div className="flex items-center gap-2 border-t border-zinc-800 px-3 pt-4 text-xs text-zinc-500">
                    <span className={`h-2 w-2 rounded-full ${isScanning ? "animate-pulse bg-amber-400" : "bg-emerald-400"}`} />
                    {isScanning ? "正在掃描媒體庫" : "媒體庫已就緒"}
                </div>
            </div>
        </aside>
    );
}
