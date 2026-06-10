# ADR-004: Tauri over Electron for Desktop

## Status

Accepted

## Context

The desktop client must provide a native feel while sharing web technology for rapid UI development. We evaluated:

1. **Electron**: Mature, large ecosystem, proven (VS Code, Slack)
2. **Tauri**: Rust-based, smaller binaries, native WebView
3. **Flutter**: Dart, cross-platform native, separate from web stack
4. **Native (Swift/Kotlin)**: Best performance, but requires separate codebases

## Decision

We will use **Tauri v2** for the desktop application, with React for the UI layer.

## Consequences

### Positive

- **Small binary**: ~5MB vs ~150MB for Electron
- **Memory efficient**: Uses system WebView, not bundled Chromium
- **Rust backend**: Same language as engine, easy to embed control plane
- **Security**: Rust memory safety, CSP by default, no Node.js integration
- **Cross-platform**: Single codebase for Windows, macOS, Linux
- **Auto-updater**: Built-in via Tauri
- **Native APIs**: Easy access to filesystem, notifications, system tray

### Negative

- **Ecosystem maturity**: Smaller than Electron, fewer plugins
- **WebView inconsistencies**: Different rendering engines per OS (WKWebView, WebView2, WebKitGTK)
- **Debugging**: Harder to debug than Electron's DevTools
- **Native modules**: Less documentation for custom Rust integrations

## Mitigations

- **Web-first UI**: Design UI to work in browser first, then Tauri shell
- **Feature detection**: Graceful degradation for WebView-specific features
- **Testing**: Automated E2E tests across all three platforms
- **Community**: Active Tauri Discord and growing ecosystem

## Alternatives Considered

### Electron

- **Pros**: Mature, huge ecosystem, excellent DevTools, proven at scale
- **Cons**: Massive bundle size, high memory usage, security surface area, separate backend language
- **Verdict**: Strong alternative, but Tauri's alignment with our Rust stack and performance wins

### Flutter

- **Pros**: Native performance, beautiful UI, single codebase
- **Cons**: Dart learning curve, separate from web client, harder to share code
- **Verdict**: Rejected — we want web and desktop to share React components

## References

- [Tauri Documentation](https://tauri.app/)
- [Tauri vs Electron](https://tauri.app/v1/references/benchmarks/)
- [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

## Date

2024-06-10

## Author

Bennett Studio Core Team
