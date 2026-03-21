# Prompt Keeper — Mission Control (Admin Dashboard)

React/Next.js admin dashboard for managing AI Functions.

## Views

- **Function List** (`/`) — Searchable table: Function Name, Active Version, Avg Latency (ms), Cost (Last 24h). Links to editor.
- **Prompt Editor** (`/editor?functionId=...`) — Markdown-capable System/User prompt editors with `{{variable}}` highlighting; side **Test** panel to set variable values and **Run** to see proxy output (SSE).
- **Version History** — Sidebar with timestamped versions; **Compare** two versions side-by-side; **Promote to Prod** per version.
- **Failover** (`/failover`) — Toggle to enable failover; **Primary** and **Backup** model selectors.

## Run

```bash
cd admin
npm install
npm run dev
```

Open [http://localhost:3001](http://localhost:3001). Point the Test panel at your proxy by setting `NEXT_PUBLIC_EXECUTE_URL` (default `http://localhost:8080/v1/execute`).

## Data

Uses mock data in `src/lib/mockData.ts`. Replace with API calls to your backend when ready.
