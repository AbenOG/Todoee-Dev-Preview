//! OpenRouter AI client for natural language task parsing
//!
//! This module provides an AI client that communicates with OpenRouter's API
//! to parse natural language input into structured task data.

use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use zeroize::ZeroizeOnDrop;

use crate::config::Config;
use crate::error::TodoeeError;

/// A parsed task extracted from natural language input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTask {
    /// The task title (required)
    pub title: String,
    /// Optional task description
    #[serde(default)]
    pub description: Option<String>,
    /// Optional due date
    #[serde(default)]
    pub due_date: Option<DateTime<Utc>>,
    /// Optional category name
    #[serde(default)]
    pub category: Option<String>,
    /// Optional priority (1-4, where 1 is highest)
    #[serde(default)]
    pub priority: Option<i32>,
    /// Optional reminder time
    #[serde(default)]
    pub reminder_at: Option<DateTime<Utc>>,
}

impl ParsedTask {
    /// Parse a ParsedTask from JSON string
    ///
    /// This method handles cases where the AI response may contain
    /// extra text around the JSON object.
    pub fn from_json(json: &str) -> Result<Self, TodoeeError> {
        // Try to extract JSON if there's extra text
        let json_str = extract_json(json).unwrap_or(json);

        serde_json::from_str(json_str).map_err(|e| TodoeeError::AiParsing {
            message: format!("Failed to parse AI response as JSON: {}", e),
        })
    }
}

/// Extract a JSON object from text that may contain surrounding text
///
/// AI models sometimes include explanatory text around the JSON response.
/// This function finds and extracts the JSON object.
pub fn extract_json(text: &str) -> Option<&str> {
    // Find the first '{' and last matching '}'
    let start = text.find('{')?;
    let text_from_start = &text[start..];

    // Track brace nesting to find the matching closing brace
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, c) in text_from_start.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match c {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text_from_start[..=i]);
                }
            }
            _ => {}
        }
    }

    None
}

// ============================================================================
// OpenRouter API Types
// ============================================================================

/// Request body for OpenRouter chat completions API
#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

/// A message in the chat conversation
#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

/// Response from OpenRouter chat completions API
#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<Choice>,
}

/// A choice in the OpenRouter response
#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

// ============================================================================
// AI Client
// ============================================================================

/// Client for communicating with OpenRouter AI API
///
/// The api_key field is automatically zeroed when the struct is dropped
/// to prevent sensitive data from remaining in memory.
#[derive(ZeroizeOnDrop)]
pub struct AiClient {
    #[zeroize(skip)]
    client: Client,
    api_key: String,
    #[zeroize(skip)]
    model: String,
}

impl AiClient {
    /// Create a new AiClient from configuration
    ///
    /// # Errors
    ///
    /// Returns `TodoeeError::AiService` if:
    /// - The API key environment variable is not set
    /// - The AI model is not configured
    pub fn new(config: &Config) -> Result<Self, TodoeeError> {
        let api_key = config.get_ai_api_key().map_err(|e| TodoeeError::AiService {
            message: format!("Failed to get AI API key: {}", e),
        })?;

        let model = config
            .ai
            .model
            .clone()
            .ok_or_else(|| TodoeeError::AiService {
                message: "AI model not configured. Set [ai].model in config.toml".to_string(),
            })?;

        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| TodoeeError::AiService {
                    message: format!("Failed to build HTTP client: {}", e),
                })?,
            api_key,
            model,
        })
    }

    /// Parse natural language input into a structured task
    ///
    /// # Arguments
    ///
    /// * `input` - Natural language description of a task
    ///
    /// # Returns
    ///
    /// A `ParsedTask` containing the extracted task information
    ///
    /// # Errors
    ///
    /// Returns `TodoeeError::AiService` if the API request fails
    /// Returns `TodoeeError::AiParsing` if the response cannot be parsed
    pub async fn parse_task(&self, input: &str) -> Result<ParsedTask, TodoeeError> {
        let current_date = Utc::now().format("%Y-%m-%d").to_string();

        let system_prompt = format!(
            r#"You are a task parsing assistant. Parse the user's input into a structured task.
Today's date is {}.

Respond ONLY with a JSON object in this exact format:
{{
    "title": "Brief task title",
    "description": "Optional longer description or null",
    "due_date": "ISO 8601 datetime or null",
    "category": "Category name or null",
    "priority": "Integer 1-4 (1=highest) or null",
    "reminder_at": "ISO 8601 datetime or null"
}}

Rules:
- title is required and should be concise
- Use null for optional fields that aren't specified
- Convert relative dates (tomorrow, next week) to absolute ISO 8601 format
- Infer category from context (Work, Personal, Shopping, Health, etc.)
- Priority: 1=urgent, 2=high, 3=normal, 4=low
- Set reminder_at to 15 minutes before due_date if a due time is specified

Respond with ONLY the JSON object, no additional text."#,
            current_date
        );

        let request = OpenRouterRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system_prompt,
                },
                Message {
                    role: "user".to_string(),
                    content: input.to_string(),
                },
            ],
            temperature: 0.1,
            max_tokens: 500,
        };

        let response = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| TodoeeError::AiService {
                message: format!("Failed to send request to OpenRouter: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(TodoeeError::AiService {
                message: format!("OpenRouter API error ({}): {}", status, body),
            });
        }

        let api_response: OpenRouterResponse =
            response.json().await.map_err(|e| TodoeeError::AiService {
                message: format!("Failed to parse OpenRouter response: {}", e),
            })?;

        let content = api_response
            .choices
            .first()
            .map(|c| c.message.content.as_str())
            .ok_or_else(|| TodoeeError::AiParsing {
                message: "No response from AI model".to_string(),
            })?;

        ParsedTask::from_json(content)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ai_response_valid() {
        let json = r#"{
            "title": "Buy groceries",
            "description": "Get milk, eggs, and bread",
            "due_date": "2024-12-25T10:00:00Z",
            "category": "Shopping",
            "priority": 2,
            "reminder_at": "2024-12-25T09:45:00Z"
        }"#;

        let task = ParsedTask::from_json(json).expect("Should parse valid JSON");

        assert_eq!(task.title, "Buy groceries");
        assert_eq!(
            task.description,
            Some("Get milk, eggs, and bread".to_string())
        );
        assert!(task.due_date.is_some());
        assert_eq!(task.category, Some("Shopping".to_string()));
        assert_eq!(task.priority, Some(2));
        assert!(task.reminder_at.is_some());
    }

    #[test]
    fn test_parse_ai_response_minimal() {
        let json = r#"{"title": "Simple task"}"#;

        let task = ParsedTask::from_json(json).expect("Should parse minimal JSON");

        assert_eq!(task.title, "Simple task");
        assert!(task.description.is_none());
        assert!(task.due_date.is_none());
        assert!(task.category.is_none());
        assert!(task.priority.is_none());
        assert!(task.reminder_at.is_none());
    }

    #[test]
    fn test_parse_ai_response_invalid() {
        let json = r#"{"not_a_title": "missing required field"}"#;

        let result = ParsedTask::from_json(json);

        assert!(result.is_err());
        match result {
            Err(TodoeeError::AiParsing { message }) => {
                assert!(message.contains("Failed to parse"));
            }
            _ => panic!("Expected AiParsing error"),
        }
    }

    #[test]
    fn test_extract_json_with_extra_text() {
        let text = r#"Here is the parsed task:

{
    "title": "Call mom",
    "description": null,
    "due_date": "2024-12-25T18:00:00Z",
    "category": "Personal",
    "priority": 2,
    "reminder_at": null
}

I hope this helps!"#;

        let extracted = extract_json(text).expect("Should extract JSON");
        let task = ParsedTask::from_json(extracted).expect("Should parse extracted JSON");

        assert_eq!(task.title, "Call mom");
        assert_eq!(task.category, Some("Personal".to_string()));
    }

    #[test]
    fn test_extract_json_nested() {
        let text = r#"Response: {"title": "Test", "metadata": {"nested": {"deep": "value"}}}"#;

        let extracted = extract_json(text).expect("Should extract nested JSON");

        // Verify the extracted JSON is complete and properly nested
        assert!(extracted.starts_with('{'));
        assert!(extracted.ends_with('}'));
        assert!(extracted.contains("metadata"));
        assert!(extracted.contains("nested"));
        assert!(extracted.contains("deep"));
    }

    #[test]
    fn test_extract_json_with_strings_containing_braces() {
        let text = r#"{"title": "Fix {bug} in code", "description": "The } character breaks things"}"#;

        let extracted = extract_json(text).expect("Should handle braces in strings");
        let parsed: serde_json::Value =
            serde_json::from_str(extracted).expect("Should be valid JSON");

        assert_eq!(parsed["title"], "Fix {bug} in code");
        assert_eq!(parsed["description"], "The } character breaks things");
    }

    #[test]
    fn test_extract_json_no_json() {
        let text = "Just some regular text without any JSON";
        assert!(extract_json(text).is_none());
    }

    #[test]
    fn test_extract_json_unclosed_brace() {
        let text = r#"{"title": "Incomplete"#;
        assert!(extract_json(text).is_none());
    }

    #[test]
    fn test_parsed_task_with_null_values() {
        let json = r#"{
            "title": "Task with nulls",
            "description": null,
            "due_date": null,
            "category": null,
            "priority": null,
            "reminder_at": null
        }"#;

        let task = ParsedTask::from_json(json).expect("Should handle explicit nulls");

        assert_eq!(task.title, "Task with nulls");
        assert!(task.description.is_none());
        assert!(task.due_date.is_none());
        assert!(task.category.is_none());
        assert!(task.priority.is_none());
        assert!(task.reminder_at.is_none());
    }
}
