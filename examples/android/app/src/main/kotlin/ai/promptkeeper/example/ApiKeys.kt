package ai.promptkeeper.example

/**
 * This app stores keys right in code for demonstration purposes only. Do not ship production apps with keys in source.
 * 
 * IMPORTANT — EXAMPLE ONLY:
 * - Do NOT store plaintext API keys in source code in production.
 * - Do NOT commit real keys to version control. Use BuildConfig, env vars, or a secure
 *   credential store (e.g. Android Keystore, backend token exchange).
 * - The values below are placeholders; replace only for local demo and never commit.
 *
 * Obtain keys via registration / CLI (`pk_mgt_live_...`) and mint an execution key with
 * `POST /v1/auth/api-tokens` using the management key (see backend README).
 */
object ApiKeys {
    /** 
     * Management keys can modify your vault (add prompts and keys).
     * DO NOT put management key to the app sources. Use it from CLI to store and modify prompts.
     * This app has management key for demonstration purposes only.
     */
    const val PROMPTKEEPER_MANAGEMENT_KEY: String = "TODO_add_pk_mgt_live_..."

    /** 
     * Execution key are only allowed to list and execute stored prompts.
     * Use this keys in your app to limit damage scope if keys get leaked.
     */
    const val PROMPTKEEPER_EXECUTION_KEY: String = "TODO_add_pk_exe_live_..."
}
