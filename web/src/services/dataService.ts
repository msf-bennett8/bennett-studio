// src/services/dataService.ts
import usersSchema from '../../data/schemas/users.json';
import ordersSchema from '../../data/schemas/orders.json';
import productsSchema from '../../data/schemas/products.json';
import categoriesSchema from '../../data/schemas/categories.json';
import orderItemsSchema from '../../data/schemas/order_items.json';
import postsSchema from '../../data/schemas/posts.json';
import commentsSchema from '../../data/schemas/comments.json';
import dbMetadata from '../../data/schemas/metadata.json';

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

const schemas: Record<string, TableSchema> = {
  users: usersSchema as TableSchema,
  orders: ordersSchema as TableSchema,
  products: productsSchema as TableSchema,
  categories: categoriesSchema as TableSchema,
  order_items: orderItemsSchema as TableSchema,
  posts: postsSchema as TableSchema,
  comments: commentsSchema as TableSchema,
};

export const dataService = {
  getAllTables(): TableSchema[] {
    return Object.values(schemas);
  },

  getTable(name: string): TableSchema | undefined {
    return schemas[name];
  },

  getMetadata(): DatabaseMetadata {
    return dbMetadata as DatabaseMetadata;
  },

  getTableNames(): string[] {
    return Object.keys(schemas);
  },

  searchTables(query: string): TableSchema[] {
    const q = query.toLowerCase();
    return Object.values(schemas).filter(
      (table) =>
        table.name.toLowerCase().includes(q) ||
        table.columns.some((col) => col.name.toLowerCase().includes(q)) ||
        table.columns.some((col) => col.type.toLowerCase().includes(q))
    );
  },

  getRelatedTables(tableName: string): { hasMany: TableSchema[]; belongsTo: TableSchema[] } {
    const table = schemas[tableName];
    if (!table) return { hasMany: [], belongsTo: [] };

    return {
      hasMany: table.relationships.has_many
        .map((name) => schemas[name])
        .filter(Boolean) as TableSchema[],
      belongsTo: table.relationships.belongs_to
        .map((name) => schemas[name])
        .filter(Boolean) as TableSchema[],
    };
  },

  getColumnStats(tableName: string): {
    total: number;
    nullable: number;
    primary: number;
    foreign: number;
    withDefault: number;
  } {
    const table = schemas[tableName];
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

