# Korean literary vocabulary and lexical richness: features, LLM behaviour, measurement, and injectable resources (vocabulary layer)

*Scope: the vocabulary layer of WriterProgram's Korean fiction prompt, skill and style linter (고유어, 의성어·의태어, 감각어·색채어, emotion words, adverbs and endings). General prompting, figurative language, Korean story-generation papers, discourse and coherence, and practitioner tips are covered in sibling notes.*

*Verification legend:*
- **Unmarked:** I opened the primary page or PDF and read the claim there.
- **"(abstract via OpenAlex/Semantic Scholar API)":** I read the abstract through a scholarly metadata API because the publisher page was blocked.
- **"(bibliographic only)":** I confirmed the title, venue and year but could not read the content.
- **"(not opened)":** seen only in search snippets. These are kept out of Cited Findings and listed in Gaps.
- **"→ Implication":** every line marked this way is my inference for the prompt or the linter, not a claim made by the source.

All web pages were accessed 2026-10-17.

---

## 1. What makes Korean prose lexically "literary"? (고유어/한자어/외래어 balance, ideophones, sensory and colour gradations, emotion words, adverbs and endings, fiction-corpus studies)

### Takeaway
The dictionary as a whole is mostly Sino-Korean (57% of headwords), but almost all verbs (97%), adverbs (94%), particles and endings (100%) are native words. Most of Korean's native texture is therefore carried by predicates, adverbs (especially ideophones) and endings, not by rare nouns. Ideophones, colour terms and other sensory terms vary in systematic ways:
- vowel brightness (양성/음성), for size and lightness;
- consonant intensity (평음/경음/격음);
- reduplication;
- degree affixes such as 샛-/싯-, -스름하다, -께하다 and -죽죽하다.

These words give vividness, but they are tightly bound to collocations and register. Literary use is a matter of choosing precisely and using them sparingly, not of using many of them.

### Cited Findings

**Word-origin balance (어종) and where native words live**
- **Headword totals.** 표준국어대사전 (1999) has 440,262 main headwords. Their origins break down as follows ([정호성 2000, 「표준국어대사전 수록 정보의 통계적 분석」, 새국어생활 10(1)](https://www.korean.go.kr/nkview/nklife/2000_1/2000_0104.pdf), Table 6):

  | Origin | Headwords | Share |
  |---|---|---|
  | 고유어 (native) | 111,299 | 25.28% |
  | 한자어 (Sino-Korean) | 251,478 | 57.12% |
  | 외래어 (loanword) | 23,196 | 5.26% |
  | 한자어 + 고유어 hybrid | 36,461 | 8.28% |
  | 한자어 + 외래어 hybrid | 15,548 | 3.53% |
  | 외래어 + 고유어 hybrid | 1,331 | 0.30% |
  | All three | 751 | 0.17% |

  - → Implication: "Korean is 60–70% Sino-Korean" is a fact about the noun-heavy dictionary. It is not a target for prose. Do not set a global 고유어 quota.
- **The split by part of speech reverses the picture** (my percentages, computed from the counts in [정호성 2000](https://www.korean.go.kr/nkview/nklife/2000_1/2000_0104.pdf)):

  | Part of speech | Entries | Native | Note |
  |---|---|---|---|
  | Nouns | 333,226 | 69,888 (≈21%) | 205,229 Sino-Korean (≈62%) |
  | Verbs | 15,135 | 14,701 (≈97%) | |
  | Adjectives | 6,438 | 5,145 (≈80%) | plus 1,266 Sino+native hybrids |
  | Adverbs | 14,076 | 13,268 (≈94%) | |
  | Particles | 356 | all | |
  | Endings | 2,523 | all | |

  - → Implication: native lexical richness is concentrated in predicates, adverbs and endings, which are exactly the slots where an LLM's word choice gives "literary" texture.
  - → Implication: a token-level 고유어 ratio is inflated by particles and endings and is nearly useless. Measure variety per part of speech instead (see §3 and §6).
- **Derivational suffixes.**
  - -거리다 forms 3,200 sub-headwords, 3,193 of them on native bases. -대다 forms 3,193, 3,186 of them native. 강인선 (1993, below) shows that most -거리다 verbs in a corpus are built on ideophone roots.
  - -하다 and -되다 combine far more often with Sino-Korean bases.
  - -적 attaches to Sino-Korean bases in 1,154 of its 1,156 forms.
  - Source: [정호성 2000](https://www.korean.go.kr/nkview/nklife/2000_1/2000_0104.pdf).
  - → Implication: ideophone-derived verbs (-거리다/-대다/-이다) are a large native-verb reservoir for the word bank.
  - → Implication: the density of -적 and Sino-Korean -하다 verbs in narration is a cheap lint proxy for abstract, report-like register.
- **Usage frequency (국립국어연구원, 『현대 국어 사용 빈도 조사』, 2002).**
  - 1,484,463 general-word tokens and 58,437 types.
  - Top lemmas: 것 (bound noun) 25,567, 하다 22,064, 있다 (auxiliary) 18,553, 있다 (adjective) 18,202, 되다 11,506.
  - Top particles: 의, 을, 에, 이, 이다. Top endings: 다, ㄴ, 는, 고, 었.
  - Source: [조남호 2003, 「우리말에는 어떤 말이 많이 쓰일까」, 새국어소식](https://www.korean.go.kr/nkview/nknews/200302/55_3.html).
  - → Implication: this provides frequency ranks for a rarity or sophistication score.
  - → Implication: 것/하다/있다/되다 must be stop-lemmas in repetition checks.
  - → Implication: a TTR of about 3.9% at 1.48M tokens shows why raw TTR cannot compare texts of different length.
- **Graded learner vocabulary list (국립국어연구원, May 2003).**
  - 5,965 words, graded A (982), B (2,111) and C (2,872), each with its rank in the 2002 frequency survey.
  - Distributed as Excel, text and HWP under 공공누리 (KOGL) Type 1.
  - Source: [국립국어원, 한국어 학습용 어휘 목록](https://www.korean.go.kr/front_eng/down/down_02V.do?etc_seq=70).
  - → Implication: being outside this list, or in grade C, is a usable proxy for "beyond-basic" vocabulary. It can drive a lexical-sophistication metric and set different targets for web novels and literary fiction.

**Ideophones (의성어·의태어, also called 흉내말, 상징어, 상징 부사)**
- **A native stronghold.** 채완 describes 의성어·의태어 as almost the only class where native words hold their ground in a vocabulary full of Sino-Korean ("한자어투성이의 어휘 체계 내에서 거의 유일하게 고유어가 영역을 지키고 있는 부류"). The class:
  - still preserves vowel harmony, which is otherwise collapsing;
  - can change its form through vowel and consonant alternation without changing its basic meaning.

  Source: [채완 1993, 「의성어·의태어의 통사와 의미」, 새국어생활 3(2)](https://www.korean.go.kr/nkview/nklife/1993_2/3_3.html).
  - → Implication: ideophones are the main native-vocabulary lever a prompt can pull to make prose feel Korean.
- **Classical alternation laws** (정인승 1938, summarised by 채완):
  - The "모음 상대 법칙" (vowel-contrast law) gives a big feel (덜렁) versus a small feel (달랑).
  - The "자음 가세 법칙" (consonant-intensification law) gives plain (감감), strong (깜깜) and aspirated (캄캄) feels.

  유창돈 (1980), also cited there, sorts 상징 부사 by sense:
  - visual: 움직움직;
  - auditory: 땅땅;
  - tactile: 매끈매끈;
  - psychological (심각적): 답답.

  Source: [채완 1993](https://www.korean.go.kr/nkview/nklife/1993_2/3_3.html).
  - → Implication: tag every ideophone in the word bank with vowel grade, consonant grade and sense modality, so the prompt can ask for selection by size, weight and intensity.
- **Single versus reduplicated forms are not interchangeable.** Reduplication adds plurality, continuity or repetition (꾹 누른다 versus 꾹꾹 누른다), but some contexts accept only one form:
  - 병이 {덜컥, \*덜컥덜컥} 났다
  - 울지 말고 {뚝, \*뚝뚝} 그쳐
  - 햇볕이 {?쨍, 쨍쨍} 내리쬔다
  - {쨍, \*쨍쨍} 하고 해 뜰 날

  Source: [채완 1993](https://www.korean.go.kr/nkview/nklife/1993_2/3_3.html).
  - → Implication: a curated (ideophone form × collocate) constraint list for the linter.
  - → Implication: a prompt rule: single form for sudden or one-off events, reduplicated form for continuous or repeated ones.
- **Alternants as separate words; vowel classes** (박창원, using 박용수 1989 『우리말 갈래 사전』 as data).
  - Consonant alternants (싱긋/씽긋) and vowel alternants (싱긋/생긋) are separate words that differ in 어감.
  - /아, 애, 오/ are 양성. /어, 에/ are 음성. /이, 으, 우/ are 음성 vowels with partly neutral behaviour.
  - Of about 640 ideophones with ㅏ in the first syllable (excluding about 50 monosyllabic reduplications), 331, about half, continue with a 양성 vowel.
  - Source: [박창원 1993, 「현대 국어 의성 의태어의 형태와 음운」, 새국어생활 3(2)](https://www.korean.go.kr/nkview/nklife/1993_2/3_2.html).
  - → Implication: vowel harmony is a tendency with neutral vowels. "Harmony checks" must be driven by dictionary lookup, never generated by rule.
- **Onomatopoeic versus cross-modal ideophones.** In a written corpus of Korean ideophonic stems, onomatopoeic (sound-imitating) ideophones are skewed toward disharmonic vowel patterns. Cross-modal ideophones, which convey sensory imagery, conform more to vowel harmony. Source: [Kwon 2018, "Iconicity correlated with vowel harmony in Korean ideophones," *Laboratory Phonology* 9(1)](https://doi.org/10.5334/labphon.53) (abstract via OpenAlex API).
  - → Implication: size and brightness heuristics are more reliable for 의태어 than for 의성어. For 의성어, the word bank should simply list attested forms.
- **Meaning-bearing paradigms.** Korean ideophones contain paradigms of meaning-bearing elements:
  - by consonant strength: pɛŋpɛŋ/pʰɛŋpʰɛŋ (뱅뱅/팽팽), "neutral vs. stronger and more violent motion of circling";
  - by vowel quality: piŋkɨl/pɛŋkɨl (빙글/뱅글), "twirling of a bigger/small object".

  Source: [Kwon 2016, "Canonical approach to phonaesthemes in Korean ideophones," NINJAL International Symposium abstract](https://www2.ninjal.ac.jp/past-events/mimetics/Kwon_2016_NINJAL_abst.pdf).
  - → Implication: ready-made few-shot contrast pairs for the prompt's ideophone rule.
- **Inventory size and the -거리다 verbs** ([강인선 1993, 「일본어의 의성 의태어」, 새국어생활 3(2)](https://www.korean.go.kr/nkview/nklife/1993_2/3_7.html)):
  - Korean ideophone dictionaries hold:
    - 3,863 words (박용수 1989);
    - 3,780 words, or 2,416 when alternants are grouped (조선어연구회 1971, per 휜들링그 1987);
    - 8,286 words (연변 언어 연구소 1982).
  - Grouping the alternants of a 6,600-entry dictionary gives 1,109 Korean ideophone groups, against 617 for Japanese, about 1.7 times as many.
  - In NIKL's 2-million-eojeol data, about 260 -거리다 verbs were found. Their roots are 의성어 (36), 의태어 (162), 의성의태어 (53) and psychological or manner roots (6).
  - Korean's three-way lenis/tense/aspirated contrast (감감/깜깜/캄캄) and its vowel alternation (보글보글/부글부글, 빙빙/뱅뱅) both create nuance differences.
  - → Implication: include derived verbs (-거리다/-대다/-이다/-하다), not only adverbs, in the word bank.
- **The dictionary encodes intensity relations in its definitions.** 표준국어대사전 defines 깡충깡충 as "짧은 다리를 모으고 자꾸 힘 있게 솟구쳐 뛰는 모양" and adds that it "gives a stronger and harsher feel than 강중강중" ("‘강중강중’보다 세고 거센 느낌을 준다"). The older spelling 깡총깡총, used in the 1993 articles, is marked as a non-standard variant pointing to 깡충깡충. Sources: [표준국어대사전: 깡충깡충](https://stdict.korean.go.kr/search/searchResult.do?searchKeyword=%EA%B9%A1%EC%B6%A9%EA%B9%A1%EC%B6%A9) and [깡총깡총](https://stdict.korean.go.kr/search/searchResult.do?searchKeyword=%EA%B9%A1%EC%B4%9D%EA%B9%A1%EC%B4%9D).
  - → Implication: grade relations (stronger or weaker counterpart) can be harvested from the dictionary to build ideophone families.
  - → Implication: forms copied from older literature may now be non-standard, so the linter must validate spellings against the current dictionary.
- **Rhetorical function and its limits** ([성기옥 1993, 「의성어 의태어의 시적 위상과 기능」, 새국어생활 3(2)](https://www.korean.go.kr/nkview/nklife/1993_2/3_6.html)):
  - The effect of ideophones depends on concretisation (具象化). They give vitality and a live sense of presence, as in "토끼는 깡총깡총, 노루는 껑충껑충".
  - Because they are intrinsically sensory, they carry stronger expressive constraints than other words.
  - Their poetic value is realised in combination (찰랑찰랑 with 물결, 햇빛, 희망 or 꿈), not in selection alone.
  - → Implication: a prompt rule to use ideophones only at sensory focal points and anchor them to a fitting subject or verb.
  - → Implication: density caps in the linter.
- **어감, not meaning, and register** ([윤희원 2003, 「의성어·의태어의 개념과 정의」, 새국어생활 13(4)](https://www.korean.go.kr/nkview/nklife/1993_2/3_1.html); the URL path says 1993 but the page header reads 제13권 제4호, 2003년 겨울):
  - The article quotes the high-school grammar textbook: vowel and consonant differences can change only a word's 어감 while keeping its basic meaning. This happens mainly in onomatopoeic and mimetic adverbs and is called "one of the characteristics of our language".
  - Ideophones pervade everyday and child-directed language: 멍멍, 아장아장, 성큼성큼, 냠냠, 콕.
  - → Implication: tag register (유아어, 구어, 문어) in the word bank so that literary narration does not slip into nursery register. Web-novel dialogue may use that register deliberately.

**Colour and sensory gradations (색채어·감각어)**
- **Two axes in the dictionary definitions.** 표준국어대사전 definitions encode one axis of brightness and turbidity, carried by the vowel, and one axis of degree, carried by affixes:

  | Word | 표준국어대사전 definition | Gloss |
  |---|---|---|
  | 노르다 | 달걀노른자의 빛깔과 같이 밝고 선명하다 | bright and vivid, like egg yolk |
  | 누르다(형용사) | 황금이나 놋쇠의 빛깔과 같이 다소 밝고 탁하다 | somewhat bright and turbid, like gold or brass |
  | 노랗다 | 병아리나 개나리꽃과 같이 밝고 선명하게 노르다 | bright and vivid, like a chick or forsythia |
  | 누렇다 | 익은 벼와 같이 다소 탁하고 어둡게 누르다 | somewhat turbid and dark, like ripe rice |
  | 샛노랗다 | 매우 노랗다 | very 노랗다 |
  | 싯누렇다 | 매우 누렇다 | very 누렇다 |
  | 새빨갛다 | 매우 빨갛다 | very 빨갛다 |
  | 시뻘겋다 | 매우 뻘겋다 | very 뻘겋다 |
  | 노르스름하다 | 조금 노르다 (≒노르스레하다, 노름하다, 노릇하다) | slightly 노르다 |
  | 누르스름하다 | 조금 누르다 (≒누르스레하다, 누름하다, 누릇하다) | slightly 누르다 |
  | 노르께하다 | 곱지도 짙지도 않게 노르다 (≒노리끼리하다) | yellow, neither fine nor deep |
  | 불그스름하다 | 조금 붉다 | slightly red |
  | 불그죽죽하다 | 칙칙하고 고르지 아니하게 불그스름하다 | dull and unevenly reddish |

  Sources: [표준국어대사전 search: 누렇다](https://stdict.korean.go.kr/search/searchResult.do?searchKeyword=%EB%88%84%EB%A0%87%EB%8B%A4) and sibling entries on the same search endpoint ([노랗다](https://stdict.korean.go.kr/search/searchResult.do?searchKeyword=%EB%85%B8%EB%9E%97%EB%8B%A4), [노르께하다](https://stdict.korean.go.kr/search/searchResult.do?searchKeyword=%EB%85%B8%EB%A5%B4%EA%BB%98%ED%95%98%EB%8B%A4), [불그죽죽하다](https://stdict.korean.go.kr/search/searchResult.do?searchKeyword=%EB%B6%88%EA%B7%B8%EC%A3%BD%EC%A3%BD%ED%95%98%EB%8B%A4)).
  - → Implication: store colour terms as families with features: hue; vowel (ㅗ/ㅏ bright and clear versus ㅜ/ㅓ turbid and dark); degree (샛-/싯-/새-/시- intensifier, -스름하다 slight, -께하다 muted); and affect (-죽죽하다 dull or uneven). The prompt should prefer one precise graded adjective over stacking "노란색의" with an adverb.
- **Colour families for seeding.** 권재일 lists the 붉다 family (불그레하다, 불그스름하다, 불그죽죽하다, 불그무레하다, 불긋불긋하다) and the 빨갛다 family (새빨갛다, 뻘겋다, 시뻘겋다). He calls Korean colour vocabulary "아주 다채롭고 아름답다". Source: [권재일, column reprinted in 국립국어원 『새 국어 소식』 (orig. 한겨레 2004-11-02)](https://www.korean.go.kr/nkview/nknews/200501/78_11.html).
  - → Implication: seed families for the colour section of the word bank.

**Emotion words**
- **A list of 434 Korean emotion words** ([박인조·민경환 2005, 「한국어 감정단어의 목록과 정서 차원 탐색」, 한국심리학회지: 사회 및 성격 19(1), 109–129](https://www.accesson.kr/ksppa/v.19/1/109/14556)):
  - About 10 emotion researchers selected the 434 words from Yonsei's 『현대 한국어의 어휘빈도』.
  - 365 undergraduates rated prototypicality, familiarity, pleasantness and activation.
  - MDS on 87 representative words gave pleasantness–unpleasantness as dimension 1 and a tentative "other-focused vs. self-focused" dimension 2.
  - No activation dimension emerged, unlike in English studies.
  - → Implication: a ready-made list of direct emotion-naming words for a show-don't-tell lint.
  - → Implication: emotion tags in the word bank by valence × self/other focus.

**Fiction-corpus and stylometric studies**
- **Authorship features in early modern novels** ([김일환·이도길 2015, 「저자 판별을 위한 전산 문체론 – 초기 현대소설을 대상으로」, 국어국문학 170: 207–240, DOI 10.17291/kolali.2015..170.007](https://scholar.korea.ac.kr/handle/2021.sw.korea/133037?mode=full)):
  - Corpus: 70 early modern Korean novels by 14 authors.
  - Features: sentence length; lexical categories (common nouns, verbs, adjectives, adverbs); function categories (particles, endings, auxiliary predicates).
  - Every feature reveals style, but function categories are more useful for authorship attribution because they are closed-class and less deliberately chosen.
  - → Implication: for a "preserve the author's voice" check, compare ending, particle and auxiliary profiles against the author's own chapters. For "richness", monitor content-word categories.

### Inferences
- **Five components of literary vocabulary.** In Korean these are:
  1. precise native predicates and adverbs;
  2. ideophones and ideophone-derived verbs, used sparingly and in the correct grade;
  3. graded sensory and colour adjectives;
  4. varied endings;
  5. restraint in abstract Sino-Korean nominal style in narration.

  These rest on the part-of-speech distribution, the ideophone literature, and the LLM nominal bias in §2.
- **Rare nouns are a weak marker of literariness.** The bigger quality risks are "rare words" that are mis-collocated or in the wrong register, and the wrong grade of a gradable form (덜렁 for a small object, 누렇다 for a fresh, bright flower).
- **Retrieve contrast sets, not isolated words.** Ideophone and colour gradations are productive and paradigmatic, so retrieval should return a whole set such as 노르스름하다/누르스름하다/노르께하다/샛노랗다/싯누렇다 or 뱅글/빙글. The model then chooses within a set of semantic contrasts rather than inserting an isolated "fancy word".
- **Web novels and literary fiction probably need different settings,** for example ideophone density and tolerance for rare words. This is an inference; I found no Korean study comparing the two genres' vocabulary.

### Gaps
- **Fiction-specific word-origin ratios.** I found no verified token-level 고유어/한자어/외래어 ratio for contemporary Korean fiction, or for fiction versus other genres. The 2002 frequency-survey article I opened gives no word-origin breakdown.
- **Ideophone frequency in novels.** I found no verified corpus count of ideophone frequency or density in contemporary novels or web novels, so ideophone density thresholds must be calibrated on WriterProgram's own human corpus.
- **No open machine-readable ideophone dictionary or classified thesaurus (분류 어휘 사전).** Print sources such as 박용수 (1989) 『우리말 갈래 사전』 (the data of 박창원 1993) and 『조선말 의성 의태어 분류 사전』 (cited by 채완 1993) are copyrighted print works.
- **Colour-space study (not opened).** An Ewha study reportedly quantified colour-suffix effects in colour space:
  - -앟- was reported as brighter and more saturated than -엏-;
  - -스름하다 had no specific perceived colour;
  - -죽죽하다 lowered lightness and saturation.

  The repository returned 503/403 errors, so this is unverified.
- **Prefix rule.** I did not verify a normative statement of the 샛-/싯-/새-/시- allomorph rule. Only the dictionary examples above are verified, so the linter should rely on dictionary existence checks rather than an encoded rule.
- **Other senses.** Sensory vocabulary beyond colour and ideophones (taste, smell and touch terms such as 시큼하다/새큼하다 or 매끈하다/미끈하다) was not researched in depth.

---

## 2. How well do LLMs handle this vocabulary, compared with human writers?

### Takeaway
Korean benchmarks show that rare native vocabulary, morphology and phonology are among LLMs' weakest areas:
- HAE-RAE Rare Words: LLaMA-2-13B scored 32% zero-shot.
- KoBALT (2025): the best model, Claude 3.7 Sonnet, scored only 36% on morphology and 31% on phonology.

Compared with human Korean text, LLM output is:
- more nominal;
- heavier in commas;
- poorer in endings and predicates;
- narrower in vocabulary growth.

Its lexical diversity is lower than English output and only weakly raised by temperature. Multimodal LLMs show some latent sound-symbolic intuition, including on Korean mimetic words. However, no study evaluates whether LLMs choose Korean ideophones, colour or sensory terms correctly in fiction.

### Cited Findings

**Knowledge benchmarks (rare, native, loan and standardised words; grammar)**
- **HAE-RAE Bench task design** ([Son et al., "HAE-RAE Bench: Evaluation of Korean Knowledge in Language Models," arXiv 2309.02706; LREC-COLING 2024, ACL Anthology 2024.lrec-main.704](https://arxiv.org/abs/2309.02706)):

  | Task | Questions | Source of items |
  |---|---|---|
  | Rare Words (RW) | 405 | Definition-to-word quizzes from KBS 「우리말 겨루기」 |
  | Loan Words (LW) | 169 | Foreign word to NIKL Korean equivalent; kept only if the term appears in both Naver and Daum encyclopedias |
  | Standard Nomenclature (SN) | 153 | NIKL standardised domain terms |

  All three use five-option multiple choice.
  - Tokenizer fertility on RW is 3.1 sub-tokens per word for Polyglot-Ko versus 6.1 for Llama-2.
  - Korean is about 0.06% of Llama-2's training corpus.
  - → Implication: rare native vocabulary is precisely where non-Korean-centric pretraining is thinnest. Never trust a model's unaided definition of a rare word; attach dictionary definitions when suggesting one.
- **HAE-RAE results.** Accuracy by number of in-context examples (log-likelihood scoring), from [Son et al.](https://arxiv.org/abs/2309.02706):

  | Model | Task | 0-shot | 5-shot | 10-shot |
  |---|---|---|---|---|
  | Polyglot-Ko-12.8B | Rare Words | 53.09 | 75.31 | 76.05 |
  | LLaMA-2-13B | Rare Words | 32.35 | 42.47 | 43.95 |
  | UMT5-13B | Rare Words | 33.09 | 30.37 | 28.64 |
  | Polyglot-Ko-12.8B | Loan Words | 87.57 | 94.67 | 94.67 |
  | LLaMA-2-13B | Loan Words | 66.86 | 77.51 | 78.11 |

  Zero-shot generation with GPT models (June 2023 snapshot), on the whole HAE-RAE Bench versus the KoBEST suite:

  | Model | HAE-RAE, Korean prompt | HAE-RAE, English prompt | KoBEST, Korean prompt | KoBEST, English prompt |
  |---|---|---|---|---|
  | GPT-3.5 | 51.2 | 55.4 | 68.0 | 79.3 |
  | GPT-4 | 67.8 | 68.2 | 81.1 | 91.0 |

  I did not open the KoBEST paper itself.
  - → Implication: in-context exemplars raise rare-word performance a lot (+23 points for Polyglot-Ko). Retrieval that injects definitions and example sentences is likely to help choice accuracy.
  - → Implication: these are 2023-era models, so current Claude models need re-testing (see Gaps).
- **CLIcK** ([Kim et al., "CLIcK: A Benchmark Dataset of Cultural and Linguistic Intelligence in Korean," arXiv 2403.06412](https://arxiv.org/html/2403.06412v4), reported as LREC-COLING 2024):
  - 1,995 questions from official exams (CSAT, TOPIK, PSE, PSAT, the Korean History Exam, Kedu) and the KIIP textbook.
  - The language part covers Textual, Functional and Grammar categories; Grammar includes vocabulary, syntax and phonology/graphology.
  - Accuracy on the language categories:

    | Model | Textual | Functional | Grammar |
    |---|---|---|---|
    | Claude-2 | 55.86 | 32.88 | 43.95 |
    | GPT-3.5 | 53.19 | 32.62 | 38.85 |
    | Polyglot-Ko 12.8B | 22.96 | 19.84 | 23.05 |
    | KULLM 12.8B | 24.65 | 19.38 | 25.67 |

  - Open Korean models were near chance (about 25%).
  - → Implication: even then-frontier models were unreliable judges of Korean grammar and vocabulary. Do not let the generator "self-certify" rare forms; verify with a dictionary or morphological analyser.
- **KoBALT** ([Shin et al., "KoBALT: Korean Benchmark For Advanced Linguistic Tasks," arXiv 2505.16125, 2025](https://arxiv.org/html/2505.16125v1)):
  - 700 expert-written, 10-option questions across 24 phenomena.
  - Scores:

    | Model | Overall | Semantics | Morphology | Phonetics/phonology |
    |---|---|---|---|---|
    | Claude 3.7 Sonnet (best) | 0.61 | 0.66 | 0.36 | 0.31 |
    | Claude 3.5 Sonnet | 0.52 | | | |
    | GPT-4o | 0.44 | | | |
    | DeepSeek-V3 | 0.43 | | | |
    | Qwen2.5-72B | 0.37 | | | |
    | Llama-3.3-70B | 0.32 | | | |

  - Verbal conjugation was weak; the authors attribute this to irregular and suppletive forms and to subword tokenization.
  - The benchmark does not include sound symbolism or onomatopoeia as phenomena.
  - Its data are licensed KOGL BY-NC-ND.
  - → Implication: phonology and morphology are the machinery behind ideophone alternation, colour-term derivation and irregular conjugation, and they remain the weakest domains even for frontier Claude models.
  - → Implication: state sound-symbolic rules explicitly in the prompt or lexicon.
  - → Implication: the linter must check that rare predicates exist and are conjugated correctly.

**Human versus LLM Korean text (vocabulary, part of speech, punctuation)**
- **KatFish dataset and spacing** ([Park, Kim, Kim & Han, "KatFishNet: Detecting LLM-Generated Korean Text through Linguistic Feature Analysis," arXiv 2503.00032; ACL 2025, 2025.acl-long.1030](https://arxiv.org/html/2503.00032v4)):
  - 2,094 texts: 470 human-written and 1,624 generated by GPT-4o, Solar, Qwen2-72B and Llama-3.1-70B.
  - Genres: argumentative essays, free-verse poetry and paper abstracts.
  - LLMs follow spacing rules rigidly, whereas humans omit spaces for stylistic or poetic reasons.
  - Human essays have higher part-of-speech n-gram diversity; the exception is 4- and 5-grams in poetry.
  - → Implication: default LLM Korean is structurally more uniform. Measure part-of-speech n-gram diversity alongside lexical diversity.
- **KatFish comma usage**, human versus LLM ([KatFishNet](https://arxiv.org/html/2503.00032v4)):

  | Measure | Human | LLM |
  |---|---|---|
  | Essays: sentences containing a comma | 26.31% | 61.03% |
  | Essays: commas per morpheme | 1.13% | 2.56% |
  | Essays: commas after connective endings | 4.10% | 19.83% |
  | Abstracts: sentences containing a comma | 47.48% | 65.21% |

  - LLMs also add commas after conjunctive adverbs. The authors suggest this reflects English punctuation norms.
  - Comma features gave the best detector: on average 19.78% higher AUC-ROC than the best baseline.
  - → Implication: adopt these as lint metrics, especially commas after connective endings such as -고, -며 and -지만.
- **KatFish part of speech and vocabulary** ([KatFishNet](https://arxiv.org/html/2503.00032v4), Appendices P and Q):
  - With the HanNanum tagger, human text uses more endings and predicates, while LLM text uses more nominals.
  - Zipf and Heaps analyses show LLM text concentrating on frequent words, with slower growth of unique words as length increases. The authors' words: "LLMs tend to use a narrower range of vocabulary."
  - → Implication: monitor the noun-to-predicate ratio, ending diversity and vocabulary growth (Heaps' slope). This is the Korean-specific evidence for the "native predicate" prompt rule.
- **비립 (2025)** ([「초거대언어모델의 한·영·중 논술형 글쓰기 능력 평가 – 어휘 다양성 비교 중심으로」, 텍스트언어학 59: 155–188, DOI 10.22832/txtlng.2025.59..006](https://scholarworks.sookmyung.ac.kr/item/32a3b366-0a67-4ef0-b296-19d35a40eca8)):
  - Claude 3.5, Gemini 2.5, GPT-4.1 and Mistral 3.1 generated 7,200 essays in Korean, English and Chinese.
  - The essays were measured with MATTR, HD-D, vocd-D, MTLD and a revised TTR.
  - Lexical diversity differed significantly by model: Gemini 2.5 and GPT-4.1 varied most, and Mistral 3.1 was consistently lowest.
  - English showed the most lexical diversity. Korean and Chinese showed more constrained variation, which the author attributes to structural and morphological factors.
  - Temperature raised variation only modestly.
  - The author concludes that single metrics or single languages are insufficient.
  - → Implication: lint thresholds must be calibrated on Korean, not ported from English.
  - → Implication: raising temperature will not fix lexical poverty in Korean output.
- **Reviriego et al.** ([Reviriego et al., "Playing with words: Comparing the vocabulary and lexical diversity of ChatGPT and humans," *Machine Learning with Applications* 18 (2024) 100602, arXiv 2308.07462](https://arxiv.org/abs/2308.07462)): on English question-answering and paraphrase data, ChatGPT "tends to use fewer distinct words and lower lexical richness than humans". The authors call the results preliminary.
  - → Implication: expect default output to underuse vocabulary.
- **Kendro, Maloney & Jarvis** ([Kendro, Maloney & Jarvis, "Do LLMs produce texts with 'human-like' lexical diversity?," arXiv 2508.00086, 2025; related DOI 10.1111/ijal.70115](https://arxiv.org/abs/2508.00086)):
  - ChatGPT-3.5, -4, -o4-mini and -4.5 texts differed significantly from 240 L1 and L2 English writers on all six lexical-diversity dimensions (volume, abundance, variety-repetition, evenness, disparity, dispersion).
  - ChatGPT-4.5 showed higher diversity than older models despite producing fewer tokens. The authors conclude that newer models are *less* human-like.
  - → Implication: the lint target should be a human reference band, not "maximise diversity". Over-diverse, thesaurus-like prose is also non-human.
- **Guo, Shang & Clavel** ([Guo, Shang & Clavel, "Benchmarking Linguistic Diversity of Large Language Models," *TACL* 13 (2025), arXiv 2412.10271](https://arxiv.org/html/2412.10271v2)):
  - In English story generation (WritingPrompts), lexical diversity (Unique-n averaged over 1- to 3-grams) was 59.04 for humans versus 52.28–56.90 for six 7–12B instruction-tuned LLMs.
  - Higher temperature increases lexical diversity with only a minor loss of relevance.
  - Instruction-tuned models are more lexically diverse than their base models.
  - 4-bit quantization reduces diversity.
  - Making the prompt explicitly ask for creativity had minimal impact on diversity.
  - → Implication: "be creative/vivid" instructions are a weak lever. Concrete lexical guidance and decoding settings matter more.
- **Kobak et al.** ([Kobak, González-Márquez, Horvát & Lause, "Delving into LLM-assisted writing in biomedical publications through excess vocabulary," *Science Advances* 11(27), 2025, arXiv 2406.07016](https://arxiv.org/abs/2406.07016)):
  - "Excess word" analysis of more than 15 million PubMed abstracts shows abrupt increases in certain style words after LLMs appeared.
  - At least 13.5% of 2024 abstracts were LLM-processed, and up to 40% in some subcorpora.
  - → Implication: a reproducible method to build a Korean list of "AI-overused words and phrases": compare word frequencies in model outputs with a human Korean fiction baseline.
- **Antislop** ([Paech, Roush, Goldfeder & Shwartz-Ziv, "Antislop: A Comprehensive Framework for Identifying and Eliminating Repetitive Patterns in Language Models," arXiv 2510.15061](https://arxiv.org/abs/2510.15061); listed as ICLR 2026 on iclr.cc, a listing I did not open):
  - Some "slop" patterns appear more than 1,000 times more often in LLM output than in human text.
  - Profiling slop against human baselines is automatable.
  - → Implication: Korean LLM clichés can be measured and listed (see §5 for suppression results).

**Sound symbolism and ideophones in LLMs**
- **LEX-ICON** ([Jeong, Lee, Lee, Han & Yu, "Do Language Models Associate Sound with Meaning? A Multimodal Study of Sound Symbolism," AAAI 2026, arXiv 2511.10045](https://arxiv.org/abs/2511.10045)):
  - The dataset contains 8,052 mimetic words from English, French, Japanese and **Korean**, plus 2,930 pseudo-words.
  - Multimodal LLMs show phonetic intuitions that align with linguistic research across up to 25 semantic dimensions, such as sharp versus round.
  - Their attention concentrates on iconic phonemes.
  - → Implication: models have some latent sensitivity to 양성/음성 and consonant-strength contrasts, so stating the dimensions explicitly (small/light/bright versus big/heavy/dark) is a plausible lever. This is recognition, not evidence of correct choice in narrative context.
- **Loakman, Li & Lin** ([Loakman, Li & Lin, "With Ears to See and Eyes to Hear: Sound Symbolism Experiments with Multimodal Large Language Models," EMNLP 2024, arXiv 2409.14917](https://arxiv.org/abs/2409.14917)), on kiki-bouba and mil-mal replications:
  - Agreement with human iconicity judgements varies by model.
  - Models need more explicit task information than human participants.
  - Magnitude symbolism is easier than shape symbolism.
  - Performance depends strongly on model size.
  - → Implication: give explicit size and intensity framing in ideophone rules. Smaller or cheaper models used for drafting may choose ideophones worse.
- **Sasaki & Koseki** ([佐々木 稔·小関 竜真, 「大規模言語モデルにおける日本語オノマトペの音韻的・意味的特徴の分析」, LRW 2025 (NINJAL)](https://clrd.ninjal.ac.jp/lrw/lrw2025/i1-1C.pdf)):
  - LLM embeddings of Japanese onomatopoeia cluster by *phonological* similarity when presented alone.
  - They cluster by *semantic* similarity when presented in sentence context.
  - With the two averaged, phonological features dominate.
  - → Implication: given bare word lists, models may conflate sound-alike ideophones. Retrieval should give each candidate with an example sentence in context.

### Inferences
- Without lexical guidance, Claude's Korean fiction is likely to:
  1. over-nominalise narration (…것이었다, …에 대한, abstract Sino-Korean nouns);
  2. reuse a narrow set of safe descriptive words across chapters, which will show up as slow vocabulary growth;
  3. insert English-style commas;
  4. under-use the variety of endings that distinguishes human Korean prose.

  This rests on KatFish, 비립 2025 and the English studies above. No study yet tests Claude's Korean *fiction* specifically.
- Errors are most likely in rare-word sense, in derived or irregular forms, and in grade choice for ideophones and colours (KoBALT morphology and phonology, HAE-RAE RW). These errors are cheap to catch with dictionary lookup and a morphological analyser.
- The benchmark numbers come mostly from 2023–2025 models. KoBALT shows that even Claude 3.7 Sonnet was below 40% on morphology and phonology, so the weakness probably persists. WriterProgram should build its own small evaluation of ideophone, colour and rare-word choice in narrative context.

### Gaps
- **No Korean fiction evaluation.** I found no study evaluating LLM generation or understanding of Korean 의성어·의태어, 색채어 or 감각어 in creative writing, and no Korean "AI-overused word" list. Korean searches on KCI-indexed sites and in English returned nothing.
- **No current model scores.** I found no HAE-RAE Rare Words results for current Claude models.
- **조신 2024 (bibliographic only).** 조신, 「ChatGPT 생성 텍스트의 문체적 특성 연구 – 학술논문의 서론을 중심으로」, 텍스트언어학 56: 115–136 ([record](https://dcollection.sogang.ac.kr/dcollection/srch/srchDetail/000000081049)). Its findings were not accessible.
- **Genre coverage.** KatFish covers essays, poetry and abstracts but not prose fiction. 비립 2025 covers argumentative essays only.
- **Japanese study (not opened).** A 2024 Harvard bachelor's thesis applying machine learning to Japanese and Korean ideophones was seen only in search results.

---

## 3. How should lexical diversity be measured for agglutinative Korean, and which metrics suit a linter?

### Takeaway
Use length-robust moving-window indices: MATTR as the primary index, with MTLD as a cross-check. Compute them on morpheme-analysed lemmas of content words, and report ending diversity and lexical density by part of speech separately. Raw TTR, by eojeol or by morpheme, is invalid across chapters of different lengths. The definition of the token unit changes results, and Korean values are lower and more constrained than English, so thresholds must be calibrated on human Korean fiction by genre.

### Cited Findings
- **MATTR.** TTR is affected by text length. MATTR computes TTR over a moving window, is independent of text length, and "can detect changes within a text as well as differences between texts". Source: [Covington & McFall 2010, "Cutting the Gordian Knot: The Moving-Average Type–Token Ratio (MATTR)," *Journal of Quantitative Linguistics*](https://doi.org/10.1080/09296171003643098) (abstract via OpenAlex API).
  - → Implication: MATTR also gives a per-paragraph diversity curve that can locate "flat" passages for targeted rewriting.
- **Stability across text lengths.** Across nine lexical-diversity indices on 4,542 L2 argumentative essays, MATTR and two versions of MTLD were the most stable. MATTR in particular maintained "a high degree of stability across all text lengths". Source: [Zenker & Kyle 2021, "Investigating minimum text lengths for lexical diversity indices," *Assessing Writing*, article 100505](https://doi.org/10.1016/j.asw.2020.100505) (abstract via Semantic Scholar API).
  - → Implication: MATTR as the primary metric and MTLD as secondary for both scene-length and chapter-length texts.
- **Original validation of MTLD, vocd-D and HD-D:** [McCarthy & Jarvis 2010, "MTLD, vocd-D, and HD-D: A validation study of sophisticated approaches to lexical diversity assessment," *Behavior Research Methods* 42(2)](https://doi.org/10.3758/BRM.42.2.381) (bibliographic only; abstract not accessible).
- **The token-unit definition matters.** Jarvis & Hashimoto compared five definitions of a word type: orthographic form, lemma with automatic POS tags, lemma with manually corrected tags, flemma, and word family. They applied MTLD, MTLD-W and MATTR to 60 English narratives and checked them against 55 human raters (α = .980). The three measures performed comparably, but two of the type definitions gave mixed results across measures. Source: [Jarvis & Hashimoto 2021, "How operationalizations of word types affect measures of lexical diversity," *International Journal of Learner Corpus Research* 7(1): 163–194](https://experts.nau.edu/en/publications/how-operationalizations-of-word-types-affect-measures-of-lexical-/).
  - → Implication: in Korean, the choice between eojeol, morpheme and lemma, and the choice of analyser, must be fixed and documented. Values are not comparable across settings.
- **Korean learner writing** ([배도용 2012, 「한국어 학습자의 쓰기에 나타난 어휘 다양도 및 어휘 밀도 연구」, 언어과학 19(1): 99–117](https://www.earticle.net/Article/A168089)):
  - TTR was inversely related to text length.
  - Word-based and morpheme-based TTR varied by learners' first language.
  - Lexical density (content-word share) stayed below 40%, with density ranked common noun > verb > adjective > common adverb.
  - → Implication: confirms that raw TTR cannot be used in Korean. Add lexical density by part of speech as a complementary metric.
- **KatFish measurement choices** ([KatFishNet](https://arxiv.org/html/2503.00032v4)): the study analysed Korean at morpheme level with the HanNanum tagger and used Zipf and Heaps curves for lexical diversity. LLM text showed a slower Heaps curve.
  - → Implication: a Heaps-curve exponent per chapter or per arc is a length-aware vocabulary-growth metric suited to long web-novel serials.
- **Korean values are lower.** 비립 2025 applied MATTR, HD-D, vocd-D, MTLD and a revised TTR to Korean, English and Chinese LLM essays. Korean varied less than English, and single metrics were judged insufficient. Source: [비립 2025](https://scholarworks.sookmyung.ac.kr/item/32a3b366-0a67-4ef0-b296-19d35a40eca8).
  - → Implication: use several indices, and take Korean-specific baselines.
- **Fixed-size sampling for batch evaluations.** For corpus-level comparison, Guo et al. used Unique-n with fixed samples of 40K n-grams per n to reduce length effects. Source: [Guo et al. 2025](https://arxiv.org/html/2412.10271v2).
  - → Implication: an option for offline A/B evaluations of prompt variants rather than per-chapter linting.

### Inferences
- **Eojeol versus morpheme.** Eojeol-based types conflate inflection with lexical choice: 집에, 집을 and 집으로 are three eojeol types but one noun lemma. Morpheme-based lemma types isolate lexical choice. The linter should compute:
  1. content-lemma MATTR over nouns, verbs, adjectives, adverbs and roots (e.g., Sejong-style tags NNG/NNP, VV, VA, MAG, XR);
  2. eojeol MATTR as a sanity check;
  3. ending-type diversity (distinct connective and final endings per window) as a separate "grammatical texture" metric, since human prose is richer in endings (KatFish) and endings are style markers (김일환·이도길 2015).
- **Window sizes.** Use two windows, such as 50 and 100 content lemmas, and calibrate both. Content-lemma windows in Korean span more running text than English word windows, so window sizes from English studies are not directly comparable. This is an inference to validate.
- **Diversity is not quality.** Kendro et al. found that newer models can be more diverse and less human-like. The linter should flag values *outside* a human band in both directions, for example below the 10th or above the 95th percentile of human chapters in the same genre.
- **Lexical sophistication** is distinct from diversity. Measure it as the share of content lemmas outside the 2003 learner list or below a frequency rank in the 2002 survey. Rising sophistication with falling naturalness is the "dictionary-stuffing" failure mode.

### Gaps
- I found no Korean study validating MATTR, MTLD or HD-D at morpheme versus eojeol level against human judgements of literary quality.
- 비립 2025 does not state its Korean tokenization unit on the record page I opened.
- Bestgen (2025), a critique of MATTR, was seen only in search results (not opened).
- No published Korean fiction norms (genre bands) for these indices were found. WriterProgram must build its own from licensed human text.

---

## 4. Which lexical resources could supply vocabulary to a prompt or retrieval step, and on what terms?

### Takeaway
The two NIKL dictionaries, 표준국어대사전 and 우리말샘, are the best-licensed and most useful sources. Their text is CC BY-SA 2.0 KR, so commercial use is allowed with attribution and ShareAlike. Example sentences that cite a published source, and multimedia files, are excluded. Their Open APIs expose word origin (native, Sino-Korean, loanword, hybrid), part of speech, definitions, examples and relations.

모두의 말뭉치 offers a copyright-cleared literary-book corpus (문어 말뭉치 2025, including 책-상상(문학)). Its terms, however, prohibit copying, redistribution and commercial use unless explicitly approved under a signed corpus-use agreement, so it suits internal statistics and calibration, not prompt injection of raw text.

Sentiment and emotion resources range from MIT-licensed (KOTE) to unlicensed (KnuSentiLex). KorLex's licence is unclear.

### Cited Findings

| Resource | Access | Licence / terms | Commercial use | Notes |
|---|---|---|---|---|
| **표준국어대사전** (NIKL) | Open API with key; `search.do` and `view.do`. The `type2` filter takes native/chinese/loanword/hybrid (고유어/한자어/외래어/혼종어); `pos` and `cat` filters; responses include `word_type` (고유어 여부), `original_language`, pronunciation, conjugation, relations, examples ([API doc](https://stdict.korean.go.kr/openapi/openApiInfo.do)) | All site material not otherwise marked is CC BY-SA 2.0 KR. Attribution and ShareAlike apply. Multimedia has separate per-file terms. Example sentences with a cited source belong to the original author and need their permission ([copyright policy](https://stdict.korean.go.kr/join/copyrightPolicy.do)) | Yes, explicitly ("상업적 용도까지 포함하여 자유롭게") | No daily quota stated on the API page |
| **우리말샘** (NIKL, open participatory dictionary) | Open API with key; 10–100 results per request; `start` 1–1000; filters `type2` (origin), `type3` (general/dialect/North Korean/archaic), `type4` (general/technical), `lang` (1 = 고유어, 2 = 한자, …); fields word, sense_no, definition, pos, examples, category, region ([API doc](https://opendict.korean.go.kr/service/openApiInfo)) | CC BY-SA 2.0 KR; attribution; ShareAlike; multimedia separately licensed; cited example sentences belong to the original holders ([copyright policy](https://opendict.korean.go.kr/service/copyrightPolicy)) | Yes | Includes dialect and archaic words: filter `type3=general` for standard narration and use dialect deliberately in dialogue |
| **한국어기초사전** (NIKL) | Open API exists (linked from the NIKL dictionary family footer) | **Unverified.** The official copyright page blocked automated access (HTTP 400/404). Commonly reported as CC BY-SA 2.0 KR | Unverified | Treat as unverified until checked by a human |
| **모두의 말뭉치** (NIKL) | Online application with a stated purpose (options include language research, AI research, "인공 지능(모델/서비스/제품 등) 개발 및 개선", AI performance evaluation, competitions, education). Admin review, then a 말뭉치 이용 약정서 is e-mailed and must be signed before download. The use period is limited to the source material's licence period; some requests require an official letter (공문) ([request page](https://kli.korean.go.kr/corpus/main/requestMain.do)) | Terms of service: except where explicitly approved, users may not copy, reproduce, transmit, modify, sell, distribute, sublicense or *commercially use* site materials, and may not use downloads beyond the approved scope ([이용 약관](https://kli.korean.go.kr/corpus/boards/termsInfo.do)) | Only as approved in the signed agreement | Relevant corpora: **문어 말뭉치 2025** (v1.0: 10,293 copyright-cleared works incl. 책-상상(문학)); 문어 말뭉치 v1.2 (books, magazines, reports); 국어 역사 자료 말뭉치 2025 (incl. 11 신소설); 어휘 의미 분석 말뭉치 (sense-tagged); 감성 분석 말뭉치 2020; 형태 분석 말뭉치. The 21세기 세종계획 corpus is distributed through a separate board ([request page](https://kli.korean.go.kr/corpus/main/requestMain.do)) |
| **한국어 학습용 어휘 목록** (2003) | Download (Excel, txt, HWP) | KOGL Type 1 ([page](https://www.korean.go.kr/front_eng/down/down_02V.do?etc_seq=70)) | Yes. KOGL Type 1 requires attribution only and allows commercial use and modification ([KOGL types](https://www.kogl.or.kr/info/license.do)) | 5,965 graded words with 2002 frequency ranks |
| **KOGL AI type** (for any NIKL material carrying it) | — | Free use only as AI training data; outputs must not be identical or substantially similar to the original; resale of training data is prohibited ([KOGL](https://www.kogl.or.kr/info/license.do)) | Model commercial use allowed | Check each item's KOGL mark |
| **KorLex 1.5** (Pusan National Univ.) | Paper describes the resource | About 130,000 synsets and 150,000 senses; nouns, verbs, adjectives, adverbs, classifiers; modelled on Princeton WordNet 2.0; built 2004–2007 ([윤애선·권혁철·이은령·황순희 2008, HCLT](https://koreascience.kr/article/CFKO200808355727766.pdf)) | **Licence not stated** | Useful for synonym and hypernym expansion if licensable |
| **KnuSentiLex** | GitHub ([repo](https://github.com/park1200656/KnuSentiLex)) | **No LICENSE file**, so by default all rights reserved. Credited to the Data Intelligence Lab, *Kunsan* National University (README dated 2018-05-11), not Kyungpook | Ask the authors | 14,843 entries (1-grams, 2-grams, idioms, patterns, abbreviations, emoticons); five-level polarity (very positive 2,597; positive 2,266; neutral 154; negative 5,029; very negative 4,797). Mostly mined from 표준국어대사전 glosses with a Bi-LSTM, then reviewed by annotators |
| **KOTE** | GitHub / Hugging Face ([repo](https://github.com/searle-j/KOTE)) | **MIT** | Yes | About 50,000 online comments, each labelled by 5 annotators with 43 emotions plus "none" (44 categories, about 250,000 labels). Online-comment register, not literary |
| **434 Korean emotion words** | Journal article ([박인조·민경환 2005](https://www.accesson.kr/ksppa/v.19/1/109/14556)) | No reuse licence stated | Unclear | Includes valence and activation ratings |
| **LEX-ICON** (mimetic words incl. Korean) | Described in [Jeong et al. AAAI 2026](https://arxiv.org/abs/2511.10045) | Licence not checked | Unknown | Possible seed for an ideophone list |
| **KoBALT data** | Described in [Shin et al. 2025](https://arxiv.org/html/2505.16125v1) | KOGL BY-NC-ND | No (non-commercial, no derivatives) | Usable for internal evaluation only |

Additional notes on these resources:
- The 표준국어대사전 API's view responses carry, for example, `<word_type>고유어</word_type>` for 나무 ([API doc](https://stdict.korean.go.kr/openapi/openApiInfo.do)).
  - → Implication: the linter can label every content lemma by word origin with one cached lookup, which makes per-part-of-speech origin profiles feasible.
- Both NIKL dictionary sites exclude cited example sentences (출전이 있는 용례) from the CC licence ([표준국어대사전 policy](https://stdict.korean.go.kr/join/copyrightPolicy.do); [우리말샘 policy](https://opendict.korean.go.kr/service/copyrightPolicy)).
  - → Implication: the word bank must drop or replace any example that quotes a literary source, which also avoids leaking other novelists' sentences into generated text.

### Inferences
- **Recommended stack.**
  - Build the word bank offline from 표준국어대사전 and 우리말샘 API pulls, keeping attribution metadata.
  - Enrich it with collocation and frequency statistics computed internally from licensed human corpora: 모두의 말뭉치 under a signed agreement, or WriterProgram users' own manuscripts with consent.
  - Rank rarity with the 2002 frequency ranks and 2003 learner grades.
- **ShareAlike risk.** If WriterProgram distributes the dictionary-derived word bank to users, for example as a browsable "어휘 서랍" inside the app, CC BY-SA 2.0 KR's ShareAlike likely applies to that adapted database, and attribution must be shown. Using dictionary data transiently in prompts carries less obvious obligations. This is my legal inference and needs counsel review.
- **Literary corpus use.** Do not paste raw 모두의 말뭉치 literary text into production prompts or few-shot examples unless the signed 약정서 explicitly allows it. Use the corpus for statistics such as frequency, collocations and lint calibration bands, and confirm even that use in the agreement.
- **Emotion resources.** Prefer KOTE (MIT) and the 박인조·민경환 list (cite it; check reuse terms) over KnuSentiLex unless permission is obtained.

### Gaps
- 한국어기초사전 licence and API quota are not verified from an official page (blocked).
- AI Hub dataset terms were not verified. Search surfaced only a sample application form and generic policy news; I found no AI Hub literary or web-novel corpus.
- KorLex distribution and licence are unknown; the authors are named in the 2008 paper.
- I found no openly licensed, machine-readable ideophone dictionary or classified thesaurus (분류 어휘 사전). A WriterProgram ideophone lexicon would have to be assembled from 우리말샘/표준국어대사전 entries (which mark 큰말/작은말/센말 relations in their relation fields; I did not check the relation field format) and curated by hand.
- The 세종 corpus's current licence terms were not checked; it is on a separate board.
- Whether derived statistics, such as frequency lists from 모두의 말뭉치, can ship in a commercial product depends on the 약정서 text, which is sent only after approval. Not verified.

---

## 5. Is there evidence that lexical constraints, word banks or dictionary retrieval in prompts improve richness without hurting fluency or naturalness?

### Takeaway
Direct evidence for Korean fiction does not exist. Evidence from other languages and tasks suggests:
- *Soft* prompt-level word lists, combined with a verify-and-rewrite loop, beat hard decoding constraints on grammaticality and natural word use.
- Planning before drafting increases uptake of the target words.
- Forced inclusion can produce uninformative or misleading uses of a word.
- Large negative lists need backtracking samplers, not plain token bans.
- Generic "be creative" instructions do little.
- Temperature and min-p sampling can raise diversity with modest coherence cost.

### Cited Findings
- **SRS-Stories** ([Kamzela, Lango & Dušek, "SRS-Stories: Vocabulary-constrained multilingual story generation for language learning," arXiv 2512.18362](https://arxiv.org/html/2512.18362v1); listed as EMNLP 2025 Industry in the ACL Anthology, a listing I did not open). The task: an LLM writes stories that must use target words at least three times while staying within a learner's known vocabulary.
  - The paper's automatic evaluation (English, B1, Llama-3.1-70B):

    | Method | Grammar | Coherence | Interestingness | Out-of-vocabulary rate | Uses per target word |
    |---|---|---|---|---|---|
    | Constrained beam search | 3.22 | 3.82 | 3.92 | 6.71% | — |
    | Simple prompting | 4.05 | 4.22 | 4.01 | 0.64% | ≈1.92 |
    | Plan-first prompting | slightly lower grammar | — | — | 0.01% | ≈2.68 |

  - Human evaluation (English), constrained beam search versus simple prompting:

    | Measure | Constrained beam search | Simple prompting | p |
    |---|---|---|---|
    | Grammar | 3.56 | 4.08 | .0151 |
    | Target-word use | 1.24 | 3.88 | < .0001 |
    | Overall quality | 3.24 | 3.08 | — |

    Coherence and interestingness were similar between the two.
  - Rewrite loops raised interestingness but sometimes increased out-of-vocabulary words and story length.
  - Agreement between the LLM judge and humans was moderate for English (Pearson about 0.46–0.56) and weak for Polish.
  - → Implication: give candidate words in the prompt, plan first, then verify and repair. Do not use hard lexical decoding constraints.
  - → Implication: do not rely on an LLM judge alone for Korean nuance.
- **Valentini et al.** ([Valentini et al., "Measuring Contextual Informativeness in Child-Directed Text," COLING 2025, arXiv 2412.17427](https://arxiv.org/abs/2412.17427); [html](https://arxiv.org/html/2412.17427v1)):
  - The authors state that in LLM-generated stories "target words are often used in uninformative and misleading contexts". Their support is one illustrative example (Figure 1), not a systematic count.
  - Their LLM-based informativeness scorer reached Spearman 0.4983 against human judgements, versus 0.3534 for the best baseline.
  - → Implication: checking that a word is included is not enough. The linter or an LLM critic must check context fit (collocation and sense), and the prompt should make candidates optional.
- **Generic prompting and temperature.** Asking explicitly for creativity had minimal effect on diversity. Higher temperature raised lexical diversity with only a minor drop in relevance. Source: [Guo et al., TACL 2025](https://arxiv.org/html/2412.10271v2).
  - → Implication: replace adjectives such as "생생하게" or "문학적으로" with concrete lexical instructions and candidates.
- **Min-p sampling.** Min-p improves both quality and diversity across Mistral and Llama-3 models from 1B to 123B, especially at higher temperatures, and human evaluators preferred its outputs for quality and creativity. Source: [Nguyen et al., "Turning Up the Heat: Min-p Sampling for Creative and Coherent LLM Outputs," ICLR 2025, arXiv 2407.01082](https://arxiv.org/abs/2407.01082).
  - → Implication: if any open-weight model is used in the pipeline (for example, a local drafting model), prefer min-p with higher temperature over top-p.
- **Antislop suppression results** ([Paech et al., Antislop, arXiv 2510.15061](https://arxiv.org/abs/2510.15061)):
  - Plain token banning "becomes unusable" at about 2,000 banned items, whereas the backtracking Antislop Sampler suppresses more than 8,000 patterns "while maintaining quality".
  - Fine-tuning with FTPO cut slop by 90% while maintaining or improving GSM8K, MMLU and creative-writing scores.
  - DPO degraded writing quality and lexical diversity.
  - → Implication: with a hosted model and no sampler control, keep the in-prompt "avoid" list short and high-precision. Enforce the long tail with the post-hoc linter plus targeted regeneration.
- **In-context examples.** These raise rare-word accuracy sharply: Polyglot-Ko-12.8B went from 53.09 to 76.05 on HAE-RAE Rare Words, and LLaMA-2-13B from 32.35 to 43.95. Source: [Son et al.](https://arxiv.org/abs/2309.02706).
  - → Implication: retrieved candidates should come with a definition and one in-context example, not as bare lemmas.
- **Temperature in Korean.** In Korean, English and Chinese essays, temperature changes raised lexical variation only modestly compared with language and model effects. Source: [비립 2025](https://scholarworks.sookmyung.ac.kr/item/32a3b366-0a67-4ef0-b296-19d35a40eca8).
  - → Implication: decoding alone will not bring Korean output to human-like richness; lexical guidance is needed.

### Inferences
- The best-supported design is *optional, contextualised candidates* (definition, example, contrast set) chosen by scene features, plus *plan-then-draft*, *post-hoc verification* and *targeted rewriting of flagged sentences only*.
- Hard quotas such as "use 5 rare words per scene" are likely to produce token-stuffing and misfit. Valentini et al. show the risk of uninformative use. SRS-Stories shows that rewriting raises other errors.
- Because LLM-judge reliability drops outside English (SRS-Stories), the Korean linter should be rule-based wherever possible: dictionary existence, part of speech and conjugation, collocation lists, density and comma metrics. LLM critique should be reserved for fit and nuance.

### Gaps
- I found no study, in Korean or any language, that A/B-tests dictionary-retrieval-augmented prompting for literary fiction on *both* lexical richness and human-rated naturalness.
- I found no Korean replication of Antislop or excess-vocabulary methods.
- The effect sizes above come from English, Chinese and Polish learner-story settings. Transfer to Korean fiction is untested.

---

## 6. Recommendations: prompt rules, word-bank/RAG design, and lint metrics

### Takeaway
Treat vocabulary as a *retrieve → choose → verify* loop:
- the prompt carries a small set of Korean-specific lexical rules;
- a dictionary-backed word bank supplies optional contrast sets matched to scene features;
- a rule-based linter measures length-robust diversity, nominal style, ending variety, ideophone and colour usage, emotion naming, commas and clichés against genre-specific human baselines, then triggers targeted rewrites.

### Cited Findings (evidence anchors for the recommendations)
- LLM Korean prose is more nominal, more comma-heavy, poorer in endings and predicates, and narrower in vocabulary ([KatFishNet](https://arxiv.org/html/2503.00032v4)).
- Native richness sits in verbs (97% native), adverbs (94%), particles and endings (100%) ([정호성 2000](https://www.korean.go.kr/nkview/nklife/2000_1/2000_0104.pdf)).
- Ideophone grade laws (덜렁/달랑; 감감/깜깜/캄캄) and single-versus-reduplicated constraints ([채완 1993](https://www.korean.go.kr/nkview/nklife/1993_2/3_3.html)); paradigms 뱅뱅/팽팽 and 빙글/뱅글 ([Kwon 2016](https://www2.ninjal.ac.jp/past-events/mimetics/Kwon_2016_NINJAL_abst.pdf)); harmony is weaker for onomatopoeia ([Kwon 2018](https://doi.org/10.5334/labphon.53)); constraints and the value of combination ([성기옥 1993](https://www.korean.go.kr/nkview/nklife/1993_2/3_6.html)).
- Colour grades as defined in 표준국어대사전 ([누렇다 etc.](https://stdict.korean.go.kr/search/searchResult.do?searchKeyword=%EB%88%84%EB%A0%87%EB%8B%A4)).
- Emotion-word list ([박인조·민경환 2005](https://www.accesson.kr/ksppa/v.19/1/109/14556)).
- MATTR and MTLD stability ([Zenker & Kyle 2021](https://doi.org/10.1016/j.asw.2020.100505); [Covington & McFall 2010](https://doi.org/10.1080/09296171003643098)); unit effects ([Jarvis & Hashimoto 2021](https://experts.nau.edu/en/publications/how-operationalizations-of-word-types-affect-measures-of-lexical-/)); Korean-specific baselines ([비립 2025](https://scholarworks.sookmyung.ac.kr/item/32a3b366-0a67-4ef0-b296-19d35a40eca8)); human band, not maximum ([Kendro et al. 2025](https://arxiv.org/abs/2508.00086)).
- Soft prompting beats hard constraints; plan-first helps uptake ([SRS-Stories](https://arxiv.org/html/2512.18362v1)); context fit matters ([Valentini et al.](https://arxiv.org/abs/2412.17427)); creativity instructions are weak ([Guo et al.](https://arxiv.org/html/2412.10271v2)); long ban lists need backtracking ([Antislop](https://arxiv.org/abs/2510.15061)); excess-frequency cliché mining ([Kobak et al.](https://arxiv.org/abs/2406.07016)).
- Licences: CC BY-SA 2.0 KR dictionaries ([표준](https://stdict.korean.go.kr/join/copyrightPolicy.do), [우리말샘](https://opendict.korean.go.kr/service/copyrightPolicy)); restrictive 모두의 말뭉치 terms ([약관](https://kli.korean.go.kr/corpus/boards/termsInfo.do)).

### Inferences (the recommendations)

#### A. Prompt rules (drop-in Korean wording; each maps to evidence above)
1. **서술 술어 (narrative predicates).** "서술문에서는 '~을 느꼈다', '~하는 것이었다', '~에 대한/~를 통한' 같은 명사화·추상 표현보다, 장면의 움직임과 감각을 직접 가리키는 고유어 동사·형용사·부사를 우선한다."
   - Evidence: LLM nominal bias (KatFish); native predicates (정호성).
2. **흉내말 사용 (ideophone use).**
   - "의성어·의태어는 감각이 초점이 되는 순간에만, 장면당 0~2개 쓴다. 대상의 크기·무게·세기에 맞는 짝말을 고른다."
   - "작고 가볍고 밝으면 양성 계열(달랑, 뱅글, 보글보글), 크고 무겁고 둔하면 음성 계열(덜렁, 빙글, 부글부글)을 쓴다."
   - "세기는 예사소리 < 된소리 < 거센소리(감감 < 깜깜 < 캄캄, 뱅뱅 < 팽팽, 강중강중 < 깡충깡충)로 조절한다. 표기는 현행 표준어(깡충깡충 ○, 깡총깡총 ×)를 따른다."
   - "흉내말은 주어·서술어와 어울려야 하며, 같은 흉내말을 한 장(章) 안에서 되풀이하지 않는다."
   - Evidence: 채완, Kwon 2016/2018, 성기옥.
3. **단독형과 반복형 (single versus reduplicated).** "한 번의 돌발적 사건은 단독형(병이 덜컥 났다, 울음을 뚝 그쳤다), 지속·반복되는 상태는 반복형(햇볕이 쨍쨍 내리쬔다)으로 쓴다."
   - Evidence: 채완.
4. **색채어 (colour terms).** "'노란색의', '붉은색의'처럼 색이름에 수식어를 겹치지 말고, 밝기·탁도·정도를 담은 색채 형용사 하나로 쓴다(노랗다↔누렇다, 샛노랗다/싯누렇다, 노르스름하다/누르스름하다, 노르께하다, 불그스름하다, 불그죽죽하다). 정서적 함축(누렇다: 다소 탁하고 어두움, 불그죽죽하다: 칙칙하고 고르지 않음)이 장면의 분위기와 맞는지 확인한다."
   - Evidence: 표준국어대사전 definitions.
5. **감정 명명 절제 (limit naming emotions).** "인물의 감정을 '슬펐다/화가 났다/행복했다' 같은 감정 단어로 직접 진술하는 것은 장면당 1회 이하로 줄이고, 신체 감각·행동·사물의 묘사로 드러낸다."
   - Evidence: the lint list is the 434-word list.
6. **어휘 후보는 선택 사항 (candidates are optional).** "아래 '어휘 후보'는 참고용이다. 장면의 감각·인물·문체에 정확히 맞는 것만 최대 2개까지 쓰고, 맞지 않으면 쓰지 않는다. 뜻이나 활용이 확실하지 않은 낱말은 쓰지 않는다. 같은 희귀어를 다른 장에서 반복하지 않는다."
   - Evidence: SRS-Stories, Valentini et al., HAE-RAE, KoBALT.
7. **유의어 돌려막기 금지 (no synonym cycling).** "같은 내용어의 근접 반복은 피하되, 사전식 유의어를 기계적으로 바꿔 끼우지 않는다. 바꿀 때는 의미 차이(크기·세기·밝기·정서)를 살린다."
   - Evidence: inference from Kendro et al. (over-diversity is non-human) and from ideophone grade semantics.
8. **쉼표 (commas).** "연결 어미(-고, -며, -지만, -는데) 뒤와 문장 첫머리의 접속 부사(그리고, 하지만, 그러나) 뒤에는 습관적으로 쉼표를 찍지 않는다."
   - Evidence: KatFish.
9. **상투어 회피 (avoid clichés).** Keep a short "avoid" list (at most about 30 items) that is data-mined: phrases over-represented in the model's Korean fiction relative to human fiction (Kobak method). The rest is enforced by the linter.
   - Evidence: Antislop, Kobak.
10. **레지스터 (register per genre and speaker).**
    - "웹소설 모드: 가독성을 해치는 희귀어·한자어는 절제하고, 흉내말은 대사와 액션 장면에 집중한다."
    - "문학 모드: 서술부에서 고유어 술어·흉내말·색채어의 폭을 넓히되 밀도는 낮게 유지한다."
    - "대사는 인물의 나이·지역·교육 배경에 맞는 어휘만 쓴다. 방언·유아어는 의도적으로만 쓴다."
    - Evidence: inference; register tags come from 우리말샘 `type3` and from 윤희원.

#### B. Word-bank / RAG design
- **Build offline** from 표준국어대사전 and 우리말샘 API pulls. Keep only standard words (`type3=general`) by default and dialect or archaic words as opt-in. Store the attribution "출처: 국립국어원 표준국어대사전/우리말샘, CC BY-SA 2.0 KR". Drop example sentences that cite a source.
- **Entry schema** (one record per lemma, grouped into *families*):
  - `lemma`, `pos`, `word_type` (고유어/한자어/외래어/혼종어 from the API), `definition`, one `example` (non-cited, or written in-house);
  - `family_id`, so that 노르스름하다/누르스름하다/노르께하다/노랗다/누렇다/샛노랗다/싯누렇다 form one family;
  - `contrast` features: vowel grade (양성/음성/neutral), consonant grade (예사/된/거센), degree (약/보통/강), brightness and turbidity for colours, size and weight for ideophones;
  - `sense_modality` (시각/청각/촉각/미각·후각/심리, following 유창돈's classes), `register` (문어/구어/유아어/방언/옛말);
  - `form` (single or reduplicated; derived verbs -거리다/-대다/-이다/-하다);
  - `collocates` (top subjects, objects and verbs, mined from licensed corpora);
  - `rarity` (2002 frequency rank; 2003 learner grade or "not in list");
  - `emotion` tags (valence, self/other focus) for emotion words;
  - `ai_overuse` flag (from cliché mining).
- **Retrieval flow:**
  1. Extract scene features from the outline or draft: dominant senses, light and colour, salient objects with size and weight, motion type, emotion valence and focus, POV character, genre mode.
  2. Query families by these features.
  3. Return 5 to 10 families, at most about 25 lemmas, each with definition, contrast feature and one example.
  4. The prompt marks them optional (rule A6), and the model writes a short plan before drafting (SRS-Stories "Planning" uptake).
  5. Lint the draft.
  6. Regenerate only flagged sentences, with the specific reason ("빙글 → 대상이 작음: 뱅글 검토").
  7. Log the rare words used per chapter to prevent cross-chapter repetition in serials.
- **Corpora for statistics only.** Use 모두의 말뭉치 문어 말뭉치 2025 (literary subset) under a signed agreement, and/or consenting authors' manuscripts, to compute collocations and human baseline bands. Do not inject raw corpus text unless the agreement allows it.

#### C. Lint metrics

Analyse with one fixed Korean morphological analyser, documented. Report per scene and per chapter, flag against genre-specific human bands, and trigger targeted rewrites.

| # | Metric | Computation | Evidence | Flag / action |
|---|---|---|---|---|
| 1 | Content-lemma **MATTR** | Window of 50 and 100 content lemmas (nouns, verbs, adjectives, adverbs, roots); also a per-paragraph curve | Covington & McFall 2010; Zenker & Kyle 2021 | Below the human-band 10th percentile → "flat" passage; above the 95th percentile → possible thesaurus-stuffing |
| 2 | **MTLD** (lemma) | Standard MTLD on content lemmas | Zenker & Kyle 2021; 비립 2025 (multiple indices) | Cross-check of metric 1 |
| 3 | Eojeol MATTR and the gap to lemma MATTR | Same window on eojeol | Jarvis & Hashimoto 2021 (unit effects) | Large gap = variety coming from inflection, not word choice |
| 4 | **Vocabulary growth** (Heaps slope) | Unique content lemmas versus tokens across a chapter or arc | KatFish (LLM slower growth) | Slope below the band → recycled vocabulary across chapters |
| 5 | **Nominal ratio** in narration | Nouns ÷ (verbs + adjectives); count of 것-nominalisations, -적 and Sino-Korean -하다 verbs (via `word_type`) per 1,000 eojeol | KatFish (more nominals); 정호성 (-적 is Sino-Korean; native verbs) | Above the band → rewrite with native predicates |
| 6 | **Ending diversity** | Distinct connective and final ending types per 100 eojeol; share of the top 3 endings | KatFish (humans use more endings); 김일환·이도길 (endings are style markers) | Low diversity or a top-3 share above the band → vary sentence endings and connectives |
| 7 | **Ideophone density and grade checks** | Ideophones per 1,000 eojeol; at most 2 per paragraph; same ideophone repeated within a chapter; single/reduplicated × collocate violations (curated list); register = 유아어 in literary narration | 채완; 성기옥; 윤희원 | Over cap, repeated or misfit → rewrite |
| 8 | **Colour specificity** | Share of generic "X색(의)" against graded colour adjectives; colour words absent from the dictionary | 표준국어대사전 colour entries | Generic-heavy → suggest a family; non-existent form → error |
| 9 | **Direct emotion naming** | Hits from the 434-word list per 1,000 eojeol in narration | 박인조·민경환 2005 | Above the band → show-don't-tell rewrite |
| 10 | **Comma profile** | % of sentences with a comma; commas after connective endings; commas after sentence-initial conjunctive adverbs | KatFish (essays: human 26.31% vs LLM 61.03% of sentences; 4.10% vs 19.83% after connective endings) | Above the band → remove the commas |
| 11 | **Dictionary validation** | Every content lemma outside the top-N frequency list must exist in 표준국어대사전/우리말샘 with a matching part of speech; check conjugation of rare predicates | HAE-RAE, CLIcK, KoBALT (morphology weak) | Not found or bad form → error, unless marked as dialect or neologism in dialogue |
| 12 | **Sophistication** | Share of content lemmas outside the 2003 learner list or beyond frequency rank N | 2003 list; 2002 survey | Genre-specific band (web novel lower, literary higher) |
| 13 | **Cliché hits** | Matches against the mined Korean over-use list | Kobak; Antislop | Any hit → targeted rewrite |
| 14 | **Proximity repetition** | Same content lemma within about 30 eojeol, excluding stop-lemmas (것/하다/있다/되다…) | 조남호 2003 (top-frequency lemmas) | Flag for review (may be intentional) |

- **Calibration.** All bands come from human Korean fiction split by genre (web novel versus literary) and, where possible, by author. Recompute them when the analyser changes. Korean values are expected to be lower than English values (비립 2025).

### Gaps
- Thresholds are placeholders until calibrated on licensed human Korean fiction. No published Korean fiction norms were found.
- WriterProgram needs its own A/B evaluation of three prompt conditions: rules only, rules plus word bank, and rules plus word bank plus lint loop. It should measure the metrics above *and* human-rated naturalness and fit. No external study has done this for Korean.
- Curated collocation constraint lists for ideophones (rule 7 in the lint table) must be built by hand or mined from licensed corpora. No ready-made list was found.
- The CC BY-SA ShareAlike implications for a distributed word bank need legal review.

---

## 7. Master table of sources

### Takeaway
46 sources and resources were consulted. Most were opened and verified, and the exceptions are flagged in the table. They span Korean linguistics (NIKL 새국어생활 articles), dictionaries and corpora (NIKL), Korean LLM benchmarks (HAE-RAE, CLIcK, KoBALT), Korean human-versus-LLM text studies (KatFish, 비립 2025), sound-symbolism studies of LLMs, lexical-diversity methodology, and evidence on constrained generation.

### Cited Findings

| # | Title (authors) | Year | Venue / publisher | Lang | URL | One-line finding or licence |
|---|---|---|---|---|---|---|
| 1 | 표준국어대사전 수록 정보의 통계적 분석 (정호성) | 2000 | 새국어생활 10(1), 국립국어연구원 | KO | https://www.korean.go.kr/nkview/nklife/2000_1/2000_0104.pdf | 440,262 headwords: 고유어 25.28%, 한자어 57.12%, 외래어 5.26%; verbs 97%, adverbs 94% native |
| 2 | 우리말에는 어떤 말이 많이 쓰일까 (조남호) | 2003 | 새국어소식, 국립국어연구원 | KO | https://www.korean.go.kr/nkview/nknews/200302/55_3.html | 2002 survey: 1,484,463 tokens, 58,437 types; top lemmas 것, 하다, 있다, 되다 |
| 3 | 한국어 학습용 어휘 목록 | 2003 | 국립국어연구원 | KO | https://www.korean.go.kr/front_eng/down/down_02V.do?etc_seq=70 | 5,965 graded words with frequency ranks; KOGL Type 1 |
| 4 | 의성어·의태어의 통사와 의미 (채완) | 1993 | 새국어생활 3(2) | KO | https://www.korean.go.kr/nkview/nklife/1993_2/3_3.html | Ideophones as native stronghold; 덜렁/달랑, 감감/깜깜/캄캄; single/reduplicated constraints |
| 5 | 현대 국어 의성 의태어의 형태와 음운 (박창원) | 1993 | 새국어생활 3(2) | KO | https://www.korean.go.kr/nkview/nklife/1993_2/3_2.html | Alternants are separate words; vowel classes; harmony is partial (331 of ~640) |
| 6 | 의성어 의태어의 시적 위상과 기능 (성기옥) | 1993 | 새국어생활 3(2) | KO | https://www.korean.go.kr/nkview/nklife/1993_2/3_6.html | Concretisation and vividness, but strong expressive constraints; value arises in combination |
| 7 | 일본어의 의성 의태어 (강인선) | 1993 | 새국어생활 3(2) | KO | https://www.korean.go.kr/nkview/nklife/1993_2/3_7.html | Korean ideophone dictionaries hold 3,780–8,286 words; 1,109 Korean vs 617 Japanese groups (~1.7×); ~260 -거리다 verbs (162 on 의태어 roots) in 2M eojeol |
| 8 | 의성어·의태어의 개념과 정의 (윤희원) | 2003 | 새국어생활 13(4) | KO | https://www.korean.go.kr/nkview/nklife/1993_2/3_1.html | Alternation changes only 어감; ideophones pervasive, including child language |
| 9 | Iconicity correlated with vowel harmony in Korean ideophones (Kwon) | 2018 | Laboratory Phonology 9(1) | EN | https://doi.org/10.5334/labphon.53 | Onomatopoeic ideophones more disharmonic than cross-modal ones (abstract via OpenAlex) |
| 10 | Canonical approach to phonaesthemes in Korean ideophones (Kwon) | 2016 | NINJAL Intl. Symposium (abstract) | EN | https://www2.ninjal.ac.jp/past-events/mimetics/Kwon_2016_NINJAL_abst.pdf | Paradigms 뱅뱅/팽팽 (strength), 빙글/뱅글 (size) |
| 11 | 표준국어대사전 entries: colour terms (노르다, 누르다, 노랗다, 누렇다, 샛노랗다, 싯누렇다, 노르스름하다, 누르스름하다, 노르께하다, 불그스름하다, 불그죽죽하다, 새빨갛다, 시뻘겋다) and 깡충깡충/깡총깡총 | live | 국립국어원 | KO | https://stdict.korean.go.kr/search/searchResult.do?searchKeyword=%EB%88%84%EB%A0%87%EB%8B%A4 | Definitions encode brightness, turbidity, degree and affect; 깡충깡충 is "stronger and harsher than 강중강중"; 깡총깡총 is non-standard |
| 12 | 권재일 column (색깔말) in 새 국어 소식 | 2004 | 국립국어원 (orig. 한겨레) | KO | https://www.korean.go.kr/nkview/nknews/200501/78_11.html | 붉다 and 빨갛다 derivative families |
| 13 | 한국어 감정단어의 목록과 정서 차원 탐색 (박인조·민경환) | 2005 | 한국심리학회지: 사회 및 성격 19(1) | KO | https://www.accesson.kr/ksppa/v.19/1/109/14556 | 434 emotion words; valence and self/other dimensions; no activation dimension |
| 14 | 저자 판별을 위한 전산 문체론 (김일환·이도길) | 2015 | 국어국문학 170 | KO | https://scholar.korea.ac.kr/handle/2021.sw.korea/133037?mode=full | 70 novels, 14 authors; function categories best for authorship |
| 15 | HAE-RAE Bench (Son et al.) | 2024 | LREC-COLING 2024 / arXiv 2309.02706 | EN | https://arxiv.org/abs/2309.02706 | Rare Words 0-shot: Polyglot-Ko 53.1%, LLaMA-2-13B 32.4%; few-shot raises scores sharply |
| 16 | CLIcK (Kim et al.) | 2024 | arXiv 2403.06412 (reported as LREC-COLING 2024) | EN | https://arxiv.org/html/2403.06412v4 | Grammar: Claude-2 44.0%, GPT-3.5 38.9%; open Korean models near chance |
| 17 | KoBALT (Shin et al.) | 2025 | arXiv 2505.16125 | EN | https://arxiv.org/html/2505.16125v1 | Best model 61%; morphology 36%, phonology 31%; data KOGL BY-NC-ND |
| 18 | KatFishNet (Park, Kim, Kim & Han) | 2025 | ACL 2025 / arXiv 2503.00032 | EN | https://arxiv.org/html/2503.00032v4 | LLM Korean: more commas, more nominals, fewer endings, narrower vocabulary |
| 19 | 초거대언어모델의 한·영·중 논술형 글쓰기 능력 평가 (비립) | 2025 | 텍스트언어학 59 | KO | https://scholarworks.sookmyung.ac.kr/item/32a3b366-0a67-4ef0-b296-19d35a40eca8 | 7,200 texts; Korean diversity more constrained; temperature effect modest |
| 20 | ChatGPT 생성 텍스트의 문체적 특성 연구 (조신) | 2024 | 텍스트언어학 56 | KO | https://dcollection.sogang.ac.kr/dcollection/srch/srchDetail/000000081049 | Bibliographic only; findings not accessed |
| 21 | Playing with words (Reviriego et al.) | 2024 | Machine Learning with Applications 18 | EN | https://arxiv.org/abs/2308.07462 | ChatGPT: fewer distinct words, lower lexical richness than humans |
| 22 | Do LLMs produce texts with "human-like" lexical diversity? (Kendro, Maloney & Jarvis) | 2025 | arXiv 2508.00086 | EN | https://arxiv.org/abs/2508.00086 | All ChatGPT models differ from humans; newer models more diverse but less human-like |
| 23 | Benchmarking Linguistic Diversity of LLMs (Guo, Shang & Clavel) | 2025 | TACL 13 | EN | https://arxiv.org/html/2412.10271v2 | Stories: human 59.04 vs LLMs 52.28–56.90; creativity prompt minimal effect |
| 24 | Delving into LLM-assisted writing … excess vocabulary (Kobak et al.) | 2025 | Science Advances 11(27) | EN | https://arxiv.org/abs/2406.07016 | Excess-word method; ≥13.5% of 2024 abstracts LLM-processed |
| 25 | Antislop (Paech, Roush, Goldfeder & Shwartz-Ziv) | 2025 | arXiv 2510.15061 (ICLR 2026 listing not opened) | EN | https://arxiv.org/abs/2510.15061 | Slop >1,000× human frequency; backtracking suppresses 8,000+ patterns |
| 26 | Turning Up the Heat: Min-p Sampling (Nguyen et al.) | 2025 | ICLR 2025 | EN | https://arxiv.org/abs/2407.01082 | Better quality and diversity at high temperature; humans preferred it |
| 27 | Do Language Models Associate Sound with Meaning? (Jeong et al.) | 2026 | AAAI 2026 | EN | https://arxiv.org/abs/2511.10045 | LEX-ICON: 8,052 mimetic words incl. Korean; models show phonosemantic intuitions |
| 28 | With Ears to See and Eyes to Hear (Loakman, Li & Lin) | 2024 | EMNLP 2024 | EN | https://arxiv.org/abs/2409.14917 | Iconicity agreement varies with model; size-dependent; needs explicit task framing |
| 29 | 大規模言語モデルにおける日本語オノマトペの音韻的・意味的特徴の分析 (佐々木·小関) | 2025 | LRW 2025, NINJAL | JA | https://clrd.ninjal.ac.jp/lrw/lrw2025/i1-1C.pdf | Isolated onomatopoeia cluster by sound; in context, by meaning |
| 30 | Cutting the Gordian Knot: MATTR (Covington & McFall) | 2010 | Journal of Quantitative Linguistics | EN | https://doi.org/10.1080/09296171003643098 | Moving-window TTR independent of length (abstract via OpenAlex) |
| 31 | Investigating minimum text lengths for lexical diversity indices (Zenker & Kyle) | 2021 | Assessing Writing (art. 100505) | EN | https://doi.org/10.1016/j.asw.2020.100505 | MATTR and MTLD most stable; MATTR stable at all lengths (abstract via Semantic Scholar) |
| 32 | MTLD, vocd-D, and HD-D (McCarthy & Jarvis) | 2010 | Behavior Research Methods 42(2) | EN | https://doi.org/10.3758/BRM.42.2.381 | Bibliographic only |
| 33 | How operationalizations of word types affect measures of lexical diversity (Jarvis & Hashimoto) | 2021 | Int. J. Learner Corpus Research 7(1) | EN | https://experts.nau.edu/en/publications/how-operationalizations-of-word-types-affect-measures-of-lexical-/ | Definition of the token unit changes results; MTLD, MTLD-W and MATTR comparable |
| 34 | 한국어 학습자의 쓰기에 나타난 어휘 다양도 및 어휘 밀도 연구 (배도용) | 2012 | 언어과학 19(1) | KO | https://www.earticle.net/Article/A168089 | TTR inversely related to length; lexical density by part of speech |
| 35 | SRS-Stories (Kamzela, Lango & Dušek) | 2025 | arXiv 2512.18362 (EMNLP 2025 Industry listing not opened) | EN | https://arxiv.org/html/2512.18362v1 | Prompted word lists beat constrained beam search on grammar and word use |
| 36 | Measuring Contextual Informativeness in Child-Directed Text (Valentini et al.) | 2025 | COLING 2025 | EN | https://arxiv.org/abs/2412.17427 | Target words often used uninformatively in LLM stories (illustrative); scorer ρ = 0.498 |
| 37 | 표준국어대사전 저작권 정책 | live | 국립국어원 | KO | https://stdict.korean.go.kr/join/copyrightPolicy.do | CC BY-SA 2.0 KR; commercial use allowed; cited examples and multimedia excluded |
| 38 | 표준국어대사전 Open API | live | 국립국어원 | KO | https://stdict.korean.go.kr/openapi/openApiInfo.do | Key required; origin filter (type2); `word_type` field |
| 39 | 우리말샘 저작권 정책 | live | 국립국어원 | KO | https://opendict.korean.go.kr/service/copyrightPolicy | CC BY-SA 2.0 KR; commercial use allowed; cited examples and multimedia excluded |
| 40 | 우리말샘 Open API | live | 국립국어원 | KO | https://opendict.korean.go.kr/service/openApiInfo | Key; 10–100 results per call; filters type2/type3/type4/lang |
| 41 | 모두의 말뭉치 서비스 이용 약관 | live | 국립국어원 | KO | https://kli.korean.go.kr/corpus/boards/termsInfo.do | No copying, redistribution or commercial use unless explicitly approved |
| 42 | 모두의 말뭉치 신청 (corpus list and application) | live | 국립국어원 | KO | https://kli.korean.go.kr/corpus/main/requestMain.do | Signed 약정서; AI-development purpose selectable; 문어 말뭉치 2025 (literary books) |
| 43 | 공공누리 유형 안내 | live | KOGL | KO | https://www.kogl.or.kr/info/license.do | Type 1 = attribution only (commercial use and modification allowed); AI-type conditions |
| 44 | 한국어 어휘의미망 KorLex 1.5의 구축방법론과 정보구조 (윤애선·권혁철·이은령·황순희) | 2008 | HCLT 2008 | KO | https://koreascience.kr/article/CFKO200808355727766.pdf | ~130k synsets, ~150k senses; licence not stated |
| 45 | KnuSentiLex (Kunsan National Univ.) | 2018 | GitHub | KO | https://github.com/park1200656/KnuSentiLex | 14,843 polarity entries; no licence file |
| 46 | KOTE: Korean Online That-gul Emotions (Jeon, Lee & Kim) | 2022 | GitHub / arXiv 2205.05300 | KO/EN | https://github.com/searle-j/KOTE | ~50k comments, 43 emotions + none; MIT |

### Inferences
- **Licensing summary for productisation:**
  - **Safe with attribution:** NIKL dictionaries (CC BY-SA 2.0 KR; mind ShareAlike and the exclusion of cited examples), the KOGL Type 1 learner list, KOTE (MIT).
  - **Restricted:** 모두의 말뭉치 (only as approved in the signed 약정서), KoBALT data (BY-NC-ND).
  - **Unclear:** KorLex, KnuSentiLex, LEX-ICON, 한국어기초사전 (not verified).
- **Evidence quality:** the strongest Korean-specific LLM evidence is KatFish (ACL 2025), KoBALT and HAE-RAE. Everything on word-bank efficacy is transferred from non-Korean or non-fiction settings.

### Gaps
- Items not opened or only partly verified: rows 20 (bibliographic only) and 32 (bibliographic only); venue claims flagged as "listing not opened" in rows 25 and 35; and the Ewha colour-space study, Bestgen (2025), the Harvard ideophone thesis and the 한국어기초사전 licence, which are excluded from findings.
- No Korean-language KCI study found on ChatGPT or LLM use of 의성어·의태어 or literary vocabulary in fiction. Searches for "의성어 의태어 대규모 언어모델", "ChatGPT 소설 창작 한국어 문체 어휘 분석" and "AI 생성 소설 어휘" returned no relevant primary studies.
