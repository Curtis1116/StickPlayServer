import { useEffect, useMemo, useState } from "react";
import { ArrowDownUp, Check, ChevronDown, FolderCog, FolderOpen, RefreshCw, Search, Settings, SlidersHorizontal, Star } from "lucide-react";
import { Library, VideoFilter } from "../types";
import CatalogDialog from "./CatalogDialog";

interface HeaderProps {
    libraries: Library[];
    activeLibraryId: string;
    onLibraryChange: (id: string) => void;
    genres: string[];
    levels: string[];
    filter: VideoFilter;
    totalCount: number;
    onFilterChange: (filter: VideoFilter) => void;
    onRefresh: () => void;
    onOpenSettings: () => void;
    isScanning: boolean;
    variant?: "mobile" | "sidebar" | "rail";
}
type Panel = "library" | "filter" | "sort" | "search" | null;

const SORT_OPTIONS = [
    { value: "date_added", label: "加入日期" },
    { value: "release_date", label: "發行日期" },
    { value: "title", label: "影片標題" },
    { value: "rating", label: "影片評分" },
    { value: "actor", label: "演員" },
    { value: "id", label: "番號" },
    { value: "level", label: "分級" },
];

function toggleListValue(values: string[] | undefined, value: string) {
    const current = values || [];
    return current.includes(value) ? current.filter((item) => item !== value) : [...current, value];
}

function ChoiceChip({ checked, label, onClick }: { checked: boolean; label: string; onClick: () => void }) {
    return (
        <button
            type="button"
            aria-pressed={checked}
            onClick={onClick}
            className={`flex min-h-11 items-center gap-2 rounded-lg border px-3 text-sm transition-colors ${
                checked
                    ? "border-indigo-500 bg-indigo-500/15 text-indigo-300"
                    : "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-600"
            }`}
        >
            <span className={`flex h-4 w-4 items-center justify-center rounded border ${checked ? "border-indigo-400 bg-indigo-500" : "border-zinc-600"}`}>
                {checked && <Check size={11} />}
            </span>
            <span className="truncate">{label}</span>
        </button>
    );
}

function FilterFields({
    value,
    genres,
    levels,
    onChange,
}: {
    value: VideoFilter;
    genres: string[];
    levels: string[];
    onChange: (filter: VideoFilter) => void;
}) {
    const shownLevels = useMemo(() => {
        const all = ["無分級", ...levels.filter((level) => level !== "無分級")];
        return Array.from(new Set(all));
    }, [levels]);

    return (
        <div className="space-y-5">
            <div>
                <p className="mb-2 text-sm font-medium text-zinc-200">收藏</p>
                <label className="flex min-h-11 cursor-pointer items-center justify-between rounded-lg border border-zinc-800 bg-zinc-950/40 px-3 text-sm text-zinc-300">
                    <span>只看我的最愛</span>
                    <input
                        type="checkbox"
                        checked={Boolean(value.favorites_only)}
                        onChange={(event) => onChange({ ...value, favorites_only: event.target.checked || undefined })}
                        className="h-5 w-5 accent-indigo-500"
                    />
                </label>
            </div>
            <div>
                <div className="mb-2 flex items-center gap-2">
                    <p className="text-sm font-medium text-zinc-200">類型</p>
                    <span className="text-xs text-zinc-600">可複選</span>
                </div>
                <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
                    {genres.map((genre) => (
                        <ChoiceChip
                            key={genre}
                            label={genre}
                            checked={Boolean(value.genres?.includes(genre))}
                            onClick={() => {
                                const next = toggleListValue(value.genres, genre);
                                onChange({ ...value, genres: next.length ? next : undefined });
                            }}
                        />
                    ))}
                    {genres.length === 0 && <p className="col-span-full text-sm text-zinc-600">尚無類型資料</p>}
                </div>
            </div>
            <div>
                <div className="mb-2 flex items-center gap-2">
                    <p className="text-sm font-medium text-zinc-200">分級</p>
                    <span className="text-xs text-zinc-600">可複選</span>
                </div>
                <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
                    {shownLevels.map((level) => (
                        <ChoiceChip
                            key={level}
                            label={level}
                            checked={Boolean(value.levels?.includes(level))}
                            onClick={() => {
                                const next = toggleListValue(value.levels, level);
                                onChange({ ...value, levels: next.length ? next : undefined });
                            }}
                        />
                    ))}
                </div>
            </div>
        </div>
    );
}

function SortFields({ value, onChange }: { value: VideoFilter; onChange: (filter: VideoFilter) => void }) {
    const currentSort = value.sort_by || "date_added";
    const currentOrder = value.sort_order || "DESC";
    return (
        <div className="space-y-5">
            <div>
                <p className="mb-2 text-sm font-medium text-zinc-400">排序欄位</p>
                <div className="space-y-1">
                    {SORT_OPTIONS.map((option) => {
                        const selected = currentSort === option.value;
                        return (
                            <button
                                type="button"
                                key={option.value}
                                onClick={() => onChange({ ...value, sort_by: option.value })}
                                className={`flex min-h-11 w-full items-center gap-3 rounded-lg px-3 text-left text-sm transition-colors ${selected ? "bg-indigo-500/10 text-indigo-300" : "text-zinc-300 hover:bg-zinc-800"}`}
                            >
                                <span className={`flex h-5 w-5 items-center justify-center rounded-full border ${selected ? "border-indigo-400" : "border-zinc-600"}`}>
                                    {selected && <span className="h-2.5 w-2.5 rounded-full bg-indigo-400" />}
                                </span>
                                {option.label}
                            </button>
                        );
                    })}
                </div>
            </div>
            <div>
                <p className="mb-2 text-sm font-medium text-zinc-400">排序方向</p>
                <div className="grid grid-cols-2 overflow-hidden rounded-lg border border-zinc-700">
                    {(["ASC", "DESC"] as const).map((order) => (
                        <button
                            type="button"
                            key={order}
                            onClick={() => onChange({ ...value, sort_order: order })}
                            className={`min-h-11 text-sm font-medium ${currentOrder === order ? "bg-indigo-500 text-white" : "bg-zinc-900 text-zinc-400"}`}
                        >
                            {order === "ASC" ? "升冪" : "降冪"}
                        </button>
                    ))}
                </div>
            </div>
        </div>
    );
}

export default function Header({ libraries, activeLibraryId, onLibraryChange, genres, levels, filter, totalCount, onFilterChange, onRefresh, onOpenSettings, isScanning, variant = "mobile" }: HeaderProps) {
    const [panel, setPanel] = useState<Panel>(null);
    const [draft, setDraft] = useState(filter);
    const activeLibrary = libraries.find((library) => library.id === activeLibraryId);
    const activeCount = (filter.genres?.length || 0) + (filter.levels?.length || 0);
    const sortLabel = SORT_OPTIONS.find((option) => option.value === (filter.sort_by || "date_added"))?.label || "加入日期";
    const sortDescription = `${sortLabel}：${filter.sort_order === "ASC" ? "升冪" : "降冪"}`;
    useEffect(() => { setPanel(null); }, [activeLibraryId, variant]);
    const open = (next: Panel) => { setDraft(filter); setPanel(next); };
    const apply = () => {
        // Apply only the fields belonging to this panel; preserve other live filters.
        if (panel === "search") onFilterChange({ ...filter, search: draft.search?.trim() || undefined });
        if (panel === "sort") onFilterChange({ ...filter, sort_by: draft.sort_by, sort_order: draft.sort_order });
        if (panel === "filter") onFilterChange({ ...filter, genres: draft.genres, levels: draft.levels, favorites_only: draft.favorites_only });
        setPanel(null);
    };
    const clearSearch = () => onFilterChange({ ...filter, search: undefined });
    const clearFilters = () => onFilterChange({
        ...filter,
        genres: undefined,
        levels: undefined,
        favorites_only: undefined,
    });
    const iconClass = "relative flex h-11 w-11 shrink-0 items-center justify-center rounded-lg text-zinc-300 hover:bg-zinc-800 disabled:opacity-50";
    const searchButton = <button type="button" onClick={() => open("search")} onContextMenu={(event) => { event.preventDefault(); clearSearch(); }} aria-label="搜尋影片；按滑鼠右鍵可清除搜尋" title="搜尋影片（右鍵清除）" className={`${iconClass} ${filter.search ? "bg-indigo-500/20 text-indigo-300" : ""} ${variant === "sidebar" ? "flex-1 border border-zinc-700" : ""}`}><Search size={19} /></button>;
    const refreshButton = <button type="button" onClick={onRefresh} disabled={isScanning} aria-label="重新掃描" title={isScanning ? "掃描中" : "重新掃描"} className={`${iconClass} ${variant === "sidebar" ? "flex-1 border border-zinc-700" : ""}`}><RefreshCw size={19} className={isScanning ? "animate-spin" : ""} /></button>;
    const sortButton = <button type="button" onClick={() => open("sort")} aria-label={`排序：${sortDescription}`} title={`排序：${sortDescription}`} className={variant === "sidebar" ? "flex min-h-11 w-full items-center gap-2 rounded-lg border border-zinc-700 px-3 text-sm text-zinc-300 hover:bg-zinc-800" : iconClass}><ArrowDownUp size={19} className="shrink-0" />{variant === "sidebar" && <><span className="min-w-0 flex-1 truncate">{sortLabel} {filter.sort_order === "ASC" ? "↑" : "↓"}</span><ChevronDown size={14} /></>}</button>;
    const filterButton = <button type="button" onClick={() => open("filter")} onContextMenu={(event) => { event.preventDefault(); clearFilters(); }} aria-label={`篩選${activeCount ? `，已選 ${activeCount} 項` : ""}；按滑鼠右鍵可清除篩選`} title="篩選（右鍵清除）" className={`${variant === "sidebar" ? "flex min-h-11 w-full items-center gap-2 rounded-lg border border-zinc-700 px-3 text-sm" : iconClass} ${activeCount ? "bg-indigo-500/20 text-indigo-300" : "text-zinc-300"}`}><SlidersHorizontal size={19} />{variant === "sidebar" && <span className="flex-1 text-left">篩選</span>}{activeCount > 0 && <span className={variant === "sidebar" ? "rounded-full bg-indigo-500 px-1.5 text-xs text-white" : "absolute right-0 top-0 rounded-full bg-indigo-500 px-1 text-[10px] text-white"}>{activeCount}</span>}</button>;
    return <>
        {variant === "mobile" ? <header className="sticky top-0 z-30 border-b border-zinc-800 bg-[#101115] lg:hidden">
            <nav aria-label="影片工具列" className="flex h-14 items-center min-[360px]:px-1.5 sm:px-3">
                <button type="button" onClick={() => open("library")} aria-label={`切換資料庫：${activeLibrary?.name || "尚未選擇"}`} title={activeLibrary?.name || "切換資料庫"} className="flex h-11 min-w-11 min-[360px]:mr-1 flex-1 items-center justify-center gap-1 overflow-hidden rounded-lg border border-zinc-700 bg-zinc-900 px-1.5 text-sm text-zinc-100"><FolderOpen size={18} className="shrink-0" /><span className="hidden min-w-0 truncate min-[360px]:block">{activeLibrary?.name || "資料庫"}</span><ChevronDown size={12} className="hidden shrink-0 min-[360px]:block" /></button>
                {sortButton}{filterButton}
                <button type="button" onClick={() => onFilterChange({ ...filter, favorites_only: !filter.favorites_only || undefined })} aria-label="只看我的最愛" aria-pressed={Boolean(filter.favorites_only)} title="只看我的最愛" className={`${iconClass} ${filter.favorites_only ? "bg-indigo-500/20 text-indigo-300" : ""}`}><Star size={19} fill={filter.favorites_only ? "currentColor" : "none"} /></button>
                {searchButton}{refreshButton}
                <button type="button" onClick={onOpenSettings} aria-label="設定" title="設定" className={iconClass}><Settings size={19} /></button>
            </nav>
        </header> : <div className={variant === "sidebar" ? "mt-4 space-y-2" : "mt-2 flex flex-col items-center gap-1"}>
            {variant === "sidebar" ? <div className="flex gap-2">{searchButton}{refreshButton}</div> : <>{searchButton}{refreshButton}</>}
            {sortButton}{filterButton}
        </div>}
        {panel && <CatalogDialog title={{ library: "切換資料庫", filter: "篩選", sort: "排序", search: "搜尋影片" }[panel]} onClose={() => setPanel(null)}>
            {panel === "library" && <div className="space-y-2">
                <p className="pb-2 text-sm text-zinc-400">{activeLibrary?.name || "尚未選擇資料庫"} · 共 {totalCount} 部</p>
                {libraries.map((library) => <button type="button" key={library.id} onClick={() => { onLibraryChange(library.id); setPanel(null); }} className={`flex min-h-14 w-full items-center gap-3 rounded-lg border px-3 text-left ${library.id === activeLibraryId ? "border-indigo-500 bg-indigo-500/10 text-indigo-300" : "border-zinc-700 text-zinc-300"}`}><FolderOpen size={20} className="shrink-0" /><span className="min-w-0 flex-1 break-words">{library.name}</span>{library.id === activeLibraryId && <Check size={18} className="shrink-0" />}</button>)}
                <button type="button" onClick={() => { setPanel(null); onOpenSettings(); }} className="flex min-h-12 w-full items-center gap-3 px-3 text-sm text-zinc-400"><FolderCog size={20} />管理媒體庫</button>
            </div>}
            {panel === "search" && <form onSubmit={(event) => { event.preventDefault(); apply(); }}>
                <label className="text-sm text-zinc-400" htmlFor={`catalog-search-${variant}`}>片名、演員或番號</label>
                <input id={`catalog-search-${variant}`} type="search" value={draft.search || ""} onChange={(event) => setDraft({ ...draft, search: event.target.value })} className="mt-2 h-12 w-full rounded-lg border border-zinc-700 bg-zinc-900 px-3 text-base outline-none focus:border-indigo-500" />
                <div className="mt-4 grid grid-cols-2 gap-3"><button type="button" onClick={() => { clearSearch(); setPanel(null); }} className="min-h-11 rounded-lg border border-zinc-700">清除搜尋</button><button type="submit" className="min-h-11 rounded-lg bg-indigo-500 font-bold">搜尋</button></div>
            </form>}
            {panel === "filter" && <FilterFields value={draft} genres={genres} levels={levels} onChange={setDraft} />}
            {panel === "sort" && <SortFields value={draft} onChange={setDraft} />}
            {(panel === "filter" || panel === "sort") && <div className="sticky bottom-0 mt-4 flex gap-3 border-t border-zinc-800 bg-[#15161b] pt-4">
                {panel === "filter" && <button type="button" onClick={() => setDraft({ ...draft, favorites_only: undefined, genres: undefined, levels: undefined })} className="min-h-11 flex-1 rounded-lg border border-zinc-700 text-sm">重設</button>}
                <button type="button" onClick={apply} className="min-h-11 flex-1 rounded-lg bg-indigo-500 text-sm font-bold">套用{panel === "filter" ? "篩選" : "排序"}</button>
            </div>}
        </CatalogDialog>}
    </>;
}
