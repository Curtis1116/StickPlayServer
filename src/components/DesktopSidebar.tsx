import { useState } from "react";
import { ChevronDown, ChevronsLeft, ChevronsRight, Film, FolderOpen, Grid2X2, Settings, Star } from "lucide-react";
import { Library, VideoFilter } from "../types";
import Header from "./Header";

interface DesktopSidebarProps {
    page: "main" | "settings";
    libraries: Library[];
    activeLibraryId: string;
    genres: string[];
    levels: string[];
    filter: VideoFilter;
    totalCount: number;
    favoriteCount: number;
    isScanning: boolean;
    onLibraryChange: (id: string) => void;
    onFilterChange: (filter: VideoFilter) => void;
    onOpenMain: () => void;
    onOpenSettings: () => void;
    onRefresh: () => void;
}

export default function DesktopSidebar(props: DesktopSidebarProps) {
    const { page, libraries, activeLibraryId, genres, filter, totalCount, favoriteCount, isScanning, onLibraryChange, onFilterChange, onOpenMain, onOpenSettings } = props;
    const [collapsed, setCollapsed] = useState(false);
    const updateFilter = (next: VideoFilter) => { onOpenMain(); onFilterChange(next); };
    const navClass = (active: boolean) => `flex min-h-11 w-full items-center rounded-lg text-left text-sm ${collapsed ? "justify-center" : "gap-3 px-3"} ${active ? "bg-indigo-500/20 text-indigo-200" : "text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100"}`;
    return (
        <aside aria-label="桌面功能列" className={`sticky top-0 hidden h-[100dvh] shrink-0 flex-col overflow-y-auto border-r border-zinc-800 bg-[#121318] py-3 lg:flex ${collapsed ? "w-14 px-1" : "w-60 px-3"}`}>
            <div className={`mb-3 flex min-h-11 items-center ${collapsed ? "justify-center" : "justify-between gap-1"}`}>
                {!collapsed && <span className="min-w-0 truncate text-base font-bold tracking-tight text-zinc-100">StickPlay<span className="text-indigo-400">Server</span></span>}
                <button type="button" onClick={() => setCollapsed((value) => !value)} aria-label={collapsed ? "展開側欄" : "收折側欄"} title={collapsed ? "展開側欄" : "收折側欄"} aria-expanded={!collapsed} className="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg text-zinc-400 hover:bg-zinc-800">{collapsed ? <ChevronsRight size={19} /> : <ChevronsLeft size={19} />}</button>
            </div>
            <label className="relative mb-3 block shrink-0" title={`切換資料庫：${libraries.find((library) => library.id === activeLibraryId)?.name || "尚未選擇"}`}>
                <span className="sr-only">切換資料庫</span>
                <FolderOpen className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-zinc-400" size={18} />
                <select value={activeLibraryId} onChange={(event) => { onOpenMain(); onLibraryChange(event.target.value); }} className={`h-11 w-full appearance-none rounded-lg border border-zinc-700 bg-zinc-900 text-sm outline-none focus:border-indigo-500 ${collapsed ? "text-transparent" : "pl-9 pr-7 text-zinc-100"}`}>
                    {libraries.length === 0 && <option value="">尚未建立資料庫</option>}
                    {libraries.map((library) => <option className="text-zinc-100" key={library.id} value={library.id}>{library.name}</option>)}
                </select>
                {!collapsed && <ChevronDown size={14} className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-zinc-500" />}
            </label>
            <nav className="space-y-1" aria-label="主要導覽">
                <button type="button" aria-label={`所有影片，共 ${totalCount} 部`} title={`所有影片 · ${totalCount}`} onClick={() => updateFilter({ ...filter, search: undefined, genres: undefined, levels: undefined, favorites_only: undefined })} className={navClass(page === "main" && !filter.favorites_only && !filter.genres?.length)}><Film size={18} className="shrink-0" />{!collapsed && <><span className="flex-1">所有影片</span><span className="text-xs tabular-nums text-zinc-500">{totalCount}</span></>}</button>
                <button type="button" aria-label={`我的最愛，共 ${favoriteCount} 部`} title={`我的最愛 · ${favoriteCount}`} onClick={() => updateFilter({ ...filter, search: undefined, favorites_only: true, genres: undefined, levels: undefined })} className={navClass(page === "main" && Boolean(filter.favorites_only))}><Star size={18} className="shrink-0" />{!collapsed && <><span className="flex-1">我的最愛</span><span className="text-xs tabular-nums text-zinc-500">{favoriteCount}</span></>}</button>
            </nav>
            <Header {...props} onFilterChange={updateFilter} variant={collapsed ? "rail" : "sidebar"} />
            {!collapsed && <>
                <div className="my-4 h-px shrink-0 bg-zinc-800" />
                <p className="mb-2 px-3 text-xs text-zinc-500">分類</p>
                <nav className="space-y-1" aria-label="影片分類">
                    <button type="button" onClick={() => updateFilter({ ...filter, genres: undefined, favorites_only: undefined })} className={navClass(false)}><Grid2X2 size={18} /><span>全部類型</span></button>
                    {genres.slice(0, 6).map((genre) => <button type="button" key={genre} onClick={() => updateFilter({ ...filter, genres: [genre], favorites_only: undefined })} className={navClass(page === "main" && filter.genres?.length === 1 && filter.genres[0] === genre)}><span className="h-1.5 w-1.5 shrink-0 rounded-full bg-zinc-500" /><span className="truncate">{genre}</span></button>)}
                </nav>
            </>}
            <div className="mt-auto space-y-2 pt-4">
                <button type="button" onClick={onOpenSettings} aria-label="設定" title="設定" className={navClass(page === "settings")}><Settings size={18} />{!collapsed && <span>設定</span>}</button>
                <div title={isScanning ? "正在掃描資料庫" : "資料庫已就緒"} className={`flex items-center gap-2 border-t border-zinc-800 pt-3 text-xs text-zinc-500 ${collapsed ? "justify-center" : "px-3"}`}><span className={`h-2 w-2 shrink-0 rounded-full ${isScanning ? "animate-pulse bg-amber-400" : "bg-emerald-400"}`} />{!collapsed && (isScanning ? "正在掃描資料庫" : "資料庫已就緒")}</div>
            </div>
        </aside>
    );
}
