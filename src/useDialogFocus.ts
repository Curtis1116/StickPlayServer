import { useEffect, useRef } from 'react';
export function useDialogFocus(onClose: () => void) {
    const ref = useRef<HTMLDivElement>(null);
    const close = useRef(onClose); close.current = onClose;
    useEffect(() => {
        const previous = document.activeElement as HTMLElement | null;
        const node = ref.current;
        if (!node) return;
        const focusable = () => Array.from(node.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex="0"]')).filter(e => e.offsetParent !== null);
        focusable()[0]?.focus();
        const keydown = (event: KeyboardEvent) => {
            if (event.key === 'Escape') { event.preventDefault(); close.current(); }
            if (event.key !== 'Tab') return;
            const elements = focusable(); const first = elements[0]; const last = elements[elements.length - 1];
            if (!first) return;
            if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
            else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
        };
        node.addEventListener('keydown', keydown);
        return () => { node.removeEventListener('keydown', keydown); previous?.focus(); };
    }, []);
    return ref;
}
