# Frontend

All web UI for Prompt Keeper lives here.

- **Static site** — `index.html`, `script.js`, `styles.css`, and legal/marketing pages (`terms.html`, `privacy.html`, `trust-center.html`, etc.). Served by the backend when `STATIC_DIR` points at this directory (e.g. `STATIC_DIR=../frontend` from `backend/`).
- **admin/** — Next.js app (Mission Control dashboard). Run with `cd admin && npm install && npm run dev` (see `admin/README.md`).
