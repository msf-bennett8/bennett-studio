//! Desktop vault stub — re-exported from desktop app
//! This is a placeholder; the actual implementation is in desktop/src/services/vaultService.ts
//! The SDK uses dynamic import to avoid bundling Tauri APIs in web builds

export { vaultService as vaultDesktop } from '../../../desktop/src/services/vaultService';
