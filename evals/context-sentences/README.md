# 맥락 문장 후보 평가셋

설계는 [docs/context-sentences.md](../../docs/context-sentences.md). 지금은 R 묶음(관계와 말 단계)만 있다.

| 파일 | 내용 |
|---|---|
| `R.json` | 대화 장면 24개 × 대사 자리 3개 = 72항목. 모든 장면과 정답 대사는 직접 작성 (검수 전 초안) |

## 실행

평가 실행기는 `crates/evals`의 `context-eval`이다. 형태소 사전을 넣느라 첫 빌드는 몇 분 걸린다.

```bash
cargo build --release -p writer-evals
B=target/release/context-eval
SET=evals/context-sentences/R.json

$B check  $SET                         # 형식 점검 + 정답 대사가 100점인지 확인
$B prompt $SET --condition x2 --item R12-3   # 모델에 보낼 내용 보기 (API 호출 없음)

export ANTHROPIC_API_KEY=...           # OPENAI_API_KEY, GEMINI_API_KEY도 됨
$B run $SET --condition x1 --out evals/context-sentences/results/x1.jsonl --model claude-sonnet-5
$B run $SET --condition x2 --out evals/context-sentences/results/x2.jsonl --model claude-sonnet-5
$B score $SET evals/context-sentences/results/x1.jsonl evals/context-sentences/results/x2.jsonl [--detail]
```

- `run`은 결과 파일에 덧붙이고 이미 끝난 항목은 건너뛴다. 끊겨도 같은 명령으로 이어서 돌린다. `--limit 5`로 먼저 몇 개만 돌려 볼 수 있다.
- 조건: `x0` 직전 원고만, `x1` + 인물 카드(지금 설정집 형식), `x2` + 관계 표와 장면 카드.
- 결과 파일(JSONL)은 git에 올리지 않는다(`.gitignore`). 결과 파일에는 후보 원문과 토큰 수가 남는다. 점수 규칙을 고친 뒤에는 API를 다시 부르지 않고 `score`만 다시 돌리면 된다.
- 샘플링 설정은 회사 기본값을 쓴다(`crates/ai`와 같음).

## 점수

후보마다 100점: 계약(관문) · 말 단계 40 · 호칭 20 · 자칭 20 · 문체 20. 항목 점수는 최고 후보 점수다. 자세한 기준은 설계 문서 7.5절.

## 검수할 것

- 자리마다 `level`, `address`, `self` 조건이 장면에 맞는지
- `gold` 대사가 자연스러운지 (판정기 점검에도 쓰인다)
- 말 단계 어미 표(`crates/revise/src/speech.rs`)를 국어학 전공자가 확인
