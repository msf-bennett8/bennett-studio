/**
 * Bennett Studio SDK
 * TypeScript client for DBaaS sharing
 */

export {
  BennettShareClient,
  clientFromUrl,
  createClient,
  extractConnectionInfo,
} from './client';

export {
  P2PConnection,
} from './p2p';

export {
  BennettGrpcWebClient,
  createGrpcWebClient,
} from './grpcClient';

export {
  vault,
  getVaultStatus,
} from './vault';

export {
  resolveHost,
  preloadHosts,
  clearResolverCache,
} from './resolver';

export type {
  BennettClientConfig,
  QueryResult,
  WriteResult,
  SchemaResult,
  TableSchema,
  ColumnSchema,
  IndexSchema,
  ConstraintSchema,
  ExportResult,
  ConnectionMode,
  ConnectionInfo,
} from './client';

export type {
  StoredToken,
  TokenVault,
  VaultStatus,
} from '@bennett/shared';

// Vault crypto (shared between desktop and web)
export {
  getMasterKey,
  encryptToken,
  decryptToken,
  DecryptionError,
  openDB,
  deleteVaultEntry,
  arrayBufferToBase64,
} from './vaultCrypto';

export type { EncryptedToken } from './vaultCrypto';
