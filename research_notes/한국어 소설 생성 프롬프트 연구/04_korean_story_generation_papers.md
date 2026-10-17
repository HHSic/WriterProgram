# Korean-language story, novel, web-novel and poetry generation with LLMs, and Korean literature × AI scholarship (research notes 04)

Verification legend (used on every entry):
- **[V]** = I opened the item's own record (abstract page, arXiv page, dataset card, repository page, or full PDF) and read the abstract or text.
- **[L]** = listing-level: seen together with its abstract snippet on a database search-results page that I opened (RISS or KoreaScience). The item's own record was not opened, so details are less certain.
- **[News]** = press article that I opened. It is not peer-reviewed and is used only for production facts or industry context.
- Searches were run on 2026-10-17 across RISS, KoreaScience, arXiv, ACL Anthology, Hugging Face, GitHub, Semantic Scholar and the open web. Full texts behind DBpia/KISS paywalls were not read, so numbers come from abstracts unless the entry says "full text".

## 1. Which papers build or evaluate Korean story, novel, web-novel, fairy-tale or poetry generation? (models, datasets, methods, results)

### Takeaway
Korean technical work on creative generation is small: mostly short conference papers and domestic journal articles, and few report solid quantitative quality results. It still converges on two points. First, long outputs drift, repeat and contradict themselves, so every recent system plans hierarchically (logline → acts/beats → scene specs) and adds an author persona or explicit style rules. Second, the most quantitative Korea-specific evidence about what "AI-sounding Korean" looks like comes from detection research (KatFish; Korean poetry form features), and it can feed a style linter directly.

### Cited Findings

#### 1a. Long-form story and novel generation (planning, persona, style control)

- **[V] 최부광 (Choe Bugwang), 이은찬 (Lee Eunchan), 안상태 (Ahn Sangtae) (Kyungpook National Univ.), "요약문 기반 문학 스타일 문장 생성 / Generating Literature-Style Sentences based on Summarized Text," HCLT 2022 (제34회 한글 및 한국어 정보처리 학술대회), full text read.**
  - Method: KoBART (124M parameters, seq2seq) was fine-tuned to expand a short summary into long literary-style prose. This is summary → original, the reverse of summarization.
  - Data: the literature subset of AI-Hub "요약문 및 레포트 생성 데이터": 10,800 summary–original pairs (9,600 for training; the other 1,200 split equally into validation and test). A second model, KoBART_add, was also trained on the publication, speech and narration summaries.
  - Results: with the extra data, ROUGE-1 rose from 0.383 to 0.391, ROUGE-L from 0.213 to 0.218, and BLEU from 0.0475 to 0.0519.
  - Main qualitative finding: both models follow the summary at the start, but "텍스트의 길이가 길어질수록 관련성이 떨어지는 문장을 생성하는 경향" (relevance falls as the text gets longer).
  - **Prompt implication:** one summary cannot steer a long passage. Restate the scene goal or summary for each chunk and generate shorter segments.
  - Source: [KoreaScience PDF](https://koreascience.kr/article/CFKO202226455339339.pdf)
- **[V] 조동하 (Jo Dongha), 전이슬 (Jeon Isle), 문미경 (Moon Mikyeong) (Dongseo Univ.), "GPT-3를 활용한 시나리오 생성 보조 시스템 / Scenario Generation Assistance System Using GPT-3," 한국컴퓨터정보학회 하계학술대회 논문집 30(2), July 2022, pp. 503–504, full text read.**
  - Method: GPT-3 generates N candidate next sentences. The writer picks one or types their own sentence (N+1 choices), and the loop repeats.
  - Motivation stated in the paper: even good language models "수백 단어가 넘어가는 텍스트에서는 같은 말을 반복하거나 모순되는 문장을 생성" (repeat themselves or contradict themselves once a text passes a few hundred words).
  - The demo was in English. The authors planned Korean follow-ups with HyperCLOVA and KoGPT. There was no quantitative evaluation.
  - **Prompt implication:** at decision points, offer several short alternatives and let the writer steer, instead of one long autonomous continuation.
  - Source: [KoreaScience PDF](https://www.koreascience.or.kr/article/CFKO202232249452420.pdf)
- **[L] 전이슬, 조동하, 문미경, "이미지 캡션 및 재귀호출을 통한 스토리 생성 방법 / Automated Story Generation with Image Captions and Recursive Calls," Journal of the Institute of Convergence Signal Processing 24(1):42–50, 2023, DOI 10.23087/jkicsp.2023.24.1.006.**
  - Method: a CNN with attention captions storyboard images. The captions are fed to KoGPT-2, with recursive calls, to produce scenarios that follow the author's plan.
  - **Prompt implication:** this is an early Korean example of grounding each generation step in an explicit scene plan.
  - Source: [KoreaScience](https://www.koreascience.kr/article/JAKO202331972864504.page)
- **[V] 차수연 (Cha Suyeon, Chung-Ang Univ.), 노영빈 (Noh Youngbin, NCSOFT), 이승진 (Lee Seungjin, NCSOFT), "구조화된 플롯팅과 작가 페르소나를 활용한 언어모델의 장편 이야기 생성 / Long-form Story Generation using Structured Plotting and Author Personas," HCLT 2024 (2024.10a), pp. 39–44.**
  - Problem: LLMs keep coherence and creativity poorly over long sequences.
  - Method: a prompt-based recursive plot framework built on plot theory, plus "author personas" that steer story development and keep a consistent narrative style.
  - Result (abstract only): human evaluation favored the method on plot length, diversity and coherence. The record page gives no scores and names no model.
  - **Prompt implication:** recursive outlining plus an explicit author persona is a recipe tested in Korean for keeping long-form style consistent.
  - Source: [KoreaScience](https://www.koreascience.kr/article/CFKO202404272002240.page)
- **[V] 지은수 (Ji Eunsu) & 유태경 (Yoo Tae-Kyung) (Chung-Ang Univ.), "서사 구조를 따르는 LLM 기반 스토리 생성 연구 / LLM-Based Story Generation Aligned with Narrative Structure," 한국컴퓨터게임학회 논문지 38(3):61–72, July 2025.**
  - Method: from a user's logline, character information and genre, the system builds a three-act structure and then the 15-beat "Save the Cat!" sheet. Each beat is produced by hierarchical generation and prompt chaining. When the user edits any level, the change propagates up and down to keep the story consistent.
  - Evaluation: human stories, single-prompt LLM stories and the system's stories were compared. The system did well on fidelity to narrative structure and on logic. The abstract gives no numbers and does not name the LLM.
  - **Prompt implication:** chaining through a beat sheet works better than single-shot prompting. When the writer edits an outline item, regenerate the beats that depend on it.
  - Source: [KoreaScholar](https://db.koreascholar.com/Article/Detail/443668)
- **[V] 김수진 (Kim Sujin), 양현호 (Yang Hyunho), 김정욱 (Kim Jungwook) (Sangmyung Univ.), "AI기반 장편소설 창작 파이프라인의 신뢰성에 대한 탐색적 연구 / An Explorative Study on a Structure-Style Concurrent Constraint Pipeline for AI-Based Long-Form Novel Creation," 인터넷정보학회논문지 27(3):133–140, 2026, DOI 10.7472/jksii.2026.27.3.133.**
  - Method: novel writing is recast as a three-phase human–LLM pipeline with quality gates: (1) narrative structure design, (2) scene-level specification (장면별 설계서), (3) style discipline (문체 규율).
  - Design: five same-genre novels were produced under five conditions: A (baseline), B (structure only), C (style only), D (structure + style) and E (full integration). Three models blind-rated them on the "FT-24" (Fiction Text-24) scale.
  - Results: B or C alone scored about 5.2. D and E scored about 5.65–5.67, roughly +0.4. The authors read this as a combined effect larger than either constraint alone.
  - Caveats: only five novels, LLM judges, no statistics on the record page, and the models are not named.
  - **Prompt implication:** give the model both a scene spec and an explicit style sheet. WriterProgram's linter plays the role of the "style discipline" gate.
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=380784b010c6543a6aae8a972f9116fb)
- **[V] 이만유 (Lee Manyu), 김예진 (Kim Yejin), 전승화 (Jeon Seunghwa), 오동석 (Oh Dongsuk) (Kyungpook National Univ. / George Washington Univ.), "생애사 프로필 기반 LLM 소설 생성의 창의성 평가 연구: 모델별 서사 생성 특성 비교를 중심으로 / Evaluating the Creativity of Life-History-Based LLM Story Generation," 한국디지털콘텐츠학회논문지 27(1):13–27, 2026.**
  - Method: narrative prompts were built from in-depth interviews with older adults. GPT-3.5, GPT-4o and GPT-4.1 each wrote 100 stories (300 in total), and human raters scored six creativity criteria.
  - Findings by model:
    - GPT-3.5 had the highest rater agreement on elaboration (정교성), which the authors attribute to its simple structure.
    - GPT-4o was stable and balanced.
    - GPT-4.1 was strongest in sensory and emotional expression, but it also had the widest item-level disagreement among raters.
  - The authors conclude that the differences reflect how each model structures narrative and how raters interpret it, more than differences in creativity as such.
  - **Prompt implication:** richer sensory and emotional prose divides readers. Let the writer set the sensory density, and do not treat "more vivid" as "better."
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=33a941b128e7fec9c85d2949c297615a)
- **[V] 권민재 (Kwon MinJae) & 강윤구 (Kang YunGu), "대형 언어 모델에서의 소설 데이터 기반 2차 창작물 생성 효율성 비교 연구," KCC 2025 (한국정보과학회 학술발표논문집 2025(7)):1162–1164.**
  - Method: LLMs were compared on Korean novel data for emotion classification and speaker identification, framed as groundwork for derivative ("2차") creation.
  - Results: EEVE did best, with 70.68 cosine similarity on emotion classification and 52.22% accuracy on speaker identification. Adding an emotion-vocabulary list to the prompt improved performance.
  - **Prompt implication:** even the best model identified the speaker in Korean prose only about half the time. The prompt must carry explicit speaker and character state, and supplying an emotion lexicon helps.
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=78314f60a5e4a3367ecd42904f0c5d65)
- **[L] 박정윤, 김가연, 배병철, "언어 모델을 이용한 장르 단편 소설 생성 및 분석," KSC 2023 (한국정보과학회 학술발표논문집 2023(12)).** No abstract was visible, so the content is unverified. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=d25a83d1f2c6292ac85d2949c297615a)
- **[L] 정용태, 김선혁, 김세훈, 정기현, "AI를 활용한 인터렉티브 소설 생성 시스템 설계 및 구현," KIIT Conference 2025(11).** It combines an LLM narrative engine with a diffusion illustration engine, and the story branches in real time on reader choices. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=8970538954df7e24c85d2949c297615a)
- **[L] 임미영 & 강신재, "도메인 정보가 강화된 워드 임베딩을 사용한 한국어 텍스트 생성," 한국지능시스템학회논문지 29(2), 2019.**
  - An RNN creative-text generator. A model pretrained on general corpora scored higher on rhetorical creativity than a domain-only one.
  - Weak implication: narrow conditioning may flatten rhetoric.
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=7b040ace4ecb54d8e9810257f7042666)

#### 1b. Poetry generation and poetic form

- **[V] 김명선 (Kim Myung-sun), 정우혁 (Jung Woo-Hyuk), 우지환 (Woo Jihwan) (CJ OliveNetworks AI Lab; Korea Univ.), "인공지능 기반 한국어 시 생성 시스템 개발 연구 / A Study on Korean Poetry Generation System Based on Artificial Intelligence," 경영정보학연구 (Information Systems Review) 25(3):43–57, 2023, DOI 10.14329/isr.2023.25.3.043.**
  - Method: interviews with poets shaped eight datasets for eight poetry genres. The system generates "foundational sentences" that support poets rather than replace them.
  - The record names no model and reports no evaluation.
  - **Prompt implication:** the poets wanted genre-specific seed lines and options, an assistant mode rather than finished poems.
  - Source: [KoreaScience](https://www.koreascience.or.kr/article/JAKO202307267862839.page?&lang=ko)
- **[L] 박요한, 정혜지, 강일민, 박천용, 최용석, 이공주, "SeqGAN 모델을 이용한 한국어 시 자동 생성," HCLT 2018, pp. 580–583.**
  - Trained on love poems. The generated poems repeated the same phrases several times.
  - This is a pre-LLM baseline: repetition is the oldest documented failure in Korean poetry generation.
  - Source: [KoreaScience](https://www.koreascience.kr/article/CFKO201832073078975.page)
- **[V] Park Keunhyeung, Yu Seunguk, Kim YoungBin, "Detecting and Guiding LLM-Generated Korean Poetry with Interpretable Form-level Features," arXiv 2608.28986 (Aug 2026), accepted to Findings of EMNLP 2026.**
  - Problem: LLMs write modern Korean poetry that reads like "line-broken prose."
  - Method: the human–LLM gap is measured on four form dimensions, turned into five features:
    - output length;
    - variety of line-final forms and how often they are connective;
    - irregularity of line lengths;
    - adherence to standard orthography.
  - Results:
    - A logistic regression on these features reached 83.60 average AUC-ROC in zero-shot tests on seven unseen LLMs, against 75.84 for KatFishNet.
    - Experts preferred feature-guided poems over an unconstrained GPT-5.2 baseline.
    - Analyses on GPT-5.2 and Gemini-3 show that targeting length, rhythm and line-ending statistics moves output toward the human distribution.
  - **Prompt implication:** for verse and lyrical prose, constrain length, ask for irregular line lengths and varied line endings rather than connective endings, and do not force textbook orthography. These features also work as linter metrics.
  - Source: [arXiv](https://arxiv.org/abs/2608.28986)
- **[V] Lim Iro, Ji Haein, Kim Byungjun, "KPoEM: A Human-Annotated Dataset for Emotion Classification and RAG-Based Poetry Generation in Korean Modern Poetry," arXiv 2509.03932 (2025; v2 Jan 2026); journal reference The Review of Korean Studies 29(1) (2026):161–206.**
  - Data: 7,662 multi-label entries from five influential modern Korean poets, with 44 fine-grained emotion categories. The record gives sub-counts of 7,007 line-level and 615 work-level entries, which sum to 7,622, so one of the figures may be wrong.
  - Results: staged fine-tuning reached F1-micro 0.60, against 0.43 for earlier models. The emotion data was used in a RAG poetry generator that the authors say reflects Korean emotional and cultural sensibility. The abstract page gives no generation metrics.
  - **Prompt implication:** retrieving emotion-tagged lines from the Korean canon as exemplars is a tested way to give generated text a Korean emotional register.
  - Source: [arXiv](https://arxiv.org/abs/2509.03932)

#### 1c. What LLM-written Korean looks like (detection studies that feed the linter directly)

- **[V] Park Shinwoo, Kim Shubin, Kim Do-Kyung, Han Yo-Sub (Yonsei Univ.), "KatFishNet: Detecting LLM-Generated Korean Text through Linguistic Feature Analysis," ACL 2025 (long paper), arXiv 2503.00032.**
  - Data (KatFish): 470 human texts (AIHub student essays, National Institute of Korean Language poems, KIISE paper abstracts) and 1,624 LLM texts (GPT-4o, Solar, Qwen2-72B, Llama-3.1-70B). Genres are argumentative essays, free-verse poetry and abstracts.
  - Findings:
    - LLMs "strictly follow spacing rules," while humans omit spaces for stylistic reasons.
    - Humans show higher POS n-gram diversity.
    - LLMs put commas in far more sentences: 61.03% vs 26.31% in essays and 4.84% vs 2.61% in poems. They also place commas later in the sentence and overuse them after connective endings and conjunctive adverbs. The authors tie this to English punctuation norms absorbed in multilingual training.
    - LLM essays are longer (GPT-4o averages 268 eojeol vs 152 for humans).
    - Poetry shows smaller differences than essays.
  - Detection: KatFishNet improves average AUC-ROC by 19.78% over the best earlier detector.
  - **Prompt/linter implication:** flag comma density (especially after connective endings such as -고/-며/-지만 and after 그리고/하지만), monotone syntax (low POS variety) and over-length. Perfect spacing is itself a tell.
  - Sources: [arXiv HTML](https://arxiv.org/html/2503.00032v4); [ACL Anthology](https://aclanthology.org/2025.acl-long.1030)

#### 1d. Model and production context (HyperCLOVA, KoGPT/SIA, 비람풍, ETRI survey)

- **[V] Kim Boseop et al. (NAVER, 37 authors), "What Changes Can Large-scale Language Models Bring? Intensive Study on HyperCLOVA: Billions-scale Korean Generative Pretrained Transformers," EMNLP 2021 (arXiv 2109.04650).**
  - HyperCLOVA is an 82B GPT-3-style model trained on a 560B-token Korean-centric corpus.
  - The paper introduces HyperCLOVA Studio, an interactive prompt-engineering interface for non-experts ("No Code AI").
  - The abstract reports no creative-writing or story evaluation.
  - Source: [arXiv](https://arxiv.org/abs/2109.04650)
- **[News] KakaoBrain SIA (시아) and 『시를 쓰는 이유』.**
  - SIA is built on KakaoBrain's KoGPT and was further trained on about 13,000 poems by the media-art group Slitscope (슬릿스코프).
  - Users give topic words and commands. Slitscope generated from many prompts, refined the output, and selected the 53 poems in the collection.
  - Published 8 Aug 2022. Part 1 is themed 공 ("0") and Part 2 일 ("1").
  - Source: [iNews24, 2022-08-01](https://www.inews24.com/view/1505892)
- **[News] 『지금부터의 세계』 (2021).**
  - Written by the publisher-developed AI novelist "비람풍" under "소설감독" 김태연.
  - The director set theme, material, setting and characters, made the storyboard, wrote the opening, preface and afterword, and adjusted commands repeatedly until the output was acceptable.
  - The publisher said the sentences needed almost no proofreading and showed some distinctive style.
  - Source: [ETNews, 2023-07-13](https://www.etnews.com/20230713000255)
- **[L] Yu J.J., Nam D.-W., Lee J.S., Lee J.W., Moon S.-W., Song D.Y. (ETRI), "생성형 AI 기반의 콘텐츠 제작 기술 동향과 안전성 이슈 / Trends and Safety Issues in Content-Creation Technology Development Based on Generative AI," 전자통신동향분석 40(2):30–39, 2025, DOI 10.22648/ETRI.2025.J.400204.**
  - A survey of generative-AI production support for novels, webtoons and short video.
  - It states that automation of long-form production "is still limited in many cases."
  - Source: [KoreaScience](https://www.koreascience.kr/article/JAKO202551143204165.page)

### Inferences
- The Korean technical literature yields three repeatable failure modes for long fiction:
  - drift from the brief as length grows (Choe et al. 2022);
  - repetition and contradiction beyond a few hundred words (Jo et al. 2022; Park et al. 2018);
  - weak tracking of characters and speakers (Kwon & Kang 2025).
- Every 2024–2026 system answers these with hierarchical planning plus a persona or style layer (Cha et al. 2024; Ji & Yoo 2025; Kim et al. 2026). A WriterProgram prompt should therefore build in outline → scene spec → prose stages and a style sheet, rather than relying on a single "write the next chapter" instruction.
- The most quantitative Korea-specific evidence on prose texture comes from detection work. Commas, spacing, POS variety, length, variety of line-final forms, regularity of line lengths and orthographic over-correctness (KatFish; Park/Yu/Kim 2026) can be turned almost directly into linter rules.
- The base models named in Korean creative-generation papers are mostly older (KoBART, KoGPT-2, KoGPT, GPT-3) or OpenAI/Google frontier models. I found no paper that evaluates the fiction quality of EXAONE, Solar, Polyglot-Ko, KoAlpaca or HyperCLOVA X specifically.

### Gaps
- I found no Korean paper that fine-tunes or evaluates a model on a Korean web-novel corpus for generation, and no AI Hub web-novel generation dataset. The only AI Hub source used in a generation paper was the literary-summary subset of "요약문 및 레포트 생성 데이터" (Choe et al. 2022). KatFish used AIHub essays and poems from the National Institute of Korean Language corpus (NIKL is the provider of 모두의 말뭉치; the specific sub-corpus was not stated in what I read), but only for detection.
- No LLM-based Korean fairy-tale (동화) generation paper surfaced on KoreaScience. The hits there were 2002–2013 rule-based illustration and animation systems, or the unrelated sense of 동화 ("assimilation").
- I did not read the full texts of Cha et al. 2024, Ji & Yoo 2025, Kim et al. 2026 or Lee et al. 2026 (paywalled or abstract only), so their model names, prompts and score tables are missing.
- The UNIST-listed paper "생성의 나무: 언어모델을 활용한 추리소설의 실마리 분석 및 개선방안" (정석훈, 이재연 et al., 2024 digital-humanities symposium) appeared in search snippets, but its record would not load. It is unverified.
- Hugging Face models such as "Tiny-Ko-Stories" appeared in search results but are model cards, not papers. They were not opened.

## 2. What evaluations or benchmarks exist for Korean creative-writing quality, and how reliable is LLM-as-judge for Korean literary quality?

### Takeaway
I found no dedicated Korean creative-writing or fiction benchmark. Creative writing appears only as a small category inside general Korean chat benchmarks: the "writing" and "roleplay" categories of KoMT-Bench, the "writing" category of LogicKor, and the "Creative Writing" reasoning type of K2-Eval. The Korean evidence on LLM judges is consistent. They are dependable for grammar, fluency and orthography. They miss factual and cultural errors, fail to reproduce human rankings, prefer LLM-written text, and are weakest exactly where literary quality lives (creativity, natural Korean phrasing, honorifics). On those same dimensions, even human raters disagree.

### Cited Findings

#### 2a. Benchmarks with a writing component
- **[V] KoMT-Bench (LG AI Research, 2024).**
  - "An in-house dataset created by translating MT-Bench dataset into Korean and modifying some questions to reflect the characteristics and cultural nuances of the Korean language," reviewed by expert linguists.
  - 80 two-turn questions in eight categories: writing, roleplay, reasoning, math, coding, extraction, stem and humanities.
  - Korean adaptations in writing follow-ups include rewriting "충청도 사투리로" (in Chungcheong dialect) and "모든 문장이 ㄱ으로 시작하게" (make every sentence start with ㄱ).
  - Reported scores include EXAONE 3.0 7.8B Instruct 8.92, Gemma 2 9B 7.92 and Qwen 2 7B 7.69. The card names no judge model.
  - **Implication:** the writing items are short, mostly translated tasks, so KoMT-Bench cannot indicate fitness for Korean fiction.
  - Source: [Hugging Face](https://huggingface.co/datasets/LGAI-EXAONE/KoMT-Bench)
- **[V] LogicKor (instructkr, 2024).**
  - A "한국어 언어모델 다분야 사고력 벤치마크" with an LLM judge called through OpenAI/Azure.
  - The repository is archived. The maintainers write that as model performance "상향 평준화" (converges upward), the top of the leaderboard no longer separates models, and some models drifted from scores that match real-world experience. They plan a "LogicKor Hard."
  - The category list (reasoning, math, writing, coding, comprehension, grammar) and "42 multi-turn prompts" come from third-party model cards seen in search, not from the repository page. The K2-Eval card lists LogicKor as 40 instances, so the counts conflict.
  - **Implication:** LogicKor writing scores (often above 9/10 on model cards) are saturated and judge-dependent, and are not evidence of literary quality.
  - Source: [GitHub](https://github.com/instructkr/LogicKor)
- **[V] K2-Eval (HAE-RAE: Guijin Son, Ko Hyun Woo, Hoyoung Lee, Seunghyeok Hong, Yewon Kim, Jungwoo Kim; 2024).**
  - 90 handwritten instructions that need Korean cultural knowledge, crossing nine knowledge types (including Literature) with seven reasoning types (including Creative Writing). It was written natively, not translated, and the README contrasts this with translated MT-Bench prompts "devoid of cultural relevance."
  - Fifteen human judges scored every response at least twice on a 1–5 scale.
  - Results:
    - HyperCLOVA X scored highest.
    - Korean-tuned EEVE-Korean-Instruct-10.8B and KULLM3 were rated comparably to much larger models (Command-R-Plus-104B, Mixtral-8x22B).
    - An earlier README version states that GPT-4 scores "correlate with human evaluation" but "fail to replicate the rankings determined by humans accurately."
    - Separability was 73.76%, against LogicKor 52.94%, MT-Bench 22.60% and Arena-Hard-v0.1 87.40%.
  - **Implication:** for Korean cultural and literary tasks, human judges ranked models differently from GPT-4, and Korean-tuned models punch above their size.
  - Sources: [Hugging Face card](https://huggingface.co/datasets/HAERAE-HUB/K2-Eval); [earlier README with GPT-4 judge statement](https://huggingface.co/datasets/HAERAE-HUB/K2-Eval/raw/03c9125d0ec9c1dce627adc81f2e3f928917a844/README.md)
- **[V] Wang Xiaonan, Yeo Jinyoung, Lim Joon-Ho, Kim Hansaem, "KULTURE Bench: A Benchmark for Assessing Language Model in Korean Cultural Context," PACLIC 38 (2024), arXiv 2412.07251.**
  - Datasets of cultural news, idioms and poetry test comprehension and reasoning at word, sentence and paragraph level.
  - Models show "significant room for improvement" on deeper Korean culture. This is comprehension, not generation.
  - **Implication:** models' grasp of Korean idioms and poetry is still shaky, so idiom and poetry allusions in generated prose need checking.
  - Source: [arXiv](https://arxiv.org/abs/2412.07251)

#### 2b. Reliability of LLM-as-judge in Korean
- **[V] Son Guijin, Ko Hyunwoo, Lee Hoyoung, Kim Yewon, Hong Seunghyeok, "LLM-as-a-Judge & Reward Model: What They Can and Cannot Do," arXiv 2409.11239 (2024), introducing the KUDGE Korean–English meta-evaluation set.**
  - Findings:
    1. English evaluation ability transfers to Korean and predicts Korean judging better than the judge's Korean proficiency does.
    2. Judges often fail to detect or penalize factual errors, cultural misrepresentations and unwanted language.
    3. State-of-the-art judges struggle with difficult prompts in both languages.
  - A search snippet (not the abstract) gives 5,012 Korean human annotations; this figure is unverified.
  - **Implication:** an LLM judge will not reliably catch culturally wrong details (period customs, honorific misuse) in Korean fiction.
  - Source: [arXiv](https://arxiv.org/abs/2409.11239)
- **[V] Kim Seungyoon & Kim Seungone, "Can Language Models Evaluate Human Written Text? Case Study on Korean Student Writing for Education," arXiv 2407.17022 (2024, work in progress).**
  - 100 texts by 32 Korean students across 15 writing types, judged by GPT-4-Turbo on grammaticality, fluency, coherence, consistency and relevance.
  - Judges "can reliably assess grammaticality and fluency," as well as "more objective types of writing," but "struggle with other criteria and types of writing."
  - **Implication:** limit automated judging to mechanical criteria.
  - Source: [arXiv](https://arxiv.org/abs/2407.17022)
- **[V] Park Shinwoo & Han Yo-Sub (Yonsei Univ.), "When LLM Essays Outscore Student Essays: What a Korean Writing Rubric Rewards and Where Readers Disagree," arXiv 2601.19913 (Jan 2026, v4 Aug 2026).**
  - Method: three Korean language and literature majors scored 6 student essays and 24 essays from four LLMs on a 16-criterion, 100-point rubric.
  - Findings:
    - The pooled LLM mean beat the student mean by 18.35 points.
    - Orthographic norms and genre-appropriate register account for 8.19 points (44.7%) of the gap. All 72 LLM ratings hit the orthography maximum, and 70 hit the register maximum.
    - Students had the highest or tied-highest means on both creativity criteria and on natural Korean phrasing, which are also the criteria where readers agreed least.
  - **Implication:** LLM Korean is "too correct." Rubrics reward it, while creativity and naturalness, the qualities WriterProgram cares about, are under-rewarded and hard to rate.
  - Source: [arXiv](https://arxiv.org/abs/2601.19913)
- **[V] Kim Sungeun & Oh Dongsuk (Kyungpook National Univ.), "Evaluating Creativity: Can LLMs Be Good Evaluators in Creative Writing Tasks?," Applied Sciences 15(6):2971, 2025, DOI 10.3390/app15062971.**
  - Criteria: fluency, flexibility, elaboration, originality, usefulness and creativity strategies.
  - LLMs rated more consistently and reached higher inter-annotator agreement than humans, but struggled with "nuanced, culturally specific, and context-dependent aspects of creativity."
  - The record page does not state the language of the texts.
  - **Implication:** consistency is not validity. An LLM judge can be reliably wrong on culturally specific creativity.
  - Source: [KNU research portal](https://research.knu.ac.kr/en/publications/evaluating-creativity-can-llms-be-good-evaluators-in-creative-wri/)
- **[V] Shafayat Sheikh, Yoon Dongkeun, Jang Woori, Choi Jiwoo, Oh Alice, Jung Seohyon, "A 2-step Framework for Automated Literary Translation Evaluation: Its Promises and Pitfalls," arXiv 2412.01340 (v1 Dec 2024; v3 Sept 2025).**
  - The framework evaluates English→Korean literary machine translation and correlates with humans better than traditional MT metrics.
  - It still falls short of inter-human agreement, "especially in metrics like Korean Honorifics."
  - "LLMs tend to favor translations generated by other LLMs."
  - **Implication:** expect self-preference bias if Claude judges Claude-written Korean prose, and expect honorific errors to go undetected.
  - Source: [arXiv](https://arxiv.org/abs/2412.01340)
- **Cross-references from section 1:**
  - Kim et al. 2026 (Sangmyung) relied on three LLM judges for its FT-24 scores ([RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=380784b010c6543a6aae8a972f9116fb)).
  - Lee et al. 2026 found that human raters diverged most on GPT-4.1's sensory-rich stories ([RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=33a941b128e7fec9c85d2949c297615a)).

#### 2c. Human detection of AI-written Korean literary text
- **[V] 정보영 (Hanyang Univ.), 유현성 (Konkuk Univ.), 김호애, "생성형 AI 시대, 인간 창작의 고유성에 관한 실증 연구: 인문실험을 통한 AI 생성과 창작의 개념적 재정립," 글로벌 거버넌스와 문화 6(1):197–223, 2026.**
  - In a "마피아 글쓰기 게임," three working writer-researchers designed a poetry-writing method, generated AI poems by prompt engineering, and then cross-evaluated anonymized poems.
  - All three identified the AI poems ("mafia") with 100% success and built a consistent framework of differences. The record does not name the model or list the cues.
  - The authors conclude that AI can produce poetic form but not output structurally equal to human creation.
  - **Implication:** experienced Korean writers can spot AI verse. WriterProgram output will be read by exactly such people.
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=916b4751430f12d87ecd42904f0c5d65)
- **[News] Economist Korea, 2026-08-01, 원태영.**
  - Readers quickly spot AI-like translationese (번역투) and flat writing in web novels.
  - An industry source says AI-only text "아직은 어딘가 엉성한 느낌" (still feels somehow rough).
  - Source: [Economist Korea](https://economist.co.kr/article/view/ecn202607280044)

### Inferences
- In WriterProgram, an LLM judge should only check verifiable properties: spelling, consistency with the setting bible, POV, tense, speaker attribution and banned-pattern counts. Literary quality judgments (freshness of metaphor, naturalness, voice) need the writer's own judgment or a human panel, because Korean judges do poorly there (KUDGE, K2-Eval, Kim & Oh 2025), even humans disagree (Park & Han 2026; Lee et al. 2026), and LLMs favor LLM text (Shafayat et al.).
- "Too correct" is a measurable signal. LLM essays hit the ceiling on orthography and register (Park & Han 2026), LLM poems adhere more closely to standard orthography (Park/Yu/Kim 2026), and LLMs follow spacing strictly (KatFish). A fiction prompt should allow deliberate, character-motivated deviations: colloquial contractions, dropped particles in dialogue, fragments.
- Existing Korean benchmarks' writing tasks are short, translated or generic, and LogicKor is saturated. WriterProgram will need its own evaluation set: writer-rated Korean scene continuations scored on genre fit, voice consistency, figurative freshness and the linter metrics.

### Gaps
- No Korean fiction or web-novel generation benchmark was found, and no study tests LLM judges on Korean fiction specifically. The existing judge studies use student essays, QA pairs or translations.
- The KoMT-Bench card names no judge model. LogicKor's category list and question count come only from third-party pages, and they conflict with the K2-Eval card (40 instances vs "42 prompts").
- Kim & Oh 2025 does not state the language of its data on the record page.
- A Chinese web-novel benchmark (WebNovelBench, EACL 2026 Findings) appeared in a search snippet but was not opened. I found no Korean analogue.

## 3. What have Korean humanities and literary-studies scholars published about AI-written fiction and poetry?

### Takeaway
Korean literary scholars produced a sizable body of mostly KCI-indexed work on AI authorship between 2019 and 2026. On the two landmark books they converge:
- 『지금부터의 세계』 (2021, 비람풍 with 김태연) is a meaningful first but falls short of established novelists: thin characters, implausible plot, raw information dumps and unrefined slang.
- 『시를 쓰는 이유』 (2022, SIA) contains at least one poem, 「오래된 집」, judged comparable to established poets, but it tends toward declarative, verbose, binary statements of scientific fact.

The dominant theoretical position is that AI is a dialogic co-creator, not an autonomous author.

### Cited Findings

#### 3a. Studies of the landmark AI-authored books
- **[V] 오태호 (Oh Tae-Ho), "2020년대 초반 인공지능(AI) 문학 창작의 현재성 고찰 - 장편소설 『지금부터의 세계』와 시집 『시를 쓰는 이유』를 중심으로," 우리문학연구 79:425–454, 2023, DOI 10.20864/skl.2023.7.79.425.**
  - On the novel: it is the first AI-involved work near full-volume length, and its use of a missing-person motif and detective-fiction techniques within a web of character relationships is "유의미한 창작 행위" (a meaningful creative act). However:
    - character personalities are not adequately realized;
    - the plot lacks organic plausibility;
    - math and science information is presented raw rather than integrated into the story;
    - colloquial language and slang are used without refinement.
  - Verdict on the novel: not yet at the level of established authors. It was a collaboration with novelist 김태연.
  - On the poems: they explore human identity earnestly, and 「오래된 집」 reaches the level of established poets. Weakness: mathematical and scientific facts are stated declaratively and laid out in a verbose, dichotomous frame.
  - **Prompt implication:** these weaknesses are the anti-patterns to ban. Dramatize information through character action instead of exposition. Give each character a motivated personality and voice. Keep slang inside character dialogue. Avoid lecturing, binary "A or B" reflection.
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=01faa8a5a6cb3a3a7f7a54760bb41745)
- **[V] 서경숙 (Chungnam National Univ.), "AI 창작 기술에 따른 인공지능 소설의 유형 분석: 『파르마코—AI』, 『지금부터의 세계』, 『길 위 1번지』를 중심으로 / Studies on Forms of AI-Generated Fictions according to AI Writing Technologies," 현대영어영문학 68(2):75–92, 2024.**
  - Analyzes *Pharmako-AI* (2020), *The World from Now* (2021) and *1 the Road* (2018) by generation process: the AI program used, the training data and the degree of human involvement. Each book explains its text-generation principle in its preface.
  - Argues that differences in technology and human involvement produce distinct literary features.
  - **Implication:** how a book was made (curation and level of control) shapes how it reads. A high-control, plan-first workflow is the human-involvement pattern that WriterProgram is choosing.
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=9595bcbd5074916f6aae8a972f9116fb)
- **[V] 이경수 (Chung-Ang Univ.), "공동 창작 주체로서의 생성형 인공지능의 가능성과 과제," 인공지능인문학연구 18:9–48, 2024.**
  - Examines the AI-involved collections 『시를 쓰는 이유』 and 『9+i』 and compares poems by two AI models from The Q Group (KAIST Graduate School of Culture Technology).
  - Concludes that generative AI is hard to regard as an independent creative subject and is better treated as a dialogic co-creative partner. Before building AI poetry models, one should ask why we want AI poetry and what kind of poetry we want.
  - Proposes hybrid imitation and parody poetry in liberal-arts education.
  - **Implication:** a dialogic, iterative mode (propose → writer reacts → revise) fits Korean literary expectations better than one-shot drafting.
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=9b2931e7dd2188b44884a65323211ff0)
- **[V] 정재훈 (Jeoung Jae-hoon), "인공지능 글쓰기와 소설의 미래 - 박금산 장편소설 『AI가 쓴 소설』을 중심으로," 비평문학 90:255–275, 2023.**
  - Uses Park Geum-san's novel, in which a novelist protagonist confronts AI writing.
  - Argues that AI writing democratizes authorship but threatens writers' livelihoods, that corporate control of AI tools risks turning writing into platform labor, and that some writers deliberately choose inefficiency and unpredictability against corporate efficiency.
  - **Implication:** WriterProgram should present itself as a tool under the writer's control and respect deliberate "inefficient" stylistic choices.
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=2ff19cf0b3fdb7a34884a65323211ff0)

#### 3b. Poetics and authorship debates (2017–2025)
- **[L] 고봉준, "인공지능(AI)은 시를 쓸 수 있는가 - 인공지능 시대에 시-쓰기에 대한 몇 가지 물음," 비평문학 89, 2023.** Reviews the 권보연–김언 debate over whether AI-generated text counts as poetry, given authorship and individuality. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=e00a88ff30be158c7f7a54760bb41745)
- **[L] 정끝별, "인공지능 시대 한국 현대시의 생성시학," 이화어문논집 60, 2023.** Places AI-generated poetry within the lineage of Korean modern poetics through analyzed texts. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=aae7139d95d0772c6aae8a972f9116fb)
- **[L] 유가은, "인공지능 창작 주체와 대화적 시쓰기," 어문논집 98, 2024.** Argues that contemporary poetry's dialogic character makes AI poetry comparable to human-written work. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=4501fa82267fa0bb47de9c1710b0298d)
- **[L] 김소륜, "포스트휴먼의 글쓰기, 창작인가 조합인가? - '생성형 AI'를 활용한 소설 창작에 관한 고찰," 돈암어문학 44, 2023.** Reviews AI-assisted fiction and argues against the binary of AI as either autonomous author or mere tool. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=d3e2d26c59e581817f7a54760bb41745)
- **[L] 신기선, "인공지능 문학의 가능성과 한계에 대한 연구," 스토리콘텐츠 5, 2024.** A theoretical discussion of GPT-3's literary creativity, its limits and its narrative capacity. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=8394854b1fde3f846aae8a972f9116fb)
- **[L] 이재연 & 한남기, "창작 보조기에서부터 문장 생성기까지: 글쓰기 기계의 과거와 현재," 한국문학연구 67, 2021.** Historical review from rule-based story generators (including the Korean StoryHelper) to neural sentence generators. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=d44afe65479690c26aae8a972f9116fb)
- **[L] 김요한 & 김겸섭, "디지털 시대 글쓰기의 도구적 전환 - 켈만의 인공지능 소설 쓰기를 중심으로," 독일어문학 100, 2023.** From Daniel Kehlmann's AI-writing experience, argues that statistical writing lacks meaning and consciousness. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=73582f6debae9088b7998d826d417196)
- **[L, search snippet] 이정엽, "AI를 통한 글쓰기와 작가의 운명-「コンピュータが小説を書く日」을 중심으로," published by 한국현대소설학회 (journal inferred to be 현대소설연구), 2017.**
  - Analyzes the Nagoya-lab AI story; finds it imitates and substitutes existing sentences rather than writing creatively.
  - Pre-2019 background only.
  - Source: [Soonchunhyang ScholarWorks](https://scholarworks.bwise.kr/sch/handle/2021.sw.sch/8006)

#### 3c. Field-level surveys and creative-writing pedagogy
- **[V] 박인성 (Busan Catholic Univ.), "디지털 전환 시대 현대한국문학 연구의 동시대성 - 웹소설·대중문화 콘텐츠·AI 연구 경향을 중심으로," 현대소설연구 100:77–109, 2025.**
  - Meta-review: web novels, popular-culture content and generative AI are reshaping Korean literary studies.
  - AI research brings in posthumanism, with AI seen as a subject rather than only an object of study.
  - The abstract gives no concrete analysis of AI-written fiction.
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=2e85e4230fedf95d7ecd42904f0c5d65)
- **[L] 김지혜, "생성형 인공지능을 활용한 소설 창작 교육의 가능성," 어문연구 51(4), 2023.** ChatGPT, Bard and Midjourney for ideation, style practice and ethics in novel-writing education. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=e61233ba27d98543b7998d826d417196)
- **[L] 이운영, "인공지능 시대의 시 창작 교육 방향 탐색," 청람어문교육 98, 2024.** Reviews SIA's poems and proposes AI–learner collaboration, process-oriented writing and ethical accountability. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=a2602bb4563ea09747de9c1710b0298d)
- **[L] 이종원, "기술창작시대의 문학 작품과 현대시 교육," 문학교육학 87, 2025.** Classifies AI as tool, co-creator or autonomous creator, and stresses planning and iterative revision. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=fec37d6d78de6141b7998d826d417196)
- **[L] 강정화, "예비 국어교사를 대상으로 한 인공지능 활용 시 창작 수업 사례 연구," 인문사회과학연구 67(4), 2024.** Prompt design kept students in the creative role, and creative thinking and interest rose. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=3c4f03debca69928d18150b21a227875)
- **[L] 조은영 & 김지윤, "인공지능(ChatGPT)을 활용한 중학생 시 쓰기 교육 실제," 학습자중심교과교육연구 23(21), 2023.** With 148 middle-schoolers, students were interested but struggled to write effective prompts. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=e6f34b78f6aa6edd47de9c1710b0298d)
- **[L] 양나리, "문예창작 전공생의 AI 협업 창작 경험에 관한 탐색적 연구: <AI 백일장> 활동을 중심으로," 예술교육연구 24(1), 2026.** Creative-writing majors rated the AI's writing ability as low, felt collaboration constrained their creativity, and refused authorship of the joint work. The study reads this as identity negotiation rather than rejection of the technology. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=7a9c1e94273a954a6aae8a972f9116fb)
- **[L] 김민정, "스토리IP 시대의 AI활용 창작 교육의 실천 모델 연구," 예술교육연구 24(3), 2026.** With 48 first-year creative-writing students, learners saw AI as a productive helper and developed "클리셰를 직시하는 메타 창작 인지" (meta-creative awareness that confronts clichés). — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=04f4db6671c83a184884a65323211ff0)
- **[L] 윤나라, "콘텐츠 창작과 AI 리터러시 — LLM의 패러프레이징을 중심으로," 인문콘텐츠 72, 2024.**
  - Uses Stephen Marche's novella, written mostly by an LLM, to argue that writers need AI literacy and must act as supervisors and orchestrators.
  - Only a journal-level record was found, so the source is the search page: [RISS search "생성형 AI 소설 창작"](https://www.riss.kr/search/Search.do?isDetailSearch=N&searchGubun=true&viewYn=OP&query=%EC%83%9D%EC%84%B1%ED%98%95+AI+%EC%86%8C%EC%84%A4+%EC%B0%BD%EC%9E%91&iStartCount=0&iGroupView=5&icate=all&colName=re_a_kor&order=%2FDESC&onHanja=false&strSort=RANK&pageScale=50&fsearchMethod=search&isFDetailSearch=N&sflag=1&searchQuery=%EC%83%9D%EC%84%B1%ED%98%95+AI+%EC%86%8C%EC%84%A4+%EC%B0%BD%EC%9E%91&pageNumber=1)

### Inferences
- Korean literary critics' criticisms of AI fiction (오태호 2023), together with the cliché-awareness and authorship-refusal findings from creative-writing classrooms (양나리 2026; 김민정 2026), map onto craft rules: no raw exposition, motivated characters, controlled slang, no lecturing binaries, no clichés. These belong in the system prompt as explicit "do not" items.
- Both landmark "AI books" were heavily human-curated: a director-designed storyboard and repeated re-prompting for 비람풍, and selection of 53 poems from many generations for SIA. Their best results came from human selection over many candidates, which supports a generate-several-and-let-the-writer-choose design.
- The scholarly framing is "AI as co-creator, writer as orchestrator" (이경수 2024; 김소륜 2023; 윤나라 2024). This framing is also the one least likely to provoke resistance from literary novelists.

### Gaps
- I found no article in 한국문예창작 or 한국문학이론과 비평 specifically on AI-written fiction. The RISS queries surfaced related journals instead (비평문학, 우리문학연구, 돈암어문학, 예술교육연구). Titles in those two journals may exist but did not appear in my queries.
- The only 대중서사연구 hit (노대원 2024, "인공지능은 기후 위기를 해결할까?") treats AI as a theme in Korean SF, not AI-written fiction.
- Full texts were not read, so the specific stylistic cues critics used (vocabulary, imagery, endings) are not extracted beyond the abstracts.
- I did not find peer-reviewed analyses of later commercially published Korean AI-assisted books (2023–2026) beyond 『AI가 쓴 소설』 (박금산) and 『9+i』.

## 4. What do studies of the Korean web-novel industry and writers say about AI adoption, co-writing practices and writers' perceptions? (including Korean HCI co-writing research)

### Takeaway
The best evidence comes from two 2026 qualitative studies by 조희정 and 박기수, of 10 aspiring and 8 professional web novelists. Writers use ChatGPT, Claude and Gemini for research, planning, world-building and revision. They discard or heavily rewrite AI drafts because of world-building errors, genre and period mismatch, insufficient length and information leaking across episodes. First drafts and pivotal scene or plot decisions are treated as non-delegable. Fear of rating attacks and lost reader trust makes writers hide even auxiliary AI use. Platform analyses add that tools fail at world-bible retention, social codes and Korean genre tags (~물). Korean HCI work on co-writing is thin; TaleBrush (CHI 2022) is the main verified example.

### Cited Findings

#### 4a. Writers' use and perceptions
- **[V] 조희정 (Cheonggang College of Culture and Industry) & 박기수 (Hanyang Univ.), "웹소설 작가 지망생의 생성형 AI 활용 경험 탐색: 창작 단계별 활용 양상과 주체성·윤리 인식을 중심으로," 한국출판학연구 52(2):105–127, 2026.**
  - Method: ten aspiring web novelists used paid ChatGPT Plus, Claude Pro and Gemini Pro to write a project proposal and 1–3 episodes, kept creation diaries, and gave written and follow-up interviews.
  - Findings by stage:
    - Planning: AI was used actively to organize settings and expand world-building.
    - Drafting: AI drafts were mostly discarded or heavily rewritten because of world-building errors, genre and historical-period mismatch, insufficient length, and information from one episode leaking into others.
    - Revision: AI was used to check spelling, logic and story-world consistency.
  - Participants feared that disclosing AI use would bring rating attacks and erode reader trust, so they hid even planning and revision use. The authors call AI "a double-edged tool."
  - **Prompt implication:** draft generation must be fed the setting bible, genre and period constraints, a length target and an episode boundary (only what is known up to this episode). The revision and consistency-check modes are where writers already find value.
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=51fcdbc638866b4a47de9c1710b0298d)
- **[V] 조희정 & 박기수, "웹소설 창작에서 생성형 AI 위임의 경계: 현업 작가의 창작 단계별 활용과 비위임 영역을 중심으로," 한국출판학연구 52(3):101–123, 2026.**
  - Method: written interviews and thematic analysis with eight professional web novelists who use generative AI.
  - Most set boundaries in advance, either "research only" or "planning plus partial drafting." One writer delegated a first draft and then redrew the boundary.
  - Delegating first-draft production and decisions on pivotal scenes and plot drew strong reservation or was treated as off-limits, for reasons of authorship, personal growth and reader trust.
  - The authors argue that authorship depends on responsibility for direct language production and the core narrative, not on the amount of AI use.
  - **Prompt implication:** default to a co-pilot mode that proposes options, critiques and continues only on request. Never silently decide major plot turns. Make the writer's choices explicit.
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=a9e23216b3fc9f787ecd42904f0c5d65)

#### 4b. Platform, genre and authorship analyses
- **[V] 박성준 (Hansung Univ.), "AI를 활용한 웹소설 창작의 현재와 미래: AI 웹소설 창작 플랫폼을 중심으로 / The Present and Future of AI-Driven Web Fiction Creation," 문화콘텐츠연구 32:199–241, 2024, DOI 10.34227/tjocc.2024..32.199.**
  - Classifies platforms as consumer-centric (GPTRPG, Story Play) or creator-centric (iMyFone ChatArt, Isaac Novel).
  - Compares ChatArt and Isaac Novel outputs against 『전지적 독자 시점』 as a control.
  - Three problems: difficulty absorbing complex settings and world-building, a limited grasp of social codes, and difficulty actively using genre tags.
  - Recommends detailed world-building templates (설정집) and a standardized list of common Korean web-novel tags (~물) as AI input.
  - **Prompt implication:** give the prompt explicit slots for 설정집, 태그 and the social or relational codes of the genre (status, honorifics, romance-fantasy etiquette).
  - Source: [Journal page](https://www.journalricc.org/articles/article/z8kO/)
- **[V] 엄채민 & 서덕순 (Kyung Hee Univ.), "생성형 인공지능을 활용한 회귀 모티프 웹소설 창작 실험 - 창작 주체로서의 ChatGPT에 관한 고찰," 구보학보 40:583–617, 2025, DOI 10.35153/gubokr.2025..40.015.**
  - ChatGPT was first guided to learn the core features and narrative structure of the regression (회귀) motif, then asked to generate stories.
  - After sufficient learning, it fleshed out structures "at a reasonable level" and proposed story directions.
  - Limits: reliance on repetitive narrative structures, limited autonomy, and unclear copyright and originality.
  - **Prompt implication:** genre-trope primers help, but the model falls back on template plots. The prompt should require a twist or variation on the trope and track which beats have already been used.
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=5b91b3d6479e1ed24884a65323211ff0)
- **[L] Other analyses (all listing-level):**
  - 황은진, "생성형 AI 시대 플랫폼 콘텐츠 창작의 저작자성 판단에 관한 소고 -웹소설·웹툰 창작 사례를 중심으로-," 인문사회과학연구 69(2), 2026: authorship turns on substantive human contribution ([RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=e3cfce31cd6ccf02d18150b21a227875)).
  - 김준현, "AI시대의 도래와 웹문예창작/교육," 돈암어문학 44, 2023: AI can help with planning, review, correction, translation and scene description in web-fiction writing, but its usefulness must be tested genre by genre ([RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=d3e2d26c59e5818147de9c1710b0298d)).
  - 나윤빈, "텍스트마이닝과 네트워크 분석을 활용한 웹소설 연구 동향," 출판잡지연구 29(1), 2021: papers focus on genre and narrative, while funded research projects emphasize ICT topics such as AI ([RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=51246ef3c6468c634884a65323211ff0)).
  - 이화진, "중국 웹소설 플랫폼의 '진화'와 AI의 작용 - 치뎬(起点)과 판치에(番茄)를 중심으로," 중국현대문학 111, 2025: how Qidian and Fanqie used their parent companies' AI ecosystems, and the resulting debates (comparison case).
  - 김정희, "구비설화 기반 서사지도를 활용한 웹소설 창작 치료 도구 개발 및 실행 연구," 문학치료연구 80, 2026: a web-novel creative-therapy tool with a generative-AI interface, tested with a single participant, designed to protect the participant's agency.
  - The last two were found on [RISS search "웹소설 생성형 AI"](https://www.riss.kr/search/Search.do?isDetailSearch=N&searchGubun=true&viewYn=OP&query=%EC%9B%B9%EC%86%8C%EC%84%A4+%EC%83%9D%EC%84%B1%ED%98%95+AI&iStartCount=0&iGroupView=5&icate=all&colName=re_a_kor&order=%2FDESC&onHanja=false&strSort=RANK&pageScale=50&fsearchMethod=search&isFDetailSearch=N&sflag=1&searchQuery=%EC%9B%B9%EC%86%8C%EC%84%A4+%EC%83%9D%EC%84%B1%ED%98%95+AI&pageNumber=1).

#### 4c. Industry context (news, not research)
- **[News] Economist Korea, "AI로 웹소설 한편 뚝딱…'5000자 구원투수' Vs '대필작가'," 2026-08-01, 원태영.**
  - Several platforms ban AI in contest entries, but there is no legal disclosure duty and no reliable detection system. An industry source said 100%-accurate detection "사실상 없다" (effectively does not exist).
  - The 2026 "지상최대 웹소설 공모전" (Naver Webtoon and Munpia) drew more than 10,000 entries (+35%). Of 7,400 entrants, 6,600 (89.2%) had no paid-serial experience.
  - Serial norms: at least 5,000 characters per episode and five or more episodes per week.
  - Readers object to AI-like translationese and flat prose.
  - Prof. 장민지 (Kyungnam Univ.) says AI eases research but is hard to rely on for story and dialogue.
  - Source: [Economist Korea](https://economist.co.kr/article/view/ecn202607280044)

#### 4d. Korean HCI co-writing research
- **[V] Chung, John Joon Young; Kim, Wooseok; Yoo, Kang Min; Lee, Hwaran; Adar, Eytan; Chang, Minsuk, "TaleBrush: Sketching Stories with Generative Pretrained Language Models," CHI 2022, DOI 10.1145/3491102.3501819.** Author affiliations were not shown in the record I opened; the paper is indexed in KAIST's institutional repository, and co-authors Yoo and Lee are NAVER researchers (my background knowledge, not verified here).
  - Writers sketch a line for the protagonist's fortune across the story, and a GPT-based language model generates text that follows the curve.
  - The pipeline reliably controlled generation while keeping sentences novel.
  - In a 14-participant study, writers used sketching to explore and write stories matching their intentions and drew inspiration from generated text.
  - **Prompt implication:** expose high-level narrative controls (tension or fortune arc per scene) as explicit prompt parameters, instead of asking the writer to express everything in prose instructions.
  - Sources: [Semantic Scholar API record](https://api.semanticscholar.org/graph/v1/paper/DOI:10.1145/3491102.3501819?fields=title,authors,venue,year,abstract,externalIds,citationCount); [DOI](https://doi.org/10.1145/3491102.3501819)
- **Cross-references:**
  - HyperCLOVA Studio (Kim et al. 2021) was NAVER's prompt-engineering interface for non-experts ([arXiv](https://arxiv.org/abs/2109.04650)).
  - The Dongseo Univ. GPT-3 scenario assistant used an N+1 candidate-choice loop (Jo et al. 2022, [PDF](https://www.koreascience.or.kr/article/CFKO202232249452420.pdf)).

### Inferences
- Writers accept AI for "before" and "after" the prose (research, planning, setting management, consistency and spelling checks) and resist it for the prose itself and for key decisions (조희정 & 박기수 2026a, 2026b). WriterProgram's prose-generation mode therefore has to clear a high bar: it must sound like the writer, not like AI, because readers punish AI-like prose (Economist 2026) and writers fear disclosure (2026a). This is a strong argument for writer-voice conditioning (style samples from the writer's own manuscript) plus the "AI-smell" linter.
- The documented draft failures (world-building errors, genre and period mismatch, short length, cross-episode leakage) are information-supply problems, so they are largely fixable by prompt structure: setting bible, tags, period rules, episode-scoped facts and length targets (5,000+ characters for serials).

### Gaps
- I found no large-scale survey of Korean web novelists' AI use; the two studies have n=10 and n=8. KOCCA's 2025 webtoon survey appeared in news snippets but was not opened and concerns webtoons.
- No verified HCI paper (CHI, CSCW, UIST or 한국HCI학회) studies Korean web novelists or Korean-language co-writing with HyperCLOVA or HyperCLOVA X. TaleBrush concerns English-language story writing, not Korean.
- No academic source documents specific AI features that Korean web-novel platforms (Naver Series, KakaoPage, Munpia, Novelpia, Ridi) have adopted. The news only covers contest bans.

## 5. What do studies of LLM translation of Korean literature reveal about which Korean nuances LLMs lose?

### Takeaway
The Korean translation-studies literature (2023–2026) agrees on several points. ChatGPT-class models clearly beat Papago and NMT on literary translation, with about half the errors and some human-like creativity. By default they stay close to the source and are literal. Richer role/persona prompts unlock creative strategies but also add unrequested material. They retain source images and metaphors rather than adapting them culturally, and human cultural substitution was rated more acceptable. Proverbs, titles and honorifics are weak spots. I found no study of how LLMs handle ideophones (의성어·의태어) or sentence endings in Han Kang's work. The only Han Kang-related study is a persona experiment modeled on Deborah Smith's translation of *The Vegetarian*.

### Cited Findings
- **[V] 이창수 (Hankuk Univ. of Foreign Studies), "챗GPT, 파파고, 인간 번역가 간의 한영 문학번역 차이점 연구 / Differences in Korean-English literary translation among ChatGPT, Papago, and human translators," 번역학연구 25(2):11–37, 2024, DOI 10.15749/jts.2024.25.2.001.**
  - Method: PCA on the 80 most frequent words, plus human-annotated creativity and quality analysis of English translations of a Korean short story by ChatGPT, Papago and two human translators.
  - Findings:
    - Papago separated clearly from the human translations in style; ChatGPT did not.
    - Papago showed almost no creativity and many errors.
    - ChatGPT made about half as many errors as Papago and showed some human-like creativity, which the author attributes to context-aware understanding.
  - **Implication:** frontier LLMs handle Korean literary context well enough to be used for literary Korean, but errors remain substantial.
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=4b8021cf7b4ee37f6aae8a972f9116fb)
- **[V] 김현웅 & 이상빈 (Hankuk Univ. of Foreign Studies), "AI 문학번역에서 프롬프트 엔지니어링이 번역 오류와 창의성에 미치는 영향 / Effects of prompt engineering on error reduction and creativity in AI literary translation," 번역학연구 26(3):147–171, 2025, DOI 10.15749/jts.2025.26.3.005.**
  - Method: English translations of 현진건's 1921 story 「술 권하는 사회」 were produced with a Role-Goal-Context (RGC) prompt and with a baseline prompt ("Please translate ... preserving its structure and details as much as possible").
  - Findings:
    - There was no significant difference in total error frequency.
    - The RGC prompt produced more over-translation, while omissions and mistranslations were stable across conditions.
    - The RGC prompt used more creative strategies (paraphrasing, elaboration) and improved context sensitivity and readability.
  - **Prompt implication:** rich role and goal framing buys stylistic freedom at the cost of embellishment. A fiction prompt that uses a literary persona must also say "do not add events or details not in the scene plan."
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=c605c96b7e08853d7f7a54760bb41745)
- **[L] 마승혜, "AI 페르소나가 문학 번역에 미치는 영향 - 챗GPT 프롬프트에 따른 번역 사례 비교·분석," 통번역학연구 29(4), 2025.**
  - ChatGPT was trained on the habitus and strategies of a translator persona modeled on Deborah Smith's translation of *The Vegetarian*.
  - Abstract quotes: "Before the habitus and strategy training, ChatGPT adhered closely to the source text," and after training the results were "found not to differ significantly from Smith's translation."
  - **Prompt implication:** an explicit persona with named strategies shifts the model's default literalness. The same approach (a persona built from a writer's stated habits) should transfer to style in Korean generation.
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=be3f028155108112d18150b21a227875)
- **[L] 최승혁 & 박응석, "생성형 AI 문학 번역에서 은유 재현의 한계와 가능성," 한국언어문화학 23(2), 2026.**
  - Compared Korean→Chinese translation strategies for 243 metaphors from a 김호연 novel, by human translators, ChatGPT and DeepSeek.
  - Humans tended to substitute metaphors to fit Chinese readers' culture; the AIs tended to retain the source image.
  - Native-speaker acceptability: human substitution 4.32, AI retention 3.83–3.87, which is still at a practical level.
  - **Implication for generation:** LLMs carry images over rather than re-grounding them culturally. When Claude writes Korean prose, the analogous risk is imported, English-shaped imagery, so the prompt should ask for imagery grounded in Korean lived experience and checked for calques.
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=51982e11fdd79393e9810257f7042666)
- **[L] 임형재 & 염선화, "생성형 AI 페르소나를 활용한 시 번역의 문체 재현과 창조적 변용 - 아이칭(艾靑) 시 「北方」 번역 실험을 중심으로," 우리말 글 107, 2025.**
  - An "AI Poet" persona built from the works of the Korean poet 이용악 was used to translate Ai Qing's poem into Korean.
  - Abstract: "the AI-generated translation faithfully reproduced Yi's specific poetic diction, stylistic texture, and imagery."
  - **Prompt implication:** a persona conditioned on a specific Korean author's corpus can reproduce Korean poetic diction. This is direct support for style-exemplar conditioning on the writer's own prose.
  - Source: [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=65fb80a3caec5fbe6aae8a972f9116fb)
- **[L] 마승혜, "AI 문학번역, 어디까지 가능한가—챗GPT가 번역한 한국 문학작품 제목의 기능 분석을 중심으로," 번역학연구 25(3), 2024.** ChatGPT's English titles for four Korean novels covered fewer title functions than professional translators' titles. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=d22ee0ac243b177ed18150b21a227875)
- **[L] 방영풍 & 서연주, "ChatGPT를 활용한 문화소 번역연구- 소설 『대장금』의 한중 번역을 중심으로 -," 國際言語文學 59, 2024.** ChatGPT handled cultural terms well compared with Papago, Baidu and human translations. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=d80a2852192163516aae8a972f9116fb)
- **[L] 윤여범, "ChatGPT의 한영 번역 도구로서의 정확성 분석," 한국초등교육 34(4), 2023.** Native-speaker ratings of factual, literary and proverb texts found proverbs least accurate. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=29a94e6b976ee8557f7a54760bb41745)
- **[L] 이선화, "챗GPT를 적용한 번역수업 실천 사례 연구," 번역학연구 24(3), 2023.** In Korean→Japanese translation of 『82년생 김지영』, prompts specifying role, audience and strategy improved quality. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=8428b02d0b78e78ac85d2949c297615a)
- **[L] 이형진, "생성형 인공지능(AI)의 한국문학 영어번역의 딜레마와 가능성," 외국문학연구 100, 2025.** Argues that human literary translators keep a distinct role. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=1d92ffab66ff50d74884a65323211ff0)
- **[L] 김미영 et al., "생성형 AI 기반 한국 문학작품의 한어 번역 질 비교 연구 —DeepSeek와 ChatGPT를 중심으로," 중국조선어문 264, 2026.** By ROUGE and BLEU on novel, prose, poetry and drama excerpts, ChatGPT was more stable and DeepSeek had some higher peaks. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=6c9c1d698540a8617ecd42904f0c5d65)
- **[L] YUCUILING & 박지숙, "챗GPT 등장 이후의 인공지능(AI)과 문학 번역 ― 채만식 『민족의 죄인』 속의 대화문을 중심으로," 중국학보 110, 2024.** Compares Papago, Google Translate and ChatGPT Chinese translations of dialogue against human translation. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=a1deb1702f95ca6a47de9c1710b0298d)
- **Cross-reference [V]:** in English→Korean literary translation evaluation, Korean honorifics were where automated evaluation fell furthest below human agreement, and LLM judges preferred LLM translations (Shafayat et al., [arXiv](https://arxiv.org/abs/2412.01340)).

### Inferences
- The translation studies suggest four default LLM tendencies that also matter when an LLM writes Korean directly:
  - literalness and source-shaped phrasing (마승혜 2025; Economist 2026 on 번역투);
  - retaining rather than localizing imagery (최승혁 & 박응석 2026);
  - weakness on proverbs and idioms (윤여범 2023; KULTURE Bench);
  - honorific nuance that both models and evaluators miss (Shafayat et al.).
- For a Korean fiction prompt, this argues for explicit honorific and speech-level maps between characters (존댓말/반말, 하십시오체/해요체/해체), a ban on calqued English idioms, and a request for culturally grounded Korean imagery.
- Persona and strategy prompts reliably move LLMs away from literal defaults (마승혜 2025; 임형재 & 염선화 2025; 이선화 2023), but they also increase embellishment (김현웅 & 이상빈 2025). Pair a persona with plan-adherence constraints.

### Gaps
- I found no study of LLM handling of Korean ideophones (의성어·의태어), sentence-final endings, or Han Kang's minimalist style as such. The Han Kang link exists only through the persona study modeled on Smith's *Vegetarian* translation (abstract-level).
- Most studies here are listing-level, so their error typologies and examples were not extracted.
- Studies of Korean web-novel translation by LLMs (Korean → English, Japanese, Chinese) did not surface in these academic searches.

## 6. Synthesis: lessons that should shape a Korean fiction prompt, and master table of papers

### Takeaway
Read together, the Korean literature points to a specific design for WriterProgram:
- plan in layers and generate scene by scene, supplying a setting bible, genre tags, period rules and episode-scoped facts;
- condition on the writer's own voice through a persona and exemplars, while pinning plot adherence;
- write imagery and honorific relations from Korean cultural ground rather than translated habits;
- treat "too-correct, comma-heavy, uniform, line-broken-prose" Korean as the signature to lint against;
- keep the writer in charge of first drafts and pivotal decisions;
- never use an LLM judge as the arbiter of literary quality.

### Cited Findings

#### 6a. Lessons for the WriterProgram prompt, skill and linter (each grounded in the sources above)
1. **Plan in layers before prose, and regenerate per scene.**
   - Logline → acts → beats → scene spec: Save-the-Cat chaining (Ji & Yoo 2025), recursive plotting (Cha et al. 2024), structure plus scene specs (김수진 et al. 2026).
   - Long outputs drift from their summary (Choe et al. 2022) and repeat or contradict themselves after a few hundred words (Jo et al. 2022).
   - Prompt clause: before writing, produce or confirm a 장면 설계서 (scene goal, conflict, turn, POV, who knows what, end hook), then write only that scene.
   - Sources: [Ji & Yoo](https://db.koreascholar.com/Article/Detail/443668); [Cha et al.](https://www.koreascience.kr/article/CFKO202404272002240.page); [김수진 et al.](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=380784b010c6543a6aae8a972f9116fb); [Choe et al.](https://koreascience.kr/article/CFKO202226455339339.pdf); [Jo et al.](https://www.koreascience.or.kr/article/CFKO202232249452420.pdf)
2. **Constrain structure and style together.** Structure-only or style-only conditions scored about 5.2, and both together about 5.65 (김수진 et al. 2026). The system prompt needs an explicit 문체 규율 block (person/POV, tense, sentence-ending palette, dialogue conventions, banned patterns), and the linter enforces it after generation. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=380784b010c6543a6aae8a972f9116fb)
3. **Supply the information whose absence caused documented failures.**
   - Writers' AI drafts failed on world-building errors, genre and period mismatch, insufficient length and cross-episode leakage (조희정 & 박기수 2026a). Platforms fail on settings, social codes and tags (박성준 2024).
   - Prompt slots: 설정집 (world bible), 태그 (~물 genre tags), 시대 고증 rules, a relationship and social-code map, "facts known as of episode N," and a length target (serial norm 5,000+ characters per episode, per Economist 2026).
   - Sources: [조희정 & 박기수 2026a](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=51fcdbc638866b4a47de9c1710b0298d); [박성준 2024](https://www.journalricc.org/articles/article/z8kO/); [Economist Korea](https://economist.co.kr/article/view/ecn202607280044)
4. **Keep the writer in charge.**
   - Professional writers treat first drafts and pivotal scene or plot decisions as non-delegable (조희정 & 박기수 2026b).
   - Scholars frame AI as a dialogic co-creator (이경수 2024).
   - The best AI books came from human selection among many candidates (SIA: 53 poems curated; 비람풍: director storyboard plus re-prompting).
   - Design: offer 2–3 alternatives at decision points (Jo et al. 2022 N+1; TaleBrush arc control), and never decide major turns unasked.
   - Sources: [조희정 & 박기수 2026b](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=a9e23216b3fc9f787ecd42904f0c5d65); [이경수 2024](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=9b2931e7dd2188b44884a65323211ff0); [iNews24](https://www.inews24.com/view/1505892); [ETNews](https://www.etnews.com/20230713000255); [TaleBrush](https://doi.org/10.1145/3491102.3501819)
5. **Condition on voice through a persona and exemplars, but pin plot adherence.**
   - An author persona improved long-story consistency (Cha et al. 2024).
   - A translator persona moved ChatGPT off literal defaults (마승혜 2025).
   - A poet persona built from 이용악's corpus reproduced his diction and imagery (임형재 & 염선화 2025).
   - Retrieving emotion-tagged canon lines steered affect (KPoEM).
   - Role/goal prompts increased embellishment (김현웅 & 이상빈 2025).
   - Prompt: "Match the writer's sample passages for diction, rhythm and endings. Do not add events, characters or facts not in the scene spec."
   - Sources: [마승혜 2025](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=be3f028155108112d18150b21a227875); [임형재 & 염선화 2025](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=65fb80a3caec5fbe6aae8a972f9116fb); [KPoEM](https://arxiv.org/abs/2509.03932); [김현웅 & 이상빈 2025](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=c605c96b7e08853d7f7a54760bb41745)
6. **Lint against the measured signature of LLM Korean.**
   - Commas in 61% vs 26% of essay sentences, overused after connective endings and conjunctive adverbs; strict spacing; lower POS-sequence diversity; over-length (KatFish).
   - In verse: "line-broken prose," uniform line lengths, low variety of line-final forms and over-connective endings, strict orthography (Park/Yu/Kim 2026).
   - Ceiling-level orthography and register (Park & Han 2026); readers spot 번역투 and flat prose (Economist 2026).
   - Linter metrics: commas per sentence, share of sentences ending in the same ending, share of connective-ending lines in verse, variance of sentence or line length, conjunctive-adverb sentence openers, length vs target.
   - Sources: [KatFish](https://arxiv.org/html/2503.00032v4); [Park/Yu/Kim](https://arxiv.org/abs/2608.28986); [Park & Han](https://arxiv.org/abs/2601.19913)
7. **Track speakers and emotions explicitly.** The best model reached only 52.22% speaker identification on Korean novel text, and an emotion-vocabulary list in the prompt helped (Kwon & Kang 2025). The prompt should carry a character sheet (speech level, verbal tics, current emotional state), and dialogue should be attributed where ambiguity is possible. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=78314f60a5e4a3367ecd42904f0c5d65)
8. **Ban the anti-patterns critics found in Korean AI fiction.**
   - 『지금부터의 세계』: underdeveloped characters, implausible plot, raw math and science information, unrefined slang. SIA: declarative, verbose, binary statements of scientific fact (오태호 2023).
   - Repetitive template plots in a genre-trope experiment (엄채민 & 서덕순 2025); cliché awareness emphasized in creative-writing pedagogy (김민정 2026).
   - Prompt: dramatize information through action and dialogue; vary the trope with at least one subversion; no summary-moralizing endings; no "A인가, B인가" reflections.
   - Sources: [오태호 2023](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=01faa8a5a6cb3a3a7f7a54760bb41745); [엄채민 & 서덕순 2025](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=5b91b3d6479e1ed24884a65323211ff0); [김민정 2026](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=04f4db6671c83a184884a65323211ff0)
9. **Ground figurative language and honorifics in Korean culture.**
   - LLMs retain source images rather than localizing them, and human substitution was rated more acceptable (4.32 vs about 3.85) (최승혁 & 박응석 2026).
   - Proverbs were the least accurate text type (윤여범 2023); models still have room to improve on Korean idioms and poetry (KULTURE).
   - Honorifics were the weakest point of LLM evaluation (Shafayat et al.).
   - Prompt: an honorific and speech-level matrix per character pair; imagery from Korean everyday life, seasons, food and spaces; no calqued English idioms; check any proverb.
   - Sources: [최승혁 & 박응석](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=51982e11fdd79393e9810257f7042666); [윤여범](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=29a94e6b976ee8557f7a54760bb41745); [KULTURE](https://arxiv.org/abs/2412.07251); [Shafayat et al.](https://arxiv.org/abs/2412.01340)
10. **Calibrate sensory density to the writer.** GPT-4.1's sensory and emotional richness produced the widest disagreement among human raters (Lee et al. 2026). Expose a "묘사 밀도" (description density) setting instead of maximizing vividness by default. — [RISS](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=33a941b128e7fec9c85d2949c297615a)
11. **Restrict LLM-as-judge to checkable properties.**
    - Korean judges transfer English skill but miss cultural and factual errors (KUDGE).
    - GPT-4 scores correlate with humans but do not reproduce human rankings (K2-Eval).
    - Judges are reliable only on grammar and fluency (Kim & Kim 2024), consistent but culturally blind (Kim & Oh 2025), and prefer LLM output (Shafayat et al.). LogicKor's top end is saturated.
    - Use automated checks for consistency, spelling, POV and linter counts, and leave literary quality to the writer.
    - Sources: [KUDGE](https://arxiv.org/abs/2409.11239); [K2-Eval](https://huggingface.co/datasets/HAERAE-HUB/K2-Eval/raw/03c9125d0ec9c1dce627adc81f2e3f928917a844/README.md); [Kim & Kim](https://arxiv.org/abs/2407.17022); [Kim & Oh](https://research.knu.ac.kr/en/publications/evaluating-creativity-can-llms-be-good-evaluators-in-creative-wri/); [LogicKor](https://github.com/instructkr/LogicKor)
12. **Build an in-house evaluation set.** No Korean fiction benchmark exists. KoMT-Bench writing items are short and translated, and K2-Eval's creative-writing items are few but human-judged. WriterProgram should create writer-rated Korean scene-continuation tests (web-novel and literary registers) scored on genre fit, voice match, figurative freshness and the lesson-6 metrics. — [KoMT-Bench](https://huggingface.co/datasets/LGAI-EXAONE/KoMT-Bench); [K2-Eval](https://huggingface.co/datasets/HAERAE-HUB/K2-Eval)

#### 6b. Master table of all sources

Ver. = verification level ([V] record opened and read; [L] listing-level abstract snippet; [News] press).

| # | Title | Authors | Year | Venue | Lang | URL | One-line finding | Ver. |
|---|---|---|---|---|---|---|---|---|
| 1 | 요약문 기반 문학 스타일 문장 생성 | 최부광, 이은찬, 안상태 | 2022 | HCLT 2022 | KO | [link](https://koreascience.kr/article/CFKO202226455339339.pdf) | KoBART on AI-Hub literary summaries; relevance to the summary drops as output lengthens (ROUGE-1 0.391) | V |
| 2 | GPT-3를 활용한 시나리오 생성 보조 시스템 | 조동하, 전이슬, 문미경 | 2022 | KSCI Summer Conf. 30(2) | KO | [link](https://www.koreascience.or.kr/article/CFKO202232249452420.pdf) | N candidate sentences + writer input; LMs repeat or contradict beyond a few hundred words | V |
| 3 | 이미지 캡션 및 재귀호출을 통한 스토리 생성 방법 | 전이슬, 조동하, 문미경 | 2023 | J. Inst. Convergence Signal Proc. 24(1) | KO | [link](https://www.koreascience.kr/article/JAKO202331972864504.page) | Storyboard captions → KoGPT-2 scenarios following the planner's intent | L |
| 4 | 구조화된 플롯팅과 작가 페르소나를 활용한 언어모델의 장편 이야기 생성 | 차수연, 노영빈, 이승진 | 2024 | HCLT 2024 | KO | [link](https://www.koreascience.kr/article/CFKO202404272002240.page) | Recursive plot prompts + author personas improved length, diversity and coherence (human eval) | V |
| 5 | 서사 구조를 따르는 LLM 기반 스토리 생성 연구 | 지은수, 유태경 | 2025 | J. Korean Soc. Computer Game 38(3) | KO | [link](https://db.koreascholar.com/Article/Detail/443668) | 3-act → Save-the-Cat 15-beat prompt chaining beat single prompts on structure and logic | V |
| 6 | AI기반 장편소설 창작 파이프라인의 신뢰성에 대한 탐색적 연구 | 김수진, 양현호, 김정욱 | 2026 | J. Internet Computing & Services 27(3) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=380784b010c6543a6aae8a972f9116fb) | Structure + style constraints together ≈5.65 vs ≈5.2 for either alone (FT-24, 3 LLM judges) | V |
| 7 | 생애사 프로필 기반 LLM 소설 생성의 창의성 평가 연구 | 이만유, 김예진, 전승화, 오동석 | 2026 | J. Digital Contents Soc. 27(1) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=33a941b128e7fec9c85d2949c297615a) | 300 stories (GPT-3.5/4o/4.1); GPT-4.1 most sensory but most rater disagreement | V |
| 8 | 대형 언어 모델에서의 소설 데이터 기반 2차 창작물 생성 효율성 비교 연구 | 권민재, 강윤구 | 2025 | KCC 2025 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=78314f60a5e4a3367ecd42904f0c5d65) | EEVE best; speaker ID only 52.22%; emotion-lexicon prompt helps | V |
| 9 | 언어 모델을 이용한 장르 단편 소설 생성 및 분석 | 박정윤, 김가연, 배병철 | 2023 | KSC 2023 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=d25a83d1f2c6292ac85d2949c297615a) | Genre short-story generation with LMs (no abstract; unverified) | L |
| 10 | AI를 활용한 인터렉티브 소설 생성 시스템 설계 및 구현 | 정용태, 김선혁, 김세훈, 정기현 | 2025 | KIIT Conf. 2025(11) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=8970538954df7e24c85d2949c297615a) | LLM + diffusion branching interactive novel | L |
| 11 | 도메인 정보가 강화된 워드 임베딩을 사용한 한국어 텍스트 생성 | 임미영, 강신재 | 2019 | J. Korean Inst. Intelligent Systems 29(2) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=7b040ace4ecb54d8e9810257f7042666) | General-corpus pretraining gave higher rhetorical creativity (RNN) | L |
| 12 | 인공지능 기반 한국어 시 생성 시스템 개발 연구 | 김명선, 정우혁, 우지환 | 2023 | Information Systems Review 25(3) | KO | [link](https://www.koreascience.or.kr/article/JAKO202307267862839.page?&lang=ko) | Poet interviews → 8 genre datasets; seed lines to assist poets | V |
| 13 | SeqGAN 모델을 이용한 한국어 시 자동 생성 | 박요한 et al. | 2018 | HCLT 2018 | KO | [link](https://www.koreascience.kr/article/CFKO201832073078975.page) | Generated poems repeated phrases (pre-LLM baseline) | L |
| 14 | Detecting and Guiding LLM-Generated Korean Poetry with Interpretable Form-level Features | Park K., Yu S., Kim Y. | 2026 | Findings of EMNLP 2026 (arXiv) | EN | [link](https://arxiv.org/abs/2608.28986) | LLM Korean poems = "line-broken prose"; 5 form features (AUC 83.60); guidance preferred by experts | V |
| 15 | KPoEM: emotion dataset and RAG poetry generation for Korean modern poetry | Lim I., Ji H., Kim B. | 2025/26 | arXiv; Review of Korean Studies 29(1) | EN | [link](https://arxiv.org/abs/2509.03932) | 7,662 entries, 44 emotions, 5 poets; F1 0.60; RAG generation | V |
| 16 | KatFishNet: Detecting LLM-Generated Korean Text through Linguistic Feature Analysis | Park S., Kim S., Kim D.-K., Han Y.-S. | 2025 | ACL 2025 | EN | [link](https://aclanthology.org/2025.acl-long.1030) | LLM Korean: strict spacing, low POS diversity, commas 61% vs 26% of sentences, longer | V |
| 17 | 생성형 AI 기반의 콘텐츠 제작 기술 동향과 안전성 이슈 | Yu J.J. et al. (ETRI) | 2025 | ETRI Electronics & Telecom Trends 40(2) | KO | [link](https://www.koreascience.kr/article/JAKO202551143204165.page) | Long-form content automation still limited | L |
| 18 | What Changes Can Large-scale Language Models Bring? (HyperCLOVA) | Kim B. et al. (NAVER) | 2021 | EMNLP 2021 | EN | [link](https://arxiv.org/abs/2109.04650) | 82B Korean GPT-3 (560B tokens); HyperCLOVA Studio prompt interface; no creative eval | V |
| 19 | TaleBrush: Sketching Stories with Generative Pretrained Language Models | Chung J.J.Y., Kim W., Yoo K.M., Lee H., Adar E., Chang M. | 2022 | CHI 2022 | EN | [link](https://doi.org/10.1145/3491102.3501819) | Sketching a protagonist's fortune arc controls GPT story generation; 14-user study | V |
| 20 | KoMT-Bench (dataset card) | LG AI Research | 2024 | Hugging Face | KO/EN | [link](https://huggingface.co/datasets/LGAI-EXAONE/KoMT-Bench) | Translated, Korean-adapted MT-Bench; 80 Qs incl. writing and roleplay | V |
| 21 | LogicKor (repository) | instructkr | 2024 | GitHub | KO | [link](https://github.com/instructkr/LogicKor) | LLM-judged Korean benchmark; archived; top end saturated | V |
| 22 | K2-Eval (dataset card) | Son G., Ko H.W., Lee H., Hong S., Kim Y., Kim J. | 2024 | Hugging Face (HAE-RAE) | KO/EN | [link](https://huggingface.co/datasets/HAERAE-HUB/K2-Eval) | 90 native Korean items incl. Creative Writing; 15 human judges; GPT-4 fails to replicate human rankings | V |
| 23 | LLM-as-a-Judge & Reward Model: What They Can and Cannot Do (KUDGE) | Son G., Ko H., Lee H., Kim Y., Hong S. | 2024 | arXiv | EN | [link](https://arxiv.org/abs/2409.11239) | Korean judging tracks English skill; misses factual and cultural errors | V |
| 24 | Can Language Models Evaluate Human Written Text? Korean Student Writing | Kim Seungyoon, Kim Seungone | 2024 | arXiv | EN | [link](https://arxiv.org/abs/2407.17022) | GPT-4-Turbo reliable on grammaticality and fluency, not other criteria | V |
| 25 | When LLM Essays Outscore Student Essays: What a Korean Writing Rubric Rewards and Where Readers Disagree | Park Shinwoo, Han Yo-Sub | 2026 | arXiv | EN | [link](https://arxiv.org/abs/2601.19913) | LLM +18.35 pts, 44.7% from orthography and register; students best on creativity and natural phrasing | V |
| 26 | Evaluating Creativity: Can LLMs Be Good Evaluators in Creative Writing Tasks? | Kim Sungeun, Oh Dongsuk | 2025 | Applied Sciences 15(6):2971 | EN | [link](https://research.knu.ac.kr/en/publications/evaluating-creativity-can-llms-be-good-evaluators-in-creative-wri/) | LLM judges consistent (high IAA) but weak on culturally specific creativity | V |
| 27 | KULTURE Bench: Korean cultural context benchmark | Wang X., Yeo J., Lim J.-H., Kim H. | 2024 | PACLIC 38 | EN | [link](https://arxiv.org/abs/2412.07251) | News, idioms and poetry comprehension; much room to improve | V |
| 28 | A 2-step Framework for Automated Literary Translation Evaluation | Shafayat S., Yoon D., Jang W., Choi J., Oh A., Jung S. | 2024/25 | arXiv | EN | [link](https://arxiv.org/abs/2412.01340) | En→Ko literary MT eval; honorifics hardest; LLMs prefer LLM translations | V |
| 29 | 생성형 AI 시대, 인간 창작의 고유성에 관한 실증 연구 | 정보영, 유현성, 김호애 | 2026 | 글로벌 거버넌스와 문화 6(1) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=916b4751430f12d87ecd42904f0c5d65) | Three writers identified anonymous AI poems with 100% success | V |
| 30 | 2020년대 초반 인공지능(AI) 문학 창작의 현재성 고찰 (『지금부터의 세계』, 『시를 쓰는 이유』) | 오태호 | 2023 | 우리문학연구 79 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=01faa8a5a6cb3a3a7f7a54760bb41745) | Novel: thin characters, implausible plot, raw information, slang; 「오래된 집」 at established-poet level | V |
| 31 | AI 창작 기술에 따른 인공지능 소설의 유형 분석 | 서경숙 | 2024 | 현대영어영문학 68(2) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=9595bcbd5074916f6aae8a972f9116fb) | Technology and degree of human involvement produce distinct literary features | V |
| 32 | 공동 창작 주체로서의 생성형 인공지능의 가능성과 과제 | 이경수 | 2024 | 인공지능인문학연구 18 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=9b2931e7dd2188b44884a65323211ff0) | AI as dialogic co-creator (『시를 쓰는 이유』, 『9+i』, KAIST Q Group) | V |
| 33 | 인공지능 글쓰기와 소설의 미래 (박금산 『AI가 쓴 소설』) | 정재훈 | 2023 | 비평문학 90 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=2ff19cf0b3fdb7a34884a65323211ff0) | AI writing democratizes authorship but risks platform labor | V |
| 34 | 디지털 전환 시대 현대한국문학 연구의 동시대성 | 박인성 | 2025 | 현대소설연구 100 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=2e85e4230fedf95d7ecd42904f0c5d65) | Meta-review: web novels and AI reshaping Korean literary studies | V |
| 35 | 인공지능(AI)은 시를 쓸 수 있는가 | 고봉준 | 2023 | 비평문학 89 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=e00a88ff30be158c7f7a54760bb41745) | 권보연–김언 debate on AI poetry and authorship | L |
| 36 | 인공지능 시대 한국 현대시의 생성시학 | 정끝별 | 2023 | 이화어문논집 60 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=aae7139d95d0772c6aae8a972f9116fb) | AI poetry within Korean modern poetics | L |
| 37 | 인공지능 창작 주체와 대화적 시쓰기 | 유가은 | 2024 | 어문논집 98 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=4501fa82267fa0bb47de9c1710b0298d) | Dialogic poetry writing with AI | L |
| 38 | 포스트휴먼의 글쓰기, 창작인가 조합인가? | 김소륜 | 2023 | 돈암어문학 44 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=d3e2d26c59e581817f7a54760bb41745) | Beyond the "AI author vs tool" binary | L |
| 39 | 인공지능 문학의 가능성과 한계에 대한 연구 | 신기선 | 2024 | 스토리콘텐츠 5 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=8394854b1fde3f846aae8a972f9116fb) | GPT-3 literary creativity and limits | L |
| 40 | 창작 보조기에서부터 문장 생성기까지: 글쓰기 기계의 과거와 현재 | 이재연, 한남기 | 2021 | 한국문학연구 67 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=d44afe65479690c26aae8a972f9116fb) | History from StoryHelper to neural generators | L |
| 41 | 디지털 시대 글쓰기의 도구적 전환 (켈만) | 김요한, 김겸섭 | 2023 | 독일어문학 100 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=73582f6debae9088b7998d826d417196) | Statistical writing lacks meaning and consciousness | L |
| 42 | AI를 통한 글쓰기와 작가의 운명 | 이정엽 | 2017 | 한국현대소설학회 (journal inferred) | KO | [link](https://scholarworks.bwise.kr/sch/handle/2021.sw.sch/8006) | Japanese AI story imitates existing sentences (background) | L |
| 43 | 생성형 인공지능을 활용한 소설 창작 교육의 가능성 | 김지혜 | 2023 | 어문연구 51(4) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=e61233ba27d98543b7998d826d417196) | Gen AI for ideation and style practice in novel-writing classes | L |
| 44 | 인공지능 시대의 시 창작 교육 방향 탐색 | 이운영 | 2024 | 청람어문교육 98 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=a2602bb4563ea09747de9c1710b0298d) | SIA review; collaborative, process-oriented poetry education | L |
| 45 | 기술창작시대의 문학 작품과 현대시 교육 | 이종원 | 2025 | 문학교육학 87 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=fec37d6d78de6141b7998d826d417196) | AI as tool, co-creator or autonomous creator; iterative revision | L |
| 46 | 예비 국어교사 대상 인공지능 활용 시 창작 수업 사례 연구 | 강정화 | 2024 | 인문사회과학연구 67(4) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=3c4f03debca69928d18150b21a227875) | Prompt design kept students as creators | L |
| 47 | 인공지능(ChatGPT)을 활용한 중학생 시 쓰기 교육 실제 | 조은영, 김지윤 | 2023 | 학습자중심교과교육연구 23(21) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=e6f34b78f6aa6edd47de9c1710b0298d) | 148 students; prompt writing was the bottleneck | L |
| 48 | 문예창작 전공생의 AI 협업 창작 경험 (<AI 백일장>) | 양나리 | 2026 | 예술교육연구 24(1) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=7a9c1e94273a954a6aae8a972f9116fb) | Majors rated AI writing low, felt constrained, refused co-authorship | L |
| 49 | 스토리IP 시대의 AI활용 창작 교육의 실천 모델 연구 | 김민정 | 2026 | 예술교육연구 24(3) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=04f4db6671c83a184884a65323211ff0) | 48 students; AI as helper; cliché meta-awareness | L |
| 50 | 콘텐츠 창작과 AI 리터러시 — LLM의 패러프레이징 | 윤나라 | 2024 | 인문콘텐츠 72 | KO | [link](https://www.riss.kr/search/Search.do?isDetailSearch=N&searchGubun=true&viewYn=OP&query=%EC%83%9D%EC%84%B1%ED%98%95+AI+%EC%86%8C%EC%84%A4+%EC%B0%BD%EC%9E%91&iStartCount=0&iGroupView=5&icate=all&colName=re_a_kor&order=%2FDESC&onHanja=false&strSort=RANK&pageScale=50&fsearchMethod=search&isFDetailSearch=N&sflag=1&searchQuery=%EC%83%9D%EC%84%B1%ED%98%95+AI+%EC%86%8C%EC%84%A4+%EC%B0%BD%EC%9E%91&pageNumber=1) | Writers as AI supervisors and orchestrators (Marche novella) | L |
| 51 | AI를 활용한 웹소설 창작의 현재와 미래: AI 웹소설 창작 플랫폼을 중심으로 | 박성준 | 2024 | 문화콘텐츠연구 32 | KO | [link](https://www.journalricc.org/articles/article/z8kO/) | AI platforms fail at world-building, social codes and ~물 tags; recommends 설정집 templates and tag lists | V |
| 52 | 웹소설 작가 지망생의 생성형 AI 활용 경험 탐색 | 조희정, 박기수 | 2026 | 한국출판학연구 52(2) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=51fcdbc638866b4a47de9c1710b0298d) | 10 aspirants: AI drafts discarded (world errors, genre/period mismatch, length, leakage) | V |
| 53 | 웹소설 창작에서 생성형 AI 위임의 경계 | 조희정, 박기수 | 2026 | 한국출판학연구 52(3) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=a9e23216b3fc9f787ecd42904f0c5d65) | 8 pros: first drafts and pivotal plot decisions non-delegable | V |
| 54 | 생성형 인공지능을 활용한 회귀 모티프 웹소설 창작 실험 | 엄채민, 서덕순 | 2025 | 구보학보 40 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=5b91b3d6479e1ed24884a65323211ff0) | ChatGPT learned the 회귀 trope but relied on repetitive structures | V |
| 55 | 생성형 AI 시대 플랫폼 콘텐츠 창작의 저작자성 판단에 관한 소고 | 황은진 | 2026 | 인문사회과학연구 69(2) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=e3cfce31cd6ccf02d18150b21a227875) | Authorship hinges on substantive human contribution | L |
| 56 | AI시대의 도래와 웹문예창작/교육 | 김준현 | 2023 | 돈암어문학 44 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=d3e2d26c59e5818147de9c1710b0298d) | AI for planning, review and scene description; test by genre | L |
| 57 | 텍스트마이닝과 네트워크 분석을 활용한 웹소설 연구 동향 | 나윤빈 | 2021 | 출판잡지연구 29(1) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=51246ef3c6468c634884a65323211ff0) | Research projects emphasize ICT and AI; papers emphasize genre | L |
| 58 | 중국 웹소설 플랫폼의 '진화'와 AI의 작용 | 이화진 | 2025 | 중국현대문학 111 | KO | [link](https://www.riss.kr/search/Search.do?isDetailSearch=N&searchGubun=true&viewYn=OP&query=%EC%9B%B9%EC%86%8C%EC%84%A4+%EC%83%9D%EC%84%B1%ED%98%95+AI&iStartCount=0&iGroupView=5&icate=all&colName=re_a_kor&order=%2FDESC&onHanja=false&strSort=RANK&pageScale=50&fsearchMethod=search&isFDetailSearch=N&sflag=1&searchQuery=%EC%9B%B9%EC%86%8C%EC%84%A4+%EC%83%9D%EC%84%B1%ED%98%95+AI&pageNumber=1) | Qidian and Fanqie AI ecosystems (comparison case) | L |
| 59 | 구비설화 기반 서사지도를 활용한 웹소설 창작 치료 도구 개발 및 실행 연구 | 김정희 | 2026 | 문학치료연구 80 | KO | [link](https://www.riss.kr/search/Search.do?isDetailSearch=N&searchGubun=true&viewYn=OP&query=%EC%9B%B9%EC%86%8C%EC%84%A4+%EC%83%9D%EC%84%B1%ED%98%95+AI&iStartCount=0&iGroupView=5&icate=all&colName=re_a_kor&order=%2FDESC&onHanja=false&strSort=RANK&pageScale=50&fsearchMethod=search&isFDetailSearch=N&sflag=1&searchQuery=%EC%9B%B9%EC%86%8C%EC%84%A4+%EC%83%9D%EC%84%B1%ED%98%95+AI&pageNumber=1) | Gen-AI web-novel therapy tool protecting the participant's agency | L |
| 60 | 챗GPT, 파파고, 인간 번역가 간의 한영 문학번역 차이점 연구 | 이창수 | 2024 | 번역학연구 25(2) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=4b8021cf7b4ee37f6aae8a972f9116fb) | ChatGPT about half of Papago's errors, some human-like creativity | V |
| 61 | AI 문학번역에서 프롬프트 엔지니어링이 번역 오류와 창의성에 미치는 영향 | 김현웅, 이상빈 | 2025 | 번역학연구 26(3) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=c605c96b7e08853d7f7a54760bb41745) | RGC prompts: more creativity and readability, more over-translation, same total errors | V |
| 62 | AI 페르소나가 문학 번역에 미치는 영향 | 마승혜 | 2025 | 통번역학연구 29(4) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=be3f028155108112d18150b21a227875) | Deborah Smith (*Vegetarian*) persona moved ChatGPT from literal to Smith-like | L |
| 63 | 생성형 AI 문학 번역에서 은유 재현의 한계와 가능성 | 최승혁, 박응석 | 2026 | 한국언어문화학 23(2) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=51982e11fdd79393e9810257f7042666) | 243 metaphors: AI retains images, humans localize (4.32 vs about 3.85) | L |
| 64 | 생성형 AI 페르소나를 활용한 시 번역의 문체 재현과 창조적 변용 | 임형재, 염선화 | 2025 | 우리말 글 107 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=65fb80a3caec5fbe6aae8a972f9116fb) | 이용악 persona reproduced Korean poetic diction and imagery | L |
| 65 | AI 문학번역, 어디까지 가능한가 (작품 제목) | 마승혜 | 2024 | 번역학연구 25(3) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=d22ee0ac243b177ed18150b21a227875) | ChatGPT titles cover fewer title functions | L |
| 66 | ChatGPT를 활용한 문화소 번역연구 (『대장금』 한중) | 방영풍, 서연주 | 2024 | 國際言語文學 59 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=d80a2852192163516aae8a972f9116fb) | ChatGPT handled cultural terms well | L |
| 67 | ChatGPT의 한영 번역 도구로서의 정확성 분석 | 윤여범 | 2023 | 한국초등교육 34(4) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=29a94e6b976ee8557f7a54760bb41745) | Proverbs least accurate | L |
| 68 | 챗GPT를 적용한 번역수업 실천 사례 연구 | 이선화 | 2023 | 번역학연구 24(3) | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=8428b02d0b78e78ac85d2949c297615a) | Role, audience and strategy prompts improved literary MT | L |
| 69 | 생성형 인공지능(AI)의 한국문학 영어번역의 딜레마와 가능성 | 이형진 | 2025 | 외국문학연구 100 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=1d92ffab66ff50d74884a65323211ff0) | Human translators keep a distinct role | L |
| 70 | 생성형 AI 기반 한국 문학작품의 한어 번역 질 비교 연구 | 김미영 et al. | 2026 | 중국조선어문 264 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=6c9c1d698540a8617ecd42904f0c5d65) | ChatGPT more stable than DeepSeek across genres | L |
| 71 | 챗GPT 등장 이후의 인공지능(AI)과 문학 번역 (채만식 대화문) | YUCUILING, 박지숙 | 2024 | 중국학보 110 | KO | [link](https://www.riss.kr/search/detail/DetailView.do?p_mat_type=1a0202e37d52c72d&control_no=a1deb1702f95ca6a47de9c1710b0298d) | MT vs ChatGPT on literary dialogue | L |
| N1 | 카카오브레인 AI 시인 시아, 첫 시집 『시를 쓰는 이유』 | 장가람 (iNews24) | 2022 | News | KO | [link](https://www.inews24.com/view/1505892) | KoGPT + 13,000 poems; 53 poems curated by Slitscope | News |
| N2 | AI 작가, 어디까지 왔을까? | 룩말 에디터 (ETNews) | 2023 | News | KO | [link](https://www.etnews.com/20230713000255) | 비람풍 wrote within director 김태연's storyboard via repeated re-prompting | News |
| N3 | AI로 웹소설 한편 뚝딱…'5000자 구원투수' Vs '대필작가' | 원태영 (Economist Korea) | 2026 | News | KO | [link](https://economist.co.kr/article/view/ecn202607280044) | Contest AI bans, no detection, readers spot 번역투 and flat prose | News |

### Inferences
- Across five research streams (technical generation, evaluation, literary criticism, writer studies, translation studies), the same three Korean-specific risks recur:
  - drift and template repetition over long text;
  - culturally ungrounded or "translated-feeling" language (번역투, retained foreign imagery, honorific errors, over-regular commas, spacing and orthography);
  - loss of authorial ownership.
- The prompt and linter design should be judged against these three risks first.
- Korean evidence that persona and exemplar conditioning transfers diction (마승혜 2025; 임형재 & 염선화 2025; Cha et al. 2024) makes the writer's own manuscript the most valuable conditioning input WriterProgram has. Generic "literary Korean" instructions are weaker.
- Because Korean LLM judges are unreliable on literary quality, the linter (deterministic, rule-based metrics drawn from KatFish and Park/Yu/Kim 2026) is likely to be a more trustworthy quality gate than LLM self-critique. LLM self-critique is still useful for consistency against the setting bible.

### Gaps
- No Korean fiction generation benchmark, no Korean web-novel generation corpus paper, and no peer-reviewed evaluation of HyperCLOVA X, EXAONE, Solar, Polyglot-Ko or KoAlpaca on fiction were found. This is the largest gap.
- No study quantifies which Korean figurative expressions or literary vocabulary LLMs overuse (a "Korean slop list"). The closest evidence is structural (commas, spacing, line endings). Other researchers' notes on vocabulary and figurative language should fill this gap.
- About half the table is listing-level ([L]). Before citing their specific claims in a final report, the report writer should treat them as abstract-snippet evidence. Full texts on DBpia/KISS were not accessed.
- Han Kang-specific LLM translation analysis (ideophones, endings, minimalism) was not found beyond the persona study.
