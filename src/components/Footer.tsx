interface FooterProps {
    totalCount: number;
    favoriteCount: number;
}

export default function Footer({ totalCount, favoriteCount }: FooterProps) {
    return (
        <footer className="p-6 bg-zinc-900/50 border-t border-zinc-800 text-[10px] text-zinc-500 flex justify-between items-center">
            <div className="flex gap-6">
                <span>項目總數: {totalCount}</span>
                <span>我的最愛: {favoriteCount}</span>
            </div>
            <div>StickPlay | 影片管理庫 v0.1.0</div>
        </footer>
    );
}
