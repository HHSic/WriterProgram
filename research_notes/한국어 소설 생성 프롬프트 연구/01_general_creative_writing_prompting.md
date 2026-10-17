# General (Language-Agnostic) Evidence on Prompting LLMs for Better Fiction: Literary Quality, Freshness, Vocabulary Diversity

_Research date: 2026-10-17. Scope: prompt-level style/quality techniques, failure modes, benchmarks, diversity/sampling, style imitation. Figurative language, Korean vocabulary/papers, discourse coherence, practitioner tips and vendor docs are covered by other researchers (vendor docs cited only briefly here)._

_Verification: every source below was opened. 23 papers were retrieved in full text and the quoted numbers were checked against the text (marked **F** in the final table); the rest were checked at abstract/metadata level via the arXiv API (**A**) or as web pages (**W**). Many human evaluations use 2023–2024 models (GPT-3.5/4/4o, Claude 1.3–3.5, Llama 2/3, Gemini 1.5); these are flagged "[older models]" because current frontier models (Claude Opus 5.x, GPT-5.5 etc.) may behave differently._

---

## 1. Which prompting strategies have measured effects on the quality of LLM fiction?

### Takeaway
The measured gains come mostly from **structure and process**, not from adjectives in the prompt. What helped: breaking the work into planning plus section-by-section writing (best with separate, specialized calls), telling the model the narrative structure explicitly (arc type, turning points), giving it specific premises or titles from a human, and revision passes that either find and rewrite flagged spans or generate several candidates and pick the best with a calibrated scorer. Persona prompts, "be creative" instructions, bare negative instructions, and in-context author exemplars show weak, inconsistent, or negative effects on literary quality as experts judge it.

### Cited Findings

#### Persona / role prompts
- PDS study (EMNLP 2024): after "extensive internal experimentation and prompt engineering," the authors tested two generation strategies. **WriterProfile** prepends "a profile of a seasoned writer known for psychologically deep, engaging stories." **Plan+Write** writes character portraits first, then the story. Plan+Write gave modest average gains in narrative complexity, emotion provocation, authenticity and empathy. WriterProfile gave higher engagement. The benefits were "not uniform or predictable by model size": WriterProfile helped GPT-4 (notably engagement), while Plan+Write "was always helpful" for Llama-2-70B [older models] — [Harel-Canada et al., EMNLP 2024](https://arxiv.org/abs/2406.12680). **Implication:** a writer persona is a small lever that depends on the model (it mostly moves engagement). Don't rely on it for craft, and A/B test it on the deployed model.
- Personas in system prompts (162 roles, 4 LLM families, 2,410 factual questions) did not improve performance compared with no persona. Per-persona effects were "largely random," and automatically picking the best persona did no better than random selection — [Zheng et al., Findings of EMNLP 2024](https://arxiv.org/abs/2311.10054). **Implication:** this evidence is on factual QA, not fiction, but it undercuts the belief that "You are an award-winning novelist" is a quality switch on its own.
- Personas help more as **critique and diversity devices**:
  - CritiCS gives its critics adaptive personas derived from the story's themes. Ablations show that personas, a leader critic, and multiple criteria each contributed to more creative story plans — [Bae & Kim, EMNLP 2024](https://arxiv.org/abs/2410.02428).
  - LLM Discussion assigns distinct roles to several LLMs "to combat the homogeneity of LLMs." It beat single-LLM and other multi-LLM baselines on divergent-thinking tests (Alternative Uses, Similarities, Instances, Scientific Creativity; not fiction) — [Lu et al., COLM 2024](https://arxiv.org/abs/2405.06373).
  - **Implication:** use personas to widen the idea space in brainstorming and critic agents, not as the main voice control for drafting.
- 2026 preprint, "Meta-Persona Anchoring": the model first picks an idiosyncratic persona for itself, then output passes through a top-p filter followed by extreme temperature (T ≥ 4.0). On Infinity-Chat with open models under ~20B, average pairwise cosine similarity fell from ≈0.85 to ≈0.65 — [Fu et al. 2026](https://arxiv.org/abs/2608.02618). **Implication:** this is unreviewed and the effect is bundled with decoding changes that hosted APIs may not allow. Persona-first anchoring is still a cheap diversity idea to test.

#### Exemplars / few-shot passages (details in Q5)
- In-context prompting to emulate 50 award-winning authors (GPT-4o, Claude 3.5 Sonnet, Gemini 1.5 Pro) was strongly disfavored by MFA-trained readers, both for stylistic fidelity (OR=0.16) and for quality (OR=0.13) [older models] — [Chakrabarty, Ginsburg & Dhillon 2025](https://arxiv.org/abs/2510.13939). **Implication:** exemplar prompting will not give expert-level emulation of an author's voice.
- Anthropic's official guidance calls examples "one of the most reliable ways to steer Claude's output format, tone, and structure." It recommends 3–5 examples that are relevant, diverse ("vary enough that Claude doesn't pick up unintended patterns"), and wrapped in `<example>` tags — [Anthropic prompting best practices](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-4-best-practices). **Implication:** give several deliberately varied exemplar passages (different scenes, rhythms, sentence lengths) so the model learns the register instead of copying one passage's images.

#### Detailed style guides; positive vs negative instructions
- Anthropic's guidance:
  - "Tell Claude what to do instead of what not to do." Instead of "Do not use markdown," write "Your response should be composed of smoothly flowing prose paragraphs."
  - "Match your prompt style to the desired output."
  - Explaining the motivation behind a rule helps the model generalize. Their example: explaining a text-to-speech reason beats "NEVER use ellipses."
  - Source: [Anthropic](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-4-best-practices).
  - **Implication:** write the fiction style guide itself in good prose, state reasons ("readers of this genre infer emotion from gesture, so…"), and phrase rules as target behaviors.
- **Pink Elephant problem.** OpenHermes models told in the system prompt to avoid an entity brought it up "as often or more often" than without the instruction. Direct Principle Feedback fine-tuning fixed this: a 13B model then matched GPT-4, which handled prompted avoidance well — [Castricato et al. 2024](https://arxiv.org/abs/2402.07896). **Implication:** bans written into the prompt can prime the banned item, especially on weaker models. Frontier models do better, but bans still need checking afterwards.
- Negated prompts showed *inverse scaling* across OPT, GPT-3 and InstructGPT (125M–175B), including with few-shot examples and with fine-tuning on negation — [Jang, Ye & Seo 2022](https://arxiv.org/abs/2209.12711). **Implication:** historical [older models; likely outdated for frontier models], but it agrees with keeping negative instructions few and concrete.
- The Antislop authors write that "instructing the model to avoid a set of banned vocabulary has limited efficacy and may induce a backfire effect due to the 'Pink elephant problem'." This is their claim, citing Castricato; they did not measure it separately. It motivated their sampler-level backtracking — [Paech et al. 2025](https://arxiv.org/abs/2510.15061). **Implication:** make the linter, not the prompt, the main slop enforcement. Keep only a short, high-value "avoid → prefer" list in the prompt.
- EQ-Bench notes about its own judge: for NSFW-tuned models it "tends to punish this severely, and instructing it not to is ineffective" — [EQ-Bench about](https://eqbench.com/about.html). **Implication:** anecdotal, but another sign that "don't do X" seldom overrides a strong model prior.

#### "Show, don't tell," subtext, over-exposition
- Professional editors' labels for the category **"Unnecessary/Redundant Exposition"** included "Show don't tell," "Unnecessary because implied" and "Over exposition." The category made up 18% of edits across GPT-4o, Claude-3.5-Sonnet and Llama-3.1-70B, and its frequency stayed "relatively constant" even for texts rated higher in quality [older models] — [Chakrabarty, Laban & Wu, CHI 2025 (LAMP)](https://arxiv.org/abs/2409.14509). **Implication:** over-explanation is a default across model families. Add an explicit "trust the reader; leave implications unstated" target and a revision pass that deletes sentences restating what the scene already shows.
- TTCW: 14 binary expert tests, including:
  - Narrative Pacing ("manipulation of time in terms of compression or stretching")
  - Scene vs Exposition ("balance between scene and summary/exposition")
  - Language Proficiency & Literary Devices ("sophisticated use of idiom or metaphor or literary allusion")
  - Narrative Ending ("natural and earned")
  - Perspective & Voice Flexibility; Emotional Flexibility ("balance between interiority and exteriority")
  - Structural Flexibility ("turns that are both surprising and appropriate")
  - Originality in Thought ("without any cliches")
  - World Building (believable "at the sensory level")
  - Character Development
  - Rhetorical Complexity ("multiple 'levels' of meaning (surface and subtext)")
  - LLM stories passed 9% (ChatGPT) to 30% (Claude) of tests, against 84.7% for New Yorker stories [older models: GPT-3.5, GPT-4, Claude 1.3] — [Chakrabarty et al., CHI 2024](https://arxiv.org/abs/2309.14556).
  - **Implication:** the TTCW questions are a ready, expert-validated checklist for a critique pass or the author's own review. They are not reliable for automatic scoring by an LLM (see Q3).
- More content detail is not enough. Prompts were tested at ~30 words and at 150–200 words; even "with detailed original content, they [LLMs] struggle to maintain good writing standards (avoiding clichés, revealing subtext, and introducing purple prose)" — [Chakrabarty, Laban & Wu 2025 (AI-Slop to AI-Polish)](https://arxiv.org/abs/2504.07532). **Implication:** rich story context is necessary but not sufficient. Style needs its own guidance and an editing stage.
- Reading subtext is weak too. GPT-4, Claude-2.1 and Llama-2-70B made faithfulness mistakes in over 50% of summaries of unpublished stories and "struggle with specificity and interpretation of difficult subtext" [older models] — [Subbiah et al., TACL 2024](https://arxiv.org/abs/2403.01061). **Implication:** when a scene depends on subtext, state the intended subtext in the hidden plan so the prose can imply it without spelling it out.
- LIIPA prompts LLMs to infer implicit character portrayal (traits revealed by actions) and outperformed non-LLM baselines — [Jaipersaud et al., NeurIPS 2024 Creativity Workshop](https://arxiv.org/abs/2412.04576). **Implication:** this is an analysis tool, not a generation study. It could be repurposed as a "shown vs told" checker.

#### Constraints, prompt specificity, human-supplied content
- GPT-4 Turbo wrote more creatively from novelist Patricio Pron's titles than from its own titles, as judged by literary critics using a Boden-based rubric [older models] — [Marco et al., EMNLP 2024](https://arxiv.org/abs/2407.01119). **Implication:** seed the model with specific, unusual anchors supplied by the author (titles, images, situations) instead of letting it choose defaults.
- Story prompts that produced low-uniqueness LLM continuations also produced low-uniqueness human ones. The authors conclude that "one can boost the Sui Generis score of human and LLM-generated stories by improving the story prompts; prompting is itself a creative endeavor" — [Xu et al., PNAS 2025](https://arxiv.org/abs/2501.00273). **Implication:** put effort into making the scene brief specific and strange.
- CS4: adding constraints to a story prompt raises specificity and stops LLMs from "retelling high-quality narratives in their training data." Models balance instruction-following and narrative coherence differently. Learning from human feedback helps models select better stories from their training data but has "limited influence" on producing unseen creative stories — [Atmakuru et al. 2024](https://arxiv.org/abs/2410.04197). **Implication:** a few concrete constraints push the model off memorized paths; piling them on strains coherence. Give each scene a few constraints that carry real weight.

#### Plan-then-write, decomposition, multi-stage drafting
- **Re3** (plan, then reprompt with plan and story state, rerank, edit): human raters judged +14% (absolute) more stories to have a coherent overarching plot and +20% to be premise-relevant, compared with same-length direct generation [older models] — [Yang et al., EMNLP 2022](https://arxiv.org/abs/2210.06774).
- **DOC** (detailed hierarchical outliner plus controller) beat Re3 on plot coherence (+22.5%), outline relevance (+28.2%) and **interestingness (+20.7%)** — [Yang et al., ACL 2023](https://arxiv.org/abs/2212.10077). **Implication:** moving creative work into a detailed outline improves interestingness, not only coherence.
- **Dramatron**: hierarchical prompt chaining (title → characters → beats → locations → dialogue). 15 theatre and film professionals found it useful for co-creation — [Mirowski et al. 2022](https://arxiv.org/abs/2209.14958).
- **Agents' Room** (ICLR 2025):
  - Setup: planning agents (conflict, character, setting, plot) and writing agents (exposition, rising action, climax, falling action, resolution) share a scratchpad and are run by an orchestrator.
  - Ratings: 9,900 pairwise ratings by writers or literature graduates.
  - Results: Agents' Room variants with writing agents were preferred over baselines "across all dimensions"; the fine-tuned "write" and "plan+write" variants were best.
  - Single-call zero-shot variants told to plan, reflect, or decompose first underperformed.
  - Human stories were still preferred overall; the gap was smallest in language use.
  - Machine stories were "more generic in their sentence structure" and more repetitive; "the most repetitive models are also the ones that produce the longest stories."
  - Source: [Huot et al.](https://arxiv.org/abs/2410.02603).
  - **Implication:** "first outline, then write" inside one call is weaker than separate calls with specialized instructions per story part. Generating in sections also limits repetition.
- Integrating story-arc awareness improved diversity by 45% over vanilla prompting; adding turning-point information improved suspense and engagement by 40% — [Tian et al., EMNLP 2024](https://arxiv.org/abs/2407.13248). **Implication:** state the arc type and where turning points fall in the plan.
- RL-learned reasoning over the story so far, to plan the next chapter, produced chapters that humans preferred "across almost all metrics," most strongly in sci-fi and fantasy — [Gurung & Lapata 2025](https://arxiv.org/abs/2503.22828). **Implication:** "condense the story so far, plan the next chapter, then write" suits serialized web novels.
- WritingBench, Literature & Arts domain: reasoning models (Claude-3.7-thinking, DeepSeek-R1, o1-Preview) outperformed their non-reasoning counterparts. Fine-tuning with chain-of-thought beat fine-tuning without it on this subset and on EQ-Bench — [Wu et al. 2025](https://arxiv.org/abs/2503.05244). **Implication:** let the model think and plan (hidden) before writing prose.
- Process supervision (reverse-engineered creative reasoning) was "highly effective" for Chinese creative writing, but needed general data mixed in (≥1 creative sample per 12 general) — [Li et al. 2025 (COIG-Writer)](https://arxiv.org/abs/2510.14763). **Implication:** this is training-level, but it supports keeping narrative logic (planning) separate from linguistic expression.

#### Self-critique, self-refine, revision loops
- **LAMP automatic editing**: detect problem spans in 7 categories, then rewrite them. Across 600 expert preference judgments, average rank (1 = best) was 1.47 for writer-edited, 1.99 for LLM-edited and 2.55 for unedited LLM text (Wilcoxon p ≤ ~1e-11) — [Chakrabarty, Laban & Wu, CHI 2025](https://arxiv.org/abs/2409.14509). **Implication:** a category-guided "find spans → rewrite only those spans" pass reliably beats the raw draft, though it does not reach human editors.
- **Generate several edits, then rank**:
  - Pipeline: identify idiosyncrasies, rewrite each span, execute edits; generate multiple candidate revisions; rank them with a trained Writing Quality Reward Model (WQRM).
  - 9 experienced writers preferred the WQRM-selected sample 66% of the time overall, and 72.2% when the reward gap was over 1 point.
  - Source: [Chakrabarty, Laban & Wu 2025](https://arxiv.org/abs/2504.07532).
  - **Implication:** selection only works with a calibrated scorer. Generic LLM judges are near chance on writing quality (Q3).
- **CritiCS**:
  - Setup: multi-round critiques. For the plan (CrPlan), 3 persona critics plus a leader. For the text (CrText), 2 critics using "Image" and "Voice" criteria, with a leader choosing.
  - Results (300 pairwise human comparisons, CrPlan vs DOC): Interesting 85.00 vs 57.56; Coherent 77.89 vs 67.33; Creative 84.33 vs 57.33.
  - One showcased "Voice" edit turns "raised an eyebrow" into "arched an incredulous eyebrow."
  - Source: [Bae & Kim, EMNLP 2024](https://arxiv.org/abs/2410.02428).
  - **Implication:** critics with specific criteria and a leader who selects work. "Expressiveness" critics, however, can drift toward ornament and stock phrasing ("arched an eyebrow" is itself a stock phrase), so pair them with a purple-prose/cliché check.
- In Agents' Room, the zero-shot "reflect" baseline (self-reflection in the style of Madaan et al. and Shinn et al.) underperformed — [Huot et al.](https://arxiv.org/abs/2410.02603).
- Self-preference bias: LLM evaluators recognize their own outputs, and self-recognition correlates linearly with how much they favor them — [Panickssery, Bowman & Feng 2024](https://arxiv.org/abs/2404.13076). **Implication:** a loop where the same model rates its own draft "better" is biased. Prefer checklists of specific defects, a different judge model, or human choice.

#### Multi-agent writer–editor–critic setups
- Agents' Room (specialized planning and writing agents), CritiCS (critics plus leader) and LLM Discussion (role-played multi-agent ideation) all beat single-model baselines in human or LLM evaluation, as summarized above — [Huot et al.](https://arxiv.org/abs/2410.02603); [Bae & Kim](https://arxiv.org/abs/2410.02428); [Lu et al.](https://arxiv.org/abs/2405.06373).

#### Human-in-the-loop
- Writers given GPT-4 story ideas:
  - Setup: 293 writers and 600 evaluators; 8-sentence stories.
  - Novelty rose 5.4% with one idea and 8.1% with five; usefulness rose 3.7% and 9.0%.
  - Gains were concentrated in less creative writers: novelty +10.7% with five ideas, writing quality up to +26.6%.
  - Stories became more similar to each other (b=0.871 with one idea, 0.718 with five), equal to ~10.7% and 8.9% of the similarity range of human-only stories.
  - Sources: [Doshi & Hauser, Science Advances 2024](https://doi.org/10.1126/sciadv.adn5290) ([PMC full text](https://pmc.ncbi.nlm.nih.gov/articles/PMC11244532)).
  - **Implication:** AI suggestions lift weaker writers but homogenize output. Offer several divergent options and keep the author's own material primary.
- In a 36-participant study, ChatGPT users produced ideas that were less semantically distinct from each other than users of an alternative tool. They generated more and more detailed ideas but "felt less responsible" for them — [Anderson, Shah & Kreminski, C&C 2024](https://arxiv.org/abs/2402.01536).
- Co-writing with InstructGPT, but not base GPT-3, made different authors' essays more similar and reduced lexical and content diversity, mainly through the text the model contributed (argumentative essays, not fiction) — [Padmakumar & He, ICLR 2024](https://arxiv.org/abs/2309.05196).
- 13 professional writers using Wordcraft found AI useful for brainstorming, story details, world-building and research. The tools "struggle to preserve style and authorial voice" and "lack deep understanding of story contents" [older models] — [Ippolito et al. 2022](https://arxiv.org/abs/2211.05030).
- Writers (n=30) sought LLM help across planning, translating (drafting) and reviewing, and found it more helpful for translating and reviewing — [Chakrabarty et al. 2023](https://arxiv.org/abs/2309.12570).
- Expert edits on LLM paragraphs were 74% replacements, 18% deletions and 8% insertions — [LAMP](https://arxiv.org/abs/2409.14509). **Implication:** design the revision skill mostly to replace and cut, not to add.

### Inferences
- The strongest lever is **decomposition with role-specific calls**, plus a narrative plan that names conflict, arc, turning-point positions, the intended subtext, and the ending's emotional valence. Single-call "plan then write" prompts are weaker (Agents' Room).
- A system prompt should spend its words on **specific, reasoned target behaviors** (what good looks like in this genre, with reasons and varied exemplars). Persona flourishes or "be creative" add little.
- Revision should be a **separate, taxonomy-driven, span-level pass**, not a vague "improve this" loop. LAMP's 7 categories and the TTCW questions give the rubric.
- For WriterProgram, the web-novel serial mode (chapter planning over the story so far) and the literary mode (subtext, restraint, ambiguity) likely need different planning templates. This is supported indirectly by Narrative Flattening (Q2): the gap to model defaults is largest for professional literary fiction.

### Gaps
- No study was found that directly A/B-tests a "show, don't tell" instruction, or a detailed literary style guide against a minimal prompt, on expert-judged fiction quality. The evidence is indirect (edit taxonomies, critiques).
- There is no rigorous measurement of in-prompt banned-word lists on *current frontier* models. The Pink Elephant results are from Llama-2-era models, and GPT-4 already handled simple avoidance.
- Most process-level studies (Re3, DOC, Agents' Room, CritiCS, PDS) use 2022–2024 models. Their relative gains may shrink or grow with stronger models; no 2026 replication was found.

---

## 2. What does LLM fiction do badly, and which interventions reduce it?

### Takeaway
Experts consistently find the same defects across model families:
- **Prose level:** awkward word choice, poor sentence structure, redundant exposition, clichés, purple prose, lack of specificity, and over-represented "slop" words, phrases and names.
- **Grammar level:** dense, noun-heavy constructions.
- **Narrative level:** homogeneously positive arcs, early turning points, low tension, "flattened" thematic and emotional variation.
- **Collective homogenization:** within one model and across models (e.g., "time is a river").

Measured fixes are span-level expert-taxonomy editing, reward-model selection, sampler-level slop suppression, explicit narrative-structure prompting, and (strongest) fine-tuning on an author's oeuvre.

### Cited Findings

#### Expert-identified prose idiosyncrasies
- **LAMP corpus**:
  - Data: 1,057 paragraphs (~205 words each) from GPT-4o, Claude-3.5-Sonnet and Llama-3.1-70B, edited by professional writers.
  - Taxonomy: Cliché; Unnecessary/Redundant Exposition; Purple Prose; Poor Sentence Structure; Lack of Specificity and Detail; Awkward Word Choice and Phrasing; Tense Inconsistency.
  - Most common edits: Awkward Word Choice and Phrasing 28%, Poor Sentence Structure 20%, Unnecessary/Redundant Exposition 18%, Clichés 17%.
  - GPT-4o used more purple prose; Llama-3.1-70B more unnecessary exposition. No model family outperformed the others.
  - Texts rated 2 averaged 10.2 edits, texts rated 10 averaged 2.4. Cliché and awkward-wording edits fall most steeply as quality rises.
  - Source: [Chakrabarty, Laban & Wu, CHI 2025](https://arxiv.org/abs/2409.14509).
  - **Implication:** these 7 categories are the evidence-based backbone for both the system prompt's targets and the linter's rule families.
- **LAMP over-represented syntactic templates.** These part-of-speech patterns appear much more in LLM paragraphs than in human seed paragraphs, and writers edited them out at the rates shown [older models] — [LAMP](https://arxiv.org/abs/2409.14509):

  | Pattern | Edited | Examples |
  |---|---|---|
  | DT NN IN NN CC | 54% | "a mix of pride and…", "a pang of nostalgia and…", "a flicker of hope or…", "a blend of curiosity and…", "the power of storytelling" |
  | DT JJ NN IN PRP$ | 40% | "the intricate tapestry of its", "the unspoken plea in her", "a silent battle between his" |
  | NN IN NN CC NN | 35% | "mix of desperation and resolve", "story of struggle and resilience" |
  | DT NN IN JJ NN | 27% | "the weight of past grievances", "a sea of unspoken expectations", "a mask of controlled concern" |

  **Implication:** "abstract-noun + of + emotion-noun (+ and + emotion-noun)" constructions are a high-precision linter target. A Korean analogue (e.g., stacked "~의 ~과 ~" abstractions) needs Korean-specific rules.
- **What experts see as "AI-ness"**:
  - Experts appear to detect "AI-ness (clichés, purple prose, too much exposition, lack of subtext, mixed metaphors)."
  - Cliché density had the strongest correlation with AI-detector scores. It mediated 16.4% of the detection effect on MFA readers' preferences for in-context outputs, but only 1.3% (not significant) after author fine-tuning.
  - Clichés were flagged by prompting Claude 4.1 Opus, then intersecting two humans' verifications, "because language models can overgenerate and penalize expressions that are not clichés."
  - Source: [Chakrabarty, Ginsburg & Dhillon 2025](https://arxiv.org/abs/2510.13939).
  - **Implication:** cliché density is the single most diagnostic quality signal. An LLM-based cliché detector over-flags, so linter hits need precision tuning or confirmation by the author.
- GPT-4o showed "near-perfect coherence and used less cliche phrases" but "more predictable language": only 3% of its synopses featured surprising associations, against 15% for a fine-tuned BART-large [older models] — [Marco, Rello & Gonzalo, COLING 2025](https://arxiv.org/abs/2409.11547). **Implication:** avoiding clichés is not the same as surprising. Prompts and lints should also push for unexpected but apt associations.
- Across 60 LLMs and 60 people (five-sentence cue-word stories), LLMs produced "stylistically complex stories" but fell short on novelty, surprise and diversity. Expert ratings agreed with these metrics; LLM judges and non-experts rated LLM stories *more* creative — [Ismayilzada et al., ICCC 2025](https://arxiv.org/abs/2411.02316).
- Creativity Index measures how much of a text can be reconstructed from web snippets. Professional authors scored on average 66.2% higher than LLMs, and **alignment reduced LLMs' Creativity Index by 30.1%** on average — [Lu et al. 2024](https://arxiv.org/abs/2410.04265).

#### Slop words, phrases, names; grammatical fingerprints
- **Antislop**: in 2,000 creative-writing samples from gemma-3-12b, compared with human baselines:

  | Pattern | Over-representation |
  |---|---|
  | name "Elara" | 85,513× |
  | "unsettlingly" | 3,833× |
  | "shimmered" | 2,882× |
  | "stammered" | 2,043× |
  | "heart hammered ribs" | 1,192× |
  | "voice trembling slightly" | 731× |
  | "said voice devoid" | 693× |
  | "felt profound sense" | 550× |

  Categories: fixed character names ("Elara," "Kael"), sensory clichés ("voice barely above a whisper"), intensifiers ("a profound sense"), go-to descriptives. "It's not X, it's Y" was 6.3× more prevalent than in human writing in some models. **Slop fingerprints cluster within model families but differ between families** — [Paech et al. 2025](https://arxiv.org/abs/2510.15061). **Implication:** slop lists must be **model-specific** and should cover names and multi-word patterns, not just single words. Profile the deployed model's Korean output against a human Korean baseline.
- Instruction-tuned LLMs, compared with humans [older models] — [Reinhart et al., PNAS 2025](https://arxiv.org/abs/2410.16107):
  - Present participial clauses at 2–5× the human rate (GPT-4o 5.3×, e.g., "Bryan, leaning on his agility, dances around the ring, evading Show's heavy blows").
  - "That" clauses as subjects 2.6×; nominalizations 2.1×; phrasal coordination 1.9×.
  - GPT-4o and 4o Mini used camaraderie, palpable, tapestry and intricate at more than 100× the human rate. "Tapestry" appeared in 23% of GPT-4o outputs and "amidst" in 27%.
  - Differences were larger for instruction-tuned than base models and persisted at larger sizes.
  - **Implication:** linter rules should cover grammatical habits (participle chains, nominalization density), not only vocabulary.
- Lexical over-representation ("delve," "intricate," "underscore"; 21 focal words): no evidence pointed to architecture or training data, while the results were "consistent with RLHF playing a role" — [Juzek & Ward, COLING 2025](https://arxiv.org/abs/2412.11385).
- A classifier tells ChatGPT, Claude, Grok, Gemini and DeepSeek apart with 97.1% accuracy. The idiosyncrasies are rooted in word-level distributions and persist after rewriting, translation or summarization by another LLM — [Sun et al., ICML 2025](https://arxiv.org/abs/2502.12150). **Implication:** a second LLM paraphrasing the text will not remove a model's fingerprint. Targeted span edits plus lints are needed.
- 76% of syntactic templates in model text appear in pretraining data, against 35% for human text, and they are not overwritten by RLHF — [Shaib et al., EMNLP 2024](https://arxiv.org/abs/2407.00211).
- EQ-Bench's **slop score** matches text "against a master slop list derived from over-represented words & phrases." Its repetition metric "sums the frequencies of the top most common words, bigrams and trigrams" — [EQ-Bench Creative Writing](https://eqbench.com/creative_writing.html). **Implication:** two cheap, reproducible linter metrics to adapt to Korean: matching against a slop list, and the top-k n-gram repetition mass.

#### Narrative-level failures
- Compared with human stories, LLM stories (GPT-4) "adopt homogeneously happier, less complex story arcs," "introduce plot turning points earlier in the timeline," and have "less suspense or fewer setbacks." GPT-4 wrote the "Man in a Hole" arc more than half the time [older models] — [Tian et al., EMNLP 2024](https://arxiv.org/abs/2407.13248). **Implication:** the plan should specify arc type, setbacks, and turning-point positions, and allow unresolved or negative endings.
- **Narrative Flattening** (EMNLP 2026):
  - Design: OLMo 32B checkpoints (Base, SFT, DPO, RLVR) compared with matched human continuations.
  - Post-training compresses variation: "thematic transitions become more uniform, high-intensity emotions give way to neutrality, and stylistic diversity across stories shrinks."
  - **Professional literary fiction (The New Yorker) is compressed most**; public-platform stories show smaller gaps.
  - Source: [Li et al. 2026](https://arxiv.org/abs/2605.27878).
  - **Implication:** literary mode needs explicit permission for, and a request for, emotional intensity, tonal swings and stylistic variation. Web-novel mode is closer to model defaults.
- Aligned models favor "pleasant writing over creative originality." Across models, better scores on common benchmarks correlated with worse performance on the creativity tasks — [West & Potts 2025](https://arxiv.org/abs/2505.00047).
- 2026 preprint: "fiction frequently depends on behaviors that assistant-tuned models are explicitly trained to avoid, particularly deception, moral ambiguity, and unreliable narration." The resulting stories are "structurally correct while remaining stylistically generic, overly explanatory." The authors claim a purpose-built, book-scale fine-tuned model beats GPT-5.5 and Claude Opus 4.8 on their writing-quality evaluation (unreviewed) — [Zierstek et al. 2026](https://arxiv.org/abs/2605.17064). **Implication:** the system prompt should explicitly license lying characters, moral ambiguity, unlikeable perspectives and unreliable narrators.
- SAGE, an LLM-based literary evaluator (2026, unreviewed), ranks Canonical > Pulp > LLM stories. The largest gaps are in cultural critique and philosophical depth (Cohen's d > 2.4); emotional representation has a smaller gap (d = 1.68) — [Wang & Zhou 2026](https://arxiv.org/abs/2605.07102).
- In Agents' Room, machine stories were more generic in sentence structure, and the longest outputs were the most repetitive — [Huot et al.](https://arxiv.org/abs/2410.02603).

#### Homogenization (within and across models and users)
- "Write a metaphor about time": 50 responses from each of 25 models formed just two clusters, a dominant "time is a river" and a smaller "time is a weaver."
- Even with high-stochasticity decoding, in 79% of cases the average similarity among one model's responses exceeded 0.8.
- Different models produced verbatim overlapping phrases.
- LMs, reward models and LM judges were less calibrated where annotators' idiosyncratic preferences diverge.
- Source: [Jiang et al., NeurIPS 2025 D&B (Artificial Hivemind)](https://arxiv.org/abs/2510.22954).
- **Implication:** figurative "freshness" cannot be left to sampling. The prompt or pipeline has to explicitly steer away from the modal metaphor.
- **Multilingual Artificial Hivemind** (Oct 2026): 23 open-weight models, 13 languages. The effect is "pervasive but uneven"; in 12 of 13 languages, pairs of LLMs agree more than pairs of humans. The size of the homogenization gap tracks the model's competence in the language. **Korean was not included** (the language set has "no language of South Asia, no Arabic, Turkish, Korean, or Indonesian"); Japanese and Chinese were — [Davey et al. 2026](https://arxiv.org/abs/2610.16755). **Implication:** homogeneity in Korean output is untested. Expect the effect, and measure it.
- Plot elements are "echoed" across generations and across different LLMs (GPT-4, LLaMA-3), while human plots are rarely recreated — [Xu et al., PNAS 2025](https://arxiv.org/abs/2501.00273).
- LLM responses on standardized creativity tests are much more similar to other LLMs' responses than humans' responses are to each other — [Wenger & Kenett 2025](https://arxiv.org/abs/2501.19361).
- Across 20 models, NoveltyBench found significantly less diversity than human writers, and "larger models within a family often exhibit less diversity" — [Zhang et al. 2025](https://arxiv.org/abs/2504.05228).
- RLHF significantly reduces output diversity compared with SFT — [Kirk et al. 2023](https://arxiv.org/abs/2310.06452). Aligned Llama-2 shows lower token entropy and "attractor states" — [Mohammadi 2024](https://arxiv.org/abs/2406.05587).
- Proposed cause: "typicality bias in preference data, whereby annotators systematically favor familiar text" — [Zhang et al. 2025 (Verbalized Sampling)](https://arxiv.org/abs/2510.01171).

#### Interventions with measured effect (details in Q1/Q4/Q5)
- Span-level editing guided by the expert taxonomy: average rank 1.99 vs 2.55 for unedited text — [LAMP](https://arxiv.org/abs/2409.14509). Edit, then rank with a reward model: 66–72% expert preference — [AI-Slop to AI-Polish](https://arxiv.org/abs/2504.07532).
- Sampler-level slop suppression (backtracking) handles 8,000+ patterns "while maintaining quality." Simple token banning "becomes unusable at just 2,000." FTPO fine-tuning cut slop by 90% — [Antislop](https://arxiv.org/abs/2510.15061).
- Explicit arc and turning-point information gave +45% diversity and +40% suspense/engagement — [Tian et al.](https://arxiv.org/abs/2407.13248).
- Verbalized Sampling gave 1.6–2.1× diversity in creative writing — [VS](https://arxiv.org/abs/2510.01171). In-context regeneration roughly matched human diversity — [NoveltyBench](https://arxiv.org/abs/2504.05228).
- Fine-tuning on an author's complete works removed detectable AI quirks (detector flag rate 3% vs 97%) — [Chakrabarty, Ginsburg & Dhillon 2025](https://arxiv.org/abs/2510.13939).
- Training-level diversity:
  - Deviation-weighted DPO/ORPO: an 8B model reached human-dataset-level diversity with quality similar to GPT-4o and DeepSeek-R1 — [Chung et al. 2025](https://arxiv.org/abs/2503.17126).
  - CrPO beat GPT-4o on novelty, diversity and surprise — [Ismayilzada et al., Findings of EMNLP 2025](https://arxiv.org/abs/2505.14442).
  - DivLM (Oct 2026): +9% diversity across genre, tone, style and named entities — [Solati Dehkordi & Lampos 2026](https://arxiv.org/abs/2610.06729).
  - Avoidance Decoding: up to 2.6× diversity and 30% less repetition across story branches — [Park, Yang & Jung 2025](https://arxiv.org/abs/2509.02170).

### Inferences
- Defects are **systematic and shared across vendors**: LAMP found no difference between GPT-4o, Claude-3.5 and Llama-3.1, and the Hivemind work found verbatim overlap across models. So a fixed linter rule set catches a lot, and model-specific slop profiling adds precision.
- There is a **tension** between WriterProgram's goal of "rich, varied literary vocabulary and fresh figurative language" and expert judgments. Experts penalize purple prose, mixed metaphors and ornamental wording (LAMP; EQ-Bench criteria in Q3). Freshness should come from **specificity and unexpected but apt pairings**, not from a higher density of rare words or metaphors.
- Narrative flattening and the positivity bias point to the same prompt need: ask explicitly for tension, setbacks, intensity peaks and unresolved endings, and license moral ambiguity.

### Gaps
- No Korean-language measurement was found of slop over-representation, grammatical fingerprints or homogenization. All slop and fingerprint numbers above are for English.
- No study was found measuring sentimentality or "telling" defects separately in web-novel genres (e.g., romance-fantasy, regression or "system" novels) as opposed to literary fiction.

---

## 3. What creative-writing benchmarks and human evaluations exist, and what do they reveal (models, prompts, LLM-as-judge reliability)?

### Takeaway
Expert human evaluations consistently put LLM fiction well below professional or literary writers (TTCW, Pron vs Prompt, Agents' Room, MFA readers). LLM fiction often matches or beats *amateur or average* writers (PDS vs Reddit; lay readers). The evaluator's expertise and the human baseline drive the conclusion.

Generic LLM judges are **unreliable for literary quality**:
- Agreement with experts is roughly zero (TTCW kappas), or near chance on writing-quality pairs (WQ).
- They prefer AI and their own text.
- Best off-the-shelf judges reach ~73% agreement on Reddit story pairs; trained reward models ~74–78%.
- Chain-of-thought *hurts* judging accuracy.

Judges become more usable with specific criteria, pairwise comparison, length truncation and position swapping.

### Cited Findings

#### Benchmarks and what they measure
- **EQ-Bench Creative Writing v3**:
  - 32 prompts × 3 iterations; generation at temperature 0.7, min_p 0.1.
  - Rubric scoring plus pairwise Elo (Glicko-based). The pairwise judge is Claude Sonnet 4.6 (since 2026-03-01; previously Sonnet 4).
  - Pairwise criteria: character authenticity and insight; interesting and original; writing quality; coherence; instruction following; world and atmosphere; "avoids clichés in characters, dialogue, and plot"; "avoids flowery verbosity and showy vocabulary"; "avoids gratuitous metaphor or poetic overload."
  - Bias controls: outputs truncated at 4,000 characters; both orders run.
  - The site states it does *not* control for judge self-preference, that positivity bias is unclear, that judges "may favour some slop," that it is English-only, and that scores are "a rough guide."
  - Leaderboard values did not load in the fetched page.
  - Sources: [EQ-Bench about](https://eqbench.com/about.html); [EQ-Bench Creative Writing](https://eqbench.com/creative_writing.html).
  - **Implication:** the pairwise criteria are a ready critique and linter checklist that explicitly penalizes showy vocabulary and metaphor overload.
- **WritingBench**:
  - 6 domains, 100 subdomains; the Literature & Arts domain has 183 queries.
  - Query-dependent criteria plus a fine-tuned critic achieved 84% human alignment, against 67% and 58% for static-criteria baselines.
  - Reasoning models led the creative domain.
  - Source: [Wu et al. 2025](https://arxiv.org/abs/2503.05244).
  - **Implication:** if WriterProgram uses an automatic critic, generate scene-specific criteria from the author's brief rather than using one fixed rubric.
- **LitBench**:
  - Test set: 2,480 debiased, human-labeled Reddit story pairs (length-balanced); 43,827 training pairs.
  - Best zero-shot judge: Claude-3.7-Sonnet at 73%. Other strong OpenAI, Anthropic and DeepSeek models sit "in the 70% range"; small open models 56–60%.
  - Trained Bradley-Terry and generative reward models reach 78%. Adding reasoning to generative reward models *lowered* accuracy to 72% ("introduces textual noise when judging narrative quality").
  - Without length debiasing, a reward model "strongly prefer[s] the longer of the two stories."
  - Source: [Fein et al. 2025](https://arxiv.org/abs/2507.00769).
- **Writing Quality Benchmark (WQ)**: 4,729 judgments consolidated from 5 datasets.
  - Zero-shot LLMs scored "below or a few percentage points above a random baseline of 50%," especially on AI-vs-human pairs. o1 and R1 did not significantly improve.
  - The trained WQRM reached 74%.
  - Source: [Chakrabarty, Laban & Wu 2025](https://arxiv.org/abs/2504.07532).
- **TTCW**: 14 expert tests (listed in Q1); 48 stories, 10 creative writers; expert stories pass 84.7% of tests vs 9–30% for LLMs [older models] — [Chakrabarty et al., CHI 2024](https://arxiv.org/abs/2309.14556).
- **Other benchmarks and resources**:
  - NoveltyBench: diversity plus quality across repeated generations — [Zhang et al. 2025](https://arxiv.org/abs/2504.05228).
  - Infinity-Chat: 26K open-ended queries, 31,250 human annotations — [Jiang et al.](https://arxiv.org/abs/2510.22954).
  - CS4: creativity under increasing constraints — [Atmakuru et al.](https://arxiv.org/abs/2410.04197).
  - Psychological Depth Scale: Krippendorff's α 0.72 among humans — [Harel-Canada et al.](https://arxiv.org/abs/2406.12680).
  - Tell Me A Story (Agents' Room) — [Huot et al.](https://arxiv.org/abs/2410.02603).

#### Human evaluations: LLM vs human writers
- **Pron vs Prompt**: GPT-4 Turbo against Patricio Pron, with 5,400 assessments by literary critics and scholars.
  - "LLMs are still far from challenging a top human creative writer."
  - GPT-4 did better with Pron's titles.
  - Its creative writing "degrade[s] in Spanish (with respect to English)."
  - Its style became "recognizable, specially after some exposure."
  - Temperature was fixed at 1; higher values "occasionally impacted on grammaticality (particularly with Spanish texts)" [older models].
  - Source: [Marco et al., EMNLP 2024](https://arxiv.org/abs/2407.01119).
  - **Implication for Korean:** expect a quality penalty outside English, and expect repeat readers (serial readers) to learn the model's style.
- **MFA vs lay readers (in-context, i.e. prompted, outputs)**: MFA-trained readers disfavored AI for fidelity (OR=0.16) and quality (OR=0.13). College-educated general readers showed no fidelity preference (OR=1.06) and *favored* AI for quality (OR=1.82). After fine-tuning, both groups preferred the AI text — [Chakrabarty, Ginsburg & Dhillon 2025](https://arxiv.org/abs/2510.13939). **Implication:** lay-reader preference and expert preference diverge sharply for prompted outputs.
- **PDS**: GPT-4 stories "either surpassed or were statistically indistinguishable from highly-rated human-written stories sourced from Reddit" on psychological depth [older models] — [Harel-Canada et al.](https://arxiv.org/abs/2406.12680). This contrasts with TTCW, Pron and the MFA readers above. The difference is in baseline (amateur Reddit vs professional/literary) and rater expertise.
- **Earlier comparisons**:
  - In 2023, some commercial LLMs matched or slightly outperformed human writers in most dimensions on an absurd-premise story (Ignatius J. Reilly vs a pterodactyl); "humans retain an edge in creativity" [older models] — [Gómez-Rodríguez & Williams, Findings of EMNLP 2023](https://arxiv.org/abs/2310.08433).
  - A fine-tuned BART-large outscored average human writers (2.11 vs 1.85) — [Marco, Rello & Gonzalo, COLING 2025](https://arxiv.org/abs/2409.11547).
- **Agents' Room**: human-written stories were preferred over all machine systems; the gap was smallest in language use — [Huot et al.](https://arxiv.org/abs/2410.02603).

#### LLM-as-judge reliability for fiction
- **TTCW**: Cohen's κ between LLM-administered tests and experts averaged 0.016 (GPT-3.5), 0.035 (GPT-4) and −0.006 (Claude) — "none of the LLMs produce assessments that correlate positively with expert assessments" [older models] — [Chakrabarty et al.](https://arxiv.org/abs/2309.14556).
- **Near chance on writing quality**, with AI-vs-human pairs worst; this "confirms prior findings that LLMs used in LLM-as-a-judge settings tend to prefer AI-generation over human-writing" — [WQ](https://arxiv.org/abs/2504.07532).
- **Prefer LLM stories**: LLM judges (Claude, Gemini, GPT-4) agreed with each other well, yet rated LLM stories as more creative than human ones, unlike experts — [Ismayilzada et al.](https://arxiv.org/abs/2411.02316).
- **Self-preference** is linked to self-recognition — [Panickssery et al.](https://arxiv.org/abs/2404.13076).
- **System-level only**: LLMs beat older automatic measures for system-level story evaluation but "struggle at providing satisfactory explanations" — [Chhun, Suchanek & Clavel, TACL 2024](https://arxiv.org/abs/2405.13769).
- **Do not track writers**: LLM ratings of story summaries correlated poorly with the authors' own ratings — [Subbiah et al.](https://arxiv.org/abs/2403.01061).
- **Miscalibrated on taste**: LM judges and reward models are less calibrated where human preferences are idiosyncratic — [Jiang et al.](https://arxiv.org/abs/2510.22954).
- **Mitigations that helped**:
  - Query-specific criteria (84% vs 58–67%) — [WritingBench](https://arxiv.org/abs/2503.05244).
  - Mixture-of-Personas judging (Spearman 0.51 with humans) — [PDS](https://arxiv.org/abs/2406.12680).
  - Length truncation and position swapping — [EQ-Bench](https://eqbench.com/about.html).
  - No CoT for judging — [LitBench](https://arxiv.org/abs/2507.00769).

#### What benchmarks reveal about models and prompts
- In LAMP, no model family (GPT-4o, Claude-3.5-Sonnet, Llama-3.1-70B) outperformed the others in expert-judged writing quality — [LAMP](https://arxiv.org/abs/2409.14509).
- Reasoning variants lead in WritingBench's literature domain — [WritingBench](https://arxiv.org/abs/2503.05244). Larger models gain more from Verbalized Sampling — [VS](https://arxiv.org/abs/2510.01171). Larger models within a family are often less diverse — [NoveltyBench](https://arxiv.org/abs/2504.05228). Better general benchmark scores correlate with worse creativity/randomness — [West & Potts](https://arxiv.org/abs/2505.00047).

### Inferences
- An automated "quality score" in WriterProgram should not come from a generic LLM grader. Prefer:
  - **defect detection** (taxonomy categories, slop and repetition counts), which is more reliable than holistic judgment;
  - **pairwise choice by the author** among candidates.
- Use any LLM judge only for narrow, criteria-specific checks (e.g., "does this sentence restate the previous one?"), not for "is this literary?".
- Benchmarks that favor lay preferences may reward the very "AI-ness" literary novelists reject. Design evaluations around the target user group (web-novel readers vs literary editors).

### Gaps
- No Korean creative-writing benchmark was covered here (other researchers handle Korean). EQ-Bench is English-only.
- LitBench's per-model judge accuracies other than Claude-3.7-Sonnet (73%) could not be extracted (table rendering). Current EQ-Bench leaderboard values were not retrievable.
- No published human evaluation of 2026 frontier models (Claude Opus 5.x, GPT-5.5) against professional novelists was found, apart from vendor-adjacent or unreviewed claims (e.g., Zierstek et al. 2026).

---

## 4. Which techniques increase lexical diversity or reduce repeated phrasing, and how do sampling settings interact with prompting?

### Takeaway
Prompt-level **distribution requests** (Verbalized Sampling, shown to keep judged quality) and **in-context regeneration** (showing previous outputs and asking for something different) measurably raise diversity. "Be unique" system prompts and paraphrasing barely help. Banned-word *instructions* are weak or backfire, while **sampler-level backtracking** suppresses thousands of patterns. Temperature is a weak creativity lever that trades away coherence, and min-p's benefits are disputed. Diversity methods compose with decoding settings. Raw lexical-diversity metrics (TTR) are a poor proxy for quality.

### Cited Findings

#### Prompt-level diversity techniques
- **Verbalized Sampling (VS)**:
  - Method: ask for a distribution, e.g., "Generate 5 jokes about coffee and their corresponding probabilities." Variants: VS-Standard, VS-CoT ("Think step-by-step, then tell 5 jokes with probabilities"), VS-Multi (multi-turn "Tell 5 more with probabilities").
  - Creative writing diversity rose 1.6–2.1× over direct prompting.
  - Human-rated story diversity (1–4 scale): Direct 2.74, Sequence ("tell 5 stories") 2.76, VS-Standard 3.06. Quality win-rates were comparable, as judged by Claude-3.7-Sonnet with the Creative Writing v3 rubric.
  - VS-CoT and VS-Multi add more diversity and can *improve* quality on larger models. Larger models benefit 1.5–2× more.
  - Diversity is tunable in the prompt ("Generate five responses with probabilities below {threshold}").
  - VS is "orthogonal to the decoding strategy": it gives the best quality–diversity frontier at every temperature, and higher temperatures raise diversity at a cost in quality.
  - Source: [Zhang et al. 2025](https://arxiv.org/abs/2510.01171) (preprint).
  - **Implication:** use VS for *micro-decisions* (candidate metaphors, opening lines, plot turns, character names), sample from the low-probability tail, then let the author or the pipeline choose.
- **NoveltyBench strategies**:
  - A system prompt ("You are a producer of unique answers…") and paraphrasing the prompt were "only marginally effective — indicating that asking a model for 'creative outputs' does not work very well."
  - **In-context regeneration** (keep previous answers in context and ask for a different one) was most successful, roughly matching human diversity.
  - Source: [Zhang et al. 2025](https://arxiv.org/abs/2504.05228).
  - **Implication:** feed back the novel's already-used images, names and signature phrases as "already used; produce something different," rather than writing "be original."
- With GPT-4, prompt engineering substantially increased idea diversity, and chain-of-thought prompting gave the highest diversity, close to groups of humans (product ideas, not fiction) — [Meincke, Mollick & Terwiesch 2024](https://arxiv.org/abs/2402.01727).
- Role-played multi-agent discussion raised creativity scores — [Lu et al. 2024](https://arxiv.org/abs/2405.06373). Meta-persona anchoring cut similarity from 0.85 to 0.65 (unreviewed; combined with extreme temperature) — [Fu et al. 2026](https://arxiv.org/abs/2608.02618).

#### Banned lists, samplers, post-processing
- **Antislop sampler**:
  - Detects banned words, phrases or regexes during generation, backtracks to the first token, lowers its probability and resamples.
  - Suppresses 8,000+ patterns while maintaining quality. Plain token banning "becomes unusable at just 2,000," and causes collateral damage (banning "catatonic" bans every word starting with the token "cat").
  - Throughput costs: −69% at a 1,000-item banlist and −96% at 8,000 (worst case, "overkill for most real-world usage").
  - FTPO fine-tuning cuts slop by 90% while maintaining writing quality, whereas DPO degraded writing quality and lexical diversity.
  - Source: [Paech et al. 2025](https://arxiv.org/abs/2510.15061).
  - **Implication:** a hosted model (e.g., Claude via API) cannot use logit-level backtracking. The practical equivalent is **detect (linter) → targeted span rewrite**.
- Pink Elephant: telling a model not to mention X can increase mentions of X in weaker models — [Castricato et al. 2024](https://arxiv.org/abs/2402.07896).

#### Generate many, then select
- Generating and ranking multiple candidate revisions with a trained writing-quality reward model gave a 66% expert preference (72.2% when the reward gap was over 1 point) — [Chakrabarty, Laban & Wu 2025](https://arxiv.org/abs/2504.07532). Reranking continuations for coherence and relevance contributed to Re3's gains — [Yang et al. 2022](https://arxiv.org/abs/2210.06774).
- Caveat: selecting with a generic LLM judge is near chance on writing quality — [WQ](https://arxiv.org/abs/2504.07532); [LitBench](https://arxiv.org/abs/2507.00769).

#### Sampling settings and their interaction with prompting
- Temperature is "weakly correlated with novelty, and unsurprisingly, moderately correlated with incoherence," with no relationship to cohesion or typicality. The "creativity parameter" claim is "far more nuanced and weak than suggested" — [Peeperkorn et al., ICCC 2024](https://arxiv.org/abs/2405.00492).
- Even under high-stochasticity decoding, 79% of queries had average within-model similarity above 0.8 — [Jiang et al.](https://arxiv.org/abs/2510.22954). The Echoes authors cite Peeperkorn when explaining their temperature choice: the influence of temperature on creativity "is weak" — [Xu et al.](https://arxiv.org/abs/2501.00273).
- **Min-p** (ICLR 2025 oral) claimed better quality *and* diversity, especially at high temperatures. It recommended min-p 0.05–0.1 and using min-p as the primary truncation method, not combined with others — [Nguyen et al.](https://arxiv.org/abs/2407.01082). **Contested**: a re-analysis found that the original human evaluation "omitted data, conducted statistical tests incorrectly," and that "min-p did not outperform baselines in quality, diversity, or a trade-off." The authors' own new human evaluation also showed no improvement — [Schaeffer, Kazdan & Denisov-Blanch 2025](https://arxiv.org/abs/2506.13681). **Implication:** treat min-p as optional tuning, not an evidence-backed creativity fix.
- Pron vs Prompt fixed temperature at 1.0 because higher values hurt grammaticality, "particularly with Spanish texts" — [Marco et al.](https://arxiv.org/abs/2407.01119). EQ-Bench generates at temperature 0.7 with min_p 0.1 — [EQ-Bench](https://eqbench.com/about.html). **Implication:** for a non-English language, keep temperature moderate and get diversity from prompting and selection.
- Avoidance Decoding penalizes similarity to previous outputs: up to 2.6× diversity and −30% repetition across story branches (needs logit access) — [Park et al. 2025](https://arxiv.org/abs/2509.02170).

#### Cautions: lexical diversity is not quality
- "Lexical diversity inversely correlates with creative quality (TTR paradox), suggesting high diversity signals compensatory behavior for logical deficiencies" (Chinese creative writing) — [Li et al. 2025 (COIG-Writer)](https://arxiv.org/abs/2510.14763).
- Instruction-tuned LLMs already write informationally dense, noun-heavy prose; "other research links these features to increased lexical diversity in generated text, as well as human judgments of linguistic mastery" — [Reinhart et al.](https://arxiv.org/abs/2410.16107). Experts nonetheless cut purple prose and awkward word choice — [LAMP](https://arxiv.org/abs/2409.14509). EQ-Bench's criteria penalize "flowery verbosity and showy vocabulary" — [EQ-Bench](https://eqbench.com/about.html).
- Human stories have higher unique-word ratios and fewer repeated trigrams than machine stories. The "repetition" that matters is cross-story and within-story n-gram reuse, which rises with output length — [Huot et al.](https://arxiv.org/abs/2410.02603).

### Inferences
- Different jobs need different tools:
  - **Within-text repetition and slop** → linter (n-gram repetition mass, model-specific slop list, syntactic-template rules) plus span rewrite.
  - **Cross-chapter and cross-work sameness** (names, recurring metaphors, scene templates) → a memory of used expressions, fed back in-context and checked by the linter.
  - **Idea- and metaphor-level freshness** → VS-style candidate generation with a low-probability threshold, then selection.
- "Vocabulary richness" should be measured as **avoiding over-represented items and repeated n-grams**, not as maximizing TTR or rare-word density.
- On Claude via API, sampling knobs are limited and weak. Diversity should be engineered through prompts and the pipeline.

### Gaps
- VS was tested on short outputs (poems, short stories, jokes). No study was found of VS for long-form chapter prose, or of its cost in that setting.
- No study measured how well a prompt-level "already-used expressions" list (in-context regeneration) avoids *specific phrases* in long fiction, as opposed to semantic diversity of short answers.
- Which samplers hosted APIs expose was not researched here (vendor-docs researcher).

---

## 5. Can LLMs imitate an author's (implicit) style from exemplars, and how does fine-tuning compare with prompting for literary style?

### Takeaway
In-context exemplars get a recognizable approximation of *structured or surface* style, but fall short of expert-level literary voice. Experts strongly reject prompted emulations, outputs stay detectable as AI, and adding more demonstrations helps little. Fine-tuning, even cheap LoRA or API fine-tuning on an author's corpus, captures lexical and syntactic style far better, removes AI quirks, and in one preregistered study was preferred over MFA writers by experts. For a prompting-only product, exemplars set the register while a defect-removal pass carries most of the quality load.

### Cited Findings
- **Fine-tuning vs in-context emulation of literary authors** (preregistered) [older models for the in-context arm: GPT-4o, Claude 3.5 Sonnet, Gemini 1.5 Pro]:
  - Task: up to 450-word excerpts emulating 50 award-winning authors.
  - Raters: 28 MFA-trained readers and 516 general readers.
  - In-context prompting was strongly disfavored by MFA readers (fidelity OR=0.16, quality OR=0.13).
  - Fine-tuning ChatGPT (GPT-4o) on each author's complete works reversed this. MFA readers favored the AI text (fidelity OR=8.16, quality OR=1.87); general readers more so (fidelity 16.65, quality 5.42).
  - Detectors flagged 3% of fine-tuned outputs vs 97% of prompted ones.
  - Median fine-tuning cost was $81 per author.
  - For prompted outputs, expert quality and style ratings were highly correlated, suggesting experts penalize "AI-ness" in style.
  - Source: [Chakrabarty, Ginsburg & Dhillon 2025](https://arxiv.org/abs/2510.13939) (preprint).
  - **Implication:** prompting cannot reach literary-grade voice emulation. Author-level fine-tuning is the evidence-backed route if WriterProgram ever offers "write in my voice" (with consent and copyright care).
- **Personal-style imitation from a few examples**:
  - Scale: 40,000+ generations per model, 400+ real authors; news, email, forums, blogs.
  - LLMs "can approximate user styles in structured formats like news and email" but "struggle with nuanced, informal writing in blogs and forums."
  - Outputs "default to an average, generic tone and remain readily detectable as AI-written."
  - "Increasing the number of demonstrations offers limited gains"; adding examples "affects the four metrics very little." Exemplar selection by content or length similarity did not reliably help.
  - Source: [Wang et al., Findings of EMNLP 2025](https://arxiv.org/abs/2509.14543).
  - **Implication:** more exemplars are not the fix. Choose a few varied, representative passages and accept that the voice will regress toward generic.
- On academic essays: few-shot prompting gave up to 23.5× higher style-matching accuracy than zero-shot; **completion-style prompting** (continuing the author's text) reached 99.9% agreement on their verification metric. Matched LLM outputs still had much lower perplexity (15.2 vs 29.5 for humans) — "stylistic fidelity and statistical detectability are separable" — [Jemama & Kumar, IEEE UEMCON 2025](https://arxiv.org/abs/2509.24930). **Implication:** continuing the author's own preceding manuscript is the strongest prompt-only form of style conditioning. This fits WriterProgram, which works inside the author's manuscript. Expect the result to remain statistically "smoother" than the human text (lower perplexity), which is a likely source of the flatness readers notice.
- LoRA fine-tuning of LLaMA-2 on 10 authors (StyleTunedLM) gave lexical, syntactic and surface alignment with each author and "is more effective at capturing the style" than "prompt engineering and few-shot learning," while avoiding memorizing content — [Liu, Diddee & Ippolito 2024](https://arxiv.org/abs/2409.04574).
- Professional writers reported that AI writing tools "struggle to preserve style and authorial voice" [older models] — [Ippolito et al. 2022](https://arxiv.org/abs/2211.05030). Without stylistic constraints GPT-4's style becomes "recognizable, specially after some exposure" — [Marco et al. 2024](https://arxiv.org/abs/2407.01119). Model idiosyncrasies survive rewriting or translation by another LLM — [Sun et al. 2025](https://arxiv.org/abs/2502.12150).
- **Specialized training beats generalists**:
  - Weaver models (1.8B–34B) pre-trained and aligned for creative writing outperformed much larger generalist LLMs on their benchmark [older, 2024] — [Wang et al. 2024](https://arxiv.org/abs/2401.17268).
  - A book-scale "prompt-to-book" fine-tuned model is claimed to beat GPT-5.5 and Claude Opus 4.8 (unreviewed) — [Zierstek et al. 2026](https://arxiv.org/abs/2605.17064).
  - GRPO with a frontier-judge rubric reward plus human-reference injection let a 9B model match a 27B model in blind human evaluation — [Rajendhran et al., EMNLP 2026](https://arxiv.org/abs/2606.04095).
- **The root cause is post-training**: base-to-aligned checkpoints progressively flatten thematic, affective and stylistic variation — [Li et al. 2026](https://arxiv.org/abs/2605.27878). Alignment lowers the Creativity Index by 30.1% — [Lu et al. 2024](https://arxiv.org/abs/2410.04265).
- **Non-English transfer**:
  - GPT-4's creative writing degrades in Spanish relative to English — [Marco et al.](https://arxiv.org/abs/2407.01119).
  - Creative capabilities were "culturally-bound with no cross-lingual transfer (89.26pp gap between Chinese and English performance)" in a Chinese fine-tuning study — [Li et al. 2025](https://arxiv.org/abs/2510.14763).
  - Homogenization gaps track model competence per language — [Davey et al. 2026](https://arxiv.org/abs/2610.16755).
  - Genre-attribute-guided data (13 creative genres) beat story-only data for fine-tuning — [Chang et al., CIKM 2026](https://arxiv.org/abs/2608.13947).

### Inferences
- For a prompt-only Korean system, the best available style conditioning is:
  1. the author's own preceding text (completion-like continuation);
  2. 3–5 varied Korean exemplar passages for register;
  3. explicit, reasoned style targets;
  4. a defect-removal pass, which matters most because detected "AI-ness" (cliché density etc.) drives expert dispreference.
- English-language findings probably *understate* the difficulty in Korean (quality penalty outside English; creative skill does not transfer across languages). Korean-native exemplars and Korean-specific lints are needed rather than translated English rules.

### Gaps
- No study of literary style imitation in Korean, and none of web-novel genre-voice imitation, was found.
- No head-to-head comparison of a *detailed written style description* vs exemplars vs completion for literary fiction under expert evaluation was found.

---

## 6. Synthesis: evidence-backed prompt patterns and anti-patterns (for the Korean fiction system prompt, skill, and linter)

### Takeaway
Treat quality as a **pipeline**: specific human anchors → explicit narrative plan → sectioned drafting with varied exemplars and reasoned positive targets → diversity for micro-choices → span-level defect removal by taxonomy + linter → human or calibrated selection. Avoid relying on persona flourishes, "be creative," long in-prompt ban lists, temperature, generic self-judgment, and TTR-maximization.

### Cited Findings

#### Evidence-backed patterns
- **P1. Decompose into planning and section-wise writing with role-specific calls.** Generate the plan (conflict, characters, setting, beats) separately; write scene by scene, keeping a scratchpad of state.
  - Evidence: Re3 +14%/+20% — [Yang et al. 2022](https://arxiv.org/abs/2210.06774); DOC +20.7% interestingness — [Yang et al. 2023](https://arxiv.org/abs/2212.10077); Agents' Room writing agents preferred, while single-call plan/reflect variants underperformed and long single outputs were most repetitive — [Huot et al. 2025](https://arxiv.org/abs/2410.02603); Dramatron — [Mirowski et al.](https://arxiv.org/abs/2209.14958).
  - For serials: condense the story so far, plan the next chapter, then write — [Gurung & Lapata 2025](https://arxiv.org/abs/2503.22828).
- **P2. Put explicit narrative-shape information in the plan.** Arc type; turning-point positions (not front-loaded); setbacks; whether the ending resolves; emotional valence; intended subtext.
  - Evidence: +45% diversity, +40% suspense/engagement — [Tian et al. 2024](https://arxiv.org/abs/2407.13248); Plan+Write gains in depth — [Harel-Canada et al. 2024](https://arxiv.org/abs/2406.12680).
- **P3. Anchor on specific, author-supplied material** (titles, images, situations, the author's own notes and prior text), not model-chosen defaults.
  - Evidence: better with Pron's titles — [Marco et al. 2024](https://arxiv.org/abs/2407.01119); prompt quality drives plot uniqueness — [Xu et al. 2025](https://arxiv.org/abs/2501.00273); a few constraints push off memorized paths — [Atmakuru et al.](https://arxiv.org/abs/2410.04197).
  - Caveat: content detail alone doesn't fix style — [Chakrabarty et al. 2025](https://arxiv.org/abs/2504.07532).
- **P4. Write targets positively, with reasons, in prose.** State what good looks like and why; match the prompt's own style to the desired prose.
  - Evidence: [Anthropic guidance](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-4-best-practices).
  - Cover the expert-defect categories as *positive* targets: concrete specificity over abstraction; implication over explanation; plain verbs over ornament; varied sentence structure — [LAMP](https://arxiv.org/abs/2409.14509).
- **P5. License literary behaviors that post-training suppresses.** Allow intensity peaks and tonal swings, unpleasant or morally ambiguous characters, lies, unreliable narration, unresolved or unhappy endings, and restraint (no summarizing moral).
  - Evidence: [Li et al. 2026 (Narrative Flattening)](https://arxiv.org/abs/2605.27878); [West & Potts 2025](https://arxiv.org/abs/2505.00047); [Tian et al.](https://arxiv.org/abs/2407.13248); [Zierstek et al. 2026](https://arxiv.org/abs/2605.17064) (preprint); TTCW tests on perspective/voice flexibility and subtext — [Chakrabarty et al. 2024](https://arxiv.org/abs/2309.14556).
- **P6. Use exemplars for register; continue the author's own text when possible; vary exemplars.**
  - Evidence: examples steer tone but must be diverse — [Anthropic](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-4-best-practices); completion prompting is the strongest prompt-only style conditioning — [Jemama & Kumar 2025](https://arxiv.org/abs/2509.24930).
  - Don't add exemplars expecting gains — [Wang et al. 2025](https://arxiv.org/abs/2509.14543). Don't expect expert-grade author emulation — [Chakrabarty et al. 2025](https://arxiv.org/abs/2510.13939).
- **P7. Diverge, then converge, for micro-choices.** Request k candidates with probabilities (VS) and sample from the tail (threshold in the prompt), or show prior outputs and ask for something different (in-context regeneration). Then choose (author or calibrated scorer).
  - Evidence: [VS](https://arxiv.org/abs/2510.01171) 1.6–2.1× diversity, quality kept; [NoveltyBench](https://arxiv.org/abs/2504.05228); CoT for idea diversity — [Meincke et al.](https://arxiv.org/abs/2402.01727).
  - Specifically counters modal metaphors ("time is a river") — [Jiang et al.](https://arxiv.org/abs/2510.22954).
- **P8. Keep a memory of used expressions across chapters and works** (names, metaphors, signature collocations, scene templates); feed it back as "already used" and lint against it.
  - Evidence: in-context regeneration works — [NoveltyBench](https://arxiv.org/abs/2504.05228); fixed character names (e.g., "Elara," "Kael") and model-family slop fingerprints — [Antislop](https://arxiv.org/abs/2510.15061); cross-model homogeneity — [Wenger & Kenett](https://arxiv.org/abs/2501.19361), [Jiang et al.](https://arxiv.org/abs/2510.22954).
- **P9. Run a separate, taxonomy-driven, span-level revision pass** (detect spans → rewrite only those → apply), mostly replacing and cutting.
  - Evidence: rank 1.99 vs 2.55 for unedited text; experts' edits are 74% replacements and 18% deletions — [LAMP](https://arxiv.org/abs/2409.14509); 3-step edit pipeline plus ranking at 66–72% — [Chakrabarty et al. 2025](https://arxiv.org/abs/2504.07532); criterion-specific critics with a leader — [CritiCS](https://arxiv.org/abs/2410.02428).
- **P10. Make the linter the main slop enforcement.** Use a model-specific over-representation list (words, multi-word patterns, names), top-k n-gram repetition mass, syntactic-template rules ("a mix of X and Y," "the weight of…"), grammatical-habit density (participle chains, nominalizations) and cliché density.
  - Evidence: [Antislop](https://arxiv.org/abs/2510.15061); [EQ-Bench metrics](https://eqbench.com/creative_writing.html); [LAMP templates](https://arxiv.org/abs/2409.14509); [Reinhart et al.](https://arxiv.org/abs/2410.16107); cliché density as the main AI tell — [Chakrabarty et al. 2025](https://arxiv.org/abs/2510.13939).
- **P11. Evaluate with narrow checks and human choice, not generic LLM grades.** If an LLM judge is used: pairwise, criteria-specific (generated per scene), length-controlled, position-swapped, no CoT, ideally a different model.
  - Evidence: [TTCW κ≈0](https://arxiv.org/abs/2309.14556); [WQ near chance](https://arxiv.org/abs/2504.07532); [LitBench 73% best; CoT hurts](https://arxiv.org/abs/2507.00769); [WritingBench 84% with query-specific criteria](https://arxiv.org/abs/2503.05244); [self-preference](https://arxiv.org/abs/2404.13076); [EQ-Bench controls](https://eqbench.com/about.html).
- **P12. Moderate sampling; diversity from prompting, not temperature.**
  - Evidence: temperature weakly raises novelty and moderately raises incoherence — [Peeperkorn et al.](https://arxiv.org/abs/2405.00492); high temperature still gives 79% high-similarity cases — [Jiang et al.](https://arxiv.org/abs/2510.22954); VS composes with any temperature — [VS](https://arxiv.org/abs/2510.01171); min-p benefit disputed — [Schaeffer et al.](https://arxiv.org/abs/2506.13681); temperature >1 hurt grammaticality outside English — [Marco et al.](https://arxiv.org/abs/2407.01119).
- **P13. Budget for a non-English penalty and Korean-native resources.**
  - Evidence: Spanish quality drop — [Marco et al.](https://arxiv.org/abs/2407.01119); no cross-lingual transfer of creative skill — [Li et al. 2025](https://arxiv.org/abs/2510.14763); competence–diversity link with Korean untested — [Davey et al. 2026](https://arxiv.org/abs/2610.16755).

#### Anti-patterns (evidence against)
- **A1. "Be creative / original / unique" one-liners or "unique answer" system prompts.** Only marginal effect — [NoveltyBench](https://arxiv.org/abs/2504.05228). Anthropic's own frontend guidance concedes that models "still tend to converge on common choices … across generations" even when told to vary — [Anthropic](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-4-best-practices).
- **A2. Long in-prompt banned-word lists as the main control.** Pink Elephant priming on weaker models — [Castricato et al.](https://arxiv.org/abs/2402.07896); "limited efficacy … backfire" per the Antislop authors — [Paech et al.](https://arxiv.org/abs/2510.15061); inverse scaling on negation [older models] — [Jang et al.](https://arxiv.org/abs/2209.12711). Put bans in the linter; keep the prompt list short and phrased as alternatives.
- **A3. Asking for "vivid, rich, evocative, poetic" language without restraint.** Invites purple prose and showy vocabulary, which experts cut and benchmarks penalize — [LAMP](https://arxiv.org/abs/2409.14509); [EQ-Bench criteria](https://eqbench.com/about.html). Even research critics' "expressiveness" edits produce stock phrases (e.g., "arched an incredulous eyebrow") — [CritiCS](https://arxiv.org/abs/2410.02428).
- **A4. Vague self-critique loops judged by the same model ("make it better").** Self-preference — [Panickssery et al.](https://arxiv.org/abs/2404.13076); near-chance quality judgment — [WQ](https://arxiv.org/abs/2504.07532); zero agreement with experts — [TTCW](https://arxiv.org/abs/2309.14556); the self-reflection baseline underperformed — [Agents' Room](https://arxiv.org/abs/2410.02603).
- **A5. Using temperature as the creativity knob, or raising it high** — [Peeperkorn et al.](https://arxiv.org/abs/2405.00492); [Marco et al.](https://arxiv.org/abs/2407.01119); [Jiang et al.](https://arxiv.org/abs/2510.22954).
- **A6. One-shot generation of long chapters.** Repetition grows with length — [Agents' Room](https://arxiv.org/abs/2410.02603); weaker coherence and interest than planned generation — [Re3](https://arxiv.org/abs/2210.06774), [DOC](https://arxiv.org/abs/2212.10077).
- **A7. Expecting exemplars or persona prompts to clone a literary voice** — [Chakrabarty et al. 2025](https://arxiv.org/abs/2510.13939); [Wang et al. 2025](https://arxiv.org/abs/2509.14543); persona effects random or model-dependent — [Zheng et al.](https://arxiv.org/abs/2311.10054), [Harel-Canada et al.](https://arxiv.org/abs/2406.12680). A single exemplar also risks copying of its images — [Anthropic](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-4-best-practices).
- **A8. Accepting default narrative shape** (happy arc, early turning points, resolved ending, explained emotions) — [Tian et al.](https://arxiv.org/abs/2407.13248); [LAMP exposition 18%](https://arxiv.org/abs/2409.14509); [Narrative Flattening](https://arxiv.org/abs/2605.27878).
- **A9. Over-constraining scenes.** Piling on requirements strains coherence — [CS4](https://arxiv.org/abs/2410.04197).
- **A10. Treating lay or LLM preference as a proxy for literary quality.** General readers favored prompted AI for quality while MFA readers rejected it — [Chakrabarty et al. 2025](https://arxiv.org/abs/2510.13939); LLMs and non-experts overrate LLM stories — [Ismayilzada et al.](https://arxiv.org/abs/2411.02316).
- **A11. Maximizing lexical diversity (TTR) or rare-word density as a goal.** TTR paradox — [Li et al. 2025](https://arxiv.org/abs/2510.14763); dense nominal style is already an LLM marker — [Reinhart et al.](https://arxiv.org/abs/2410.16107).
- **A12. Showing a single AI "idea" to the author as the default suggestion.** It homogenizes outputs across users — [Doshi & Hauser](https://doi.org/10.1126/sciadv.adn5290); [Anderson et al.](https://arxiv.org/abs/2402.01536); [Padmakumar & He](https://arxiv.org/abs/2309.05196). Show several divergent options instead.

### Inferences
Suggested WriterProgram architecture, inferred from the evidence above:

1. **Context ingestion**: author's notes and prior chapters → condensed story state.
2. **Plan call**: scene goal, conflict, arc and turning-point positions, ending valence, intended subtext, and 2–3 load-bearing concrete constraints.
3. **Micro-choice call (VS)**: candidate images, names and turns from the probability tail, checked against the used-expression memory.
4. **Draft call**, scene by scene, with:
   - positive, reasoned style targets in Korean prose;
   - 3–5 varied Korean exemplars;
   - the author's own preceding text;
   - explicit licenses for intensity, ambiguity and unresolved endings.
5. **Linter**: model-specific slop list, n-gram repetition, template patterns, cliché candidates, exposition-restatement heuristics.
6. **Span-level rewrite call**, limited to flagged spans and their LAMP category.
7. **Optional**: 2–3 revision candidates → author chooses (or a narrow pairwise check).

Prompt wording should prefer "write X" over "don't write Y," except for a short, high-value avoid list.

### Gaps
- No direct test of combining all these patterns in one system, and no Korean validation of any of them.
- Cost and latency trade-offs of multi-call pipelines vs single-call quality on current frontier models are unmeasured in the literature found.
- Claims from 2026 preprints (book-scale fine-tuning beating GPT-5.5/Opus 4.8; meta-persona anchoring) are unreviewed and should be treated as tentative.

#### Table of all sources

Verified depth: **F** = full text retrieved and key passages checked; **A** = abstract/metadata via arXiv API; **W** = web page opened.

| # | Title (short) | Authors | Year | Venue | URL | One-line finding | V |
|---|---|---|---|---|---|---|---|
| 1 | Art or Artifice? LLMs and the False Promise of Creativity (TTCW) | Chakrabarty, Laban, Agarwal, Muresan, Wu | 2024 | CHI 2024 | https://arxiv.org/abs/2309.14556 | LLM stories pass 9–30% of 14 expert tests vs 84.7% for New Yorker stories; LLM judges κ≈0 with experts | F |
| 2 | Can AI writing be salvaged? (LAMP) | Chakrabarty, Laban, Wu | 2025 | CHI 2025 | https://arxiv.org/abs/2409.14509 | 7-category defect taxonomy (awkward wording 28%, structure 20%, exposition 18%, cliché 17%); auto detect+rewrite ranks 1.99 vs 2.55 raw | F |
| 3 | Pron vs Prompt | Marco, Gonzalo, del Castillo, Mateo Girona | 2024 | EMNLP 2024 | https://arxiv.org/abs/2407.01119 | Top novelist beats GPT-4 on all dimensions; human titles help GPT-4; worse in Spanish; style recognizable | F |
| 4 | AI-Slop to AI-Polish? (WQ, WQRM) | Chakrabarty, Laban, Wu | 2025 | arXiv (under submission) | https://arxiv.org/abs/2504.07532 | LLM judges ≈ chance on writing quality; edit + rank with reward model preferred 66% (72.2% at large gap) | F |
| 5 | Readers Prefer Outputs of AI Trained on Copyrighted Books… | Chakrabarty, Ginsburg, Dhillon | 2025 | arXiv (under review) | https://arxiv.org/abs/2510.13939 | In-context emulation rejected by MFA readers (OR 0.13–0.16); author fine-tuning reverses this; cliché density is the main AI tell | F |
| 6 | Verbalized Sampling | Zhang, Yu, Chong, Sicilia, Tomz, Manning, Shi | 2025 | arXiv preprint | https://arxiv.org/abs/2510.01171 | Asking for k responses with probabilities gives 1.6–2.1× creative diversity, quality kept; orthogonal to temperature | F |
| 7 | Antislop | Paech, Roush, Goldfeder, Shwartz-Ziv | 2025 | arXiv | https://arxiv.org/abs/2510.15061 | Slop up to 85,513× over human rate; backtracking sampler handles 8k patterns; token bans fail at 2k; FTPO −90% slop | F |
| 8 | Turning Up the Heat: Min-p Sampling | Nguyen, Baker, Neo, Roush, Kirsch, Shwartz-Ziv | 2025 | ICLR 2025 (oral) | https://arxiv.org/abs/2407.01082 | Claims better quality and diversity at high temperature; recommends min-p 0.05–0.1 | F |
| 9 | Min-p, Max Exaggeration | Schaeffer, Kazdan, Denisov-Blanch | 2025 | arXiv | https://arxiv.org/abs/2506.13681 | Re-analysis: min-p does not beat baselines on quality or diversity | A |
| 10 | WritingBench | Wu, Mei, Yan, Li et al. | 2025 | arXiv | https://arxiv.org/abs/2503.05244 | Query-specific criteria give 84% human alignment vs 58–67%; reasoning models lead in literature | F |
| 11 | LitBench | Fein, Russo, Xiang, Jolly, Rafailov, Haber | 2025 | arXiv | https://arxiv.org/abs/2507.00769 | Best zero-shot judge 73%; trained reward models 78%; CoT lowers judge accuracy to 72% | F |
| 12 | Agents' Room | Huot, Amplayo, Palomaki, Jakobovits, Clark, Lapata | 2025 | ICLR 2025 | https://arxiv.org/abs/2410.02603 | Specialized planning and writing agents beat single-call baselines; longer outputs more repetitive | F |
| 13 | Collective Critics for Creative Story Generation (CritiCS) | Bae, Kim | 2024 | EMNLP 2024 | https://arxiv.org/abs/2410.02428 | Persona critics + leader: plan creativity 84.33 vs 57.33 over DOC | F |
| 14 | Are LLMs Capable of Generating Human-Level Narratives? | Tian, Huang, Liu, Jiang, Spangher, Chen, May, Peng | 2024 | EMNLP 2024 | https://arxiv.org/abs/2407.13248 | LLM arcs positive, turning points early; arc/turning-point info +45% diversity, +40% suspense | F |
| 15 | Measuring Psychological Depth in Language Models | Harel-Canada, Zhou, Muppalla, Yildiz, Kim, Sahai, Peng | 2024 | EMNLP 2024 | https://arxiv.org/abs/2406.12680 | Writer persona vs Plan+Write effects model-dependent; GPT-4 ≥ highly-rated Reddit stories | F |
| 16 | Is Temperature the Creativity Parameter of LLMs? | Peeperkorn, Kouwenhoven, Brown, Jordanous | 2024 | ICCC 2024 | https://arxiv.org/abs/2405.00492 | Temperature weakly linked to novelty, moderately to incoherence | A |
| 17 | Artificial Hivemind | Jiang, Chai, Li, Liu, Fok, Dziri, Tsvetkov, Sap et al. | 2025 | NeurIPS 2025 D&B (oral) | https://arxiv.org/abs/2510.22954 | 25 models' time metaphors collapse to "river" or "weaver"; 79% of cases above 0.8 similarity even at high temperature | F |
| 18 | The Multilingual Artificial Hivemind | Davey, Mishra, Adelöw, Liu | 2026 | arXiv | https://arxiv.org/abs/2610.16755 | Homogenization across 13 languages tracks competence in the language; Korean not tested | F |
| 19 | NoveltyBench | Zhang, Diddee, Holm, Liu et al. | 2025 | arXiv | https://arxiv.org/abs/2504.05228 | "Be unique" prompts marginal; in-context regeneration roughly reaches human diversity; bigger models less diverse | F |
| 20 | Echoes in AI: lack of plot diversity | Xu, Jojic, Rao, Brockett, Dolan | 2025 | PNAS 122(35) | https://arxiv.org/abs/2501.00273 | Plot elements echo across generations and models; better prompts raise uniqueness | F |
| 21 | Evaluating Creative Short Story Generation in Humans and LLMs | Ismayilzada, Stevenson, van der Plas | 2025 | ICCC 2025 | https://arxiv.org/abs/2411.02316 | LLMs stylistically complex but less novel or surprising; LLM judges overrate LLM stories | F |
| 22 | Catch Me If You Can? Not Yet | Wang, Tripto, Park, Li, Zhou | 2025 | Findings of EMNLP 2025 | https://arxiv.org/abs/2509.14543 | Few-shot personal-style imitation regresses to generic tone; more demonstrations barely help | F |
| 23 | Customizing LLM Generation Style using PEFT (StyleTunedLM) | Liu, Diddee, Ippolito | 2024 | arXiv | https://arxiv.org/abs/2409.04574 | LoRA per author captures style better than prompting or few-shot | F |
| 24 | Do LLMs write like humans? Grammatical and rhetorical styles | Reinhart, Markey, Laudenbach et al. | 2025 | PNAS 122 | https://arxiv.org/abs/2410.16107 | Participial clauses 2–5×, nominalizations ~2×; "tapestry" in 23% of GPT-4o outputs | F |
| 25 | Suppressing Pink Elephants with Direct Principle Feedback | Castricato, Lile, Anand, Schoelkopf, Verma, Biderman | 2024 | arXiv | https://arxiv.org/abs/2402.07896 | Telling a model to avoid X can raise mentions of X (weaker models) | F |
| 26 | Generative AI enhances individual creativity but reduces collective diversity | Doshi, Hauser | 2024 | Science Advances 10, eadn5290 | https://doi.org/10.1126/sciadv.adn5290 | GPT-4 ideas raise novelty (+8.1%) and help weak writers most, but make stories more similar | W |
| 27 | Homogenization Effects of LLMs on Human Creative Ideation | Anderson, Shah, Kreminski | 2024 | C&C 2024 | https://arxiv.org/abs/2402.01536 | ChatGPT users' ideas less distinct across users | A |
| 28 | Does Writing with Language Models Reduce Content Diversity? | Padmakumar, He | 2024 | ICLR 2024 | https://arxiv.org/abs/2309.05196 | InstructGPT co-writing reduces lexical and content diversity across authors | A |
| 29 | We're Different, We're the Same: Creative Homogeneity Across LLMs | Wenger, Kenett | 2025 | arXiv | https://arxiv.org/abs/2501.19361 | LLMs are more similar to each other than humans are to each other | A |
| 30 | AI as Humanity's Salieri (Creativity Index) | Lu, Sclar, Hallinan, Mireshghallah et al. | 2024 | arXiv | https://arxiv.org/abs/2410.04265 | Professional authors 66.2% higher Creativity Index; alignment −30.1% | A |
| 31 | Modifying LLM Post-Training for Diverse Creative Writing | Chung, Padmakumar, Roemmele, Sun, Kreminski | 2025 | arXiv | https://arxiv.org/abs/2503.17126 | Deviation-weighted DPO/ORPO gives human-level diversity with near-GPT-4o quality | A |
| 32 | Base Models Beat Aligned Models at Randomness and Creativity | West, Potts | 2025 | arXiv | https://arxiv.org/abs/2505.00047 | Aligned models favor pleasant over original writing | A |
| 33 | Reading Subtext: LLMs on Short Story Summarization with Writers | Subbiah, Zhang, Chilton, McKeown | 2024 | TACL | https://arxiv.org/abs/2403.01061 | >50% unfaithful summaries; weak on subtext; LLM ratings don't track writers | A |
| 34 | Re3: Recursive Reprompting and Revision | Yang, Tian, Peng, Klein | 2022 | EMNLP 2022 | https://arxiv.org/abs/2210.06774 | Plan + reprompt + rerank + edit: +14% coherence, +20% relevance | A |
| 35 | DOC: Detailed Outline Control | Yang, Klein, Peng, Tian | 2023 | ACL 2023 | https://arxiv.org/abs/2212.10077 | Detailed outlines: +22.5% coherence, +20.7% interestingness over Re3 | A |
| 36 | Co-Writing Screenplays and Theatre Scripts (Dramatron) | Mirowski, Mathewson, Pittman, Evans | 2022 | arXiv | https://arxiv.org/abs/2209.14958 | Hierarchical prompt chaining for scripts; useful to 15 professionals | A |
| 37 | Creative Writing with an AI-Powered Writing Assistant (Wordcraft) | Ippolito, Yuan, Coenen, Burnam | 2022 | arXiv | https://arxiv.org/abs/2211.05030 | Professionals: AI helps brainstorming and details but loses authorial voice | A |
| 38 | Creativity Support in the Age of LLMs (emerging writers) | Chakrabarty, Padmakumar, Brahman, Muresan | 2023 | arXiv | https://arxiv.org/abs/2309.12570 | Writers find LLMs most helpful for drafting and reviewing | A |
| 39 | A Confederacy of Models | Gómez-Rodríguez, Williams | 2023 | Findings of EMNLP 2023 | https://arxiv.org/abs/2310.08433 | Some commercial LLMs match human writers on most dimensions; humans lead in creativity | A |
| 40 | Small Language Models can Outperform Humans in Short Creative Writing | Marco, Rello, Gonzalo | 2025 | COLING 2025 | https://arxiv.org/abs/2409.11547 | GPT-4o coherent with fewer clichés but predictable (3% vs 15% surprising associations) | A |
| 41 | Do Language Models Enjoy Their Own Stories? | Chhun, Suchanek, Clavel | 2024 | TACL | https://arxiv.org/abs/2405.13769 | LLM story evaluation works at system level; explanations unsatisfactory | A |
| 42 | LLM Evaluators Recognize and Favor Their Own Generations | Panickssery, Bowman, Feng | 2024 | arXiv | https://arxiv.org/abs/2404.13076 | Self-preference bias linked to self-recognition | A |
| 43 | Detection and Measurement of Syntactic Templates | Shaib, Elazar, Li, Wallace | 2024 | EMNLP 2024 | https://arxiv.org/abs/2407.00211 | 76% of model templates found in pretraining data (vs 35% human); RLHF doesn't remove them | A |
| 44 | Why Does ChatGPT "Delve" So Much? | Juzek, Ward | 2025 | COLING 2025 | https://arxiv.org/abs/2412.11385 | 21 over-represented focal words; RLHF a plausible cause | A |
| 45 | Idiosyncrasies in Large Language Models | Sun, Yin, Xu, Kolter, Liu | 2025 | ICML 2025 | https://arxiv.org/abs/2502.12150 | 97.1% model attribution; fingerprints survive rewriting and translation | A |
| 46 | Understanding the Effects of RLHF on LLM Generalisation and Diversity | Kirk, Mediratta, Nalmpantis et al. | 2023 | arXiv | https://arxiv.org/abs/2310.06452 | RLHF significantly reduces output diversity vs SFT | A |
| 47 | Creativity Has Left the Chat | Mohammadi | 2024 | arXiv | https://arxiv.org/abs/2406.05587 | Aligned models show lower entropy and "attractor states" | A |
| 48 | When "A Helpful Assistant" Is Not Really Helpful (personas) | Zheng, Pei, Logeswaran, Lee, Jurgens | 2024 | Findings of EMNLP 2024 | https://arxiv.org/abs/2311.10054 | Personas in system prompts don't improve accuracy; effects largely random | A |
| 49 | Can LLMs Truly Understand Prompts? Negated Prompts | Jang, Ye, Seo | 2022 | arXiv | https://arxiv.org/abs/2209.12711 | Inverse scaling on negated prompts (older models) | A |
| 50 | CS4: Story-Writing Constraints | Atmakuru, Nainani, Bheemreddy et al. | 2024 | arXiv | https://arxiv.org/abs/2410.04197 | More constraints block regurgitation but strain coherence | A |
| 51 | Prompting Diverse Ideas | Meincke, Mollick, Terwiesch | 2024 | arXiv | https://arxiv.org/abs/2402.01727 | CoT prompting gives the highest idea diversity, near human groups | A |
| 52 | LLM Discussion: Discussion Framework and Role-Play | Lu, Chen, Pai, Yu, Lee, Sun | 2024 | COLM 2024 | https://arxiv.org/abs/2405.06373 | Role-played multi-agent discussion raises creativity scores | A |
| 53 | Learning to Reason for Long-Form Story Generation | Gurung, Lapata | 2025 | arXiv | https://arxiv.org/abs/2503.22828 | Learned next-chapter planning preferred by humans | A |
| 54 | Creative Preference Optimization (CrPO) | Ismayilzada, Laverghetta, Luchini et al. | 2025 | Findings of EMNLP 2025 | https://arxiv.org/abs/2505.14442 | Multi-dimension creativity preference training beats GPT-4o on novelty | A |
| 55 | Weaver: Foundation Models for Creative Writing | Wang, Chen, Jia et al. | 2024 | arXiv | https://arxiv.org/abs/2401.17268 | Writing-specialized models beat larger generalists (2024) | A |
| 56 | Show, Don't Tell: Implicit Character Portrayal (LIIPA) | Jaipersaud, Zhu, Rudzicz, Creager | 2024 | NeurIPS 2024 Creativity Workshop | https://arxiv.org/abs/2412.04576 | LLMs infer traits shown through actions; usable as a "showing" analyzer | A |
| 57 | How Well Do LLMs Imitate Human Writing Style? | Jemama, Kumar | 2025 | IEEE UEMCON 2025 | https://arxiv.org/abs/2509.24930 | Few-shot 23.5× better than zero-shot; completion prompting best; perplexity gap remains | A |
| 58 | COIG-Writer | Li, Ying, Qu et al. | 2025 | arXiv | https://arxiv.org/abs/2510.14763 | Process supervision helps; no cross-lingual transfer; TTR inversely tracks quality | A |
| 59 | Narrative Flattening | Li, Zhu, Wu, Bao, Evans | 2026 | EMNLP 2026 | https://arxiv.org/abs/2605.27878 | Post-training flattens theme, affect and style; literary fiction most compressed | A |
| 60 | Towards Human-Level Book-Writing Capability | Zierstek, Batelic, Medjad, Schönenberger | 2026 | arXiv (unreviewed) | https://arxiv.org/abs/2605.17064 | Assistant tuning avoids deception and ambiguity; prompt-to-book fine-tuning claimed to beat GPT-5.5/Opus 4.8 | A |
| 61 | Beyond the Hivemind: Meta-Persona Anchoring | Fu, Conde, Arriaga, Martínez, Reviriego, Coronado-Blázquez | 2026 | arXiv (unreviewed) | https://arxiv.org/abs/2608.02618 | Persona self-selection + extreme-temperature sieve: similarity 0.85 → 0.65 | A |
| 62 | Improving Diversity in LLM Short Story Generation (DivLM) | Solati Dehkordi, Lampos | 2026 | arXiv | https://arxiv.org/abs/2610.06729 | RL composite reward: +9% diversity across genre, tone, style, entities | A |
| 63 | Avoidance Decoding for Diverse Multi-Branch Story Generation | Park, Yang, Jung | 2025 | arXiv | https://arxiv.org/abs/2509.02170 | Similarity-penalty decoding: up to 2.6× diversity, −30% repetition | A |
| 64 | POLARIS: Guiding Small Models to Write Long Stories | Rajendhran, Russell, Iyyer, Wieting | 2026 | EMNLP 2026 | https://arxiv.org/abs/2606.04095 | Rubric-judge GRPO + human reference: 9B ≈ 27B in human evaluation | A |
| 65 | SAGE: Hierarchical LLM-Based Literary Evaluation | Wang, Zhou | 2026 | arXiv | https://arxiv.org/abs/2605.07102 | LLM stories trail most on cultural critique and philosophical depth | A |
| 66 | Scaling Creative Writing Beyond Story-Centric Data | Chang, Kim, Yeen, Kim, Lee, Lee | 2026 | CIKM 2026 | https://arxiv.org/abs/2608.13947 | Genre-attribute-guided data beats story-only fine-tuning data | A |
| 67 | EQ-Bench Creative Writing v3 (about + leaderboard) | Paech (EQ-Bench) | 2023–2026 | Benchmark site | https://eqbench.com/about.html | Claude-judged rubric + pairwise Elo; penalizes clichés, showy vocabulary, metaphor overload; slop and repetition metrics | W |
| 68 | Prompting best practices (Claude) | Anthropic | 2026 | Vendor docs | https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-4-best-practices | Say what to do, not what not to do; give reasons; use varied examples; models converge on generic choices | W |
