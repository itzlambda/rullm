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
    /// User prompt template with {{placeholder}} syntax
    pub user_prompt: String,
    /// Default values for placeholders
    #[serde(default)]
    pub defaults: HashMap<String, String>,
    /// Description of the template (optional)
    pub description: Option<String>,
}

impl Template {
    pub fn new(name: String, user_prompt: String) -> Self {
        Self {
            name,
            system_prompt: None,
            user_prompt,
            defaults: HashMap::new(),
            description: None,
        }
    }

    /// Render the template by replacing placeholders with provided values
    /// Returns an error if any required placeholders are missing
    pub fn render(&self, params: &HashMap<String, String>) -> Result<RenderedTemplate> {
        let mut rendered_user = self.user_prompt.clone();
        let mut rendered_system = self.system_prompt.clone();
        let mut missing_placeholders = Vec::new();

        // Extract all placeholders from user prompt
        let user_placeholders = extract_placeholders(&self.user_prompt);

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
                    let pattern = format!("{{{{{}}}}}", placeholder);
                    rendered_user = rendered_user.replace(&pattern, val);
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
    pub fn get_placeholders(&self) -> Vec<String> {
        let mut placeholders = extract_placeholders(&self.user_prompt);

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
}

/// A rendered template ready for use
#[derive(Debug)]
pub struct RenderedTemplate {
    pub system_prompt: Option<String>,
    pub user_prompt: String,
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
                        eprintln!("Warning: Failed to load template {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Load a single template file
    fn load_template_file(&self, path: &Path) -> Result<Template> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read template file: {:?}", path))?;

        let template: Template = toml::from_str(&content)
            .with_context(|| format!("Failed to parse template file: {:?}", path))?;

        Ok(template)
    }

    /// Save a template to disk
    pub fn save(&self, template: &Template) -> Result<()> {
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
            .with_context(|| format!("Failed to write temporary template file: {:?}", temp_path))?;

        fs::rename(&temp_path, &file_path)
            .with_context(|| format!("Failed to rename template file: {:?}", file_path))?;

        Ok(())
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
            user_prompt: "Hello {{name}}, the weather is {{weather}}".to_string(),
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
        assert_eq!(rendered.user_prompt, "Hello Alice, the weather is sunny");
    }

    #[test]
    fn test_template_render_missing_placeholder() {
        let template = Template {
            name: "test".to_string(),
            system_prompt: None,
            user_prompt: "Hello {{name}}".to_string(),
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
        assert_eq!(loaded.user_prompt, "Hello {{name}}");
    }
}
