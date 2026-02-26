//! Handlebars-style string interpolation for prompt variables.

use handlebars::Handlebars;
use std::collections::HashMap;

/// Render prompt template with variables. Uses Handlebars syntax: {{name}}, {{#if}} etc.
pub fn render_prompt(
    template: &str,
    variables: &HashMap<String, serde_json::Value>,
) -> Result<String, anyhow::Error> {
    let mut reg = Handlebars::new();
    reg.register_template_string("prompt", template)?;
    let out = reg.render("prompt", variables)?;
    Ok(out)
}
