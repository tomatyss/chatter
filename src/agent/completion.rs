//! Task completion detection for autonomous agent operations
//!
//! Analyzes conversation patterns and tool usage to determine when tasks are complete.

use super::ToolCall;
use std::time::{Duration, Instant};

/// Detector for autonomous task completion
#[derive(Debug)]
pub struct CompletionDetector {
    last_tool_execution: Option<Instant>,
    completion_patterns: Vec<CompletionPattern>,
    inactivity_threshold: Duration,
}

impl CompletionDetector {
    /// Create a new completion detector
    pub fn new() -> Self {
        Self {
            last_tool_execution: None,
            completion_patterns: Self::default_patterns(),
            inactivity_threshold: Duration::from_secs(30), // 30 seconds of no tool activity
        }
    }

    /// Check for explicit completion signals in recent messages
    fn has_completion_signals(&self, messages: &[String]) -> bool {
        let completion_phrases = [
            "task completed",
            "task complete",
            "finished successfully",
            "done with",
            "completed successfully",
            "task is finished",
            "work is complete",
            "all done",
            "successfully completed",
            "task accomplished",
            "objective achieved",
            "mission accomplished",
            "finished the task",
            "completed the request",
            "task has been completed",
        ];

        for message in messages.iter().rev().take(3) {
            // Check last 3 messages
            let message_lower = message.to_lowercase();
            for phrase in &completion_phrases {
                if message_lower.contains(phrase) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if recent messages match completion patterns
    fn matches_completion_patterns(&self, messages: &[String], tool_history: &[ToolCall]) -> bool {
        for pattern in &self.completion_patterns {
            if pattern.matches(messages, tool_history) {
                return true;
            }
        }

        false
    }

    /// Get the human-readable descriptions of patterns that currently match
    pub fn matching_patterns(&self, messages: &[String], tool_history: &[ToolCall]) -> Vec<String> {
        self.completion_patterns
            .iter()
            .filter(|pattern| pattern.matches(messages, tool_history))
            .map(|pattern| format!("{}: {}", pattern.name, pattern.description))
            .collect()
    }

    /// Check for tool execution inactivity
    fn has_tool_inactivity(&self) -> bool {
        if let Some(last_execution) = self.last_tool_execution {
            last_execution.elapsed() > self.inactivity_threshold
        } else {
            false
        }
    }

    /// Check for successful execution patterns
    fn has_successful_execution_pattern(&self, tool_history: &[ToolCall]) -> bool {
        if tool_history.is_empty() {
            return false;
        }

        // Look for patterns indicating successful completion
        let recent_tools: Vec<&str> = tool_history
            .iter()
            .rev()
            .take(5)
            .map(|call| call.tool.as_str())
            .collect();

        // Pattern: Read -> Process -> Write (common completion pattern)
        if recent_tools.len() >= 3 {
            let has_read = recent_tools
                .iter()
                .any(|&tool| tool == "read_file" || tool == "search_files");
            let has_write = recent_tools
                .iter()
                .any(|&tool| tool == "write_file" || tool == "update_file");

            if has_read && has_write {
                return true;
            }
        }

        // Pattern: Multiple successful file operations
        if recent_tools.len() >= 2 {
            let file_ops = recent_tools
                .iter()
                .filter(|&&tool| matches!(tool, "write_file" | "update_file" | "read_file"))
                .count();

            if file_ops >= 2 {
                return true;
            }
        }

        false
    }

    /// Update the last tool execution time
    pub fn record_tool_execution(&mut self) {
        self.last_tool_execution = Some(Instant::now());
    }

    /// Get default completion patterns
    fn default_patterns() -> Vec<CompletionPattern> {
        vec![
            // Summary generation pattern
            CompletionPattern {
                name: "summary_generation".to_string(),
                description: "Task involves creating a summary or report".to_string(),
                message_patterns: vec![
                    "summary".to_string(),
                    "report".to_string(),
                    "analysis complete".to_string(),
                    "findings".to_string(),
                ],
                tool_sequence: vec![
                    "search_files".to_string(),
                    "read_file".to_string(),
                    "write_file".to_string(),
                ],
                min_tools: 2,
            },
            // File organization pattern
            CompletionPattern {
                name: "file_organization".to_string(),
                description: "Task involves organizing or restructuring files".to_string(),
                message_patterns: vec![
                    "organized".to_string(),
                    "restructured".to_string(),
                    "cleaned up".to_string(),
                    "files arranged".to_string(),
                ],
                tool_sequence: vec![
                    "list_directory".to_string(),
                    "read_file".to_string(),
                    "write_file".to_string(),
                ],
                min_tools: 3,
            },
            // Documentation pattern
            CompletionPattern {
                name: "documentation".to_string(),
                description: "Task involves creating or updating documentation".to_string(),
                message_patterns: vec![
                    "documentation".to_string(),
                    "readme".to_string(),
                    "docs updated".to_string(),
                    "documented".to_string(),
                ],
                tool_sequence: vec!["read_file".to_string(), "write_file".to_string()],
                min_tools: 2,
            },
            // Code analysis pattern
            CompletionPattern {
                name: "code_analysis".to_string(),
                description: "Task involves analyzing code files".to_string(),
                message_patterns: vec![
                    "analysis".to_string(),
                    "reviewed".to_string(),
                    "examined".to_string(),
                    "code structure".to_string(),
                ],
                tool_sequence: vec!["search_files".to_string(), "read_file".to_string()],
                min_tools: 2,
            },
        ]
    }

    /// Get completion confidence score (0.0 to 1.0)
    pub fn completion_confidence(&self, messages: &[String], tool_history: &[ToolCall]) -> f64 {
        let mut confidence: f64 = 0.0;

        // Explicit completion signals (high confidence)
        if self.has_completion_signals(messages) {
            confidence += 0.8;
        }

        // Pattern matching (medium confidence)
        if self.matches_completion_patterns(messages, tool_history) {
            confidence += 0.6;
        }

        // Successful execution pattern (medium confidence)
        if self.has_successful_execution_pattern(tool_history) {
            confidence += 0.5;
        }

        // Tool inactivity (low confidence)
        if self.has_tool_inactivity() {
            confidence += 0.3;
        }

        // Recent tool activity reduces confidence
        if let Some(last_execution) = self.last_tool_execution {
            if last_execution.elapsed() < Duration::from_secs(5) {
                confidence *= 0.5; // Reduce confidence if tools were used very recently
            }
        }

        confidence.min(1.0_f64)
    }

    /// Get a human-readable completion status
    pub fn completion_status(
        &self,
        messages: &[String],
        tool_history: &[ToolCall],
    ) -> CompletionStatus {
        let confidence = self.completion_confidence(messages, tool_history);

        if confidence >= 0.8 {
            CompletionStatus::Complete
        } else if confidence >= 0.5 {
            CompletionStatus::LikelyComplete
        } else if confidence >= 0.3 {
            CompletionStatus::PossiblyComplete
        } else {
            CompletionStatus::InProgress
        }
    }
}

/// A pattern that indicates task completion
#[derive(Debug, Clone)]
pub struct CompletionPattern {
    pub name: String,
    pub description: String,
    pub message_patterns: Vec<String>,
    pub tool_sequence: Vec<String>,
    pub min_tools: usize,
}

impl CompletionPattern {
    /// Check if this pattern matches the current state
    pub fn matches(&self, messages: &[String], tool_history: &[ToolCall]) -> bool {
        // Check message patterns
        let has_message_pattern = if self.message_patterns.is_empty() {
            true // No message pattern required
        } else {
            messages.iter().rev().take(3).any(|message| {
                let message_lower = message.to_lowercase();
                self.message_patterns
                    .iter()
                    .any(|pattern| message_lower.contains(pattern))
            })
        };

        // Check tool sequence
        let has_tool_sequence = if self.tool_sequence.is_empty() {
            true // No tool sequence required
        } else {
            let recent_tools: Vec<&str> = tool_history
                .iter()
                .rev()
                .take(10)
                .map(|call| call.tool.as_str())
                .collect();

            // Check if all required tools were used
            self.tool_sequence.iter().all(|required_tool| {
                recent_tools
                    .iter()
                    .any(|&used_tool| used_tool == required_tool)
            })
        };

        // Check minimum tool count
        let has_min_tools = tool_history.len() >= self.min_tools;

        has_message_pattern && has_tool_sequence && has_min_tools
    }
}

/// Task completion status
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionStatus {
    InProgress,
    PossiblyComplete,
    LikelyComplete,
    Complete,
}

impl CompletionStatus {
    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            CompletionStatus::InProgress => "Task is still in progress",
            CompletionStatus::PossiblyComplete => "Task might be complete",
            CompletionStatus::LikelyComplete => "Task is likely complete",
            CompletionStatus::Complete => "Task appears to be complete",
        }
    }

    /// Check if the status indicates completion
    pub fn is_complete(&self) -> bool {
        matches!(
            self,
            CompletionStatus::Complete | CompletionStatus::LikelyComplete
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_explicit_completion_signals() {
        let detector = CompletionDetector::new();
        let messages = vec![
            "I'm working on the task".to_string(),
            "Task completed successfully!".to_string(),
        ];

        assert!(detector.has_completion_signals(&messages));
    }

    #[test]
    fn test_successful_execution_pattern() {
        let detector = CompletionDetector::new();
        let tool_history = vec![
            ToolCall {
                tool: "read_file".to_string(),
                parameters: HashMap::new(),
                thought: None,
                reasoning: None,
            },
            ToolCall {
                tool: "write_file".to_string(),
                parameters: HashMap::new(),
                thought: None,
                reasoning: None,
            },
        ];

        assert!(detector.has_successful_execution_pattern(&tool_history));
    }

    #[test]
    fn test_completion_confidence() {
        let detector = CompletionDetector::new();
        let messages = vec!["Task completed successfully!".to_string()];
        let tool_history = vec![
            ToolCall {
                tool: "read_file".to_string(),
                parameters: HashMap::new(),
                thought: None,
                reasoning: None,
            },
            ToolCall {
                tool: "write_file".to_string(),
                parameters: HashMap::new(),
                thought: None,
                reasoning: None,
            },
        ];

        let confidence = detector.completion_confidence(&messages, &tool_history);
        assert!(confidence > 0.8);
    }
}
