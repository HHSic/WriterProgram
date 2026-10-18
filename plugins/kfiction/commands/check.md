---
description: 한국어 소설 원고를 점검하고 리포트를 보여 줍니다. 파일은 고치지 않습니다. Check a Korean fiction manuscript.
argument-hint: "[파일]"
allowed-tools: Bash(node:*), Read
---

`$ARGUMENTS` 원고를 점검합니다. 인자가 없으면 이 대화에서 가장 최근에 쓰거나 고친 원고 파일(.md, .txt)을 대상으로 하고, 정할 수 없으면 파일 경로를 묻습니다.

1. 아래 두 명령을 실행합니다.

   ```
   node "${CLAUDE_PLUGIN_ROOT}/bin/kfiction.js" check "<파일>"
   node "${CLAUDE_PLUGIN_ROOT}/bin/kfiction.js" levels "<파일>"
   ```

2. `check` 리포트를 줄이지 말고 그대로 보여 줍니다. 마지막의 "점검기가 못 보는 것"도 포함합니다.
3. `levels` 결과는 그대로 붙이지 말고, 관계에 비해 말 단계가 의심스러운 대사만 골라 한두 줄로 말합니다. 화자는 원고에서 판단합니다.
4. 참고 항목까지 보려면 사용자가 요청할 때만 `--all`을 붙입니다.
5. 파일을 고치지 않습니다. 고치려면 작가가 고를 문장을 정해 달라고 한 줄로 안내합니다.
