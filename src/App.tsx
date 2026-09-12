import { useCallback, useEffect, useState, useRef } from "react";

import Header from "./components/Header";
import VideoGrid from "./components/VideoGrid";
import SettingsPage from "./components/SettingsPage";
import Toast from "./components/Toast";
import DesktopSidebar from "./components/DesktopSidebar";
import MobileNav from "./components/MobileNav";
import { VideoEntry, VideoFilter } from "./types";
import {
  selectedLibrary,
  clearLibrary,
  getAllGenres,
  getAllLevels,
  getLibraries,
  saveLibraries,
  getStats,
  queryVideos,
  scanLibrary,
  switchDatabase,
  syncWatchPaths,
  subscribeToEvents,
} from "./api";
import { Library } from "./types";

type Page = "main" | "settings";

export default function App() {
  const querySequence = useRef(0);
  const [authEpoch, setAuthEpoch] = useState(0);
  useEffect(() => { const restore = () => setAuthEpoch(v => v + 1); window.addEventListener("stickplay-auth-restored", restore); return () => window.removeEventListener("stickplay-auth-restored", restore); }, []);
  const [page, setPage] = useState<Page>("main");
  const [videos, setVideos] = useState<VideoEntry[]>([]);
  const [genres, setGenres] = useState<string[]>([]);
  const [levels, setLevels] = useState<string[]>([]);
  const [filter, setFilter] = useState<VideoFilter>({
    sort_by: "date_added",
    sort_order: "DESC",
  });
  const [totalCount, setTotalCount] = useState(0);
  const [favoriteCount, setFavoriteCount] = useState(0);
  const [isScanning, setIsScanning] = useState(false);
  const [toastMessage, setToastMessage] = useState<string | null>(null);
  const [, setIsModalOpen] = useState(false);

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
      const libs: Library[] = await getLibraries();
      setLibraries(libs || []);

      if (libs.length > 0) {
        let active = getStore<string>("active_library_id");
        if (!active || !libs.find(l => l.id === active)) {
          active = libs[0].id;
        }


        const activeLib = libs.find(l => l.id === active)!;
        await switchDatabase(activeLib.db_name);
        setActiveLibraryId(active || "");

        // 載入上次的排序選項
        const savedSortBy = getStore<string>(`${active}_last_sort_by`);
        const savedSortOrder = getStore<string>(`${active}_last_sort_order`);
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
      setToastMessage(`初始化載入失敗：${e}`);
    }
  }, []);

  // 載入影片列表
  const loadVideos = useCallback(async () => {
    if (!activeLibraryId || selectedLibrary() !== activeLibraryId) return;
    const sequence = ++querySequence.current;
    try {
      const list = await queryVideos(filter);
      if (sequence === querySequence.current && selectedLibrary() === activeLibraryId) setVideos(list);
    } catch (e) {
      console.error("查詢失敗:", e);
    }
  }, [filter, activeLibraryId]);

  // 載入篩選選項及統計
  const loadMeta = useCallback(async () => {
    const id = selectedLibrary();
    if (!id) return;
    try {
      const [g, l, stats] = await Promise.all([
        getAllGenres(),
        getAllLevels(),
        getStats(),
      ]);
      if (id !== selectedLibrary()) return;
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

  // 訂閱 SSE 即時事件：偵測到媒體庫變化時自動重新載入
  useEffect(() => {
    const cleanup = subscribeToEvents(() => {
      loadVideos();
      loadMeta();
    });
    return cleanup;
  }, [loadVideos, loadMeta, authEpoch]);

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
  const handleFavoriteToggled = useCallback((id: string, newState: boolean) => {
    setVideos((prev) =>
      prev.map((v) =>
        v.id === id ? { ...v, is_favorite: newState } : v
      )
    );
    setFavoriteCount((prev) => prev + (newState ? 1 : -1));
  }, []);


  // 影片重新索引後更新本地狀態
  const handleVideoUpdated = useCallback((updated: VideoEntry) => {
    setVideos((prev) => prev.map((v) => (v.id === updated.id || v.folder_path === updated.folder_path ? updated : v)));
  }, []);

  // 切換媒體庫
  const handleLibraryChange = async (id: string) => {
    const lib = libraries.find(l => l.id === id);
    if (lib) {
      try {
        // 先確保後端切換成功
        ++querySequence.current;
        setVideos([]);
        await switchDatabase(lib.db_name);
        localStorage.setItem("stickplay_active_library_id", JSON.stringify(id));
        // 更新狀態
        setActiveLibraryId(id);

        // 讀取該媒體庫儲存的排序選項
        const savedSortBy = localStorage.getItem(`stickplay_${id}_last_sort_by`);
        const savedSortOrder = localStorage.getItem(`stickplay_${id}_last_sort_order`);

        setFilter({
          sort_by: savedSortBy ? JSON.parse(savedSortBy) : "date_added",
          sort_order: savedSortOrder ? JSON.parse(savedSortOrder) : "DESC",
          search: undefined,
          genres: undefined,
          levels: undefined,
          favorites_only: undefined,
        });

        // 顯式重新載入 Meta 資料，filter 改變或 activeLibraryId 改變會自動觸發 loadVideos
        await loadMeta();
      } catch (e) {
        showToast(`切換媒體庫失敗: ${e}`);
      }
    }
  };

  const handleVideoRemoved = useCallback((id: string) => {
    setVideos((prev) => prev.filter((v) => v.id !== id));
    setTotalCount((prev) => Math.max(0, prev - 1));
  }, []);

  // 篩選條件改變
  const handleFilterChange = async (newFilter: VideoFilter) => {
    setFilter(newFilter);

    // 如果排序選項改變，儲存到設定
    if (newFilter.sort_by !== filter.sort_by || newFilter.sort_order !== filter.sort_order) {
      try {
        if (newFilter.sort_by) localStorage.setItem(`stickplay_${activeLibraryId}_last_sort_by`, JSON.stringify(newFilter.sort_by));
        if (newFilter.sort_order) localStorage.setItem(`stickplay_${activeLibraryId}_last_sort_order`, JSON.stringify(newFilter.sort_order));
      } catch (e) {
        console.error("儲存排序偏好失敗:", e);
      }
    }
  };

  // 媒體庫路徑更新
  const handleLibrariesChanged = async (newLibs: Library[], persisted = false) => {
    if (!persisted) await saveLibraries(newLibs);
    setLibraries(newLibs);
    if (!newLibs.find(l => l.id === activeLibraryId)) {
      ++querySequence.current;
      setVideos([]);
      setTotalCount(0);
      setFavoriteCount(0);
      setGenres([]); setLevels([]);
      const first = newLibs[0];
      if (first) {
        await switchDatabase(first.db_name);
        setActiveLibraryId(first.id);
        localStorage.setItem("stickplay_active_library_id", JSON.stringify(first.id));
        await loadMeta();
      } else { clearLibrary(); setActiveLibraryId(""); }
    }
  };

  // Toast 通知
  const showToast = useCallback((msg: string) => {
    setToastMessage(msg);
  }, []);

  return (
    <div className="flex min-h-[100dvh] bg-[#0d0e12]">
      <DesktopSidebar
        page={page}
        libraries={libraries}
        activeLibraryId={activeLibraryId}
        onLibraryChange={handleLibraryChange}
        genres={genres}
        filter={filter}
        totalCount={totalCount}
        favoriteCount={favoriteCount}
        onFilterChange={handleFilterChange}
        onOpenMain={() => setPage("main")}
        onOpenSettings={() => setPage("settings")}
        isScanning={isScanning}
      />

      <div className="min-w-0 flex-1">
        {page === "settings" ? (
          <SettingsPage
            libraries={libraries}
            activeLibraryId={activeLibraryId}
            onBack={() => setPage("main")}
            onLibrariesChanged={handleLibrariesChanged}
          />
        ) : (
          <>
            <Header
              libraries={libraries}
              activeLibraryId={activeLibraryId}
              onLibraryChange={handleLibraryChange}
              genres={genres}
              levels={levels}
              filter={filter}
              totalCount={totalCount}
              onFilterChange={handleFilterChange}
              onRefresh={handleScan}
              onOpenSettings={() => setPage("settings")}
              isScanning={isScanning}
            />
            <main className="mx-auto w-full max-w-[1600px] px-3 pb-24 pt-3 sm:px-6 sm:pt-6 lg:px-8 lg:pb-10">
              <VideoGrid
                videos={videos}
                onFavoriteToggled={handleFavoriteToggled}
                onVideoUpdated={handleVideoUpdated}
                onVideoRemoved={handleVideoRemoved}
                onToast={showToast}
                onModalStateChange={setIsModalOpen}
              />
            </main>
          </>
        )}
      </div>

      <MobileNav
        active={page === "settings" ? "settings" : filter.favorites_only ? "favorites" : "videos"}
        onVideos={() => {
          setPage("main");
          void handleFilterChange({ ...filter, search: undefined, favorites_only: undefined, genres: undefined, levels: undefined });
        }}
        onFavorites={() => {
          setPage("main");
          void handleFilterChange({ ...filter, search: undefined, favorites_only: true, genres: undefined, levels: undefined });
        }}
        onSettings={() => setPage("settings")}
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
