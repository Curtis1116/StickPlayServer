import { useCallback, useEffect, useRef, useState } from "react";
import {
    Search,
    RefreshCw,
    Settings,
    ArrowUpDown,
    FolderOpen,
    ListFilter,
} from "lucide-react";
import { Library, VideoFilter } from "../types";

interface HeaderProps {
    libraries: Library[];
    activeLibraryId: string;
    onLibraryChange: (id: string) => void;
    genres: string[];
    levels: string[];
    filter: VideoFilter;
    onFilterChange: (filter: VideoFilter) => void;
    onRefresh: () => void;
    onOpenSettings: () => void;
    isScanning: boolean;
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
                className={`relative w-9 h-9 flex items-center justify-center transition-colors ${isOpen ? "text-indigo-400" : "text-zinc-400 hover:text-white"}`}
                title="篩選器"
            >
                <ListFilter size={18} />
                {selectedCount > 0 && (
                    <span className="absolute top-1 right-1 w-2 h-2 bg-indigo-500 rounded-full border border-zinc-900" />
                )}
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
    onFilterChange,
    onRefresh,
    onOpenSettings,
    isScanning,
}: HeaderProps) {
    const [searchValue, setSearchValue] = useState(filter.search || "");
    const [isSearchVisible, setIsSearchVisible] = useState(false);
    const [isLibDropdownOpen, setIsLibDropdownOpen] = useState(false);
    const [isSortDropdownOpen, setIsSortDropdownOpen] = useState(false);
    const debounceRef = useRef<ReturnType<typeof setTimeout>>(null);
    const sortRef = useRef<HTMLDivElement>(null);
    const libRef = useRef<HTMLDivElement>(null);

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
        const handleClickOutside = (e: MouseEvent) => {
            if (sortRef.current && !sortRef.current.contains(e.target as Node)) {
                setIsSortDropdownOpen(false);
            }
            if (libRef.current && !libRef.current.contains(e.target as Node)) {
                setIsLibDropdownOpen(false);
            }
        };
        document.addEventListener("mousedown", handleClickOutside);
        return () => {
            document.removeEventListener("mousedown", handleClickOutside);
            if (debounceRef.current) clearTimeout(debounceRef.current);
        };
    }, []);

    // 排序選項
    const sortOptions = [
        { value: "date_added", label: "加入日期" },
        { value: "release_date", label: "發行日期" },
        { value: "title", label: "影片標題" },
        { value: "rating", label: "影片評分" },
        { value: "actor", label: "女優" },
        { value: "id", label: "番號" },
        { value: "level", label: "分級" },
    ];

    const currentSort = filter.sort_by || "date_added";
    const currentOrder = filter.sort_order || "DESC";

    return (
        <header className="sticky top-0 z-50 glass-panel border-b border-zinc-800 shadow-2xl">
            <div className="max-w-[1800px] mx-auto px-4 h-16 flex items-center justify-between gap-2">
                {/* Logo Section */}
                {!isSearchVisible && (
                    <div className="flex items-center gap-3">
                        <span className="text-xl font-black tracking-tighter bg-gradient-to-br from-indigo-400 to-purple-500 bg-clip-text text-transparent select-none">
                            StickPlay
                        </span>
                    </div>
                )}

                {/* Main Tools */}
                <div className={`flex items-center gap-1 sm:gap-4 ${isSearchVisible ? "w-full" : ""}`}>
                    {/* Search Field (Conditional) */}
                    {isSearchVisible ? (
                        <div className="flex-grow relative animate-in slide-in-from-right-4 duration-300 flex items-center gap-2">
                            <div className="relative flex-grow group">
                                <input
                                    autoFocus
                                    type="text"
                                    placeholder="搜尋影片..."
                                    value={searchValue}
                                    onChange={(e) => handleSearchChange(e.target.value)}
                                    className="w-full bg-zinc-900 border border-zinc-800 rounded-full py-1.5 pl-9 pr-9 text-sm focus:outline-none focus:ring-1 focus:ring-indigo-500/50 transition-all text-zinc-200"
                                />
                                <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-zinc-500" />
                                {searchValue && (
                                    <button onClick={() => handleSearchChange("")} className="absolute right-3 top-1/2 -translate-y-1/2 text-zinc-500">
                                        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                                    </button>
                                )}
                            </div>
                            <button 
                                onClick={() => { setIsSearchVisible(false); handleSearchChange(""); }}
                                className="text-xs font-bold text-zinc-400 px-2"
                            >
                                取消
                            </button>
                        </div>
                    ) : (
                        <div className="flex items-center gap-1 sm:gap-3">
                            {/* Library Select Icon */}
                            {libraries.length > 0 && (
                                <div className="relative" ref={libRef}>
                                    <button 
                                        onClick={() => setIsLibDropdownOpen(!isLibDropdownOpen)}
                                        className="w-9 h-9 flex items-center justify-center text-zinc-400 hover:text-white transition-colors"
                                        title="切換媒體庫"
                                    >
                                        <FolderOpen size={18} />
                                    </button>
                                    {isLibDropdownOpen && (
                                        <div className="absolute top-full left-0 mt-2 w-48 bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl overflow-hidden py-1 z-50">
                                            {libraries.map(lib => (
                                                <button
                                                    key={lib.id}
                                                    onClick={() => { onLibraryChange(lib.id); setIsLibDropdownOpen(false); }}
                                                    className={`w-full text-left px-4 py-2 text-sm transition-colors ${activeLibraryId === lib.id ? "bg-indigo-500/10 text-indigo-400 font-bold" : "text-zinc-400 hover:bg-zinc-800"}`}
                                                >
                                                    {lib.name}
                                                </button>
                                            ))}
                                        </div>
                                    )}
                                </div>
                            )}

                            {/* Filter Icon */}
                            <FilterDropdown
                                genres={genres}
                                levels={levels}
                                filter={filter}
                                onFilterChange={onFilterChange}
                            />

                            {/* Sort Icon */}
                            <div className="relative" ref={sortRef}>
                                <button 
                                    onClick={() => setIsSortDropdownOpen(!isSortDropdownOpen)}
                                    className={`w-9 h-9 flex items-center justify-center transition-colors ${isSortDropdownOpen ? "text-indigo-400" : "text-zinc-400 hover:text-white"}`}
                                    title="排序"
                                >
                                    <ArrowUpDown size={18} />
                                </button>
                                {isSortDropdownOpen && (
                                    <div className="absolute top-full right-0 mt-2 w-40 bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl overflow-hidden py-1 z-50">
                                        <div className="px-4 py-1.5 border-b border-zinc-800 flex items-center justify-between">
                                            <span className="text-[10px] font-bold text-zinc-500 uppercase">順序</span>
                                            <button
                                                onClick={() => onFilterChange({ ...filter, sort_order: currentOrder === "ASC" ? "DESC" : "ASC" })}
                                                className="text-xs text-indigo-400 font-bold"
                                            >
                                                {currentOrder === "ASC" ? "升序" : "降序"}
                                            </button>
                                        </div>
                                        {sortOptions.map(opt => (
                                            <button
                                                key={opt.value}
                                                onClick={() => { onFilterChange({ ...filter, sort_by: opt.value }); setIsSortDropdownOpen(false); }}
                                                className={`w-full text-left px-4 py-2 text-sm transition-colors ${currentSort === opt.value ? "text-indigo-400 font-bold bg-indigo-500/5" : "text-zinc-400 hover:bg-zinc-800"}`}
                                            >
                                                {opt.label}
                                            </button>
                                        ))}
                                    </div>
                                )}
                            </div>

                            {/* Search Icon */}
                            <button 
                                onClick={() => setIsSearchVisible(true)}
                                className="w-9 h-9 flex items-center justify-center text-zinc-400 hover:text-white transition-colors"
                            >
                                <Search size={18} />
                            </button>
                        </div>
                    )}
                </div>

                {/* Right Utils */}
                {!isSearchVisible && (
                    <div className="flex items-center gap-1 sm:gap-4">
                        <button
                            onClick={onRefresh}
                            disabled={isScanning}
                            className="w-9 h-9 flex items-center justify-center text-zinc-400 hover:text-white transition-colors disabled:opacity-50"
                            title="重新掃描"
                        >
                            <RefreshCw size={18} className={isScanning ? "animate-spin" : ""} />
                        </button>
                        <button
                            onClick={onOpenSettings}
                            className="w-9 h-9 flex items-center justify-center text-zinc-400 hover:text-white transition-colors"
                            title="設定"
                        >
                            <Settings size={18} />
                        </button>
                    </div>
                )}
            </div>
        </header>
    );
}
