import { Star } from "lucide-react";
import { VideoEntry } from "../types";
import VideoCard from "./VideoCard";

interface VideoGridProps {
    videos: VideoEntry[];
    onFavoriteToggled: (id: string, newState: boolean) => void;
    onVideoUpdated: (updated: VideoEntry) => void;
    onVideoRemoved: (id: string) => void;
    onToast: (msg: string) => void;
    disableHover: boolean;
    onModalStateChange: (open: boolean) => void;
}

export default function VideoGrid({
    videos,
    onFavoriteToggled,
    onVideoUpdated,
    onVideoRemoved,
    onToast,
    disableHover,
    onModalStateChange,
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
            className="grid gap-x-3 gap-y-6 sm:gap-x-8 sm:gap-y-12 transition-all duration-300"
            style={{
                gridTemplateColumns: `repeat(auto-fill, minmax(105px, 1fr))`,
            }}
        >
            {videos.map((video) => (
                <VideoCard
                    key={video.id}
                    video={video}
                    onFavoriteToggled={onFavoriteToggled}
                    onVideoUpdated={onVideoUpdated}
                    onVideoRemoved={onVideoRemoved}
                    onToast={onToast}
                    disableHover={disableHover}
                    onModalStateChange={onModalStateChange}
                />
            ))}
        </div>
    );
}
