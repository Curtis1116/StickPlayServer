import { ReactNode, useEffect } from "react";
import { createPortal } from "react-dom";
import { X } from "lucide-react";
import { useDialogFocus } from "../useDialogFocus";

export default function CatalogDialog({ title, onClose, children }: { title: string; onClose: () => void; children: ReactNode }) {
    const ref = useDialogFocus(onClose);
    useEffect(() => {
        const previous = document.body.style.overflow;
        document.body.style.overflow = "hidden";
        return () => { document.body.style.overflow = previous; };
    }, []);
    return createPortal(
        <div ref={ref} role="dialog" aria-modal="true" aria-label={title} className="fixed inset-0 z-[100] flex items-end justify-center bg-black/65 lg:items-center lg:p-4" onPointerDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
            <section className="max-h-[86dvh] w-full overflow-y-auto rounded-t-2xl border border-zinc-700 bg-[#15161b] px-4 pb-[calc(1rem+env(safe-area-inset-bottom))] shadow-2xl lg:max-w-md lg:rounded-2xl">
                <header className="sticky top-0 z-10 mb-3 flex items-center justify-between border-b border-zinc-800 bg-[#15161b] py-2">
                    <h2 className="text-lg font-bold">{title}</h2>
                    <button type="button" onClick={onClose} aria-label={`關閉${title}`} className="flex h-11 w-11 items-center justify-center rounded-lg text-zinc-400 hover:bg-zinc-800"><X size={20} /></button>
                </header>
                {children}
            </section>
        </div>, document.body,
    );
}
