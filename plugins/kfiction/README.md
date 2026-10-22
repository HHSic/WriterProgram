# kfiction

한국어 소설·웹소설을 쓰는 작가를 위한 Claude Code 플러그인과 Codex 연동. 채팅에서 원고의 한 자리에 들어갈 문장·대사 후보를 맥락에 맞게 내고, 원고 파일을 쓰면 점검기가 알려 준다.

목표는 "잘 쓴 소설"을 대신 쓰는 것이 아니라 **주어진 맥락에서 작가가 고를 만한 문장 후보**다. 근거와 설계는 WriterProgram 저장소의 [연구 보고서](../../reports/한국어%20소설%20생성%20프롬프트%20연구.md)와 [과제·평가셋 설계](../../docs/context-sentences.md)에 있다.

## 설치

Claude Code:

```
/plugin marketplace add HHSic/WriterProgram
/plugin install kfiction@writerprogram
```

터미널에서는 `claude plugin marketplace add HHSic/WriterProgram` 뒤에 `claude plugin install kfiction@writerprogram`. Node.js 20 이상이 필요하고 따로 `npm install`할 것은 없다.

Codex CLI (Claude Code 플러그인을 읽지 못하므로 확장 지점에 직접 놓는다):

```
git clone https://github.com/HHSic/WriterProgram
node WriterProgram/plugins/kfiction/integrations/codex/install.mjs --dry-run   # 계획 보기
node WriterProgram/plugins/kfiction/integrations/codex/install.mjs             # AGENTS.md 절, skill, MCP 도구, prompt
node WriterProgram/plugins/kfiction/integrations/codex/install.mjs --with-hooks   # 원고 점검 hook 추가
node WriterProgram/plugins/kfiction/integrations/codex/install.mjs --uninstall
```

설치 전에 `config.toml`과 `AGENTS.md`를 백업한다. skill은 `~/.agents/skills/kfiction/`에 복사되므로 클론한 폴더를 옮기면 다시 설치한다.

ChatGPT (웹 프로젝트·GPT, Work 모드 스킬, 데스크톱 앱 개인 플러그인): [integrations/chatgpt](integrations/chatgpt/README.md). 웹에서는 `integrations/chatgpt/dist/instructions.md`를 프로젝트 지침에 붙이고 `dist/knowledge/` 파일을 올리는 방법이 가장 확실하다. 점검기는 명령을 실행할 수 없어 빠진다.

## 쓰는 법

채팅에서 원고를 붙이거나 원고 파일을 가리키고 요청하면 된다. 한국어 소설을 이어 쓰거나 고치는 요청이면 `kfiction` 스킬이 저절로 쓰인다.

```
서윤이 팀장한테 오늘은 일찍 들어가겠다고 말하는 대사 후보 줘
1화.md 23번째 줄 다음 문장 이어서 3개
이 문장 번역투 같아, 다르게 써 줘: "그 사람으로부터 연락을 받았다."
```

답은 이렇게 온다.

```
맥락: 시점 서윤(3인칭) · 과거형 · 서윤→도윤 해요체, "팀장님" · 도윤→서윤 해체, "한 대리"
① “팀장님, 내일 아침까지 본문으로 옮겨 놓을게요. 오늘은 먼저 들어가 봐도 될까요?”
   · 할 일을 먼저 약속하고 허락을 구함
② …
```

맥락 메모가 틀렸으면 고쳐 말하면 된다. 특히 화자→청자 말 단계와 호칭은 맞는지 한번 본다.

| Claude Code | Codex | 하는 일 |
|---|---|---|
| `/kfiction:candidates [메모]` | `/prompts:kfiction-candidates` | 문장·대사 후보 3개 |
| `/kfiction:check [파일]` | `/prompts:kfiction-check`, MCP `kfiction_check` | 원고 점검 리포트 |
| | MCP `kfiction_levels` | 대사별 말 단계 어림 |
| `/kfiction:set on\|off\|status` | `.kfiction/state.json` 직접 수정 | 원고 점검 hook 켜기·끄기 |
| `/kfiction:set dir <폴더>` | | 원고 폴더 정하기 (Google Drive 등) |

명령줄에서도 쓸 수 있다.

```
node plugins/kfiction/bin/kfiction.js check 1화.md [--all] [--json]
node plugins/kfiction/bin/kfiction.js levels 1화.md
```

## 원고 폴더 (Google Drive)

원고는 보통 `claude`를 켠 폴더에 써진다. 원고 폴더를 정해 두면 어디서 켜든 새 원고가 그 폴더로 간다. Google Drive 데스크톱 앱이 만든 폴더를 고르면 원고가 Drive에 저장된다.

```
/kfiction:set dir G:\내 드라이브\소설                                   (Windows)
/kfiction:set dir ~/Library/CloudStorage/GoogleDrive-계정/내 드라이브/소설   (Mac)
/kfiction:set dir          지금 설정 보기
/kfiction:set dir off      설정 지우기
```

- 설정은 `~/.kfiction/config.json`에 남고 모든 프로젝트에 적용된다. 세션을 시작할 때 hook이 원고 폴더를 모델에게 알린다.
- 폴더가 없으면 만든다. 바로 위 폴더(예: `내 드라이브`)는 있어야 한다. 드라이브가 꺼져 있어 폴더가 안 보이면 세션 시작 때 알리고, 모델은 원고를 쓰기 전에 묻는다.
- `/kfiction:check 1화.md`처럼 이름만 주면 지금 폴더에 없을 때 원고 폴더에서 찾는다.
- Claude Code는 켠 폴더 밖에 파일을 쓸 때 허락을 묻는다. 묻지 않게 하려면 `claude --add-dir "G:\내 드라이브\소설"`로 켜거나 원고 폴더 안에서 켠다.
- 같은 원고를 두 컴퓨터에서 동시에 고치면 Drive가 사본을 따로 만들 수 있다.
- Codex 연동은 아직 이 설정을 읽지 않는다.

## WriterProgram 작품과 함께 쓰기

WriterProgram 앱의 작품 폴더를 Claude가 직접 읽고 쓰게 하려면 `writer-mcp`를 MCP 서버로 붙인다. 회차 목차·본문·설정집·메모를 다루고, 본문을 고치기 전에 앱에서 되돌릴 수 있는 기록을 남긴다. 아래 원고 점검 hook은 MCP로 쓴 회차를 보지 못하므로, `writer-mcp`가 같은 점검기를 Rust로 옮겨 쓰기 도구 안에서 돌린다(계약 위반은 저장 전에 거절). 설치와 도구는 [AI 도우미 연결](../../docs/mcp.md).

```
cargo install --path crates/mcp
claude mcp add writerprogram -- writer-mcp serve --project "<작품 폴더>"
```

## 원고 점검 hook

`.md`나 `.txt` 원고를 쓰거나 고치면 점검기가 돈다. 기술문서나 메모는 건드리지 않는다(한글 비율, 대사와 서술 문장으로 소설 원고인지 먼저 본다).

- **계약 위반**: 원고 안에 "다음은 ~입니다", "원하시면", "물론이죠. 장면을 이어 썼습니다" 같은 어시스턴트 말투(글을 두고 하는 말. 따옴표 안 대사는 빼고)나 마크다운 강조·목록이 섞이면 모델에게 돌려줘 고치게 한다. 파일당 2회까지.
- **경고**: 겹친 피동, '의' 연쇄, 같은 끝맺음·같은 시작 반복, 연결 어미 뒤 쉼표, 부정 대조 반복, 영문. 사용자에게만 한 줄로 알린다. 모델을 멈추지 않는다.
- **참고**: 번역 투, 상투어 후보, 줄표, 문장 길이, 대사만 이어짐, 한 대사 안 말 단계 섞임. `/kfiction:check --all`에서만 보인다.

KSTE와 달리 문체 경고로 모델을 막지 않는다. 문체 수치를 0으로 몰면 모델이 문장과 개성을 깎는다. 사람이 쓴 1924년 「운수 좋은 날」도 경고 1건, 참고 13건이 나온다.

## 한계

- 점검기는 형태소 분석 없이 문자 규칙으로 돈다. 말 단계 어림은 "바다."처럼 명사로 끝나는 문장을 해라체로 읽는 식의 오판이 있다. WriterProgram 평가셋의 정답 대사 72개로 확인하면 72개를 모두 읽고 높임·안높임 구분이 71개 맞는다. 이 대사들은 판정 규칙을 만들 때 본 것이라 실제 정확도보다 높게 나온다.
- 대사의 화자를 정하지 못하므로 "이 관계에 맞는 말 단계인가"는 점검기가 보지 못한다. 스킬이 모델에게 직접 확인하게 한다.
- 상투어 후보 목록은 영어 목록을 옮긴 가설이다. 한국어 소설에서 잰 적이 없다.
- 이 플러그인을 쓰면 원고가 실제로 좋아지는지는 아직 재지 않았다. WriterProgram의 `context-eval`(R 묶음 72항목)로 맥락 형식의 효과를 재는 것이 다음 단계다.

## 구성

| 위치 | 내용 |
|---|---|
| `skills/kfiction/SKILL.md` | 작문 지침: 과제, 순서, 확인 목록, 문체 카드 |
| `skills/kfiction/references/` | 맥락 잡기, 말 단계와 호칭, 비유 절차, 어휘, 고치기, 작품 해석 |
| `lib/` | 점검기(문단·문장 나누기, 말 단계 어림, 규칙, 리포트) |
| `hooks/` | Claude Code PostToolUse hook |
| `commands/` | `/kfiction:candidates`, `/kfiction:check`, `/kfiction:set` |
| `integrations/codex/` | AGENTS.md 절, MCP 서버, hook, prompt, 설치 스크립트 |
| `integrations/chatgpt/` | ChatGPT용 지침·지식 파일·스킬 zip(`dist/`)과 데스크톱 앱 설치 스크립트 |
| `plugin.json` | OpenAI 플러그인 형식(agent-plugins) 매니페스트. `.claude-plugin/plugin.json`은 Claude Code용 |
| `test/` | `npm test` (node --test) |
| `scripts/lint-cases.mjs` | 점검기 시험 사례(`test/fixtures/lint-cases.json`)를 만든다. `writer-mcp`의 Rust 점검기도 같은 사례로 시험하므로, 규칙을 바꾸면 다시 돌린다 |
