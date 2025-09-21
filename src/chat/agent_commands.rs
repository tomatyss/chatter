//! Agent-specific commands for the chat interface
//!
//! Provides commands to control and interact with the agent mode.

use crate::agent::{Agent, AgentConfig, CompletionStatus};
use anyhow::Result;
use colored::*;
use std::path::PathBuf;

/// Handle agent-specific commands in the chat interface
pub async fn handle_agent_command(
    command: &str,
    args: &str,
    agent: &mut Option<Agent>,
) -> Result<()> {
    match command {
        "/agent" => {
            match args.trim() {
                "on" | "enable" => {
                    if agent.is_none() {
                        let config = AgentConfig::default();
                        let mut new_agent = Agent::new(config)?;
                        new_agent.set_enabled(true);
                        *agent = Some(new_agent);
                        println!("🤖 {} Agent mode enabled! I can now use tools to help with file operations.", "AGENT:".bright_green().bold());
                        println!(
                            "   Available tools: {}",
                            agent.as_ref().unwrap().available_tools().join(", ")
                        );
                    } else if let Some(ref mut agent) = agent {
                        agent.set_enabled(true);
                        println!(
                            "🤖 {} Agent mode re-enabled!",
                            "AGENT:".bright_green().bold()
                        );
                    }
                }
                "off" | "disable" => {
                    if let Some(ref mut agent) = agent {
                        agent.set_enabled(false);
                        println!(
                            "🤖 {} Agent mode disabled.",
                            "AGENT:".bright_yellow().bold()
                        );
                    } else {
                        println!("❌ Agent mode is not initialized.");
                    }
                }
                "status" => {
                    if let Some(ref agent) = agent {
                        let status = agent.status();
                        println!("🤖 {} Agent Status:", "AGENT:".bright_cyan().bold());
                        println!(
                            "   Enabled: {}",
                            if status.enabled {
                                "Yes".bright_green()
                            } else {
                                "No".bright_red()
                            }
                        );
                        println!("   Tools executed: {}", status.tools_executed);
                        println!(
                            "   Working directory: {}",
                            status.working_directory.display()
                        );
                        println!(
                            "   Dry run mode: {}",
                            if status.dry_run_mode {
                                "Yes".bright_yellow()
                            } else {
                                "No".bright_green()
                            }
                        );
                        println!("   Available tools: {}", status.available_tools.join(", "));
                    } else {
                        println!("❌ Agent mode is not initialized.");
                    }
                }
                args if args.starts_with("dry-run") => {
                    if let Some(ref mut agent) = agent {
                        let parts: Vec<&str> = args.split_whitespace().collect();
                        if parts.len() < 2 {
                            println!("Usage: /agent dry-run <on|off>");
                        } else {
                            let mut cfg = agent.config().clone();
                            match parts[1] {
                                "on" => {
                                    cfg.dry_run_mode = true;
                                    if let Err(e) = agent.update_config(cfg) {
                                        println!("❌ Failed to enable dry-run: {e}");
                                    } else {
                                        println!("🧪 {} Dry-run mode enabled. No changes will be written.", "AGENT:".bright_yellow().bold());
                                    }
                                }
                                "off" => {
                                    cfg.dry_run_mode = false;
                                    if let Err(e) = agent.update_config(cfg) {
                                        println!("❌ Failed to disable dry-run: {e}");
                                    } else {
                                        println!(
                                            "✅ {} Dry-run mode disabled.",
                                            "AGENT:".bright_green().bold()
                                        );
                                    }
                                }
                                _ => println!("Usage: /agent dry-run <on|off>"),
                            }
                        }
                    } else {
                        println!("❌ Agent mode is not initialized.");
                    }
                }
                "history" => {
                    if let Some(ref agent) = agent {
                        let history = agent.tool_history();
                        if history.is_empty() {
                            println!("📭 No tool execution history.");
                        } else {
                            println!(
                                "🤖 {} Tool Execution History:",
                                "AGENT:".bright_cyan().bold()
                            );
                            for (i, tool_call) in history.iter().enumerate() {
                                println!(
                                    "   {}. {} {}",
                                    i + 1,
                                    tool_call.tool.bright_yellow(),
                                    format!("({})", tool_call.parameters.len()).bright_black()
                                );
                                if let Some(ref thought) = tool_call.thought {
                                    println!("      💭 {}", thought.bright_white());
                                }
                            }
                        }
                    } else {
                        println!("❌ Agent mode is not initialized.");
                    }
                }
                "clear" => {
                    if let Some(ref mut agent) = agent {
                        agent.clear_history();
                        println!(
                            "🤖 {} Tool execution history cleared.",
                            "AGENT:".bright_green().bold()
                        );
                    } else {
                        println!("❌ Agent mode is not initialized.");
                    }
                }
                "tools" => {
                    if let Some(ref agent) = agent {
                        let catalog = agent.tool_catalog();
                        println!("🤖 {} Available Tools:", "AGENT:".bright_cyan().bold());
                        for entry in catalog {
                            println!("\n{}", entry);
                        }
                    } else {
                        println!("❌ Agent mode is not initialized.");
                    }
                }
                "config" => {
                    if let Some(ref agent) = agent {
                        let config = agent.config();
                        println!("🤖 {} Agent Configuration:", "AGENT:".bright_cyan().bold());
                        println!(
                            "   Enabled: {}",
                            if config.enabled {
                                "Yes".bright_green()
                            } else {
                                "No".bright_red()
                            }
                        );
                        println!("   Max file size: {} bytes", config.max_file_size);
                        println!(
                            "   Working directory: {}",
                            config.working_directory.display()
                        );
                        println!(
                            "   Auto backup: {}",
                            if config.auto_backup {
                                "Yes".bright_green()
                            } else {
                                "No".bright_red()
                            }
                        );
                        println!(
                            "   Dry run mode: {}",
                            if config.dry_run_mode {
                                "Yes".bright_yellow()
                            } else {
                                "No".bright_green()
                            }
                        );
                        println!(
                            "   Allowed extensions: {}",
                            config.allowed_extensions.join(", ")
                        );

                        let allowed_paths = agent.allowed_paths();
                        if !allowed_paths.is_empty() {
                            println!("   Allowed paths:");
                            for path in allowed_paths {
                                println!("      • {}", path.display());
                            }
                        }

                        let forbidden_paths = agent.forbidden_paths();
                        if !forbidden_paths.is_empty() {
                            println!("   Forbidden paths:");
                            for path in forbidden_paths {
                                println!("      • {}", path.display());
                            }
                        }
                    } else {
                        println!("❌ Agent mode is not initialized.");
                    }
                }
                args if args.starts_with("allow-path") => {
                    if let Some(ref mut agent) = agent {
                        let path = args["allow-path".len()..].trim();
                        if path.is_empty() {
                            println!("Usage: /agent allow-path <path>");
                        } else {
                            agent.add_allowed_path(PathBuf::from(path));
                            println!("🛡️  Added allowed path: {}", path.bright_green());
                        }
                    } else {
                        println!("❌ Agent mode is not initialized.");
                    }
                }
                args if args.starts_with("forbid-path") => {
                    if let Some(ref mut agent) = agent {
                        let path = args["forbid-path".len()..].trim();
                        if path.is_empty() {
                            println!("Usage: /agent forbid-path <path>");
                        } else {
                            agent.add_forbidden_path(PathBuf::from(path));
                            println!("🚫 Added forbidden path: {}", path.bright_red());
                        }
                    } else {
                        println!("❌ Agent mode is not initialized.");
                    }
                }
                args if args.starts_with("check-path") => {
                    if let Some(ref agent) = agent {
                        let path = args["check-path".len()..].trim();
                        if path.is_empty() {
                            println!("Usage: /agent check-path <path>");
                        } else {
                            let allowed = agent.is_path_allowed(path);
                            if allowed {
                                println!(
                                    "✅ Path '{}' is permitted by the safety manager.",
                                    path.bright_green()
                                );
                            } else {
                                println!(
                                    "⚠️  Path '{}' would be blocked by safety rules.",
                                    path.bright_red()
                                );
                            }
                        }
                    } else {
                        println!("❌ Agent mode is not initialized.");
                    }
                }
                "help" => {
                    display_agent_help();
                }
                _ => {
                    println!("❌ Unknown agent command. Use '/agent help' for available commands.");
                }
            }
        }
        _ => {
            println!("❌ Unknown agent command: {command}");
        }
    }

    Ok(())
}

/// Display help for agent commands
fn display_agent_help() {
    println!("🤖 {} Agent Commands:", "AGENT:".bright_cyan().bold());
    println!("   {} - Enable agent mode", "/agent on".bright_green());
    println!("   {} - Disable agent mode", "/agent off".bright_yellow());
    println!("   {} - Show agent status", "/agent status".bright_blue());
    println!(
        "   {} - Show tool execution history",
        "/agent history".bright_blue()
    );
    println!(
        "   {} - Clear tool execution history",
        "/agent clear".bright_red()
    );
    println!(
        "   {} - List available tools and schemas",
        "/agent tools".bright_blue()
    );
    println!(
        "   {} - Show agent configuration",
        "/agent config".bright_blue()
    );
    println!(
        "   {} - Toggle dry-run mode (no writes)",
        "/agent dry-run <on|off>".bright_blue()
    );
    println!(
        "   {} - Allow an extra path for tool access",
        "/agent allow-path <path>".bright_blue()
    );
    println!(
        "   {} - Forbid a specific path",
        "/agent forbid-path <path>".bright_blue()
    );
    println!(
        "   {} - Check whether a path is allowed",
        "/agent check-path <path>".bright_blue()
    );
    println!("   {} - Show this help", "/agent help".bright_white());
    println!();
    println!(
        "💡 {} When agent mode is enabled, I can automatically detect and execute",
        "TIP:".bright_yellow().bold()
    );
    println!("   tool requests in your messages. For example:");
    println!("   • \"Please read the file config.json\"");
    println!("   • \"Search for 'TODO' in all Rust files\"");
    println!("   • \"List all files in the src directory\"");
}

/// Check if a message contains agent tool requests and execute them
pub async fn process_agent_tools(
    message: &str,
    agent: &mut Option<Agent>,
) -> Result<Option<String>> {
    if let Some(ref mut agent) = agent {
        if !agent.is_enabled() {
            return Ok(None);
        }

        // Detect tool calls in the message
        let tool_calls = agent.detect_tool_calls(message)?;

        if tool_calls.is_empty() {
            return Ok(None);
        }

        let mut results = Vec::new();

        for tool_call in tool_calls {
            println!(
                "🔧 {} Executing tool: {}",
                "AGENT:".bright_green().bold(),
                tool_call.tool.bright_yellow()
            );

            if let Some(ref thought) = tool_call.thought {
                println!("   💭 {}", thought.bright_white());
            }

            match agent.execute_tool(tool_call.clone()).await {
                Ok(result) => {
                    if result.success {
                        if let Some(ref message) = result.message {
                            println!("   ✅ {}", message.bright_green());
                        }

                        // Format the result for display
                        let formatted_result = format_tool_result(&tool_call.tool, &result);
                        results.push(formatted_result);
                    } else {
                        let error_msg = result
                            .message
                            .unwrap_or_else(|| "Unknown error".to_string());
                        println!("   ❌ {}", error_msg.bright_red());
                        results.push(format!("Tool {} failed: {}", tool_call.tool, error_msg));
                    }
                }
                Err(e) => {
                    println!(
                        "   ❌ {}",
                        format!("Tool execution error: {e}").bright_red()
                    );
                    results.push(format!("Tool {} error: {}", tool_call.tool, e));
                }
            }
        }

        if !results.is_empty() {
            let combined_result = results.join("\n\n");
            return Ok(Some(combined_result));
        }
    }

    Ok(None)
}

/// Format tool execution results for display
fn format_tool_result(tool_name: &str, result: &crate::agent::ToolResult) -> String {
    match tool_name {
        "read_file" => {
            if let Some(content) = result.data.get("content").and_then(|c| c.as_str()) {
                let path = result
                    .data
                    .get("path")
                    .and_then(|p| p.as_str())
                    .unwrap_or("unknown");
                let size = result
                    .data
                    .get("size")
                    .and_then(|s| s.as_u64())
                    .unwrap_or(0);

                format!("📄 **File: {path}** ({size} bytes)\n```\n{content}\n```")
            } else {
                "File read completed".to_string()
            }
        }
        "write_file" => {
            let path = result
                .data
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or("unknown");
            let size = result
                .data
                .get("size")
                .and_then(|s| s.as_u64())
                .unwrap_or(0);
            format!("💾 **File written:** {path} ({size} bytes)")
        }
        "update_file" => {
            let path = result
                .data
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or("unknown");
            let operation = result
                .data
                .get("operation")
                .and_then(|o| o.as_str())
                .unwrap_or("unknown");
            format!("✏️ **File updated:** {path} (operation: {operation})")
        }
        "search_files" => {
            let pattern = result
                .data
                .get("pattern")
                .and_then(|p| p.as_str())
                .unwrap_or("unknown");
            let matches_found = result
                .data
                .get("matches_found")
                .and_then(|m| m.as_u64())
                .unwrap_or(0);
            let files_searched = result
                .data
                .get("files_searched")
                .and_then(|f| f.as_u64())
                .unwrap_or(0);

            let mut output = format!("🔍 **Search results for '{pattern}':** {matches_found} matches in {files_searched} files");

            if let Some(results) = result.data.get("results").and_then(|r| r.as_array()) {
                if !results.is_empty() {
                    output.push_str("\n\n**Matches:**");
                    for (i, match_result) in results.iter().take(10).enumerate() {
                        if let (Some(file), Some(line), Some(content)) = (
                            match_result.get("file").and_then(|f| f.as_str()),
                            match_result.get("line").and_then(|l| l.as_u64()),
                            match_result.get("content").and_then(|c| c.as_str()),
                        ) {
                            output.push_str(&format!(
                                "\n{}. **{}:{}** `{}`",
                                i + 1,
                                file,
                                line,
                                content
                            ));
                        }
                    }
                    if results.len() > 10 {
                        output.push_str(&format!("\n... and {} more matches", results.len() - 10));
                    }
                }
            }

            output
        }
        "list_directory" => {
            let path = result
                .data
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or("unknown");
            let entry_count = result
                .data
                .get("entry_count")
                .and_then(|e| e.as_u64())
                .unwrap_or(0);

            let mut output =
                format!("📁 **Directory listing for '{path}':** {entry_count} entries");

            if let Some(entries) = result.data.get("entries").and_then(|e| e.as_array()) {
                if !entries.is_empty() {
                    output.push_str("\n\n**Contents:**");
                    for entry in entries.iter().take(20) {
                        if let (Some(name), Some(entry_type)) = (
                            entry.get("name").and_then(|n| n.as_str()),
                            entry.get("type").and_then(|t| t.as_str()),
                        ) {
                            let icon = if entry_type == "directory" {
                                "📁"
                            } else {
                                "📄"
                            };
                            output.push_str(&format!("\n{icon} {name}"));
                        }
                    }
                    if entries.len() > 20 {
                        output.push_str(&format!("\n... and {} more entries", entries.len() - 20));
                    }
                }
            }

            output
        }
        "file_info" => {
            let path = result
                .data
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or("unknown");
            let file_type = result
                .data
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown");
            let size = result
                .data
                .get("size")
                .and_then(|s| s.as_u64())
                .unwrap_or(0);

            format!("ℹ️ **File info for '{path}':** {path} ({size} bytes, type: {file_type})")
        }
        _ => result
            .message
            .clone()
            .unwrap_or_else(|| "Tool executed successfully".to_string()),
    }
}

/// Check if the current task appears to be complete based on recent messages
pub fn check_task_completion(
    recent_messages: &[String],
    agent: &Option<Agent>,
) -> Option<(CompletionStatus, f64, Vec<String>)> {
    if let Some(ref agent) = agent {
        if agent.is_enabled() {
            if !agent.is_task_complete(recent_messages) {
                return None;
            }

            let status = agent.completion_status(recent_messages);
            let confidence = agent.completion_confidence(recent_messages);
            let patterns = agent.completion_pattern_matches(recent_messages);
            return Some((status, confidence, patterns));
        }
    }
    None
}
