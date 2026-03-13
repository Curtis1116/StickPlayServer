import { useCallback, useEffect, useRef, useState } from "react";
import {
    Search,
    SlidersHorizontal,
    RefreshCw,
    Settings,
    ArrowUpDown,
} from "lucide-react";
import { Library, VideoFilter } from "../types";

interface HeaderProps {
    libraries: Library[];
    activeLibraryId: string;
    onLibraryChange: (id: string) => void;
    genres: string[];
    levels: string[];
    filter: VideoFilter;
    gridSize: number;
    onFilterChange: (filter: VideoFilter) => void;
    onGridSizeChange: (size: number) => void;
    onRefresh: () => void;
    onOpenSettings: () => void;
    isScanning: boolean;
    totalCount?: number;
    currentCount?: number;
}

function FilterDropdown({
    genres,
    levels,
    filter,
    onFilterChange,
}: {
    genres: string[];
    levels: string[];
    filter: VideoFilter;
    onFilterChange: (f: VideoFilter) => void;
}) {
    const [isOpen, setIsOpen] = useState(false);
    const ref = useRef<HTMLDivElement>(null);

    // click outside
    useEffect(() => {
        const handleClickOutside = (e: MouseEvent) => {
            if (ref.current && !ref.current.contains(e.target as Node)) {
                setIsOpen(false);
            }
        };
        document.addEventListener("mousedown", handleClickOutside);
        return () =>
            document.removeEventListener("mousedown", handleClickOutside);
    }, []);

    // 取得目前選取的數量，用來顯示在按鈕上
    const selectedCount =
        (filter.favorites_only ? 1 : 0) +
        (filter.genres?.length || 0) +
        (filter.levels?.length || 0);

    const toggleFavorite = () => {
        onFilterChange({
            ...filter,
            favorites_only: filter.favorites_only ? undefined : true,
        });
    };

    const toggleGenre = (genre: string) => {
        const current = filter.genres || [];
        const next = current.includes(genre)
            ? current.filter((g) => g !== genre)
            : [...current, genre];
        onFilterChange({ ...filter, genres: next.length > 0 ? next : undefined });
    };

    const toggleLevel = (level: string) => {
        const current = filter.levels || [];
        const next = current.includes(level)
            ? current.filter((l) => l !== level)
            : [...current, level];
        onFilterChange({ ...filter, levels: next.length > 0 ? next : undefined });
    };

    const clearFilters = () => {
        onFilterChange({
            ...filter,
            favorites_only: undefined,
            genres: undefined,
            levels: undefined,
        });
        setIsOpen(false);
    };

    return (
        <div className="relative" ref={ref}>
            <button
                onClick={() => setIsOpen(!isOpen)}
                className="flex items-center gap-2 bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-1.5 hover:bg-zinc-800 transition-colors"
            >
                <span className="text-[10px] font-bold text-zinc-500 uppercase">
                    篩選
                </span>
                <span className="text-sm text-zinc-300">
                    {selectedCount > 0 ? `已選取 ${selectedCount} 項` : "全部"}
                </span>
            </button>

            {isOpen && (
                <div className="absolute top-full left-0 mt-2 w-56 bg-zinc-900/95 backdrop-blur-xl border border-zinc-800 rounded-xl shadow-2xl overflow-hidden py-2 z-50">
                    <button
                        onClick={clearFilters}
                        className="w-full text-left px-4 py-2 text-sm text-red-400 font-bold hover:bg-zinc-800/50 transition-colors"
                    >
                        [ 清除篩選 ]
                    </button>

                    <div className="px-4 py-1 flex items-center gap-2 text-[10px] font-bold text-zinc-500 uppercase mt-1">
                        類型
                    </div>
                    <label className="flex items-center gap-3 px-4 py-1.5 hover:bg-zinc-800/50 cursor-pointer">
                        <input
                            type="checkbox"
                            checked={filter.favorites_only || false}
                            onChange={toggleFavorite}
                            className="rounded border-zinc-700 bg-zinc-900 text-indigo-500 focus:ring-indigo-500"
                        />
                        <span className="text-sm text-zinc-300 font-medium">
                            ⭐ 我的最愛
                        </span>
                    </label>
                    {genres.map((g) => (
                        <label
                            key={g}
                            className="flex items-center gap-3 px-4 py-1.5 hover:bg-zinc-800/50 cursor-pointer"
                        >
                            <input
                                type="checkbox"
                                checked={filter.genres?.includes(g) || false}
                                onChange={() => toggleGenre(g)}
                                className="rounded border-zinc-700 bg-zinc-900 text-indigo-500 focus:ring-indigo-500"
                            />
                            <span className="text-sm text-zinc-300 font-medium">{g}</span>
                        </label>
                    ))}

                    <div className="w-full h-px bg-zinc-800 my-2"></div>

                    <div className="px-4 py-1 flex items-center gap-2 text-[10px] font-bold text-zinc-500 uppercase mt-1">
                        分級
                    </div>
                    <label className="flex items-center gap-3 px-4 py-1.5 hover:bg-zinc-800/50 cursor-pointer">
                        <input
                            type="checkbox"
                            checked={filter.levels?.includes("無分級") || false}
                            onChange={() => toggleLevel("無分級")}
                            className="rounded border-zinc-700 bg-zinc-900 text-indigo-500 focus:ring-indigo-500"
                        />
                        <span className="text-sm text-zinc-300 font-medium">無分級</span>
                    </label>
                    {levels.map((l) => (
                        <label
                            key={l}
                            className="flex items-center gap-3 px-4 py-1.5 hover:bg-zinc-800/50 cursor-pointer"
                        >
                            <input
                                type="checkbox"
                                checked={filter.levels?.includes(l) || false}
                                onChange={() => toggleLevel(l)}
                                className="rounded border-zinc-700 bg-zinc-900 text-indigo-500 focus:ring-indigo-500"
                            />
                            <span className="text-sm text-zinc-300 font-medium">{l}</span>
                        </label>
                    ))}
                </div>
            )}
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
    gridSize,
    onFilterChange,
    onGridSizeChange,
    onRefresh,
    onOpenSettings,
    isScanning,
    totalCount = 0,
    currentCount = 0,
}: HeaderProps) {
    const [searchValue, setSearchValue] = useState(filter.search || "");
    const debounceRef = useRef<ReturnType<typeof setTimeout>>(null);

    // 防抖搜尋
    const handleSearchChange = useCallback(
        (value: string) => {
            setSearchValue(value);
            if (debounceRef.current) clearTimeout(debounceRef.current);
            debounceRef.current = setTimeout(() => {
                onFilterChange({ ...filter, search: value || undefined });
            }, 300);
        },
        [filter, onFilterChange]
    );

    useEffect(() => {
        return () => {
            if (debounceRef.current) clearTimeout(debounceRef.current);
        };
    }, []);

    // 排序選項
    const sortOptions = [
        { value: "date_added", label: "加入日期" },
        { value: "release_date", label: "發行日期" },
        { value: "title", label: "標題" },
        { value: "rating", label: "評分" },
        { value: "actor", label: "女優" },
        { value: "id", label: "番號" },
    ];

    const currentSort = filter.sort_by || "date_added";
    const currentOrder = filter.sort_order || "DESC";

    return (
        <header className="sticky top-0 z-50 glass-panel border-b border-zinc-800 shadow-2xl">
            <div className="max-w-[1800px] mx-auto px-6 h-16 flex items-center justify-between gap-4">
                {/* Logo + 篩選 */}
                <div className="flex items-center gap-5">
                    <div className="flex items-baseline gap-3">
                        <span className="text-2xl font-black tracking-tighter bg-gradient-to-br from-indigo-400 to-purple-500 bg-clip-text text-transparent select-none leading-none">
                            StickPlay
                        </span>
                        <span className="text-xs text-zinc-500 font-medium tracking-wider">
                            {currentCount} / {totalCount} 影片
                        </span>
                    </div>

                    {libraries.length > 0 && (
                        <div className="flex items-center">
                            <select
                                value={activeLibraryId}
                                onChange={(e) => onLibraryChange(e.target.value)}
                                className="bg-zinc-900 border border-zinc-800 text-zinc-300 text-sm rounded-lg px-2 py-1 outline-none cursor-pointer"
                            >
                                {libraries.map(lib => (
                                    <option key={lib.id} value={lib.id} className="bg-zinc-900">
                                        {lib.name}
                                    </option>
                                ))}
                            </select>
                        </div>
                    )}

                    <div className="hidden lg:flex items-center gap-3 ml-2">
                        {/* 合併的篩選下拉選單 */}
                        <FilterDropdown
                            genres={genres}
                            levels={levels}
                            filter={filter}
                            onFilterChange={onFilterChange}
                        />

                        {/* 排序 */}
                        <div className="flex items-center gap-2 bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-1.5">
                            <span className="text-[10px] font-bold text-zinc-500 uppercase">
                                排序
                            </span>
                            <select
                                value={currentSort}
                                onChange={(e) =>
                                    onFilterChange({
                                        ...filter,
                                        sort_by: e.target.value,
                                    })
                                }
                                className="bg-transparent text-sm focus:outline-none cursor-pointer text-zinc-300"
                            >
                                {sortOptions.map((opt) => (
                                    <option
                                        key={opt.value}
                                        value={opt.value}
                                        className="bg-zinc-900"
                                    >
                                        {opt.label}
                                    </option>
                                ))}
                            </select>
                            <button
                                onClick={() =>
                                    onFilterChange({
                                        ...filter,
                                        sort_order:
                                            currentOrder === "ASC"
                                                ? "DESC"
                                                : "ASC",
                                    })
                                }
                                className="text-zinc-400 hover:text-white transition-colors"
                                title={
                                    currentOrder === "ASC"
                                        ? "升序 ↑"
                                        : "降序 ↓"
                                }
                            >
                                <ArrowUpDown size={14} />
                            </button>
                        </div>
                    </div>
                </div>

                {/* 搜尋 */}
                <div className="flex-grow max-w-xs relative group">
                    <input
                        type="text"
                        placeholder="搜尋影片..."
                        value={searchValue}
                        onChange={(e) => handleSearchChange(e.target.value)}
                        className="w-full bg-zinc-900 border border-zinc-800 rounded-full py-2 pl-10 pr-10 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500/50 transition-all text-zinc-200 placeholder-zinc-500"
                    />
                    <Search
                        size={16}
                        className="absolute left-3.5 top-1/2 -translate-y-1/2 text-zinc-500"
                    />
                    {searchValue && (
                        <button
                            onClick={() => handleSearchChange("")}
                            className="absolute right-3.5 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-white transition-colors"
                            title="清除搜尋"
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                                <line x1="18" y1="6" x2="6" y2="18"></line>
                                <line x1="6" y1="6" x2="18" y2="18"></line>
                            </svg>
                        </button>
                    )}
                </div>

                {/* 右側工具 */}
                <div className="flex items-center gap-5">
                    <div className="hidden sm:flex items-center gap-3">
                        <SlidersHorizontal
                            size={14}
                            className="text-zinc-500"
                        />
                        <input
                            type="range"
                            min="140"
                            max="320"
                            value={gridSize}
                            onChange={(e) =>
                                onGridSizeChange(Number(e.target.value))
                            }
                            className="w-24 h-1.5 bg-zinc-800 rounded-lg appearance-none cursor-pointer"
                        />
                    </div>
                    <div className="w-px h-6 bg-zinc-800"></div>
                    <button
                        onClick={onRefresh}
                        disabled={isScanning}
                        className="text-zinc-400 hover:text-white transition-colors disabled:opacity-50"
                        title="重新掃描媒體庫"
                    >
                        <RefreshCw
                            size={18}
                            className={isScanning ? "animate-spin" : ""}
                        />
                    </button>
                    <button
                        onClick={onOpenSettings}
                        className="text-zinc-400 hover:text-white transition-colors"
                        title="設定"
                    >
                        <Settings size={18} />
                    </button>
                </div>
            </div>
        </header>
    );
}
