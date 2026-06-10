#!/bin/bash
set -e

# ============================================================
# BENNETT STUDIO - Complete Project Structure Generator
# ============================================================

echo 'Creating Bennett Studio project structure...'

# --- Root files ---
touch Cargo.toml
touch package.json
touch docker-compose.yml
touch Makefile
touch LICENSE-MIT
touch LICENSE-ENTERPRISE
touch CHANGELOG.md
touch RESEARCH.md

# --- .github ---
mkdir -p .github

# --- .github/workflows ---
mkdir -p .github/workflows
touch .github/workflows/ci.yml
touch .github/workflows/release.yml
touch .github/workflows/nightly.yml
touch .github/workflows/deploy-relay.yml

# --- .github/ISSUE_TEMPLATE ---
mkdir -p .github/ISSUE_TEMPLATE
touch .github/ISSUE_TEMPLATE/bug_report.md
touch .github/ISSUE_TEMPLATE/feature_request.md
touch .github/ISSUE_TEMPLATE/sharing_issue.md
touch .github/ISSUE_TEMPLATE/performance_issue.md

# --- .github/PULL_REQUEST_TEMPLATE ---
mkdir -p .github/PULL_REQUEST_TEMPLATE
touch .github/PULL_REQUEST_TEMPLATE/pull_request_template.md

# --- docs ---
mkdir -p docs

# --- docs/adr ---
mkdir -p docs/adr
touch docs/adr/template.md
touch docs/adr/adr-001-headless-engine.md
touch docs/adr/adr-002-rust-control-plane.md
touch docs/adr/adr-003-docker-runtime.md
touch docs/adr/adr-004-tauri-desktop.md
touch docs/adr/adr-005-reverse-tunnel.md
touch docs/adr/adr-006-schema-policy.md

# --- docs/rfcs ---
mkdir -p docs/rfcs
touch docs/rfcs/template.md

# --- docs/guides ---
mkdir -p docs/guides
touch docs/guides/getting-started.md
touch docs/guides/architecture-overview.md
touch docs/guides/sharing-guide.md
touch docs/guides/plugin-development.md
touch docs/guides/self-hosted-relay.md

# --- docs/api ---
mkdir -p docs/api
touch docs/api/grpc-api.md
touch docs/api/websocket-protocol.md
touch docs/api/authentication.md

# --- scripts ---
mkdir -p scripts
touch scripts/setup-dev.sh
touch scripts/setup-dev.ps1
touch scripts/build-all.sh
touch scripts/release.sh
touch scripts/install-docker.sh
touch scripts/run-e2e.sh

# --- docker ---
mkdir -p docker
touch docker/Dockerfile.engine
touch docker/Dockerfile.relay
touch docker/Dockerfile.web
touch docker/docker-compose.dev.yml
touch docker/docker-compose.prod.yml

# --- infra ---
mkdir -p infra

# --- infra/terraform ---
mkdir -p infra/terraform
touch infra/terraform/main.tf
touch infra/terraform/variables.tf
touch infra/terraform/outputs.tf

# --- infra/terraform/modules ---
mkdir -p infra/terraform/modules

# --- infra/terraform/modules/relay ---
mkdir -p infra/terraform/modules/relay
touch infra/terraform/modules/relay/main.tf
touch infra/terraform/modules/relay/variables.tf
touch infra/terraform/modules/relay/outputs.tf

# --- infra/terraform/modules/monitoring ---
mkdir -p infra/terraform/modules/monitoring
touch infra/terraform/modules/monitoring/main.tf
touch infra/terraform/modules/monitoring/variables.tf
touch infra/terraform/modules/monitoring/outputs.tf

# --- infra/k8s ---
mkdir -p infra/k8s
touch infra/k8s/namespace.yaml
touch infra/k8s/relay-deployment.yaml
touch infra/k8s/relay-service.yaml
touch infra/k8s/relay-ingress.yaml

# --- infra/ansible ---
mkdir -p infra/ansible
touch infra/ansible/playbook.yml
touch infra/ansible/inventory.ini

# --- shared ---
mkdir -p shared

# --- shared/proto ---
mkdir -p shared/proto
touch shared/proto/api.proto
touch shared/proto/database.proto
touch shared/proto/sharing.proto
touch shared/proto/auth.proto
touch shared/proto/telemetry.proto

# --- shared/types ---
mkdir -p shared/types
touch shared/types/index.ts
touch shared/types/database.ts
touch shared/types/sharing.ts
touch shared/types/api.ts

# --- shared/schemas ---
mkdir -p shared/schemas
touch shared/schemas/meta-db.sql
touch shared/schemas/audit-log.sql

# --- engine ---
mkdir -p engine
touch engine/Cargo.toml
touch engine/build.rs

# --- engine/src ---
mkdir -p engine/src
touch engine/src/main.rs
touch engine/src/lib.rs

# --- engine/src/api ---
mkdir -p engine/src/api
touch engine/src/api/mod.rs
touch engine/src/api/grpc.rs
touch engine/src/api/http.rs
touch engine/src/api/websocket.rs
touch engine/src/api/middleware.rs

# --- engine/src/auth ---
mkdir -p engine/src/auth
touch engine/src/auth/mod.rs
touch engine/src/auth/jwt.rs
touch engine/src/auth/rbac.rs
touch engine/src/auth/api_keys.rs
touch engine/src/auth/oauth.rs

# --- engine/src/control_plane ---
mkdir -p engine/src/control_plane
touch engine/src/control_plane/mod.rs

# --- engine/src/control_plane/connection ---
mkdir -p engine/src/control_plane/connection
touch engine/src/control_plane/connection/mod.rs
touch engine/src/control_plane/connection/pool.rs
touch engine/src/control_plane/connection/manager.rs
touch engine/src/control_plane/connection/health.rs

# --- engine/src/control_plane/query ---
mkdir -p engine/src/control_plane/query
touch engine/src/control_plane/query/mod.rs
touch engine/src/control_plane/query/engine.rs
touch engine/src/control_plane/query/parser.rs
touch engine/src/control_plane/query/plan.rs
touch engine/src/control_plane/query/executor.rs

# --- engine/src/control_plane/export ---
mkdir -p engine/src/control_plane/export
touch engine/src/control_plane/export/mod.rs
touch engine/src/control_plane/export/orchestrator.rs
touch engine/src/control_plane/export/sql_dump.rs
touch engine/src/control_plane/export/csv_export.rs
touch engine/src/control_plane/export/json_export.rs
touch engine/src/control_plane/export/parquet_export.rs

# --- engine/src/control_plane/migration ---
mkdir -p engine/src/control_plane/migration
touch engine/src/control_plane/migration/mod.rs
touch engine/src/control_plane/migration/runner.rs
touch engine/src/control_plane/migration/migra.rs
touch engine/src/control_plane/migration/skeema.rs
touch engine/src/control_plane/migration/version.rs

# --- engine/src/control_plane/vault ---
mkdir -p engine/src/control_plane/vault
touch engine/src/control_plane/vault/mod.rs
touch engine/src/control_plane/vault/store.rs
touch engine/src/control_plane/vault/rotation.rs
touch engine/src/control_plane/vault/encryption.rs

# --- engine/src/runtime ---
mkdir -p engine/src/runtime
touch engine/src/runtime/mod.rs

# --- engine/src/runtime/container ---
mkdir -p engine/src/runtime/container
touch engine/src/runtime/container/mod.rs
touch engine/src/runtime/container/docker.rs
touch engine/src/runtime/container/podman.rs
touch engine/src/runtime/container/image.rs

# --- engine/src/runtime/process ---
mkdir -p engine/src/runtime/process
touch engine/src/runtime/process/mod.rs
touch engine/src/runtime/process/supervisor.rs
touch engine/src/runtime/process/health.rs
touch engine/src/runtime/process/limits.rs

# --- engine/src/runtime/volume ---
mkdir -p engine/src/runtime/volume
touch engine/src/runtime/volume/mod.rs
touch engine/src/runtime/volume/manager.rs
touch engine/src/runtime/volume/backup.rs

# --- engine/src/runtime/network ---
mkdir -p engine/src/runtime/network
touch engine/src/runtime/network/mod.rs
touch engine/src/runtime/network/isolation.rs

# --- engine/src/runtime/port ---
mkdir -p engine/src/runtime/port
touch engine/src/runtime/port/mod.rs
touch engine/src/runtime/port/allocator.rs
touch engine/src/runtime/port/range.rs

# --- engine/src/sharing ---
mkdir -p engine/src/sharing
touch engine/src/sharing/mod.rs

# --- engine/src/sharing/lan ---
mkdir -p engine/src/sharing/lan
touch engine/src/sharing/lan/mod.rs
touch engine/src/sharing/lan/mdns.rs
touch engine/src/sharing/lan/discovery.rs
touch engine/src/sharing/lan/direct.rs

# --- engine/src/sharing/relay ---
mkdir -p engine/src/sharing/relay
touch engine/src/sharing/relay/mod.rs
touch engine/src/sharing/relay/client.rs
touch engine/src/sharing/relay/protocol.rs
touch engine/src/sharing/relay/reconnect.rs

# --- engine/src/sharing/session ---
mkdir -p engine/src/sharing/session
touch engine/src/sharing/session/mod.rs
touch engine/src/sharing/session/manager.rs
touch engine/src/sharing/session/uuid.rs
touch engine/src/sharing/session/state.rs

# --- engine/src/sharing/policy ---
mkdir -p engine/src/sharing/policy
touch engine/src/sharing/policy/mod.rs
touch engine/src/sharing/policy/engine.rs
touch engine/src/sharing/policy/rewrite.rs
touch engine/src/sharing/policy/validate.rs

# --- engine/src/sharing/multiplex ---
mkdir -p engine/src/sharing/multiplex
touch engine/src/sharing/multiplex/mod.rs
touch engine/src/sharing/multiplex/tunnel.rs
touch engine/src/sharing/multiplex/router.rs
touch engine/src/sharing/multiplex/buffer.rs

# --- engine/src/plugins ---
mkdir -p engine/src/plugins
touch engine/src/plugins/mod.rs
touch engine/src/plugins/loader.rs
touch engine/src/plugins/manifest.rs
touch engine/src/plugins/registry.rs

# --- engine/src/telemetry ---
mkdir -p engine/src/telemetry
touch engine/src/telemetry/mod.rs
touch engine/src/telemetry/tracing.rs
touch engine/src/telemetry/metrics.rs
touch engine/src/telemetry/logs.rs

# --- engine/src/wasm ---
mkdir -p engine/src/wasm
touch engine/src/wasm/mod.rs
touch engine/src/wasm/runtime.rs
touch engine/src/wasm/host.rs

# --- engine/src/models ---
mkdir -p engine/src/models
touch engine/src/models/mod.rs
touch engine/src/models/database.rs
touch engine/src/models/connection.rs
touch engine/src/models/user.rs
touch engine/src/models/share.rs
touch engine/src/models/query.rs

# --- engine/src/config ---
mkdir -p engine/src/config
touch engine/src/config/mod.rs
touch engine/src/config/settings.rs
touch engine/src/config/env.rs

# --- engine/src/errors ---
mkdir -p engine/src/errors
touch engine/src/errors/mod.rs
touch engine/src/errors/api.rs
touch engine/src/errors/runtime.rs
touch engine/src/errors/sharing.rs

# --- engine/src/utils ---
mkdir -p engine/src/utils
touch engine/src/utils/mod.rs
touch engine/src/utils/crypto.rs
touch engine/src/utils/fs.rs
touch engine/src/utils/net.rs

# --- engine/proto ---
mkdir -p engine/proto
touch engine/proto/api.proto
touch engine/proto/database.proto
touch engine/proto/sharing.proto
touch engine/proto/auth.proto
touch engine/proto/telemetry.proto

# --- engine/tests ---
mkdir -p engine/tests
touch engine/tests/integration_tests.rs

# --- engine/tests/fixtures ---
mkdir -p engine/tests/fixtures
touch engine/tests/fixtures/sample-schema.sql
touch engine/tests/fixtures/test-config.toml

# --- engine/benches ---
mkdir -p engine/benches
touch engine/benches/query_benchmark.rs
touch engine/benches/connection_benchmark.rs

# --- engine/migrations ---
mkdir -p engine/migrations
touch engine/migrations/001_initial.sql

# --- desktop ---
mkdir -p desktop
touch desktop/package.json
touch desktop/vite.config.ts
touch desktop/tsconfig.json
touch desktop/tailwind.config.js
touch desktop/index.html

# --- desktop/src ---
mkdir -p desktop/src
touch desktop/src/main.tsx
touch desktop/src/App.tsx
touch desktop/src/index.css

# --- desktop/src/components ---
mkdir -p desktop/src/components
touch desktop/src/components/index.ts

# --- desktop/src/components/ui ---
mkdir -p desktop/src/components/ui
touch desktop/src/components/ui/Button.tsx
touch desktop/src/components/ui/Input.tsx
touch desktop/src/components/ui/Modal.tsx
touch desktop/src/components/ui/Toast.tsx
touch desktop/src/components/ui/Table.tsx
touch desktop/src/components/ui/Sidebar.tsx
touch desktop/src/components/ui/Tabs.tsx
touch desktop/src/components/ui/Dropdown.tsx

# --- desktop/src/components/database ---
mkdir -p desktop/src/components/database
touch desktop/src/components/database/DatabaseList.tsx
touch desktop/src/components/database/DatabaseCard.tsx
touch desktop/src/components/database/DatabaseForm.tsx
touch desktop/src/components/database/DatabaseStatus.tsx

# --- desktop/src/components/query ---
mkdir -p desktop/src/components/query
touch desktop/src/components/query/QueryEditor.tsx
touch desktop/src/components/query/QueryResults.tsx
touch desktop/src/components/query/QueryHistory.tsx
touch desktop/src/components/query/QueryPlan.tsx

# --- desktop/src/components/schema ---
mkdir -p desktop/src/components/schema
touch desktop/src/components/schema/SchemaTree.tsx
touch desktop/src/components/schema/TableView.tsx
touch desktop/src/components/schema/ColumnView.tsx
touch desktop/src/components/schema/RelationshipView.tsx

# --- desktop/src/components/sharing ---
mkdir -p desktop/src/components/sharing
touch desktop/src/components/sharing/SharePanel.tsx
touch desktop/src/components/sharing/ShareLink.tsx
touch desktop/src/components/sharing/ShareSettings.tsx
touch desktop/src/components/sharing/GuestList.tsx

# --- desktop/src/components/export ---
mkdir -p desktop/src/components/export
touch desktop/src/components/export/ExportDialog.tsx
touch desktop/src/components/export/ExportProgress.tsx

# --- desktop/src/pages ---
mkdir -p desktop/src/pages
touch desktop/src/pages/HomePage.tsx
touch desktop/src/pages/DatabasePage.tsx
touch desktop/src/pages/QueryPage.tsx
touch desktop/src/pages/SchemaPage.tsx
touch desktop/src/pages/SettingsPage.tsx
touch desktop/src/pages/SharePage.tsx

# --- desktop/src/hooks ---
mkdir -p desktop/src/hooks
touch desktop/src/hooks/useDatabase.ts
touch desktop/src/hooks/useQuery.ts
touch desktop/src/hooks/useSharing.ts
touch desktop/src/hooks/useConnection.ts
touch desktop/src/hooks/useExport.ts
touch desktop/src/hooks/useAuth.ts

# --- desktop/src/stores ---
mkdir -p desktop/src/stores
touch desktop/src/stores/databaseStore.ts
touch desktop/src/stores/queryStore.ts
touch desktop/src/stores/uiStore.ts
touch desktop/src/stores/authStore.ts

# --- desktop/src/services ---
mkdir -p desktop/src/services
touch desktop/src/services/api.ts
touch desktop/src/services/grpc.ts
touch desktop/src/services/websocket.ts
touch desktop/src/services/engine.ts

# --- desktop/src/types ---
mkdir -p desktop/src/types
touch desktop/src/types/index.ts

# --- desktop/src/utils ---
mkdir -p desktop/src/utils
touch desktop/src/utils/formatters.ts
touch desktop/src/utils/validators.ts
touch desktop/src/utils/crypto.ts

# --- desktop/src-tauri ---
mkdir -p desktop/src-tauri
touch desktop/src-tauri/Cargo.toml
touch desktop/src-tauri/build.rs
touch desktop/src-tauri/tauri.conf.json

# --- desktop/src-tauri/src ---
mkdir -p desktop/src-tauri/src
touch desktop/src-tauri/src/main.rs
touch desktop/src-tauri/src/lib.rs

# --- desktop/src-tauri/src/commands ---
mkdir -p desktop/src-tauri/src/commands
touch desktop/src-tauri/src/commands/mod.rs
touch desktop/src-tauri/src/commands/database.rs
touch desktop/src-tauri/src/commands/query.rs
touch desktop/src-tauri/src/commands/sharing.rs
touch desktop/src-tauri/src/commands/system.rs

# --- desktop/src-tauri/src/engine ---
mkdir -p desktop/src-tauri/src/engine
touch desktop/src-tauri/src/engine/mod.rs
touch desktop/src-tauri/src/engine/launcher.rs
touch desktop/src-tauri/src/engine/process.rs

# --- desktop/public ---
mkdir -p desktop/public
touch desktop/public/logo.svg
touch desktop/public/favicon.ico

# --- desktop/tests ---
mkdir -p desktop/tests
touch desktop/tests/e2e.spec.ts

# --- web ---
mkdir -p web
touch web/package.json
touch web/vite.config.ts
touch web/tsconfig.json
touch web/tailwind.config.js
touch web/index.html

# --- web/src ---
mkdir -p web/src
touch web/src/main.tsx
touch web/src/App.tsx
touch web/src/index.css

# --- web/src/components ---
mkdir -p web/src/components
touch web/src/components/index.ts

# --- web/src/components/ui ---
mkdir -p web/src/components/ui
touch web/src/components/ui/Button.tsx
touch web/src/components/ui/Input.tsx
touch web/src/components/ui/Modal.tsx
touch web/src/components/ui/Toast.tsx

# --- web/src/components/database ---
mkdir -p web/src/components/database
touch web/src/components/database/RemoteDatabaseList.tsx
touch web/src/components/database/ConnectionForm.tsx

# --- web/src/components/query ---
mkdir -p web/src/components/query
touch web/src/components/query/RemoteQueryEditor.tsx
touch web/src/components/query/RemoteQueryResults.tsx

# --- web/src/components/sharing ---
mkdir -p web/src/components/sharing
touch web/src/components/sharing/JoinShare.tsx
touch web/src/components/sharing/ShareSession.tsx

# --- web/src/pages ---
mkdir -p web/src/pages
touch web/src/pages/HomePage.tsx
touch web/src/pages/ConnectPage.tsx
touch web/src/pages/QueryPage.tsx

# --- web/src/hooks ---
mkdir -p web/src/hooks
touch web/src/hooks/useRemoteDatabase.ts
touch web/src/hooks/useRemoteQuery.ts
touch web/src/hooks/useShareSession.ts

# --- web/src/services ---
mkdir -p web/src/services
touch web/src/services/api.ts
touch web/src/services/websocket.ts

# --- web/public ---
mkdir -p web/public
touch web/public/logo.svg
touch web/public/favicon.ico

# --- cli ---
mkdir -p cli
touch cli/Cargo.toml

# --- cli/src ---
mkdir -p cli/src
touch cli/src/main.rs

# --- cli/src/commands ---
mkdir -p cli/src/commands
touch cli/src/commands/mod.rs
touch cli/src/commands/database.rs
touch cli/src/commands/query.rs
touch cli/src/commands/share.rs
touch cli/src/commands/config.rs
touch cli/src/commands/diagnose.rs

# --- vscode-ext ---
mkdir -p vscode-ext
touch vscode-ext/package.json
touch vscode-ext/tsconfig.json

# --- vscode-ext/src ---
mkdir -p vscode-ext/src
touch vscode-ext/src/extension.ts

# --- vscode-ext/src/commands ---
mkdir -p vscode-ext/src/commands
touch vscode-ext/src/commands/connect.ts
touch vscode-ext/src/commands/query.ts
touch vscode-ext/src/commands/export.ts

# --- vscode-ext/src/providers ---
mkdir -p vscode-ext/src/providers
touch vscode-ext/src/providers/databaseTree.ts
touch vscode-ext/src/providers/queryResults.ts

# --- relay ---
mkdir -p relay
touch relay/Cargo.toml
touch relay/Dockerfile

# --- relay/src ---
mkdir -p relay/src
touch relay/src/main.rs

# --- relay/src/server ---
mkdir -p relay/src/server
touch relay/src/server/mod.rs
touch relay/src/server/websocket.rs
touch relay/src/server/session.rs
touch relay/src/server/multiplex.rs

# --- relay/src/auth ---
mkdir -p relay/src/auth
touch relay/src/auth/mod.rs
touch relay/src/auth/jwt.rs
touch relay/src/auth/rate_limit.rs

# --- relay/src/telemetry ---
mkdir -p relay/src/telemetry
touch relay/src/telemetry/mod.rs
touch relay/src/telemetry/metrics.rs
touch relay/src/telemetry/logs.rs

# --- plugins ---
mkdir -p plugins

# --- plugins/postgres-adapter ---
mkdir -p plugins/postgres-adapter
touch plugins/postgres-adapter/manifest.json
touch plugins/postgres-adapter/Cargo.toml

# --- plugins/postgres-adapter/src ---
mkdir -p plugins/postgres-adapter/src
touch plugins/postgres-adapter/src/lib.rs

# --- plugins/mysql-adapter ---
mkdir -p plugins/mysql-adapter
touch plugins/mysql-adapter/manifest.json
touch plugins/mysql-adapter/Cargo.toml

# --- plugins/mysql-adapter/src ---
mkdir -p plugins/mysql-adapter/src
touch plugins/mysql-adapter/src/lib.rs

# --- e2e ---
mkdir -p e2e
touch e2e/package.json
touch e2e/playwright.config.ts

# --- e2e/tests ---
mkdir -p e2e/tests
touch e2e/tests/install-database.spec.ts
touch e2e/tests/run-query.spec.ts
touch e2e/tests/share-database.spec.ts
touch e2e/tests/export-data.spec.ts

# --- e2e/fixtures ---
mkdir -p e2e/fixtures
touch e2e/fixtures/sample-data.sql

# ============================================================
# DONE - Project structure created
# ============================================================

echo 'Done! Total directories: $(find . -type d | wc -l)'
echo 'Total files: $(find . -type f | wc -l)'