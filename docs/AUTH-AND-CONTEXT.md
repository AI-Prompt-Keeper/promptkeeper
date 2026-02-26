# Auth flow and what `context_id` is for

This doc ties together: **registration → our API key → identifying the user → picking and deciphering the right stored key**. It also defines **`context_id`** in one place.

---

## What’s missing today

You’re right that we’re missing:

1. **Registration** — create a new user (email/password or OAuth).
2. **Authentication** — login (e.g. session or token).
3. **“Our” API key** — after registration/login we generate an API key (or long‑lived token) that the **client** sends on every request (header or param).
4. **Resolving the token** — we receive the auth token, look it up, get **user_id** (and optionally **workspace_id**). Then we can pick the right **stored** key and decipher it.

The schema already has **users**, **sessions** (token → user_id), and **workspaces**. It does **not** yet have a dedicated table for “our” API keys (long‑lived tokens that map to user + workspace). You can either:

- Use **sessions** with long expiry as “our” API key, or  
- Add an **api_tokens** (or similar) table: `(token_hash, user_id, workspace_id, label, created_at)` and return the plaintext token once at creation.

So: **registration + login + “our” API key generation** are the missing pieces; the rest (identify user by token, then pick key and decipher) follows from that.

---

## End‑to‑end flow (once auth exists)

1. **User registers** → we create a row in **users** (and maybe a default **workspace** and **workspace_members**).
2. **User logs in** (or we create an API key right after signup) → we create a **session** or an **api_tokens** row and return **our API key** (or session token) to the client. That token is tied to **user_id** (and optionally a default **workspace_id**).
3. **Client calls Put** (store their OpenAI/Anthropic key):
   - Sends **Authorization: Bearer &lt;our_api_key&gt;** (or `X-API-Key`).
   - We resolve token → **user_id**, **workspace_id** (from token or default).
   - Body includes **context_id** (see below) and the raw secret. We encrypt and store the key for (user_id, workspace_id, provider), using **context_id** as AAD.
4. **Client calls Execute**:
   - Sends **Authorization: Bearer &lt;our_api_key&gt;** again.
   - We resolve token → **user_id**, **workspace_id**.
   - We load function config (prompt, provider). We look up the **stored** key by (user_id, workspace_id, provider), decipher it (using the same **context_id** as AAD), and call the LLM with that key.

So we **never** put the raw secret in the response; we only **identify the user (and scope)** from the auth token, then **select and decipher** the right stored key.

---

## What is `context_id` for?

**`context_id`** is the **scope** that a stored secret belongs to. It has two roles:

1. **Cryptographic (AAD)**  
   When we encrypt the user’s key (e.g. OpenAI key), we pass **context_id** as *Additional Authenticated Data* (AAD). That binds the ciphertext to that scope: decrypt only succeeds if you use the **same** context_id. So even if someone got the ciphertext and the DEK, they still couldn’t decrypt without knowing the correct scope.

2. **Lookup / authorization**  
   We store keys per (user_id, workspace_id, provider). The **scope** we use for that is the same as **context_id** (e.g. workspace_id or workspace_slug). So:
   - **Put:** “Store this key for **this** scope” → we save it and set AAD = context_id.
   - **Execute:** We already know (user_id, workspace_id) from the auth token; we look up the key for that workspace (and provider) and decrypt with **context_id = that workspace** (same value we used on Put).

So **context_id** is not “who is the user?” (that comes from the auth token). It is **“which scope does this key belong to?”** — e.g. which workspace. One user can have several workspaces; each workspace has its own set of stored keys. Using the same value for both AAD and lookup keeps encryption and authorization consistent.

---

## Summary

| Topic | Answer |
|--------|--------|
| **What’s missing?** | Registration, login, and generating “our” API key. Then: accept that token in requests, resolve to (user_id, workspace_id), and use that to pick and decipher the stored key. |
| **What is context_id?** | The **scope** the stored secret belongs to (e.g. workspace_id or workspace_slug). Used (1) as AAD when encrypting/decrypting so the ciphertext is bound to that scope, and (2) so we know *which* stored key to use for that scope at execute time. |
| **Flow** | Register → login / create “our” API key → client sends that key in header → we identify user (and workspace) → on Put we store their LLM key under that scope with AAD = context_id → on Execute we look up key by (user, workspace, provider) and decipher with the same context_id. |

You’re not missing something: the design just needs **registration + auth + “our” API key** implemented, and **context_id** clearly defined as the **scope** (e.g. workspace) used for both AAD and key lookup.

---

## Encoding workspace in our API key (no workspace on every request)

**Yes — encode (bind) the workspace in the API key.** Then the client **never** sends workspace on each request; they only send the key.

- **One API key = one (user_id, workspace_id).** When we issue “our” API key we store it in a table (e.g. `api_tokens`) with columns like `(token_hash, user_id, workspace_id, label)`. So when we receive the key we look it up and get both user and workspace in one step.
- **Client flow:** Client sends only `Authorization: Bearer <our_api_key>` (or `X-API-Key`). No `workspace_id` in body or query. We resolve the key → (user_id, workspace_id) and use that for Put/Execute.
- **Multiple workspaces:** If a user has several workspaces (e.g. “Personal”, “Team Alpha”), they get **one API key per workspace** (or per “environment”). Each key is bound to a different workspace_id. So the client chooses scope by choosing which key to use, not by sending a parameter every time.

So: **workspace is not attached to every request by the user; it’s encoded in which API key they use.**

---

## How workspace_id is created (when does a workspace exist?)

The workspace must exist **before** we can issue an API key bound to it. Two common patterns:

1. **Default workspace at signup**  
   When the user **registers**, we create:
   - a row in **users**
   - a default **workspace** (e.g. name “Personal”, slug `{user_id}-personal` or similar)
   - a row in **workspace_members** (user as owner).  
   Then when they create “our” API key (e.g. from settings), we can default it to that workspace. So every user has at least one workspace from day one.

2. **User creates workspaces explicitly**  
   In the app, the user can **“Create workspace”** / “Create project” (e.g. “Team Alpha”). We insert a row in **workspaces** and **workspace_members**. Later, when they **“Create API key”** we let them pick **which workspace** the key is for (or default to their first one). The key is then stored with that **workspace_id**.

So:

- **Workspace is created** either (a) automatically at registration (default workspace) or (b) when the user creates a workspace/project in the UI.
- **API key is created** after at least one workspace exists; we bind the key to a chosen (or default) **workspace_id**. That workspace_id is then “encoded” in the key (in the sense that it’s stored with the key in the DB). The user does **not** need to attach workspace to every request — they just use the right key for the workspace they want.
