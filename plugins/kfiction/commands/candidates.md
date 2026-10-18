---
description: 원고의 한 자리에 들어갈 문장·대사 후보 3개를 맥락에 맞게 냅니다. Propose three context-fitting sentence or dialogue candidates.
argument-hint: "[무엇을 쓸지 한 줄 메모]"
allowed-tools: Read
---

kfiction 스킬의 순서대로 후보를 냅니다. 작가의 메모: `$ARGUMENTS`

1. 이 대화에서 작가가 보여 준 원고나 가장 최근에 다룬 원고 파일에서 자리를 정합니다. 자리가 분명하지 않으면 "어느 문장 뒤인지" 한 줄로 묻습니다.
2. 맥락 메모 한 줄(시점, 시제, 이 자리의 화자→청자 말 단계와 호칭)을 먼저 보이고, 서로 다른 후보 3개와 각각의 한 줄 차이점을 냅니다.
3. 파일은 고치지 않습니다. 작가가 고른 후보를 넣어 달라고 하면 그때 그 후보만 넣습니다.
