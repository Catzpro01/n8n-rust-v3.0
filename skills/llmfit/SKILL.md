---
name: llmfit
description: >-
  CLI/TUI hardware sizing tool for local LLMs. Measures CPU, RAM, and VRAM
  and determines which open-source models can run efficiently on the machine.
---

# LLMFit — Hardware × Model Compatibility

## Quick Start
```bash
pip install llmfit
llmfit scan        # Scan hardware specs
llmfit check 7b   # Check if 7B models fit
llmfit matrix     # Full compatibility matrix
```

## What It Measures
- Total RAM and available RAM
- GPU model, VRAM total, and VRAM available
- CPU cores and architecture
- Storage speed (affects model load time)

## Compatibility Matrix Output
```
Hardware: 16GB RAM, RTX 3080 (10GB VRAM)

Model           | Quant  | VRAM   | RAM    | Fits?
----------------|--------|--------|--------|-------
Llama 3.1 8B   | Q4_K_M | 4.7GB  | 8GB    | ✅ GPU
Llama 3.1 8B   | Q8_0   | 9.1GB  | 8GB    | ✅ GPU (tight)
Llama 3.1 70B  | Q4_K_M | 39GB   | 8GB    | ❌ Too large
Mistral 7B     | Q4_K_M | 4.1GB  | 8GB    | ✅ GPU
Phi-3-medium   | Q4_K_M | 7.8GB  | 4GB    | ✅ GPU
```

## Use Cases
1. Before downloading a model — check if it fits
2. Before setting up Ollama — know your limits
3. Comparing machines for AI workloads
4. Choosing quantization level for best quality/speed tradeoff

## Notes
- CPU inference is 10-50x slower than GPU — consider task requirements
- For coding tasks: 7B models often sufficient with good prompting
- For complex reasoning: 13B+ recommended
- llmfit is purely diagnostic — no network calls, no data sent externally
