// src/services/dataService.ts
// Schemas loaded via fetch to avoid Vite JSON import issues

export interface ColumnSchema {
  name: string;
  type: string;
  nullable: boolean;
  default: string | null;
  is_primary: boolean;
  is_foreign: boolean;
  references: string | null;
  description: string;
  constraints?: string[];
  on_delete?: string;
  on_update?: string;
}

export interface IndexSchema {
  name: string;
  columns: string[];
  type: string;
  unique: boolean;
}

export interface ConstraintSchema {
  name: string;
  type: string;
  columns: string[];
  references?: string;
  definition?: string;
}

export interface TriggerSchema {
  name: string;
  event: string;
  timing: string;
  definition: string;
}

export interface RelationshipSchema {
  has_many: string[];
  belongs_to: string[];
}

export interface TableSchema {
  name: string;
  type: string;
  engine: string;
  version: string;
  row_count: number;
  size: string;
  created_at: string;
  columns: ColumnSchema[];
  indexes: IndexSchema[];
  constraints: ConstraintSchema[];
  triggers: TriggerSchema[];
  relationships: RelationshipSchema;
}

export interface DatabaseMetadata {
  database_name: string;
  engine: string;
  version: string;
  host: string;
  port: number;
  total_tables: number;
  total_size: string;
  total_rows: number;
  collation: string;
  timezone: string;
  created_at: string;
  last_backup: string;
  tables: string[];
}

const schemaCache: Record<string, TableSchema> = {};
let metadataCache: DatabaseMetadata | null = null;

const tableNames = ['users', 'orders', 'products', 'categories', 'order_items', 'posts', 'comments'];

export const dataService = {
  async loadAllSchemas(): Promise<void> {
    await Promise.all(tableNames.map(name => this.loadSchema(name)));
  },

  async loadSchema(name: string): Promise<TableSchema> {
    if (schemaCache[name]) return schemaCache[name];

    const response = await fetch(`/src/data/schemas/${name}.json`);
    const schema = await response.json();
    schemaCache[name] = schema;
    return schema;
  },

  async getAllTables(): Promise<TableSchema[]> {
    await this.loadAllSchemas();
    return Object.values(schemaCache);
  },

  async getTable(name: string): Promise<TableSchema | undefined> {
    await this.loadSchema(name);
    return schemaCache[name];
  },

  async getMetadata(): Promise<DatabaseMetadata> {
    if (metadataCache) return metadataCache;

    const response = await fetch('/src/data/schemas/metadata.json');
    metadataCache = await response.json();
    return metadataCache;
  },

  getTableNames(): string[] {
    return tableNames;
  },

  async searchTables(query: string): Promise<TableSchema[]> {
    const q = query.toLowerCase();
    const tables = await this.getAllTables();
    return tables.filter(
      (table) =>
        table.name.toLowerCase().includes(q) ||
        table.columns.some((col) => col.name.toLowerCase().includes(q)) ||
        table.columns.some((col) => col.type.toLowerCase().includes(q))
    );
  },

  async getRelatedTables(tableName: string): Promise<{ hasMany: TableSchema[]; belongsTo: TableSchema[] }> {
    const table = await this.getTable(tableName);
    if (!table) return { hasMany: [], belongsTo: [] };

    await this.loadAllSchemas();

    return {
      hasMany: table.relationships.has_many
        .map((name) => schemaCache[name])
        .filter(Boolean) as TableSchema[],
      belongsTo: table.relationships.belongs_to
        .map((name) => schemaCache[name])
        .filter(Boolean) as TableSchema[],
    };
  },

  async getColumnStats(tableName: string): Promise<{
    total: number;
    nullable: number;
    primary: number;
    foreign: number;
    withDefault: number;
  }> {
    const table = await this.getTable(tableName);
    if (!table) return { total: 0, nullable: 0, primary: 0, foreign: 0, withDefault: 0 };

    return {
      total: table.columns.length,
      nullable: table.columns.filter((c) => c.nullable).length,
      primary: table.columns.filter((c) => c.is_primary).length,
      foreign: table.columns.filter((c) => c.is_foreign).length,
      withDefault: table.columns.filter((c) => c.default !== null).length,
    };
  },
};