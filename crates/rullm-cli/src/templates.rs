use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::constants::TEMPLATES_DIR_NAME;

/// A template for LLM queries with placeholder support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    /// Name of the template
    pub name: String,
    /// System prompt/message (optional)
    pub system_prompt: Option<String>,
    /// User prompt template with {{placeholder}} syntax (optional)
    pub user_prompt: Option<String>,
    /// Default values for placeholders
    #[serde(default)]
    pub defaults: HashMap<String, String>,
    /// Description of the template (optional)
    pub description: Option<String>,
}

impl Template {
    /// Create a new template providing a user prompt.
    /// If you need a template that only contains a system prompt, construct it manually
    /// via `Template { .. }` or add a helper if needed.
    pub fn new(name: String, user_prompt: String) -> Self {
        Self {
            name,
            system_prompt: None,
            user_prompt: Some(user_prompt),
            defaults: HashMap::new(),
            description: None,
        }
    }

    /// Render the template by replacing placeholders with provided values
    /// Returns an error if any required placeholders are missing
    pub fn render(&self, params: &HashMap<String, String>) -> Result<RenderedTemplate> {
        // Ensure we have at least one prompt defined
        if self.user_prompt.is_none() && self.system_prompt.is_none() {
            return Err(anyhow::anyhow!(
                "Template must have at least a user_prompt or system_prompt"
            ));
        }

        let mut rendered_user = self.user_prompt.clone();
        let mut rendered_system = self.system_prompt.clone();
        let mut missing_placeholders = Vec::new();

        // Extract placeholders from user prompt if it exists
        let user_placeholders = if let Some(ref user) = self.user_prompt {
            extract_placeholders(user)
        } else {
            Vec::new()
        };

        // Extract placeholders from system prompt if it exists
        let system_placeholders = if let Some(ref system) = self.system_prompt {
            extract_placeholders(system)
        } else {
            Vec::new()
        };

        // Combine all unique placeholders
        let mut all_placeholders = user_placeholders;
        for ph in system_placeholders {
            if !all_placeholders.contains(&ph) {
                all_placeholders.push(ph);
            }
        }

        // Replace placeholders
        for placeholder in &all_placeholders {
            let value = params
                .get(placeholder)
                .or_else(|| self.defaults.get(placeholder));

            match value {
                Some(val) => {
                    let pattern = format!("{{{{{placeholder}}}}}");
                    if let Some(ref mut user) = rendered_user {
                        *user = user.replace(&pattern, val);
                    }
                    if let Some(ref mut system) = rendered_system {
                        *system = system.replace(&pattern, val);
                    }
                }
                None => {
                    missing_placeholders.push(placeholder.clone());
                }
            }
        }

        if !missing_placeholders.is_empty() {
            return Err(anyhow::anyhow!(
                "Missing required placeholders: {}",
                missing_placeholders.join(", ")
            ));
        }

        Ok(RenderedTemplate {
            system_prompt: rendered_system,
            user_prompt: rendered_user,
        })
    }

    /// Get all placeholders required by this template
    #[allow(dead_code)]
    pub fn get_placeholders(&self) -> Vec<String> {
        let mut placeholders = if let Some(ref user) = self.user_prompt {
            extract_placeholders(user)
        } else {
            Vec::new()
        };

        if let Some(ref system) = self.system_prompt {
            let system_placeholders = extract_placeholders(system);
            for ph in system_placeholders {
                if !placeholders.contains(&ph) {
                    placeholders.push(ph);
                }
            }
        }

        placeholders
    }

    /// Simplified rendering that only supports a single `{{input}}` placeholder.
    /// This method constructs the minimal parameter map with the provided input
    /// and delegates to `render`. All other placeholder parameters are no
    /// longer supported.
    pub fn render_input(&self, input: &str) -> Result<RenderedTemplate> {
        let mut params = HashMap::new();
        params.insert("input".to_string(), input.to_string());
        self.render(&params)
    }
}

/// A rendered template ready for use
#[derive(Debug)]
pub struct RenderedTemplate {
    pub system_prompt: Option<String>,
    pub user_prompt: Option<String>,
}

/// Store for managing templates
pub struct TemplateStore {
    templates_dir: PathBuf,
    templates: HashMap<String, Template>,
}

impl TemplateStore {
    /// Create a new TemplateStore with the given base directory
    pub fn new(base_path: &Path) -> Self {
        let templates_dir = base_path.join(TEMPLATES_DIR_NAME);
        Self {
            templates_dir,
            templates: HashMap::new(),
        }
    }

    /// Load all templates from the templates directory
    pub fn load(&mut self) -> Result<()> {
        self.templates.clear();

        // Create templates directory if it doesn't exist
        if !self.templates_dir.exists() {
            fs::create_dir_all(&self.templates_dir)
                .context("Failed to create templates directory")?;
            return Ok(());
        }

        // Read all .toml files in the templates directory
        let entries =
            fs::read_dir(&self.templates_dir).context("Failed to read templates directory")?;

        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                match self.load_template_file(&path) {
                    Ok(template) => {
                        self.templates.insert(template.name.clone(), template);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to load template {path:?}: {e}");
                    }
                }
            }
        }

        Ok(())
    }

    /// Load a single template file
    fn load_template_file(&self, path: &Path) -> Result<Template> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read template file: {path:?}"))?;

        let template: Template = toml::from_str(&content)
            .with_context(|| format!("Failed to parse template file: {path:?}"))?;

        Ok(template)
    }

    /// Save a template to disk
    pub fn save(&mut self, template: &Template) -> Result<()> {
        // Update in-memory map so caller sees it immediately
        self.templates
            .insert(template.name.clone(), template.clone());

        if !self.templates_dir.exists() {
            fs::create_dir_all(&self.templates_dir)
                .context("Failed to create templates directory")?;
        }

        let file_path = self.templates_dir.join(format!("{}.toml", template.name));
        let temp_path = file_path.with_extension("toml.tmp");

        // Serialize to TOML
        let content =
            toml::to_string_pretty(template).context("Failed to serialize template to TOML")?;

        // Atomic write: write to temp file then rename
        fs::write(&temp_path, content)
            .with_context(|| format!("Failed to write temporary template file: {temp_path:?}"))?;

        fs::rename(&temp_path, &file_path)
            .with_context(|| format!("Failed to rename template file: {file_path:?}"))?;

        Ok(())
    }

    /// Delete a template file from disk. Returns true if deleted.
    pub fn delete(&mut self, name: &str) -> Result<bool> {
        let file_path = self.templates_dir.join(format!("{name}.toml"));

        if !file_path.exists() {
            // Nothing to delete
            return Ok(false);
        }

        std::fs::remove_file(&file_path)
            .with_context(|| format!("Failed to delete template file: {file_path:?}"))?;

        // Also remove from in-memory cache if present
        self.templates.remove(name);

        Ok(true)
    }

    /// Get a template by name
    pub fn get(&self, name: &str) -> Option<&Template> {
        self.templates.get(name)
    }

    /// List all template names
    pub fn list(&self) -> Vec<String> {
        self.templates.keys().cloned().collect()
    }

    /// Check if a template exists
    pub fn contains(&self, name: &str) -> bool {
        self.templates.contains_key(name)
    }

    /// Get the templates directory path
    #[allow(dead_code)]
    pub fn templates_dir(&self) -> &Path {
        &self.templates_dir
    }
}

/// Extract placeholder names from a template string
/// Finds all occurrences of {{placeholder}} and returns the placeholder names
fn extract_placeholders(template: &str) -> Vec<String> {
    let mut placeholders = Vec::new();
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() == Some(&'{') {
            chars.next(); // consume second '{'

            let mut placeholder = String::new();
            let mut found_closing = false;

            while let Some(ch) = chars.next() {
                if ch == '}' && chars.peek() == Some(&'}') {
                    chars.next(); // consume second '}'
                    found_closing = true;
                    break;
                } else if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                    placeholder.push(ch);
                } else {
                    // Invalid character in placeholder, skip
                    break;
                }
            }

            if found_closing && !placeholder.is_empty() && !placeholders.contains(&placeholder) {
                placeholders.push(placeholder);
            }
        }
    }

    placeholders
}

/// Resolves a template (by name or @file) and renders it with the user query.
pub fn resolve_template_prompts(
    template_name: &str,
    user_query: &str,
    config_base_path: &std::path::Path,
) -> anyhow::Result<(Option<String>, String)> {
    if template_name.starts_with('@') {
        // Ad-hoc template from file
        let path = template_name.trim_start_matches('@');
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read template file '{}': {}", path, e))?;
        let template: Template = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse template file '{}': {}", path, e))?;
        let rendered = template
            .render_input(user_query)
            .map_err(|e| anyhow::anyhow!("Failed to render template from '{}': {}", path, e))?;
        let final_query = rendered
            .user_prompt
            .unwrap_or_else(|| user_query.to_string());
        Ok((rendered.system_prompt, final_query))
    } else {
        // Load template store
        let mut template_store = TemplateStore::new(config_base_path);
        template_store
            .load()
            .map_err(|e| anyhow::anyhow!("Failed to load templates: {}", e))?;

        // Get the template
        let template = template_store
            .get(template_name)
            .ok_or_else(|| anyhow::anyhow!("Template '{}' not found", template_name))?;

        // Render the template using the user input as the only parameter
        let rendered = template
            .render_input(user_query)
            .map_err(|e| anyhow::anyhow!("Failed to render template '{}': {}", template_name, e))?;

        let final_query = rendered
            .user_prompt
            .unwrap_or_else(|| user_query.to_string());
        Ok((rendered.system_prompt, final_query))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_extract_placeholders() {
        assert_eq!(extract_placeholders("Hello {{name}}!"), vec!["name"]);
        assert_eq!(
            extract_placeholders("{{greeting}} {{name}}, how is {{weather}}?"),
            vec!["greeting", "name", "weather"]
        );
        assert_eq!(
            extract_placeholders("No placeholders here"),
            Vec::<String>::new()
        );
        assert_eq!(extract_placeholders("{{}}"), Vec::<String>::new());
        assert_eq!(
            extract_placeholders("{{name}} and {{name}} again"),
            vec!["name"]
        );
    }

    #[test]
    fn test_template_render() {
        let template = Template {
            name: "test".to_string(),
            system_prompt: Some("You are a {{role}}".to_string()),
            user_prompt: Some("Hello {{name}}, the weather is {{weather}}".to_string()),
            defaults: [("weather".to_string(), "sunny".to_string())].into(),
            description: None,
        };

        let mut params = HashMap::new();
        params.insert("role".to_string(), "assistant".to_string());
        params.insert("name".to_string(), "Alice".to_string());

        let rendered = template.render(&params).unwrap();
        assert_eq!(
            rendered.system_prompt,
            Some("You are a assistant".to_string())
        );
        assert_eq!(
            rendered.user_prompt,
            Some("Hello Alice, the weather is sunny".to_string())
        );
    }

    #[test]
    fn test_template_render_missing_placeholder() {
        let template = Template {
            name: "test".to_string(),
            system_prompt: None,
            user_prompt: Some("Hello {{name}}".to_string()),
            defaults: HashMap::new(),
            description: None,
        };

        let params = HashMap::new();
        let result = template.render(&params);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing required placeholders: name")
        );
    }

    #[test]
    fn test_template_store() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = TemplateStore::new(temp_dir.path());

        // Load should succeed even with empty directory
        store.load().unwrap();
        assert_eq!(store.list().len(), 0);

        // Save and reload
        let template = Template::new("test".to_string(), "Hello {{name}}".to_string());
        store.save(&template).unwrap();
        store.load().unwrap();

        assert_eq!(store.list().len(), 1);
        assert!(store.contains("test"));

        let loaded = store.get("test").unwrap();
        assert_eq!(loaded.name, "test");
        assert_eq!(loaded.user_prompt, Some("Hello {{name}}".to_string()));
    }

    #[test]
    fn test_template_with_defaults() {
        let mut template = Template::new(
            "greeting".to_string(),
            "Hello {{name}}, {{greeting}}!".to_string(),
        );
        template
            .defaults
            .insert("greeting".to_string(), "how are you".to_string());

        let mut params = HashMap::new();
        params.insert("name".to_string(), "Alice".to_string());

        let rendered = template.render(&params).unwrap();
        assert_eq!(
            rendered.user_prompt,
            Some("Hello Alice, how are you!".to_string())
        );
    }

    #[test]
    fn test_template_param_override_defaults() {
        let mut template = Template::new(
            "greeting".to_string(),
            "Hello {{name}}, {{greeting}}!".to_string(),
        );
        template
            .defaults
            .insert("greeting".to_string(), "how are you".to_string());

        let mut params = HashMap::new();
        params.insert("name".to_string(), "Bob".to_string());
        params.insert("greeting".to_string(), "welcome".to_string()); // Override default

        let rendered = template.render(&params).unwrap();
        assert_eq!(
            rendered.user_prompt,
            Some("Hello Bob, welcome!".to_string())
        );
    }

    #[test]
    fn test_template_with_system_prompt() {
        let template = Template {
            name: "assistant".to_string(),
            system_prompt: Some("You are a helpful {{role}} assistant. Be {{tone}}.".to_string()),
            user_prompt: Some("Help me with {{task}}".to_string()),
            defaults: [("tone".to_string(), "professional".to_string())].into(),
            description: Some("Assistant template".to_string()),
        };

        let mut params = HashMap::new();
        params.insert("role".to_string(), "coding".to_string());
        params.insert("task".to_string(), "debugging".to_string());

        let rendered = template.render(&params).unwrap();
        assert_eq!(
            rendered.system_prompt,
            Some("You are a helpful coding assistant. Be professional.".to_string())
        );
        assert_eq!(
            rendered.user_prompt,
            Some("Help me with debugging".to_string())
        );
    }

    #[test]
    fn test_extract_placeholders_edge_cases() {
        // Malformed placeholders should be ignored
        assert_eq!(extract_placeholders("{{"), Vec::<String>::new());
        assert_eq!(extract_placeholders("}}"), Vec::<String>::new());
        assert_eq!(extract_placeholders("{single}"), Vec::<String>::new());
        assert_eq!(
            extract_placeholders("{{invalid char}}"),
            Vec::<String>::new()
        );
        assert_eq!(
            extract_placeholders("{{invalid-space }}"),
            Vec::<String>::new()
        );

        // Valid placeholders with underscores and hyphens
        assert_eq!(extract_placeholders("{{valid_name}}"), vec!["valid_name"]);
        assert_eq!(extract_placeholders("{{valid-name}}"), vec!["valid-name"]);
        assert_eq!(extract_placeholders("{{name123}}"), vec!["name123"]);

        // Mixed valid and invalid
        assert_eq!(
            extract_placeholders("{{valid}} and {{invalid char}} and {{also_valid}}"),
            vec!["valid", "also_valid"]
        );
    }

    #[test]
    fn test_template_get_placeholders() {
        let template = Template {
            name: "test".to_string(),
            system_prompt: Some("System: {{role}} with {{mood}}".to_string()),
            user_prompt: Some("User: {{input}} for {{task}}".to_string()),
            defaults: HashMap::new(),
            description: None,
        };

        let placeholders = template.get_placeholders();
        let mut sorted_placeholders = placeholders;
        sorted_placeholders.sort();
        assert_eq!(sorted_placeholders, vec!["input", "mood", "role", "task"]);
    }

    #[test]
    fn test_template_store_multiple_templates() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = TemplateStore::new(temp_dir.path());

        // Create multiple templates
        let template1 = Template::new("first".to_string(), "First: {{value}}".to_string());
        let template2 = Template::new("second".to_string(), "Second: {{other}}".to_string());

        store.save(&template1).unwrap();
        store.save(&template2).unwrap();
        store.load().unwrap();

        assert_eq!(store.list().len(), 2);
        assert!(store.contains("first"));
        assert!(store.contains("second"));
        assert!(!store.contains("nonexistent"));

        let names = store.list();
        assert!(names.contains(&"first".to_string()));
        assert!(names.contains(&"second".to_string()));
    }

    #[test]
    fn test_template_store_invalid_toml_files() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = TemplateStore::new(temp_dir.path());

        // Create a valid template file
        let template = Template::new("valid".to_string(), "Valid: {{test}}".to_string());
        store.save(&template).unwrap();

        // Create an invalid TOML file manually
        let invalid_path = temp_dir.path().join("invalid.toml");
        std::fs::write(&invalid_path, "invalid toml content [[[").unwrap();

        // Loading should succeed but skip the invalid file (with warning)
        store.load().unwrap();
        assert_eq!(store.list().len(), 1);
        assert!(store.contains("valid"));
        assert!(!store.contains("invalid"));
    }

    #[test]
    fn test_template_rendering_empty_strings() {
        let template = Template {
            name: "test".to_string(),
            system_prompt: Some("".to_string()),
            user_prompt: Some("Hello {{name}}".to_string()),
            defaults: HashMap::new(),
            description: None,
        };

        let mut params = HashMap::new();
        params.insert("name".to_string(), "".to_string());

        let rendered = template.render(&params).unwrap();
        assert_eq!(rendered.system_prompt, Some("".to_string()));
        assert_eq!(rendered.user_prompt, Some("Hello ".to_string()));
    }

    #[test]
    fn test_template_multiple_missing_placeholders() {
        let template = Template {
            name: "test".to_string(),
            system_prompt: Some("System {{a}} {{b}}".to_string()),
            user_prompt: Some("User {{c}} {{d}}".to_string()),
            defaults: HashMap::new(),
            description: None,
        };

        let params = HashMap::new();
        let result = template.render(&params);
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Missing required placeholders"));
        // Should contain all missing placeholders
        assert!(error_msg.contains("a"));
        assert!(error_msg.contains("b"));
        assert!(error_msg.contains("c"));
        assert!(error_msg.contains("d"));
    }

    #[test]
    fn test_template_store_templates_dir() {
        let temp_dir = TempDir::new().unwrap();
        let store = TemplateStore::new(temp_dir.path());

        assert_eq!(store.templates_dir(), temp_dir.path().join("templates"));
    }

    #[test]
    fn test_template_complex_placeholders() {
        let template_str = "Start {{first}} middle {{second}} {{first}} end {{third}}";
        let placeholders = extract_placeholders(template_str);

        // Should deduplicate placeholders
        assert_eq!(placeholders.len(), 3);
        assert!(placeholders.contains(&"first".to_string()));
        assert!(placeholders.contains(&"second".to_string()));
        assert!(placeholders.contains(&"third".to_string()));
    }
}
