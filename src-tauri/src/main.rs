#[tokio::main]
async fn main() {
    if std::env::args().any(|arg| arg == "--auth-recovery") {
        let dir = std::env::var("STICKPLAY_CONFIG_DIR").unwrap_or_else(|_| "./config".into());
        if let Err(e) = stickplay_lib::auth::recovery_code(std::path::Path::new(&dir)) {
            eprintln!("無法產生復原碼：{e}");
            std::process::exit(1);
        }
        return;
    }
    use std::io::Write;
    println!("[INIT] 進入 main function, 準備啟動伺服器...");
    std::io::stdout().flush().ok();
    stickplay_lib::run().await;
}
