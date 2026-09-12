export type PlayerChoice = "browser" | "potplayer" | "vlc" | "infuse" | "justplayer";
export type DevicePlatform = "windows" | "macos" | "ios" | "android" | "other";

const PLAYER_STORE_KEY = "stickplay_player";

// iPadOS 13+ 會使用 Macintosh UA，因此要在 macOS 之前搭配觸控點數判斷。
export function detectDevicePlatform(
    userAgent: string = navigator.userAgent,
    platform: string = navigator.platform,
    maxTouchPoints: number = navigator.maxTouchPoints,
): DevicePlatform {
    if (/Android/i.test(userAgent)) return "android";
    if (/iPhone|iPad|iPod/i.test(userAgent) || (platform === "MacIntel" && maxTouchPoints > 1)) {
        return "ios";
    }
    if (/Windows NT/i.test(userAgent)) return "windows";
    if (/Macintosh|Mac OS X/i.test(userAgent) || platform === "MacIntel") return "macos";
    return "other";
}

export function getAvailablePlayersForPlatform(platform: DevicePlatform): PlayerChoice[] {
    switch (platform) {
        case "windows": return ["browser", "potplayer", "vlc"];
        case "macos": return ["browser", "vlc", "infuse"];
        case "ios": return ["browser", "infuse", "vlc"];
        case "android": return ["browser", "vlc", "justplayer"];
        default: return ["browser"];
    }
}

export function getAvailablePlayers(): PlayerChoice[] {
    return getAvailablePlayersForPlatform(detectDevicePlatform());
}

export function getPlayerPreference(): PlayerChoice {
    const stored = localStorage.getItem(PLAYER_STORE_KEY) as PlayerChoice | null;
    const available = getAvailablePlayers();
    return stored && available.includes(stored) ? stored : "browser";
}

export function setPlayerPreference(player: PlayerChoice): void {
    localStorage.setItem(PLAYER_STORE_KEY, player);
}

export function androidIntentUrl(url: string, packageName: string): string {
    const parsed = new URL(url);
    const scheme = parsed.protocol.slice(0, -1);
    const target = `${parsed.host}${parsed.pathname}${parsed.search}`;
    return `intent://${target}#Intent;scheme=${scheme};package=${packageName};type=video/*;S.browser_fallback_url=${encodeURIComponent(url)};end`;
}
