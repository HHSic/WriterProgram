---
description: kfiction 원고 점검 hook 켜기·끄기, 원고 폴더(예: Google Drive) 정하기. Toggle the kfiction hook or set the manuscript folder.
argument-hint: on | off | status | dir [폴더|off]
allowed-tools: Bash(node:*)
---

인자: `$ARGUMENTS` (비어 있으면 `status`)

- `on`, `off`, `status`: 아래 명령을 그대로 실행합니다.

  ```
  node "${CLAUDE_PLUGIN_ROOT}/hooks/kfiction-hook.mjs" set $ARGUMENTS
  ```

- `dir 폴더`: 폴더 경로의 `\`를 `/`로 바꾸고 큰따옴표로 감싸 실행합니다. `dir`만 있으면 지금 설정을 보이고, `dir off`는 설정을 지웁니다.

  ```
  node "${CLAUDE_PLUGIN_ROOT}/hooks/kfiction-hook.mjs" set dir "G:/내 드라이브/소설"
  ```

출력 한 줄을 보여 줍니다. 원고 폴더를 정했으면 이 대화에서도 새 원고는 그 폴더에 씁니다.

hook 상태 파일은 프로젝트의 `.kfiction/state.json`, 원고 폴더 설정은 사용자 폴더의 `~/.kfiction/config.json`입니다.
