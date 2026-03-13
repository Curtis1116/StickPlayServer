import { Star } from "lucide-react";
import { VideoEntry } from "../types";
import VideoCard from "./VideoCard";

interface VideoGridProps {
    videos: VideoEntry[];
    gridSize: number;
    onFavoriteToggled: (id: string, newState: boolean) => void;
    onRatingUpdated: (id: string, newRating: number, nfosPath: string) => void;
    onVideoUpdated: (updated: VideoEntry) => void;
    onVideoRemoved: (id: string) => void;
    onToast: (msg: string) => void;
}

export default function VideoGrid({
    videos,
    gridSize,
    onFavoriteToggled,
    onRatingUpdated,
    onVideoUpdated,
    onVideoRemoved,
    onToast,
}: VideoGridProps) {
    if (videos.length === 0) {
        return (
            <div className="flex flex-col items-center justify-center py-32 text-zinc-700">
                <Star size={48} className="mb-4 text-zinc-700" />
                <p className="text-xl font-medium">找不到符合條件的影片</p>
                <p className="text-sm mt-2 text-zinc-600">
                    請嘗試調整篩選條件或搜尋關鍵字。
                </p>
            </div>
        );
    }

    return (
        <div
            className="grid gap-x-8 gap-y-12 transition-all duration-300"
            style={{
                gridTemplateColumns: `repeat(auto-fill, minmax(${gridSize}px, 1fr))`,
            }}
        >
            {videos.map((video) => (
                <VideoCard
                    key={video.id}
                    video={video}
                    onFavoriteToggled={onFavoriteToggled}
                    onRatingUpdated={onRatingUpdated}
                    onVideoUpdated={onVideoUpdated}
                    onVideoRemoved={onVideoRemoved}
                    onToast={onToast}
                />
            ))}
        </div>
    );
}
