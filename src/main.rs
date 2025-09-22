//! Chatter - A terminal-based chat interface for Google's Gemini AI
//!
//! This CLI tool provides an interactive chat experience with Google's Gemini API,
//! supporting multi-turn conversations, streaming responses, and session management.

use anyhow::{anyhow, Result};
use clap::Parser;

mod agent;
mod api;
mod chat;
mod cli;
mod config;
mod templates;

use api::LlmClient;
use chat::ChatSession;
use cli::{Cli, Commands, TemplateAction};
use config::{Config, ModelProvider};
use templates::TemplateManager;

#[tokio::main]
async fn main() -> Result<()> {
    let mut cli = Cli::parse();

    if let Some(command) = cli.command.take() {
        match command {
            Commands::Config { action } => {
                handle_config_command(action).await?;
            }
            Commands::Query {
                message,
                model,
                provider,
                system,
                template,
            } => {
                // Load configuration (API key required for queries)
                let config = Config::load().await?;
                handle_query_command(message, model, provider, system, template, config).await?;
            }
            Commands::Template { action } => {
                handle_template_command(action).await?;
            }
        }
        return Ok(());
    }

    if let Some(message) = cli.prompt.take() {
        let config = Config::load().await?;
        handle_query_command(
            message,
            cli.model.clone(),
            cli.provider,
            cli.system.clone(),
            cli.template.clone(),
            config,
        )
        .await?;
        return Ok(());
    }

    // Load configuration (API key required for interactive chat)
    let config = Config::load().await?;
    handle_interactive_chat(cli, config).await?;
    Ok(())
}

/// Handle configuration commands
async fn handle_config_command(action: cli::ConfigAction) -> Result<()> {
    match action {
        cli::ConfigAction::SetApiKey => {
            // For setting API key, we don't require an existing API key
            let mut config = Config::load_with_api_key_required(false).await?;
            config.set_api_key_interactive().await?;
            println!("✅ API key configured successfully!");
        }
        cli::ConfigAction::Show => {
            // For showing config, we don't require an API key
            let config = Config::load_with_api_key_required(false).await?;
            config.display();
        }
        cli::ConfigAction::Reset => {
            // For resetting config, we don't require an API key
            let mut config = Config::load_with_api_key_required(false).await?;
            config.reset().await?;
            println!("✅ Configuration reset successfully!");
        }
    }
    Ok(())
}

/// Handle one-shot query commands
async fn handle_query_command(
    message: String,
    model: Option<String>,
    provider: Option<cli::ProviderArg>,
    system: Option<String>,
    template: Option<String>,
    config: Config,
) -> Result<()> {
    let provider = resolve_provider(provider, &config);
    let client = create_llm_client(&config, &provider)?;

    let model_name = model.unwrap_or_else(|| config.default_model.clone());

    // Resolve system instruction from template or direct input
    let system_instruction = resolve_system_instruction(system, template).await?;

    // Create a temporary chat session for the query
    let mut session = ChatSession::new(model_name, provider, system_instruction);

    // Send the message and display response
    let response = session.send_with_client(&client, &message).await?;
    println!("{response}");

    Ok(())
}

/// Handle interactive chat mode
async fn handle_interactive_chat(cli: Cli, config: Config) -> Result<()> {
    let provider = resolve_provider(cli.provider, &config);
    let client = create_llm_client(&config, &provider)?;

    // Determine model to use
    let model_override = cli.model.clone();
    let resolved_model = model_override
        .clone()
        .unwrap_or_else(|| config.default_model.clone());

    // Resolve system instruction from template or direct input
    let system_instruction = resolve_system_instruction(cli.system, cli.template).await?;

    // Create or load chat session
    let mut session = if let Some(session_file) = cli.load_session {
        let mut loaded = ChatSession::load_from_file(&session_file).await?;
        loaded.provider = provider.clone();
        if model_override.is_some() {
            loaded.model = resolved_model.clone();
        }
        loaded
    } else {
        ChatSession::new(
            resolved_model.clone(),
            provider.clone(),
            system_instruction.clone(),
        )
    };

    if let Some(instr) = system_instruction {
        session.system_instruction = Some(instr);
    }

    // Start interactive chat
    session
        .start_interactive_chat(&client, cli.auto_save, Some(config.sessions_dir.clone()))
        .await?;

    Ok(())
}

/// Handle template commands
async fn handle_template_command(action: TemplateAction) -> Result<()> {
    use colored::*;
    use dialoguer::{Confirm, Editor, Input};

    let mut manager = TemplateManager::new().await?;

    match action {
        TemplateAction::List { category, search } => {
            let templates = if let Some(search_query) = search {
                manager.search(&search_query)
            } else if let Some(cat) = category {
                manager.list_by_category(&cat)
            } else {
                manager.list_all()
            };

            if templates.is_empty() {
                println!("📭 No templates found");
                return Ok(());
            }

            println!("📋 Available Templates:");
            println!();

            // Group by category
            let mut by_category: std::collections::HashMap<String, Vec<_>> =
                std::collections::HashMap::new();
            for template in templates {
                by_category
                    .entry(template.category.clone())
                    .or_default()
                    .push(template);
            }

            for (cat, templates) in by_category {
                println!("{}", cat.bright_cyan().bold());
                for template in templates {
                    let builtin_marker = if template.builtin {
                        " (built-in)".bright_black()
                    } else {
                        "".normal()
                    };
                    println!(
                        "  {} - {}{}",
                        template.name.bright_green(),
                        template.description,
                        builtin_marker
                    );
                }
                println!();
            }
        }

        TemplateAction::Show { name } => {
            if let Some(template) = manager.get(&name) {
                println!("📄 Template: {}", template.name.bright_green().bold());
                println!("Description: {}", template.description);
                println!("Category: {}", template.category.bright_cyan());
                println!("Tags: {}", template.tags.join(", ").bright_yellow());
                println!(
                    "Built-in: {}",
                    if template.builtin {
                        "Yes".bright_green()
                    } else {
                        "No".bright_red()
                    }
                );
                println!(
                    "Created: {}",
                    template.created_at.format("%Y-%m-%d %H:%M:%S UTC")
                );
                println!(
                    "Updated: {}",
                    template.updated_at.format("%Y-%m-%d %H:%M:%S UTC")
                );
                println!();
                println!("Content:");
                println!("{}", "─".repeat(60).bright_black());
                println!("{}", template.content);
                println!("{}", "─".repeat(60).bright_black());
            } else {
                println!("❌ Template '{name}' not found");
            }
        }

        TemplateAction::Create {
            name,
            description,
            category,
        } => {
            // Get template details interactively
            let description = if let Some(desc) = description {
                desc
            } else {
                Input::new()
                    .with_prompt("Template description")
                    .interact()?
            };

            let category = if let Some(cat) = category {
                cat
            } else {
                let categories = manager.get_categories();
                if categories.is_empty() {
                    Input::new()
                        .with_prompt("Template category")
                        .default("general".to_string())
                        .interact()?
                } else {
                    println!("Existing categories: {}", categories.join(", "));
                    Input::new().with_prompt("Template category").interact()?
                }
            };

            // Get content via editor
            let content = if let Some(content) =
                Editor::new().edit("Enter the system instruction content:")?
            {
                content
            } else {
                return Err(anyhow::anyhow!("Template content is required"));
            };

            // Get tags
            let tags_input: String = Input::new()
                .with_prompt("Tags (comma-separated)")
                .default("".to_string())
                .interact()?;

            let tags: Vec<String> = tags_input
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let template =
                templates::Template::new(name.clone(), description, content, category, tags);

            manager.create(template).await?;
            println!("✅ Template '{name}' created successfully!");
        }

        TemplateAction::Edit { name } => {
            if let Some(existing) = manager.get(&name).cloned() {
                if existing.builtin {
                    println!("❌ Cannot edit built-in template '{name}'");
                    return Ok(());
                }

                // Edit description
                let description: String = Input::new()
                    .with_prompt("Template description")
                    .default(existing.description.clone())
                    .interact()?;

                // Edit content via editor
                let content = if let Some(content) = Editor::new().edit(&existing.content)? {
                    content
                } else {
                    existing.content.clone()
                };

                // Edit tags
                let current_tags = existing.tags.join(", ");
                let tags_input: String = Input::new()
                    .with_prompt("Tags (comma-separated)")
                    .default(current_tags)
                    .interact()?;

                let tags: Vec<String> = tags_input
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                let mut updated = existing.clone();
                updated.description = description;
                updated.content = content;
                updated.tags = tags;

                manager.update(&name, updated).await?;
                println!("✅ Template '{name}' updated successfully!");
            } else {
                println!("❌ Template '{name}' not found");
            }
        }

        TemplateAction::Delete { name, force } => {
            if let Some(template) = manager.get(&name) {
                if template.builtin {
                    println!("❌ Cannot delete built-in template '{name}'");
                    return Ok(());
                }

                let should_delete = if force {
                    true
                } else {
                    Confirm::new()
                        .with_prompt(format!("Delete template '{name}'?"))
                        .default(false)
                        .interact()?
                };

                if should_delete {
                    manager.delete(&name).await?;
                    println!("✅ Template '{name}' deleted successfully!");
                } else {
                    println!("❌ Template deletion cancelled");
                }
            } else {
                println!("❌ Template '{name}' not found");
            }
        }

        TemplateAction::Use {
            name,
            model,
            provider,
        } => {
            if let Some(template) = manager.get(&name) {
                // Load configuration (API key required for chat)
                let config = Config::load().await?;
                let provider = resolve_provider(provider, &config);
                let client = create_llm_client(&config, &provider)?;

                // Determine model to use
                let model_name = model.unwrap_or_else(|| config.default_model.clone());

                // Create chat session with template
                let mut session =
                    ChatSession::new(model_name, provider, Some(template.content.clone()));

                println!(
                    "🚀 Starting chat with template: {}",
                    template.name.bright_green()
                );
                println!("Description: {}", template.description);
                println!();

                // Start interactive chat
                session
                    .start_interactive_chat(&client, false, Some(config.sessions_dir.clone()))
                    .await?;
            } else {
                println!("❌ Template '{name}' not found");
            }
        }
    }

    Ok(())
}

fn resolve_provider(cli_provider: Option<cli::ProviderArg>, config: &Config) -> ModelProvider {
    cli_provider
        .map(|p| p.into())
        .unwrap_or_else(|| config.provider.clone())
}

fn create_llm_client(config: &Config, provider: &ModelProvider) -> Result<LlmClient> {
    match provider {
        ModelProvider::Gemini => {
            if config.api_key.trim().is_empty() {
                return Err(anyhow!(
                    "Gemini provider requires an API key. Run 'chatter config set-api-key'."
                ));
            }
            LlmClient::new_gemini(config.api_key.clone())
        }
        ModelProvider::Ollama => LlmClient::new_ollama(config.ollama.endpoint.clone()),
    }
}

/// Resolve system instruction from template name or direct input
async fn resolve_system_instruction(
    system: Option<String>,
    template: Option<String>,
) -> Result<Option<String>> {
    // Direct system instruction takes precedence
    if let Some(instruction) = system {
        return Ok(Some(instruction));
    }

    // Try to resolve template
    if let Some(template_name) = template {
        let manager = TemplateManager::new().await?;
        if let Some(template) = manager.get(&template_name) {
            return Ok(Some(template.content.clone()));
        } else {
            return Err(anyhow::anyhow!("Template '{}' not found", template_name));
        }
    }

    Ok(None)
}
