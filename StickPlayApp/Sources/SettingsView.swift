import SwiftUI

struct SettingsView: View {
    @Environment(\.dismiss) var dismiss
    @ObservedObject var api = APIService.shared
    
    @State private var tempHost: String = ""
    @State private var tempPort: String = ""
    
    var body: some View {
        NavigationView {
            Form {
                Section(header: Text("Server Connection")) {
                    HStack {
                        Label("Server IP", systemImage: "network")
                            .frame(width: 120, alignment: .leading)
                        TextField("e.g. 192.168.1.100", text: $tempHost)
                            .keyboardType(.numbersAndPunctuation)
                            .autocapitalization(.none)
                            .disableAutocorrection(true)
                    }
                    
                    HStack {
                        Label("Port", systemImage: "bolt.horizontal.fill")
                            .frame(width: 120, alignment: .leading)
                        TextField("8000", text: $tempPort)
                            .keyboardType(.numberPad)
                    }
                }
                
                Section(header: Text("Network Status")) {
                    HStack {
                        Label("VPN Status", systemImage: "lock.shield.fill")
                        Spacer()
                        if api.isVPNConnected {
                            HStack(spacing: 4) {
                                Circle()
                                    .fill(Color.green)
                                    .frame(width: 8, height: 8)
                                Text("Connected")
                                    .foregroundColor(.green)
                            }
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .background(Color.green.opacity(0.1))
                            .cornerRadius(8)
                        } else {
                            HStack(spacing: 4) {
                                Circle()
                                    .fill(Color.red)
                                    .frame(width: 8, height: 8)
                                Text("Disconnected")
                                    .foregroundColor(.red)
                            }
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .background(Color.red.opacity(0.1))
                            .cornerRadius(8)
                        }
                    }
                }
                
                Section(footer: Text("StickPlayApp v1.0\nCopyright (c) 2026 huachun")) {
                    Button(action: {
                        api.serverHost = tempHost
                        api.serverPort = tempPort
                        dismiss()
                    }) {
                        Text("Save Changes")
                            .frame(maxWidth: .infinity)
                            .fontWeight(.bold)
                    }
                    .buttonStyle(.borderedProminent)
                    .listRowBackground(Color.clear)
                    .padding(.vertical, 8)
                }
            }
            .navigationTitle("Settings")
            .navigationBarItems(trailing: Button("Done") {
                dismiss()
            })
            .onAppear {
                tempHost = api.serverHost
                tempPort = api.serverPort
            }
        }
    }
}
