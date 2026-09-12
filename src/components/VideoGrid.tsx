import { Star } from "lucide-react";
import { VideoEntry } from "../types";
import VideoCard from "./VideoCard";
import { useWindowVirtualizer } from "@tanstack/react-virtual";
import { useRef, useState, useLayoutEffect, useMemo } from "react";

interface VideoGridProps {
    videos: VideoEntry[];
    onFavoriteToggled: (id: string, newState: boolean) => void;
    onVideoUpdated: (updated: VideoEntry) => void;
    onVideoRemoved: (id: string) => void;
    onToast: (msg: string) => void;
    onModalStateChange: (open: boolean) => void;
}

export default function VideoGrid({
    videos,
    onFavoriteToggled,
    onVideoUpdated,
    onVideoRemoved,
    onToast,
    onModalStateChange,
}: VideoGridProps) {
    const parentRef = useRef<HTMLDivElement>(null);
    // 初始化為視窗寬度，避免初次渲染出現 1-column 或空白的情況
    const [containerWidth, setContainerWidth] = useState(() => 
        typeof window !== 'undefined' ? window.innerWidth - 24 : 390
    );
    const [offsetTop, setOffsetTop] = useState(0);

    useLayoutEffect(() => {
        if (!parentRef.current) return;
        
        // 紀錄目前的 offsetTop，供 WindowVirtualizer 正確減去 Header 等區域高度
        setOffsetTop(parentRef.current.offsetTop);

        const observer = new ResizeObserver((entries) => {
            for (const entry of entries) {
                if (entry.contentRect.width > 0) {
                    setContainerWidth(entry.contentRect.width);
                }
            }
        });
        
        observer.observe(parentRef.current);
        return () => observer.disconnect();
    }, []);

    const columns = useMemo(() => {
        if (containerWidth < 640) return 3;
        if (containerWidth < 1024) return 4;
        return 5;
    }, [containerWidth]);

    const videoRows = useMemo(() => {
        const rows: VideoEntry[][] = [];
        for (let i = 0; i < videos.length; i += columns) {
            rows.push(videos.slice(i, i + columns));
        }
        return rows;
    }, [videos, columns]);

    const estimateRowHeight = useMemo(() => {
        const isDesktop = typeof window !== "undefined" && window.matchMedia("(min-width: 1024px)").matches;
        const gapX = containerWidth >= 640 ? 20 : 8;
        const gapY = isDesktop ? 28 : 14;
        const itemWidth = (containerWidth - (columns - 1) * gapX) / columns;
        const detailsHeight = isDesktop ? 112 : 48;
        return itemWidth * 1.5 + detailsHeight + gapY;
    }, [containerWidth, columns]);

    const virtualizer = useWindowVirtualizer({
        count: videoRows.length,
        estimateSize: () => estimateRowHeight,
        overscan: 5,
        scrollMargin: offsetTop,
    });

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

    const items = virtualizer.getVirtualItems();

    return (
        <div ref={parentRef} className="w-full">
            <div
                style={{
                    height: `${virtualizer.getTotalSize()}px`,
                    width: "100%",
                    position: "relative",
                }}
            >
                {items.map((virtualRow) => {
                    const row = videoRows[virtualRow.index];
                    if (!row) return null;

                    return (
                        <div
                            key={virtualRow.key}
                            data-index={virtualRow.index}
                            ref={virtualizer.measureElement}
                            className="absolute left-0 top-0 grid w-full gap-x-2 sm:gap-x-5"
                            style={{
                                gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
                                // 修正絕對定位的 translateY：因為父元素已經受 offsetTop 影響，
                                // 所以必須扣除 scrollMargin 避免位置往下偏移兩次。
                                transform: `translateY(${virtualRow.start - offsetTop}px)`,
                            }}
                        >
                            {row.map((video) => (
                                <VideoCard
                                    key={video.id}
                                    video={video}
                                    onFavoriteToggled={onFavoriteToggled}
                                    onVideoUpdated={onVideoUpdated}
                                    onVideoRemoved={onVideoRemoved}
                                    onToast={onToast}
                                    onModalStateChange={onModalStateChange}
                                />
                            ))}
                        </div>
                    );
                })}
            </div>
        </div>
    );
}
