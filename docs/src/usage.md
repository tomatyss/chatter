# Using Chatter

The default mode launches an interactive shell with streaming responses and an always-on history buffer. You can also invoke Chatter for one-shot prompts or scripted automation.

```bash
chatter
```

Inside the interface, type `/help` for a list of commands. Use `/model` or `/provider` to switch models, `/save` to persist the transcript, and `/exit` to leave the session.

For quick questions, pass the prompt as a positional argument:

```bash
chatter "Explain ownership in Rust"
```

Additional flags let you set the model, override the provider, and inject system instructions.
