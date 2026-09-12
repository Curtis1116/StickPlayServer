import { FormEvent, useEffect, useState } from "react";
import {
    ChevronRight,
    KeyRound,
    Laptop,
    LogOut,
    MonitorSmartphone,
    ShieldCheck,
    Smartphone,
} from "lucide-react";
import { authRequest, Device, expired, refreshSession } from "../auth";

export default function AccountSettings() {
    const [devices, setDevices] = useState<Device[]>([]);
    const [error, setError] = useState("");
    const [notice, setNotice] = useState("");
    const [busy, setBusy] = useState(false);
    const [devicesOpen, setDevicesOpen] = useState(false);
    const [passwordOpen, setPasswordOpen] = useState(false);
    const [currentPassword, setCurrentPassword] = useState("");
    const [newPassword, setNewPassword] = useState("");
    const [confirmPassword, setConfirmPassword] = useState("");

    const loadDevices = async () => setDevices(await authRequest<Device[]>("devices"));

    useEffect(() => {
        void loadDevices().catch((reason) => setError(String(reason)));
    }, []);

    const action = async (run: () => Promise<void>) => {
        setBusy(true);
        setError("");
        setNotice("");
        try {
            await run();
        } catch (reason) {
            setError(String(reason));
        } finally {
            setBusy(false);
        }
    };

    const changePassword = (event: FormEvent) => {
        event.preventDefault();
        void action(async () => {
            if (newPassword !== confirmPassword) throw new Error("兩次輸入的密碼不一致");
            await authRequest("password", { currentPassword, newPassword });
            await refreshSession();
            setCurrentPassword("");
            setNewPassword("");
            setConfirmPassword("");
            setPasswordOpen(false);
            setNotice("密碼已更新，其他裝置已登出。");
            await loadDevices();
        });
    };

    return (
        <section className="rounded-xl border border-zinc-800 bg-zinc-900/35 p-4 lg:p-5">
            <div className="flex items-center justify-between gap-3">
                <div className="flex items-center gap-2">
                    <ShieldCheck size={21} className="text-indigo-400" />
                    <h2 className="text-lg font-bold text-zinc-100">帳號與登入裝置</h2>
                </div>
                <button type="button" onClick={() => setPasswordOpen((value) => !value)} className="hidden min-h-11 items-center gap-2 rounded-lg border border-zinc-700 px-3 text-sm text-zinc-300 hover:border-zinc-600 lg:flex">
                    <KeyRound size={16} />變更密碼
                </button>
            </div>

            {error && <p role="alert" className="mt-4 rounded-lg bg-red-500/10 p-3 text-sm text-red-300">{error}</p>}
            {notice && <p role="status" className="mt-4 rounded-lg bg-emerald-500/10 p-3 text-sm text-emerald-300">{notice}</p>}

            <div className="mt-4 overflow-hidden rounded-xl border border-zinc-800 lg:hidden">
                <button type="button" onClick={() => setDevicesOpen((value) => !value)} aria-expanded={devicesOpen} className="flex min-h-14 w-full items-center gap-3 px-3 text-left text-sm text-zinc-200">
                    <MonitorSmartphone size={19} className="text-zinc-500" />
                    <span className="flex-1">已登入裝置</span>
                    <span className="text-xs text-zinc-600">{devices.length}</span>
                    <ChevronRight size={17} className={`text-zinc-500 transition-transform ${devicesOpen ? "rotate-90" : ""}`} />
                </button>
                <button type="button" onClick={() => setPasswordOpen((value) => !value)} aria-expanded={passwordOpen} className="flex min-h-14 w-full items-center gap-3 border-t border-zinc-800 px-3 text-left text-sm text-zinc-200">
                    <KeyRound size={19} className="text-zinc-500" />
                    <span className="flex-1">變更密碼</span>
                    <ChevronRight size={17} className={`text-zinc-500 transition-transform ${passwordOpen ? "rotate-90" : ""}`} />
                </button>
            </div>

            <div className={`${devicesOpen ? "block" : "hidden"} mt-4 overflow-hidden rounded-xl border border-zinc-800 lg:block`}>
                {devices.length === 0 ? (
                    <p className="p-4 text-sm text-zinc-600">尚無登入裝置資料</p>
                ) : devices.map((device, index) => (
                    <div key={device.id} className={`flex min-h-16 items-center gap-3 px-3 sm:px-4 ${index ? "border-t border-zinc-800" : ""}`}>
                        {/iphone|ipad|android|mobile/i.test(device.device) ? <Smartphone size={19} className="shrink-0 text-zinc-500" /> : <Laptop size={19} className="shrink-0 text-zinc-500" />}
                        <div className="min-w-0 flex-1">
                            <p className="truncate text-sm text-zinc-200">{device.device}</p>
                            <p className="mt-0.5 text-xs text-zinc-600">最近使用：{new Date(device.lastUsed * 1000).toLocaleString()}</p>
                        </div>
                        {device.current ? (
                            <span className="shrink-0 rounded-md bg-indigo-500/15 px-2 py-1 text-xs text-indigo-300">目前裝置</span>
                        ) : (
                            <button type="button" disabled={busy} onClick={() => void action(async () => { await authRequest("revoke", { id: device.id }); await loadDevices(); })} className="min-h-10 shrink-0 rounded-lg px-2 text-sm text-red-400 hover:bg-red-500/10 disabled:opacity-50">登出</button>
                        )}
                    </div>
                ))}
            </div>

            {passwordOpen && (
                <form onSubmit={changePassword} className="mt-4 grid gap-3 rounded-xl border border-zinc-800 bg-zinc-950/30 p-4 md:grid-cols-3">
                    <label className="text-sm text-zinc-300">目前密碼<input type="password" autoComplete="current-password" required value={currentPassword} onChange={(event) => setCurrentPassword(event.target.value)} className="auth-input" /></label>
                    <label className="text-sm text-zinc-300">新密碼<input type="password" autoComplete="new-password" required minLength={12} maxLength={256} value={newPassword} onChange={(event) => setNewPassword(event.target.value)} className="auth-input" /></label>
                    <label className="text-sm text-zinc-300">確認新密碼<input type="password" autoComplete="new-password" required value={confirmPassword} onChange={(event) => setConfirmPassword(event.target.value)} className="auth-input" /></label>
                    <div className="flex gap-2 md:col-span-3 md:justify-end">
                        <button type="button" onClick={() => setPasswordOpen(false)} className="min-h-11 rounded-lg px-4 text-sm text-zinc-400">取消</button>
                        <button type="submit" disabled={busy} className="auth-button min-h-11">更新密碼並登出其他裝置</button>
                    </div>
                </form>
            )}

            <div className="mt-4 flex flex-col gap-2 border-t border-zinc-800 pt-4 sm:flex-row sm:items-center sm:justify-between">
                <button type="button" disabled={busy} onClick={() => void action(async () => { await authRequest("revoke", {}); await loadDevices(); })} className="min-h-11 rounded-lg px-3 text-left text-sm text-red-400 hover:bg-red-500/10 disabled:opacity-50">登出其他所有裝置</button>
                <button type="button" disabled={busy} onClick={() => void action(async () => { await authRequest("logout", {}); expired(); })} className="flex min-h-11 items-center gap-2 rounded-lg px-3 text-sm text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100 disabled:opacity-50"><LogOut size={16} />登出目前裝置</button>
            </div>
        </section>
    );
}
