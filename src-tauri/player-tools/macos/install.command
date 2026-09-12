#!/bin/zsh
set -eu

app_dir="${HOME}/Applications/StickPlay VLC.app"
source_file="$(/usr/bin/mktemp -t stickplay-vlc)"
trap '/bin/rm -f "${source_file}"' EXIT

if ! /usr/bin/open -Ra "VLC"; then
    echo "找不到 VLC，請先安裝 VLC for macOS。" >&2
    read -r "?按 Enter 關閉..."
    exit 1
fi

/bin/cat > "${source_file}" <<'APPLESCRIPT'
on «event GURLGURL» incomingURL
    set schemePrefix to "stickplay-vlc:"
    if incomingURL does not start with schemePrefix then return

    set encodedURL to text ((length of schemePrefix) + 1) thru -1 of incomingURL
    if encodedURL ends with "/" then set encodedURL to text 1 thru -2 of encodedURL

    set decodeScript to "function run(argv) { return decodeURIComponent(argv[0]); }"
    set videoURL to «event sysoexec» ("/usr/bin/osascript -l JavaScript -e " & quoted form of decodeScript & " -- " & quoted form of encodedURL)
    if videoURL does not start with "https://" and videoURL does not start with "http://" then return
    «event sysoexec» ("/usr/bin/open -a VLC " & quoted form of videoURL)
end «event GURLGURL»
APPLESCRIPT

/bin/mkdir -p "${HOME}/Applications"
/bin/rm -rf "${app_dir}"
/usr/bin/osacompile -o "${app_dir}" "${source_file}"

info_plist="${app_dir}/Contents/Info.plist"
if ! /usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier tw.stickplay.vlc-launcher" "${info_plist}" 2>/dev/null; then
    /usr/libexec/PlistBuddy -c "Add :CFBundleIdentifier string tw.stickplay.vlc-launcher" "${info_plist}"
fi
if ! /usr/libexec/PlistBuddy -c "Set :LSBackgroundOnly true" "${info_plist}" 2>/dev/null; then
    /usr/libexec/PlistBuddy -c "Add :LSBackgroundOnly bool true" "${info_plist}"
fi
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes array" "${info_plist}"
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0 dict" "${info_plist}"
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLName string 'StickPlay VLC URL'" "${info_plist}"
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLSchemes array" "${info_plist}"
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLSchemes:0 string stickplay-vlc" "${info_plist}"

/usr/bin/plutil -lint "${info_plist}"
/usr/bin/touch "${app_dir}"
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "${app_dir}"

echo "已安裝 ${app_dir}。重新開啟瀏覽器後即可從 StickPlay 使用 VLC。"
read -r "?按 Enter 關閉..."
