# Agent Mode

Agent mode grants the assistant controlled access to your filesystem. When enabled, Chatter exposes a curated set of tools (such as `read_file`, `write_file`, and `search_files`) that the model can invoke under supervision.

Enable agent mode from inside a chat session:

```text
/agent on
/agent allow-path .
```

You can inspect history with `/agent history`, view available tools with `/agent tools`, and disable the feature with `/agent off`. The agent never leaves the directories you explicitly allow.

Use agent mode for repetitive local tasks: summarizing files, quick refactors, or generating reports. Keep an eye on the streamed tool output to ensure each action matches your expectations.
