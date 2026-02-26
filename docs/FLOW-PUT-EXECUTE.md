# Flow: Storing a key (Put) and using it (Execute)

This doc clarifies how **Put** (store secret) and **Execute** (run prompt with a provider) stay in sync so we know **which stored key to use** for a given execute request.

---

## The gap you’re pointing at

- **Put** stores a secret with `context_id` — but it’s unclear what that is and how it’s used later.
- **Execute** only gets `function_id` + `variables` — there is no user, app, or workspace, so we can’t select “the key this user stored for this app.”

So we’re missing **shared attributes** that identify:
1. **Who** (user / org)
2. **Which scope** (app, package, workspace)
3. **Which key** within that scope (e.g. provider: openai vs anthropic)

Those same attributes must be used when storing the key and when executing.

---

## What the schema gives us

From `schema/002_auth_and_workspaces.sql`:

- **users** — identity (e.g. after login or API key auth).
- **workspaces** — scope (e.g. “My App”, “Team Alpha”).
- **api_keys** — stored keys per **(user_id, workspace_id, provider, label)**.

So “which key” in the DB is: **(user_id, workspace_id, provider)** (and optionally label).  
We need the same notion of identity and scope on both Put and Execute.

---

## Recommended model: user + workspace + provider

Use **one consistent set of attributes** for both storing and resolving keys.

| Attribute   | Meaning | Where it comes from |
|------------|---------|----------------------|
| **User**   | Identity (who owns the key) | Auth: session token or “our” API key → `user_id` |
| **Workspace** | Scope (which app/package/project) | Auth default workspace, or explicit in request (e.g. `workspace_id` / `workspace_slug`) |
| **Provider** | Which LLM (openai, anthropic, …) | From function config or from execute request (e.g. `provider` or chosen by function) |

Then:

- **Put** stores a key for **(user_id, workspace_id, provider)** (and optionally label).  
  `context_id` should be a **stable identifier for that scope** used as AAD when encrypting, so the ciphertext is bound to that scope. A good choice is **workspace_id** (or a compound like `workspace_id` or `org_id:app_id` if you add org/app later).
- **Execute** runs in the context of the same **(user_id, workspace_id)** (from auth) and a **provider** (from function or request). We look up the stored key by **(user_id, workspace_id, provider)** and decrypt it (using the same `context_id` / workspace binding) to call the LLM.

So you’re not missing something — the design needs to **explicitly** use these common attributes on both sides.

---

## What `context_id` should be

Today `context_id` is “OrgID or AppID” for AAD only. To align with the schema and with “which key to use”:

- **Treat `context_id` as the scope identifier** that:
  - Binds the ciphertext (AAD) so only that scope can use the key.
  - Is the same value we use at execute time to **look up** the key (e.g. workspace_id or a stable slug).

Concrete options:

1. **`context_id` = `workspace_id` (UUID)**  
   - Put: store key for this workspace; AAD = workspace_id.  
   - Execute: auth → user_id + workspace_id; look up key by (user_id, workspace_id, provider); decrypt with AAD = workspace_id.

2. **`context_id` = `workspace_slug`**  
   - Same idea; use slug instead of UUID if your API is slug-based.

3. **`context_id` = composite** (e.g. `"org_123:app_456"`)  
   - Use when you have org + app and no workspace; then execute must pass or resolve the same composite.

So: **define `context_id` as “the scope this key belongs to”** and use that same value for both encryption (AAD) and lookup (execute).

---

## Auth: how we get user (and workspace)

Execute (and Put) need to know **who** and **which scope**:

- **Option A — Session / Bearer token**  
  Request has `Authorization: Bearer <session_token>`. Backend resolves token → (user_id, default_workspace_id). Optional query/body param to override workspace (e.g. `workspace_id` or `workspace_slug`).

- **Option B — “Our” API key**  
  Request has `Authorization: Bearer <our_api_key>` or `X-API-Key: <our_api_key>`. Backend maps API key → (user_id, workspace_id). One key = one workspace (or default workspace).

- **Option C — Explicit in request (for server-to-server)**  
  Body or headers carry `workspace_id` (and maybe `user_id` or an internal token). Backend validates and uses those. Less safe unless combined with strong auth.

So there should be **one auth mechanism** that yields (user_id, workspace_id). Then both Put and Execute use that.

---

## Minimal API shape to support the flow

### Put (store key)

- **Auth** (header): identifies user (and optionally workspace). If not provided, 401.
- **Body**: `secret_kind`, `raw_secret`, **`context_id`** = scope (e.g. workspace_id or workspace_slug). Optional: **`provider`** (openai / anthropic) and **`label`**.
- Backend: resolve auth → (user_id, workspace_id). Store encrypted key for (user_id, workspace_id, provider), with AAD = context_id (same as workspace_id or slug).

### Execute (use key)

- **Auth** (header): same as Put → (user_id, workspace_id).
- **Body**: `function_id`, `variables`. Optional: **`workspace_id`** or **`workspace_slug`** to override default; **`provider`** if the function allows multiple.
- Backend: resolve auth → (user_id, workspace_id). Load function config (prompt, provider). Look up stored key by (user_id, workspace_id, provider). Decrypt with context_id = workspace_id (or slug). Call LLM with that key.

So the **common attributes** are:

- **Auth** → user_id (+ default workspace_id).
- **context_id** on Put = **workspace_id** (or slug) = scope used for AAD and for “which key” at execute.
- **provider** (from function or request) to pick which of the user’s keys (openai vs anthropic, etc.).

---

## Summary

| Question | Answer |
|----------|--------|
| What is `context_id`? | The **scope** the key belongs to (e.g. workspace_id or workspace_slug). Same value is used for AAD and for looking up the key at execute. |
| How does execute know which key to use? | Auth gives (user_id, workspace_id); function or request gives provider. We look up stored key by (user_id, workspace_id, provider) and decrypt with context_id = that scope. |
| What was missing? | (1) Clear definition of context_id as “scope”; (2) Auth on both Put and Execute so we have user + workspace; (3) Execute using that same (user, workspace, provider) to resolve the key. |

Next implementation steps:

1. **Define** `context_id` in API/spec as “workspace_id or workspace_slug (scope for this key)”.
2. **Add auth** to Put and Execute (e.g. Bearer or X-API-Key → user_id + workspace_id).
3. **Wire Execute** to resolve key by (user_id, workspace_id, provider) and decrypt with that context_id (and stop using a single global/mock key when auth is present).
