import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
    Check,
    ChevronDown,
    FolderCog,
    FolderOpen,
    RefreshCw,
    Search,
    Settings,
    SlidersHorizontal,
    X,
} from "lucide-react";
import { Library, VideoFilter } from "../types";

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
}

type MobilePanel = "library" | "filter" | "sort" | null;
type DesktopPopover = "filter" | "sort" | null;

const SORT_OPTIONS = [
    { value: "date_added", label: "加入日期" },
    { value: "release_date", label: "發行日期" },
    { value: "title", label: "影片標題" },
    { value: "rating", label: "影片評分" },
    { value: "actor", label: "演員" },
    { value: "id", label: "番號" },
    { value: "level", label: "分級" },
];

function filterCount(filter: VideoFilter) {
    return Number(Boolean(filter.favorites_only)) + (filter.genres?.length || 0) + (filter.levels?.length || 0);
}

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

export default function Header({
    libraries,
    activeLibraryId,
    onLibraryChange,
    genres,
    levels,
    filter,
    totalCount,
    onFilterChange,
    onRefresh,
    onOpenSettings,
    isScanning,
}: HeaderProps) {
    const [searchValue, setSearchValue] = useState(filter.search || "");
    const [mobilePanel, setMobilePanel] = useState<MobilePanel>(null);
    const [desktopPopover, setDesktopPopover] = useState<DesktopPopover>(null);
    const [draftFilter, setDraftFilter] = useState<VideoFilter>(filter);
    const debounceRef = useRef<ReturnType<typeof setTimeout>>(null);
    const filterRef = useRef(filter);
    const desktopToolsRef = useRef<HTMLDivElement>(null);
    const activeLibrary = libraries.find((library) => library.id === activeLibraryId);
    const activeFilterCount = filterCount(filter);
    const currentSortLabel = SORT_OPTIONS.find((option) => option.value === (filter.sort_by || "date_added"))?.label || "加入日期";

    useEffect(() => {
        filterRef.current = filter;
        setSearchValue(filter.search || "");
    }, [filter]);

    const handleSearchChange = useCallback((value: string) => {
        setSearchValue(value);
        if (debounceRef.current) clearTimeout(debounceRef.current);
        debounceRef.current = setTimeout(() => {
            onFilterChange({ ...filterRef.current, search: value || undefined });
        }, 300);
    }, [onFilterChange]);

    useEffect(() => () => {
        if (debounceRef.current) clearTimeout(debounceRef.current);
    }, []);

    useEffect(() => {
        if (!mobilePanel) return;
        const previousOverflow = document.body.style.overflow;
        document.body.style.overflow = "hidden";
        return () => { document.body.style.overflow = previousOverflow; };
    }, [mobilePanel]);

    useEffect(() => {
        const close = (event: MouseEvent) => {
            if (desktopToolsRef.current && !desktopToolsRef.current.contains(event.target as Node)) {
                setDesktopPopover(null);
            }
        };
        document.addEventListener("mousedown", close);
        return () => document.removeEventListener("mousedown", close);
    }, []);

    const openMobilePanel = (panel: Exclude<MobilePanel, null>) => {
        setDraftFilter({ ...filter, search: searchValue || undefined });
        setMobilePanel(panel);
    };

    const openDesktopPopover = (panel: Exclude<DesktopPopover, null>) => {
        setDraftFilter({ ...filter, search: searchValue || undefined });
        setDesktopPopover((current) => current === panel ? null : panel);
    };

    const applyDraft = () => {
        onFilterChange(draftFilter);
        setMobilePanel(null);
        setDesktopPopover(null);
    };

    const clearDraftFilters = () => setDraftFilter({
        ...draftFilter,
        favorites_only: undefined,
        genres: undefined,
        levels: undefined,
    });

    return (
        <>
            <header className="sticky top-0 z-30 border-b border-zinc-800 bg-[#101115]/95 backdrop-blur-xl">
                <div className="lg:hidden">
                    <div className="flex h-14 items-center gap-2 px-3">
                        <span className="flex-1 text-lg font-bold tracking-tight text-zinc-100">
                            StickPlay<span className="text-indigo-400">Server</span>
                        </span>
                        <button type="button" onClick={onRefresh} disabled={isScanning} aria-label="重新掃描" className="flex h-11 w-11 items-center justify-center rounded-lg text-zinc-400 hover:bg-zinc-800 hover:text-white disabled:opacity-50">
                            <RefreshCw size={19} className={isScanning ? "animate-spin" : ""} />
                        </button>
                        <button type="button" onClick={onOpenSettings} aria-label="設定" className="flex h-11 w-11 items-center justify-center rounded-lg text-zinc-400 hover:bg-zinc-800 hover:text-white">
                            <Settings size={20} />
                        </button>
                    </div>
                    <div className="flex h-12 items-center gap-2 border-t border-zinc-800/60 px-3">
                        <button type="button" onClick={() => openMobilePanel("library")} className="flex h-10 min-w-0 flex-1 items-center gap-2 rounded-lg border border-zinc-700 bg-zinc-900 px-3 text-sm font-medium text-zinc-100">
                            <FolderOpen size={17} className="shrink-0 text-zinc-400" />
                            <span className="truncate">{activeLibrary?.name || "選擇媒體庫"}</span>
                            <ChevronDown size={15} className="ml-auto shrink-0 text-zinc-500" />
                        </button>
                        <span className="shrink-0 text-xs tabular-nums text-zinc-500">{totalCount} 部</span>
                        <button type="button" onClick={() => openMobilePanel("sort")} className="flex h-10 shrink-0 items-center gap-1 rounded-lg border border-zinc-700 px-3 text-sm font-medium text-zinc-200">
                            排序 <ChevronDown size={14} />
                        </button>
                    </div>
                    <div className="flex gap-2 px-3 pb-3 pt-1">
                        <label className="relative min-w-0 flex-1">
                            <span className="sr-only">搜尋演員或番號</span>
                            <Search size={17} className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-zinc-500" />
                            <input type="search" value={searchValue} onChange={(event) => handleSearchChange(event.target.value)} placeholder="搜尋演員、番號" className="h-11 w-full rounded-lg border border-zinc-700 bg-zinc-900 pl-10 pr-3 text-base text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-indigo-500" />
                        </label>
                        <button type="button" onClick={() => openMobilePanel("filter")} className={`relative flex h-11 shrink-0 items-center gap-2 rounded-lg border px-3 text-sm font-medium ${activeFilterCount ? "border-indigo-500 bg-indigo-500/10 text-indigo-300" : "border-zinc-700 text-zinc-300"}`}>
                            <SlidersHorizontal size={17} />
                            篩選{activeFilterCount > 0 && ` · ${activeFilterCount}`}
                        </button>
                    </div>
                </div>

                <div className="hidden px-8 py-5 lg:block">
                    <div className="flex items-center gap-5">
                        <label className="relative max-w-4xl flex-1">
                            <span className="sr-only">搜尋影片</span>
                            <Search size={19} className="pointer-events-none absolute left-4 top-1/2 -translate-y-1/2 text-zinc-500" />
                            <input type="search" value={searchValue} onChange={(event) => handleSearchChange(event.target.value)} placeholder="搜尋片名、演員或番號" className="h-12 w-full rounded-lg border border-zinc-700 bg-zinc-900/70 pl-12 pr-4 text-sm text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-indigo-500" />
                        </label>
                        <button type="button" onClick={onRefresh} disabled={isScanning} className="flex h-12 items-center gap-2 rounded-lg border border-zinc-700 px-4 text-sm font-medium text-zinc-300 transition-colors hover:border-zinc-600 hover:text-white disabled:opacity-50">
                            <RefreshCw size={18} className={isScanning ? "animate-spin" : ""} />
                            {isScanning ? "掃描中" : "重新掃描"}
                        </button>
                    </div>
                    <div className="mt-6 flex items-end justify-between gap-4">
                        <div>
                            <h1 className="text-3xl font-bold tracking-tight text-zinc-100">{filter.favorites_only ? "我的最愛" : "所有影片"}</h1>
                            <p className="mt-1 text-sm text-zinc-500">{activeLibrary?.name || "尚未選擇媒體庫"} · 共 {totalCount} 部</p>
                        </div>
                        <div ref={desktopToolsRef} className="relative flex items-center gap-2">
                            <button type="button" onClick={() => openDesktopPopover("filter")} className={`flex h-11 items-center gap-2 rounded-lg border px-4 text-sm font-medium ${activeFilterCount ? "border-indigo-500 bg-indigo-500/10 text-indigo-300" : "border-zinc-700 text-zinc-300 hover:border-zinc-600"}`}>
                                <SlidersHorizontal size={17} />
                                篩選{activeFilterCount > 0 && ` · ${activeFilterCount}`}
                            </button>
                            <button type="button" onClick={() => openDesktopPopover("sort")} className="flex h-11 min-w-48 items-center justify-between gap-3 rounded-lg border border-zinc-700 px-4 text-sm text-zinc-300 hover:border-zinc-600">
                                {currentSortLabel}：{filter.sort_order === "ASC" ? "升冪" : "降冪"}
                                <ChevronDown size={15} />
                            </button>
                            {desktopPopover && (
                                <div className="absolute right-0 top-full z-50 mt-2 max-h-[70vh] w-[420px] overflow-y-auto rounded-xl border border-zinc-700 bg-zinc-900 p-5 shadow-2xl">
                                    <div className="mb-4 flex items-center justify-between">
                                        <h2 className="text-lg font-bold">{desktopPopover === "filter" ? "篩選" : "排序"}</h2>
                                        <button type="button" onClick={() => setDesktopPopover(null)} aria-label="關閉" className="flex h-9 w-9 items-center justify-center rounded-lg text-zinc-500 hover:bg-zinc-800 hover:text-white"><X size={18} /></button>
                                    </div>
                                    {desktopPopover === "filter" ? (
                                        <FilterFields value={draftFilter} genres={genres} levels={levels} onChange={setDraftFilter} />
                                    ) : (
                                        <SortFields value={draftFilter} onChange={setDraftFilter} />
                                    )}
                                    <div className="mt-5 grid grid-cols-2 gap-3 border-t border-zinc-800 pt-4">
                                        {desktopPopover === "filter" && <button type="button" onClick={clearDraftFilters} className="min-h-11 rounded-lg border border-zinc-700 text-sm text-zinc-300">重設</button>}
                                        <button type="button" onClick={applyDraft} className={`${desktopPopover === "sort" ? "col-span-2" : ""} min-h-11 rounded-lg bg-indigo-500 text-sm font-bold text-white hover:bg-indigo-400`}>套用{desktopPopover === "filter" ? "篩選" : "排序"}</button>
                                    </div>
                                </div>
                            )}
                        </div>
                    </div>
                </div>
            </header>

            {mobilePanel && (
                <div className="fixed inset-0 z-[100] flex items-end bg-black/65 lg:hidden" role="presentation" onMouseDown={(event) => event.target === event.currentTarget && setMobilePanel(null)}>
                    <section role="dialog" aria-modal="true" aria-label={mobilePanel === "library" ? "切換媒體庫" : mobilePanel === "filter" ? "篩選" : "排序"} className="max-h-[86dvh] w-full overflow-y-auto rounded-t-2xl border-t border-zinc-700 bg-[#15161b] px-4 pb-[calc(1rem+env(safe-area-inset-bottom))] pt-2 shadow-2xl">
                        <div className="mx-auto mb-2 h-1 w-12 rounded-full bg-zinc-600" />
                        <div className="sticky top-0 z-10 mb-3 flex items-center justify-between bg-[#15161b] py-2">
                            <div>
                                <h2 className="text-xl font-bold text-zinc-100">{mobilePanel === "library" ? "切換媒體庫" : mobilePanel === "filter" ? "篩選" : "排序"}</h2>
                                {mobilePanel === "library" && <p className="mt-1 text-sm text-zinc-500">選擇要瀏覽的資料庫</p>}
                            </div>
                            <button type="button" onClick={() => setMobilePanel(null)} aria-label="關閉" className="flex h-11 w-11 items-center justify-center rounded-lg text-zinc-400 hover:bg-zinc-800"><X size={22} /></button>
                        </div>

                        {mobilePanel === "library" && (
                            <div className="space-y-2">
                                {libraries.map((library) => {
                                    const selected = library.id === activeLibraryId;
                                    return (
                                        <button type="button" key={library.id} onClick={() => { onLibraryChange(library.id); setMobilePanel(null); }} className={`flex min-h-16 w-full items-center gap-3 rounded-xl border px-4 text-left ${selected ? "border-indigo-500 bg-indigo-500/10" : "border-zinc-700 bg-zinc-900"}`}>
                                            <FolderOpen size={22} className={selected ? "text-indigo-400" : "text-zinc-500"} />
                                            <span className="min-w-0 flex-1">
                                                <span className="block truncate font-medium text-zinc-100">{library.name}</span>
                                                {selected && <span className="block text-xs text-indigo-400">目前使用</span>}
                                            </span>
                                            {selected ? <span className="flex h-7 w-7 items-center justify-center rounded-full bg-indigo-500 text-white"><Check size={16} /></span> : <span className="h-6 w-6 rounded-full border-2 border-zinc-600" />}
                                        </button>
                                    );
                                })}
                                <button type="button" onClick={() => { setMobilePanel(null); onOpenSettings(); }} className="mt-4 flex min-h-14 w-full items-center gap-3 rounded-xl border border-zinc-700 px-4 text-left text-zinc-300">
                                    <FolderCog size={20} className="text-indigo-400" />
                                    <span className="flex-1">管理媒體庫</span>
                                    <span aria-hidden="true">›</span>
                                </button>
                                <p className="pt-1 text-xs text-zinc-600">點選後立即切換</p>
                            </div>
                        )}

                        {mobilePanel === "filter" && (
                            <>
                                <FilterFields value={draftFilter} genres={genres} levels={levels} onChange={setDraftFilter} />
                                <p className="mt-4 text-xs text-zinc-500">已選 {filterCount(draftFilter)} 項條件</p>
                                <div className="sticky bottom-0 -mx-4 mt-4 grid grid-cols-2 gap-3 border-t border-zinc-800 bg-[#15161b] px-4 pt-4">
                                    <button type="button" onClick={clearDraftFilters} className="min-h-12 rounded-lg border border-zinc-700 text-sm font-medium text-zinc-300">重設</button>
                                    <button type="button" onClick={applyDraft} className="min-h-12 rounded-lg bg-indigo-500 text-sm font-bold text-white">套用篩選</button>
                                </div>
                            </>
                        )}

                        {mobilePanel === "sort" && (
                            <>
                                <SortFields value={draftFilter} onChange={setDraftFilter} />
                                <p className="mt-4 text-xs text-zinc-500">目前：{currentSortLabel}，{filter.sort_order === "ASC" ? "升冪" : "降冪"}</p>
                                <div className="sticky bottom-0 -mx-4 mt-4 border-t border-zinc-800 bg-[#15161b] px-4 pt-4">
                                    <button type="button" onClick={applyDraft} className="min-h-12 w-full rounded-lg bg-indigo-500 text-sm font-bold text-white">套用排序</button>
                                </div>
                            </>
                        )}
                    </section>
                </div>
            )}
        </>
    );
}
