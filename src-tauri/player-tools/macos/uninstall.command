#!/bin/zsh
set -eu

app_dir="${HOME}/Applications/StickPlay VLC.app"
if [[ -d "${app_dir}" ]]; then
    /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -u "${app_dir}" 2>/dev/null || true
    /bin/rm -rf "${app_dir}"
fi

echo "已移除 StickPlay VLC 啟動工具。"
read -r "?按 Enter 關閉..."
