import { Film, Settings, Star } from "lucide-react";

interface MobileNavProps {
    active: "videos" | "favorites" | "settings";
    onVideos: () => void;
    onFavorites: () => void;
    onSettings: () => void;
}

export default function MobileNav({ active, onVideos, onFavorites, onSettings }: MobileNavProps) {
    const itemClass = (selected: boolean) =>
        `flex min-h-14 flex-1 flex-col items-center justify-center gap-0.5 text-[11px] font-medium transition-colors ${
            selected ? "text-indigo-400" : "text-zinc-500"
        }`;

    return (
        <nav className="fixed inset-x-0 bottom-0 z-40 flex border-t border-zinc-800 bg-[#111216]/98 pb-[env(safe-area-inset-bottom)] lg:hidden" aria-label="手機導覽">
            <button type="button" onClick={onVideos} className={itemClass(active === "videos")}>
                <Film size={21} />
                <span>影片</span>
            </button>
            <button type="button" onClick={onFavorites} className={itemClass(active === "favorites")}>
                <Star size={21} />
                <span>最愛</span>
            </button>
            <button type="button" onClick={onSettings} className={itemClass(active === "settings")}>
                <Settings size={21} />
                <span>設定</span>
            </button>
        </nav>
    );
}
