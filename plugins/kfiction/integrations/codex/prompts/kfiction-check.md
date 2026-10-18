---
description: 한국어 소설 원고 점검
argument-hint: FILE=<파일>
---

`$FILE` 원고를 점검합니다. 인자가 없으면 최근에 쓴 원고 파일을 대상으로 합니다.

1. `node "${CLAUDE_PLUGIN_ROOT}/bin/kfiction.js" check "$FILE"` 을 실행하고 리포트를 그대로 보여 줍니다.
2. 파일은 고치지 않습니다.
