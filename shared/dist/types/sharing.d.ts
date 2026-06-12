export interface ShareLink {
    id: string;
    database_id: string;
    token: string;
    expires_at?: string;
    permissions: 'read' | 'write' | 'admin';
}
export interface ShareSession {
    id: string;
    database_id: string;
    guest_count: number;
    active: boolean;
    created_at: string;
}
//# sourceMappingURL=sharing.d.ts.map