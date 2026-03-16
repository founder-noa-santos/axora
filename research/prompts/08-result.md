AXORA’s evaluation should combine strong single‑agent benchmarks (for raw coding/reasoning ability) with multi‑agent–specific, production‑grade metrics (task success, collaboration quality, cost, and drift), plus a proper A/B and monitoring setup.

***

## Section 1: Benchmark Landscape

### Single‑agent code benchmarks

**HumanEval**

- 164 hand‑written Python function problems; each has a natural‑language prompt, a reference solution, and hidden unit tests evaluating functional correctness. [github](https://github.com/openai/human-eval)
- Measured via **pass@k**: probability that at least one of k generated samples passes all tests; in practice pass@1 and pass@10 are standard. [statsig](https://www.statsig.com/perspectives/humaneval-code-benchmarks)
- SOTA: frontier models like GPT‑4o, Claude 3.5, Qwen2.5‑Coder achieve ~90–97% pass@1 on the original HumanEval; HumanEval Pro variants are ~10–25 points harder and expose gaps (e.g., o1‑mini 96–97% on HumanEval vs ~76% on HumanEval Pro). [emergentmind](https://www.emergentmind.com/topics/humaneval-coding-benchmark)
- Limitations: small size (164 items), single‑function Python only, heavy contamination risk, and easy to overfit / “game” by prompt‑tuning without real robustness. [statsig](https://www.statsig.com/perspectives/humaneval-code-benchmarks)

**MBPP (Mostly Basic Python Problems)**

- 974 crowd‑sourced Python tasks designed to be solvable by entry‑level programmers, each with a description, reference solution, and a few visible tests; evaluates broader basic coding skills than HumanEval. [llm-stats](https://llm-stats.com/benchmarks/mbpp-++-base-version)
- Uses the same execution‑based pass@k metrics as HumanEval, but with a more diverse and larger problem set. [runloop](https://runloop.ai/blog/mbpp-the-benchmark-that-democratized-code-generation)
- Modern models exceed ~95% accuracy on MBPP, though many still perform *slightly worse* on MBPP than HumanEval despite MBPP’s “simpler” problems—suggesting MBPP provides a more challenging and diverse coverage of fundamentals. [runloop](https://runloop.ai/blog/mbpp-the-benchmark-that-democratized-code-generation)
- Limitations: Python‑only, visible tests (encourages test‑pattern exploitation), contamination, and still “toy” relative to real software engineering. [runloop](https://runloop.ai/blog/mbpp-the-benchmark-that-democratized-code-generation)

**LiveCodeBench**

- Real‑world competitive programming problems from recent online contests (2023–2024+), with >500 problems spanning difficulties; focuses on algorithmic problem solving and code generation under more realistic constraints. [artificialanalysis](https://artificialanalysis.ai/evaluations/livecodebench)
- Scored by functional correctness (pass rate on hidden tests), often reported as a single percentage; leaderboards show top models above 90% overall, with much lower success on medium/hard sub‑splits (e.g., ~53% pass@1 on medium and 0% on hard for best 2025 models). [pricepertoken](https://pricepertoken.com/leaderboards/benchmark/livecodebench)
- SOTA: Gemini 3 Pro Preview and similar frontier models reach ~91–92% overall score. [pricepertoken](https://pricepertoken.com/leaderboards/benchmark/livecodebench)
- Limitations: still single‑file competitive coding, not full‑project work; contaminated by public contest archives over time; not Python‑only but still limited in language and domain scope.

**SWE‑bench, SWE‑bench Verified, SWE‑bench‑Live**

- SWE‑bench: 2,294 software‑engineering tasks mined from issues and pull‑requests across 12 popular Python repos (e.g., Django, Matplotlib). [swebench](https://www.swebench.com/original.html)
  - Given the repo and an issue description, the model/agent must produce a patch that makes failing tests pass; evaluation is purely via repo‑level tests before vs after patch. [arxiv](https://arxiv.org/abs/2310.06770)
  - Original paper showed very low resolution rates (best model ~2% resolution), highlighting the gap between single‑function benchmarks and real SE tasks. [pli.princeton](https://pli.princeton.edu/blog/2023/swe-bench-can-language-models-resolve-real-world-github-issues)
- SWE‑bench Verified: a human‑validated subset (~500 instances; ~480 used in some evaluations) that filters out underspecified issues and unfair tests, making scores more meaningful; still uses test‑based resolution. [openai](https://openai.com/index/introducing-swe-bench-verified/)
  - Filtered out ~68% of original samples due to underspecification or problematic tests, then re‑evaluated; GPT‑4o on a strong scaffold reaches ~33% on Verified vs ~16% on original SWE‑bench. [openai](https://openai.com/index/introducing-swe-bench-verified/)
  - External leaderboards report agent‑based systems now exceeding 60% resolution on Verified for some frontier models, indicating rapid progress and strong agent architectures. [arxiv](https://arxiv.org/html/2505.23419v2)
- SWE‑bench‑Live: a continuously updated, contamination‑resistant benchmark built from *live* GitHub issues in the same repos, with identical evaluation setup. [arxiv](https://arxiv.org/html/2505.23419v2)
  - Methods that reach >60% on SWE‑bench Verified drop to <50% on SWE‑bench‑Live, demonstrating strong overfitting to static benchmarks and the need for evolving testbeds. [arxiv](https://arxiv.org/html/2505.23419v2)

These SWE‑bench variants are the closest thing to an end‑to‑end autonomous coding/agent benchmark we have today.

**2025–2026 code benchmarks & variants**

Relevant newer additions and variants:

- **HumanEval Pro / HumanEval‑T**: LLM‑generated and human‑verified transformations of HumanEval that introduce more complex behaviors, self‑invocation, and different test structures; models lose 10–25 points in pass@1 compared with original HumanEval, exposing brittleness. [arxiv](https://arxiv.org/html/2412.21199v2)
- **MBPP Pro** (in self‑invoking benchmarks): similar idea—“Pro” variants with multi‑step or self‑calling tasks that significantly drop pass@1. [arxiv](https://arxiv.org/html/2412.21199v2)
- **CodeElo / LiveCodeBench Pro**: combine contest problems with Elo‑like rating systems to place model performance in a human‑competitive‑programming context, showing top models roughly at “candidate master” level and still far from human grandmasters on medium/hard tasks. [emergentmind](https://www.emergentmind.com/topics/livecodebench-and-codeelo-benchmarks)
- **SWE‑bench‑Lite**: an easier curated subset; mostly useful as a stepping stone but less relevant once frontier models solve it. [openai](https://openai.com/index/introducing-swe-bench-verified/)

These show a trend toward **harder, contamination‑aware, evolving** benchmarks (SWE‑bench‑Live, GSM8K‑Platinum, MMLU‑Pro, HumanEval Pro).

### General reasoning benchmarks

**MMLU & MMLU‑Pro**

- MMLU: 15,908 multiple‑choice questions across 57 subjects (STEM, humanities, social sciences, professional exams); evaluates general knowledge and multi‑task reasoning. [en.wikipedia](https://en.wikipedia.org/wiki/MMLU)
- MMLU‑Pro: a harder follow‑up with more reasoning‑heavy questions and slightly different consolidation of tasks; top models still cluster around ~90% (Gemini 3 Pro ~90%, Claude Opus ~90%). [intuitionlabs](https://intuitionlabs.ai/articles/mmlu-pro-ai-benchmark-explained)
- 2026 MMLU leaderboard: best models (e.g., GLM‑5, Kimi K2.5) score ~90–92%. [pricepertoken](https://pricepertoken.com/leaderboards/benchmark/mmlu)

**GSM8K & GSM8K‑Platinum**

- GSM8K: 8,500 grade‑school math word problems (7,500 train, 1,000 test), each requiring 2–8 reasoning steps; measures chain‑of‑thought style arithmetic reasoning. [klu](https://klu.ai/glossary/GSM8K-eval)
- Frontier models (GPT‑4 family, Claude 3, Gemini Ultra) reach ~90–95% accuracy using chain‑of‑thought and verifier‑style selection. [klu](https://klu.ai/glossary/GSM8K-eval)
- GSM8K‑Platinum: a 2025 re‑annotation of the full GSM8K test set to remove label noise and ambiguous problems; reveals larger gaps between frontier models and a more accurate picture of their math robustness. [gradientscience](https://gradientscience.org/gsm8k-platinum/)

**BIG‑Bench Hard (BBH)**

- Subset of 23 especially hard BIG‑Bench tasks (6,511 examples) chosen because prior LLMs could not beat average humans; covers arithmetic, logic, temporal reasoning, causal judgment, and complex language tasks. [llm-stats](https://llm-stats.com/benchmarks/big-bench-hard)
- 2026 BBH leaderboard: best models (e.g., Claude 4.5, GLM‑5) reach ~93–94% accuracy. [pricepertoken](https://pricepertoken.com/leaderboards/benchmark/bbh)

Other advanced reasoning sets (not exhaustive): AIME 2025, GPQA Diamond, MATH‑500, and domain‑specific law/medical exams; these are useful for stress‑testing very hard reasoning, but for AXORA the above three are sufficient anchors. [vertu](https://vertu.com/lifestyle/open-source-llm-leaderboard-2026-rankings-benchmarks-the-best-models-right-now/)

### Benchmark limitations & Goodhart’s law

- **Narrow coverage**: Most coding benchmarks either test stand‑alone functions (HumanEval, MBPP) or single‑file competitive problems (LiveCodeBench), far from the multi‑file, multi‑service, multi‑stakeholder environment of real software engineering. [pli.princeton](https://pli.princeton.edu/blog/2023/swe-bench-can-language-models-resolve-real-world-github-issues)
- **Contamination & overfitting**: HumanEval/MBPP are widely available and heavily used; many models were trained on them, and fine‑tuning/prompting specifically to improve benchmark scores is common. [emergentmind](https://www.emergentmind.com/topics/humaneval-coding-benchmark)
- **Static datasets**: SWE‑bench and GSM8K showed apparent “plateaus” (models near ceiling), but re‑annotation (Verified, GSM8K‑Platinum) and live variants (SWE‑bench‑Live) revealed that static benchmarks underestimated model deficiencies and encouraged over‑specialization. [gradientscience](https://gradientscience.org/gsm8k-platinum/)
- **Goodhart’s law**: When benchmarks become the target (e.g., marketing around a single score like HumanEval or MMLU), systems are optimized for that metric at the expense of generalization; explicit discussions of this risk now appear in both LLM evaluation blogs and practitioner write‑ups. [towardsdatascience](https://towardsdatascience.com/how-metrics-and-llms-can-trick-you-a-field-guide-to-paradoxes/)

For AXORA, these benchmarks are **necessary but not sufficient**: treat them as regression gates and sanity checks, not as proxies for real‑world coding or multi‑agent performance.

### Which benchmarks to adopt vs deprioritize

For AXORA (multi‑agent coding system) I’d recommend:

- **Adopt as hard gates / regressions**
  - **HumanEval + HumanEval Pro**: quick sanity check on Python function synthesis; use primarily as a **regression suite**, not as a KPI; track pass@1 as main metric. [deepeval](https://deepeval.com/docs/benchmarks-human-eval)
  - **MBPP**: broader coverage of basic Python skills; again, use for regression and early‑stage model selection. [llm-stats](https://llm-stats.com/benchmarks/mbpp)
  - **LiveCodeBench**: gate raw algorithmic coding ability and competitive‑programming style reasoning. [artificialanalysis](https://artificialanalysis.ai/evaluations/livecodebench)
  - **SWE‑bench Verified**: primary *single‑agent* proxy for real‑world bugfixing; track resolution rate; this is the closest offline proxy to AXORA’s target work. [epoch](https://epoch.ai/benchmarks/swe-bench-verified)
  - **SWE‑bench‑Live**: use periodically (e.g., weekly or per major release) to detect overfitting to static SWE‑bench; expensive but critical for frontier systems. [arxiv](https://arxiv.org/html/2505.23419v2)
  - **MMLU / MMLU‑Pro, GSM8K / GSM8K‑Platinum, BBH**: ensure general reasoning ability—especially important if AXORA agents must plan and reason beyond code (requirements analysis, refactoring strategies, design discussions). [pricepertoken](https://pricepertoken.com/leaderboards/benchmark/mmlu-pro)

- **Use selectively**
  - Extreme reasoning sets (AIME 2025, GPQA Diamond, MATH‑500): useful when deciding between closely matched top‑tier models, but not central to day‑to‑day AXORA evaluation. [vertu](https://vertu.com/lifestyle/open-source-llm-leaderboard-2026-rankings-benchmarks-the-best-models-right-now/)

- **Deprioritize / ignore for AXORA**
  - Benchmarks focused purely on chat quality, summarization, or multimodal perception (unless you later extend AXORA beyond coding).  
  - Easy subsets (e.g., SWE‑bench‑Lite) once your system is clearly above them. [openai](https://openai.com/index/introducing-swe-bench-verified/)

***

## Section 2: Multi‑Agent Metrics

You’ll need **system‑level metrics** that capture how well AXORA’s agents collaborate, not just whether one model can write code.

### Task completion & efficiency

Recommended metrics:

- **Task Success Rate (TSR)**  
  - Definition: fraction of tasks where the final outcome meets acceptance criteria (e.g., all tests green, user marks “satisfied”).  
  - Formula: \(\text{TSR} = \frac{\text{tasks completed successfully}}{\text{total tasks attempted}}\).  
  - Targets:  
    - Internal benchmarks: >85–90% on curated tasks.  
    - Production “organic” tasks: start with >60–70%, push toward >80% as product matures.
- **Time to Completion (TTC)**  
  - Time between task creation and resolution (wall‑clock), plus a decomposition into **agent time** (LLM compute + tools) and **human time** (waiting for feedback, human edits).  
  - Targets: at least 2–5× faster than human‑only baseline for common workflows; track distribution (p50, p90).
- **Iteration Count**  
  - Number of full agent “rounds” or tool‑invocation cycles before success or abandonment.  
  - Useful as a proxy for complexity and for identifying workflows where agents thrash.
- **Human Intervention Rate**  
  - Fraction of tasks where a human had to step in (e.g., manual patching, override, or explicit “fix this agent output”).  
  - Targets:  
    - On curated SWE‑style tasks: aim for <20% after a few iterations.  
    - In production: treat as a leading indicator of where to invest in workflow/design.

These align with metrics used in multi‑agent benchmarks like MultiAgentBench/MARBLE (task completion and milestone achievement) and recent evaluation surveys. [arxiv](https://arxiv.org/abs/2503.01935)

### Communication metrics

Drawing from communication‑centric surveys of LLM multi‑agent systems and benchmarks like MultiAgentBench: [themoonlight](https://www.themoonlight.io/en/review/multiagentbench-evaluating-the-collaboration-and-competition-of-llm-agents)

- **Messages per Task**  
  - Total number of agent‑to‑agent and agent‑to‑orchestrator messages per completed task.  
  - Signals coordination complexity and potential overhead.
- **Coordination Token Overhead**  
  - Definition: fraction of tokens spent on intra‑agent communication vs total tokens (communication + “work” like code, tool calls).  
  - Formula: \(\text{CommOverhead} = \frac{\text{tokens in system + agent messages}}{\text{total tokens in session}}\).  
  - Target: <15–25% for mature workflows; higher overhead is acceptable during exploration or on very complex tasks.
- **Communication Efficiency Ratio**  
  - Task‑completion quality (e.g., TSR or code quality score) divided by coordination tokens; essentially “quality per coordination token”.  
  - Use for comparing architectures (e.g., star vs graph vs chain coordination) as in MARBLE. [ui.adsabs.harvard](https://ui.adsabs.harvard.edu/abs/2025arXiv250301935Z/abstract)
- **Information Density**  
  - Average information per message, approximated via:  
    - compression ratio (tokens vs semantic embeddings), or  
    - rubric‑based/LLM‑as‑judge scores for “relevance/clarity” of messages (e.g., 1–5 Likert scale).  
  - Inspired by metrics like Communication Score (Cscore) in MultiAgentBench. [samiranama](https://samiranama.com/posts/Evaluating-LLM-based-Agents-Metrics,-Benchmarks,-and-Best-Practices/)

### Collaboration quality

Based on MultiAgentBench KPIs and broader LLM‑MAS surveys: [arxiv](https://arxiv.org/html/2502.14321v2)

- **Conflict Frequency**  
  - Count how often agents propose mutually incompatible plans or patches (e.g., conflicting edits to same file or divergent problem diagnoses) per task.  
  - Compute via structural diffing of proposals or via LLM‑based conflict detection on messages.
- **Conflict Resolution Success Rate**  
  - Fraction of conflicts that are resolved without human intervention and still yield a successful task outcome.  
  - High conflict plus high resolution may be acceptable (brainstorm then converge); high conflict with low resolution indicates coordination problems.
- **Redundancy Ratio**  
  - Portion of agent efforts that duplicate others’ work (e.g., two agents independently implement similar patches).  
  - Measured via code similarity, overlapping tool calls, or semantic clustering of messages.  
  - Lower is generally better, but **some** redundancy can improve robustness.
- **Coverage / Missed Requirements Rate**  
  - Whether the final output addresses *all* stated requirements and test cases; MultiAgentBench uses milestone‑based KPIs to capture partial progress; you can mimic this with checklists or LLM‑judged rubrics. [arxiv](https://arxiv.org/abs/2503.01935)
  - For AXORA, you can define per‑task checklists (e.g., “bug fixed”, “tests added/updated”, “docs updated”) and track coverage.

### Cost metrics

Following LLMOps monitoring guides that emphasize token and cost awareness: [nexos](https://nexos.ai/blog/llm-monitoring/)

- **Token Cost per Task**  
  - Total prompt + completion tokens × model price; track per workflow, per user, and per model.  
- **Compute Cost per Task**  
  - If self‑hosting, translate GPU/CPU time to cost; otherwise use API cost only.
- **Human Time Saved**  
  - Baseline human‑only time vs human+AXORA time; measure via user studies and instrumentation.  
- **ROI per Task / per User**  
  - Approximate as: \(\text{ROI} \approx \frac{\text{human time saved} \times \text{blended hourly rate}}{\text{LLM + infra cost}}\).  
  - Use as a management‑level metric to justify model upgrades or architecture changes.

***

## Section 3: Evaluation Framework

### Automated evaluation

1. **Unit / integration tests**

   - For coding tasks with clear ground truth (e.g., HumanEval, MBPP, SWE‑style tasks), treat **test pass rate** as the primary automated metric. [github](https://github.com/openai/human-eval)
   - For AXORA:
     - Require every benchmark task to come with or derive a test harness.  
     - For real‑world tickets, encourage users to supply failing tests or at least reproduction scripts; if not, use LLM‑as‑judge or static analysis as a fallback.

2. **Static analysis & linters**

   - Run tools like `ruff`, `flake8`, `mypy`, security scanners, etc.  
   - Track:
     - syntax errors per LOC,
     - linter violation density,
     - type‑checking success rate,
     - security findings per LOC.  
   - Use as guardrail metrics to catch regressions in style/safety even if tests pass.

3. **LLM‑as‑judge**

   - CodeJudgeBench and “From Code to Courtroom” review show LLM‑as‑judge can reliably rank or score code for correctness, readability, and usefulness across code generation, repair, and test‑generation tasks, often correlating well with human judgments when designed carefully. [arxiv](https://arxiv.org/abs/2507.10535)
   - Key findings:
     - “Thinking” models (chain‑of‑thought or reasoning‑optimized) are significantly better judges than vanilla instruction models, sometimes outperforming larger specialized judge models. [arxiv](https://arxiv.org/abs/2507.10535)
     - Pairwise comparison (A vs B) is more reliable than scalar scoring; prompts that retain the full context and comments lead to better judge performance. [arxiv](https://arxiv.org/html/2503.02246v1)
   - For AXORA:
     - Use a *different* model (or different version) as judge than the one generating code.  
     - Use pairwise comparisons (baseline vs candidate) where possible.  
     - Have the judge output a structured rubric (e.g., 1–5 for correctness, 1–5 for readability, 1–5 for robustness) plus a textual justification.

4. **Pros / cons of automated methods**

   - Pros: scalable, cheap per sample, consistent, and can be integrated into CI/CD and regression suites.  
   - Cons: blind to user experience nuances, can be gamed (Goodhart), and LLM‑as‑judge inherits biases and may mis‑score edge cases. [stackoverflow](https://stackoverflow.blog/2025/10/09/who-watches-the-watchers-llm-on-llm-evaluations/)

### Human evaluation

When and how to incorporate humans:

- **When necessary**
  - Novel workflows without ground truth tests.  
  - UX‑heavy tasks (diagnosis explanations, refactoring plans, code review comments).  
  - Safety‑critical or high‑risk changes.  
- **Methods**
  - **Code review scores**: human reviewers rate AXORA outputs on clarity, correctness, maintainability, and adherence to style (e.g., 1–5 each).  
  - **Preference studies / A/B**: show engineers diffs or side‑by‑side solutions (AXORA vs Copilot vs Cursor vs human baseline) and record preferences.  
  - **User satisfaction instruments**: SUS (System Usability Scale), NPS, and task‑specific satisfaction ratings for UX. [langchain](https://www.langchain.com/conceptual-guides/production-monitoring)

Human evaluation is expensive; use it **sparingly but regularly** to calibrate automated metrics (especially LLM‑as‑judge).

### Hybrid evaluation

- **Automated first pass + human sampling**
  - Run tests, static analysis, and LLM‑as‑judge on **all** benchmark tasks.  
  - Apply **stratified sampling** for human review:
    - samples where automated metrics disagree (tests pass but judge score low, or vice versa),  
    - random slice of high‑ and low‑scoring items per release,  
    - critical product workflows.  
- **Calibration**
  - Periodically compute correlation between human scores and LLM‑as‑judge/static metrics; adjust judge prompts and rubrics when correlation drifts.  
  - Use human labels to retrain or select better judges, as recommended in recent LLM‑as‑judge research. [arxiv](https://arxiv.org/html/2503.02246v1)

### Continuous evaluation & drift detection

LLMOps and agent‑observability guides emphasize continuous, online evaluation using production traces, not just offline benchmarks. [mlflow](https://mlflow.org/llmops)

- **In‑production monitoring**
  - Log complete traces: prompts, intermediate reasoning, tool calls, code diffs, test results, and user feedback. [mlflow](https://mlflow.org/llmops)
  - Run **online evaluations** on a sampled subset of traces:
    - LLM‑as‑judge scoring of helpfulness and correctness,  
    - safety and hallucination detectors,  
    - task‑completion detection for workflows where tests are available. [oneuptime](https://oneuptime.com/blog/post/2026-01-30-llmops-monitoring/view)
- **Drift detection**
  - Track:
    - success rate by model/version,  
    - distribution shifts in task types,  
    - tool‑usage patterns (e.g., which AXORA tools are used/overused),  
    - coordination metrics (messages per task, role adherence) to detect “agent drift”. [co-r-e](https://co-r-e.com/method/agent-drift-multi-agent)
  - Use statistical tests (e.g., KL divergence on tool‑usage distributions, chi‑squared tests on success rates) to flag deviations, as suggested in drift‑analysis write‑ups. [co-r-e](https://co-r-e.com/method/agent-drift-multi-agent)

***

## Section 4: A/B Testing

### What to test

AXORA is a perfect candidate for systematic experiments:

- **Agent architectures**
  - Single‑agent vs multi‑agent, chain vs star vs graph orchestration, presence/absence of specialist roles.  
- **Models & tool choices**
  - Different base models, mixture‑of‑experts vs single model, specialized code models vs generalist LLMs.  
- **Prompts & coordination protocols**
  - System prompts for each role, explicit planning steps, rules for when agents can talk to each other, and policies for tool usage.  
- **Safety / guardrail variants**
  - Stricter vs looser safety filters; their effect on productivity.

### Experimental design

Borrowing from standard online experimentation and modern LLM evaluation practice: [nexos](https://nexos.ai/blog/llm-monitoring/)

- **Control variables**
  - Keep user population, task routing, test harnesses, and environment constant between variants.  
  - Avoid overlapping experiments on the same metrics without factorial design.
- **Randomization**
  - Randomly assign tasks or users to variants (A vs B) at allocation time; stratify by task type or difficulty to avoid imbalance.
- **Sample size & power**
  - Choose primary metric (e.g., Task Success Rate).  
  - Use standard power analysis (approximate or via tools like Statsig, GrowthBook) to determine how many tasks/users you need to detect, say, a 3–5 percentage‑point change with sufficient power. [statsig](https://www.statsig.com/perspectives/humaneval-code-benchmarks)
  - For rare events (e.g., high‑severity failures), aggregate over longer periods or use sequential testing.

### Metrics to track

- **Primary**: Task Success Rate, optionally per task category (bugfix, refactor, new feature, test‑writing).  
- **Secondary**:
  - Time to completion (p50/p90),  
  - token and cost per task,  
  - communication overhead,  
  - static‑analysis and judge scores.  
- **Guardrail metrics**:
  - bug introduction / regression rate (new failing tests),  
  - security findings,  
  - user satisfaction (thumbs up/down, SUS/NPS).  

Guardrail metrics ensure that success‑rate improvements are not purchased by shortcuts or unsafe behavior—directly addressing Goodhart’s law concerns. [towardsdatascience](https://towardsdatascience.com/how-metrics-and-llms-can-trick-you-a-field-guide-to-paradoxes/)

### Implementation in AXORA

- **Assignment layer**: in your orchestration service, implement experiment flags (e.g., via a simple experiment service or an external A/B platform) that select:  
  - agent graph configuration,  
  - model choice,  
  - prompts.  
- **Logging schema**: log `experiment_id`, `variant`, `user_id` (or anonymized ID), `task_id`, and all metrics to a central warehouse or time‑series DB.  
- **Rollback procedures**:
  - Always maintain a “safe” baseline variant.  
  - Allow production rollback by flag/config without redeploy.  
  - Add automatic rollback when primary metrics degrade beyond thresholds over a given window.

***

## Section 5: Implementation Plan

### Tools & infrastructure

You can implement this evaluation/monitoring stack with standard components plus an agent‑observability layer: [langchain](https://www.langchain.com/conceptual-guides/production-monitoring)

- **Evaluation harness**
  - Use or adapt existing open‑source harnesses for HumanEval/MBPP/LiveCodeBench/SWE‑bench to run offline evals in CI. [swebench](https://www.swebench.com/original.html)
  - Add your own benchmark harness for AXORA tasks with:
    - test runners (pytest for Python, etc.),  
    - static analysis pipeline,  
    - LLM‑as‑judge evaluation.
- **LLMOps / observability**
  - Tracing & eval: LangSmith, MLflow’s LLMOps stack, or equivalent; track prompts, tool calls, and metrics per run. [mlflow](https://mlflow.org/llmops)
  - Monitoring: Prometheus + Grafana, OpenTelemetry, or commercial alternatives; integrate token usage, latency, error rates, and quality scores. [oneuptime](https://oneuptime.com/blog/post/2026-01-30-llmops-monitoring/view)
- **Experimentation**
  - Statsig, GrowthBook, or an internal A/B testing service to handle randomization, exposure control, and statistical analysis. [nexos](https://nexos.ai/blog/llm-monitoring/)
- **Storage**
  - Time‑series DB for metrics (Prometheus, VictoriaMetrics).  
  - Warehouse (Postgres/ClickHouse/BigQuery/Snowflake) for deeper analysis and offline experiments.  

All of these are language‑agnostic and can easily support a Rust‑based backend or orchestrator if you structure logging and metrics via standard protocols (e.g., OTEL).

### Phased rollout

**Phase 1 – Offline baseline (weeks 1–2)**

- Implement harnesses for: HumanEval, MBPP, LiveCodeBench, SWE‑bench Verified. [llm-stats](https://llm-stats.com/benchmarks/mbpp)
- Decide and document **core metrics & targets**:
  - e.g., HumanEval pass@1 ≥ 90%, MBPP ≥ 95%, SWE‑bench Verified ≥ X% for chosen model.  
- Build a small internal AXORA benchmark suite (e.g., 50–100 real coding tasks from your own projects) with:
  - tests where possible,  
  - human‑annotated outcomes,  
  - initial LLM‑as‑judge rubrics.

**Phase 2 – Multi‑agent evaluation (weeks 3–5)**

- Instrument your orchestrator to log multi‑agent traces and compute:
  - TSR, TTC, messages per task, CommOverhead, conflict frequency, redundancy, coverage.  
- Run controlled experiments comparing:
  - single‑ vs multi‑agent,  
  - different coordination topologies (star vs chain vs graph), inspired by MARBLE/MultiAgentBench. [themoonlight](https://www.themoonlight.io/en/review/multiagentbench-evaluating-the-collaboration-and-competition-of-llm-agents)
- Use offline evaluation with synthetic and real tasks to converge on a baseline architecture that meets your targets.

**Phase 3 – Production monitoring & A/B (weeks 6–8)**

- Deploy AXORA to a limited beta; enable:
  - trace logging,  
  - online LLM‑as‑judge sampling,  
  - token and cost tracking. [langchain](https://www.langchain.com/conceptual-guides/production-monitoring)
- Build dashboards showing:
  - task success rate over time,  
  - latency (p50, p95, p99) per workflow,  
  - human intervention rate,  
  - communication metrics,  
  - cost per task/user.  
- Run your first A/B tests on safe variants (e.g., prompt tweaks, minor coordination changes) with clearly defined rollback rules.

**Phase 4 – User studies & competitive benchmarking (ongoing)**

- User studies:
  - Recruit internal and external developers; design tasks that mirror real tickets (bugfix, refactor, small feature).  
  - Have them work under three conditions: baseline (no AI), AXORA, and competitor (e.g., Copilot or Cursor).  
  - Measure task completion time, error rate (bugs introduced, tests failing), satisfaction (SUS, NPS), and qualitative feedback. [nexos](https://nexos.ai/blog/llm-monitoring/)
- Competitive benchmarking:
  - Re‑implement a subset of your internal benchmark tasks as scripts for Copilot/Cursor or manual workflows.  
  - For head‑to‑head comparisons, standardize:
    - task prompts,  
    - environment setup,  
    - test harness.  
  - Treat these as *internal R&D metrics*; don’t overfit to them the way vendors sometimes overfit to HumanEval/SWE‑bench.

### Success criteria for AXORA

At a minimum, to say “AXORA works” in a production‑grade sense, you’d want:

- On curated benchmarks:
  - Meets or exceeds SOTA models on HumanEval/MBPP/LiveCodeBench for your chosen model tier.  
  - Achieves competitive SWE‑bench Verified resolution (within, say, 10–15 points of leading open‑source or proprietary agents). [vertu](https://vertu.com/lifestyle/open-source-llm-leaderboard-2026-rankings-benchmarks-the-best-models-right-now/)
- On internal + real tasks:
  - Task success rate >80% on well‑specified internal tasks and trending upward on production tasks.  
  - ≥2× speed‑up vs human‑only baselines on typical bugfix or refactoring tasks.  
  - Human intervention rate decreasing over time as workflows stabilize.
- On multi‑agent metrics:
  - Communication overhead stabilized within an acceptable band (e.g., <20% tokens) while maintaining or improving success. [samiranama](https://samiranama.com/posts/Evaluating-LLM-based-Agents-Metrics,-Benchmarks,-and-Best-Practices/)
  - Conflict frequency and redundancy well‑understood and controlled (not exploding as you add more agents).  
- On business metrics:
  - Positive ROI per user or per seat: human time saved clearly exceeds LLM + infrastructure costs.  
  - No sustained regressions in guardrail metrics (safety, regression rate, user satisfaction) across releases, reflecting Goodhart‑aware evaluation rather than score chasing. [stackoverflow](https://stackoverflow.blog/2025/10/09/who-watches-the-watchers-llm-on-llm-evaluations/)

This gives AXORA a **scientifically grounded, production‑oriented** evaluation framework that integrates classical benchmarks, multi‑agent‑specific metrics, and real‑world monitoring.