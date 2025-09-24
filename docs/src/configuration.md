# Configuration

Chatter stores its configuration on disk so you can reuse API keys, default providers, and Ollama settings. Configuration lives in platform-specific directories:

- **macOS:** `~/Library/Application Support/chatter/config.json`
- **Linux:** `~/.config/chatter/config.json`
- **Windows:** `%APPDATA%\chatter\config.json`

## Managing API Keys

Set a Gemini API key once and Chatter will reuse it for future sessions:

```bash
chatter config set-api-key
```

Alternatively, export the `GEMINI_API_KEY` environment variable before starting the CLI. The configuration file stores keys securely using your OS keyring when available.

## Provider Defaults

Configuration fields worth knowing:

- `provider` — active provider (`"gemini"` or `"ollama"`)
- `default_model` — fallback model when you omit `--model`
- `ollama.endpoint` — base URL for the Ollama server (defaults to `http://localhost:11434`)

Edit these values through the CLI or by modifying the JSON file directly.
