#[tokio::main]
async fn main() {
    use std::io::Write;
    println!("[INIT] 進入 main function, 準備啟動伺服器...");
    std::io::stdout().flush().ok();
    stickplay_lib::run().await;
}
