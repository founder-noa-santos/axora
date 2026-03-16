

# R-05 Findings: Local Model Optimization for AI Coding Systems

**Research Date:** 2026-03-16  
**Researcher:** Claude with Web Search  
**Sources:** SitePoint Local LLM Benchmarks 2026 , Local AI Master Model Comparison , Qwen 2.5 Coder Technical Report , Deepgram Local Coding LLM Analysis , Ollama Performance Analysis , ICPC 2026 Quantization Study , Apple Silicon Token Throughput Matrix , NVIDIA GPU Benchmark Suite 

---

## 1. Executive Summary & Primary Recommendation

The landscape for locally-deployable large language models has matured dramatically between 2024 and 2026, with code-specialized models now achieving performance levels that approach or exceed earlier cloud-based alternatives for specific tasks. For AXORA's multi-agent AI coding system, the research establishes that **Tier 2 quality is achievable on consumer hardware**, enabling production-grade local inference with appropriate model selection and optimization.

### 1.1 Top-Tier Local Code Models (2025-2026)

Three models emerge as clear leaders for AXORA's deployment, each occupying a distinct position in the quality-speed-hardware trade-off space.

#### 1.1.1 Qwen 2.5 Coder 32B — Flagship Choice for Quality-Critical Tasks

The **Qwen 2.5 Coder 32B** represents the current state-of-the-art for open-source code generation, achieving **88.4% on HumanEval in full precision**—surpassing GPT-4's 87.1% and approaching GPT-4o's 90.2% . This model delivers near-cloud quality for complex software engineering tasks including multi-file refactoring, architectural design, and subtle bug diagnosis.

| Attribute | Specification |
|-----------|-------------|
| **HumanEval (FP16)** | 88.4%  |
| **HumanEval (Q4_K_M estimated)** | ~49-55%  |
| **HumanEval (Q5_K_M estimated)** | ~60-65% |
| **Quantized size (Q4_K_M)** | ~20GB |
| **Quantized size (Q5_K_M)** | ~22GB |
| **Tokens/sec (M2/M3 Max)** | 6-10  |
| **Tokens/sec (RTX 4090)** | 30-40  |
| **Context window** | 128K tokens  |
| **License** | Apache 2.0  |

The model's **128K token context window** enables repository-level understanding, critical for AXORA's multi-file operations. However, the **20GB+ memory requirement** restricts deployment to high-end workstations: M3 Max with 36-48GB unified memory, or RTX 4090 with 24GB VRAM and system RAM fallback. The **6-10 tok/s generation speed on Apple Silicon** is acceptable for non-interactive quality-critical tasks but marginal for real-time workflows.

The **Apache 2.0 license** permits unrestricted commercial use, eliminating legal barriers for AXORA's distribution. This model serves as AXORA's **quality tier** for complex architectural tasks, debugging, and multi-file refactoring where reasoning depth justifies latency.

#### 1.1.2 Qwen 2.5 Coder 7B — Balanced Performance Default

The **Qwen 2.5 Coder 7B** achieves the remarkable feat of **matching its 32B sibling's 88.4% HumanEval score in full precision**, demonstrating exceptional architecture efficiency . When quantized to **Q4_K_M**, it delivers approximately **76% HumanEval-equivalent performance** with dramatically reduced resource requirements.

| Attribute | Specification |
|-----------|-------------|
| **HumanEval (FP16)** | 88.4%  |
| **HumanEval (Q4_K_M estimated)** | ~76%  |
| **Quantized size (Q4_K_M)** | ~4.7GB  |
| **Quantized size (Q6_K)** | ~6.3GB |
| **VRAM requirement** | 5-8GB |
| **Tokens/sec (M2 Mac)** | 10-15  |
| **Tokens/sec (desktop GPU)** | ~38  |
| **Minimum RAM** | 8GB |
| **Comfortable RAM** | 16GB |

Real-world validation by Deepgram demonstrated this model's practical reliability: it successfully implemented functional versions of classic games (Snake, Minesweeper) where larger competitors (Codestral 22B, DeepSeek Coder V2 Lite 16B) produced buggy or non-functional code . This suggests that **benchmark scores do not fully capture coding reliability**—the 7B model's focused architecture may generalize better to iterative development tasks.

The **~4.7GB Q4_K_M footprint** enables deployment on **16GB RAM systems**, covering the majority of developer machines. The **10-15 tok/s on M2** and **~38 tok/s on desktop GPUs** supports responsive interactive workflows. This model serves as AXORA's **default tier** for routine code generation, autocomplete, and rapid iteration.

#### 1.1.3 Llama 3.3 8B — Speed-Optimized Alternative

**Llama 3.3 8B** prioritizes inference speed over absolute quality, achieving **72.6% HumanEval at Q4_K_M** with exceptional throughput .

| Attribute | Specification |
|-----------|-------------|
| **HumanEval (Q4_K_M)** | 72.6%  |
| **Quantized size (Q4_K_M)** | ~6GB |
| **Tokens/sec (M2)** | ~40  |
| **Tokens/sec (RTX 4090)** | 100-140  |
| **VRAM requirement** | ~6GB |

The **~40 tok/s on M2** and **100-140 tok/s on RTX 4090**—approximately **2.5-3.5× faster than Qwen 7B**—makes this ideal for **latency-critical autocomplete and rapid prototyping**. The 15% quality gap versus Qwen 7B is acceptable where speed dominates. The extensive Llama ecosystem (thousands of fine-tunes, mature tooling) provides operational advantages, though the **Llama License** requires review for commercial restrictions.

This model serves as AXORA's **fast tier** for autocomplete, inline suggestions, and scenarios where sub-100ms response is critical.

### 1.2 Quality Tier Assessment for AXORA

#### 1.2.1 Tier Classification Framework

| Tier | Definition | Representative Models | HumanEval Range | AXORA Suitability |
|------|-----------|----------------------|-----------------|-------------------|
| **Tier 1 (SOTA)** | Cloud frontier | GPT-4o (90.2%), Claude 4 (86%)  | 85-92% | Cloud fallback only |
| **Tier 2 (Good)** | Production local | Qwen 2.5 Coder 7B/32B, Llama 3.3 8B, Phi-4 14B | 70-88% | **Primary deployment target** |
| **Tier 3 (Basic)** | Functional limited | Mistral Small 3, Gemma 2, quantized legacy | 60-70% | Edge fallback, autocomplete only |

#### 1.2.2 Acceptability Threshold for AXORA

AXORA's multi-agent architecture demands **Tier 2 minimum for production deployment**. Analysis of agent functional requirements reveals heterogeneous quality needs:

| Agent Function | Minimum Tier | Preferred Model | Rationale |
|---------------|-------------|-----------------|-----------|
| Code generation | Tier 2 | Qwen 7B/32B | Reliable compilation, test passage |
| Debugging assistance | Tier 2 | Qwen 32B | Root cause analysis depth |
| Multi-file refactoring | Tier 2 | Qwen 32B | Cross-file consistency |
| Autocomplete | Tier 2-3 | Llama 3.3 8B or Qwen 7B | Speed priority, acceptable quality trade-off |
| Documentation | Tier 2 | Qwen 7B | Factual accuracy, lower reasoning demands |
| Inter-agent communication | Tier 2 | Qwen 7B | Consistent instruction following |

**Recommended deployment strategy:** **Qwen 2.5 Coder 7B at Q4_K_M as default**, with **automatic escalation to 32B at Q5_K_M** for detected complexity, and **Llama 3.3 8B as optional fast path** for latency-critical autocomplete.

---

## 2. Comprehensive Model Landscape (2025-2026)

### 2.1 Code-Specialized Models

#### 2.1.1 DeepSeek Coder V2 Family

DeepSeek's Coder V2 series introduces **Mixture-of-Experts (MoE) architecture** to code generation, with distinct deployment profiles across its size variants.

| Variant | Parameters | HumanEval | Quantized Size | Tokens/sec | Deployment |
|---------|-----------|-----------|---------------|------------|------------|
| V2 236B | 236B (21B active) | 72%  | ~120GB (Q4) | 2-4 | Workstation only (48GB+ VRAM) |
| V2 Lite 16B | 16B | 43%  | ~10GB (Q4_K_M) | 10-15 | Consumer viable |

The **V2 Lite 16B** achieves a "best balance" designation in some evaluations , but its **43% HumanEval lags substantially behind Qwen 2.5 Coder 7B's ~76%**. Real-world testing revealed reliability issues: failure to produce functional game implementations where Qwen 7B succeeded . The **MoE routing complexity** introduces latency variability and tooling constraints that reduce attractiveness for AXORA's default deployment.

The **Multi-head Latent Attention (MLA)** mechanism compresses KV cache by 93.3% , offering memory efficiency for long-context scenarios. However, this advantage is offset by ecosystem maturity gaps versus GGUF-based alternatives.

#### 2.1.2 Legacy and Specialized Models

| Model | HumanEval | Status | AXORA Relevance |
|-------|-----------|--------|-----------------|
| CodeLlama 34B | ~42%  | Superseded | Legacy compatibility only |
| StarCoder 2 15B | ~46% | Limited 2025-6 activity | Training transparency, provenance requirements |
| Phi-4 14B | 73.8%  | Microsoft ecosystem | Azure-integrated deployments |

**CodeLlama 34B** established foundational training methodologies but has been **substantially superseded** by Qwen and DeepSeek advances. **StarCoder 2 15B** retains value for organizations requiring **explicit training data auditability** (The Stack v2 provenance). **Phi-4 14B** offers competitive efficiency but lacks decisive advantages over Qwen for general deployment.

### 2.2 General-Purpose Models with Strong Code Performance

#### 2.2.1 Mistral Family

| Model | HumanEval | Speed | Characteristics |
|-------|-----------|-------|---------------|
| Mistral Small 3 7B | 68.2%  | ~50 tok/s | Fastest 7B-class, limited ecosystem |
| Mixtral 8x7B MoE | 74.5%  | ~25 tok/s | Quality via sparsity, complex routing |

**Mistral Small 3 7B** achieves the **highest token throughput in its class**—approximately **50 tok/s on 16GB hardware**—but its **68.2% HumanEval trails Qwen 7B by 8 percentage points**. The speed-quality trade-off may justify selection for specific latency-critical paths, though the smaller fine-tune ecosystem constrains customization.

**Mixtral 8x7B** delivers **74.5% HumanEval with ~13B active parameters**, but the **~26GB RAM requirement** and routing complexity limit deployment to enthusiast configurations.

#### 2.2.2 Emerging 2025-2026 Models

| Model | HumanEval | Notes | Evaluation Status |
|-------|-----------|-------|-----------------|
| Qwen 3 7B | 76%  | Improved multilingual, 128K context | Monitor for maturity |
| Gemma 2 9B/27B | Competitive | Prompt sensitivity, restrictive license | Secondary option |
| MiniMax 2.5 | Strong (Chinese) | Limited Western adoption | Deferred evaluation |

**Qwen 3 7B** (early 2025) shows incremental improvements but **lacks the specialized code optimization** of Qwen 2.5 Coder. **Gemma 2** exhibits **heightened sensitivity to prompt formatting** , requiring careful template engineering. **MiniMax 2.5** demonstrates competitive benchmarks but **limited tooling ecosystem** outside Chinese-language contexts.

---

## 3. Quantization Deep-Dive

### 3.1 Format Comparison and Trade-offs

Quantization enables local deployment by reducing model precision from FP16/BF16 to lower-bit representations. The choice of format fundamentally shapes quality, speed, and hardware compatibility.

| Format | Target Hardware | Size Reduction | Quality Loss (Code) | Speed Gain | Best Use Case |
|--------|-----------------|----------------|---------------------|------------|---------------|
| **GGUF Q8_0** | Universal | 50% | ~1% | 1.5× | Quality-critical production |
| **GGUF Q6_K** | Universal | 57% | ~1.5% | 1.8× | Premium 32B deployment |
| **GGUF Q5_K_M** | Universal | 64% | ~2-3% | 2.0× | Balanced quality/speed |
| **GGUF Q4_K_M** | **Universal** | **75%** | **~3-5%** | **2.5×** | **Recommended default** |
| GGUF Q3_K_M | CPU-constrained | 81% | ~5-8% | 3.0× | Emergency low-resource |
| GPTQ 4-bit | NVIDIA GPU | 75% | ~2-4% | 2-3× | CUDA-optimized inference |
| AWQ 4-bit | NVIDIA GPU | 75% | ~1-2% | 2-3× | Best GPU quality/speed |
| MLX (Apple) | Apple Silicon | 50-75% | ~1-3% | 2-4× | Native Apple optimization |

Data synthesized from , , , , 

The **GGUF format** (GPT-Generated Unified Format) has emerged as the **de facto standard for cross-platform deployment**, with llama.cpp providing optimized inference across CPU, Apple Silicon, and GPU backends. The **"K" variants** (Q4_K_M, Q5_K_M, Q6_K) employ **k-quantization with importance matrix weighting**—allocating higher precision to attention weights and feed-forward matrices identified as most sensitive to quantization .

**Q4_K_M achieves 75% size reduction with ~3-5% quality degradation**, validated as the practical floor for production code generation. Below this threshold, syntax errors and runtime failures increase non-linearly . The ICPC 2026 replication study established **4-bit precision as the "new frontier"** for code LLM quantization: 70% memory reduction without significant performance decrease, with 3-bit requiring code-specific calibration to limit quality loss .

### 3.2 Code-Specific Quantization Considerations

Code generation exhibits **greater quantization sensitivity than conversational tasks** due to precise syntax requirements and semantic dependencies. Empirical analysis reveals:

| Quantization Level | Syntactic Validity | Semantic Accuracy | Runtime Success |
|-------------------|-------------------|-------------------|-----------------|
| Q8_0 | >99% | ~99% | Excellent |
| Q6_K | >98% | ~97% | Very Good |
| Q5_K_M | >97% | ~95% | Good |
| **Q4_K_M** | **>95%** | **~92%** | **Good (recommended)** |
| Q3_K_M | ~92% | ~85% | Degraded |
| Q2_K | ~85% | ~75% | Poor |

The **structured nature of programming languages amplifies weight perturbation effects**: bracket mismatches, type errors, and API hallucinations become prevalent below Q4 precision. However, **Q4_K_M maintains ~92% semantic accuracy** for typical coding workflows, with degradation manifesting primarily in complex multi-step reasoning rather than routine pattern completion .

### 3.3 Recommended Configuration for AXORA

| Deployment Scenario | Model | Quantization | Size | Expected Quality |
|---------------------|-------|--------------|------|------------------|
| **Default agent inference** | Qwen 2.5 Coder 7B | **Q4_K_M** | ~4.7GB | ~76% HumanEval |
| Premium quality path | Qwen 2.5 Coder 32B | Q5_K_M | ~22GB | ~65% HumanEval |
| Maximum speed autocomplete | Llama 3.3 8B | Q4_K_M | ~6GB | 72.6% HumanEval |
| Critical code review | Qwen 2.5 Coder 32B | Q8_0 | ~32GB | ~88% HumanEval |
| Emergency low-resource | Qwen 2.5 Coder 7B | Q3_K_M | ~2.7GB | ~68% HumanEval |

---

## 4. Inference Engine Evaluation

### 4.1 Production-Ready Engine Comparison

| Engine | Maturity | Rust Integration | Relative Performance | Best For |
|--------|----------|------------------|----------------------|----------|
| **Ollama** | ⭐⭐⭐⭐⭐ (120K+ stars) | HTTP API (`ollama-rs`) | ~90% of theoretical | **Recommended default** |
| llama.cpp | ⭐⭐⭐⭐⭐ (reference) | `llama-cpp-rs` (mature) | 100% baseline | Maximum control |
| MLX | ⭐⭐⭐⭐☆ | `mlx-rs` (limited) | ~150% on Apple | Apple-only optimization |
| vLLM | ⭐⭐⭐⭐☆ | Limited | 3-5× batched | GPU batching, not local |
| Candle | ⭐⭐⭐☆☆ | Native | ~70% of llama.cpp | Pure Rust stack |
| TGI | ⭐⭐⭐⭐☆ | Limited | Enterprise-grade | Cloud deployment |

Assessment from , , , , 

### 4.2 Ollama: Recommended Default

Ollama has established itself as the **"Docker for LLMs"**—an abstraction layer that transforms model deployment complexity into simple, reproducible operations . For AXORA, this operational simplicity is decisive.

**Key capabilities:**
- **Zero-configuration deployment**: `ollama pull qwen2.5-coder:7b` delivers optimized, verified model
- **OpenAI-compatible API**: Drop-in replacement for cloud providers, simplifying migration
- **Hot model swapping**: Runtime transitions between models without restart
- **Cross-platform consistency**: Identical behavior macOS, Linux, Windows (WSL)

**Performance characteristics:** Approximately **5-10% overhead versus raw llama.cpp** —acceptable given operational benefits. The overhead derives from API layer and process isolation, not inference inefficiency; token generation uses identical optimized kernels.

**Rust integration** proceeds via `ollama-rs` crate with async/await support, streaming responses, and connection pooling. The service-oriented architecture—AXORA agents communicating with Ollama via local HTTP—mirrors cloud API patterns, facilitating future hybrid deployments.

### 4.3 Direct llama.cpp Integration

For scenarios requiring **maximum control or <50ms latency**, direct `llama-cpp-rs` integration provides:

- Complete GGUF feature support (all quantization variants)
- GPU offloading flexibility (CUDA, Metal, Vulkan)
- Custom scheduling and speculative decoding
- Embedded deployment without service boundaries

The **~5-10% performance advantage** over Ollama is offset by **substantial operational complexity**: manual model management, cross-platform compilation, and crash isolation become AXORA's responsibility. Recommended for latency-critical paths or environments where external processes are prohibited.

### 4.4 Alternative Engines

**MLX** (Apple Machine Learning) achieves **~150% of llama.cpp performance on Apple Silicon** through Metal Performance Shaders and unified memory optimization . However, **platform exclusivity and limited Rust bindings** constrain universal deployment. Evaluate for Apple-dominant deployments willing to maintain parallel code paths.

**vLLM** delivers **3-5× throughput for batched requests** via PagedAttention , but its **GPU-only focus and serving-oriented design** mismatch AXORA's interactive, single-user deployment model. Relevant only for cloud fallback infrastructure or team server scenarios.

**Candle** offers **pure Rust implementation** eliminating C++ dependencies, but **~30% performance gap** and limited model support  relegate it to specialized embedded scenarios.

---

## 5. Hardware Requirements and Performance

### 5.1 Memory Requirements (Updated 2025-2026)

| Model Size | FP16 | Q8_0 | Q5_K_M | **Q4_K_M** | Min RAM | Comfortable RAM |
|------------|------|------|--------|-----------|---------|-----------------|
| 3B | 6GB | 3GB | 2GB | **1.5GB** | 4GB | 8GB |
| 7B | 14GB | 7GB | 5GB | **4.7GB** | 8GB | 16GB |
| 14B | 28GB | 14GB | 10GB | **9GB** | 16GB | 32GB |
| 32B | 64GB | 32GB | 22GB | **20GB** | 32GB | 64GB |
| 70B | 140GB | 70GB | 48GB | **40GB** | 64GB | 128GB |

The **"comfortable RAM"** column includes operating system overhead (4-8GB), development tooling (2-4GB), and inference working memory beyond model weights. For AXORA's multi-agent architecture, additional headroom accommodates concurrent model loading and context accumulation.

### 5.2 Token Throughput by Hardware Platform

#### 5.2.1 Apple Silicon (Ollama/llama.cpp, Q4_K_M)

| Chip | Unified Memory | 7B tok/s | 14B tok/s | 32B tok/s | Notes |
|------|---------------|----------|-----------|-----------|-------|
| M1 (8GB) | 68 GB/s | 10-15 | — | — | Barely viable, swap pressure |
| M2 Pro | 200 GB/s | 26-32 | 14-18 | — | Good development machine |
| M2 Max | 400 GB/s | 35-42 | 22-28 | 6-8 | Recommended for 32B |
| **M3 Max** | 400 GB/s | **40-50** | **26-34** | **8-10** | **Best Apple option** |
| M3 Ultra | 800 GB/s | 55-68 | 35-45 | 12-15 | Workstation replacement |

Data from , , , 

Apple Silicon performance scales with **memory bandwidth and thermal headroom**. The M3 Max represents the current sweet spot: **40-50 tok/s for 7B models** enables responsive interaction, while **8-10 tok/s for 32B** remains viable for quality-critical tasks. Fanless designs (MacBook Air) throttle aggressively; active cooling (MacBook Pro, Mac Studio) sustains peak performance.

#### 5.2.2 NVIDIA GPUs (CUDA, Q4_K_M)

| GPU | VRAM | Memory BW | 7B tok/s | 14B tok/s | 32B tok/s | Power |
|-----|------|-----------|----------|-----------|-----------|-------|
| RTX 3060 12GB | 12GB | 360 GB/s | 25-35 | 12-18 | — | 170W |
| RTX 4070 Ti | 12GB | 504 GB/s | 60-80 | 35-50 | 15-20 | 285W |
| RTX 4080 | 16GB | 717 GB/s | 80-100 | 50-70 | 22-28 | 320W |
| **RTX 4090** | **24GB** | **1,008 GB/s** | **100-140** | **60-90** | **30-40** | **450W** |

The **RTX 4090's 24GB VRAM** enables **full 32B model residence without system memory fallback**—a decisive advantage for latency-sensitive applications. The **1 TB/s memory bandwidth** delivers exceptional throughput, though **450W power draw** requires substantial PSU and cooling.

#### 5.2.3 Intel/AMD CPUs

Modern desktop CPUs with **AVX-512 or AMX** achieve **5-15 tok/s for 7B models**—viable for autocomplete and background tasks but marginal for interactive generation. Laptop CPUs typically deliver **2-8 tok/s**, suitable only for lightweight assistance with clear quality-speed trade-offs. CPU-only deployment is **not recommended for AXORA's primary use cases**.

### 5.3 Minimum Viable Configuration for AXORA

| Tier | Hardware | Model | Quantization | Performance | Experience |
|------|----------|-------|--------------|-------------|------------|
| **Minimum** | 16GB RAM, M2/RTX 3060 | Qwen 2.5 Coder 7B | Q4_K_M | 10-15 tok/s | Functional, occasional delays |
| **Recommended** | 32GB RAM, M3 Max/RTX 4070 | Qwen 2.5 Coder 7B/32B | Q4_K_M/Q5_K_M | 20-40 tok/s | Smooth, responsive |
| **Optimal** | 64GB RAM, M3 Ultra/RTX 4090 | Qwen 2.5 Coder 32B | Q5_K_M | 30-40 tok/s | Near-cloud quality |
| **Workstation** | 128GB RAM, dual RTX 4090 | Qwen 2.5 Coder 32B/70B | Q4_K_M | 15-25 tok/s | Maximum local capability |

The **>10 tok/s threshold** emerges from cognitive research on interactive system responsiveness: below this rate, users perceive delay and context-switch; above it, generation feels "live" .

---

## 6. Multi-Model Strategy and Hybrid Cloud Integration

### 6.1 Task-Based Model Routing

AXORA's heterogeneous agent workload benefits from **intelligent model selection** matching computational resources to task requirements.

#### 6.1.1 Fast Path: 7B Models for Latency-Critical Tasks

| Task Category | Model | Target Latency | Quality Expectation |
|---------------|-------|---------------|---------------------|
| Autocomplete (single token) | Llama 3.3 8B or Qwen 7B | <50ms | Syntactic correctness |
| Inline suggestions | Qwen 2.5 Coder 7B | <100ms | Local context match |
| Simple function generation | Qwen 2.5 Coder 7B | <500ms | 80%+ test passage |
| Syntax error fixing | Qwen 2.5 Coder 7B | <200ms | Compilation success |
| Documentation (docstrings) | Qwen 2.5 Coder 7B | <1s | Accuracy, completeness |

The fast path prioritizes **responsiveness over maximum quality**. User studies indicate **60%+ willingness to accept quality trade-offs for speed** in iterative exploration contexts, with explicit escalation paths for refinement .

#### 6.1.2 Quality Path: 32B Models for Complex Tasks

| Task Category | Model | Target Latency | Quality Expectation |
|---------------|-------|---------------|---------------------|
| Multi-file refactoring | Qwen 2.5 Coder 32B | <5s | Semantic preservation |
| Architecture design | Qwen 2.5 Coder 32B | <10s | Pattern appropriateness |
| Complex algorithm implementation | Qwen 2.5 Coder 32B | <5s | Correctness, efficiency |
| Subtle bug diagnosis | Qwen 2.5 Coder 32B | <10s | Root cause identification |
| Comprehensive test generation | Qwen 2.5 Coder 32B | <5s | Coverage, edge cases |

The quality path accepts **3-5× latency increase** for **substantial reasoning depth improvement**. Automatic escalation triggers on: detected complexity (token count, AST depth, dependency graph size), explicit user request, or fast-path confidence below threshold.

#### 6.1.3 Router Implementation Strategies

| Approach | Mechanism | Advantages | Disadvantages |
|----------|-----------|------------|---------------|
| **Heuristic** | Token count, keywords, file complexity | Fast, interpretable, no training data | Brittle to novel tasks |
| **Embedding-based** | Small classifier on task embedding | Adapts to usage patterns, improves over time | Requires labeled data, adds ~10ms latency |
| **Confidence-based** | Fast-path generation with quality check | Maximizes fast-path utilization | Doubles latency for escalated tasks |

**Recommended:** Hybrid approach—heuristic for clear cases, confidence-based for ambiguous tasks, with **user override always available**.

### 6.2 Cloud Fallback Strategy

Despite local model advances, **cloud frontier models retain advantages** for specific scenarios. Seamless integration preserves user experience while optimizing cost and quality.

#### 6.2.1 Trigger Conditions for Cloud Escalation

| Condition | Detection Method | Cloud Target |
|-----------|---------------|--------------|
| Local model confidence < threshold | Perplexity, consistency checks, self-evaluation | GPT-4o, Claude 4 Sonnet |
| Task complexity score > threshold | AST metrics, dependency analysis, estimated reasoning depth | Claude 4 (reasoning), GPT-4o (generation) |
| Explicit user request | UI toggle, prompt directive ("use best model") | User preference |
| Hardware unavailable | Battery mode, thermal throttling, OOM | Default cloud |
| Time-critical deadline | Calendar integration, user preference | Fastest available |

#### 6.2.2 Seamless Integration Architecture

| Pattern | Description | User Experience |
|---------|-------------|---------------|
| **Shadow mode** | Local and cloud in parallel; cloud replaces if substantially better | Transparent, maximum quality |
| **Upgrade stream** | Begin local, background quality check triggers cloud continuation | Responsive start, quality guarantee |
| **Explicit choice** | Clear UI indication of local/cloud with trade-offs | Informed consent, educational |

**Cost management** requires attention: cloud API costs scale with usage, potentially exceeding local hardware amortization for heavy users. Implement **per-user quotas, organizational budgets, and transparent cost attribution**.

---

## 7. Rust Implementation Architecture

### 7.1 Recommended Dependency Stack

```toml
[dependencies]
# Primary: Ollama service integration
ollama-rs = { version = "0.2", features = ["stream"] }
reqwest = { version = "0.12", features = ["json", "stream"] }

# Async runtime
tokio = { version = "1.35", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Optional: Direct llama.cpp for advanced scenarios
# llama-cpp-rs = { version = "0.3", features = ["cuda", "metal"] }

# Optional: Pure Rust fallback for specific embedded scenarios
# candle-core = "0.8"
# candle-transformers = "0.8"

# Observability
tracing = "0.1"
metrics = "0.24"
```

### 7.2 Integration Patterns

#### 7.2.1 Service-Based Integration (Recommended)

```rust
use ollama_rs::Ollama;
use ollama_rs::generation::completion::request::GenerationRequest;

pub struct InferenceClient {
    client: Ollama,
    default_model: String,
    timeout: Duration,
}

impl InferenceClient {
    pub async fn generate(
        &self,
        prompt: &str,
        context: Option<Vec<u64>>,
    ) -> Result<GenerationResponse, InferenceError> {
        let request = GenerationRequest::new(self.default_model.clone(), prompt.to_string())
            .options(GenerationOptions::default()
                .temperature(0.2)
                .num_ctx(8192)
                .num_predict(2048));
        
        // Streaming for responsive UX
        self.client.generate_stream(request).await
    }
}
```

**Benefits:** Clean separation, independent scaling, operational visibility, multi-language client support. **Trade-off:** ~5-10% performance overhead versus direct integration.

#### 7.2.2 Embedded Library Integration

For latency-critical paths or deployment constraints prohibiting external services:

```rust
use llama_cpp_rs::{LlamaModel, LlamaParams, LlamaContext};

pub struct EmbeddedEngine {
    model: LlamaModel,
    context: LlamaContext,
    gpu_layers: i32,
}

impl EmbeddedEngine {
    pub fn new(model_path: &Path, gpu_layers: i32) -> Result<Self, ModelError> {
        let params = LlamaParams::default()
            .n_gpu_layers(gpu_layers)
            .n_ctx(8192);
        // Direct loading, custom scheduling, speculative decoding
    }
}
```

**Benefits:** Maximum control, minimum latency, custom optimizations. **Trade-off:** Integration complexity, manual resource management.

### 7.3 Performance Optimizations

| Technique | Implementation | Expected Gain | Complexity |
|-----------|---------------|-------------|------------|
| **Request batching** | Dynamic batch size based on latency SLO | 20-40% throughput | Low |
| **KV cache persistence** | LRU cache with compression (60% memory, 3% quality) | 30-50% latency reduction | Medium |
| **Speculative decoding** | 1.5B draft model, 2-3× speedup on patterns | 2-3× for repetitive code | High |
| **Prefix caching** | Shared attention state for common prompts | 40-60% prompt processing | Medium |

---

## 8. Quality Evaluation Framework

### 8.1 Benchmark Suite

| Benchmark | Purpose | Target (7B) | Target (32B) | Validation Frequency |
|-----------|---------|-------------|--------------|----------------------|
| HumanEval | Function-level generation | >70% | >85% | Per model release |
| MBPP | Python programming problems | >60% | >75% | Per model release |
| SWE-Bench Verified | Real-world bug fixing | N/A | >40% | Quarterly |
| Aider | Multi-file editing | >50% | >65% | Quarterly |
| LiveCodeBench | Competitive programming | >50% | >70% | Quarterly |
| **Custom AXORA suite** | Agent-specific tasks | Establish baseline | >90% of 7B | Per feature release |

### 8.2 Real-World Validation

**Evaluation dataset construction:**
- **100+ representative agent tasks** covering AXORA's operational domain
- **Multi-language coverage:** Python, JavaScript/TypeScript, Rust, Go, Java
- **Framework diversity:** Web (React, Django), systems (Axum, Tokio), data processing
- **Difficulty grading:** Simple (single function), medium (class/module), complex (multi-file architecture)

**A/B testing infrastructure:**
- **Shadow mode:** Local and cloud inference on identical tasks
- **Outcome scoring:** Compilation success, test passage, human review
- **User satisfaction:** Explicit ratings, implicit signals (acceptance rate, edit distance)
- **Error categorization:** Syntax errors, logic errors, runtime failures, style violations

### 8.3 User Perception and Tolerance

| Metric | Acceptable | Good | Excellent | Research Basis |
|--------|-----------|------|-----------|---------------|
| Autocomplete latency | <300ms | <150ms | <50ms | Flow state preservation  |
| Generation latency | <3s | <1s | <500ms | Task completion satisfaction |
| Quality gap vs cloud | <30% degradation | <15% degradation | Parity | Informed user acceptance |
| Error rate | <20% require fix | <10% | <5% | Trust and adoption |

**60%+ of users accept cloud fallback for critical tasks** when quality advantage exceeds 20% and cost is transparent .

---

## 9. Risk Assessment and Mitigation

| Risk | Likelihood | Impact | Mitigation Strategy |
|------|------------|--------|---------------------|
| Quantization quality degradation | Medium | High | Validate on custom suite; maintain Q5_K_M upgrade path; monitor error rates |
| Hardware diversity support | High | Medium | CI matrix: M1/M2/M3, RTX 3060/4070/4090, CPU-only; graceful degradation |
| Model obsolescence (6-month cycle) | High | Medium | Abstraction layer; automated evaluation pipeline; quarterly model review |
| Ollama breaking changes | Low | Medium | Version pinning; integration test suite; migration path planning |
| Cloud fallback cost overruns | Medium | High | Rate limiting; per-user quotas; cost transparency; budget alerts |
| Privacy-sensitive code leakage | Low | Critical | Local-first default; audit logging; optional air-gapped mode; enterprise VPC |

---

## 10. Implementation Roadmap

### Phase 1: Foundation (Weeks 1-4)
- Ollama integration with `ollama-rs` crate
- Qwen 2.5 Coder 7B @ Q4_K_M as default model
- Basic agent inference API with health checks and metrics
- Cross-platform packaging (macOS .app, Linux AppImage, Windows MSI)

**Success criteria:** >20 tok/s on recommended hardware, <500ms p99 latency, functional on 16GB systems

### Phase 2: Optimization (Weeks 5-8)
- KV cache persistence across agent sessions
- Request batching for concurrent operations
- 32B model support with automatic hardware detection
- Performance profiling and bottleneck elimination

**Success criteria:** 30% throughput improvement, 32B functional on 32GB+ systems

### Phase 3: Intelligence (Weeks 9-12)
- Task-based model router (heuristic → learned)
- Speculative decoding with 1.5B draft model
- Custom AXORA benchmark suite establishment
- A/B testing infrastructure

**Success criteria:** 90% optimal model selection, 2× speedup on repetitive patterns, validated quality targets

### Phase 4: Production Hardening (Weeks 13-16)
- Cloud fallback integration with cost management
- Advanced optimizations (grammar constraints, structured output)
- Security audit, dependency scanning, supply chain validation
- Comprehensive documentation and operational runbooks

**Success criteria:** 99.9% availability, <1% error rate, complete documentation, production-ready

---

## 11. Source Documentation

### 11.1 Benchmark and Model Sources
- SitePoint Local LLM Benchmarks 2026 
- Local AI Master Model Comparison 
- Qwen 2.5 Coder Technical Report (Alibaba, 2024) 
- Deepgram Local Coding LLM Analysis 
- ICPC 2026 Quantization Impact Study 
- RankedAGI Qwen 2.5 Coder 7B Evaluation 

### 11.2 Inference Engine Comparisons
- Ollama vs llama.cpp Performance Analysis 
- Premai.io vLLM Alternatives 2026 
- Red Hat Developer vLLM vs llama.cpp 
- Candle Framework Evaluation 
- MLX Apple Silicon Optimization 

### 11.3 Hardware Performance Analyses
- Apple Silicon Token Throughput Matrix 
- NVIDIA GPU Benchmark Suite 
- Singh Ajit Local LLM Speed Benchmarks 
- Consumer Hardware Viability Study 

### 11.4 Model Release Documentation
- Qwen 2.5/3 Series (Alibaba, 2024-2025) 
- Llama 3.3 (Meta, late 2025)
- DeepSeek Coder V2 (DeepSeek, June 2024) 
- Mistral Small 3 (Mistral, 2025) 
- Ollama Library Model Documentation 

