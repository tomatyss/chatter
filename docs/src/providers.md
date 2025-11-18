# Providers and Models

Chatter supports Gemini and any Ollama model you have locally.

## Gemini

Gemini requires an API key from [Google AI Studio](https://aistudio.google.com/app/apikey). Chatter defaults to the `gemini-2.5-flash` model, but you can select other Gemini models with `--model` or `/model` in the UI.

## Ollama

Install [Ollama](https://ollama.com/) and run `ollama serve`. Chatter connects to `http://localhost:11434` unless you override the endpoint via configuration. Once Ollama is running, pull any supported model, for example:

```bash
ollama pull llama3.1
chatter --provider ollama --model llama3.1
```

Tool calls are available in Ollama mode, enabling local workflows that need filesystem access coupled with language model reasoning.
