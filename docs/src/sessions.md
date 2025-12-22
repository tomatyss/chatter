# Sessions

Chatter stores conversations as JSON so you can pause and resume long-running threads. Saving a session preserves messages, system instructions, and model selections.

```text
/save my-session.json
```

Reload the transcript later with `/load my-session.json`. Session files default to the `sessions/` directory in the configuration path, but you can supply absolute or relative paths.

When sharing sessions, remove sensitive content manually—Chatter does not scrub secrets on export.
