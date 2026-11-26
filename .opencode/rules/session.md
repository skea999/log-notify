---
globs:
  - "state/session-state.json"
  - "handoff/**/*.md"
  - ".agent-memory/**/*.jsonl"
keywords:
  - "session"
  - "memory"
  - "recall"
  - "handoff"
  - "summary"
  - "commit"
  - "push"
  - "tag"
  - "release"
---
## Session

```bash
# Start
.agents/skills/session-memory/scripts/memory.sh context
.agents/skills/session-memory/scripts/memory.sh recall "project" --limit 10
.agents/skills/session-memory/scripts/memory.sh stats

# End
.agents/skills/session-memory/scripts/memory.sh save "summary" "Today: X, next: Y" -Tags @("chart","frontend")
```

### Final verification

```bash
.agents/skills/session-memory/scripts/memory.sh context --days 14
.agents/skills/session-memory/scripts/memory.sh stats
.agents/skills/session-memory/scripts/memory.sh recall "project" --limit 5
```
