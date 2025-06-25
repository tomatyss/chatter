# Chatter 🤖

A terminal-based chat interface for Google's Gemini AI, built in Rust.

## Features

- **Interactive Chat Mode**: Real-time conversation with Gemini AI
- **Streaming Responses**: See responses as they're generated
- **Multi-turn Conversations**: Maintains conversation history for context
- **Multiple Models**: Support for different Gemini models (2.5-flash, 2.5-pro, etc.)
- **Session Management**: Save and load chat sessions
- **Rich Terminal UI**: Colored output, progress indicators, and intuitive commands
- **Configuration Management**: Secure API key storage
- **Homebrew Installation**: Easy installation via `brew install`

## Installation

### Via Homebrew (Recommended)

```bash
brew tap tomatyss/chatter
brew install chatter
```

### From Source

```bash
git clone https://github.com/tomatyss/chatter.git
cd chatter
cargo build --release
sudo cp target/release/chatter /usr/local/bin/
```

## Setup

1. Get your Gemini API key from [Google AI Studio](https://aistudio.google.com/app/apikey)

2. Configure the API key:
   ```bash
   chatter config set-api-key
   ```
   
   Or set it as an environment variable:
   ```bash
   export GEMINI_API_KEY="your-api-key-here"
   ```

## Usage

### Interactive Chat Mode

Start an interactive chat session:

```bash
chatter
```

This opens a real-time chat interface where you can have conversations with Gemini AI.

### One-Shot Queries

Send a single message without entering interactive mode:

```bash
chatter "What is Rust programming language?"
```

### Advanced Options

```bash
# Use a specific model
chatter --model gemini-2.5-pro "Explain quantum computing"

# Set system instructions
chatter --system "You are a helpful coding assistant" "Help me with Rust"

# Load a previous session
chatter --load-session my-chat.json

# Auto-save the session
chatter --auto-save
```

### Interactive Commands

While in interactive mode, you can use these commands:

- `/help` - Show available commands
- `/clear` - Clear conversation history
- `/save <filename>` - Save current session
- `/load <filename>` - Load a session
- `/model <name>` - Switch models
- `/system <instruction>` - Set system instruction
- `/history` - Show conversation history
- `/info` - Show session information
- `exit` or `quit` - Exit the chat

### Configuration Commands

```bash
# Show current configuration
chatter config show

# Set API key interactively
chatter config set-api-key

# Reset configuration
chatter config reset
```

## Supported Models

- `gemini-2.5-flash` (default) - Fast responses, good for most tasks
- `gemini-2.5-pro` - Higher quality responses, better for complex tasks
- `gemini-1.5-flash` - Previous generation, fast
- `gemini-1.5-pro` - Previous generation, high quality

## Examples

### Basic Chat
```bash
$ chatter
🤖 Chatter - Gemini AI Chat
Model: gemini-2.5-flash | Session: a1b2c3d4
────────────────────────────────────────────────────────────
Type 'exit' to quit, '/help' for commands

You: Hello! Can you help me learn Rust?

Gemini: Hello! I'd be happy to help you learn Rust! Rust is a systems programming 
language that focuses on safety, speed, and concurrency. What specific aspect 
of Rust would you like to start with?

You: What makes Rust special?

Gemini: Rust has several unique features that make it special:

1. **Memory Safety**: Rust prevents common bugs like null pointer dereferences...
```

### Quick Query
```bash
$ chatter "Write a simple 'Hello, World!' program in Rust"

fn main() {
    println!("Hello, World!");
}

This is the simplest Rust program. The `main` function is the entry point...
```

### Using Different Models
```bash
$ chatter --model gemini-2.5-pro "Explain the differences between ownership, borrowing, and lifetimes in Rust"
```

## Configuration

Chatter stores its configuration in:
- **macOS**: `~/Library/Application Support/chatter/config.json`
- **Linux**: `~/.config/chatter/config.json`
- **Windows**: `%APPDATA%\chatter\config.json`

Session files are saved in the `sessions/` subdirectory.

## API Usage

The Gemini API follows the multi-turn conversation format:

```json
{
  "contents": [
    {
      "role": "user",
      "parts": [{"text": "Hello"}]
    },
    {
      "role": "model", 
      "parts": [{"text": "Great to meet you. What would you like to know?"}]
    },
    {
      "role": "user",
      "parts": [{"text": "I have two dogs in my house. How many paws are in my house?"}]
    }
  ]
}
```

## Development

### Building from Source

```bash
git clone https://github.com/tomatyss/chatter.git
cd chatter
cargo build
```

### Running Tests

```bash
cargo test
```

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/)
- Uses Google's [Gemini API](https://ai.google.dev/gemini-api)
- Terminal UI powered by [crossterm](https://github.com/crossterm-rs/crossterm) and [ratatui](https://github.com/ratatui-org/ratatui)
