# Context handling for Korean fiction generation: Korean discourse and pragmatics, translationese, long-form coherence

Researcher 05 notes for the WriterProgram Korean-fiction system prompt / skill / style linter. Scope: how LLMs handle Korean speech levels and address terms, zero anaphora and pronouns, narration conventions, translationese and prompt language, long-form coherence, and context formats. 36 sources were opened and checked (as of 2026-10-17). Leads I found but could not open are listed under Gaps and marked unverified.
Legend: "→ P:" = implication for the prompt / context package; "→ L:" = implication for the linter.

---

## 1. How well do LLMs handle Korean speech levels (높임법), honorifics and address terms (호칭), and keep them consistent with character relationships?

### Takeaway
Current LLMs reliably tell polite speech from casual speech, but finer speech-level control is unstable. They confuse 해요체 with 하십시오체, default to overly polite forms when the relationship is not given, and infer relationships from Korean dialogue much worse than from English (58–69% vs 75–80%). So each speaker→listener register has to be stated explicitly in the context and checked with a morphological (sentence-ending) checker. Neither the model itself nor an LLM judge is reliable enough for this.

### Cited Findings
- **LoCar (11 LLMs, including 6 Korean-developed models plus global APIs; the figure legend shows claude-haiku-4.5, gpt-5.1, gemini-2.5-flash, deepseek-v3.1 and Qwen3).** Each test item was randomly assigned one of three of Korean's "six-level honorific system" registers: 해체 (hae), 해요체 (haeyo) or 하십시오체 (hapsyo). Models were told to answer in that register. The paper concludes that "fine-grained honorific control remains unstable in current LLMs, which often fail to maintain fixed speech levels." It also reports: "Models frequently conflate haeyo and hapsyo, despite both encoding politeness"; "While most models achieve high compliance for coarse politeness distinctions, fine-grained register control—particularly between haeyo and hapsyo—remains inconsistent. Performance gaps are more pronounced for deferential forms." — [Jeong et al., LoCar, arXiv 2605.21086 (ACL 2026 Industry Track)](https://arxiv.org/abs/2605.21086)
  - → P: For each pair, give the actual endings (e.g., 하십시오체 = -습니다/-습니까/-십시오/-ㅂ시다), not just a label like "존댓말". The 해요/하십시오 boundary is the main risk: butlers, soldiers, secretaries, formal subordinates, announcer-style speech.
  - → L: Speech-level checking is needed even for the strongest models.
- **LoCar's evaluation design.** LLM-only judges "struggled to reliably distinguish between closely related Korean honorific forms (e.g., haeyo vs. hapsyo)." The authors added a sentence-level morphological suffix check, so that any sentence violating the target level fails. This raised human–judge agreement from **0.69 to 0.94 (+24 pp)**. The largest gains were for hapsyo (e.g., +0.52 agreement for the gpt-5-mini judge). The suffix cues were: hae "어, 해, 야, 다, 까"; haeyo "요"; hapsyo "오, 다, 까". A suffix belonging to a different level "triggers automatic filtering." The rule check acts as a "high-precision constraint filter" and the LLM handles context. — [LoCar full text, §3.3 and App. E.1](https://arxiv.org/pdf/2605.21086)
  - → L: This is the most direct evidence for the linter. Use a rule-based ending classifier as a high-precision first gate and an LLM for contextual judgment (a hybrid). Never let an LLM self-check be the only speech-level gate.
- **Translation of Korean honorifics.** Two settings were tested: implicit (sentence only) and explicit (addressee specified). Translation-tuned LLMs prefer polite forms more consistently than instruction-tuned LLMs in both settings. LM-based models "become more casual when the addressee is known." Compared with human judgments, translation-tuned LLMs "systematically overuse polite forms." — [Cihlar et al., "Investigating the Automatic Translation of Korean Honorifics," IAAI Workshop 2026, ELRA](https://aclanthology.org/2026.iaai-1.3/)
  - → P: Without explicit relationship information the model's default is polite. Intimate 반말 relationships (friends, lovers, siblings, boss→subordinate) must be stated, or they drift into 존댓말. Specifying the addressee measurably changes the register the model picks.
- **SCRIPTS (1.1k movie-script dialogues in English and Korean; nine LLMs).** The task is inferring the relationship between speakers. Accuracy was about **75–80% in English vs 58–69% in Korean**. Thinking models and chain-of-thought gave "minimal benefit" and sometimes amplified social biases. — [Kim et al., "Are they lovers or friends?", arXiv 2510.19028 (ACL 2026)](https://arxiv.org/abs/2510.19028)
  - → P: Models read relationships off Korean dialogue poorly, so "let the model infer it from earlier chapters" is unreliable. Relationships must be written into the setting book.
- **SCRIPTS error analysis (full text).**
  - "Korean relies more heavily on terms of address and honorifics to encode relational information in dialogue."
  - Korean speakers "often use terms of address to refer to themselves" (e.g., a teacher saying "선생님이…"). Models misread this as a third person.
  - Missing honorifics signalled peers, but GPT-4o labelled the pair supervisor–subordinate.
  - 어머니 used for a friend's mother or an older woman was read as parent–child. Qwen read 형수 as "older brother" and 형님 as "father."
  - In 30 GPT-4o failures, **46% of Korean errors** came from language- or culture-specific features. English errors were led by failure to combine cues (36.7%).
  - Per-model "Highly Likely" rate, English vs Korean: GPT-4o 0.791 vs 0.693; Gemini-2.5-Flash 0.758 vs 0.589; o3 0.807 vs 0.742.
  - Source: [Kim et al., full text](https://arxiv.org/html/2510.19028v3)
  - → P: For each pair, record the address term (호칭), the self-reference term (자칭: 저/나/형/엄마/선생님) and kinship terms with their real referents.
  - → L: Check that each pair uses the registered 호칭 and 자칭.
- **Korean indirect speech acts.** The best model, Claude 3 Opus, scored 71.94% on multiple choice and 65% on open-ended questions. No LLM reached human performance. Most models (all except Claude 3 Opus) did significantly worse on indirect than on direct speech acts. — [Koo et al., arXiv 2502.10995](https://arxiv.org/abs/2502.10995)
  - → P: Subtext is a weak spot. Scene cards should state each character's unspoken intent, so the model writes indirectness on purpose (refusing by trailing off, hinting) instead of having to infer it.
- **Politeness of the Korean prompt itself (PACLIC 2024).** Five request-politeness levels were tested, from L1 "질문에 알맞은 답을 골라." to L5 "질문에 알맞은 답을 골라주면 좋겠어."
  - GPT-4 accuracy fell from 59.3% to 55.8% as politeness rose; CLOVA X rose from 41.6% to 46.9% (both significant by regression).
  - GPT-4's explanation rate rose from 85.8% to 100%, and its response length from 335.75 to 467.13 syllables (CLOVA X: 83.24 → 198.88).
  - Mixtral and Solar showed no consistent pattern.
  - The authors note that for multilingual models, "directly and concisely stating the desired outcome is more effective than focusing on politeness."
  - Source: [Jung et al., PACLIC 38 (2024), pp. 894–903](https://aclanthology.org/2024.paclic-1.86)
  - → P: The register of a Korean instruction shifts how verbose the output is. Write the system prompt in plain, direct Korean and state length targets explicitly.

### Inferences
- In Korean fiction, speech level belongs to a relationship, not to a character. The same character uses 반말 with a sibling and 하십시오체 with a superior. The setting book should therefore store an **asymmetric matrix of ordered pairs (A→B and B→A)**. Each cell holds the level, 호칭, 자칭 and the conditions for switching (public vs private settings, anger, after a confession). Character-level speech style (verbal tics, dialect, favorite endings such as -거든/-다만) is a separate field.
- Models confuse 해요체 and 하십시오체 and default to politeness. So the prompt should give concrete ending lists plus one or two verbatim sample lines per pair, and the linter should check every dialogue line against the matrix.
- Register switches are plot beats in Korean fiction: dropping into 반말 signals intimacy or contempt. Because models drift toward politeness unless told otherwise, an intended switch should be declared in the scene card (e.g., "이 장면에서 민준이 처음으로 반말을 쓴다").
- The narration register (usually 해라체 narration, but some web novels and children's stories narrate in 해요체) is a global setting, separate from the dialogue registers. This is inference; I found no study testing it.

### Gaps
- I found no study measuring speech-level consistency across a whole scene or chapter of LLM-generated fiction, or drift over long generation. LoCar's multi-turn dialogues are only 3–5 turns long.
- No benchmark covers 하게체 or 하오체, the archaic registers common in 사극 and 무협 web novels. LoCar tests only three of the six levels.
- **Found but not opened (unverified):**
  - "Context-Aware Neural Machine Translation for Korean Honorific Expressions," *Electronics* (authors and year not confirmed; the ProQuest page failed to load). Search snippets say a contextual encoder plus context-aware post-editing (CAPE) improved honorific translation over context-free baselines, using speaker relationships from neighboring sentences.
  - KU×Upstage formality-control system for English→Korean ([IWSLT 2023 PDF](https://aclanthology.org/2023.iwslt-1.40.pdf)).
  - The KBS benchmark suite, whose Ko-EQ-Bench component includes honorific usage ([arXiv 2610.21227](https://arxiv.org/html/2610.21227v1)).

---

## 2. Zero anaphora, pronouns (그/그녀) and subject omission

### Takeaway
Korean prose usually drops a continuing topic or subject, or repeats the noun. 그 and 그녀 are marginal, written-only words that mostly mark the protagonist in literary fiction, and rendering "she" as 그녀 is a recognized translation artifact. I found **no study measuring how often LLMs over-produce explicit subjects or pronouns when writing Korean**. However, LLMs do struggle with zero pronouns in pro-drop languages, and pronoun frequency is a standard marker of translationese in LLM output. Subject and pronoun density is therefore a prime linter target. The context fed to the model should name referents explicitly even though the output prose should omit them.

### Cited Findings
- **Roh & Lee (2003): when Korean drops the subject.**
  - "A redundant noun in Korean is frequently omitted while a redundant noun in English is replaced by a pronoun." In an English→Korean example, translating *her* as 어머니 is "more natural than" 그녀. Their generator therefore chooses only between repeating the noun and omitting it (a zero pronoun).
  - Data: 87 texts (news, stories, descriptive texts) with omitted elements manually restored. In "Continue" transitions, where the same topic carries on, **46% (175/374)** of topics were omitted.
  - When the topic was easy to infer ("cheap" Continues), **74% (151/203)** were omitted, versus 14% (24/171) when it was hard to infer. 86% of all zero pronouns came from easy-to-infer Continues.
  - At a topic shift, the old and new topic "had better not be omitted."
  - Citing Ryu (2000): written texts omit only 26% of continuing topics, and only 6% in "Smooth-Shift" transitions, where 86% are stated with 은/는. Zero pronouns are rarer in written than in spoken texts.
  - Source: [Roh & Lee, ALTA 2003, pp. 30–37](https://aclanthology.org/U03-1004/)
  - → P: An operational rule: when the same character remains the topic of consecutive sentences, drop the subject; in the first sentence of a topic shift, re-establish it with the name plus 은/는. Even written narration does not omit 100% of the time, so don't over-correct into ambiguity.
  - → L: Flag runs of sentences that each open with the same explicit subject (name, 그, 그녀).
- **안소진 (2008).** 그 and 그녀 "play a very minor role." They appear only in written language, and even there writers "usually repeat nouns instead." Their main use is in short stories, where they mark the referent as the main character (a focus marker, 초점 표지). Given all the restrictions, they "almost do not exist" as third-person pronouns. — [안소진, 「소위 3인칭 대명사 '그, 그녀'의 기능에 대하여」, 한국어학 38](https://scholarworks.sookmyung.ac.kr/item/5715ab4b-0718-425e-8b92-c6efbf13dfed)
  - → P: 그/그녀 can be reserved for the POV or focal character in third-person literary fiction if the author writes that way. They should never be a generic pronoun for any character.
  - → L: Count 그/그녀 per 1,000 characters and flag their use for non-focal characters.
- **이상원 (2012).** 그녀 appeared in the early 20th century to translate Western female third-person pronouns. It never became a full pronoun and now works as a stylistic device with constraints. "She" cannot be automatically replaced with 그녀. The paper closes with strategies for avoiding it. — [이상원, 「나는 왜 '그녀'를 꺼리는가」, 통역과 번역 14(1), 193–211](https://www.earticle.net/Article/A178001)
  - → P / L: A habit of writing "she" as 그녀 is a translationese marker. Prefer the name, a role noun (언니, 어머니, 팀장) or omission.
- **김순영 (2012)** cites the classic example "그녀는 아름다운 목소리를 가지고 있다" → "그녀는 목소리가 아름답다" (English *have* carried over as 가지다). — [김순영, 「영한 번역에 나타난 번역투 문장」, 새국어생활 22(1)](https://www.korean.go.kr/nkview/nklife/2012_1/22_0105.pdf)
  - → L: Unneeded 그녀 tends to co-occur with other English-transfer patterns.
- **Valentini et al. (2026)** used pronoun frequency as one of their translationese indicators. A classifier labelled large shares of LLM output as "translated" (German: Gemini 60.2%, Llama 37.6%). — [Valentini et al., arXiv 2608.17399 (COLM 2026)](https://arxiv.org/html/2608.17399v1)
  - → L: Pronoun rate is a standard, measurable translationese feature.
- **Wang et al. (2023), survey of zero pronoun translation.** Zero pronouns dropped in pro-drop languages must be restored when translating into languages like English. Identifying the antecedent is the core difficulty. General-purpose metrics are unreliable for this, so targeted metrics are needed. Zero pronouns also carry a risk of gender bias. — [Wang et al., ACL 2023](https://arxiv.org/abs/2305.10196)
  - → L: BLEU-style metrics or an LLM's "overall quality" score will not catch the reverse error, over-explicit pronouns. Use a dedicated metric such as explicit-subject density. Gender errors (그 vs 그녀) are a known failure.
- **Li, Chen & He (2026).** Chinese zero pronouns "remain highly challenging for current LLMs," especially the upstream tasks of identifying them and classifying whether they refer. Even state-of-the-art reasoning LLMs correctly translate fewer than half of Chinese zero pronouns into English. — [Li et al., arXiv 2605.31056](https://arxiv.org/abs/2605.31056)
  - → P: When the model reads a Korean manuscript full of omissions, it may get wrong who did or said what. The summaries and scene cards we feed it should name referents explicitly, even though the generated prose should omit them.

### Inferences
- The context and the output should follow opposite rules. Summaries and cards should be maximally explicit (names, no omitted subjects) to protect the model's comprehension. The generated manuscript should follow Korean omission norms.
- Dialogue tolerates more omission than narration (Ryu 2000, as cited by Roh & Lee, finds omission rarer in written text). Linter thresholds for dialogue and narration should differ.
- Heavy use of 그 for the focal character is a legitimate authorial choice in some literary prose. The linter should calibrate against the author's own earlier chapters, not a single universal threshold.

### Gaps
- I found no paper measuring overt-subject or pronoun overuse in LLM-generated Korean (or Japanese) prose. The belief that "LLMs overuse 그녀" is common among practitioners but unquantified in everything I found.
- I found no Korean zero-anaphora benchmark for LLMs. Searches in Korean and English returned only pre-LLM work.

---

## 3. Narration conventions: POV (시점), tense (-었다 / -ㄴ다), sentence endings, dialogue tags, quotation

### Takeaway
I found **no direct LLM study of Korean narration**: not of 시점, not of past vs present narration, not of dialogue tags. Adjacent evidence:
- POV, tone and style errors form their own error class in long LLM stories, and that class is local.
- LLM-written Korean has low structural diversity and predictable ending and length patterns, and explicit numeric targets can push these toward human norms.
- Korean quotation conventions can be checked with rules.

### Cited Findings
- **ConStory-Bench "Narrative & Style" category.** It covers Perspective Confusions, Tone Inconsistencies and Style Shifts. Perspective confusions have the **smallest distance between the established fact and the contradiction (4.7% of the text)**. Narrative & Style errors are almost uncorrelated with the other error categories. The authors suggest stylistic errors "may be handled with local consistency checks." — [Li et al., ConStory-Bench, arXiv 2603.05890](https://arxiv.org/html/2603.05890v1)
  - → L: POV and tense drift are local, so a paragraph-level check (declared POV/tense vs what is observed) fits.
  - → P: Declare the POV and base tense for every scene.
- **LLM-generated Korean poetry (Park, Yu & Kim, 2026).** LLM poems read like "line-broken prose." Four form dimensions separate them from human poems:
  - Volume (length);
  - Structure Variation ("diversity and connective use of line-final forms");
  - Rhythmic Irregularity ("irregularity of line lengths");
  - Normative Adherence (standard orthography).
  - A five-feature logistic regression detects LLM poems with AUC-ROC 83.60 on seven unseen LLMs (KatFishNet: 75.84). Steering generation toward feature targets moved GPT-5.2 and Gemini-3 outputs' "targeted length, rhythm, and ending statistics… toward the human distribution," and experts preferred the guided poems.
  - Source: [Park et al., arXiv 2608.28986 (Findings of EMNLP 2026)](https://arxiv.org/abs/2608.28986)
  - → P: Numeric targets for ending variety and sentence-length variation work as prompt constraints.
  - → L: Measure ending diversity and the variance of sentence length.
- **KatFishNet.** LLM-written Korean has lower part-of-speech (POS) n-gram diversity than human writing (models "reuse common structures"), follows spacing rules strictly and overuses commas (numbers in Q4). — [Park et al., KatFishNet, arXiv 2503.00032 (ACL 2025)](https://arxiv.org/html/2503.00032v4)
  - → L: Structural monotony (repeated sentence templates) is measurable.
- **그/그녀 as a protagonist marker.** They appear mainly in short stories to mark the main character. — [안소진 2008](https://scholarworks.sookmyung.ac.kr/item/5715ab4b-0718-425e-8b92-c6efbf13dfed)
  - → P: In third-person limited narration, how the focal character is referred to is itself a POV signal; tie it to the POV declaration.
- **Quotation particles (국립국어원 newsletter, 2005).** "큰따옴표 뒤의 '-고'는 '-라고'로 고쳐 써야 한다": use -라고 after double quotes and -고 after single quotes, with the sentence-final period inside the quotation marks. — [한규희, 「문장 부호의 통일이 필요하다」, 새 국어 소식 (국립국어원), 2005-11](https://www.korean.go.kr/nkview/nknews/200511/88_9.html)
  - → L: Flag "…"고 (suggest "…"라고) and ‘…’라고 (suggest ‘…’고).
- **Omission differs by register.** Ryu (2000), cited by Roh & Lee, finds different omission rates in written and spoken text. — [Roh & Lee 2003](https://aclanthology.org/U03-1004/)
  - → L: Use separate thresholds for narration and dialogue.

### Inferences
- The scene card should declare:
  - 시점: first person (protagonist or observer), or third person (limited or omniscient);
  - the POV character;
  - the base narration tense: -었다 by default, or -ㄴ다 present-tense narration if the author uses it;
  - the narration register: 해라체, or the 해요체 narration some web novels and children's stories use;
  - how inner monologue is shown: in ‘ ’, or as free indirect style.
- Lack of variety in sentence endings is probably the most visible "AI 티" in Korean narration (runs of "-었다." sentences). Park et al. (2026) show that ending statistics both separate LLM from human text and can be steered. That supports stating explicit targets (e.g., "같은 종결형이 세 문장 넘게 이어지지 않게; 현재형·명사형·의문형·말줄임을 섞는다") and linting them.

### Gaps
- Searches in English and Korean found no LLM study of Korean narration tense choice, tense consistency, first vs third person, or overuse of dialogue tags (말했다/물었다).
- **Found but not opened (unverified):** "The Narrative Tense in the Korean Novel: A Speculative Observation," *Korean Studies* 5 (1981), pp. 79–91, doi:10.1353/ks.1981.0001 ([Project MUSE](https://muse.jhu.edu/article/397691), blocked by a verification page). The search snippet says it argues that present-tense verbs dominate narrative passages in the modern Korean novel. Check it in a library before relying on it.
- The 국립국어원 rule that 작은따옴표 marks "마음속으로 한 말" (thoughts) appeared only in search summaries; I did not open the official punctuation-rules text. It is standard, but unverified here.

---

## 4. Translationese (번역투) in LLM-generated Korean: what it looks like, how to measure it, how to reduce it, and whether prompt language matters

### Takeaway
LLM output in languages other than English carries measurable translationese. It is mostly statistical (function words, pronouns, punctuation, syntax) rather than outright errors, and plausibly comes from English-centric internal processing and from supervised fine-tuning data. What helps or hurts:
- Prompting in the target language improves naturalness for languages far from English (consistently for Chinese, mixed for French).
- Rewrite prompts that spell out specific linguistic rules help.
- Plan-in-English-then-translate pipelines are a poor fit.

For Korean, the best-documented machine-detectable traces are commas after connective endings and conjunctive adverbs, low structural diversity, and the classic 번역투 constructions catalogued by Korean translation studies.

### Cited Findings
- **Valentini et al. (2026): translationese in direct LLM generation.**
  - Setup: Gemini-2.0-Flash and Llama-3.3-70B, in English, German, Spanish, Greek and Pashto (no Korean). Features: type-token ratio, function words, sentence and word length, pronoun and punctuation frequency, POS n-grams. Tasks: continuing a European Parliament speech and writing Wikipedia intros, plus pipelines that wrote in English and then ran Google Translate.
  - Headline: "MLLMs do in fact generate text that resembles translationese," more strongly in non-English languages. A classifier labelled as translated: German Gemini 60.2%, German Llama 37.6%, Spanish average 45.7%.
  - Human annotators flagged far fewer (German Gemini 0%; Spanish 28.6% for both models), mostly for unusual words or word combinations.
  - Models avoid rare, marked function words but distribute the common ones differently from native text. Greek and Pashto showed less translationese.
  - Source: [Valentini et al., arXiv 2608.17399 (COLM 2026)](https://arxiv.org/html/2608.17399v1)
  - → L: Translationese in direct generation is largely statistical. A corpus-level linter that compares rates against a native baseline catches what line-by-line human reading misses.
  - → P: Write directly in Korean.
- **Guo et al. ("Do Large Language Models Have an English Accent?").**
  - Metrics: lexical naturalness (Jensen-Shannon divergence of vocabulary) and syntactic naturalness (divergence of dependency trees), both measured against human text.
  - All models are far from human. Chinese syntactic divergence: best model Llama-3.1 at 10.32 vs a human reference of 2.93.
  - "For Chinese outputs, prompting in Chinese consistently leads to better naturalness," and "prompting in English degrades the naturalness of generated Chinese." For French, results vary by model.
  - Llama-3.1's Chinese syntax shows "greater similarity to human-written English than to native Chinese."
  - Preference training (DPO) with the human-written response as "chosen" and a back-translated version as "rejected" gave consistent but modest gains (e.g., Llama-3.1 QA syntactic divergence 12.19 → 9.44) without hurting general benchmarks.
  - Source: [Guo et al., arXiv 2410.15956 (ACL 2025)](https://arxiv.org/html/2410.15956v3)
  - → P: Korean, like Chinese, is far from English typologically. Write the system prompt, style rules and examples in Korean. Pairs of original Korean vs back-translated Korean make good bad/good few-shot contrasts and linter calibration data.
- **Liu et al. ("Is Translation All You Need?").**
  - Translating queries into English helps English-centric LLMs on NLP tasks.
  - On culture-related exams (M3Exam), native-language prompting worked better for models not centered on English (translation hurt Qwen1.5-72B and Yi-34B on Chinese); for ChatGPT results were mixed. The abstract says native prompting is more effective for culture-related tasks needing deep language understanding.
  - On real user queries (ShareGPT), ChatGPT won more often with the original queries in high-resource languages (Japanese, Chinese, Spanish), while Llama-2-70B-Chat preferred translated ones.
  - Source: [Liu et al., arXiv 2403.10258 (NAACL 2025)](https://arxiv.org/html/2403.10258v3)
  - → P: Korean fiction is a culture-heavy generation task, so prompt in Korean. Avoid "plan in English, translate to Korean."
- **Schut et al. ("Do Multilingual LLMs Think In English?").** Multilingual LLMs "make key decisions in a representation space closest to English," whatever the input or output language. For meaningful words, models form an English-like representation before producing the target-language word (tested in French, German, Dutch and Mandarin). — [Schut et al., arXiv 2502.15603](https://arxiv.org/abs/2502.15603)
  - → P: This is a mechanistic reason why calques and English collocations leak even with Korean prompts. Anchor word choice with Korean lexical material: setting-book terms and verbatim character lines. There is no evidence behind telling the model to "think in Korean" (see Gaps).
- **Li et al. ("Lost in Literalism").** LLM translations stay overly literal, and the bias is traced to supervised fine-tuning. Polishing reference translations and filtering unnatural training examples reduce translationese, by human and automatic evaluation. — [Li et al., arXiv 2503.04369 (ACL 2025)](https://arxiv.org/abs/2503.04369)
  - → P / L: Translationese is a trained-in habit. Prompts reduce it but won't remove it, so a lint-and-revise pass is needed.
- **Kunilovskaya et al. (EAMT 2024).** GPT-4 rewrote translated text to look like original target-language text more successfully when the prompt included specific linguistic instructions than when it relied on the model's own knowledge (German–English, European Parliament data). — [Kunilovskaya et al., "Mitigating Translationese with GPT-4," EAMT 2024, pp. 411–430](https://aclanthology.org/2024.eamt-1.35/)
  - → P: The revision prompt should list concrete 번역투 patterns and their fixes, not just say "자연스럽게 다듬어라."
- **KatFishNet: how LLM Korean differs from human Korean.** Models: GPT-4o, Solar, Qwen2-72B, Llama3.1-70B; genres: essays, poetry, paper abstracts. LLM vs human:

  | Feature | Essays | Poetry | Abstracts |
  |---|---|---|---|
  | Sentences with at least one comma | 61.03% vs 26.31% | 42.90% vs 27.01% | 65.21% vs 47.48% |
  | Commas after a connective ending | 19.83% vs 4.10% | 15.57% vs 4.68% | 28.01% vs 13.27% |
  | Commas per morpheme | 2.56% vs 1.13% | – | – |

  - LLMs also add commas after conjunctive adverbs (그러나, 하지만), which the authors attribute to English punctuation habits.
  - LLMs follow spacing rules strictly and show lower POS diversity.
  - A detector built on comma features reached AUROC 94.88 on essays.
  - Source: [Park et al., KatFishNet (ACL 2025)](https://arxiv.org/html/2503.00032v4)
  - → L: Concrete rules: flag "-고, / -며, / -지만, / -는데, / -면서," and "그러나, / 하지만, / 그리고," and compare commas per sentence against a native-fiction baseline.
- **김순영 (2012, 국립국어원 『새국어생활』): a taxonomy of English→Korean 번역투**, building on 김정우 (2007) and 이근희 (2005), with fixes:
  - **Sentence structures carried over**
    - *have* → 가지다: "목소리를 가지고 있다" → "목소리가 아름답다"
    - *too … not to*: "…기에는 너무 …했다"
    - passive with *by*: "용감한 시민에 의하여 붙잡혔다" → "시민한테 붙잡혔다"
    - agentless passive: "노력이 기울여졌다" → "노력을 기울였다"
  - **Plural marking carried over**
    - "아이디어들과 방법들" → "여러 가지 … 생각과 방법"
    - -들 is not neutral: "책이 많다" and "책들이 많다" differ in meaning.
  - **English prepositional phrases carried over**
    - "~에 관하여"
    - "~로부터": "그 사람으로부터" → "그 사람에게서"; "긴장으로부터의 해방" → "긴장에서 벗어나는"
    - "~을 통해": "이번 기회를 통해" → "이번 기회에"
    - "~에 의해": "폭격에 의해" → "폭격으로"
    - stacked particles: "2층에서의 살림" → "2층에서 시작한 살림"
  - 김정우 (2003) found that English-origin 번역투 far outnumbered Chinese- and Japanese-origin types in school textbooks.
  - Source: [김순영 2012](https://www.korean.go.kr/nkview/nklife/2012_1/22_0105.pdf)
  - → L: This gives a sourced pattern list with suggested rewrites.
  - → P: Put four to six of these before/after pairs in the style sheet.
- **Prompt register matters.** More polite Korean requests changed GPT-4's and CLOVA X's verbosity and how often they explained (see Q1). — [Jung et al. 2024](https://aclanthology.org/2024.paclic-1.86)
  - → P: Keep the instruction register neutral and specify output length and format explicitly.
- **Cross-lingual prompt steerability.** Across five languages and three LLMs, system-prompt components such as chain-of-thought, emotion and scenario framing correlate with robust behavior across languages. Optimized prompts improved all metrics by 5–10% and showed "less unnecessary language-switching" in reasoning. — [Zhang et al., arXiv 2512.02841](https://arxiv.org/abs/2512.02841)
  - → P: Scenario framing helps across languages.
  - → L: Detect switches into other scripts (Latin letters, Chinese characters, Japanese kana) in Korean output.

### Inferences
- Korean is typologically far from English: verb-final, subject-dropping, agglutinative, with honorific morphology. It is closer to Guo et al.'s Chinese case than to French. The evidence therefore favors Korean system prompts and Korean examples by default. English is acceptable at most for machine-facing metadata such as JSON keys.
- The best mitigation stack is layered:
  1. A Korean prompt with Korean examples taken from the author's own manuscript.
  2. An explicit list of patterns to avoid, with replacements.
  3. A deterministic linter.
  4. A targeted LLM revision prompt that quotes the flagged spans and the specific rule (Kunilovskaya et al.).
- 번역투 rules should be applied with the author's style in mind. Some constructions (-들, passives, "것이다") are fine in moderation, so the linter should report density against a baseline rather than ban them.

### Gaps
- I found no study quantifying 번역투 in LLM-generated Korean fiction, and none comparing Korean vs English system prompts for Korean creative writing.
- I found no evidence for or against "think in Korean" or Korean-only reasoning instructions.
- **Found but not opened (unverified):**
  - Translationese-index (Liu et al., EMNLP 2025), a graded measure based on likelihood ratios; the search snippet reports r = 0.568 with human ratings ([arXiv 2507.12260](https://www.arxiv.org/abs/2507.12260)).
  - The SEACrowd translationese classifier ([arXiv 2406.10118](https://arxiv.org/pdf/2406.10118)).
  - The search-summary claim that English system prompts are only "marginally" worse than same-language prompts (attributed to the steerability paper) is not in that paper's abstract, so it is unverified.

---

## 5. Long-form coherence methods for serialized fiction: planning, memory, consistency checking, long-context limits

### Takeaway
Three things consistently beat end-to-end generation: planning before writing with detailed hierarchical outlines; re-injecting the plan and story state into every call; and labeled shared memory (scratchpads, state trackers). Even so:
- 2025–26 frontier models still make consistency errors that grow with length and cluster in the middle of the narrative; factual and timeline errors are the most common.
- Long-context comprehension of novels is weak, especially for heavy world-building.
- LLM-written summaries introduce plot holes.

What is needed is curated, author-verified story state, not just a bigger context window.

### Cited Findings
- **Re3.** Build a structured plan, then generate passage by passage with "plan and story-state context" injected into each prompt; finally rerank candidates for coherence and premise relevance and edit for factual consistency. Compared with direct generation of the same length: **+14% (absolute)** more stories with a coherent overall plot and **+20%** more relevant to the premise. Stories exceed 2,000 words. — [Yang et al., EMNLP 2022](https://arxiv.org/abs/2210.06774)
  - → P: Every chapter call should re-inject the plan and the current story state.
- **DOC.** A detailed, hierarchical outline moves creative work into planning, and a controller keeps passages faithful to the outline's details. Compared with Re3: **+22.5% plot coherence, +28.2% outline relevance, +20.7% interestingness**. It was also more controllable interactively. — [Yang et al., ACL 2023](https://arxiv.org/abs/2212.10077)
  - → P: Finer-grained scene beats in the scene card give better coherence.
- **Dramatron.** Hierarchical prompt chaining produces a title, characters, story beats and location descriptions, then dialogue. It was evaluated in a co-writing study with 15 theatre and film professionals. — [Mirowski et al., arXiv 2209.14958 (usually cited as CHI 2023; the venue is not shown on the arXiv page)](https://arxiv.org/abs/2209.14958)
  - → P: Build structure layer by layer, with characters and locations as explicit intermediate products.
- **Agents' Room.** Planning agents (conflict, character, setting, plot) write only to a shared scratchpad. Writing agents (exposition, rising action, climax, falling action, resolution) write the story. Each output is appended with its agent's label so later agents can refer to it. Human raters preferred Agents' Room variants over end-to-end baselines on every dimension (smallest gap for language use), although human-written stories were preferred overall. — [Huot et al., ICLR 2025](https://arxiv.org/html/2410.02603v2)
  - → P: Keep the story bible as labeled sections that every call can quote.
- **StoryWriter.** An outline agent writes event-based outlines (plots, characters, how events relate); a planning agent assigns events to chapters; a writing agent "dynamically compresses the story history based on the current event." Its LongStory dataset has about 6,000 stories averaging about 8,000 words. — [Xia et al., arXiv 2506.16445](https://arxiv.org/abs/2506.16445)
  - → P: Select the history relevant to the current event instead of feeding in every earlier chapter.
- **RecurrentGPT.** A natural-language long- and short-term memory, stored on disk and in the prompt and updated at each step. The memory is human-readable and editable, which enables interactive writing. — [Zhou et al., arXiv 2305.13304](https://arxiv.org/abs/2305.13304)
  - → P: WriterProgram's setting book is this kind of editable memory. The AI should propose updates and the author should approve them.
- **SCORE.** Tracks the status of key items, writes episode summaries and retrieves related episodes (retrieval-augmented generation) to detect and fix inconsistencies. It reports better consistency than baseline GPT models; the abstract gives no numbers. — [Yi et al., arXiv 2503.23512](https://arxiv.org/abs/2503.23512)
  - → P: Add state tracking for items and conditions: who holds what, injuries, secrets.
- **ConStory-Bench.** 2,000 prompts across four scenarios, with a taxonomy of 5 categories and 19 error subtypes:
  - **Timeline & Plot Logic:** absolute-time, duration and simultaneity contradictions; effects without causes; broken causal logic; abandoned plot elements.
  - **Characterization:** contradictions in what a character remembers or knows; skills that fluctuate; abilities that are forgotten.
  - **World-building & Setting:** violations of core rules or social norms; geographical contradictions.
  - **Factual & Detail:** mismatched appearance; confused names; mismatched quantities.
  - **Narrative & Style:** perspective confusions; tone inconsistencies; style shifts.

  Results across 21 models:
  - Error density per 10,000 words: GPT-5-Reasoning 0.113 (lowest), Gemini-2.5-Pro 0.305, Claude-Sonnet-4.5 0.520, MiniMax-M1-80k 3.447 (highest).
  - Contradictions cluster at **40–60%** of the narrative, while the facts they contradict are mostly established at 15–30%. Geographical contradictions have the longest gap between fact and contradiction (31.0% of the text).
  - Errors rise with output length (r = 0.478 for Claude-Sonnet-4.5; r = 0.973 for DeepSeek-V3.2-Exp).
  - Text containing errors has higher token-level entropy (+12.03% for Qwen3-30B-A3B, +19.24% for Qwen3-4B).
  - Factual & Detail errors co-occur with Characterization (r = 0.304), World-building (0.255) and Timeline (0.176) errors.
  - The authors suggest entropy as an early warning signal, long-range memory for timeline and geography errors, and local checks for style errors.
  - Source: [Li et al., arXiv 2603.05890 (Findings of ACL 2026)](https://arxiv.org/html/2603.05890v1)
  - → P: Mirror these categories in the setting-book schema: timeline, what each character knows, abilities, world rules, geography, appearance, names, quantities.
  - → L / checker: Concentrate checks on the middle of long chapters, and keep each generation chunk short.
- **NoCha.** 1,001 pairs of true and false claims about 67 recently published English novels. GPT-4o scored best at **55.8%**, and no open-weight model beat chance. Models do better on claims answerable from a single sentence than on claims requiring reasoning across the book. Their explanations are often wrong even when the label is right. They do noticeably worse on speculative fiction with extensive world-building. — [Karpinska et al., EMNLP 2024](https://arxiv.org/abs/2406.16264)
  - → P: Don't rely on the model absorbing a whole manuscript. World rules for fantasy, 무협 and 헌터물 must be stated explicitly in cards.
- **FlawedFictions.** State-of-the-art LLMs struggle to detect plot holes "regardless of the reasoning effort allowed," and performance "significantly degrad[es] as story length increases." LLM summarization and LLM story generation raise plot-hole rates by **more than 50% and 100%** respectively over the human originals. — [Ahuja, Sclar & Tsvetkov, arXiv 2504.11900 (COLM 2025)](https://arxiv.org/abs/2504.11900)
  - → P: AI-drafted chapter summaries must be checked by the author before entering the setting book. LLM consistency checkers are aids, not gates.
- **NoLiMa.** 13 LLMs advertising at least 128K-token contexts were tested on retrieval where the question shares few words with the needed fact. At 32K tokens, **11 of the 13 fell below 50%** of their short-context baseline; GPT-4o fell from 99.3% to 69.7%. Reasoning models and chain-of-thought did not fully prevent the drop. — [Modarressi et al., ICML 2025](https://arxiv.org/abs/2502.05167)
  - → P: Aliases and epithets ("그 남자", "대공", a nickname) don't match a card titled with the character's real name. The context builder should resolve aliases and insert the relevant cards directly.
- **Lost in the Middle.** Models use information best when it is at the beginning or end of the input and worst when it is in the middle, even models built for long context. — [Liu et al., TACL (arXiv 2307.03172)](https://arxiv.org/abs/2307.03172)
  - → P: Put critical constraints at the edges of the prompt, not in the middle of a long setting dump.
- **DramaBench.** Scores six dimensions (format, narrative efficiency, character consistency, emotional depth, logical consistency, conflict handling) using rule-based analysis, LLM labeling and statistics. The dimensions are essentially independent (mean |r| = 0.020). It covered 8 models and 1,103 scripts. — [Ma et al., arXiv 2512.19012](https://arxiv.org/abs/2512.19012)
  - → L: Evaluate character consistency separately from overall quality, combining rules and LLM labels.

### Inferences
- A serialized web novel runs daily episodes for hundreds of chapters, so the whole story will never fit in the context. Story state must be curated: the arc outline, each character's state, world rules, the timeline, and open threads and foreshadowing.
- Contradictions appear mid-text and increase with length. So generate a chapter in scene-sized chunks and re-inject the scene card for each chunk.
- Agents' Room's split (planners write to a scratchpad, the writer reads it) maps directly onto WriterProgram:
  - planning artifacts, such as scene cards, are editable documents;
  - the writing call reads them;
  - a checking call compares the output against them.

### Gaps
- I found no Korean-language long-form coherence benchmark; ConStory, NoCha and FlawedFictions are English or Chinese. Korean story-generation papers are covered by another researcher.
- I found no dedicated benchmark for character-voice consistency in prose fiction, as opposed to scripts and role-play.
- I could not locate or verify Fiction.LiveBench, a practitioner leaderboard for long-context fiction comprehension.

---

## 6. What context format works best (setting cards, previous-chapter summaries, scene goals, POV/tense declarations), and what is the evidence?

### Takeaway
No study compares story-bible formats for fiction head to head. The most transferable evidence:
- Put long reference material at the top and the task and critical constraints at the end. Put each document in labeled XML tags. For long-document tasks, have the model quote the relevant facts first.
- Position matters (the "lost in the middle" effect), and alias-only matches fail at long context. The context builder should pre-select material and resolve aliases.
- The choice of format (plain text, Markdown, JSON, YAML) changes results, but larger models are more robust to it.
- Four kinds of explicit input each measurably change output: relationship information, detailed outlines, specific linguistic instructions, and numeric style targets.

### Cited Findings
- **Anthropic's Claude prompting guide.**
  - "Put longform data at the top: Place your long documents and inputs near the top of your prompt, above your query, instructions, and examples. This improves performance across all models."
  - Wrap each document in `<document>` tags with `<document_content>` and `<source>` subtags.
  - "Ground responses in quotes: For long document tasks, ask Claude to quote relevant parts of the documents first before carrying out its task."
  - Tags such as `<instructions>`, `<context>` and `<input>` reduce misinterpretation.
  - Source: [Claude prompting best practices, platform.claude.com](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices)
  - → P: Setting book at the top in tags; scene card and instructions at the bottom. Optionally add a planning step ("먼저 이 장면에 필요한 설정을 인용하라") whose output does not go into the manuscript.
- **Position effects.** — [Liu et al., Lost in the Middle](https://arxiv.org/abs/2307.03172)
  - → P: Keep the bundle compact, and place the speech-level matrix for the characters in the scene near the end, next to the instructions.
- **Format effects.** The same content in plain text, Markdown, JSON or YAML gave different GPT results. GPT-3.5-turbo varied by up to 40% on a code-translation task; GPT-4 was more robust. — [He et al., arXiv 2411.10541](https://arxiv.org/abs/2411.10541)
  - → P: Pick one format, use it consistently, and A/B test it on the target model. Don't assume JSON beats prose.
- **Retrieval without literal word overlap collapses with length.** — [Modarressi et al., NoLiMa](https://arxiv.org/abs/2502.05167)
  - → P: Put alias lists in the cards and resolve aliases in the context builder.
- **Planning and memory structure.** Detailed outlines (DOC, +22.5% coherence over Re3) — [Yang et al. 2023](https://arxiv.org/abs/2212.10077); plan plus state re-injection (Re3) — [Yang et al. 2022](https://arxiv.org/abs/2210.06774); a labeled shared scratchpad — [Huot et al., Agents' Room](https://arxiv.org/html/2410.02603v2); history compressed around the current event — [Xia et al., StoryWriter](https://arxiv.org/abs/2506.16445); a human-editable memory — [Zhou et al., RecurrentGPT](https://arxiv.org/abs/2305.13304).
  - → P: A scene card with beats; plan and state re-injected in every call; labeled sections; only history relevant to the current events; memory the author can edit.
- **Explicit addressee information changes honorific choice.** — [Cihlar et al. 2026](https://aclanthology.org/2026.iaai-1.3/)
  - → P: The relationship and register matrix earns its place in the context.
- **Specific linguistic instructions beat relying on the model's own knowledge** when rewriting toward natural text. — [Kunilovskaya et al., EAMT 2024](https://aclanthology.org/2024.eamt-1.35/)
  - → P: State rules concretely, with examples.
- **Numeric feature targets** (length, rhythm, ending statistics) move output toward human distributions. — [Park, Yu & Kim 2026](https://arxiv.org/abs/2608.28986)
  - → P: Numeric style targets are a valid way to specify style.
- **LLM summaries add plot holes.** — [Ahuja et al., FlawedFictions](https://arxiv.org/abs/2504.11900)
  - → P: The summaries in the context should be author-verified.
- **Native-language prompts improve naturalness for a language far from English.** — [Guo et al.](https://arxiv.org/html/2410.15956v3)
  - → P: Write the instructions and card contents in Korean.

### Inferences
- A hybrid format is the evidence-consistent choice:
  - XML tags for the major sections;
  - compact Korean key–value lines or Markdown tables inside them for facts (relationship matrix, character cards);
  - short Korean prose for summaries;
  - verbatim manuscript excerpts for voice.
- Per call, include only what the scene needs: characters present or mentioned, locations, active threads. Add an always-on core: the POV/tense declaration and the style sheet.

### Gaps
- No controlled study compares story-bible formats (tables vs JSON vs prose) for fiction, or establishes how much verbatim previous text best preserves voice. These need in-house A/B tests.

---

## 7. Recommendations: what context to give the model, what rules to state, what a linter can check

### Takeaway
Give the model an explicit, curated, Korean-language context package:
- a speech-level and address-term matrix for each speaker→listener pair;
- character cards with aliases and verbatim sample lines;
- world rules, plus timeline and state;
- author-verified summaries and the last 1–2k characters of manuscript verbatim;
- a scene card declaring POV, tense, beats and subtext.

Long material goes at the top in XML tags; the scene card and rules go at the end.

State Korean-specific rules with concrete forms: speech levels with their endings, 호칭, subject omission, avoiding 그녀, the 번역투 list, comma habits, ending variety and quotation marks.

Back these with a deterministic linter (speech level, address terms, pronoun and subject density, 번역투 patterns, punctuation, monotonous endings, POV/tense drift, foreign-script intrusion, unknown names) plus an LLM checker for timeline, knowledge and world-rule contradictions that must quote evidence.

### Cited Findings
- The recommendations below synthesize §1–6; supporting sources are cited inline in brackets. The main ones: [LoCar](https://arxiv.org/abs/2605.21086), [Cihlar et al.](https://aclanthology.org/2026.iaai-1.3/), [SCRIPTS](https://arxiv.org/html/2510.19028v3), [Roh & Lee](https://aclanthology.org/U03-1004/), [안소진](https://scholarworks.sookmyung.ac.kr/item/5715ab4b-0718-425e-8b92-c6efbf13dfed), [이상원](https://www.earticle.net/Article/A178001), [김순영](https://www.korean.go.kr/nkview/nklife/2012_1/22_0105.pdf), [KatFishNet](https://arxiv.org/html/2503.00032v4), [Park et al. 2026](https://arxiv.org/abs/2608.28986), [Guo et al.](https://arxiv.org/html/2410.15956v3), [Kunilovskaya et al.](https://aclanthology.org/2024.eamt-1.35/), [ConStory-Bench](https://arxiv.org/html/2603.05890v1), [NoLiMa](https://arxiv.org/abs/2502.05167), [FlawedFictions](https://arxiv.org/abs/2504.11900), [Anthropic guide](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices).

### Inferences (recommendations)

#### A. Context package for each generation call (top → bottom)

| # | Block (XML tag) | Contents | Why (evidence) |
|---|---|---|---|
| 1 | `<style_sheet>` (always on) | Author's style rules in Korean; 3–5 번역투 before/after pairs; 2–3 paragraphs of the author's own prose as voice examples | Native-language prompts are more natural [Guo]; explicit linguistic rules help [Kunilovskaya]; anchors word choice [Schut] |
| 2 | `<world_rules>` | Core rules, social norms, geography (scene-relevant plus always-on core) | World-building-heavy fiction is hardest [NoCha]; world-building and geography errors [ConStory] |
| 3 | `<characters>` | For characters present or mentioned: name, **aliases/epithets** (also the retrieval keys), age/status, speech style with **verbatim sample lines**, what they know and don't know, abilities and limits, fixed appearance, current state as of chapter N | Characterization and factual subtypes [ConStory]; aliases fail at long context [NoLiMa] |
| 4 | `<relationships>` | **Ordered-pair matrix**: speaker→listener, speech level (with endings), 호칭, 자칭, switch conditions and history | Unstable fine-grained control [LoCar]; default to polite [Cihlar]; Korean relationship inference is weak [SCRIPTS] |
| 5 | `<timeline_state>` | Current in-story date and time, elapsed time, locations, items, injuries, secrets | Timeline errors most common [ConStory]; item-state tracking [SCORE] |
| 6 | `<open_threads>` | Foreshadowing and setups awaiting payoff, with planned payoff points | "Abandoned plot elements" subtype [ConStory] |
| 7 | `<story_so_far>` | Arc summary plus summaries of the last 1–3 chapters, **author-verified**, written with explicit names (no omitted subjects) | LLM summaries add plot holes [FlawedFictions]; models mis-resolve omitted referents [Li et al. 2026]; event-relevant history [StoryWriter] |
| 8 | `<previous_text>` | The last ~1,000–2,000 characters of manuscript, verbatim | Voice, tense and POV continuity (inference) |
| 9 | `<scene_card>` (near the end) | 시점 and focal character; base tense and narration register; characters; place and time; scene goal; 3–6 beats; **unspoken intent behind key lines**; any planned register switch; target length | Detailed outlines [DOC]; plan and state re-injection [Re3]; indirect speech is weak [Koo]; addressee info changes register [Cihlar] |
| 10 | `<instructions>` (last) | The task, plus a re-statement of the critical rules (register matrix for the characters in the scene, POV, tense) | Long data on top, task at the end [Anthropic]; middle content gets lost [Lost in the Middle] |

Format and assembly:
- Use XML tags for the blocks. Inside them, write compact Korean key–value lines or small Markdown tables. Keep the format identical across calls, and A/B test it on Claude [He et al.].
- The context builder picks cards by alias matching plus the scene card's character list, rather than dumping the whole setting book [NoLiMa; Lost in the Middle].
- Generate one scene at a time and re-inject the cards for each chunk [ConStory: errors grow with length and cluster mid-text].
- Optionally run a planning pass that first quotes the setting lines needed for the scene [Anthropic, "quote first"]; this output never enters the manuscript.
- Ask the model to list any names or facts it invented in a separate `<새_설정>` block for the author to approve [RecurrentGPT's editable memory; ConStory's name-confusion subtype].

Illustrative card snippet (format sketch only, not a tested template):
```
<relationships>
| 화자 → 청자 | 말 단계 (어미) | 호칭 | 자칭 | 전환 조건 / 이력 |
| 서윤 → 도윤 | 해요체 (-어요/-지요) | 팀장님 | 저 | 둘만 있어도 유지. 31화 고백 이후 반말 예정 |
| 도윤 → 서윤 | 해체 (-어/-지/-야) | 한 대리 (공적), 서윤 씨 (사적) | 나 | 화나면 "한서윤 씨" + 하십시오체 |
| 비서 → 도윤 | 하십시오체 (-습니다/-습니까) | 팀장님 | 저 | 절대 해요체로 바꾸지 않음 |
</relationships>
<character id="seoyun">
이름: 한서윤 | 별칭: 서윤, 한 대리, 윤아 (가족만)
말투: 짧게 끊어 말함. '-거든요'를 자주 씀. 화나면 오히려 더 깍듯해짐
예시 대사: "그건 제 일 아니거든요." / "알겠습니다. 그렇게 하시죠."
아는 것/모르는 것 (23화 기준): 도윤의 손목 흉터의 이유를 모름
현재 상태: 오른손 붕대 (22화)
</character>
```

#### B. Rules to state in the system prompt or skill (write them in Korean)
A draft synthesized from §1–4. The wording is mine and needs testing.
```
[말 단계·호칭]
- <relationships>의 '화자→청자' 말 단계·호칭·자칭을 그대로 따른다. 표에 없는 쌍은 첫 등장 때 정하고, 이후 바꾸지 않는다.
- 한 사람의 한 발화 안에서 말 단계를 섞지 않는다. 하십시오체(-습니다/-습니까/-십시오/-ㅂ시다)와 해요체(-어요/-지요/-네요/-ㄹ게요)를 구별한다.
- 말 단계 전환(반말 트기, 갑자기 존댓말)은 <scene_card>에 적힌 경우에만 쓴다.
[주어·대명사]
- 앞 문장과 같은 인물이 화제로 이어지면 주어를 생략한다. 화제가 다른 인물로 바뀌는 첫 문장에서는 이름이나 호칭으로 다시 세운다.
- '그녀'는 쓰지 않는다. '그'는 <style_sheet>가 허용할 때 초점 인물에게만 쓴다. 사람을 '그들', 사물을 '그것'으로 기계적으로 받지 않는다.
[번역투]
- '~을 가지고 있다'(속성·소유), '~에 의해', 불필요한 피동('-되어지다', '기울여졌다'), '~로부터', '~을 통해', '~에 대해/관하여', 불필요한 '-들', '~에서의/~와의' 같은 겹조사를 피하고 우리말 구조로 쓴다. (예: 목소리를 가지고 있다 → 목소리가 곱다 / 그 사람으로부터 → 그 사람에게서 / 이번 기회를 통해 → 이번 기회에)
- 연결 어미(-고, -며, -지만, -는데, -면서) 뒤나 문장 첫머리 접속 부사(그러나, 하지만, 그리고) 뒤에 습관적으로 쉼표를 찍지 않는다.
[서술]
- <scene_card>의 시점·초점 인물·서술 시제를 지킨다. 3인칭 제한 시점에서는 초점 인물이 모르는 사실이나 다른 인물의 속마음을 서술하지 않는다.
- 같은 종결형(예: '-었다.')이 세 문장 넘게 이어지지 않게 하고, 문장 길이를 섞는다.
- 대사는 큰따옴표 “ ”, 속으로 한 말은 작은따옴표 ‘ ’. 인용 조사는 “…”라고 / ‘…’고.
[언어·설정]
- 처음부터 한국어 문장으로 쓴다. 영어로 구상한 뒤 옮기지 않는다. 설정에 없는 영문·한자·가나를 섞지 않는다.
- 설정집·요약과 어긋나는 사실(나이, 외모, 능력, 지명, 시간 경과)을 만들지 않는다. 새로 만든 고유명사·사실은 원고 뒤 <새_설정>에 따로 적는다.
```

#### C. What the linter can check

Deterministic checks need a Korean morphological analyzer (for example Kiwi or MeCab-ko/ko-dic, or Lindera for Rust). I have not evaluated any of these.

| # | Check | Method | Evidence | Notes |
|---|---|---|---|---|
| L1 | Speech-level mismatch for a pair | Classify each dialogue sentence by its ending; compare with the matrix for the attributed speaker→listener | [LoCar]: a suffix check raised agreement 0.69 → 0.94; 해요/하십시오 confusion | Needs speaker/listener attribution (author markup, dialogue-tag parsing, or LLM attribution followed by a rule check) |
| L2 | Mixed speech levels within one utterance | Same classifier, applied within a single quoted span | [LoCar] | Allow an intentional 반존대 style if the card declares it |
| L3 | 호칭/자칭 mismatch | Match vocatives (이름+아/야, 씨, 님, 선배, 오빠…) and self-reference words against the matrix | [SCRIPTS]: 46% of Korean errors involve address, honorific and kinship terms | Kinship terms used for non-kin (어머니, 이모, 형님) need a glossary |
| L4 | 그/그녀/그들 density and use for non-focal characters | Count per 1,000 characters vs the author's baseline; flag 그녀 when the name appears in the same paragraph | [안소진]; [이상원]; pronoun rate as a translationese feature [Valentini] | Per-author setting (some literary authors use 그 deliberately) |
| L5 | Repeated explicit subject | Consecutive sentences opening with the same noun phrase + 은/는/이/가 (the same topic continuing) | [Roh & Lee]: 74% of easily inferred continuing topics are omitted | Narration and dialogue get different thresholds |
| L6 | 번역투 patterns | Regex/morphology rules for 에 의해(서), 로부터, 을/를 통해(서), 에 대해(서)/에 관해, 가지고 있다 (with attributes), -되어지다/-어지다 passives, 에서의/으로의/와의, -들 on abstract nouns, …기에는 너무…; each with a suggested rewrite | [김순영 2012] | Report density, not just hits; some uses are legitimate |
| L7 | Comma habits | Connective ending + comma; sentence-initial conjunctive adverb + comma; commas per sentence vs baseline | [KatFishNet]: 19.83% vs 4.10% commas after connective endings (essays) | Calibrate on Korean fiction, not essays |
| L8 | Monotonous endings and rhythm | Runs of 3 or more identical final endings; entropy of the ending distribution per scene; variance of sentence length; repeated POS templates | [Park et al. 2026]; lower POS diversity [KatFishNet] | Targets can be fed back into the prompt |
| L9 | Tense drift | Share of past vs present finals in each paragraph vs the declared base tense; flag isolated switches | [ConStory]: style errors are local | Allow a configured historical present |
| L10 | POV drift (heuristic) | First person: narrator named or referred to in the third person outside dialogue. Third-person limited: thought or feeling verbs (생각했다/느꼈다/알았다) attached to non-POV characters | [ConStory]: perspective confusions | Heuristic, so treat hits as warnings |
| L11 | Quotation rules | "…"고 → "…"라고; ‘…’라고 → ‘…’고; consistent curly vs straight quotes; inner thoughts in ‘ ’ | [국립국어원 2005] | The inner-thought rule is unverified here (see Q3 Gaps) |
| L12 | Foreign-script intrusion | Runs of Latin letters, Chinese characters, kana or Cyrillic not in the setting-book glossary | Language switching [Zhang et al.] | Cheap, high precision |
| L13 | Unknown names or wrong alias | Proper nouns not in the setting book; an alias attached to the wrong person | [ConStory]: name confusions; [NoLiMa] | Feeds the `<새_설정>` review |
| L14 | LLM-assisted contradiction check (not deterministic) | Compare the draft with the cards for timeline, knowledge, abilities, world rules, geography; **must quote the evidence span**; weight the middle 40–60% of long texts | [ConStory]; LLM checkers are unreliable as gates [FlawedFictions] | Output is advisory and needs author confirmation |

Reference: high-precision ending markers for L1/L2. These are standard school grammar, not from a fetched source; have a Korean linguist verify them before shipping.

| Level | High-precision sentence-final markers | Caveats |
|---|---|---|
| 하십시오체 | -(스)ㅂ니다, -(스)ㅂ니까, -(으)십시오, -(으)ㅂ시다 | Very reliable |
| 해요체 | Sentence-final -요 (-어요/-지요/-네요/-군요/-ㄹ게요/-ㄹ까요) | Watch for 요 used as a mid-sentence particle |
| 해체 (반말) | -어/-아, -야, -지, -냐, -니, -잖아, -거든, with no 요 | -네 overlaps with 하게체 |
| 해라체 | -(는)다, -(느)냐, -어라/-(으)라, -자 | Narration also ends in -다, so apply only inside dialogue |
| 하게체 / 하오체 | -네, -게, -나, -(으)세 / -(으)오, -소, -구려 | Common in 사극 and 무협; ambiguous with 해체; per-project setting |

Calibration: compute each metric's baseline from the author's existing chapters and from a reference set of published Korean fiction in the same genre (web novel vs literary). Flag deviations rather than absolute values. This follows the compare-against-human-distribution approach of [Valentini] and [Guo] (inference).

#### D. Pipeline
1. Plan: the author edits the scene card.
2. Generate in scene-sized chunks, re-injecting the cards each time.
3. Run the deterministic linter (L1–L13).
4. Run the LLM consistency check (L14), which must quote evidence.
5. Run a targeted revision prompt that quotes the flagged spans and the specific rule [Kunilovskaya].
6. The author accepts the text and approves updates to the setting book (summary, state, new names). Only author-verified summaries enter future context [FlawedFictions; RecurrentGPT].

### Gaps
- None of the Korean-specific rules (A–C) has been validated together on Claude. WriterProgram should build a small Korean test set to calibrate thresholds, run A/B tests on context format and on Korean vs English system prompts, and measure linter flag rates. The test set should include asymmetric speech levels, register switches, alias-heavy casts and long serial context.
- Attributing dialogue to speakers, which L1–L3 depend on, was not researched here and is non-trivial.
- I found no Korean benchmark for speech-level or subject-omission behavior in long LLM-generated fiction. Those are the two areas where this design rests most on linguistics rather than LLM evaluations.

### Source table (all sources opened and cited above)

| # | Title | Year | Venue | Lang | URL | One-line finding |
|---|---|---|---|---|---|---|
| 1 | LoCar: Localization-Aware Evaluation of In-Vehicle Assistants through Fine-Grained Sociolinguistic Control (Jeong et al.) | 2026 | ACL 2026 Industry (arXiv) | EN | https://arxiv.org/abs/2605.21086 | 11 LLMs fail to hold fixed speech levels and conflate 해요체/하십시오체; a rule-based suffix check lifts judge–human agreement 0.69 → 0.94 |
| 2 | Investigating the Automatic Translation of Korean Honorifics (Cihlar et al.) | 2026 | 2nd Workshop on Identity-Aware AI (ELRA) | EN | https://aclanthology.org/2026.iaai-1.3/ | Translation-tuned LLMs overuse polite forms vs humans; knowing the addressee makes models more casual |
| 3 | Are they lovers or friends? Evaluating LLMs' Social Reasoning in English and Korean Dialogues (Kim et al.) | 2025 | ACL 2026 (arXiv) | EN | https://arxiv.org/abs/2510.19028 | Relationship inference 75–80% in English vs 58–69% in Korean; 46% of Korean errors come from address, honorific and kinship terms |
| 4 | Evaluating Large Language Models on Understanding Korean Indirect Speech Acts (Koo et al.) | 2025 | arXiv | EN | https://arxiv.org/abs/2502.10995 | Best 71.94% MCQ (Claude 3 Opus); most models worse on indirect than direct speech acts; none at human level |
| 5 | Are large language models affected by politeness? Focusing on request speech acts in Korean (Jung et al.) | 2024 | PACLIC 38 | EN | https://aclanthology.org/2024.paclic-1.86 | Politeness of a Korean prompt shifts GPT-4/CLOVA X accuracy, explanation rate and length |
| 6 | An Investigation of Translationese in the Generations of Multilingual LLMs (Valentini et al.) | 2026 | COLM 2026 (arXiv) | EN | https://arxiv.org/html/2608.17399v1 | LLM text resembles translationese (60.2% of German Gemini output classified as translated); mostly distributional |
| 7 | Do Large Language Models Have an English Accent? (Guo et al.) | 2024 | ACL 2025 (arXiv) | EN | https://arxiv.org/html/2410.15956v3 | English prompts degrade Chinese naturalness; Llama-3.1's Chinese syntax resembles English; DPO against back-translations helps modestly |
| 8 | Is Translation All You Need? A Study on Solving Multilingual Tasks with LLMs (Liu et al.) | 2024 | NAACL 2025 | EN | https://arxiv.org/html/2403.10258v3 | Native prompts better for culture-related tasks; translating to English helps English-centric models on NLP tasks |
| 9 | Do Multilingual LLMs Think In English? (Schut et al.) | 2025 | arXiv | EN | https://arxiv.org/abs/2502.15603 | Key decisions are made in an English-like latent space; content words are routed through English |
| 10 | Lost in Literalism: How Supervised Training Shapes Translationese in LLMs (Li et al.) | 2025 | ACL 2025 | EN | https://arxiv.org/abs/2503.04369 | Translationese stems from supervised fine-tuning; polishing and filtering training data reduce it |
| 11 | Mitigating Translationese with GPT-4: Strategies and Performance (Kunilovskaya et al.) | 2024 | EAMT 2024 | EN | https://aclanthology.org/2024.eamt-1.35/ | Specific linguistic instructions beat relying on the model's knowledge for de-translationese rewriting |
| 12 | Cross-Lingual Prompt Steerability (Zhang et al.) | 2025 | arXiv | EN | https://arxiv.org/abs/2512.02841 | CoT, emotion and scenario prompt components give robust multilingual behavior; optimization +5–10%; less language switching |
| 13 | KatFishNet: Detecting LLM-Generated Korean Text through Linguistic Feature Analysis (Park et al.) | 2025 | ACL 2025 | EN | https://arxiv.org/html/2503.00032v4 | LLM Korean overuses commas (especially after connective endings), has lower POS diversity and strict spacing |
| 14 | Detecting and Guiding LLM-Generated Korean Poetry with Interpretable Form-level Features (Park, Yu, Kim) | 2026 | Findings of EMNLP 2026 (arXiv) | EN | https://arxiv.org/abs/2608.28986 | Ending diversity and length irregularity detect LLM poems (AUC 83.6); numeric targets steer output toward human statistics |
| 15 | 나는 왜 '그녀'를 꺼리는가 (이상원) | 2012 | 통역과 번역 14(1) | KO | https://www.earticle.net/Article/A178001 | 그녀 is a translation-born stylistic device; "she" ≠ 그녀 automatically |
| 16 | 소위 3인칭 대명사 '그, 그녀'의 기능에 대하여 (안소진) | 2008 | 한국어학 38 | KO | https://scholarworks.sookmyung.ac.kr/item/5715ab4b-0718-425e-8b92-c6efbf13dfed | 그/그녀 are written-only, mainly protagonist focus markers in short stories; writers usually repeat nouns |
| 17 | 영한 번역에 나타난 번역투 문장 (김순영) | 2012 | 새국어생활 22(1), 국립국어원 | KO | https://www.korean.go.kr/nkview/nklife/2012_1/22_0105.pdf | 번역투 taxonomy with before/after fixes: 가지다, 에 의해, passives, -들, 로부터, 을 통해, 에서의 |
| 18 | 문장 부호의 통일이 필요하다 (한규희) | 2005 | 새 국어 소식, 국립국어원 | KO | https://www.korean.go.kr/nkview/nknews/200511/88_9.html | Use "…"라고 after double quotes and ‘…’고 after single quotes |
| 19 | An Empirical Study for Generating Zero Pronoun in Korean based on Cost-based Centering Model (Roh & Lee) | 2003 | ALTA 2003 | EN | https://aclanthology.org/U03-1004/ | Korean omits redundant nouns; 74% of easily inferred continuing topics are omitted; restate at topic shifts |
| 20 | A Survey on Zero Pronoun Translation (Wang et al.) | 2023 | ACL 2023 | EN | https://arxiv.org/abs/2305.10196 | Finding the antecedent is the core difficulty; general metrics are unreliable; gender-bias risk |
| 21 | How Much Do LLMs Know About Chinese Zero Pronouns? (Li, Chen, He) | 2026 | arXiv | EN | https://arxiv.org/abs/2605.31056 | Zero pronouns remain highly challenging; SOTA LLMs translate fewer than half of Chinese zero pronouns correctly |
| 22 | Re3: Generating Longer Stories With Recursive Reprompting and Revision (Yang et al.) | 2022 | EMNLP 2022 | EN | https://arxiv.org/abs/2210.06774 | Plan + state re-injection + rerank/edit: +14% coherent plots, +20% premise relevance |
| 23 | DOC: Improving Long Story Coherence With Detailed Outline Control (Yang et al.) | 2023 | ACL 2023 | EN | https://arxiv.org/abs/2212.10077 | Detailed hierarchical outlines: +22.5% coherence and +28.2% outline relevance vs Re3 |
| 24 | Co-Writing Screenplays and Theatre Scripts with Language Models (Dramatron; Mirowski et al.) | 2022 | arXiv (cited as CHI 2023) | EN | https://arxiv.org/abs/2209.14958 | Hierarchical prompt chaining (title, characters, beats, locations, dialogue); study with 15 professionals |
| 25 | Agents' Room: Narrative Generation through Multi-step Collaboration (Huot et al.) | 2025 | ICLR 2025 | EN | https://arxiv.org/abs/2410.02603 | Planning agents write a labeled scratchpad; beats end-to-end baselines in human evaluation |
| 26 | StoryWriter: A Multi-Agent Framework for Long Story Generation (Xia et al.) | 2025 | arXiv | EN | https://arxiv.org/abs/2506.16445 | Event-based outline, chapter planning and event-relevant history compression; LongStory has about 6k stories |
| 27 | RecurrentGPT: Interactive Generation of (Arbitrarily) Long Text (Zhou et al.) | 2023 | arXiv | EN | https://arxiv.org/abs/2305.13304 | Human-editable natural-language long/short-term memory for arbitrarily long text |
| 28 | SCORE: Story Coherence and Retrieval Enhancement for AI Narratives (Yi et al.) | 2025 | arXiv | EN | https://arxiv.org/abs/2503.23512 | Item-state tracking + episode summaries + retrieval improve consistency (abstract-level claim) |
| 29 | Lost in Stories: Consistency Bugs in Long Story Generation by LLMs (ConStory-Bench; Li et al.) | 2026 | Findings of ACL 2026 (arXiv) | EN | https://arxiv.org/abs/2603.05890 | 19-subtype taxonomy; errors grow with length and cluster at 40–60%; factual and timeline errors most common |
| 30 | One Thousand and One Pairs: A "novel" challenge for long-context language models (NoCha; Karpinska et al.) | 2024 | EMNLP 2024 | EN | https://arxiv.org/abs/2406.16264 | Best model 55.8% on book-level claims; worse on world-building-heavy fiction |
| 31 | Finding Flawed Fictions: Evaluating Complex Reasoning in LMs via Plot Hole Detection (Ahuja et al.) | 2025 | COLM 2025 (arXiv) | EN | https://arxiv.org/abs/2504.11900 | Plot-hole detection degrades with length; LLM summaries/generation add 50%/100% more plot holes |
| 32 | NoLiMa: Long-Context Evaluation Beyond Literal Matching (Modarressi et al.) | 2025 | ICML 2025 | EN | https://arxiv.org/abs/2502.05167 | Without literal word overlap, 11 of 13 models fall below 50% of baseline at 32K; GPT-4o 99.3% → 69.7% |
| 33 | Lost in the Middle: How Language Models Use Long Contexts (Liu et al.) | 2023 | TACL | EN | https://arxiv.org/abs/2307.03172 | Information in the middle of a long context is used worst |
| 34 | Does Prompt Formatting Have Any Impact on LLM Performance? (He et al.) | 2024 | arXiv | EN | https://arxiv.org/abs/2411.10541 | Format (plain/Markdown/JSON/YAML) shifts results up to 40% for GPT-3.5; GPT-4 more robust |
| 35 | DramaBench: A Six-Dimensional Evaluation Framework for Drama Script Continuation (Ma et al.) | 2025 | arXiv | EN | https://arxiv.org/abs/2512.19012 | Character consistency is an independent quality dimension (mean \|r\| = 0.020); hybrid rule + LLM evaluation |
| 36 | Claude prompting best practices, "Long context prompting" (Anthropic) | 2026 (accessed) | Vendor documentation | EN | https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices | Long documents at the top, XML-tagged documents, quote relevant parts first |

Unverified leads (found in search, not opened; do not cite without checking):
- "Context-Aware NMT for Korean Honorific Expressions" (*Electronics*)
- KU×Upstage IWSLT 2023 formality control (English→Korean)
- KBS / Ko-EQ-Bench honorific usage (arXiv 2610.21227)
- Translationese-index (EMNLP 2025, arXiv 2507.12260)
- SEACrowd translationese classifier (arXiv 2406.10118)
- "The Narrative Tense in the Korean Novel" (*Korean Studies* 5, 1981)
- 국립국어원 rule on 작은따옴표 for 마음속으로 한 말
- Fiction.LiveBench
