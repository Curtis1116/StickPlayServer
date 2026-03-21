import { useState, useRef, useEffect, SyntheticEvent } from "react";
import { createPortal } from "react-dom";
import { X, Check, Search, ChevronLeft, ChevronRight } from "lucide-react";
import ReactCrop, { Crop, PixelCrop, centerCrop, makeAspectCrop } from 'react-image-crop';
import 'react-image-crop/dist/ReactCrop.css';
import { getFolderImages, cropAndSavePoster, readImage } from "../api";

interface ManualCropModalProps {
    folderPath: string;
    videoId: string;
    onClose: () => void;
    onSaved: (newPosterPath: string) => void;
    onToast: (msg: string) => void;
}

export default function ManualCropModal({
    folderPath,
    videoId,
    onClose,
    onSaved,
    onToast,
}: ManualCropModalProps) {
    const [images, setImages] = useState<string[]>([]);
    const [selectedIndex, setSelectedIndex] = useState(0);
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState(false);
    const [crop, setCrop] = useState<Crop>();
    const [completedCrop, setCompletedCrop] = useState<PixelCrop>();
    const imgRef = useRef<HTMLImageElement>(null);
    const [imgUrl, setImgUrl] = useState<string>("");

    useEffect(() => {
        loadImages();
    }, [folderPath]);

    const loadImages = async () => {
        setLoading(true);
        try {
            const list = await getFolderImages(folderPath);
            setImages(list);
            if (list.length > 0) {
                setSelectedIndex(0);
            }
        } catch (e) {
            onToast("讀取圖片列表失敗");
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        if (images.length > 0) {
            updateImgUrl(images[selectedIndex]);
        }
    }, [selectedIndex, images]);

    const updateImgUrl = async (path: string) => {
        const url = await readImage(path);
        setImgUrl(url);
    };

    const onImageLoad = (e: SyntheticEvent<HTMLImageElement>) => {
        const { width, height } = e.currentTarget;
        const initialCrop = centerCrop(
            makeAspectCrop(
                { unit: '%', width: 90 },
                2 / 3,
                width,
                height
            ),
            width,
            height
        );
        setCrop(initialCrop);
    };

    const handleSave = async () => {
        if (!completedCrop || !imgRef.current || !images[selectedIndex]) return;

        setSaving(true);
        try {
            const img = imgRef.current;
            const scaleX = img.naturalWidth / img.width;
            const scaleY = img.naturalHeight / img.height;

            const x = Math.round(completedCrop.x * scaleX);
            const y = Math.round(completedCrop.y * scaleY);
            const width = Math.round(completedCrop.width * scaleX);
            const height = Math.round(completedCrop.height * scaleY);

            const result = await cropAndSavePoster(
                images[selectedIndex],
                x,
                y,
                width,
                height,
                folderPath,
                videoId
            );
            onToast("✅ 海報已儲存");
            onSaved(result);
            onClose();
        } catch (e) {
            onToast(`儲存失敗: ${e}`);
        } finally {
            setSaving(false);
        }
    };

    return createPortal(
        <div className="fixed inset-0 z-[1000] flex items-center justify-center p-4">
            <div className="absolute inset-0 bg-black/90 backdrop-blur-md" onClick={onClose} />
            
            <div className="relative w-full max-w-5xl bg-zinc-900 rounded-2xl border border-white/10 shadow-2xl flex flex-col h-[90vh] overflow-hidden animate-in fade-in zoom-in duration-200">
                <div className="p-4 border-b border-white/5 flex justify-between items-center">
                    <h2 className="text-sm sm:text-lg font-bold flex items-center gap-2">
                        <Search size={18} className="text-indigo-400" />
                        手動裁切海報
                    </h2>
                    <button onClick={onClose} className="p-1.5 hover:bg-white/5 rounded-full transition-colors">
                        <X size={18} />
                    </button>
                </div>

                <div className="flex-1 overflow-hidden flex flex-col md:flex-row gap-2 sm:gap-4 p-2 sm:p-4">
                    <div className="w-full md:w-32 flex md:flex-col gap-2 overflow-x-auto md:overflow-y-auto pb-2 md:pb-0 scrollbar-hide shrink-0">
                        {images.map((path, idx) => (
                            <button
                                key={path}
                                onClick={() => setSelectedIndex(idx)}
                                className={`flex-shrink-0 w-16 sm:w-20 md:w-full aspect-[2/3] rounded-lg overflow-hidden border-2 transition-all ${
                                    idx === selectedIndex ? "border-indigo-500 scale-95" : "border-transparent opacity-50 hover:opacity-100"
                                }`}
                            >
                                <img src={readImageSync(path)} alt="" className="w-full h-full object-cover" />
                            </button>
                        ))}
                    </div>

                    <div className="flex-1 flex flex-col min-h-0 bg-black/20 rounded-xl border border-white/5 relative items-center justify-center overflow-auto p-2 sm:p-4">
                        {loading ? (
                            <div className="animate-spin text-zinc-500">
                                <Search size={40} />
                            </div>
                        ) : images.length > 0 ? (
                            <ReactCrop
                                crop={crop}
                                onChange={(c: PixelCrop) => setCrop(c)}
                                onComplete={(c: PixelCrop) => setCompletedCrop(c)}
                                aspect={2 / 3}
                                className="max-h-full"
                            >
                                <img
                                    ref={imgRef}
                                    src={imgUrl}
                                    onLoad={onImageLoad}
                                    alt="Source"
                                    className="max-h-[50vh] sm:max-h-[60vh] object-contain"
                                />
                            </ReactCrop>
                        ) : (
                            <div className="text-zinc-500">資料夾內無圖片</div>
                        )}
                        
                        {images.length > 1 && (
                            <>
                                <button 
                                    onClick={() => setSelectedIndex(prev => (prev - 1 + images.length) % images.length)}
                                    className="absolute left-4 top-1/2 -translate-y-1/2 p-3 bg-black/60 rounded-full hover:bg-black/90 text-white transition-all shadow-xl"
                                >
                                    <ChevronLeft size={24} />
                                </button>
                                <button 
                                    onClick={() => setSelectedIndex(prev => (prev + 1) % images.length)}
                                    className="absolute right-4 top-1/2 -translate-y-1/2 p-3 bg-black/60 rounded-full hover:bg-black/90 text-white transition-all shadow-xl"
                                >
                                    <ChevronRight size={24} />
                                </button>
                            </>
                        )}
                    </div>
                </div>

                <div className="p-3 sm:p-4 border-t border-white/5 flex justify-end gap-2 sm:gap-3 bg-zinc-950/50">
                    <button
                        onClick={onClose}
                        className="px-4 sm:px-6 py-2 rounded-xl text-xs sm:text-sm font-medium hover:bg-white/5 transition-colors"
                    >
                        取消
                    </button>
                    <button
                        onClick={handleSave}
                        disabled={saving || !completedCrop}
                        className="px-6 sm:px-8 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed rounded-xl text-xs sm:text-sm font-bold flex items-center gap-2 shadow-lg shadow-indigo-600/20 transition-all"
                    >
                        {saving ? "儲存中..." : <><Check size={16} /> 確定並儲存</>}
                    </button>
                </div>
            </div>
        </div>,
        document.body
    );
}

function readImageSync(path: string) {
    return `/api/image?path=${encodeURIComponent(path)}`;
}
