//! Workspace CRUD: create, list, get (metadata + client API token rows), rename, delete.
//! Exec/store continue to resolve workspace **only** from the API key (`api_tokens.workspace_id`).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::auth::api_token::Auth;
use crate::auth::keygen;
use crate::auth::session::hash_token;
use crate::auth::ApiKeyScope;

use super::AppState;

fn forbid_execution_key(auth: &Auth) -> Option<(StatusCode, Json<serde_json::Value>)> {
    if matches!(auth.api_key_scope, Some(ApiKeyScope::Exe)) {
        Some((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "execution-only API key cannot access workspace management"
            })),
        ))
    } else {
        None
    }
}

async fn user_workspace_count(pool: &sqlx::PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::bigint FROM workspace_members WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
}

async fn user_is_member(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    workspace_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let ok = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM workspace_members WHERE user_id = $1 AND workspace_id = $2)",
    )
    .bind(user_id)
    .bind(workspace_id)
    .fetch_one(pool)
    .await?;
    Ok(ok)
}

/// POST /v1/workspaces — create workspace, add caller as owner, mint one management API key.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateWorkspaceBody {
    /// Display name (required, non-empty after trim).
    pub name: String,
    /// Client surface tag for analytics.
    #[serde(default = "super::default_surface")]
    pub surface: String,
}

#[derive(Debug, serde::Serialize)]
pub struct CreateWorkspaceResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    /// Prompt Keeper client key (`pk_mgt_live_...`); shown once.
    pub api_key: String,
    pub api_key_scope: &'static str,
}

pub async fn create_workspace(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<CreateWorkspaceBody>,
) -> Result<(StatusCode, Json<CreateWorkspaceResponse>), (StatusCode, Json<serde_json::Value>)> {
    if let Some(e) = forbid_execution_key(&auth) {
        return Err(e);
    }
    let surface = body.surface.clone();
    let name = body.name.trim();
    if name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "name is required" })),
        ));
    }

    let slug = format!("ws-{}", Uuid::new_v4());
    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!(error = ?e, "workspace create: begin tx");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create workspace" })),
        )
    })?;

    let row = sqlx::query("INSERT INTO workspaces (name, slug) VALUES ($1, $2) RETURNING id")
        .bind(name)
        .bind(&slug)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "insert workspace");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "failed to create workspace" })),
            )
        })?;

    let workspace_id: Uuid = row.try_get("id").map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create workspace" })),
        )
    })?;

    sqlx::query(
        "INSERT INTO workspace_members (workspace_id, user_id, role) VALUES ($1, $2, 'owner')",
    )
    .bind(workspace_id)
    .bind(auth.user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "workspace member insert");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create workspace" })),
        )
    })?;

    let api_key = keygen::generate_management_key();
    let token_hash = hash_token(&api_key);
    sqlx::query(
        "INSERT INTO api_tokens (user_id, workspace_id, token_hash, label, scope) VALUES ($1, $2, $3, $4, 'mgt')",
    )
    .bind(auth.user_id)
    .bind(workspace_id)
    .bind(&token_hash)
    .bind("Default")
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "api token insert for new workspace");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create workspace" })),
        )
    })?;

    tx.commit().await.map_err(|e| {
        tracing::error!(error = ?e, "workspace create commit");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create workspace" })),
        )
    })?;

    state.analytics.track_workspace_created(&surface, auth.user_id, workspace_id);

    Ok((
        StatusCode::CREATED,
        Json(CreateWorkspaceResponse {
            id: workspace_id,
            name: name.to_string(),
            slug,
            api_key,
            api_key_scope: "mgt",
        }),
    ))
}

#[derive(Debug, serde::Serialize)]
pub struct WorkspaceTitle {
    pub id: Uuid,
    /// Display title (workspace name).
    pub name: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ListWorkspacesResponse {
    pub workspaces: Vec<WorkspaceTitle>,
}

/// GET /v1/workspaces — list workspaces the user belongs to (id + name only).
pub async fn list_workspaces(
    State(state): State<AppState>,
    auth: Auth,
) -> Result<Json<ListWorkspacesResponse>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(e) = forbid_execution_key(&auth) {
        return Err(e);
    }

    let rows = sqlx::query(
        r#"SELECT w.id, w.name FROM workspaces w
           INNER JOIN workspace_members m ON m.workspace_id = w.id
           WHERE m.user_id = $1
           ORDER BY w.name ASC"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "list workspaces");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to list workspaces" })),
        )
    })?;

    let workspaces = rows
        .into_iter()
        .filter_map(|r| {
            Some(WorkspaceTitle {
                id: r.try_get("id").ok()?,
                name: r.try_get("name").ok()?,
            })
        })
        .collect();

    Ok(Json(ListWorkspacesResponse { workspaces }))
}

#[derive(Debug, serde::Serialize)]
pub struct WorkspaceTokenRow {
    pub id: Uuid,
    pub label: String,
    pub scope: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize)]
pub struct GetWorkspaceResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    /// Rows from `api_tokens` for this workspace (no plaintext secrets; hashes only in DB).
    pub api_tokens: Vec<WorkspaceTokenRow>,
    pub note: &'static str,
}

/// GET /v1/workspaces/:workspace_id — workspace name/slug plus client API token metadata.
pub async fn get_workspace(
    State(state): State<AppState>,
    auth: Auth,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<GetWorkspaceResponse>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(e) = forbid_execution_key(&auth) {
        return Err(e);
    }

    if !user_is_member(&state.db, auth.user_id, workspace_id)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "membership check");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "failed to load workspace" })),
            )
        })? {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "workspace not found" })),
        ));
    }

    let row = sqlx::query("SELECT id, name, slug FROM workspaces WHERE id = $1")
        .bind(workspace_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "select workspace");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "failed to load workspace" })),
            )
        })?;

    let row = row.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "workspace not found" })),
        )
    })?;

    let token_rows = sqlx::query(
        "SELECT id, label, scope, created_at FROM api_tokens WHERE workspace_id = $1 ORDER BY created_at ASC",
    )
    .bind(workspace_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "list api_tokens");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to load workspace" })),
        )
    })?;

    let api_tokens = token_rows
        .into_iter()
        .filter_map(|r| {
            Some(WorkspaceTokenRow {
                id: r.try_get("id").ok()?,
                label: r.try_get("label").ok()?,
                scope: r.try_get("scope").ok()?,
                created_at: r.try_get("created_at").ok()?,
            })
        })
        .collect();

    Ok(Json(GetWorkspaceResponse {
        id: row.try_get("id").unwrap(),
        name: row.try_get("name").unwrap(),
        slug: row.try_get("slug").unwrap(),
        api_tokens,
        note: "Client API key secrets (pk_mgt_live_ / pk_exe_live_) are only shown once when created; only metadata is listed here.",
    }))
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateWorkspaceBody {
    pub name: String,
}

/// PATCH /v1/workspaces/:workspace_id — rename workspace (keys unchanged).
pub async fn update_workspace(
    State(state): State<AppState>,
    auth: Auth,
    Path(workspace_id): Path<Uuid>,
    Json(body): Json<UpdateWorkspaceBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(e) = forbid_execution_key(&auth) {
        return Err(e);
    }

    if !user_is_member(&state.db, auth.user_id, workspace_id)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "membership check");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "failed to update workspace" })),
            )
        })? {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "workspace not found" })),
        ));
    }

    let name = body.name.trim();
    if name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "name is required" })),
        ));
    }

    let updated = sqlx::query(
        "UPDATE workspaces SET name = $1, updated_at = now() WHERE id = $2 RETURNING id, name, slug",
    )
    .bind(name)
    .bind(workspace_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "update workspace");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to update workspace" })),
        )
    })?;

    let row = updated.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "workspace not found" })),
        )
    })?;

    Ok(Json(serde_json::json!({
        "id": row.try_get::<Uuid, _>("id").unwrap(),
        "name": row.try_get::<String, _>("name").unwrap(),
        "slug": row.try_get::<String, _>("slug").unwrap(),
    })))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MintWorkspaceMgtKeyBody {
    /// Optional label for the minted key (shown only once to the caller).
    #[serde(default = "default_mgt_key_label")]
    pub label: String,
    /// Optional client surface tag for analytics (currently only stored for consistency).
    #[serde(default = "super::default_surface")]
    pub surface: String,
}

fn default_mgt_key_label() -> String {
    "Workspace management".to_string()
}

#[derive(Debug, serde::Serialize)]
pub struct MintWorkspaceMgtKeyResponse {
    pub api_key: String,
    pub api_key_scope: &'static str,
    pub label: String,
}

/// POST /v1/workspaces/:workspace_id/mgt-key — mint a new management client API key for this workspace.
///
/// Returned plaintext key is **not** persisted; only `token_hash` is stored.
/// Authorization: member of the workspace, and not an execution-only key.
pub async fn mint_workspace_management_key(
    State(state): State<AppState>,
    auth: Auth,
    Path(workspace_id): Path<Uuid>,
    Json(body): Json<MintWorkspaceMgtKeyBody>,
) -> Result<(StatusCode, Json<MintWorkspaceMgtKeyResponse>), (StatusCode, Json<serde_json::Value>)> {
    if let Some(e) = forbid_execution_key(&auth) {
        return Err(e);
    }
    if !user_is_member(&state.db, auth.user_id, workspace_id)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "membership check");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "failed to mint workspace key" })),
            )
        })?
    {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "workspace not found" })),
        ));
    }

    let label = body.label.trim().to_string();
    if label.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "label is required" })),
        ));
    }

    let api_key = keygen::generate_management_key();
    let token_hash = hash_token(&api_key);

    sqlx::query(
        r#"
        INSERT INTO api_tokens (user_id, workspace_id, token_hash, label, scope)
        VALUES ($1, $2, $3, $4, 'mgt')
        "#,
    )
    .bind(auth.user_id)
    .bind(workspace_id)
    .bind(&token_hash)
    .bind(&label)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "mint workspace management key insert");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to mint workspace key" })),
        )
    })?;

    // Keep consistent with other endpoints: `surface` isn't used yet for response, but it is parsed.
    let _ = body.surface;

    Ok((
        StatusCode::CREATED,
        Json(MintWorkspaceMgtKeyResponse {
            api_key,
            api_key_scope: "mgt",
            label,
        }),
    ))
}

/// DELETE /v1/workspaces/:workspace_id — remove workspace, prompts/deployments for its context, keys, tokens.
pub async fn delete_workspace(
    State(state): State<AppState>,
    auth: Auth,
    Path(workspace_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    if let Some(e) = forbid_execution_key(&auth) {
        return Err(e);
    }

    if !user_is_member(&state.db, auth.user_id, workspace_id)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "membership check");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "failed to delete workspace" })),
            )
        })? {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "workspace not found" })),
        ));
    }

    let n = user_workspace_count(&state.db, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "workspace count");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "failed to delete workspace" })),
            )
        })?;
    if n <= 1 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "cannot delete last workspace" })),
        ));
    }

    let ctx = workspace_id.to_string();

    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!(error = ?e, "delete workspace begin");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to delete workspace" })),
        )
    })?;

    sqlx::query("DELETE FROM deployments WHERE context_id = $1")
        .bind(&ctx)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "delete deployments for workspace context");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "failed to delete workspace" })),
            )
        })?;

    sqlx::query("DELETE FROM prompt_versions WHERE context_id = $1")
        .bind(&ctx)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "delete prompt_versions for workspace context");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "failed to delete workspace" })),
            )
        })?;

    let res = sqlx::query("DELETE FROM workspaces WHERE id = $1")
        .bind(workspace_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "delete workspace row");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "failed to delete workspace" })),
            )
        })?;

    if res.rows_affected() == 0 {
        tx.rollback().await.ok();
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "workspace not found" })),
        ));
    }

    tx.commit().await.map_err(|e| {
        tracing::error!(error = ?e, "delete workspace commit");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to delete workspace" })),
        )
    })?;

    state
        .analytics
        .track_workspace_deleted(auth.user_id, workspace_id);

    Ok(StatusCode::NO_CONTENT)
}
