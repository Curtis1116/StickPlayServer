import { FormEvent, ReactNode, useEffect, useState } from 'react';
import { useDialogFocus } from '../useDialogFocus';
import { createPortal } from 'react-dom';
import { authRequest, refreshSession } from '../auth';

function AuthDialog({ children }: { children: ReactNode }) {
    const ref = useDialogFocus(() => {});
    return <div ref={ref} role="dialog" aria-modal="true" aria-label="管理員登入" className="fixed inset-0 z-[2000] bg-zinc-950 flex items-center justify-center p-5 overflow-y-auto">{children}</div>;
}

export default function AuthGate({ children }: { children: ReactNode }) {
    const [mode, setMode] = useState<'loading' | 'setup' | 'login' | 'recover' | 'ready'>('loading');
    const [mounted, setMounted] = useState(false);
    const [username, setUsername] = useState('admin');
    const [password, setPassword] = useState('');
    const [confirm, setConfirm] = useState('');
    const [code, setCode] = useState('');
    const [remember, setRemember] = useState(true);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState('');
    const initialize = async () => {
        setError(''); setMode('loading');
        try {
            const status = await authRequest<{ setupRequired: boolean }>('status');
            if (status.setupRequired) { setMode('setup'); return; }
            try { await refreshSession(); setMounted(true); setMode('ready'); }
            catch { setMode('login'); }
        } catch (e) { setError(String(e)); }
    };
    useEffect(() => {
        void initialize();
        const expire = () => { setMode('login'); setError('登入已失效。重新登入後可繼續編輯，尚未送出的內容會保留。'); };
        window.addEventListener('stickplay-auth-expired', expire);
        return () => window.removeEventListener('stickplay-auth-expired', expire);
    }, []);
    useEffect(() => {
        if (mode !== 'ready') return;
        const check = async () => { try { await refreshSession(); } catch { setMode('login'); } };
        const timer = window.setInterval(check, 60000);
        return () => window.clearInterval(timer);
    }, [mode]);
    const submit = async (e: FormEvent) => {
        e.preventDefault(); setError('');
        if (mode !== 'login' && password !== confirm) { setError('兩次輸入的密碼不一致'); return; }
        setBusy(true);
        try {
            await authRequest(mode, { username, password, code, remember });
            await refreshSession(); setPassword(''); setConfirm(''); setCode(''); setMounted(true); setMode('ready');
            window.dispatchEvent(new Event('stickplay-auth-restored'));
        } catch (e) { setError(String(e)); } finally { setBusy(false); }
    };
    return <>
        {mounted && <div inert={mode !== 'ready'} aria-hidden={mode !== 'ready'}>{children}</div>}
        {mode !== 'ready' && createPortal(<AuthDialog key={mode}>
            <form onSubmit={submit} className="w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-7 space-y-5" aria-label="StickPlay 身份驗證">
                <div><p className="text-indigo-400 text-sm mb-2">StickPlay</p><h1 className="text-2xl font-bold">{mode === 'setup' ? '首次設定' : mode === 'recover' ? '復原管理員帳號' : mode === 'loading' ? '正在連線…' : '登入媒體庫'}</h1></div>
                {mode !== 'loading' && <>
                    {mode !== 'login' && <><p className="text-sm text-zinc-400">{mode === 'setup' ? '請從 NAS 容器日誌取得一次性設定碼，建立管理員後會直接登入。' : '請由 NAS 管理員產生一次性復原碼。完成後，所有舊裝置會登出，媒體資料保留。'}</p><label className="block">一次性設定碼<input required autoComplete="off" value={code} onChange={e => setCode(e.target.value)} className="auth-input" /></label></>}
                    <label className="block">管理員帳號<input required maxLength={64} autoComplete="username" value={username} onChange={e => setUsername(e.target.value)} className="auth-input" /></label>
                    <label className="block">{mode === 'login' ? '密碼' : '設定密碼（至少 12 個字元）'}<input type="password" required minLength={mode === 'login' ? 1 : 12} maxLength={256} autoComplete={mode === 'login' ? 'current-password' : 'new-password'} value={password} onChange={e => setPassword(e.target.value)} className="auth-input" /></label>
                    {mode !== 'login' && <label className="block">確認密碼<input type="password" required autoComplete="new-password" value={confirm} onChange={e => setConfirm(e.target.value)} className="auth-input" /></label>}
                    <label className="flex gap-2 items-start text-sm"><input type="checkbox" checked={remember} onChange={e => setRemember(e.target.checked)} /><span>記住這台裝置<span className="block text-zinc-400 mt-1">共用電腦請取消勾選。90 天未使用才需重新登入。</span></span></label>
                </>}
                {error && <p role="alert" className="text-red-400 text-sm">{error}</p>}
                {mode === 'loading' ? error && <button type="button" onClick={initialize} className="auth-button">重試</button> : <button disabled={busy} className="auth-button w-full">{busy ? '處理中…' : mode === 'setup' ? '建立並開始使用' : mode === 'recover' ? '重設並登入' : '登入'}</button>}
                {mode === 'login' && <button type="button" className="text-sm text-zinc-400" onClick={() => { setError(''); setMode('recover'); }}>忘記密碼？</button>}
                {mode === 'recover' && <button type="button" className="text-sm text-zinc-400" onClick={() => setMode('login')}>返回登入</button>}
            </form>
        </AuthDialog>, document.body)}
    </>;
}
