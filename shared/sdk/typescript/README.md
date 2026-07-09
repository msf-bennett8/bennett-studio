Markdown
Copy
Code
Preview
# @bennett/sdk

Query shared Bennett databases from any JavaScript environment.

## Installation

```bash
npm install @bennett/sdk
Or via CDN:
HTML
<script type="module">
  import { BennettClient } from 'https://cdn.jsdelivr.net/npm/@bennett/sdk@latest/dist/index.esm.js';
</script>
Quick Start
TypeScript
import { BennettClient } from '@bennett/sdk';

// From a share URL
const client = await BennettClient.fromShareUrl(
  'https://share.bennett.studio/db/AG8VBEUASN?t=eyJ0eXAi...'
);

// Query
const result = await client.query('SELECT * FROM users LIMIT 10');
console.log(result.rows);

// Schema
const schema = await client.getSchema();
console.log(schema.tables);
Connection Modes
The SDK automatically selects the best connection:
Table
Mode	Latency	Requires
P2P	~5-20ms	WebRTC support, direct network path
Relay	~50-200ms	Internet access
Direct	~1-5ms	Same network
API Reference
BennettClient.fromShareUrl(url)
Create client from a Bennett share URL.
client.query(sql, limit?, offset?)
Execute a SELECT query.
client.write(sql, parameters?)
Execute INSERT/UPDATE/DELETE.
client.getSchema()
Fetch database schema.
client.getConnectionMode()
Returns 'p2p' | 'relay' | 'direct'.
client.close()
Clean up connections.
plain

---

## PHASE 5: WebRTC Bridge in Relay (Optional Advanced)

This phase is **optional** — the SDK P2P in Phase 2 already handles browser-to-engine direct. Only implement this if you want the relay to also bridge WebRTC.

Skip this for now. The current architecture works:
- **SDK P2P** → direct browser-to-engine WebRTC (bypasses relay entirely)
- **SDK fallback** → WebSocket to relay → engine TCP

---

## DEPLOYMENT CHECKLIST

After all find-and-replaces:

| Step | Command/Action | Verify |
|------|-------------|--------|
| 1. Build SDK | `cd shared/sdk/typescript && npm run build` | `dist/` folder created |
| 2. Publish SDK | `npm publish --access public` | `npm view @bennett/sdk` works |
| 3. Deploy relay | Push to GitHub → Render dashboard → New Web Service | `curl https://bennett-relay.onrender.com/health` |
| 4. Deploy web | `cd web && vercel --prod` | `https://share.bennett.studio/db/...` loads |
| 5. Test end-to-end | Create share → open URL → query | Results appear |

---

Run these find-and-replaces in order, create the new files, then deploy.