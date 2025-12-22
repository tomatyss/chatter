# Interactive Chat

Interactive mode maintains a conversation state so later prompts have access to earlier context. Streaming output keeps the terminal responsive while Gemini or Ollama streams tokens back to the client.

Useful commands during a chat session:

- `/help` — show command reference
- `/system` — set the system prompt mid-conversation
- `/clear` — reset the transcript without restarting the binary
- `/save` — write the session to disk (defaults to `./session-<timestamp>.json`)
- `/load` — load a previous session file

You can toggle providers on the fly with `/provider gemini` or `/provider ollama`, and pick a specific model with `/model <name>`.
