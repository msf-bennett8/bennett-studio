/**
 * Bennett Studio SDK
 * TypeScript client for DBaaS sharing
 */
export { BennettShareClient, clientFromUrl, createClient, extractConnectionInfo, } from './client';
export { P2PConnection, } from './p2p';
export { BennettGrpcWebClient, createGrpcWebClient, } from './grpcClient';
export { vault, getVaultStatus, } from './vault';
export { resolveHost, preloadHosts, clearResolverCache, } from './resolver';
// Vault crypto (shared between desktop and web)
export { getMasterKey, encryptToken, decryptToken, DecryptionError, openDB, deleteVaultEntry, arrayBufferToBase64, } from './vaultCrypto';
