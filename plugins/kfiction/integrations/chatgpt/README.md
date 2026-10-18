# ChatGPT에서 kfiction 쓰기

ChatGPT는 Claude Code 플러그인이나 Codex 설치 스크립트를 그대로 읽지 못한다. 하지만 kfiction의 핵심은 지침(skill)이고, OpenAI의 플러그인 형식도 `plugin.json` + `skills/<이름>/SKILL.md`라서 같은 원본을 그대로 쓸 수 있다. ChatGPT에서는 명령을 실행할 수 없으므로 원고 점검기(hook, `kfiction check`)는 빠진다.

어디서 쓰느냐에 따라 방법이 다르다.

| 쓰는 곳 | 방법 | 점검기 |
|---|---|---|
| ChatGPT 웹, 모든 요금제 | **프로젝트 지침** (방법 1) | 없음 |
| ChatGPT 웹 | **GPT 만들기** (방법 1과 같은 파일) | 없음 |
| ChatGPT 웹, Work 모드 | **@skill-creator로 스킬 가져오기** (방법 2) | 없음 |
| ChatGPT 데스크톱 앱 | **개인 플러그인 설치** (방법 3) | 없음 |
| Codex CLI | `integrations/codex/install.mjs` | 있음 |

## 파일

`dist/`에 미리 만들어 두었다. 원본(`skills/kfiction`)을 고친 뒤에는 `node integrations/chatgpt/build.mjs`로 다시 만든다. 테스트가 `dist/`가 최신인지 확인한다.

| 파일 | 쓰는 곳 |
|---|---|
| `dist/instructions.md` | 프로젝트 지침, GPT 지침 칸 (약 4,300자, 한도 8,000자 안) |
| `dist/knowledge/*.md` | 프로젝트 파일, GPT 지식 파일 (참고 문서 5개) |
| `dist/kfiction-skill.zip` | 스킬 올리기. 맨 위 폴더 `kfiction/`에 `SKILL.md`와 `references/` |

## 방법 1. 프로젝트 지침 (가장 확실함)

1. ChatGPT 왼쪽 메뉴에서 새 프로젝트를 만든다(예: "소설 1부").
2. 프로젝트 설정의 지침 칸에 `dist/instructions.md` 내용을 붙여 넣는다.
3. 프로젝트 파일에 `dist/knowledge/`의 다섯 파일을 올린다. 작품 설정집(인물, 관계, 세계관)과 원고도 함께 올리면 맥락 메모가 정확해진다.
4. 그 프로젝트 안에서 대화한다. "1화 마지막 문단 다음 문장 후보 줘", "도윤이 서윤한테 사표 얘기 꺼내는 대사 써 줘".

GPT로 만들어 여러 대화에서 쓰고 싶으면 GPT 만들기 → 구성에서 같은 지침과 지식 파일을 넣는다.

## 방법 2. @skill-creator로 스킬 가져오기

ChatGPT Work 모드에서 `@skill-creator`를 부르고 `dist/kfiction-skill.zip`을 첨부해 "이 스킬을 그대로 만들어 줘"라고 한다. 스킬이 만들어지면 소설 작업 대화에서 저절로 쓰이거나 이름으로 부를 수 있다. 이 경로는 OpenAI 문서에 스킬 생성기로만 설명되어 있고 zip을 그대로 가져오는 동작은 확인하지 못했다. 안 되면 방법 1을 쓴다.

## 방법 3. ChatGPT 데스크톱 앱의 개인 플러그인

```
node plugins/kfiction/integrations/chatgpt/install-desktop.mjs --dry-run   # 계획 보기
node plugins/kfiction/integrations/chatgpt/install-desktop.mjs
node plugins/kfiction/integrations/chatgpt/install-desktop.mjs --uninstall
```

`~/.codex/plugins/kfiction/`에 플러그인을 복사하고 `~/.agents/plugins/marketplace.json`에 항목을 더한다(기존 파일은 백업). 앱을 다시 시작한 뒤 플러그인 목록에서 kfiction을 설치하고, Work 모드에서 `@kfiction`으로 부른다.

저장소를 통째로 받아 쓰는 경우에는 저장소 루트의 `.agents/plugins/marketplace.json`이 이미 kfiction을 가리킨다.

## 웹에 플러그인으로 올리려면

ChatGPT 웹의 플러그인 목록(chatgpt.com/plugins)에 직접 추가하는 길은 지금 두 가지다.

- **원격 MCP 서버 연결**: 플러그인 → + → Add custom MCP server. HTTPS로 접속할 수 있는 서버가 필요하다. kfiction의 MCP 서버(`integrations/codex/mcp`)는 내 컴퓨터에서 도는 stdio 방식이라 그대로는 안 되고, HTTP로 바꿔 어딘가에 올려야 한다. 그러면 웹에서도 점검기(`kfiction_check`)를 쓸 수 있다.
- **워크스페이스에 게시**: 데스크톱에서 설치한 개인 플러그인을 Personal → 메뉴 → Publish로 올린다. 워크스페이스 관리자 권한이 필요하다.

공개 플러그인 디렉터리에 등록하려면 OpenAI 제출 절차(신원·도메인 확인, 테스트 사례)를 거쳐야 한다.
