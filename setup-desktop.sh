#!/bin/bash
set -e

echo "=== Setting up Bennett Studio Desktop ==="
cd ~/studio.dev/bennett\ studio/desktop

# 1. Install dependencies
npm install react-router-dom lucide-react

# 2. Create directory structure
mkdir -p src/data/schemas
mkdir -p src/services
mkdir -p src/components
mkdir -p src/pages
mkdir -p src/stores
mkdir -p src/utils
mkdir -p src/theme

# 3. Create JSON schema files

cat << 'JSONEOF' > src/data/schemas/users.json
{
  "name": "users",
  "type": "table",
  "engine": "PostgreSQL",
  "version": "16.2",
  "row_count": 15420,
  "size": "2.4 MB",
  "created_at": "2024-01-15T08:00:00Z",
  "columns": [
    {
      "name": "id",
      "type": "SERIAL",
      "nullable": false,
      "default": null,
      "is_primary": true,
      "is_foreign": false,
      "references": null,
      "description": "Auto-increment primary key"
    },
    {
      "name": "email",
      "type": "VARCHAR(255)",
      "nullable": false,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Unique email address",
      "constraints": [
        "UNIQUE",
        "NOT NULL"
      ]
    },
    {
      "name": "username",
      "type": "VARCHAR(50)",
      "nullable": false,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Unique username",
      "constraints": [
        "UNIQUE"
      ]
    },
    {
      "name": "password_hash",
      "type": "VARCHAR(255)",
      "nullable": false,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Bcrypt hashed password"
    },
    {
      "name": "full_name",
      "type": "VARCHAR(100)",
      "nullable": true,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Display name"
    },
    {
      "name": "avatar_url",
      "type": "TEXT",
      "nullable": true,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Profile image URL"
    },
    {
      "name": "role",
      "type": "ENUM('admin','user','guest')",
      "nullable": false,
      "default": "'user'",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "User role"
    },
    {
      "name": "status",
      "type": "ENUM('active','inactive','suspended')",
      "nullable": false,
      "default": "'active'",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Account status"
    },
    {
      "name": "email_verified",
      "type": "BOOLEAN",
      "nullable": false,
      "default": "FALSE",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Email verification status"
    },
    {
      "name": "last_login_at",
      "type": "TIMESTAMP",
      "nullable": true,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Last login timestamp"
    },
    {
      "name": "created_at",
      "type": "TIMESTAMP",
      "nullable": false,
      "default": "NOW()",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Registration timestamp"
    },
    {
      "name": "updated_at",
      "type": "TIMESTAMP",
      "nullable": false,
      "default": "NOW()",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Last update timestamp"
    }
  ],
  "indexes": [
    {
      "name": "idx_users_email",
      "columns": [
        "email"
      ],
      "type": "btree",
      "unique": true
    },
    {
      "name": "idx_users_username",
      "columns": [
        "username"
      ],
      "type": "btree",
      "unique": true
    },
    {
      "name": "idx_users_role",
      "columns": [
        "role"
      ],
      "type": "btree",
      "unique": false
    },
    {
      "name": "idx_users_status",
      "columns": [
        "status"
      ],
      "type": "btree",
      "unique": false
    }
  ],
  "constraints": [
    {
      "name": "pk_users",
      "type": "PRIMARY KEY",
      "columns": [
        "id"
      ]
    },
    {
      "name": "uq_users_email",
      "type": "UNIQUE",
      "columns": [
        "email"
      ]
    },
    {
      "name": "uq_users_username",
      "type": "UNIQUE",
      "columns": [
        "username"
      ]
    }
  ],
  "triggers": [
    {
      "name": "trg_users_updated_at",
      "event": "UPDATE",
      "timing": "BEFORE",
      "definition": "SET NEW.updated_at = NOW()"
    }
  ],
  "relationships": {
    "has_many": [
      "orders",
      "posts",
      "comments"
    ],
    "belongs_to": []
  }
}
JSONEOF

cat << 'JSONEOF' > src/data/schemas/orders.json
{
  "name": "orders",
  "type": "table",
  "engine": "PostgreSQL",
  "version": "16.2",
  "row_count": 8934,
  "size": "1.8 MB",
  "created_at": "2024-02-01T10:00:00Z",
  "columns": [
    {
      "name": "id",
      "type": "SERIAL",
      "nullable": false,
      "default": null,
      "is_primary": true,
      "is_foreign": false,
      "references": null,
      "description": "Order ID"
    },
    {
      "name": "user_id",
      "type": "INTEGER",
      "nullable": false,
      "default": null,
      "is_primary": false,
      "is_foreign": true,
      "references": "users.id",
      "description": "Customer reference",
      "on_delete": "CASCADE",
      "on_update": "CASCADE"
    },
    {
      "name": "order_number",
      "type": "VARCHAR(50)",
      "nullable": false,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Human-readable order number",
      "constraints": [
        "UNIQUE"
      ]
    },
    {
      "name": "total_amount",
      "type": "DECIMAL(10,2)",
      "nullable": false,
      "default": "0.00",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Total order value"
    },
    {
      "name": "currency",
      "type": "VARCHAR(3)",
      "nullable": false,
      "default": "'USD'",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Currency code"
    },
    {
      "name": "status",
      "type": "ENUM('pending','confirmed','shipped','delivered','cancelled')",
      "nullable": false,
      "default": "'pending'",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Order status"
    },
    {
      "name": "shipping_address",
      "type": "JSONB",
      "nullable": true,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Shipping address JSON"
    },
    {
      "name": "billing_address",
      "type": "JSONB",
      "nullable": true,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Billing address JSON"
    },
    {
      "name": "notes",
      "type": "TEXT",
      "nullable": true,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Internal notes"
    },
    {
      "name": "created_at",
      "type": "TIMESTAMP",
      "nullable": false,
      "default": "NOW()",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Order creation time"
    },
    {
      "name": "updated_at",
      "type": "TIMESTAMP",
      "nullable": false,
      "default": "NOW()",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Last update time"
    }
  ],
  "indexes": [
    {
      "name": "idx_orders_user_id",
      "columns": [
        "user_id"
      ],
      "type": "btree",
      "unique": false
    },
    {
      "name": "idx_orders_status",
      "columns": [
        "status"
      ],
      "type": "btree",
      "unique": false
    },
    {
      "name": "idx_orders_created_at",
      "columns": [
        "created_at"
      ],
      "type": "btree",
      "unique": false
    }
  ],
  "constraints": [
    {
      "name": "pk_orders",
      "type": "PRIMARY KEY",
      "columns": [
        "id"
      ]
    },
    {
      "name": "uq_orders_order_number",
      "type": "UNIQUE",
      "columns": [
        "order_number"
      ]
    },
    {
      "name": "fk_orders_user_id",
      "type": "FOREIGN KEY",
      "columns": [
        "user_id"
      ],
      "references": "users.id"
    }
  ],
  "relationships": {
    "has_many": [
      "order_items"
    ],
    "belongs_to": [
      "users"
    ]
  }
}
JSONEOF

cat << 'JSONEOF' > src/data/schemas/products.json
{
  "name": "products",
  "type": "table",
  "engine": "PostgreSQL",
  "version": "16.2",
  "row_count": 342,
  "size": "456 KB",
  "created_at": "2024-01-20T14:30:00Z",
  "columns": [
    {
      "name": "id",
      "type": "SERIAL",
      "nullable": false,
      "default": null,
      "is_primary": true,
      "is_foreign": false,
      "references": null,
      "description": "Product ID"
    },
    {
      "name": "sku",
      "type": "VARCHAR(100)",
      "nullable": false,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Stock keeping unit",
      "constraints": [
        "UNIQUE"
      ]
    },
    {
      "name": "name",
      "type": "VARCHAR(200)",
      "nullable": false,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Product name"
    },
    {
      "name": "description",
      "type": "TEXT",
      "nullable": true,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Product description"
    },
    {
      "name": "price",
      "type": "DECIMAL(10,2)",
      "nullable": false,
      "default": "0.00",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Unit price"
    },
    {
      "name": "cost",
      "type": "DECIMAL(10,2)",
      "nullable": true,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Production cost"
    },
    {
      "name": "stock_quantity",
      "type": "INTEGER",
      "nullable": false,
      "default": "0",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Available stock"
    },
    {
      "name": "category_id",
      "type": "INTEGER",
      "nullable": true,
      "default": null,
      "is_primary": false,
      "is_foreign": true,
      "references": "categories.id",
      "description": "Product category",
      "on_delete": "SET NULL"
    },
    {
      "name": "tags",
      "type": "TEXT[]",
      "nullable": true,
      "default": "'{}'",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Product tags array"
    },
    {
      "name": "is_active",
      "type": "BOOLEAN",
      "nullable": false,
      "default": "TRUE",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Product visibility"
    },
    {
      "name": "metadata",
      "type": "JSONB",
      "nullable": true,
      "default": "'{}'",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Additional metadata"
    },
    {
      "name": "created_at",
      "type": "TIMESTAMP",
      "nullable": false,
      "default": "NOW()",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Creation time"
    },
    {
      "name": "updated_at",
      "type": "TIMESTAMP",
      "nullable": false,
      "default": "NOW()",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Update time"
    }
  ],
  "indexes": [
    {
      "name": "idx_products_sku",
      "columns": [
        "sku"
      ],
      "type": "btree",
      "unique": true
    },
    {
      "name": "idx_products_category",
      "columns": [
        "category_id"
      ],
      "type": "btree",
      "unique": false
    },
    {
      "name": "idx_products_price",
      "columns": [
        "price"
      ],
      "type": "btree",
      "unique": false
    },
    {
      "name": "idx_products_active",
      "columns": [
        "is_active"
      ],
      "type": "btree",
      "unique": false
    },
    {
      "name": "idx_products_tags",
      "columns": [
        "tags"
      ],
      "type": "gin",
      "unique": false
    }
  ],
  "constraints": [
    {
      "name": "pk_products",
      "type": "PRIMARY KEY",
      "columns": [
        "id"
      ]
    },
    {
      "name": "uq_products_sku",
      "type": "UNIQUE",
      "columns": [
        "sku"
      ]
    },
    {
      "name": "fk_products_category",
      "type": "FOREIGN KEY",
      "columns": [
        "category_id"
      ],
      "references": "categories.id"
    },
    {
      "name": "chk_products_price",
      "type": "CHECK",
      "definition": "price >= 0"
    }
  ],
  "relationships": {
    "has_many": [
      "order_items"
    ],
    "belongs_to": [
      "categories"
    ]
  }
}
JSONEOF

cat << 'JSONEOF' > src/data/schemas/categories.json
{
  "name": "categories",
  "type": "table",
  "engine": "PostgreSQL",
  "version": "16.2",
  "row_count": 28,
  "size": "64 KB",
  "created_at": "2024-01-10T09:00:00Z",
  "columns": [
    {
      "name": "id",
      "type": "SERIAL",
      "nullable": false,
      "default": null,
      "is_primary": true,
      "is_foreign": false,
      "references": null,
      "description": "Category ID"
    },
    {
      "name": "slug",
      "type": "VARCHAR(100)",
      "nullable": false,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "URL-friendly name",
      "constraints": [
        "UNIQUE"
      ]
    },
    {
      "name": "name",
      "type": "VARCHAR(100)",
      "nullable": false,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Display name"
    },
    {
      "name": "description",
      "type": "TEXT",
      "nullable": true,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Category description"
    },
    {
      "name": "parent_id",
      "type": "INTEGER",
      "nullable": true,
      "default": null,
      "is_primary": false,
      "is_foreign": true,
      "references": "categories.id",
      "description": "Parent category",
      "on_delete": "SET NULL"
    },
    {
      "name": "sort_order",
      "type": "INTEGER",
      "nullable": false,
      "default": "0",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Display order"
    },
    {
      "name": "is_active",
      "type": "BOOLEAN",
      "nullable": false,
      "default": "TRUE",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Category visibility"
    },
    {
      "name": "created_at",
      "type": "TIMESTAMP",
      "nullable": false,
      "default": "NOW()",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Creation time"
    }
  ],
  "indexes": [
    {
      "name": "idx_categories_slug",
      "columns": [
        "slug"
      ],
      "type": "btree",
      "unique": true
    },
    {
      "name": "idx_categories_parent",
      "columns": [
        "parent_id"
      ],
      "type": "btree",
      "unique": false
    }
  ],
  "constraints": [
    {
      "name": "pk_categories",
      "type": "PRIMARY KEY",
      "columns": [
        "id"
      ]
    },
    {
      "name": "uq_categories_slug",
      "type": "UNIQUE",
      "columns": [
        "slug"
      ]
    },
    {
      "name": "fk_categories_parent",
      "type": "FOREIGN KEY",
      "columns": [
        "parent_id"
      ],
      "references": "categories.id"
    }
  ],
  "relationships": {
    "has_many": [
      "products"
    ],
    "belongs_to": []
  }
}
JSONEOF

cat << 'JSONEOF' > src/data/schemas/order_items.json
{
  "name": "order_items",
  "type": "table",
  "engine": "PostgreSQL",
  "version": "16.2",
  "row_count": 24567,
  "size": "3.2 MB",
  "created_at": "2024-02-01T10:00:00Z",
  "columns": [
    {
      "name": "id",
      "type": "SERIAL",
      "nullable": false,
      "default": null,
      "is_primary": true,
      "is_foreign": false,
      "references": null,
      "description": "Line item ID"
    },
    {
      "name": "order_id",
      "type": "INTEGER",
      "nullable": false,
      "default": null,
      "is_primary": false,
      "is_foreign": true,
      "references": "orders.id",
      "description": "Parent order",
      "on_delete": "CASCADE"
    },
    {
      "name": "product_id",
      "type": "INTEGER",
      "nullable": false,
      "default": null,
      "is_primary": false,
      "is_foreign": true,
      "references": "products.id",
      "description": "Product reference",
      "on_delete": "RESTRICT"
    },
    {
      "name": "quantity",
      "type": "INTEGER",
      "nullable": false,
      "default": "1",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Quantity ordered"
    },
    {
      "name": "unit_price",
      "type": "DECIMAL(10,2)",
      "nullable": false,
      "default": "0.00",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Price at time of order"
    },
    {
      "name": "discount_amount",
      "type": "DECIMAL(10,2)",
      "nullable": false,
      "default": "0.00",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Applied discount"
    },
    {
      "name": "subtotal",
      "type": "DECIMAL(10,2)",
      "nullable": false,
      "default": "0.00",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Line total"
    },
    {
      "name": "created_at",
      "type": "TIMESTAMP",
      "nullable": false,
      "default": "NOW()",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Creation time"
    }
  ],
  "indexes": [
    {
      "name": "idx_order_items_order",
      "columns": [
        "order_id"
      ],
      "type": "btree",
      "unique": false
    },
    {
      "name": "idx_order_items_product",
      "columns": [
        "product_id"
      ],
      "type": "btree",
      "unique": false
    }
  ],
  "constraints": [
    {
      "name": "pk_order_items",
      "type": "PRIMARY KEY",
      "columns": [
        "id"
      ]
    },
    {
      "name": "fk_order_items_order",
      "type": "FOREIGN KEY",
      "columns": [
        "order_id"
      ],
      "references": "orders.id"
    },
    {
      "name": "fk_order_items_product",
      "type": "FOREIGN KEY",
      "columns": [
        "product_id"
      ],
      "references": "products.id"
    },
    {
      "name": "chk_order_items_quantity",
      "type": "CHECK",
      "definition": "quantity > 0"
    }
  ],
  "relationships": {
    "has_many": [],
    "belongs_to": [
      "orders",
      "products"
    ]
  }
}
JSONEOF

cat << 'JSONEOF' > src/data/schemas/posts.json
{
  "name": "posts",
  "type": "table",
  "engine": "PostgreSQL",
  "version": "16.2",
  "row_count": 156,
  "size": "1.1 MB",
  "created_at": "2024-03-01T11:00:00Z",
  "columns": [
    {
      "name": "id",
      "type": "SERIAL",
      "nullable": false,
      "default": null,
      "is_primary": true,
      "is_foreign": false,
      "references": null,
      "description": "Post ID"
    },
    {
      "name": "user_id",
      "type": "INTEGER",
      "nullable": false,
      "default": null,
      "is_primary": false,
      "is_foreign": true,
      "references": "users.id",
      "description": "Author",
      "on_delete": "CASCADE"
    },
    {
      "name": "slug",
      "type": "VARCHAR(200)",
      "nullable": false,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "URL slug",
      "constraints": [
        "UNIQUE"
      ]
    },
    {
      "name": "title",
      "type": "VARCHAR(255)",
      "nullable": false,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Post title"
    },
    {
      "name": "content",
      "type": "TEXT",
      "nullable": true,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Post content (markdown)"
    },
    {
      "name": "excerpt",
      "type": "VARCHAR(500)",
      "nullable": true,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Short excerpt"
    },
    {
      "name": "featured_image",
      "type": "TEXT",
      "nullable": true,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Hero image URL"
    },
    {
      "name": "status",
      "type": "ENUM('draft','published','archived')",
      "nullable": false,
      "default": "'draft'",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Publication status"
    },
    {
      "name": "published_at",
      "type": "TIMESTAMP",
      "nullable": true,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Publication date"
    },
    {
      "name": "view_count",
      "type": "INTEGER",
      "nullable": false,
      "default": "0",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "View counter"
    },
    {
      "name": "created_at",
      "type": "TIMESTAMP",
      "nullable": false,
      "default": "NOW()",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Creation time"
    },
    {
      "name": "updated_at",
      "type": "TIMESTAMP",
      "nullable": false,
      "default": "NOW()",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Update time"
    }
  ],
  "indexes": [
    {
      "name": "idx_posts_slug",
      "columns": [
        "slug"
      ],
      "type": "btree",
      "unique": true
    },
    {
      "name": "idx_posts_user",
      "columns": [
        "user_id"
      ],
      "type": "btree",
      "unique": false
    },
    {
      "name": "idx_posts_status",
      "columns": [
        "status"
      ],
      "type": "btree",
      "unique": false
    },
    {
      "name": "idx_posts_published",
      "columns": [
        "published_at"
      ],
      "type": "btree",
      "unique": false
    }
  ],
  "constraints": [
    {
      "name": "pk_posts",
      "type": "PRIMARY KEY",
      "columns": [
        "id"
      ]
    },
    {
      "name": "uq_posts_slug",
      "type": "UNIQUE",
      "columns": [
        "slug"
      ]
    },
    {
      "name": "fk_posts_user",
      "type": "FOREIGN KEY",
      "columns": [
        "user_id"
      ],
      "references": "users.id"
    }
  ],
  "relationships": {
    "has_many": [
      "comments"
    ],
    "belongs_to": [
      "users"
    ]
  }
}
JSONEOF

cat << 'JSONEOF' > src/data/schemas/comments.json
{
  "name": "comments",
  "type": "table",
  "engine": "PostgreSQL",
  "version": "16.2",
  "row_count": 3421,
  "size": "512 KB",
  "created_at": "2024-03-15T13:00:00Z",
  "columns": [
    {
      "name": "id",
      "type": "SERIAL",
      "nullable": false,
      "default": null,
      "is_primary": true,
      "is_foreign": false,
      "references": null,
      "description": "Comment ID"
    },
    {
      "name": "post_id",
      "type": "INTEGER",
      "nullable": false,
      "default": null,
      "is_primary": false,
      "is_foreign": true,
      "references": "posts.id",
      "description": "Parent post",
      "on_delete": "CASCADE"
    },
    {
      "name": "user_id",
      "type": "INTEGER",
      "nullable": false,
      "default": null,
      "is_primary": false,
      "is_foreign": true,
      "references": "users.id",
      "description": "Commenter",
      "on_delete": "CASCADE"
    },
    {
      "name": "parent_id",
      "type": "INTEGER",
      "nullable": true,
      "default": null,
      "is_primary": false,
      "is_foreign": true,
      "references": "comments.id",
      "description": "Reply to",
      "on_delete": "CASCADE"
    },
    {
      "name": "content",
      "type": "TEXT",
      "nullable": false,
      "default": null,
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Comment text"
    },
    {
      "name": "is_approved",
      "type": "BOOLEAN",
      "nullable": false,
      "default": "FALSE",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Moderation status"
    },
    {
      "name": "created_at",
      "type": "TIMESTAMP",
      "nullable": false,
      "default": "NOW()",
      "is_primary": false,
      "is_foreign": false,
      "references": null,
      "description": "Creation time"
    }
  ],
  "indexes": [
    {
      "name": "idx_comments_post",
      "columns": [
        "post_id"
      ],
      "type": "btree",
      "unique": false
    },
    {
      "name": "idx_comments_user",
      "columns": [
        "user_id"
      ],
      "type": "btree",
      "unique": false
    },
    {
      "name": "idx_comments_parent",
      "columns": [
        "parent_id"
      ],
      "type": "btree",
      "unique": false
    }
  ],
  "constraints": [
    {
      "name": "pk_comments",
      "type": "PRIMARY KEY",
      "columns": [
        "id"
      ]
    },
    {
      "name": "fk_comments_post",
      "type": "FOREIGN KEY",
      "columns": [
        "post_id"
      ],
      "references": "posts.id"
    },
    {
      "name": "fk_comments_user",
      "type": "FOREIGN KEY",
      "columns": [
        "user_id"
      ],
      "references": "users.id"
    },
    {
      "name": "fk_comments_parent",
      "type": "FOREIGN KEY",
      "columns": [
        "parent_id"
      ],
      "references": "comments.id"
    }
  ],
  "relationships": {
    "has_many": [],
    "belongs_to": [
      "posts",
      "users"
    ]
  }
}
JSONEOF

cat << 'JSONEOF' > src/data/schemas/metadata.json
{
  "database_name": "bennett_ecommerce",
  "engine": "PostgreSQL",
  "version": "16.2",
  "host": "localhost",
  "port": 5433,
  "total_tables": 7,
  "total_size": "12.4 MB",
  "total_rows": 52868,
  "collation": "en_US.UTF-8",
  "timezone": "UTC",
  "created_at": "2024-01-01T00:00:00Z",
  "last_backup": "2024-06-09T02:00:00Z",
  "tables": [
    "users",
    "orders",
    "products",
    "categories",
    "order_items",
    "posts",
    "comments"
  ]
}
JSONEOF

# 4. Create dataService.ts
cat << 'EOF' > src/services/dataService.ts
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

EOF

# 5. Create JSON type declaration
cat << 'EOF' > src/json.d.ts
declare module '*.json' {
  const value: any;
  export default value;
}
EOF

# 6. Create desktop components

cat << 'EOF' > src/App.tsx
import { useEffect } from 'react';
import { HashRouter, Routes, Route } from 'react-router-dom';
import { useThemeStore } from './stores/themeStore';
import { initConsoleBranding } from './utils/consoleBranding';
import { Layout } from './components/Layout';
import { HomePage } from './pages/HomePage';
import { DatabasePage } from './pages/DatabasePage';
import { QueryPage } from './pages/QueryPage';
import { SchemaPage } from './pages/SchemaPage';
import { SharePage } from './pages/SharePage';
import { SettingsPage } from './pages/SettingsPage';
import './index.css';

function App() {
  const { theme, colors } = useThemeStore();

  useEffect(() => {
    initConsoleBranding();
    const root = document.documentElement;
    Object.entries(colors).forEach(([key, value]) => {
      root.style.setProperty(`--${key}`, value);
    });
    root.setAttribute('data-theme', theme);
  }, [theme, colors]);

  return (
    <HashRouter>
      <Layout>
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/databases" element={<DatabasePage />} />
          <Route path="/query" element={<QueryPage />} />
          <Route path="/schema" element={<SchemaPage />} />
          <Route path="/share" element={<SharePage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </Layout>
    </HashRouter>
  );
}

export default App;

EOF


cat << 'EOF' > src/components/Layout.tsx
import { ReactNode } from 'react';
import { Sidebar } from './Sidebar';
import { TitleBar } from './TitleBar';

interface LayoutProps {
  children: ReactNode;
}

export function Layout({ children }: LayoutProps) {
  return (
    <div className="flex flex-col h-screen" style={{ backgroundColor: 'var(--bgPrimary)' }}>
      <TitleBar />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar />
        <main className="flex-1 overflow-auto">
          {children}
        </main>
      </div>
    </div>
  );
}

EOF


cat << 'EOF' > src/components/TitleBar.tsx
import { useState } from 'react';
import { Minus, Square, X, Terminal } from 'lucide-react';

export function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);

  const handleMinimize = () => {
    // @ts-ignore
    if (window.__TAURI__) {
      // @ts-ignore
      window.__TAURI__.window.appWindow.minimize();
    }
  };

  const handleMaximize = () => {
    // @ts-ignore
    if (window.__TAURI__) {
      // @ts-ignore
      window.__TAURI__.window.appWindow.toggleMaximize();
      setIsMaximized(!isMaximized);
    }
  };

  const handleClose = () => {
    // @ts-ignore
    if (window.__TAURI__) {
      // @ts-ignore
      window.__TAURI__.window.appWindow.close();
    }
  };

  return (
    <div 
      className="h-10 flex items-center justify-between px-4 select-none"
      style={{ 
        backgroundColor: 'var(--bgSecondary)', 
        borderBottom: '1px solid var(--borderDefault)',
        WebkitAppRegion: 'drag' as any,
      }}
      data-tauri-drag-region
    >
      <div className="flex items-center gap-2" style={{ WebkitAppRegion: 'no-drag' as any }}>
        <Terminal size={16} style={{ color: 'var(--accentPrimary)' }} />
        <span className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>
          Bennett Studio
        </span>
        <span className="text-xs px-2 py-0.5 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textMuted)' }}>
          Desktop
        </span>
      </div>

      <div className="flex items-center gap-2" style={{ WebkitAppRegion: 'no-drag' as any }}>
        <button 
          onClick={handleMinimize}
          className="w-8 h-8 rounded-lg flex items-center justify-center transition-all hover:bg-white/10"
          style={{ color: 'var(--textSecondary)' }}
        >
          <Minus size={14} />
        </button>
        <button 
          onClick={handleMaximize}
          className="w-8 h-8 rounded-lg flex items-center justify-center transition-all hover:bg-white/10"
          style={{ color: 'var(--textSecondary)' }}
        >
          <Square size={14} />
        </button>
        <button 
          onClick={handleClose}
          className="w-8 h-8 rounded-lg flex items-center justify-center transition-all hover:bg-red-500/20"
          style={{ color: 'var(--textSecondary)' }}
        >
          <X size={14} />
        </button>
      </div>
    </div>
  );
}

EOF


cat << 'EOF' > src/components/Sidebar.tsx
import { useLocation, useNavigate } from 'react-router-dom';
import { Database, Search, Table2, Share2, Settings, Home, Terminal, Cpu } from 'lucide-react';

const navItems = [
  { icon: Home, label: 'Home', path: '/' },
  { icon: Database, label: 'Databases', path: '/databases' },
  { icon: Search, label: 'Query', path: '/query' },
  { icon: Table2, label: 'Schema', path: '/schema' },
  { icon: Share2, label: 'Share', path: '/share' },
  { icon: Settings, label: 'Settings', path: '/settings' },
];

export function Sidebar() {
  const location = useLocation();
  const navigate = useNavigate();

  return (
    <aside className="w-64 flex flex-col border-r" style={{ backgroundColor: 'var(--bgSecondary)', borderColor: 'var(--borderDefault)' }}>
      <div className="p-6 flex items-center gap-3">
        <div className="w-10 h-10 rounded-xl flex items-center justify-center font-bold text-xl" style={{ backgroundColor: 'var(--accentPrimary)', color: 'var(--textInverse)' }}>
          <Terminal size={20} />
        </div>
        <div>
          <h1 className="font-bold text-lg" style={{ color: 'var(--textPrimary)' }}>Bennett</h1>
          <p className="text-xs" style={{ color: 'var(--textMuted)' }}>Studio Desktop</p>
        </div>
      </div>

      <nav className="flex-1 px-3 py-4 space-y-1">
        {navItems.map((item) => {
          const Icon = item.icon;
          const isActive = location.pathname === item.path;
          return (
            <button key={item.path} onClick={() => navigate(item.path)} className="w-full flex items-center gap-3 px-4 py-3 rounded-xl text-sm font-medium transition-all"
              style={{ backgroundColor: isActive ? 'var(--surfaceActive)' : 'transparent', color: isActive ? 'var(--accentPrimary)' : 'var(--textSecondary)', borderRight: isActive ? '3px solid var(--accentPrimary)' : '3px solid transparent' }}>
              <Icon size={18} />
              {item.label}
            </button>
          );
        })}
      </nav>

      <div className="p-4 border-t space-y-2" style={{ borderColor: 'var(--borderDefault)' }}>
        <div className="flex items-center gap-3 px-4 py-3 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)' }}>
          <div className="w-2 h-2 rounded-full" style={{ backgroundColor: 'var(--accentSuccess)' }} />
          <span className="text-xs" style={{ color: 'var(--textSecondary)' }}>Engine Online</span>
        </div>
        <div className="flex items-center gap-3 px-4 py-2 rounded-xl" style={{ backgroundColor: 'var(--bgTertiary)' }}>
          <Cpu size={14} style={{ color: 'var(--textMuted)' }} />
          <span className="text-xs" style={{ color: 'var(--textMuted)' }}>Rust v1.78</span>
        </div>
      </div>
    </aside>
  );
}

EOF


cat << 'EOF' > src/pages/HomePage.tsx
import { useNavigate } from 'react-router-dom';
import { Database, Share2, Terminal, Zap, Shield, Globe, HardDrive, Wifi } from 'lucide-react';

export function HomePage() {
  const navigate = useNavigate();

  const features = [
    { icon: Database, title: 'Local Databases', description: 'Install PostgreSQL, MySQL, MariaDB, SQLite with one click. Docker-powered, zero config.', color: 'var(--accentPrimary)' },
    { icon: Share2, title: 'Secure Sharing', description: 'Share database access via secure tunnels. No firewall holes, UUID-based URLs.', color: 'var(--accentSecondary)' },
    { icon: Terminal, title: 'SQL Editor', description: 'Write queries with Monaco Editor, syntax highlighting, autocomplete, and real-time results.', color: 'var(--accentWarning)' },
    { icon: Zap, title: 'Native Performance', description: 'Built with Rust + Tauri. Direct system integration, native notifications, global hotkeys.', color: 'var(--accentSuccess)' },
    { icon: Shield, title: 'Enterprise Security', description: 'Schema-aware permissions, credential vaulting, audit logging, end-to-end encryption.', color: 'var(--accentError)' },
    { icon: Globe, title: 'Multi-Client Sync', description: 'Desktop, web, CLI, VS Code — all share the same headless engine via gRPC/WebSocket.', color: 'var(--accentInfo)' },
  ];

  const nativeFeatures = [
    { icon: HardDrive, label: 'Docker Runtime', value: 'Active', status: 'running' },
    { icon: Wifi, label: 'Relay Connection', value: 'Connected', status: 'active' },
    { icon: Terminal, label: 'Engine Process', value: 'PID 4521', status: 'running' },
  ];

  return (
    <div className="p-8 max-w-6xl mx-auto">
      {/* Native Status Bar */}
      <div className="grid grid-cols-3 gap-4 mb-8">
        {nativeFeatures.map((feature, index) => (
          <div key={index} className="card p-4 rounded-xl flex items-center gap-3" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
            <feature.icon size={20} style={{ color: feature.status === 'running' ? 'var(--accentSuccess)' : 'var(--accentPrimary)' }} />
            <div>
              <p className="text-xs" style={{ color: 'var(--textMuted)' }}>{feature.label}</p>
              <p className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>{feature.value}</p>
            </div>
          </div>
        ))}
      </div>

      {/* Hero */}
      <div className="text-center mb-16">
        <h1 className="text-5xl font-bold mb-4" style={{ color: 'var(--textPrimary)' }}>Bennett Studio</h1>
        <p className="text-xl mb-2" style={{ color: 'var(--textSecondary)' }}>The Database Workspace for Modern Developers</p>
        <p className="text-sm mb-8" style={{ color: 'var(--textMuted)' }}>林深时见鹿，海深时见鲸，情深时见你</p>
        <div className="flex justify-center gap-4">
          <button onClick={() => navigate('/databases')} className="btn-primary px-6 py-3 rounded-xl font-medium">
            <Database size={18} className="inline mr-2" />Add Database
          </button>
          <button onClick={() => navigate('/share')} className="btn-secondary px-6 py-3 rounded-xl font-medium">
            <Share2 size={18} className="inline mr-2" />Share Access
          </button>
        </div>
      </div>

      {/* Features Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {features.map((feature, index) => {
          const Icon = feature.icon;
          return (
            <div key={index} className="card p-6 rounded-xl transition-all hover:scale-[1.02]" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
              <div className="w-12 h-12 rounded-xl flex items-center justify-center mb-4" style={{ backgroundColor: `${feature.color}20` }}>
                <Icon size={24} style={{ color: feature.color }} />
              </div>
              <h3 className="text-lg font-semibold mb-2" style={{ color: 'var(--textPrimary)' }}>{feature.title}</h3>
              <p className="text-sm leading-relaxed" style={{ color: 'var(--textSecondary)' }}>{feature.description}</p>
            </div>
          );
        })}
      </div>

      {/* Quick Stats */}
      <div className="mt-12 grid grid-cols-2 md:grid-cols-4 gap-4 p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
        {[
          { label: 'Active Databases', value: '0', color: 'var(--accentPrimary)' },
          { label: 'Active Shares', value: '0', color: 'var(--accentSecondary)' },
          { label: 'Queries Today', value: '0', color: 'var(--accentWarning)' },
          { label: 'Connected Peers', value: '0', color: 'var(--accentSuccess)' },
        ].map((stat, index) => (
          <div key={index} className="text-center">
            <div className="text-3xl font-bold mb-1" style={{ color: stat.color }}>{stat.value}</div>
            <div className="text-xs" style={{ color: 'var(--textMuted)' }}>{stat.label}</div>
          </div>
        ))}
      </div>
    </div>
  );
}

EOF


cat << 'EOF' > src/pages/DatabasePage.tsx
import { useState, useEffect } from 'react';
import { Database, Plus, Play, Trash2, RefreshCw, CheckCircle, XCircle, Clock, FolderOpen } from 'lucide-react';

interface DatabaseInstance {
  id: string; name: string; type: 'postgres' | 'mysql' | 'mariadb' | 'sqlite' | 'redis';
  version: string; status: 'running' | 'stopped' | 'error' | 'starting';
  port: number; size: string; createdAt: string; containerId?: string;
}

const mockDatabases: DatabaseInstance[] = [
  { id: '1', name: 'local-postgres', type: 'postgres', version: '16.2', status: 'running', port: 5433, size: '245 MB', createdAt: '2024-06-10', containerId: 'pg-16-local' },
  { id: '2', name: 'dev-mysql', type: 'mysql', version: '8.0', status: 'stopped', port: 3307, size: '128 MB', createdAt: '2024-06-09' },
];

const dbTypes = [
  { id: 'postgres', name: 'PostgreSQL', versions: ['16.2', '15.6', '14.11'] },
  { id: 'mysql', name: 'MySQL', versions: ['8.0', '8.4'] },
  { id: 'mariadb', name: 'MariaDB', versions: ['11.2', '10.11'] },
  { id: 'sqlite', name: 'SQLite', versions: ['3.45'] },
  { id: 'redis', name: 'Redis', versions: ['7.2', '7.0'] },
];

export function DatabasePage() {
  const [databases, setDatabases] = useState<DatabaseInstance[]>(mockDatabases);
  const [showAddModal, setShowAddModal] = useState(false);
  const [selectedType, setSelectedType] = useState('postgres');
  const [selectedVersion, setSelectedVersion] = useState('16.2');
  const [dbName, setDbName] = useState('');
  const [logs, setLogs] = useState<string[]>([]);

  const addLog = (message: string) => {
    setLogs(prev => [...prev.slice(-50), `[${new Date().toLocaleTimeString()}] ${message}`]);
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'running': return <CheckCircle size={16} style={{ color: 'var(--accentSuccess)' }} />;
      case 'stopped': return <XCircle size={16} style={{ color: 'var(--accentError)' }} />;
      case 'starting': return <Clock size={16} style={{ color: 'var(--accentWarning)' }} />;
      default: return <XCircle size={16} style={{ color: 'var(--accentError)' }} />;
    }
  };

  const handleAddDatabase = () => {
    if (!dbName) return;
    addLog(`Initializing ${selectedType} ${selectedVersion} container...`);
    const newDb: DatabaseInstance = {
      id: Date.now().toString(), name: dbName, type: selectedType as any, version: selectedVersion,
      status: 'starting', port: 5432 + databases.length + 1, size: '0 MB', createdAt: new Date().toISOString().split('T')[0],
    };
    setDatabases([...databases, newDb]);
    setShowAddModal(false); setDbName('');
    addLog(`Pulling ${selectedType}:${selectedVersion}-alpine image...`);
    setTimeout(() => {
      addLog(`Container created. Starting health checks...`);
      setDatabases(prev => prev.map(db => db.id === newDb.id ? { ...db, status: 'running', containerId: `${selectedType}-${selectedVersion}-${dbName}` } : db));
      addLog(`Database ${dbName} is ready on port ${newDb.port}`);
    }, 3000);
  };

  const handleDelete = (id: string) => {
    const db = databases.find(d => d.id === id);
    if (db) addLog(`Removing container ${db.containerId || db.name}...`);
    setDatabases(databases.filter(db => db.id !== id));
    addLog(`Database ${db?.name} removed`);
  };

  const handleToggle = (id: string) => {
    const db = databases.find(d => d.id === id);
    const isStarting = db?.status === 'stopped';
    setDatabases(databases.map(db => db.id === id ? { ...db, status: db.status === 'running' ? 'stopped' : 'starting' as any } : db));
    addLog(`${isStarting ? 'Starting' : 'Stopping'} ${db?.name}...`);
    setTimeout(() => {
      setDatabases(prev => prev.map(db => db.id === id ? { ...db, status: db.status === 'starting' ? 'running' : 'stopped' as any } : db));
      addLog(`${db?.name} ${isStarting ? 'started' : 'stopped'}`);
    }, 2000);
  };

  const handleOpenFolder = (id: string) => {
    // @ts-ignore
    if (window.__TAURI__) {
      // @ts-ignore
      window.__TAURI__.shell.open(`~/studio.dev/bennett studio/data/${id}`);
    }
  };

  return (
    <div className="flex h-full">
      <div className="flex-1 p-8 max-w-6xl mx-auto overflow-auto">
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-3xl font-bold" style={{ color: 'var(--textPrimary)' }}>Databases</h1>
            <p className="text-sm mt-1" style={{ color: 'var(--textSecondary)' }}>Manage your local Docker database instances</p>
          </div>
          <button onClick={() => setShowAddModal(true)} className="btn-primary flex items-center gap-2 px-4 py-2 rounded-xl">
            <Plus size={18} /> Add Database
          </button>
        </div>

        <div className="space-y-4">
          {databases.map((db) => (
            <div key={db.id} className="card p-6 rounded-xl flex items-center justify-between" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
              <div className="flex items-center gap-4">
                <div className="w-12 h-12 rounded-xl flex items-center justify-center" style={{ backgroundColor: 'var(--bgTertiary)' }}>
                  <Database size={24} style={{ color: 'var(--accentPrimary)' }} />
                </div>
                <div>
                  <h3 className="font-semibold" style={{ color: 'var(--textPrimary)' }}>{db.name}</h3>
                  <div className="flex items-center gap-2 mt-1">
                    <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>{db.type} {db.version}</span>
                    <span className="text-xs" style={{ color: 'var(--textMuted)' }}>port:{db.port}</span>
                    <span className="text-xs" style={{ color: 'var(--textMuted)' }}>{db.size}</span>
                    {db.containerId && <span className="text-xs font-mono" style={{ color: 'var(--textMuted)' }}>{db.containerId}</span>}
                  </div>
                </div>
              </div>
              <div className="flex items-center gap-3">
                <div className="flex items-center gap-2 px-3 py-1 rounded-full" style={{ backgroundColor: db.status === 'running' ? 'rgba(0,212,170,0.1)' : db.status === 'starting' ? 'rgba(255,170,0,0.1)' : 'rgba(255,68,68,0.1)' }}>
                  {getStatusIcon(db.status)}
                  <span className="text-xs font-medium" style={{ color: db.status === 'running' ? 'var(--accentSuccess)' : db.status === 'starting' ? 'var(--accentWarning)' : 'var(--accentError)' }}>{db.status}</span>
                </div>
                <button onClick={() => handleToggle(db.id)} className="p-2 rounded-lg transition-all" style={{ backgroundColor: 'var(--bgTertiary)' }} title={db.status === 'running' ? 'Stop' : 'Start'}>
                  {db.status === 'running' ? <RefreshCw size={16} /> : <Play size={16} />}
                </button>
                <button onClick={() => handleOpenFolder(db.id)} className="p-2 rounded-lg transition-all" style={{ backgroundColor: 'var(--bgTertiary)' }} title="Open data folder">
                  <FolderOpen size={16} />
                </button>
                <button onClick={() => handleDelete(db.id)} className="p-2 rounded-lg transition-all hover:bg-red-500/20" style={{ backgroundColor: 'var(--bgTertiary)' }} title="Delete">
                  <Trash2 size={16} style={{ color: 'var(--accentError)' }} />
                </button>
              </div>
            </div>
          ))}
          {databases.length === 0 && (
            <div className="text-center py-16 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px dashed var(--borderDefault)' }}>
              <Database size={48} className="mx-auto mb-4" style={{ color: 'var(--textMuted)' }} />
              <p style={{ color: 'var(--textSecondary)' }}>No databases yet</p>
              <p className="text-sm mt-1" style={{ color: 'var(--textMuted)' }}>Click "Add Database" to get started</p>
            </div>
          )}
        </div>

        {showAddModal && (
          <div className="fixed inset-0 flex items-center justify-center z-50" style={{ backgroundColor: 'var(--bgOverlay)' }}>
            <div className="w-full max-w-md p-6 rounded-2xl" style={{ backgroundColor: 'var(--bgElevated)', border: '1px solid var(--borderDefault)' }}>
              <h2 className="text-xl font-bold mb-6" style={{ color: 'var(--textPrimary)' }}>Add Database</h2>
              <div className="space-y-4">
                <div>
                  <label className="block text-sm mb-2" style={{ color: 'var(--textSecondary)' }}>Database Name</label>
                  <input type="text" value={dbName} onChange={(e) => setDbName(e.target.value)} placeholder="e.g., my-project-db" className="input" />
                </div>
                <div>
                  <label className="block text-sm mb-2" style={{ color: 'var(--textSecondary)' }}>Database Type</label>
                  <div className="grid grid-cols-2 gap-2">
                    {dbTypes.map((type) => (
                      <button key={type.id} onClick={() => { setSelectedType(type.id); setSelectedVersion(type.versions[0]); }}
                        className="p-3 rounded-xl text-sm font-medium transition-all"
                        style={{ backgroundColor: selectedType === type.id ? 'var(--accentPrimary)' : 'var(--bgTertiary)', color: selectedType === type.id ? 'var(--textInverse)' : 'var(--textSecondary)' }}>
                        {type.name}
                      </button>
                    ))}
                  </div>
                </div>
                <div>
                  <label className="block text-sm mb-2" style={{ color: 'var(--textSecondary)' }}>Version</label>
                  <div className="flex gap-2">
                    {dbTypes.find(t => t.id === selectedType)?.versions.map((version) => (
                      <button key={version} onClick={() => setSelectedVersion(version)} className="px-4 py-2 rounded-xl text-sm font-medium transition-all"
                        style={{ backgroundColor: selectedVersion === version ? 'var(--accentPrimary)' : 'var(--bgTertiary)', color: selectedVersion === version ? 'var(--textInverse)' : 'var(--textSecondary)' }}>{version}</button>
                    ))}
                  </div>
                </div>
              </div>
              <div className="flex gap-3 mt-6">
                <button onClick={() => setShowAddModal(false)} className="btn-secondary flex-1 py-2 rounded-xl">Cancel</button>
                <button onClick={handleAddDatabase} className="btn-primary flex-1 py-2 rounded-xl" disabled={!dbName}>Add Database</button>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Logs Panel */}
      <div className="w-80 border-l flex flex-col" style={{ backgroundColor: 'var(--bgSecondary)', borderColor: 'var(--borderDefault)' }}>
        <div className="p-4 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
          <h3 className="font-semibold text-sm" style={{ color: 'var(--textPrimary)' }}>Engine Logs</h3>
        </div>
        <div className="flex-1 overflow-auto p-3 space-y-1 font-mono text-xs">
          {logs.length === 0 && (
            <p style={{ color: 'var(--textMuted)' }}>No logs yet...</p>
          )}
          {logs.map((log, index) => (
            <div key={index} className="py-1" style={{ color: 'var(--textSecondary)' }}>
              <span style={{ color: 'var(--accentPrimary)' }}>$</span> {log}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

EOF


cat << 'EOF' > src/pages/QueryPage.tsx
import { useState } from 'react';
import { Play, Copy, Check, Download, Clock, Save, FileText } from 'lucide-react';

interface QueryResult {
  columns: string[]; rows: any[][]; executionTime: number; rowCount: number;
}

const mockResults: QueryResult = {
  columns: ['id', 'name', 'email', 'created_at', 'status'],
  rows: [
    [1, 'Alice Johnson', 'alice@example.com', '2024-01-15', 'active'],
    [2, 'Bob Smith', 'bob@example.com', '2024-02-20', 'active'],
    [3, 'Charlie Brown', 'charlie@example.com', '2024-03-10', 'inactive'],
    [4, 'Diana Prince', 'diana@example.com', '2024-04-05', 'active'],
    [5, 'Eve Davis', 'eve@example.com', '2024-05-12', 'pending'],
  ],
  executionTime: 142, rowCount: 5,
};

const queryHistory = [
  'SELECT * FROM users WHERE status = \'active\'',
  'SELECT COUNT(*) FROM orders',
  'UPDATE users SET status = \'active\' WHERE id = 3',
  'CREATE INDEX idx_users_email ON users(email)',
];

export function QueryPage() {
  const [query, setQuery] = useState('SELECT * FROM users WHERE status = \'active\';');
  const [results, setResults] = useState<QueryResult | null>(null);
  const [isExecuting, setIsExecuting] = useState(false);
  const [copied, setCopied] = useState(false);
  const [savedQueries, setSavedQueries] = useState<string[]>([]);

  const handleExecute = () => {
    setIsExecuting(true);
    setTimeout(() => { setResults(mockResults); setIsExecuting(false); }, 500);
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(query);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleSave = () => {
    setSavedQueries([...savedQueries, query]);
  };

  const handleExport = async () => {
    if (!results) return;
    const csv = [results.columns.join(','), ...results.rows.map(row => row.join(','))].join('\n');

    // Try native save dialog first
    // @ts-ignore
    if (window.__TAURI__) {
      // @ts-ignore
      const { save } = window.__TAURI__.dialog;
      // @ts-ignore
      const { writeTextFile } = window.__TAURI__.fs;
      const path = await save({ defaultPath: 'query-results.csv', filters: [{ name: 'CSV', extensions: ['csv'] }] });
      if (path) {
        // @ts-ignore
        await writeTextFile(path, csv);
        return;
      }
    }

    // Fallback to browser download
    const blob = new Blob([csv], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url; a.download = 'query-results.csv'; a.click();
  };

  return (
    <div className="flex h-full">
      <div className="w-64 border-r flex flex-col" style={{ backgroundColor: 'var(--bgSecondary)', borderColor: 'var(--borderDefault)' }}>
        <div className="p-4 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
          <h3 className="font-semibold text-sm" style={{ color: 'var(--textPrimary)' }}>Query History</h3>
        </div>
        <div className="flex-1 overflow-auto p-2 space-y-1">
          {queryHistory.map((q, index) => (
            <button key={index} onClick={() => setQuery(q)} className="w-full text-left p-3 rounded-xl text-xs transition-all"
              style={{ backgroundColor: query === q ? 'var(--surfaceActive)' : 'transparent', color: 'var(--textSecondary)' }}>
              <Clock size={12} className="inline mr-2" />
              {q.length > 40 ? q.substring(0, 40) + '...' : q}
            </button>
          ))}
        </div>
        {savedQueries.length > 0 && (
          <>
            <div className="p-4 border-t border-b" style={{ borderColor: 'var(--borderDefault)' }}>
              <h3 className="font-semibold text-sm" style={{ color: 'var(--textPrimary)' }}>Saved Queries</h3>
            </div>
            <div className="p-2 space-y-1">
              {savedQueries.map((q, index) => (
                <button key={index} onClick={() => setQuery(q)} className="w-full text-left p-3 rounded-xl text-xs transition-all"
                  style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>
                  <FileText size={12} className="inline mr-2" />
                  {q.length > 40 ? q.substring(0, 40) + '...' : q}
                </button>
              ))}
            </div>
          </>
        )}
      </div>

      <div className="flex-1 flex flex-col">
        <div className="flex items-center justify-between p-4 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
          <div className="flex items-center gap-2">
            <button onClick={handleExecute} disabled={isExecuting} className="btn-primary flex items-center gap-2 px-4 py-2 rounded-xl">
              <Play size={16} /> {isExecuting ? 'Executing...' : 'Execute'}
            </button>
            <button onClick={handleCopy} className="btn-secondary flex items-center gap-2 px-3 py-2 rounded-xl">
              {copied ? <Check size={16} /> : <Copy size={16} />} {copied ? 'Copied!' : 'Copy'}
            </button>
            <button onClick={handleSave} className="btn-secondary flex items-center gap-2 px-3 py-2 rounded-xl">
              <Save size={16} /> Save
            </button>
            <button onClick={handleExport} disabled={!results} className="btn-secondary flex items-center gap-2 px-3 py-2 rounded-xl">
              <Download size={16} /> Export
            </button>
          </div>
          <span className="text-xs" style={{ color: 'var(--textMuted)' }}>Ctrl+Enter to execute</span>
        </div>

        <div className="flex-1 flex flex-col">
          <textarea value={query} onChange={(e) => setQuery(e.target.value)} onKeyDown={(e) => { if (e.ctrlKey && e.key === 'Enter') handleExecute(); }}
            className="sql-editor flex-1 p-4 resize-none outline-none" placeholder="Write your SQL query here..." spellCheck={false} />
        </div>

        {results && (
          <div className="flex-1 border-t overflow-auto" style={{ borderColor: 'var(--borderDefault)' }}>
            <div className="flex items-center justify-between p-3 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
              <div className="flex items-center gap-4">
                <span className="text-sm" style={{ color: 'var(--textSecondary)' }}>{results.rowCount} rows</span>
                <span className="text-sm" style={{ color: 'var(--textMuted)' }}>{results.executionTime}ms</span>
              </div>
            </div>
            <table className="w-full">
              <thead>
                <tr style={{ backgroundColor: 'var(--bgSecondary)' }}>
                  {results.columns.map((col, index) => (
                    <th key={index} className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>{col}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {results.rows.map((row, rowIndex) => (
                  <tr key={rowIndex} className="transition-all" style={{ backgroundColor: rowIndex % 2 === 0 ? 'var(--bgPrimary)' : 'var(--bgSecondary)' }}>
                    {row.map((cell, cellIndex) => (
                      <td key={cellIndex} className="px-4 py-3 text-sm font-mono" style={{ color: 'var(--textPrimary)', borderBottom: '1px solid var(--borderDefault)' }}>{cell}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}

EOF


cat << 'EOF' > src/pages/SchemaPage.tsx
import { useState, useMemo } from 'react';
import { Table2, Columns, Key, Link2, Search, Database, Hash, Filter, ArrowRight } from 'lucide-react';
import { dataService } from '../services/dataService';
import type { TableSchema, ColumnSchema } from '../services/dataService';

export function SchemaPage() {
  const [selectedTable, setSelectedTable] = useState<string>('users');
  const [searchQuery, setSearchQuery] = useState('');
  const [activeTab, setActiveTab] = useState<'columns' | 'indexes' | 'constraints' | 'triggers' | 'relations'>('columns');

  const tables = dataService.getAllTables();
  const metadata = dataService.getMetadata();
  const selectedTableData = dataService.getTable(selectedTable);
  const relatedTables = selectedTableData ? dataService.getRelatedTables(selectedTable) : { hasMany: [], belongsTo: [] };
  const columnStats = selectedTableData ? dataService.getColumnStats(selectedTable) : null;

  const filteredTables = useMemo(() => {
    if (!searchQuery) return tables;
    return dataService.searchTables(searchQuery);
  }, [searchQuery, tables]);

  const getTypeColor = (type: string) => {
    if (type.includes('SERIAL') || type.includes('INTEGER')) return 'var(--accentInfo)';
    if (type.includes('VARCHAR') || type.includes('TEXT')) return 'var(--accentSecondary)';
    if (type.includes('DECIMAL') || type.includes('NUMERIC')) return 'var(--accentWarning)';
    if (type.includes('TIMESTAMP') || type.includes('DATE')) return 'var(--accentSuccess)';
    if (type.includes('BOOLEAN')) return 'var(--accentPrimary)';
    if (type.includes('JSON')) return 'var(--accentError)';
    if (type.includes('ENUM')) return 'var(--accentInfo)';
    return 'var(--textMuted)';
  };

  const getConstraintBadges = (column: ColumnSchema) => {
    const badges = [];
    if (column.is_primary) badges.push({ label: 'PK', color: 'var(--accentPrimary)', icon: Key });
    if (column.is_foreign) badges.push({ label: 'FK', color: 'var(--accentSecondary)', icon: Link2 });
    if (column.constraints?.includes('UNIQUE')) badges.push({ label: 'UQ', color: 'var(--accentWarning)', icon: Hash });
    if (!column.nullable) badges.push({ label: 'NN', color: 'var(--accentError)', icon: Filter });
    return badges;
  };

  return (
    <div className="flex h-full">
      <div className="w-72 border-r flex flex-col" style={{ backgroundColor: 'var(--bgSecondary)', borderColor: 'var(--borderDefault)' }}>
        <div className="p-4 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
          <div className="flex items-center gap-2 mb-2">
            <Database size={16} style={{ color: 'var(--accentPrimary)' }} />
            <span className="text-sm font-semibold" style={{ color: 'var(--textPrimary)' }}>{metadata.database_name}</span>
          </div>
          <div className="flex items-center gap-2 text-xs" style={{ color: 'var(--textMuted)' }}>
            <span>{metadata.engine} {metadata.version}</span>
            <span>•</span>
            <span>{metadata.total_tables} tables</span>
          </div>
        </div>
        <div className="p-3">
          <div className="relative">
            <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2" style={{ color: 'var(--textMuted)' }} />
            <input type="text" value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} placeholder="Search tables, columns..." className="input pl-9 text-sm" />
          </div>
        </div>
        <div className="flex-1 overflow-auto px-2 pb-2 space-y-1">
          {filteredTables.map((table) => (
            <button key={table.name} onClick={() => { setSelectedTable(table.name); setActiveTab('columns'); }}
              className="w-full text-left p-3 rounded-xl text-sm transition-all"
              style={{ backgroundColor: selectedTable === table.name ? 'var(--surfaceActive)' : 'transparent', color: selectedTable === table.name ? 'var(--accentPrimary)' : 'var(--textSecondary)', borderRight: selectedTable === table.name ? '3px solid var(--accentPrimary)' : '3px solid transparent' }}>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Table2 size={16} />
                  <span className="font-medium">{table.name}</span>
                </div>
                <span className="text-xs" style={{ color: 'var(--textMuted)' }}>{table.row_count.toLocaleString()}</span>
              </div>
              <div className="flex items-center gap-2 mt-1">
                <span className="text-xs" style={{ color: 'var(--textMuted)' }}>{table.columns.length} cols</span>
                <span className="text-xs" style={{ color: 'var(--textMuted)' }}>{table.size}</span>
              </div>
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-auto">
        {selectedTableData && (
          <div className="p-8">
            <div className="flex items-start justify-between mb-6">
              <div>
                <div className="flex items-center gap-3 mb-2">
                  <h1 className="text-2xl font-bold" style={{ color: 'var(--textPrimary)' }}>{selectedTableData.name}</h1>
                  <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>{selectedTableData.engine} {selectedTableData.version}</span>
                </div>
                <p className="text-sm" style={{ color: 'var(--textSecondary)' }}>{selectedTableData.row_count.toLocaleString()} rows • {selectedTableData.size} • {selectedTableData.columns.length} columns</p>
              </div>
              <div className="flex gap-2">
                <button className="btn-secondary px-4 py-2 rounded-xl text-sm">View Data</button>
                <button className="btn-primary px-4 py-2 rounded-xl text-sm">Export Schema</button>
              </div>
            </div>

            {columnStats && (
              <div className="grid grid-cols-5 gap-3 mb-6">
                {[
                  { label: 'Total', value: columnStats.total, color: 'var(--accentPrimary)' },
                  { label: 'Nullable', value: columnStats.nullable, color: 'var(--accentWarning)' },
                  { label: 'Primary Keys', value: columnStats.primary, color: 'var(--accentSuccess)' },
                  { label: 'Foreign Keys', value: columnStats.foreign, color: 'var(--accentSecondary)' },
                  { label: 'With Default', value: columnStats.withDefault, color: 'var(--accentInfo)' },
                ].map((stat, i) => (
                  <div key={i} className="p-3 rounded-xl text-center" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
                    <div className="text-xl font-bold" style={{ color: stat.color }}>{stat.value}</div>
                    <div className="text-xs" style={{ color: 'var(--textMuted)' }}>{stat.label}</div>
                  </div>
                ))}
              </div>
            )}

            <div className="flex gap-1 mb-4 p-1 rounded-xl" style={{ backgroundColor: 'var(--bgSecondary)' }}>
              {[
                { id: 'columns', label: 'Columns', count: selectedTableData.columns.length },
                { id: 'indexes', label: 'Indexes', count: selectedTableData.indexes.length },
                { id: 'constraints', label: 'Constraints', count: selectedTableData.constraints.length },
                { id: 'triggers', label: 'Triggers', count: selectedTableData.triggers.length },
                { id: 'relations', label: 'Relations', count: relatedTables.hasMany.length + relatedTables.belongsTo.length },
              ].map((tab) => (
                <button key={tab.id} onClick={() => setActiveTab(tab.id as any)} className="flex-1 py-2 px-3 rounded-lg text-sm font-medium transition-all"
                  style={{ backgroundColor: activeTab === tab.id ? 'var(--surfaceActive)' : 'transparent', color: activeTab === tab.id ? 'var(--accentPrimary)' : 'var(--textSecondary)' }}>
                  {tab.label} <span className="text-xs" style={{ color: 'var(--textMuted)' }}>({tab.count})</span>
                </button>
              ))}
            </div>

            {activeTab === 'columns' && (
              <div className="rounded-xl overflow-hidden" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
                <table className="w-full">
                  <thead>
                    <tr style={{ backgroundColor: 'var(--bgSecondary)' }}>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Column</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Type</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Nullable</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Default</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Constraints</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Description</th>
                    </tr>
                  </thead>
                  <tbody>
                    {selectedTableData.columns.map((column, index) => {
                      const badges = getConstraintBadges(column);
                      return (
                        <tr key={index} style={{ backgroundColor: index % 2 === 0 ? 'var(--bgPrimary)' : 'var(--bgSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>
                          <td className="px-4 py-3">
                            <div className="flex items-center gap-2">
                              <Columns size={14} style={{ color: 'var(--textMuted)' }} />
                              <span className="text-sm font-medium font-mono" style={{ color: 'var(--textPrimary)' }}>{column.name}</span>
                            </div>
                          </td>
                          <td className="px-4 py-3">
                            <span className="text-xs px-2 py-1 rounded-full font-mono" style={{ backgroundColor: 'var(--bgTertiary)', color: getTypeColor(column.type) }}>{column.type}</span>
                          </td>
                          <td className="px-4 py-3">
                            <span className="text-sm" style={{ color: column.nullable ? 'var(--accentWarning)' : 'var(--accentSuccess)' }}>{column.nullable ? 'YES' : 'NO'}</span>
                          </td>
                          <td className="px-4 py-3">
                            <span className="text-sm font-mono" style={{ color: 'var(--textMuted)' }}>{column.default || '-'}</span>
                          </td>
                          <td className="px-4 py-3">
                            <div className="flex items-center gap-1">
                              {badges.map((badge, bi) => (
                                <span key={bi} className="flex items-center gap-1 text-xs px-2 py-0.5 rounded-full" style={{ backgroundColor: `${badge.color}20`, color: badge.color }}>
                                  <badge.icon size={10} /> {badge.label}
                                </span>
                              ))}
                              {badges.length === 0 && <span className="text-xs" style={{ color: 'var(--textMuted)' }}>-</span>}
                            </div>
                          </td>
                          <td className="px-4 py-3">
                            <span className="text-xs" style={{ color: 'var(--textSecondary)' }}>{column.description}</span>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}

            {activeTab === 'indexes' && (
              <div className="rounded-xl overflow-hidden" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
                <table className="w-full">
                  <thead>
                    <tr style={{ backgroundColor: 'var(--bgSecondary)' }}>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Name</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Columns</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Type</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Unique</th>
                    </tr>
                  </thead>
                  <tbody>
                    {selectedTableData.indexes.map((idx, index) => (
                      <tr key={index} style={{ backgroundColor: index % 2 === 0 ? 'var(--bgPrimary)' : 'var(--bgSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>
                        <td className="px-4 py-3 text-sm font-mono" style={{ color: 'var(--textPrimary)' }}>{idx.name}</td>
                        <td className="px-4 py-3">
                          <div className="flex gap-1">
                            {idx.columns.map((col, ci) => (
                              <span key={ci} className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>{col}</span>
                            ))}
                          </div>
                        </td>
                        <td className="px-4 py-3">
                          <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--accentInfo)' }}>{idx.type}</span>
                        </td>
                        <td className="px-4 py-3">
                          <span className="text-sm" style={{ color: idx.unique ? 'var(--accentSuccess)' : 'var(--textMuted)' }}>{idx.unique ? 'Yes' : 'No'}</span>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}

            {activeTab === 'constraints' && (
              <div className="rounded-xl overflow-hidden" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
                <table className="w-full">
                  <thead>
                    <tr style={{ backgroundColor: 'var(--bgSecondary)' }}>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Name</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Type</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Columns</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>References</th>
                    </tr>
                  </thead>
                  <tbody>
                    {selectedTableData.constraints.map((constraint, index) => (
                      <tr key={index} style={{ backgroundColor: index % 2 === 0 ? 'var(--bgPrimary)' : 'var(--bgSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>
                        <td className="px-4 py-3 text-sm font-mono" style={{ color: 'var(--textPrimary)' }}>{constraint.name}</td>
                        <td className="px-4 py-3">
                          <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--accentWarning)' }}>{constraint.type}</span>
                        </td>
                        <td className="px-4 py-3">
                          <div className="flex gap-1">
                            {constraint.columns.map((col, ci) => (
                              <span key={ci} className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>{col}</span>
                            ))}
                          </div>
                        </td>
                        <td className="px-4 py-3 text-sm font-mono" style={{ color: 'var(--textMuted)' }}>{constraint.references || constraint.definition || '-'}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}

            {activeTab === 'triggers' && (
              <div className="rounded-xl overflow-hidden" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
                <table className="w-full">
                  <thead>
                    <tr style={{ backgroundColor: 'var(--bgSecondary)' }}>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Name</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Event</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Timing</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Definition</th>
                    </tr>
                  </thead>
                  <tbody>
                    {selectedTableData.triggers.map((trigger, index) => (
                      <tr key={index} style={{ backgroundColor: index % 2 === 0 ? 'var(--bgPrimary)' : 'var(--bgSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>
                        <td className="px-4 py-3 text-sm font-mono" style={{ color: 'var(--textPrimary)' }}>{trigger.name}</td>
                        <td className="px-4 py-3">
                          <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--accentError)' }}>{trigger.event}</span>
                        </td>
                        <td className="px-4 py-3">
                          <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--accentInfo)' }}>{trigger.timing}</span>
                        </td>
                        <td className="px-4 py-3">
                          <code className="text-xs font-mono px-2 py-1 rounded" style={{ backgroundColor: 'var(--bgSecondary)', color: 'var(--textSecondary)' }}>{trigger.definition}</code>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}

            {activeTab === 'relations' && (
              <div className="space-y-4">
                {relatedTables.belongsTo.length > 0 && (
                  <div className="p-4 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
                    <h3 className="text-sm font-semibold mb-3" style={{ color: 'var(--textSecondary)' }}>Belongs To</h3>
                    <div className="space-y-2">
                      {relatedTables.belongsTo.map((table) => (
                        <button key={table.name} onClick={() => setSelectedTable(table.name)} className="w-full flex items-center justify-between p-3 rounded-xl text-left transition-all hover:opacity-80" style={{ backgroundColor: 'var(--bgSecondary)' }}>
                          <div className="flex items-center gap-3">
                            <Table2 size={16} style={{ color: 'var(--accentSecondary)' }} />
                            <span className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>{table.name}</span>
                            <span className="text-xs" style={{ color: 'var(--textMuted)' }}>{table.row_count.toLocaleString()} rows</span>
                          </div>
                          <ArrowRight size={14} style={{ color: 'var(--textMuted)' }} />
                        </button>
                      ))}
                    </div>
                  </div>
                )}
                {relatedTables.hasMany.length > 0 && (
                  <div className="p-4 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
                    <h3 className="text-sm font-semibold mb-3" style={{ color: 'var(--textSecondary)' }}>Has Many</h3>
                    <div className="space-y-2">
                      {relatedTables.hasMany.map((table) => (
                        <button key={table.name} onClick={() => setSelectedTable(table.name)} className="w-full flex items-center justify-between p-3 rounded-xl text-left transition-all hover:opacity-80" style={{ backgroundColor: 'var(--bgSecondary)' }}>
                          <div className="flex items-center gap-3">
                            <Table2 size={16} style={{ color: 'var(--accentPrimary)' }} />
                            <span className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>{table.name}</span>
                            <span className="text-xs" style={{ color: 'var(--textMuted)' }}>{table.row_count.toLocaleString()} rows</span>
                          </div>
                          <ArrowRight size={14} style={{ color: 'var(--textMuted)' }} />
                        </button>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

EOF

# 7. Copy SharePage and SettingsPage from web (same code)
cp ~/studio.dev/bennett\ studio/web/src/pages/SharePage.tsx src/pages/SharePage.tsx
cp ~/studio.dev/bennett\ studio/web/src/pages/SettingsPage.tsx src/pages/SettingsPage.tsx

# 8. Copy shared utilities from web
cp ~/studio.dev/bennett\ studio/web/src/utils/consoleBranding.ts src/utils/consoleBranding.ts
cp ~/studio.dev/bennett\ studio/web/src/stores/themeStore.ts src/stores/themeStore.ts
cp ~/studio.dev/bennett\ studio/web/src/theme/index.ts src/theme/index.ts
cp ~/studio.dev/bennett\ studio/web/src/index.css src/index.css

# 9. Update vite.config.ts for desktop
cat << 'EOF' > vite.config.ts
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

const bennettBranding = () => ({
  name: 'bennett-branding',
  configureServer(server) {
    const originalPrintUrls = server.printUrls;
    server.printUrls = () => {
      console.log();
      console.log('  \x1b[38;2;0;212;170m╔══════════════════════════════════════════════════════════╗\x1b[0m');
      console.log('  \x1b[38;2;0;212;170m║              B E N N E T T   S T U D I O                 ║\x1b[0m');
      console.log('  \x1b[38;2;0;212;170m║     silicon swimming ducks isotope foundation            ║\x1b[0m');
      console.log('  \x1b[38;2;0;212;170m╚══════════════════════════════════════════════════════════╝\x1b[0m');
      console.log();
      originalPrintUrls();
    };
  }
});

export default defineConfig({
  plugins: [react(), bennettBranding()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 5174,
    host: true,
  },
  assetsInclude: ['**/*.json'],
});
EOF

# 10. Update main.tsx for HashRouter
cat << 'EOF' > src/main.tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { initConsoleBranding } from './utils/consoleBranding';
import './index.css';

initConsoleBranding();

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
EOF

echo "=== Desktop setup complete! ==="
echo "Run: npm run dev"
