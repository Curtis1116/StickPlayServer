import assert from "node:assert/strict";
import {
    androidIntentUrl,
    detectDevicePlatform,
    getAvailablePlayersForPlatform,
} from "../src/player.ts";

assert.equal(detectDevicePlatform("Mozilla/5.0 (Windows NT 10.0; Win64; x64)", "Win32", 0), "windows");
assert.equal(detectDevicePlatform("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)", "MacIntel", 0), "macos");
assert.equal(detectDevicePlatform("Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)", "iPhone", 5), "ios");
assert.equal(detectDevicePlatform("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15) Mobile/15E148", "MacIntel", 5), "ios");
assert.equal(detectDevicePlatform("Mozilla/5.0 (Linux; Android 15; Pixel 9)", "Linux armv8l", 5), "android");

assert.deepEqual(getAvailablePlayersForPlatform("windows"), ["browser", "potplayer", "vlc"]);
assert.deepEqual(getAvailablePlayersForPlatform("macos"), ["browser", "vlc", "infuse"]);
assert.deepEqual(getAvailablePlayersForPlatform("ios"), ["browser", "infuse", "vlc"]);
assert.deepEqual(getAvailablePlayersForPlatform("android"), ["browser", "vlc", "justplayer"]);
assert.deepEqual(getAvailablePlayersForPlatform("other"), ["browser"]);

const playback = "https://stickplay.example.com/api/playback?ticket=abc123";
assert.equal(
    androidIntentUrl(playback, "com.brouken.player"),
    "intent://stickplay.example.com/api/playback?ticket=abc123#Intent;scheme=https;package=com.brouken.player;type=video/*;S.browser_fallback_url=https%3A%2F%2Fstickplay.example.com%2Fapi%2Fplayback%3Fticket%3Dabc123;end",
);

console.log("player option tests passed");
