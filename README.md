# Chatter 🤖

A terminal-based chat interface for Google's Gemini API and local Ollama models, built in Rust.

## Features

- **Interactive Chat Mode**: Real-time conversation with Gemini AI
- **Agent Mode**: Autonomous file operations with tool execution
- **Streaming Responses**: See responses as they're generated
- **Multi-turn Conversations**: Maintains conversation history for context
- **Multiple Models**: Seamlessly switch between Gemini (cloud) and Ollama (local) models
- **Tool Calling**: Expose local file-operation tools directly to Ollama models
- **Session Management**: Save and load chat sessions
- **Rich Terminal UI**: Colored output, progress indicators, and intuitive commands
- **Configuration Management**: Secure API key storage
- **Homebrew Installation**: Easy installation via `brew install`

## Documentation

Full project docs live in the `docs/` directory and can be built with [mdBook](https://rust-lang.github.io/mdBook/):

```bash
mdbook serve docs
```

This command launches a local preview server at http://localhost:3000.

## Installation

### Debian / Ubuntu (APT repository)

The GitHub Pages site hosts a signed APT repository for 64-bit Debian-based systems. Add the key and source, then install:

```bash
curl -fsSL https://tomatyss.github.io/chatter/apt/KEY.gpg | \
  sudo gpg --dearmor -o /usr/share/keyrings/chatter-archive-keyring.gpg
echo "deb [arch=amd64 signed-by=/usr/share/keyrings/chatter-archive-keyring.gpg] \
  https://tomatyss.github.io/chatter/apt stable main" | \
  sudo tee /etc/apt/sources.list.d/chatter.list
sudo apt update
sudo apt install chatter
```

Replace the base URL if you're working from a fork. The repository currently publishes `amd64` builds produced by the automated release workflow.

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

2. Configure the API key (required for the Gemini provider):
   ```bash
   chatter config set-api-key
   ```
   
   Or set it as an environment variable:
   ```bash
   export GEMINI_API_KEY="your-api-key-here"
   ```

3. (Optional) For Ollama support, install and run [Ollama](https://ollama.com/):
   ```bash
   # macOS example
   brew install ollama
   ollama serve
   ```
   By default Chatter connects to `http://localhost:11434`. You can change the endpoint in the configuration file under the `ollama.endpoint` field.

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

# Talk to a local Ollama model
chatter --provider ollama --model llama3.1 "Summarize the latest meeting notes"

# Set system instructions
chatter --system "You are a helpful coding assistant" "Help me with Rust"

# Load a previous session
chatter --load-session my-chat.json

# Auto-save the session
chatter --auto-save
```

If you omit `--provider`, Chatter uses the provider stored in your configuration file (default is `gemini`).

### Agent Mode

Enable autonomous file operations with agent mode:

```bash
# In interactive chat, enable agent mode and set the working directory (optional)
/agent on
/agent allow-path .

# The AI can now execute file operations automatically relative to the current directory
You: Please read the file config.json and search for TODO comments in all Rust files

🔧 AGENT: Executing tool: read_file
   💭 Reading file content as requested
   ✅ Successfully read 245 bytes from config.json

🔧 AGENT: Executing tool: search_files  
   💭 Searching for files as requested
   ✅ Found 3 matches in 12 files
```

#### Agent Commands

- `/agent on` - Enable agent mode
- `/agent off` - Disable agent mode  
- `/agent status` - Show agent status
- `/agent history` - Show tool execution history
- `/agent tools` - List available tools
- `/agent config` - Show agent configuration
- `/agent allow-path <path>` - Temporarily permit an additional directory
- `/agent forbid-path <path>` - Block access to a directory
- `/agent help` - Show agent help

#### Available Tools

- **read_file** - Read file contents
- **write_file** - Create or overwrite files
- **update_file** - Update files with targeted changes
- **search_files** - Search for patterns across files
- **list_directory** - List directory contents
- **file_info** - Get detailed file information

### Ollama Integration

- Run any locally installed model exposed by Ollama with `--provider ollama --model <name>`
- When agent mode is enabled, Chatter automatically exposes its file-system tools to the model using Ollama's function-calling API. Tools operate relative to the current working directory by default; add extra directories with `/agent allow-path` as needed.
- Tool results are sent back to the model and also summarized in the terminal so you can follow along
- The Ollama endpoint defaults to `http://localhost:11434`; override it in `config.json` if your server runs elsewhere

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

### Gemini (Cloud)

- `gemini-2.5-flash` (default)
- `gemini-2.5-pro`
- `gemini-1.5-flash`
- `gemini-1.5-pro`

### Ollama (Local)

- Any model installed via `ollama pull ...` (e.g. `llama3.1`, `qwen2.5-coder`, etc.)
- List available models with `ollama list`
- Select them with `--provider ollama --model <name>`

## Examples

### Basic Chat
```bash
$ chatter
🤖 Chatter - Gemini AI Chat
Model: gemini-2.5-flash | Provider: Gemini | Session: a1b2c3d4
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

### Ollama Chat (with tools)
```bash
$ chatter --provider ollama --model llama3.1
🤖 Chatter - Ollama AI Chat
Model: llama3.1 | Provider: Ollama | Session: f9e1c3b2
────────────────────────────────────────────────────────────
Type 'exit' to quit, '/help' for commands

You: Could you read Cargo.toml and summarize the dependencies?

🔧 TOOL Executing tool: read_file
   ✅ Read 426 bytes from Cargo.toml

Ollama: Cargo.toml declares crates such as `reqwest`, `tokio`, `ratatui`, and `rustyline`.
```

### Using Different Models
```bash
$ chatter --model gemini-2.5-pro "Explain the differences between ownership, borrowing, and lifetimes in Rust"
```

## Configuration

Chatter stores its configuration in:
- **macOS**: `~/Library/Application Support/chatter/config.json`
- **Linux**: `~/.config/chatter/config.json`
- **Windows**: `%APPDATA%\\chatter\\config.json`

Key fields:

- `provider`: `"gemini"` (default) or `"ollama"`
- `default_model`: Model name used when `--model` is not provided
- `ollama.endpoint`: Base URL for the Ollama server (defaults to `http://localhost:11434`)

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

### Debian package publishing

The `Publish Debian Package` workflow builds the `.deb` with `cargo deb`, regenerates the APT metadata, signs the `Release` files, and commits the result to the `gh-pages` branch under `apt/`. To enable signing you need to add repository secrets:

- `APT_GPG_PRIVATE_KEY` – ASCII-armored private key used for signing.
- `APT_GPG_PASSPHRASE` – Passphrase for the key (leave empty for an unprotected key).
- `APT_GPG_KEY_ID` – Fingerprint or key ID exported as the public key (`KEY.gpg`).

Once the secrets exist, publishing a GitHub Release (tag `v*`) will automatically update the APT repository at `https://tomatyss.github.io/chatter/apt`.

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
