import SwiftUI

@main
struct StickPlayApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
                // 強制套用淺色模式 (依據使用者偏好設定)
                .preferredColorScheme(.light)
        }
    }
}
