export interface Session { id: string; username: string; csrf: string; expiresAt: number; remember: boolean }
export interface Device { id: string; device: string; current: boolean; lastUsed: number; created: number }
let csrf = '';
export function csrfToken() { return csrf; }
export function expired() { csrf = ''; window.dispatchEvent(new Event('stickplay-auth-expired')); }
export async function authRequest<T>(endpoint: string, payload?: unknown): Promise<T> {
    const response = await fetch(`/api/auth/${endpoint}`, {
        method: payload === undefined ? 'GET' : 'POST', credentials: 'same-origin',
        headers: payload === undefined ? {} : { 'Content-Type': 'application/json', 'X-CSRF-Token': csrf },
        body: payload === undefined ? undefined : JSON.stringify(payload),
    });
    if (!response.ok) throw new Error(await response.text() || '連線失敗');
    return response.json();
}
export async function refreshSession() {
    const session = await authRequest<Session>('me'); csrf = session.csrf; return session;
}
