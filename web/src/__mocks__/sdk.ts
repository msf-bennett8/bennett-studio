// Mock for @bennett/sdk — prevents build failure when package isn't built
export const clientFromUrl = () => ({ query: async () => ({ rows: [], columns: [] }) });
export const extractConnectionInfo = () => null;
export type ConnectionInfo = any;
