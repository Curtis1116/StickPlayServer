import { useEffect, useRef, useState } from "react";

interface ToastProps {
    message: string;
    onDone: () => void;
}

export default function Toast({ message, onDone }: ToastProps) {
    const [visible, setVisible] = useState(false);
    const timerRef = useRef<ReturnType<typeof setTimeout>>(null);

    useEffect(() => {
        // 進入動畫
        requestAnimationFrame(() => setVisible(true));

        timerRef.current = setTimeout(() => {
            setVisible(false);
            setTimeout(onDone, 300);
        }, 2500);

        return () => {
            if (timerRef.current) clearTimeout(timerRef.current);
        };
    }, [onDone]);

    return (
        <div
            className={`fixed bottom-10 left-1/2 z-[200] px-6 py-3 bg-zinc-100 text-zinc-900 rounded-2xl font-bold shadow-2xl pointer-events-none transition-all duration-300 whitespace-pre-line text-sm ${visible
                ? "opacity-100 -translate-x-1/2 translate-y-0"
                : "opacity-0 -translate-x-1/2 translate-y-3"
                }`}
        >
            {message}
        </div>
    );
}
