# Figurative Language in LLM Fiction Generation (Metaphor, Simile, Personification, Idiom, Proverb), with a Korean Focus

Scope: how LLMs generate and understand figurative language; what makes LLM figures fresh/apt vs. clichéd; which prompting patterns help; how LLMs handle Korean 관용구·속담·사자성어·은유·직유·의인법; what Korean linguistic/literary scholarship offers as prompt guidance; failure modes for a style linter.
Research date: 2026-10-17. Notation: "Implication (inference)" marks my own reasoning about prompt design. Everything else carries an inline source I opened. Anything not opened is marked **[UNVERIFIED]**.

---

## 1. What research measures LLM metaphor/simile generation quality, novelty vs. cliché, and aptness?

### Takeaway
Figurative-generation research has moved from fine-tuned seq2seq systems (SCOPE, MERMAID, CM-BART, 2020–21) to LLM studies (2023–25). The evaluation axes are fairly stable: **metaphoricity, novelty/originality (often measured by how rare the vehicle is), aptness/relevance (how well tenor and vehicle are linked), coherence, sentiment consistency, fluency, and "would a writer use it"**. The headline result repeats across studies: LLM figures are fluent and recognizably figurative but **more generic and less original/inspiring than expert human figures** when experts judge them. Lay readers and automated scorers, by contrast, often rate LLM output as good or better. Several studies also show that **LLMs interpret figurative language much better than they generate fresh figures**.

### Cited Findings

#### 1a. Generation systems with human evaluation (foundational, 2020–2021)
- **SCOPE simile generation (Chakrabarty, Muresan & Peng, EMNLP 2020).** Method: Reddit similes were converted to literal sentences using commonsense *properties*, then BART was fine-tuned on literal→simile pairs. Results: about **88% of generated similes did not share properties with the training data** (novelty). In pairwise human judgments the model beat **two literary experts 37% of the time** (experts averaged 32.6% and 41.3%) and beat three baselines 71% of the time. Swapping literal sentences for its similes made machine-generated **stories more evocative and better received** — [arXiv 2009.08942](https://arxiv.org/abs/2009.08942)
  - Implication (inference): making the *property* (ground) explicit is what enables novel vehicles. A prompt should name the property first ("말라붙은 → ?"), not ask for "a simile for X".
- **MERMAID (Chakrabarty, Zhang, Muresan & Peng, NAACL 2021).** Method: literal→metaphoric verb replacement, trained on a parallel corpus built from Gutenberg Poetry with masked LM + commonsense inference, motivated by the "theoretically-grounded connection between metaphors and symbols". A metaphor discriminator guides decoding. Results: humans preferred its metaphors over three baselines **66%** of the time on average. Poems augmented with its metaphors were preferred **68%** of the time over versions without them — [arXiv 2103.06779](https://arxiv.org/abs/2103.06779); [ACL Anthology 2021.naacl-main.336](https://aclanthology.org/2021.naacl-main.336)
  - Implication (inference): **verbs are the cheapest place for a fresh figure** ("햇살이 창틀을 *갉았다*"), and discriminator-style filtering (generate → judge metaphoricity) improves quality.
- **Metaphor Generation with Conceptual Mappings (Stowe, Chakrabarty, Peng, Muresan & Gurevych, ACL-IJCNLP 2021, pp. 6724–6736).** Method: generation controlled by conceptual-metaphor-theory **source→target domain mappings**. CM-Lex is unsupervised (FrameNet embeddings). CM-BART is controlled seq2seq. Results: CM-Lex was competitive with deep-learning baselines, and **CM-BART outperformed all other models in automatic and human evaluation** (metaphoricity and presence of the intended conceptual metaphor) — [ACL Anthology 2021.acl-long.524](https://aclanthology.org/2021.acl-long.524)
  - Implication (inference): **explicitly choosing the source domain** gives control and coherence. In a prompt: "이 장면의 지배 은유: 분노 = 끓는 솥 (요리 영역에서만 확장)".
- **Writing Polishment with Simile (WPS) (Zhang et al., AAAI 2021).** Task: insert similes into *existing context* as a writer would. Dataset: about **5 million Chinese similes with context**. Model: Locate&Gen, which first decides *where* a simile belongs and then generates a location-specific simile — [arXiv 2012.08117](https://arxiv.org/abs/2012.08117)
  - Implication (inference): treat figurative language as a **revision pass over a draft** in two decisions: *where* (which sentence deserves a figure) and *what* (a context-specific figure). Do not sprinkle figures during first-draft generation.

#### 1b. Evaluation frameworks and automatic metrics
- **HAUSER (He et al., ACL 2023).** Holistic simile evaluation with **five criteria across three perspectives**:
  - *Quality:* relevance (topic–vehicle), logical consistency, and sentiment consistency with the literal sentence.
  - *Creativity:* scored as −log(1 + average vehicle frequency) in a simile corpus/MAPS-KB, so **rarer vehicles score higher**.
  - *Informativeness:* how much content the vehicle carries.
  - Correlation with human ratings (Pearson/Spearman): creativity **0.592/0.645**, versus the best prior diversity metrics Dist2 0.319 and Dist3 0.379. Quality **0.320/0.292**, versus BLEU2 0.255 and METEOR 0.233. Informativeness **0.798/0.882**.
  - [ar5iv 2306.07554](https://ar5iv.labs.arxiv.org/html/2306.07554); [ACL Anthology 2023.acl-long.702](https://aclanthology.org/2023.acl-long.702)
  - Implication (inference): a Korean style linter can approximate "cliché" by **vehicle frequency** in a reference corpus of Korean similes. Extract nouns governed by -처럼/-같이/-듯(이)/같은 and penalize high-frequency vehicles for a given tenor. **Sentiment consistency** is a separate check: does the vehicle's affect match the scene?
- **Automatic Scoring of Metaphor Creativity with LLMs (DiStefano, Patterson & Beaty, *Creativity Research Journal* 37(4):555–569, 2025; online 2024).** Data: **4,589 human metaphor responses from 1,546 participants** with human creativity ratings. Fine-tuned RoBERTa reached **r = .72** and GPT-2 **r = .70** against held-out human ratings, versus **r = .42 for semantic distance**. On unseen prompts: r = .68 and .63 — [Penn State record](https://pure.psu.edu/en/publications/automatic-scoring-of-metaphor-creativity-with-large-language-mode/); [DOI](https://doi.org/10.1080/10400419.2024.2326343)
  - Implication (inference): raw semantic distance (embedding distance between tenor and vehicle) is a **weak proxy for creativity**. A linter's novelty score needs calibration against human judgments, for example Korean writers' ratings of a few hundred figures.
- **FLUTE (Chakrabarty, Saakyan, Ghosh & Muresan, EMNLP 2022, pp. 7139–7159).** About **9,000 figurative NLI instances with textual explanations** covering sarcasm, simile, metaphor, and idiom, built with GPT-3 plus crowd workers and expert annotators. The paper argues that explanations are needed because figurative NLI benchmarks contain spurious correlations — [ACL Anthology 2022.emnlp-main.481](https://aclanthology.org/2022.emnlp-main.481)
  - Implication (inference): an LLM judge in the linter should **explain why a figure works or fails** (its ground and entailments), not just output a score.
- **Metaphorian's evaluation rubric (Kim, Suh, Chilton & Xia, DIS 2023).** Vehicles were rated on originality, metaphoricity, relevance of sub-vehicles, coherence of sub-vehicles, willingness to adopt, and inspirational effect. Mappings were rated on fluency, precision, relatedness, willingness to adopt, and inspirational effect — [paper PDF](https://hci.ucsd.edu/papers/metaphorian.pdf); [DOI](https://doi.org/10.1145/3563657.3595996)

#### 1c. LLM vs. human figurative output (2023–2025)
- **Metaphorian (DIS 2023): GPT-3 vs. professional writers (extended metaphors).** Share of figures rated willing-to-use / inspirational:
  - **vehicles:** LLM **0.23 / 0.10** vs. human **0.58 / 0.71**
  - **mappings:** LLM **0.31 / 0.15** vs. human **0.58 / 0.71**
  - Humans were significantly more **original** (median 4 vs. 3, p < .005) and more **metaphorical** (4 vs. 3, p < .0001). There was no significant difference in fluency, where the LLM was slightly higher.
  - [paper PDF](https://hci.ucsd.edu/papers/metaphorian.pdf)
- **Multimodal CoT metaphor generation (Lugli & Strapparava, CLiC-it 2024).** Model: LLaVA-1.5 13B. **92%** of outputs were judged metaphors by a majority of five MTurk workers. As metaphoricity and appeal rose, **familiarity fell**, so the more appealing figures were also the less familiar ones. BERTScore against human metaphors was only F1 = 0.42. The authors write that human metaphors "often reference real-world examples, including real people and events", while generated ones "tend to be more generic and less nuanced" — [paper PDF](https://aclanthology.org/2024.clicit-1.62.pdf)
- **Porter & Machery, *Scientific Reports* 14:26133 (2024).** Non-expert readers identified ChatGPT-3.5 poems at **46.6%** (below chance), with n = 1,634. A second study (n = 696) rated AI poems higher on **13 of 14 qualities, including imagery**. The exception was **"original"**, which showed no significant difference. AI poems "rarely use complex metaphors" and are "more accessible". The authors suggest readers misread the complexity of human poems as incoherence — [PMC11564748](https://pmc.ncbi.nlm.nih.gov/articles/PMC11564748)
- **Dinu, Florescu & Resceanu (KES 2025; *Procedia CS* 270:1292–1301).** 24 humans vs. 24 LLMs on an 8-task linguistic creativity test (derivation, compounding, metaphorical use), scored automatically with OCSAI. **LLMs outscored humans on originality, elaboration, and flexibility** and won 6 of 8 tasks, with only minor differences in answer uniqueness. Humans leaned toward E-creativity (extending existing forms) and LLMs toward F-creativity (fixed forms) — [arXiv 2507.12039](https://arxiv.org/abs/2507.12039)
- **Conflict to note:** **expert ratings** (Metaphorian) and human-reference comparison (Lugli) put humans ahead on originality and specificity. **Automated scoring** (Dinu, OCSAI) and **lay readers** (Porter & Machery) put LLM output level or ahead. The judging population drives the verdict.

#### 1d. LLM interpretation of figurative language (relevant to the "reads the story" half)
- **Ichien, Stamenković & Holyoak (arXiv 2023; rev. Jan 2024).** GPT-4 interpreted novel literary metaphors from Serbian poetry (translated, with no sign it had seen them). Blind judges rated its interpretations **superior to college students'**. It showed human-like sensitivity to the Gricean cooperative principle on reversed metaphors, and a critic rated its readings of novel English poems "excellent or good". Journal publication not verified — [arXiv 2308.01497](https://arxiv.org/abs/2308.01497)
- **Sanchez-Bayona & Agerri (2025)** evaluated metaphor interpretation across several NLI/QA datasets. LLM performance was driven more by **lexical overlap and sentence length** than by metaphoricity, so apparent metaphor understanding partly reflects surface features and in-context learning. Listed in ACL Anthology as Findings of ACL 2025 (per search listing) — [arXiv 2507.15357](https://arxiv.org/abs/2507.15357)
- **Kabra et al. (Findings of ACL 2023)** built a figurative-language inference dataset in Hindi, Indonesian, Javanese, Kannada, Sundanese, Swahili, and Yoruba. **All languages scored significantly below English**, the gap tracked pre-training data size, and each language relied on **its own cultural/regional concepts** — [arXiv 2305.16171](https://arxiv.org/abs/2305.16171)

### Inferences
- Freshness can be operationalized as (a) **vehicle rarity** relative to a corpus (HAUSER) plus (b) **calibrated human-like creativity scoring** (DiStefano). Aptness can be operationalized as tenor–vehicle relevance, logical consistency, and sentiment consistency (HAUSER), plus "is the ground statable in one clause?" (Metaphorian's mapping criteria).
- LLMs read figures well but write generic ones. The prompt's job is therefore not to *teach* the model what a metaphor is but to **constrain the search**: ground first, domain control, distance, specificity, density.
- Because lay readers prefer accessible figures (Porter & Machery), the **web-novel** mode (fast serial reading) and the **literary/print** mode should probably get different figurative settings: clarity-first versus originality-first. This is an inference; no Korean reader study was found.

### Gaps
- No 2024–2026 study was found that measures *LLM simile novelty in narrative fiction* (as opposed to isolated sentences, poems, or science writing). Simile work after HAUSER (2023) is thin.
- Studies used GPT-3, GPT-3.5, LLaVA, and older models. No controlled expert evaluation of 2026 frontier models' figurative originality was found.

---

## 2. Which prompting methods produce fresher, more apt figurative language?

### Takeaway
The best-supported levers are:
1. **Ground/property-first generation**, where the shared feature is chosen before the vehicle (CMDAG, SCOPE, Ge & Parikh).
2. **Explicit source-domain control** (Stowe et al.) and **consistent domain extension** (Metaphorian).
3. **Elaborate-then-figure two-step prompting**: describe the literal scene concretely, then metaphorize (I Spy a Metaphor; Lugli & Strapparava).
4. **Chaining plus appended writer requirements** (distance, originality, coherence), which significantly raised originality and metaphoricity (Metaphorian).
5. **Generate many candidates, then select**, with explicit typicality awareness (Verbalized Sampling, Creative Beam Search, Tang et al.).
6. **Post-hoc suppression of over-used patterns** (Antislop).

Few-shot exemplars help generation quality but can cause stylistic overfitting. Chain-of-thought helps *generation* but **hurt Korean idiom *recognition*** (KIM Bench, §3).

### Cited Findings

#### 2a. Ground / source-domain control
- **CMDAG (Shao et al., LREC-COLING 2024).** A Chinese corpus of about **28K sentences** from poems, prose, and song lyrics, annotated with **tenor–vehicle–ground** triples. It also covers **similes, personification**, juxtaposition, and hyperbole. Generation uses the **ground as chain-of-thought input**: either infer the ground from tenor + vehicle, or infer a plausible vehicle from tenor + ground. Belle, Baichuan, and Chinese-alpaca-33B produced creative, fluent metaphors more often with dataset samples. No numbers appear on the abstract page — [arXiv 2402.13145](https://arxiv.org/abs/2402.13145); [ACL Anthology 2024.lrec-main.298](https://aclanthology.org/2024.lrec-main.298)
- **Ge & Parikh, "Visual Conceptual Blending" (2021; ICCC 2021 per search listing).** A two-prompt blend:
  - "the moon is like a [MASK]" yields *ghost*; "the ghost has the property of [MASK]" yields *dead*; the template gives "a moon that is dead like a ghost". Another example from the paper: "the moon … sliced like an orange".
  - For concept association on 66,442 simile pairs, precision@1000 was **GPT 66.38 vs. ConceptNet 5.90**, so LMs beat the knowledge base at finding related vehicles.
  - [ar5iv 2106.14127](https://ar5iv.labs.arxiv.org/html/2106.14127)
- **Creative Blends (Sun et al., 2025; CHI 2025 per search listing).** Pipeline: abstract concept → ConceptNet top-50 related objects → GPT-3.5 picks the fitting ones with the template "{Concept} is like a/an {Object}" **plus a stated reasoning for each link** → attributes (shape, color …) → blend. In a within-subject study (N = 24), metaphoricity was **5.66 vs. 5.05** for a ChatGPT + search baseline (p = .032, d = .558) — [arXiv 2502.16062](https://arxiv.org/html/2502.16062v1)
- **Stowe et al. 2021 and SCOPE 2020** (see §1a): domain-mapping control and property-conditioned generation beat unconstrained baselines — [ACL 2021](https://aclanthology.org/2021.acl-long.524); [arXiv 2009.08942](https://arxiv.org/abs/2009.08942)

#### 2b. Elaborate-then-generate (CoT) and prompt chaining
- **I Spy a Metaphor (Chakrabarty et al., Findings of ACL 2023).** GPT-3 (davinci-002) used **CoT to write a "visual elaboration"** that made explicit the metaphor's **implicit meaning and relevant objects**, which then went to a diffusion model. Human-AI collaboration produced **6,476 visual metaphors for 1,540 linguistic metaphors**, and professional illustrators judged the approach promising — [arXiv 2305.14724](https://arxiv.org/abs/2305.14724)
- **Lugli & Strapparava (CLiC-it 2024).** Two-step CoT: first caption the *source-domain* and *target-domain* images, then generate the metaphor. For ENVIRONMENTAL HARM IS PHYSICAL INJURY:
  - with CoT the model produced *"wounded environment"*.
  - **without CoT** it produced "The sun shines brightly over the barren landscape, illuminating the industrial complex **like a beacon of hope**", which the authors note "does not involve a metaphor and fails to consider the images of both source and target domains". That is a textbook cliché and a sentiment mismatch.
  - [paper PDF](https://aclanthology.org/2024.clicit-1.62.pdf)
- **Metaphorian (DIS 2023)** split extended-metaphor writing into a **four-step chain**: main vehicle → main mapping ("because …") → sub-vehicle (conditioned on the main vehicle) → sub-mapping. A single combined prompt gave only one reason for all sub-metaphors ("solar system is like a giant family … sun = father, planets = children …"), which motivated the chaining — [paper PDF](https://hci.ucsd.edu/papers/metaphorian.pdf)

#### 2c. Generate many candidates, diversify, then select
- **Verbalized Sampling (Zhang, Yu, Chong, Sicilia, Tomz, Manning & Shi, arXiv 2025).** Attributes mode collapse to **typicality bias in preference data**: annotators favor familiar text. The training-free fix asks the model to **output several candidates with their probabilities** (e.g., "5 jokes about coffee and their corresponding probabilities"). Creative-writing diversity rose **1.6–2.1×** over direct prompting, with larger gains for more capable models and no reported loss of factual accuracy or safety — [arXiv 2510.01171](https://arxiv.org/abs/2510.01171)
- **Creative Beam Search (Franceschelli & Musolesi, ICCC 2024).** Diverse Beam Search candidates plus an **LLM-as-a-judge** validation step beat standard sampling in a qualitative study, and the **validation step was necessary**. No percentages on the abstract page — [arXiv 2405.00099](https://arxiv.org/abs/2405.00099)
- **Tang, Song, Qin & Yan, "Creative and Context-Aware Translation of East Asian Idioms with GPT-4" (2024).** Coverage: **Chinese 4,310, Japanese 2,440, Korean 2,316 idioms**. Strategies included *multiple candidates* (Diversity Explicit/Dialog), *creativity-oriented* prompts (Zero-Shot Creatively, Analogy Creative …), and **Context Explicit (genre: news, romance novel, conversation, history book)**. Some strategies were Pareto-better than Google/DeepL on faithfulness × creativity (GPT-3.5-judged, 1–5). On the 500-idiom extension:
  - "Zero-Shot Creatively" scored **4.27 faithfulness / 4.08 creativity**.
  - "**Analogy Creative**" had the **highest creativity (4.27) but the lowest faithfulness (3.01)**.
  - "Few-Shot" had the highest faithfulness (4.31).
  - [arXiv 2410.00988](https://arxiv.org/html/2410.00988v1); ACL Anthology lists it at WAT 2024 and Findings of EMNLP 2024 (conflicting listings) — [ACL Anthology 2024.wat-1.1](https://aclanthology.org/2024.wat-1.1)

#### 2d. Exemplars and appended requirement lists
- **Metaphorian (DIS 2023).** Professional writers rated 810 metaphors across 0-shot, 10-shot, and 10-shot-plus-requirements conditions.
  - **10-shot** (the best writer-made exemplars) improved vehicle *willingness-to-use* **2×** and *inspirational effect* **1.2×** over 0-shot.
  - **Appending the writers' requirements** as natural-language rules on top of 10-shot significantly improved **originality** (p < .05), **metaphoricity** (p < .005), and **coherence of sub-metaphors** (p < .05). Example: "superstring theory like *a web*" (10-shot) became "*tapestry*", "*music*" (augmented).
  - The requirements were: vehicle **not in the same domain** as the tenor; **original** vehicles; **all sub-vehicles in the same domain**; every mapping must **closely connect** tenor and vehicle; mappings must be **precise**.
  - [paper PDF](https://hci.ucsd.edu/papers/metaphorian.pdf)
- **Counter-evidence on few-shot:**
  - GPT-3.5's accuracy on Korean somatosensory figurative expressions **fell from 92.59% (zero-shot) to 74.07% (1-shot) and 81.48% (2-shot)** — [KCI ART003104112](https://www.kci.go.kr/kciportal/ci/sereArticleSearch/ciSereArtiView.kci?sereArticleSearchBean.artiId=ART003104112)
  - On the Vietnamese idiom benchmark VIVID, "few-shot prompting does not universally improve performance, with **GPT-4o exhibiting degradation due to stylistic overfitting**" — [LREC 2026 PDF](https://aclanthology.org/2026.lrec-1.422.pdf)

#### 2e. Context and character grounding ("mind style")
- **Semino & Swindlehurst, *Style* 30(1):143–166 (1996).** Consistent **metaphorical patterns project a narrator's idiosyncratic "mind style"** and chart his development through Kesey's novel. **Conventional metaphors connect to culture; unconventional metaphors connect to individual worldviews** — [Lancaster EPrints](https://eprints.lancs.ac.uk/id/eprint/33485/)
- **Lugli & Strapparava 2024**: human metaphors are grounded in **real-world specifics** (people, events) while LLM metaphors are generic — [paper PDF](https://aclanthology.org/2024.clicit-1.62.pdf)
- **Tang et al. 2024**: specifying **genre/context** (e.g., romance novel) was one of the strategies that produced many high-quality, context-appropriate idiom renderings — [arXiv 2410.00988](https://arxiv.org/html/2410.00988v1)

### Inferences
- **Order of operations matters.** Every successful method makes the model commit to (1) what is being described, (2) which property to transfer, and (3) which source domain, *before* surface wording. Unconstrained "add a vivid simile" prompts invite the modal (most typical) vehicle, which is the cliché.
- **Distance needs an upper and a lower bound.** Too close gives a literal comparison (Metaphorian's "volcano is like a mountain"). Too far, or pushed by "be creative", gives an off-target figure (Tang's Analogy Creative: creativity 4.27 but faithfulness 3.01). Prompts should ask for "다른 영역에서 왔지만 공통점이 한 문장으로 설명되는" vehicles.
- **Character-grounded source domains** are the most practical route to both freshness and aptness in fiction. The POV character's occupation, era, region, body, and obsessions supply vehicles that are rare in the corpus (fresh, per HAUSER) yet justified by the story (apt, per mind style). This is supported by mind-style theory plus Lugli's specificity finding. No LLM experiment testing POV-grounded prompts directly was found.
- **CoT should be used for *generation scaffolding* (ground, domain, scene elaboration), kept internal, and not used to "reason out" idiom choice** (see KIM Bench in §3).
- **Few-shot:** use exemplars to demonstrate *structure* (ground → vehicle, density, restraint), rotate them, and forbid reusing their vehicles, given the stylistic-overfitting evidence.

### Gaps
- **[UNVERIFIED]** Summers-Stay, Lukin & Voss (2023), "Brainstorm, then select: a generative language model improves its creativity score". I found it only as a bibliography entry in other papers and did not open it. The brainstorm-then-select pattern is supported here instead by Verbalized Sampling, Creative Beam Search, and Tang et al.
- No study was found that tests **metaphor-density instructions** ("at most one figure per paragraph") on LLM output quality.
- No text-only benchmark of **conceptual-blending prompts** for narrative prose was found. The evidence is from visual or design settings (Ge & Parikh; Creative Blends).

---

## 3. How well do LLMs understand and generate Korean figurative language (관용구, 속담, 사자성어, 은유, 직유, 의인법)?

### Takeaway
Korean figurative-language evaluation is almost entirely about **understanding idioms**: 관용구 matching, 사자성어 cloze, Hanja-based idioms, and somatosensory figurative expressions. Frontier and Korean-native models now approach native speakers on **definition matching** (best 90.38% vs. native 91.3%). They fall well short when they must **pick the right idiom for a context** (best 69.12% on news-context 사자성어 cloze) or interpret **Korean poetry** (best 45.7%). They also confuse **Korea-specific meanings with Chinese readings** of Hanja idioms. **CoT reduced Korean idiom accuracy for 4 of 6 models.** I found **no Korean benchmark or Korean-language paper on LLM *generation* of 은유/직유/의인법**, which is a clear gap.

### Cited Findings

#### 3a. Korean idiom / 사자성어 benchmarks
- **KIM Bench (Wang, Park & Kim, Yonsei; RANLP 2025, pp. 1341–1351).**
  - Data: **1,175 commonly used Korean idioms** with definitions, manually collected from **표준국어대사전 and 우리말샘** and expert-reviewed for current prevalence. Examples: 누워서 떡 먹기 ("very easy"); 가슴 불타다 ("passion boils and spreads in one's heart").
  - Format: 5-way definition matching, with distractors chosen at cosine-similarity bands from 0.50 to 0.90.
  - Direct-prompting accuracy:
    - **HyperCLOVA X 90.38%**
    - o1 89.02%
    - **GPT-4o 87.32%**
    - o3-mini 86.72%
    - GPT-4o-mini 80.85%
    - o1-mini 70.55%
    - **Native speakers 91.3%**
  - **CoT lowered accuracy for 4 of 6 models:** HyperCLOVA X 90.38 → **83.83**; o1 89.02 → 83.66; o3-mini 86.72 → **76.68**; GPT-4o 87.32 → 86.47. Only GPT-4o-mini (80.85 → 81.45) and o1-mini (70.55 → 73.11) improved. The authors hypothesize idiom matching depends on "cultural background knowledge, contextual appropriateness, and quick recognition of fixed expressions rather than explicit multi-step logical deductions".
  - Error analysis (GPT-4o / HyperCLOVA X):
    - "Complete Lack of Understanding" 66.7% / 68.1%
    - **"Surface-Level Idiomatic Misinterpretation" 29% / 23%**
    - Breakdown in Logical Process 1.2% / 7.1%
    - Nuance Differentiation 3.1% / 0.9%
  - [ACL Anthology PDF](https://aclanthology.org/2025.ranlp-1.156.pdf); [landing page](https://aclanthology.org/2025.ranlp-1.156/)
- **KULTURE Bench (Wang, Yeo, Lim & Kim; PACLIC 38, 2024).**
  - Components: **KorID**, **1,631** four-character idiom (사자성어) cloze items embedded in news passages, with the vocabulary drawn from two 사자성어 dictionaries (5,372 unique idioms) and 5 options per item. **KorPD**, 453 poetry cloze items. **KorCND**, 1,500 cultural-news items.
  - Results: the native Korean model **Clova X** performed best overall, followed by GPT-4. Yet even Clova X reached "only a **69.12%** accuracy on the dataset containing idiomatic expressions". The best poetry result was "a mere **45.7%** accuracy by GPT-4".
  - [arXiv 2412.07251](https://arxiv.org/abs/2412.07251) (PDF read in full)
- **"Knowing the Words, Missing the Meaning" (Lee, Do, Kim & Oh, Kyungpook Nat'l Univ.; *Applied Sciences* 15(13):7561, 2025).** A benchmark modeled on the **Korean Hanja Proficiency Test** with four categories: **four-character idioms**, synonyms, antonyms, and homophones. GPT-4o had the best accuracy and explanation quality. Models struggled with **fine-grained character nuances and with meanings unique to Korean usage rather than standard Chinese interpretations**, a "gap in LLMs' understanding of Korea-specific Hanja culture". Read via the abstract page. Full-text numbers not retrieved (the MDPI page returned 403) — [KNU research portal](https://research.knu.ac.kr/en/publications/knowing-the-words-missing-the-meaning-evaluating-llms-cultural-un/); [DOI](https://doi.org/10.3390/app15137561)

#### 3b. Broad Korean benchmarks: what they do and don't cover for figurative language
- **CLIcK (Kim, Suk, Oh, et al.; LREC-COLING 2024).**
  - Size and sources: **1,995 QA** from Korean exams and textbooks in 11 categories.
  - Categories: Culture 1,345 (Society 309, Tradition 222, History 280, Law 219, Politics 84, Economy 59, Geography 131, Pop culture 41) and Language 650 (Textual 285, Functional 133, Grammar 232).
  - **No category is defined for 속담/관용어/사자성어.**
  - Results: on **Functional** knowledge, GPT-3.5 and Claude-2 scored only **about 33%**, and open models scored 10–50% overall. Model size (p = .57) and extra Korean training (p = .59) had no significant effect.
  - [arXiv 2403.06412](https://arxiv.org/html/2403.06412v4)
- **HAE-RAE Bench (Son et al.; LREC-COLING 2024):** six tasks across four domains (vocabulary, history, general knowledge, reading comprehension). The abstract mentions no figurative or idiom task — [arXiv 2309.02706](https://arxiv.org/abs/2309.02706)
- **KMMLU (Son et al., 2024):** 35,030 expert-level multiple-choice questions across 45 subjects from Korean exams. Best public model 50.5%; GPT-4 and HyperCLOVA X under 60%. Exam-knowledge focused, with no figurative component described. **Not useful for figurative evaluation** — [arXiv 2402.11548](https://arxiv.org/abs/2402.11548)

#### 3c. Korean-language (KCI) studies on LLMs and figurative expressions
- **김해진·이동훈·임승락·김민호 (2024), 「한국어 체성감각 비유 표현에 대한 사전학습 언어모델의 이해 능력 분석」, *한국정보통신학회논문지* 28(7):787–794** (Korean paper). The authors built **27 somatosensory figurative expressions** (bodily-sensation-based), each with a literal-sense and a figurative-sense sentence pair. Zero-shot: **GPT-4 and GPT-4o 100%**, GPT-3.5 92.59%. GPT-3.5 one-shot 74.07% and two-shot 81.48%, both below zero-shot. Framed as computational support for embodied-cognition theory — [KCI ART003104112](https://www.kci.go.kr/kciportal/ci/sereArticleSearch/ciSereArtiView.kci?sereArticleSearchBean.artiId=ART003104112); [DOI 10.6109/jkiice.2024.28.7.787](https://doi.org/10.6109/jkiice.2024.28.7.787)
- **강예지 외 14인 (2024), 「인공지능의 한국어 능력 종합 평가를 위한 벤치마크」, *한글* 85(3):679–715** (Korean paper). Six domains, seven metrics, six Korean/multilingual LLMs, seven human raters, and GPT-4 automatic evaluation checked against the human ratings. **The abstract does not say whether idioms, proverbs, or figurative language are included** — [KCI ART003123750](https://www.kci.go.kr/kciportal/ci/sereArticleSearch/ciSereArtiView.kci?sereArticleSearchBean.artiId=ART003123750)

#### 3d. Cross-lingual analogues that bear on Korean
- **Tang et al. 2024** included **2,316 Korean idioms**. In one Korean example, **Google rendered the key phrase as "temperamental" and DeepL as "dummy"** (faithfulness 2.2 and 2.0). GPT-4 also judged **50 GPT-4-generated, idiom-like Chinese strings ("Plausible Chinese") to be real idioms**, which shows the model can fabricate plausible-looking four-character idioms — [arXiv 2410.00988](https://arxiv.org/html/2410.00988v1)
- **VIVID (Vietnamese, LREC 2026)** is analogous to Korean because of its Sino-Vietnamese vocabulary and folk knowledge. On 1,636 idioms and proverbs, top models scored under 50% correctness. Failures were "**literal over-interpretation, lexical gaps, and pragmatic flattening**". It cites KorID as the Korean counterpart — [LREC 2026 PDF](https://aclanthology.org/2026.lrec-1.422.pdf)
- **MIDI (Almheiri et al., arXiv 2026):** 18 languages. In every resource tier, "**literal readings are considerably harder to interpret than figurative ones**", and conversational context helps but does not close the gaps. Korean inclusion not confirmed on the abstract page — [arXiv 2606.02147](https://arxiv.org/abs/2606.02147)

### Inferences
- **Knowing definitions ≠ using idioms aptly.** Definition matching is about 87–90% (KIM), while context cloze is at most 69% (KorID) and poetry at most 46% (KorPD). For a fiction generator this is the important gap: the model may *know* 사자성어 meanings yet **drop one into the wrong context or register**. A generator should therefore (a) prefer native 관용구 and plain imagery over 사자성어 in narration unless the voice calls for erudition, and (b) have the linter **verify every 사자성어/속담 against a dictionary** (표준국어대사전/우리말샘, the sources KIM Bench used) for existence *and* Korean meaning, because of the "Plausible Chinese" fabrication and the Korea-vs-Chinese meaning findings.
- **Common embodied figures are safe; rare or culture-loaded ones are risky.** GPT-4-class models handle frequent somatosensory figurative expressions near-perfectly (27/27). Risk concentrates in rare idioms, Hanja nuance, poetic allusion, and literal/figurative double readings.
- **Don't make the model "reason out" idiom choice step by step.** KIM Bench found CoT hurt even strong models. Supply candidate idioms with dictionary glosses and usage examples instead (retrieval), and let the model choose.
- The 2024 CLIcK figures come from GPT-3.5/Claude-2-era models. Current models are likely stronger, but I found **no 2025–2026 re-evaluation** on CLIcK's Functional category.

### Gaps
- **No Korean metaphor/simile/personification *generation* dataset, benchmark, or paper was found**, despite Korean searches (은유 생성 언어모델, 비유 표현 생성, 직유 생성). The searches returned metaphor *detection* work (e.g., MelBERT) or unrelated results.
- **No 속담-specific LLM benchmark** was found. KIM Bench covers 관용구 (its sources contain proverbs, but the dataset is framed as idioms), and KorID covers 사자성어.
- Not opened: 이창수, 「챗GPT, 파파고, 인간 번역가 간의 한영 문학번역 차이점」 and 방영풍·서연주, 「『대장금』 소설의 한중 번역을 중심으로 ChatGPT를 활용한 문화소 번역연구」. Both surfaced only as National Assembly Library listings, so their findings are unknown.

---

## 4. What do Korean literary and linguistic studies say about characteristic metaphor and imagery that can become prompt guidance?

### Takeaway
Korean cognitive-linguistic work stresses that Korean **conceptualizes emotion through internal organs and the body** more than English does (간, 애, 속, 가슴, 배 …), and that body-part idioms encode social values such as community, hierarchy, and face (체면). Spoken Korean freely **varies and updates idioms**. Korean modern-poetry scholarship emphasizes **multisensory (복합감각) "pictorial" imagery** (1930s modernists), **nature imagery shaped by East-Asian landscape aesthetics and 여백** (restraint that conveys the unsaid), and **flowers as brief beauty and transience**. Together these suggest prompt guidance: favor somatic and sensory Korean source domains, restraint, and implication, and treat canonical images (진달래, 국화, 산) as loaded allusions rather than neutral decoration. **Fiction-specific (소설/웹소설) scholarship on figurative style was not found.**

### Cited Findings

#### 4a. Emotion and body-part metaphors
- **김도훈·최은실 (2011), 「한국어-영어 감정 은유의 개념화 과정 및 한영 번역」, *영어영문학21* 24(3):99–120** (Korean paper). Covers happiness, anger, and sadness. Both languages use source domains such as **"a fluid in a container", stuff, and thread**, plus physiological responses. Compared with English, **Korean tends to use various internal organs (내장 기관) to express emotion**. The paper gives four translation strategies for emotion metaphors: same metaphor → same; metaphor → different metaphor; metaphor → non-metaphor; zero → metaphor (adding one) — [KCI ART001590822](http://www.kci.go.kr/kciportal/landing/article.kci?arti_id=ART001590822); [DOI](https://www.doi.org/10.35771/ENGDOI.2011.24.3.005)
- **Turker (2013), "A corpus-based approach to emotion metaphors in Korean: anger, happiness, and sadness", *Review of Cognitive Linguistics* 11(1):73–144.** A corpus study separating **primary (universal) from complex (cultural) emotion metaphors** in Korean. It finds a **positive correlation between frequency and productivity** of emotion metaphors — [ASU record](https://asu.elsevierpure.com/en/publications/a-corpus-based-approach-to-emotion-metaphors-in-korean-a-case-stu); [DOI 10.1075/rcl.11.1.03tur](https://doi.org/10.1075/rcl.11.1.03tur)
- **Kim-De Crescenzo, H. G. (2018), 「신체부위 은유와 한국어 속담 및 관용어 교육」 (EAKLE-7 conference abstract, Korean).** Body-part-coded proverbs and idioms are metaphorical and carry socio-cultural values. They fall under **community** themes (solidarity, hierarchy/rank, **체면**) and **individual** themes (self-interest, attitude/personality, self-humiliation, **caution in speech**). Abstract only — [PDF](https://blogs.helsinki.fi/eakle-7/files/2018/04/신체부위-은유와-한국어-속담-및-관용어-교육.pdf)
- The KIM Bench examples (가슴 불타다; "'간 붓다'를 속되게 이르는 말" → 지나치게 대담해지다) illustrate the same somatic pattern in dictionary idioms — [RANLP 2025 PDF](https://aclanthology.org/2025.ranlp-1.156.pdf)

#### 4b. Idiom pragmatics: variation in real usage
- **김선영 (2014), 「한국어 관용표현의 화용적 특징 연구 - 변이 양상을 중심으로 -」, *한국언어문화학* 11(3):23–46** (Korean paper). Compared idioms in TV drama scripts with their 표준국어대사전 base forms. Speakers:
  - **create their own variants** for emphasis instead of keeping fixed forms
  - **shorten** long idioms to their core for ease and economy
  - **replace outdated words with current ones** or combine idioms with neologisms, more freely in speech than in writing
  - prefer **colloquial or slangy** substitutes
  - [KCI ART001948243](https://www.kci.go.kr/kciportal/landing/article.kci?arti_id=ART001948243); [DOI 10.15652/ink.2014.11.3.023](https://doi.org/10.15652/ink.2014.11.3.023)

#### 4c. Poetry imagery traditions (sensory, nature, flowers, 여백)
- **윤의섭 (2008), 「감각의 복합성과 모더니즘 시의 '회화성' 연구 - 1930년대 김기림·김광균·정지용 시를 중심으로」, *한중인문학연구* 25:223–247** (Korean paper). The 1930s modernists maximized visual "회화성", but their images are in fact **multisensory**: other senses work together so the image conveys its meaning, and **simultaneous multisensory perception creates a three-dimensional, plural spatiality** — [earticle A100553](https://www.earticle.net/Article/A100553)
- **정수자 (2005), 「한국 현대시의 고전적 미의식 연구: 정지용·조지훈·박목월의 山詩를 중심으로」, Ajou Univ. PhD dissertation** (Korean).
  - Mountain poems share worldview and technique with **산수화** (landscape painting), built on **은일** (seclusion), **이상향** (ideal refuge), and **여백**, which expresses "無" through "有" to convey the inexpressible.
  - 정지용: winter mountains and snowscapes as a clean, white, high spiritual space. 조지훈: Zen stillness. 박목월: an imagined mountain in a simple, concise style.
  - [Ajou dCollection](https://dcoll.ajou.ac.kr/dcollection/srch/srchDetail/000000000358)
- **유성호 (2017.4.11), 「[유성호의 문학의 길목] 꽃의 심상과 현대시」, 서울신문** (Korean newspaper column by a literary critic and Hanyang Univ. professor; not peer-reviewed).
  - Flowers suit life-metaphor and are a core image of Korean modern poetry: 김소월 '진달래꽃', 김영랑 '모란', 서정주 '국화', 이육사 '매화 향기', 신동엽 '진달래 산천', among others.
  - The flower's nature is "짧은 절정의 아름다움". **Falling flowers (낙화) metaphorize transience and death**, and in modern history flowers took on historical meaning.
  - [Seoul Shinmun](https://m.seoul.co.kr/news/2017/04/11/20170411030008)

#### 4d. Korean literature and AI translation
- **이형진 (2025), 「생성형 인공지능(AI)의 한국문학 영어번역의 딜레마와 가능성」, *외국문학연구* 100:59–87** (Korean paper). ChatGPT can imitate stylistic diversity through varied prompts and prioritizes fidelity to the source. The author asks whether human translators' omissions and modifications may give readers a **more emotionally resonant** experience than AI's "logical, omission-free" output. This is an argument, and the abstract reports no empirical figures — [Sookmyung ScholarWorks](https://scholarworks.sookmyung.ac.kr/item/af5537be-a3b3-4474-90b6-685ba452d41c); [DOI 10.22344/fls.2025.100.59](https://doi.org/10.22344/fls.2025.100.59)

### Inferences (prompt guidance derived from §4; not experimentally validated on LLMs)
- **Somatic-emotion source domains as a native resource.** Korean readily maps emotion onto organs and inner body states (속이 끓다, 애가 타다, 간이 콩알만 해지다, 가슴이 미어지다 …). Note that these examples come from my general Korean knowledge, while the organ-centrality claim is sourced. Instruct the model to **render emotion through bodily, visceral sensation in Korean idiom space** rather than English-style abstractions ("슬픔의 파도가 그를 덮쳤다" reads as translated).
- **Freshness via extension, not replacement.** Turker's frequency–productivity link suggests frequent Korean emotion metaphors are productive templates. A fresh figure can **extend a conventional Korean somatic metaphor with a story-specific detail** (e.g., the heat metaphor of 속이 끓다 realized through the character's kitchen) instead of importing an exotic vehicle.
- **Dialogue vs. narration.** Per 김선영 2014, characters' speech can **twist, truncate, and modernize idioms** (in character), while narration in literary fiction should use idioms sparingly and keep 사자성어 for erudite voices.
- **Multisensory concreteness and 여백.** Korean poetic tradition values multisensory images and restraint that leaves meaning unstated. That aligns with the LAMP finding that "unnecessary exposition" is a top LLM flaw (§5). Prompt: "비유 뒤에 그 의미를 설명하지 말 것; 한 감각이 아니라 두 감각 이상을 겹칠 것."
- **Canonical images are loaded.** 진달래, 국화, 낙화, 겨울 산, 매화 carry strong intertextual associations. Use them deliberately, as allusion fitting the character's education and era, or not at all. Used casually they read as cliché. The linter could flag canonical-image + canonical-sentiment pairs (e.g., 낙화 + 덧없음).

### Gaps
- **No KCI/RISS paper on figurative style in Korean web novels (웹소설) or on simile/metaphor frequency in Korean fiction** was found. All imagery sources here are about poetry or linguistics.
- 임지룡's cognitive-semantics work on Korean emotion concepts (e.g., 화) was searched for but not found or opened.
- No quantitative Korean counterpart to the VU Amsterdam metaphor-density figures (§5c) was found.

---

## 5. Failure modes and cultural appropriateness: mixed metaphors, purple prose, clichés, calques, translation

### Takeaway
The strongest empirical evidence on LLM figurative failure modes comes from professional-writer annotation (LAMP: **clichés are 17% of all expert edits**, and purple prose is a named category defined by excess metaphors and adjectives), corpus profiling (**some LLM phrasings are over 1,000× more frequent than in human text**), and preference-data analysis (typicality bias drives mode collapse). Mixed metaphors are **less of a problem than folk wisdom claims**: 76% of real metaphor clusters mix domains and remain comprehensible. Mixing fails mainly when clashing images share one clause. For Korean, the documented cultural risks are **Korea-vs-Chinese meaning confusion in Hanja idioms, fabricated idioms, literal over-interpretation, and literal machine translation of idioms**. Direct evidence on **English-calqued figures in LLM-generated Korean fiction is missing** from the peer-reviewed literature I could find.

### Cited Findings

#### 5a. Clichés, "slop", purple prose, homogenization
- **LAMP (Chakrabarty, Laban & Wu, CHI 2025).** **1,057** LLM-generated paragraphs (GPT-4o, Claude-3.5-Sonnet, Llama-3.1-70B) were edited by **18 professional writers** (**8,035 edits**) under a seven-category taxonomy.
  - Share of edits: **Awkward Word Choice 28%**, Poor Sentence Structure 20%, **Unnecessary/Redundant Exposition 18%**, **Cliché 17%**.
  - Other categories: **Purple Prose** ("ornate writing that draws attention to its own style … excess adjectives or metaphors"), Lack of Specificity, and Tense Inconsistency.
  - Cliché examples: "a tapestry of modernity threaded with the hum of traffic"; "the numbers glaring back at me like an unsolvable riddle". Purple-prose examples: "the unceasing flow of pedestrians" (labeled Purple Prose in the paper's Table 10); "a residue of weary resilience" appears in a related Table 5 example, but its exact label was not confirmed.
  - No model family wrote better than the others.
  - **LLM detection of these spans was weak:** best general precision **0.46** (expert–expert agreement 0.57) and categorical precision **0.21** (expert 0.23).
  - Preference ranking: writer-edited > LLM-edited > LLM-generated.
  - [arXiv 2409.14509](https://arxiv.org/abs/2409.14509); [HTML full text](https://arxiv.org/html/2409.14509)
- **Antislop (Paech, Roush, Goldfeder & Shwartz-Ziv, arXiv 2025; ICLR 2026 per search listing).** Profiles model-specific "slop" against human baselines and finds some patterns **over 1,000× more frequent in LLM output than in human text**. A backtracking **sampler suppresses 8,000+ patterns** without destroying vocabulary, whereas plain token banning becomes unusable at about 2,000. **FTPO** fine-tuning cut slop **about 90%** while maintaining or improving GSM8K, MMLU, and creative-writing scores. **DPO degraded writing quality and lexical diversity** — [arXiv 2510.15061](https://arxiv.org/abs/2510.15061)
- **Verbalized Sampling (2025):** mode collapse traces to **typicality bias** in human preference data, where raters favor familiar text, so aligned models gravitate to the most typical figure — [arXiv 2510.01171](https://arxiv.org/abs/2510.01171)
- **Metaphorian's five LLM failure patterns** (writers' requirements the LLM did not meet):
  1. vehicle **too close** to the tenor ("the volcano is like a mountain")
  2. **unoriginal** vehicles ("the brain is like a computer"; "Unoriginal metaphors … are no more than noises")
  3. **incoherent sub-vehicles** mixing domains ("the sun is like a light bulb. The planets are like family. The moon is like a sibling.")
  4. **unclear tenor–vehicle link**
  5. **imprecise mapping**
  - [paper PDF](https://hci.ucsd.edu/papers/metaphorian.pdf)
- **Lugli & Strapparava 2024**: without scaffolding, the model fell back on "**like a beacon of hope**", a stock vehicle with mismatched sentiment, and failed to produce a real metaphor — [paper PDF](https://aclanthology.org/2024.clicit-1.62.pdf)
- **Porter & Machery 2024**: AI poems "rarely use complex metaphors"; their accessibility wins lay readers, but they show no originality advantage — [PMC11564748](https://pmc.ncbi.nlm.nih.gov/articles/PMC11564748)

#### 5b. Mixed metaphors and coherence
- **Kimmel (2010), "Why we mix metaphors (and mix them well)", *Journal of Pragmatics* 42(1):97–115.** Corpus: **675** newspaper commentaries (Sun, Guardian) with **2,574 metaphors**; **39% and 62%** of metaphors respectively occurred in clusters. **Ontologically mixed metaphors were 76% of all clusters, and "almost all of these are straightforwardly comprehensible"**. Mixing works because metaphors sit "in separate clauses situated at different temporal, causal, speaker, or belief-related conceptual planes", so no strong joint-processing pressure creates "a perceived clash of metaphorical imagery". Opened via the authorized Portuguese translation, which reproduces the English abstract — [Cadernos de Tradução 46 (2021) PDF](https://seer.ufrgs.br/index.php/cadernosdetraducao/article/download/108972/64583/488504); [original DOI](https://doi.org/10.1016/j.pragma.2009.05.017)
- **Metaphorian**: for *extended* metaphors, writers demanded **all sub-vehicles in the same domain**, and augmented prompts significantly improved sub-metaphor coherence (p < .05) — [paper PDF](https://hci.ucsd.edu/papers/metaphorian.pdf)

#### 5c. Density norms: how much figurative language human fiction contains
- **Steen, Dorst, Herrmann, Kaal & Krennmayr (2010), "Metaphor in usage", *Cognitive Linguistics* 21(4):765–796.** Metaphor-related words: **academic 18.5%, news 16.4%, fiction 11.7%, conversation 7.7%**, or about 1 in 7.5 lexical units overall (BNC-Baby excerpts of about 47K words per register). *The publisher page was blocked (HTTP 405), so these figures come from search-result snippets of the abstract and a university repository listing; treat them as snippet-verified* — [De Gruyter](https://www.degruyterbrill.com/document/doi/10.1515/cogl.2010.024/pdf); [Bielefeld record](https://pub.uni-bielefeld.de/record/2955473)
- **Reijnierse, Burgers, Krennmayr & Steen (2019), *Corpora* 14(3):301–326.** Of 24,762 metaphors, only **4.36% were potentially *deliberate***. **News and fiction contain significantly more deliberate metaphors than expected**, and **nouns and adjectives** are relatively more often deliberate than verbs, adverbs, and prepositions — [VU research portal](https://research.vu.nl/en/publications/metaphor-in-communication-the-distribution-of-potentially-deliber-3/); [DOI](https://doi.org/10.3366/cor.2019.0176)

#### 5d. Cultural appropriateness and translation failures
- **Lee et al. 2025 (Sino-Korean)**: models default to **standard Chinese interpretations** over Korea-specific meanings of Hanja words and idioms — [KNU portal](https://research.knu.ac.kr/en/publications/knowing-the-words-missing-the-meaning-evaluating-llms-cultural-un/)
- **Tang et al. 2024**: GPT-4 accepted **fabricated idiom-like strings as real**. MT engines translated a Korean idiom literally or wrongly ("temperamental", "dummy"). And **creativity-maximizing prompts traded away faithfulness** (Analogy Creative: 4.27 creativity, 3.01 faithfulness) — [arXiv 2410.00988](https://arxiv.org/html/2410.00988v1)
- **KIM Bench**: 23–29% of idiom errors were **surface-level misinterpretation** (reading the parts rather than the idiom) — [RANLP 2025 PDF](https://aclanthology.org/2025.ranlp-1.156.pdf)
- **VIVID**: "literal over-interpretation, lexical gaps, and pragmatic flattening". **Kabra et al.**: each language's figurative expressions draw on its own cultural concepts, and performance drops sharply outside English — [LREC 2026 PDF](https://aclanthology.org/2026.lrec-1.422.pdf); [arXiv 2305.16171](https://arxiv.org/abs/2305.16171)
- **김도훈·최은실 2011**: Korean and English emotion metaphors differ in source domains, with Korean leaning on internal organs. Translation needs explicit strategies, including replacing a metaphor with a *different* metaphor — [KCI ART001590822](http://www.kci.go.kr/kciportal/landing/article.kci?arti_id=ART001590822)
- **Valentini et al. (COLM 2026)** studies whether multilingual LLMs' *native generations* show translationese, testing five languages with German and Spanish named for human annotation. **The abstract reports no results, and Korean is not among the named languages**, so this cannot yet support claims about Korean output — [arXiv 2608.17399](https://arxiv.org/abs/2608.17399)

### Inferences

#### Consolidated evidence-backed prompt patterns (for the Korean fiction system prompt / skill)
*(Each pattern lists its evidence. The Korean wording is my synthesis for the prompt designer, not quoted from sources.)*

1. **Ground-first figure construction (본의 → 공통점 → 보조관념).** Before any figure, internally fix the tenor and **one transferable property** (texture, temperature, speed, weight, sound, smell, color, motion), then search for a vehicle sharing it. *Evidence:* CMDAG (ground as CoT); SCOPE (property-conditioned similes, 88% novel properties); Ge & Parikh (object → related object → hinge property); Metaphorian (unclear link = failure).
   - 예시 지시문: 「비유를 쓰기 전에 (1) 무엇을 (2) 어떤 한 가지 속성으로 전달할지 정하고, (3) 그 속성을 공유하는 다른 영역의 사물을 고른다. 공통점을 한 문장으로 말할 수 없으면 비유를 쓰지 않는다.」
2. **POV-grounded source domains, i.e., a per-character "metaphor palette" (mind style).** Draw vehicles only from the POV character's world: occupation, era, region, class, body, history, obsessions. Keep a stable palette across chapters and let it evolve with character development. *Evidence:* Semino & Swindlehurst (metaphor patterns project and track mind style); Stowe et al. (domain control beat unconstrained models); Lugli (human figures are specific, LLM figures generic); Tang (context/genre-explicit prompting).
   - 예시: 「시점 인물의 직업·시대·지역·경험에서 온 사물만 보조관념으로 쓴다. 이 인물이 알 수 없는 사물(시대착오, 다른 문화권의 관용구)은 금지.」
3. **Distance window: different domain, but statable ground.** Reject same-domain vehicles (literal comparisons) and arbitrary far-fetched ones. *Evidence:* Metaphorian ("volcano is like a mountain" rejected; requirement lists raised originality and metaphoricity); Tang (creativity-maximizing prompts lowered faithfulness to 3.01).
4. **Elaborate the literal scene first, then figure.** In a hidden scratchpad, describe concretely what the character literally perceives, then decide whether a figure adds anything. *Evidence:* I Spy a Metaphor (CoT visual elaboration); Lugli (two-step CoT avoided the "beacon of hope" non-metaphor). **But do not apply CoT to idiom recognition or selection** (KIM Bench: CoT cut HyperCLOVA X from 90.38 to 83.83% and o3-mini from 86.72 to 76.68%).
5. **Brainstorm with typicality awareness, then select.** For key figures, generate about 5 candidates with an estimated "흔함" (typicality) score, drop the most typical and any banned vehicles, and choose the one best fitting the scene's emotional temperature. *Evidence:* Verbalized Sampling (1.6–2.1× diversity); Creative Beam Search (the judge step was necessary); Tang (multiple candidates Pareto-optimal); HAUSER (vehicle rarity tracks human creativity judgments, r ≈ 0.59–0.65).
6. **Requirement list appended to the prompt** (originality, distance, single domain per figure, tight mapping, no explanation). *Evidence:* Metaphorian (requirement concatenation significantly improved originality, metaphoricity, and coherence over 10-shot alone).
7. **Density budget: few deliberate figures, placed at salient beats.** Human fiction is full of *conventional* metaphor (about 11.7% of words, snippet-verified), but **deliberate** metaphors are a small minority (4.36% of metaphors across registers, more than expected in fiction). Purple prose is defined by excess metaphors and adjectives. Heuristic, as inference: roughly one fresh, deliberate figure per scene beat or paragraph at most. Let concrete sensory detail and conventional Korean idiom carry the rest, and give web-novel mode an even lower budget for clarity (Porter & Machery: lay readers favor accessible imagery). *Evidence:* Steen et al. 2010; Reijnierse et al. 2019; LAMP.
8. **One image domain per clause; extend within a domain or start fresh in a new sentence.** *Evidence:* Kimmel (mixing across clauses at different conceptual planes is normal and comprehensible; clashes arise under joint processing); Metaphorian (extended metaphors need same-domain sub-vehicles).
9. **Don't explain the figure (여백).** No follow-up gloss such as "…처럼, 그것은 마치 ~라는 뜻이었다". *Evidence:* LAMP (unnecessary exposition = 18% of edits); 정수자 2005 (여백 aesthetics: expressing 無 through 有).
10. **Korean-native figurative resources first.** Render emotion through Korean somatic and visceral idiom space and multisensory images, and allow in-character idiom variation in dialogue. Avoid English-origin stock figures, and use 사자성어 only when the narrator's voice warrants it. *Evidence:* 김도훈·최은실 2011 (organ-centric Korean emotion metaphors); Turker 2013; 김선영 2014 (idiom variation in speech); 윤의섭 2008 (multisensory imagery); KorID (contextual 사자성어 use at most 69%).
11. **Exemplars for structure, not content.** If using few-shot, show 2–3 diverse exemplars illustrating ground-first construction and restraint, rotate them, and forbid reusing their vehicles. *Evidence:* Metaphorian (10-shot doubled willingness-to-use) vs. KCI 2024 (GPT-3.5 few-shot fell from 92.59% to 74–81%) and VIVID (GPT-4o "stylistic overfitting").
12. **Revision-pass architecture.** Draft the plain scene first, then run a figurative pass that decides *where* a figure earns its place and *what* it should be. *Evidence:* WPS (Locate&Gen: location, then location-specific simile); MERMAID (discriminator-guided selection).

#### Failure-mode checklist for the style linter
| # | Failure mode | Detection idea (inference) | Evidence |
|---|---|---|---|
| F1 | **Clichéd / stock vehicles** (EN-origin stock such as 태피스트리, 등대 같은 희망, 수수께끼처럼; Korean web-novel stock phrases) | Maintain a ban/penalty list. Compute vehicle frequency against a Korean fiction corpus (HAUSER-style rarity). Use backtracking regeneration (Antislop) rather than plain token bans | [LAMP](https://arxiv.org/abs/2409.14509); [Antislop](https://arxiv.org/abs/2510.15061); [HAUSER](https://ar5iv.labs.arxiv.org/html/2306.07554); [Lugli](https://aclanthology.org/2024.clicit-1.62.pdf) |
| F2 | **Purple prose / over-metaphorization** | Count simile markers (처럼/같이/듯/마냥/같은) and metaphoric predicates per 1,000 characters and per paragraph. Flag paragraphs with more than one deliberate figure or with stacked adjectives | [LAMP](https://arxiv.org/html/2409.14509); [Reijnierse 2019](https://research.vu.nl/en/publications/metaphor-in-communication-the-distribution-of-potentially-deliber-3/) |
| F3 | **Intra-clause mixed metaphor** (e.g., 분노의 불길이 가슴에 뿌리를 내렸다: fire + plant) | Flag source-domain clashes *within one clause or predicate*. Do **not** flag domain changes across sentences | [Kimmel 2010](https://seer.ufrgs.br/index.php/cadernosdetraducao/article/download/108972/64583/488504); [Metaphorian](https://hci.ucsd.edu/papers/metaphorian.pdf) |
| F4 | **Too-close vehicle** (same domain, so no figurative lift) | Have an LLM judge check that tenor and vehicle are in the same category | [Metaphorian](https://hci.ucsd.edu/papers/metaphorian.pdf) |
| F5 | **Unstatable or weak ground** | Ask a judge to state the shared property in one clause; flag failures | [Metaphorian](https://hci.ucsd.edu/papers/metaphorian.pdf); [HAUSER](https://ar5iv.labs.arxiv.org/html/2306.07554); [FLUTE](https://aclanthology.org/2022.emnlp-main.481) |
| F6 | **Sentiment mismatch** (cheerful vehicle in a grief scene, unless intended irony) | Compare vehicle affect with scene affect | [HAUSER](https://ar5iv.labs.arxiv.org/html/2306.07554); [Lugli](https://aclanthology.org/2024.clicit-1.62.pdf) |
| F7 | **Explained figure / exposition after the image** | Pattern: figure followed by a gloss clause (…라는 뜻이었다 / …것만 같았다 explaining the meaning) | [LAMP](https://arxiv.org/html/2409.14509) |
| F8 | **Generic, non-specific imagery** | Flag abstract-noun vehicles (희망, 운명, 시간의 강); prefer concrete, story-world nouns | [Lugli](https://aclanthology.org/2024.clicit-1.62.pdf); [LAMP](https://arxiv.org/html/2409.14509) |
| F9 | **Out-of-world or anachronistic vehicle** (e.g., a machine image in a Joseon setting) | Check vehicles against the story bible's era/world lexicon (inference from mind-style theory) | [Semino & Swindlehurst](https://eprints.lancs.ac.uk/id/eprint/33485/) |
| F10 | **Calqued English figures and emotion metaphors** ("슬픔의 파도", "심장이 가라앉았다"-type literal renderings) | List of EN-calque patterns, plus a judge prompt: "would a Korean writer phrase this emotion this way?" **Weakly evidenced:** no Korean LLM study found | [김도훈·최은실 2011](http://www.kci.go.kr/kciportal/landing/article.kci?arti_id=ART001590822); [Kabra 2023](https://arxiv.org/abs/2305.16171) |
| F11 | **Fabricated or misused 사자성어/속담; Chinese-sense Hanja idioms** | Look up every 사자성어/속담 in 표준국어대사전/우리말샘 for existence + Korean sense + register | [Tang 2024](https://arxiv.org/html/2410.00988v1); [Lee 2025](https://research.knu.ac.kr/en/publications/knowing-the-words-missing-the-meaning-evaluating-llms-cultural-un/); [KULTURE](https://arxiv.org/abs/2412.07251) |
| F12 | **Idiom misfit in context / literal–figurative confusion** | Judge checks idiom sense against the context; flag unintended double readings | [KIM Bench](https://aclanthology.org/2025.ranlp-1.156.pdf); [MIDI](https://arxiv.org/abs/2606.02147); [VIVID](https://aclanthology.org/2026.lrec-1.422.pdf) |
| F13 | **Repetition / mode collapse across a manuscript** (same vehicles every chapter) | Track a vehicle inventory per manuscript; flag reuse beyond N chapters unless it is a deliberate motif | [Verbalized Sampling](https://arxiv.org/abs/2510.01171); [Antislop](https://arxiv.org/abs/2510.15061) |
| F14 | **Over-literal translation of figures** (when translating to or from Korean) | Choose explicitly among same metaphor, different metaphor, or non-metaphor strategies | [Tang 2024](https://arxiv.org/html/2410.00988v1); [김도훈·최은실 2011](http://www.kci.go.kr/kciportal/landing/article.kci?arti_id=ART001590822) |
| F15 | **Over-reliance on an LLM judge** | LLM span detection of clichés and purple prose has about 0.46 general / 0.21 categorical precision. Use rule + list + frequency checks first and the LLM as a second pass | [LAMP](https://arxiv.org/html/2409.14509) |

### Gaps
- **No peer-reviewed study on English-calqued idioms or figures in LLM-generated Korean (fiction or otherwise).** Korean searches (번역투 ChatGPT, 관용 표현 번역) surfaced only practitioner guides and blog posts, which I did not open and do not cite.
- **[UNVERIFIED]** Bowdle & Gentner (2005), "The career of metaphor", *Psychological Review* 112(1):193–216, and Jones & Estes (2006). I saw them only in search snippets. Bowdle & Gentner propose that novel figures are processed by comparison (simile-like) and conventional ones by categorization. Jones & Estes counter that **aptness, not conventionality**, predicts metaphor-over-simile preference. If verified, this would support "use a simile (처럼) for genuinely novel comparisons; use the A는 B다 form only when the figure is very apt or conventional". Treat as a hypothesis.
- No study measured mixed-metaphor rates in LLM output specifically. The Kimmel evidence comes from human newspaper prose.
- The density figures are English (BNC). No Korean equivalent was found.
- Most Korean idiom benchmarks test 2024–2025 models (GPT-4o, o1/o3-mini, HyperCLOVA X, Clova X). No Claude-family results on KIM Bench or KULTURE were found.

### Source table (all sources cited in these notes)
Access: **F** = full text read; **A** = abstract/landing page read; **S** = search-snippet only (not opened).

| # | Title | Year | Venue | Paper language | URL | Access | One-line finding |
|---|---|---|---|---|---|---|---|
| 1 | Generating similes effortlessly like a Pro (SCOPE) | 2020 | EMNLP | EN | https://arxiv.org/abs/2009.08942 | A | Property-conditioned BART similes: 88% novel properties; beat literary experts 37% of the time; similes made stories more evocative |
| 2 | MERMAID: Metaphor Generation with Symbolism and Discriminative Decoding | 2021 | NAACL | EN | https://arxiv.org/abs/2103.06779 | A | Symbolism + discriminator decoding: 66% preference over baselines; metaphor-augmented poems preferred 68% |
| 3 | Metaphor Generation with Conceptual Mappings | 2021 | ACL-IJCNLP | EN | https://aclanthology.org/2021.acl-long.524 | A | Source→target domain control (CM-BART) beat all models in automatic + human evaluation |
| 4 | Writing Polishment with Simile: Task, Dataset and A Neural Approach | 2021 | AAAI | EN (Chinese data) | https://arxiv.org/abs/2012.08117 | A | ~5M contextual Chinese similes; Locate&Gen decides where to insert, then what simile |
| 5 | FLUTE: Figurative Language Understanding through Textual Explanations | 2022 | EMNLP | EN | https://aclanthology.org/2022.emnlp-main.481 | A | ~9K figurative NLI items with explanations (sarcasm, simile, metaphor, idiom) |
| 6 | I Spy a Metaphor: LLMs and Diffusion Models Co-Create Visual Metaphors | 2023 | Findings of ACL | EN | https://arxiv.org/abs/2305.14724 | A | CoT "visual elaboration" of implicit meaning + objects before rendering; 6,476 visual metaphors |
| 7 | HAUSER: Towards Holistic and Automatic Evaluation of Simile Generation | 2023 | ACL | EN | https://ar5iv.labs.arxiv.org/html/2306.07554 | F | 5 criteria / 3 perspectives; vehicle-rarity creativity metric correlates 0.59–0.65 with humans |
| 8 | Metaphorian: LLMs to Support Extended Metaphor Creation for Science Writing | 2023 | ACM DIS | EN | https://hci.ucsd.edu/papers/metaphorian.pdf | F | LLM vehicles willing-to-use 0.23 vs human 0.58; chaining + requirement lists significantly raised originality and coherence |
| 9 | Large Language Model Displays Emergent Ability to Interpret Novel Literary Metaphors | 2023/24 | arXiv (journal version unverified) | EN | https://arxiv.org/abs/2308.01497 | A | GPT-4 interpretations of novel poetic metaphors rated above college students' |
| 10 | CMDAG: Chinese Metaphor Dataset with Annotated Grounds as CoT | 2024 | LREC-COLING | EN (Chinese data) | https://arxiv.org/abs/2402.13145 | A | 28K tenor–vehicle–ground annotations; ground-as-CoT improves creative metaphor generation |
| 11 | Multimodal Chain-of-Thought Prompting for Metaphor Generation | 2024 | CLiC-it | EN | https://aclanthology.org/2024.clicit-1.62.pdf | F | 92% judged metaphors; no-CoT output fell back to "beacon of hope"; LLM figures generic vs human specifics |
| 12 | Automatic Scoring of Metaphor Creativity with Large Language Models | 2025 (online 2024) | Creativity Research Journal 37(4) | EN | https://pure.psu.edu/en/publications/automatic-scoring-of-metaphor-creativity-with-large-language-mode/ | A | Fine-tuned scorers r=.72/.70 vs semantic distance r=.42 |
| 13 | AI-generated poetry is indistinguishable from human-written poetry and is rated more favorably | 2024 | Scientific Reports 14 | EN | https://pmc.ncbi.nlm.nih.gov/articles/PMC11564748 | F | Lay readers 46.6% at detection; AI rated higher on imagery but not originality; "rarely use complex metaphors" |
| 14 | Can AI writing be salvaged? (LAMP) | 2025 | CHI | EN | https://arxiv.org/abs/2409.14509 | F | Clichés = 17% of 8,035 expert edits; purple prose defined; LLM detection precision 0.46/0.21 |
| 15 | Metaphor and LLMs: When Surface Features Matter More than Deep Understanding | 2025 | arXiv / Findings of ACL (listing) | EN | https://arxiv.org/abs/2507.15357 | A | Metaphor-task performance driven by lexical overlap and sentence length |
| 16 | A Comparative Approach to Assessing Linguistic Creativity of LLMs and Humans | 2025 | KES / Procedia CS 270 | EN | https://arxiv.org/abs/2507.12039 | A | OCSAI-scored LLMs beat humans on originality; humans more E-creative |
| 17 | Antislop: Identifying and Eliminating Repetitive Patterns in LMs | 2025 | arXiv (ICLR 2026 per listing) | EN | https://arxiv.org/abs/2510.15061 | A | Some LLM patterns >1,000× human frequency; sampler bans 8,000+; FTPO −90% slop |
| 18 | Verbalized Sampling: How to Mitigate Mode Collapse | 2025 | arXiv | EN | https://arxiv.org/abs/2510.01171 | A | Typicality bias causes mode collapse; candidates + probabilities give 1.6–2.1× creative diversity |
| 19 | Creative Beam Search: LLM-as-a-Judge for Improving Response Generation | 2024 | ICCC | EN | https://arxiv.org/abs/2405.00099 | A | Diverse candidates + LLM judge beat standard sampling; judge step necessary |
| 20 | Visual Conceptual Blending with Large-scale Language and Vision Models | 2021 | arXiv (ICCC per listing) | EN | https://ar5iv.labs.arxiv.org/html/2106.14127 | F | "X is like [MASK]" → hinge property prompts; LMs beat ConceptNet at vehicle association |
| 21 | Creative Blends of Visual Concepts | 2025 | arXiv (CHI per listing) | EN | https://arxiv.org/html/2502.16062v1 | F | Concept→object→attribute pipeline with stated reasoning; metaphoricity 5.66 vs 5.05 |
| 22 | Multi-lingual and Multi-cultural Figurative Language Understanding | 2023 | Findings of ACL | EN | https://arxiv.org/abs/2305.16171 | A | 7 languages all far below English; figurative language is culture-specific |
| 23 | Creative and Context-Aware Translation of East Asian Idioms with GPT-4 | 2024 | WAT / Findings of EMNLP (conflicting listings) | EN (incl. 2,316 Korean idioms) | https://arxiv.org/html/2410.00988v1 | F | Multi-candidate and context prompts beat Google/DeepL; creativity–faithfulness trade-off; fake idioms accepted as real |
| 24 | VIVID: Figurative Language Gap in Vietnamese NLP | 2026 | LREC | EN (Vietnamese data) | https://aclanthology.org/2026.lrec-1.422.pdf | F | Top models <50%; few-shot caused GPT-4o stylistic overfitting; literal over-interpretation |
| 25 | Benchmarking Korean Idiom Understanding (KIM Bench) | 2025 | RANLP | EN (Korean data) | https://aclanthology.org/2025.ranlp-1.156.pdf | F | HyperCLOVA X 90.38% vs native speakers 91.3%; CoT lowered accuracy for 4 of 6 models |
| 26 | KULTURE Bench: Assessing LMs in Korean Cultural Context | 2024 | PACLIC 38 | EN (Korean data) | https://arxiv.org/abs/2412.07251 | F | 사자성어 cloze (KorID 1,631): best 69.12% (Clova X); poetry best 45.7% (GPT-4) |
| 27 | Knowing the Words, Missing the Meaning: Sino-Korean Words and Four-Character Idioms | 2025 | Applied Sciences 15(13) | EN (Korean data) | https://research.knu.ac.kr/en/publications/knowing-the-words-missing-the-meaning-evaluating-llms-cultural-un/ | A | Models miss Korea-specific (vs Chinese) meanings of Hanja idioms; GPT-4o best |
| 28 | CLIcK: Cultural and Linguistic Intelligence in Korean | 2024 | LREC-COLING | EN (Korean data) | https://arxiv.org/html/2403.06412v4 | F | 1,995 items, no idiom category; Functional knowledge ~33% (GPT-3.5/Claude-2) |
| 29 | HAE-RAE Bench: Evaluation of Korean Knowledge in LMs | 2024 | LREC-COLING | EN (Korean data) | https://arxiv.org/abs/2309.02706 | A | Vocabulary/history/general knowledge/reading; no figurative task listed |
| 30 | KMMLU: Measuring Massive Multitask Language Understanding in Korean | 2024 | arXiv | EN (Korean data) | https://arxiv.org/abs/2402.11548 | A | 35,030 exam MCQs; best public 50.5%; not figurative-focused |
| 31 | 한국어 체성감각 비유 표현에 대한 사전학습 언어모델의 이해 능력 분석 | 2024 | 한국정보통신학회논문지 28(7) | KO | https://www.kci.go.kr/kciportal/ci/sereArticleSearch/ciSereArtiView.kci?sereArticleSearchBean.artiId=ART003104112 | A | GPT-4/4o 100% on 27 somatosensory figures; GPT-3.5 few-shot worse than zero-shot |
| 32 | 인공지능의 한국어 능력 종합 평가를 위한 벤치마크 | 2024 | 한글 85(3) | KO | https://www.kci.go.kr/kciportal/ci/sereArticleSearch/ciSereArtiView.kci?sereArticleSearchBean.artiId=ART003123750 | A | 6-domain Korean proficiency benchmark with human + GPT-4 evaluation; figurative coverage unstated |
| 33 | 한국어 관용표현의 화용적 특징 연구 - 변이 양상을 중심으로 | 2014 | 한국언어문화학 11(3) | KO | https://www.kci.go.kr/kciportal/landing/article.kci?arti_id=ART001948243 | A | Speakers vary, truncate, modernize and slangify idioms in drama dialogue |
| 34 | 한국어-영어 감정 은유의 개념화 과정 및 한영 번역 | 2011 | 영어영문학21 24(3) | KO | http://www.kci.go.kr/kciportal/landing/article.kci?arti_id=ART001590822 | A | Korean emotion metaphors lean on internal organs; 4 translation strategies |
| 35 | A corpus-based approach to emotion metaphors in Korean | 2013 | Review of Cognitive Linguistics 11(1) | EN | https://asu.elsevierpure.com/en/publications/a-corpus-based-approach-to-emotion-metaphors-in-korean-a-case-stu | A | Universal vs cultural Korean emotion metaphors; frequency correlates with productivity |
| 36 | 신체부위 은유와 한국어 속담 및 관용어 교육 | 2018 | EAKLE-7 (conference abstract) | KO | https://blogs.helsinki.fi/eakle-7/files/2018/04/신체부위-은유와-한국어-속담-및-관용어-교육.pdf | A | Body-part proverbs/idioms encode community, hierarchy, face (체면) and speech-caution values |
| 37 | 감각의 복합성과 모더니즘 시의 '회화성' 연구 | 2008 | 한중인문학연구 25 | KO | https://www.earticle.net/Article/A100553 | A | 1930s "pictorial" imagery is actually multisensory, creating spatial depth |
| 38 | 한국 현대시의 고전적 미의식 연구: 정지용·조지훈·박목월의 山詩 | 2005 | Ajou Univ. PhD diss. | KO | https://dcoll.ajou.ac.kr/dcollection/srch/srchDetail/000000000358 | A | Mountain imagery via 산수화 aesthetics, 은일, and 여백 (expressing 無 through 有) |
| 39 | [유성호의 문학의 길목] 꽃의 심상과 현대시 | 2017 | 서울신문 (column) | KO | https://m.seoul.co.kr/news/2017/04/11/20170411030008 | F | Flowers = brief peak beauty; falling flowers = transience; canonical flower poems |
| 40 | 생성형 인공지능(AI)의 한국문학 영어번역의 딜레마와 가능성 | 2025 | 외국문학연구 100 | KO | https://scholarworks.sookmyung.ac.kr/item/af5537be-a3b3-4474-90b6-685ba452d41c | A | ChatGPT imitates style via prompts but is fidelity-bound; questions its emotional resonance |
| 41 | Metaphor and mind style in Ken Kesey's One Flew Over the Cuckoo's Nest | 1996 | Style 30(1) | EN | https://eprints.lancs.ac.uk/id/eprint/33485/ | A | Consistent metaphor patterns project and track a narrator's worldview |
| 42 | Why we mix metaphors (and mix them well) | 2010 | Journal of Pragmatics 42(1) | EN (read via PT translation) | https://seer.ufrgs.br/index.php/cadernosdetraducao/article/download/108972/64583/488504 | F | 76% of metaphor clusters are mixed yet comprehensible; clashes arise within shared clauses |
| 43 | Metaphor in communication: distribution of potentially deliberate metaphor | 2019 | Corpora 14(3) | EN | https://research.vu.nl/en/publications/metaphor-in-communication-the-distribution-of-potentially-deliber-3/ | A | Only 4.36% of metaphors deliberate; fiction above expected |
| 44 | Metaphor in usage | 2010 | Cognitive Linguistics 21(4) | EN | https://pub.uni-bielefeld.de/record/2955473 | S | Metaphor-related words: fiction 11.7% (academic 18.5%, news 16.4%, conversation 7.7%) |
| 45 | Multilingual Idioms in Sentences and Conversations (MIDI) | 2026 | arXiv | EN | https://arxiv.org/abs/2606.02147 | A | Literal readings of idioms harder than figurative across 18 languages; Korean inclusion unconfirmed |
| 46 | An Investigation of Translationese in the Generations of Multilingual LLMs | 2026 | COLM | EN | https://arxiv.org/abs/2608.17399 | A | Studies translationese in native LLM generations; abstract gives no results; Korean not named |
| — | The career of metaphor (Bowdle & Gentner) | 2005 | Psychological Review 112(1) | EN | https://dedregentner.owlstown.net/publications/38513-the-career-of-metaphor | S **[UNVERIFIED]** | Novel figures processed as comparisons, conventional as categories (snippet only) |
| — | Brainstorm, then select (Summers-Stay, Lukin & Voss) | 2023 | unknown | EN | (not located) | S **[UNVERIFIED]** | Generating multiple candidates and selecting raises creativity scores (bibliography mention only) |
