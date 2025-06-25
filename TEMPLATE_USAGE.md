# System Instruction Templates

Chatter now supports reusable system instruction templates that allow you to quickly apply different AI personalities and behaviors to your conversations.

## Overview

Templates are pre-defined system instructions that can be:
- **Built-in**: Professionally crafted templates for common use cases
- **Custom**: User-created templates saved for reuse
- **Organized**: Categorized and tagged for easy discovery
- **Persistent**: Stored locally and available across sessions

## Quick Start

### Using Templates in CLI

```bash
# Start chat with a template
chatter --template coding_assistant

# Use template in one-shot query
chatter query "How do I implement a binary search?" --template coding_assistant

# List available templates
chatter template list

# Show template details
chatter template show coding_assistant

# Create a new template
chatter template create my_assistant --description "My custom assistant"
```

### Using Templates in Interactive Chat

```bash
# List available templates
/templates

# Apply a template to current session
/template coding_assistant

# Save current system instruction as template
/save-template my_custom_template
```

## Built-in Templates

Chatter comes with 11 professionally crafted templates:

### Development
- **coding_assistant**: Expert programming assistant for code help and debugging
- **code_reviewer**: Thorough code review specialist focusing on quality and best practices
- **technical_writer**: Technical documentation and writing specialist

### Creative & Writing
- **creative_writer**: Creative writing assistant for stories, poems, and creative content
- **message_editor**: Message editor agent that reviews and improves English text
- **translator**: Professional translator with cultural context awareness

### Education & Analysis
- **tutor**: Patient and knowledgeable tutor for learning and education
- **data_analyst**: Data analysis expert for insights and visualization guidance

### Business
- **product_manager**: Strategic product management advisor for planning and execution

### General Purpose
- **friendly_assistant**: Warm, helpful, and conversational general assistant
- **concise_assistant**: Direct and efficient assistant for quick, focused responses

## Template Management Commands

### CLI Commands

```bash
# List all templates
chatter template list

# Filter by category
chatter template list --category development

# Search templates
chatter template list --search "coding"

# Show template details
chatter template show <name>

# Create new template (interactive)
chatter template create <name>

# Create with details
chatter template create my_template \
  --description "My custom template" \
  --category "custom"

# Edit existing template
chatter template edit <name>

# Delete template
chatter template delete <name>

# Delete without confirmation
chatter template delete <name> --force

# Start chat with template
chatter template use <name>
```

### Interactive Chat Commands

```bash
# Show help
/help

# List all templates
/templates

# Apply template to current session
/template <name>

# Save current system instruction as template
/save-template <name>

# Set system instruction manually
/system <instruction>
```

## Template Structure

Each template contains:

```json
{
  "name": "coding_assistant",
  "description": "Expert programming assistant for code help and debugging",
  "content": "You are an expert software engineer...",
  "category": "development",
  "tags": ["coding", "programming", "development", "debugging"],
  "created_at": "2025-01-01T00:00:00Z",
  "updated_at": "2025-01-01T00:00:00Z",
  "builtin": true
}
```

## Creating Custom Templates

### Method 1: CLI Creation

```bash
chatter template create my_assistant
```

This will prompt you for:
- Description
- Category
- Content (opens your default editor)
- Tags

### Method 2: Interactive Chat

```bash
# First, set your system instruction
/system You are a helpful assistant specialized in cooking recipes.

# Then save it as a template
/save-template cooking_assistant
```

### Method 3: Direct Template Use

```bash
# Start with a base template
chatter --template friendly_assistant

# Modify the system instruction in chat
/system You are a friendly cooking assistant who loves to share recipes and cooking tips.

# Save the modified version
/save-template cooking_assistant
```

## Template Storage

Templates are stored in:
- **Location**: `~/.config/chatter/templates/`
- **Format**: JSON files named `<template_name>.json`
- **Built-ins**: Loaded from code, not stored as files
- **Custom**: Saved as individual JSON files

## Examples

### Example 1: Code Review Session

```bash
# Start with code reviewer template
chatter --template code_reviewer

# Or in interactive mode
/template code_reviewer

# Now paste your code for review
```

### Example 2: Creative Writing

```bash
# Use creative writer template
chatter template use creative_writer

# Start writing
"Help me write a short story about a time traveler"
```

### Example 3: Quick Technical Query

```bash
# One-shot query with technical writer template
chatter query "Explain REST APIs" --template technical_writer
```

### Example 4: Message Editing

```bash
# Use message editor template to improve text
chatter query "me and my friend goes to store yesterday" --template message_editor

# Output: "My friend and I went to the store yesterday."

# Or in interactive mode
chatter --template message_editor
# Then paste text that needs improvement
```

### Example 5: Custom Learning Assistant

```bash
# Create a custom template for learning Python
chatter template create python_tutor \
  --description "Python programming tutor" \
  --category "education"

# Content: "You are a patient Python programming tutor..."
```

## Tips and Best Practices

1. **Template Naming**: Use descriptive, lowercase names with underscores
2. **Categories**: Group related templates (development, creative, business, etc.)
3. **Tags**: Add relevant tags for easy searching
4. **Content**: Be specific about the AI's role, expertise, and behavior
5. **Testing**: Test templates with various queries to ensure consistency
6. **Sharing**: Templates are just JSON files - easy to share with others

## Integration with Gemini API

Templates work seamlessly with the Gemini API's `system_instruction` field:

```json
{
  "system_instruction": {
    "parts": [
      {
        "text": "You are an expert software engineer and programming assistant..."
      }
    ]
  },
  "contents": [
    {
      "parts": [
        {
          "text": "How do I implement a binary search in Python?"
        }
      ]
    }
  ]
}
```

The template content becomes the `system_instruction.parts[0].text` value, exactly as shown in your original Gemini API example.
