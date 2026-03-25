//! Handlebars-style string interpolation for prompt variables.

use handlebars::Handlebars;
use std::collections::HashMap;

/// Max serialized JSON size per variable value (bytes).
const MAX_VARIABLE_VALUE_BYTES: usize = 64 * 1024;
/// Substrings (case-insensitive) rejected in string leaves to reduce prompt-injection surface.
const BLOCKED_SUBSTRINGS: &[&str] = &[
    "ignore previous instructions",
    "ignore all previous",
    "disregard the above",
    "you are now in developer mode",
    "system override",
];

fn extract_string_leaves(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::String(s) => out.push(s.clone()),
        serde_json::Value::Array(a) => a.iter().for_each(|x| extract_string_leaves(x, out)),
        serde_json::Value::Object(o) => o.values().for_each(|x| extract_string_leaves(x, out)),
        _ => {}
    }
}

fn check_variable_blocklist(s: &str) -> Result<(), anyhow::Error> {
    let lower = s.to_lowercase();
    for pat in BLOCKED_SUBSTRINGS {
        if lower.contains(pat) {
            anyhow::bail!("variable content rejected (prompt injection policy)");
        }
    }
    Ok(())
}

/// Validate execute `variables` before Handlebars render (size + basic injection patterns).
pub fn validate_execute_variables(variables: &HashMap<String, serde_json::Value>) -> Result<(), anyhow::Error> {
    for (k, v) in variables {
        if k.len() > 256 {
            anyhow::bail!("variable name too long");
        }
        let ser = serde_json::to_vec(v)?;
        if ser.len() > MAX_VARIABLE_VALUE_BYTES {
            anyhow::bail!("variable value too large");
        }
        let mut leaves = Vec::new();
        extract_string_leaves(v, &mut leaves);
        for s in leaves {
            check_variable_blocklist(&s)?;
        }
    }
    Ok(())
}

/// Render prompt template with variables. Uses Handlebars syntax: {{name}}, {{#if}} etc.
pub fn render_prompt(
    template: &str,
    variables: &HashMap<String, serde_json::Value>,
) -> Result<String, anyhow::Error> {
    validate_execute_variables(variables)?;
    let mut reg = Handlebars::new();
    reg.register_template_string("prompt", template)?;
    let out = reg.render("prompt", variables)?;
    Ok(out)
}
