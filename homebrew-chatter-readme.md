# Homebrew Tap for Chatter

This is the official Homebrew tap for [Chatter](https://github.com/tomatyss/chatter), a terminal-based chat interface for Google's Gemini AI.

## Installation

```bash
brew tap tomatyss/chatter
brew install chatter
```

## About Chatter

Chatter is a Rust-based terminal application that provides an interactive chat interface for Google's Gemini AI with features like:

- Real-time streaming responses
- Session management
- Multiple model support
- Rich terminal UI

For more information, visit the [main repository](https://github.com/tomatyss/chatter).

## Setup

After installation, you'll need to configure your Gemini API key:

1. Get your API key from [Google AI Studio](https://aistudio.google.com/app/apikey)
2. Set it up with: `chatter config set-api-key`
3. Or export it: `export GEMINI_API_KEY="your-api-key"`

Then start chatting with: `chatter`
