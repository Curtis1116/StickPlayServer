import { useCallback, useEffect, useState } from "react";

import Header from "./components/Header";
import VideoGrid from "./components/VideoGrid";
import SettingsPage from "./components/SettingsPage";
import Footer from "./components/Footer";
import Toast from "./components/Toast";
import { VideoEntry, VideoFilter } from "./types";
import {
  getAllGenres,
  getAllLevels,
  getLibraries,
  saveLibraries,
  getStats,
  queryVideos,
  scanLibrary,
  switchDatabase,
  syncWatchPaths,
} from "./api";
import { Library } from "./types";

type Page = "main" | "settings";

export default function App() {
  const [page, setPage] = useState<Page>("main");
  const [videos, setVideos] = useState<VideoEntry[]>([]);
  const [genres, setGenres] = useState<string[]>([]);
  const [levels, setLevels] = useState<string[]>([]);
  const [filter, setFilter] = useState<VideoFilter>({
    sort_by: "date_added",
    sort_order: "DESC",
  });
  const [gridSize, setGridSize] = useState(180);
  const [totalCount, setTotalCount] = useState(0);
  const [favoriteCount, setFavoriteCount] = useState(0);
  const [isScanning, setIsScanning] = useState(false);
  const [toastMessage, setToastMessage] = useState<string | null>(null);

  const [libraries, setLibraries] = useState<Library[]>([]);
  const [activeLibraryId, setActiveLibraryId] = useState<string>("");

  // 載入設定、初始化資料庫連線並載入內容
  const loadSettingsAndData = useCallback(async () => {
    try {
      const getStore = <T,>(key: string): T | null => {
        const val = localStorage.getItem(`stickplay_${key}`);
        return val ? JSON.parse(val) : null;
      };

      // 優先從伺服器載入媒體庫設定
      let libs: any[] = await getLibraries();
      if (!libs || libs.length === 0) {
        // 回退到 localStorage (用於遷移或初次設定)
        const localLibs = getStore<Library[]>("libraries");
        if (!localLibs) {
          // 遷移更舊的 library_paths
          const oldPaths = getStore<string[]>("library_paths");
          if (oldPaths && oldPaths.length > 0) {
            libs = [{
              id: "default",
              name: "預設媒體庫",
              paths: oldPaths,
              db_name: "stickplay"
            }];
            await saveLibraries(libs);
          } else {
            libs = [];
          }
        } else {
          // 將本地設定同步到伺服器
          libs = localLibs;
          await saveLibraries(libs);
        }
      }
      setLibraries(libs || []);

      if (libs.length > 0) {
        let active = getStore<string>("active_library_id");
        if (!active || !libs.find(l => l.id === active)) {
          active = libs[0].id;
        }
        setActiveLibraryId(active || "");

        const activeLib = libs.find(l => l.id === active)!;
        await switchDatabase(activeLib.db_name);

        // 載入上次的排序選項
        const savedSortBy = getStore<string>("last_sort_by");
        const savedSortOrder = getStore<string>("last_sort_order");
        if (savedSortBy || savedSortOrder) {
          setFilter(prev => ({
            ...prev,
            sort_by: savedSortBy || prev.sort_by,
            sort_order: savedSortOrder || prev.sort_order
          }));
        }

        // 必須等待 filter state 更新後，useEffect() 內的 loadVideos 才會自動觸發
        // 但由於一啟動就需要呈現，可以直接調用
        await loadMeta();
        // loadVideos() will be triggered by useEffect due to initial mount and filter change
      }
    } catch (e) {
      console.error("初始化載入失敗:", e);
    }
  }, []);

  // 載入影片列表
  const loadVideos = useCallback(async () => {
    if (!activeLibraryId) return;
    try {
      const list = await queryVideos(filter);
      setVideos(list);
    } catch (e) {
      console.error("查詢失敗:", e);
    }
  }, [filter, activeLibraryId]);

  // 載入篩選選項及統計
  const loadMeta = useCallback(async () => {
    try {
      const [g, l, stats] = await Promise.all([
        getAllGenres(),
        getAllLevels(),
        getStats(),
      ]);
      setGenres(g);
      setLevels(l);
      setTotalCount(stats[0]);
      setFavoriteCount(stats[1]);
    } catch (e) {
      console.error("載入元資料失敗:", e);
    }
  }, []);

  // 篩選條件改變時重新查詢
  useEffect(() => {
    loadVideos();
  }, [loadVideos]);

  // 同步監控路徑到後端
  useEffect(() => {
    if (activeLibraryId && libraries.length > 0) {
      const lib = libraries.find(l => l.id === activeLibraryId);
      if (lib) {
        syncWatchPaths(lib.paths).catch(console.error);
      }
    }
  }, [activeLibraryId, libraries]);

  // 初始載入
  useEffect(() => {
    loadSettingsAndData();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 掃描媒體庫
  const handleScan = async () => {
    setIsScanning(true);
    try {
      if (!activeLibraryId) {
        showToast("請先在設定頁面新增媒體庫");
        setPage("settings");
        return;
      }
      const lib = libraries.find(l => l.id === activeLibraryId);
      if (!lib || lib.paths.length === 0) {
        showToast("目前的媒體庫尚未設定任何路徑");
        setPage("settings");
        return;
      }
      const count = await scanLibrary(lib.paths);
      showToast(`掃描完成，共索引 ${count} 部影片`);
      await loadMeta();
      await loadVideos();
    } catch (e) {
      showToast(`掃描失敗: ${e}`);
    } finally {
      setIsScanning(false);
    }
  };

  // 最愛切換後更新本地狀態
  const handleFavoriteToggled = (id: string, newState: boolean) => {
    setVideos((prev) =>
      prev.map((v) =>
        v.id === id ? { ...v, is_favorite: newState } : v
      )
    );
    setFavoriteCount((prev) => prev + (newState ? 1 : -1));
  };

  // 評分更新後更新本地狀態（含新 nfos_path）
  const handleRatingUpdated = (id: string, newRating: number, nfosPath: string) => {
    setVideos((prev) =>
      prev.map((v) =>
        v.id === id ? { ...v, rating: newRating, nfos_path: nfosPath || v.nfos_path } : v
      )
    );
  };

  // 影片重新索引後更新本地狀態
  const handleVideoUpdated = (updated: VideoEntry) => {
    setVideos((prev) => prev.map((v) => (v.id === updated.id ? updated : v)));
  };

  // 切換媒體庫
  const handleLibraryChange = async (id: string) => {
    setActiveLibraryId(id);
    const lib = libraries.find(l => l.id === id);
    if (lib) {
      try {
        await switchDatabase(lib.db_name);
        localStorage.setItem("stickplay_active_library_id", JSON.stringify(id));

        await loadMeta();
        // filter 改變或 activeLibraryId 改變會自動觸發 loadVideos
      } catch (e) {
        showToast(`切換媒體庫失敗: ${e}`);
      }
    }
  };

  const handleVideoRemoved = (id: string) => {
    setVideos((prev) => prev.filter((v) => v.id !== id));
    setTotalCount((prev) => Math.max(0, prev - 1));
  };

  // 篩選條件改變
  const handleFilterChange = async (newFilter: VideoFilter) => {
    setFilter(newFilter);

    // 如果排序選項改變，儲存到設定
    if (newFilter.sort_by !== filter.sort_by || newFilter.sort_order !== filter.sort_order) {
      try {
        if (newFilter.sort_by) localStorage.setItem("stickplay_last_sort_by", JSON.stringify(newFilter.sort_by));
        if (newFilter.sort_order) localStorage.setItem("stickplay_last_sort_order", JSON.stringify(newFilter.sort_order));
      } catch (e) {
        console.error("儲存排序偏好失敗:", e);
      }
    }
  };

  // 媒體庫路徑更新
  const handleLibrariesChanged = async (newLibs: Library[]) => {
    setLibraries(newLibs);
    try {
      await saveLibraries(newLibs);
    } catch (e) {
      console.error("儲存設定到伺服器失敗:", e);
    }
    
    // 如果目前選擇的被刪轉了，切回到第一個
    if (!newLibs.find(l => l.id === activeLibraryId)) {
      if (newLibs.length > 0) {
        handleLibraryChange(newLibs[0].id);
      } else {
        setActiveLibraryId("");
        setVideos([]);
        setTotalCount(0);
      }
    }
  };

  // Toast 通知
  const showToast = (msg: string) => {
    setToastMessage(msg);
  };

  if (page === "settings") {
    return (
      <div className="min-h-screen flex flex-col">
        <SettingsPage
          libraries={libraries}
          activeLibraryId={activeLibraryId}
          onLibraryChange={handleLibraryChange}
          onBack={() => setPage("main")}
          onLibrariesChanged={handleLibrariesChanged}
        />

        {toastMessage && (
          <Toast
            key={toastMessage}
            message={toastMessage}
            onDone={() => setToastMessage(null)}
          />
        )}
      </div>
    );
  }

  return (
    <div className="min-h-screen flex flex-col">
      <Header
        libraries={libraries}
        activeLibraryId={activeLibraryId}
        onLibraryChange={handleLibraryChange}
        genres={genres}
        levels={levels}
        filter={filter}
        gridSize={gridSize}
        totalCount={totalCount}
        currentCount={videos.length}
        onFilterChange={handleFilterChange}
        onGridSizeChange={setGridSize}
        onRefresh={handleScan}
        onOpenSettings={() => setPage("settings")}
        isScanning={isScanning}
      />

      <main className="flex-grow p-8">
        <VideoGrid
          videos={videos}
          gridSize={gridSize}
          onFavoriteToggled={handleFavoriteToggled}
          onRatingUpdated={handleRatingUpdated}
          onVideoUpdated={handleVideoUpdated}
          onVideoRemoved={handleVideoRemoved}
          onToast={showToast}
        />
      </main>

      <Footer totalCount={totalCount} favoriteCount={favoriteCount} />

      {toastMessage && (
        <Toast
          key={toastMessage}
          message={toastMessage}
          onDone={() => setToastMessage(null)}
        />
      )}
    </div>
  );
}
