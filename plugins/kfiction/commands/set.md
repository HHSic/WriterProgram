---
description: kfiction 원고 점검 hook 켜기·끄기. Toggle the kfiction hook.
argument-hint: on | off | status
allowed-tools: Bash(node:*)
---

아래 명령을 그대로 실행하고 출력 한 줄을 보여 줍니다. 인자가 비어 있으면 `status`로 실행합니다.

```
node "${CLAUDE_PLUGIN_ROOT}/hooks/kfiction-hook.mjs" set $ARGUMENTS
```

상태 파일은 프로젝트의 `.kfiction/state.json`입니다.
