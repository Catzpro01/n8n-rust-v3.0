---
name: whichllm
description: >-
  Hardware profiler and open-source LLM recommender. Analyzes local system specs
  (CPU, RAM, VRAM) and recommends which LLMs can run efficiently on that hardware.
---

# WhichLLM — Hardware-Aware LLM Selector

## Usage
```bash
# Analyze current system
python -m whichllm

# Output includes:
# - RAM available
# - VRAM available (GPU)
# - Recommended models by size (1B, 7B, 13B, 34B, 70B)
# - Quantization options (Q4, Q5, Q8, F16)
# - Estimated inference speed (tokens/sec)
```

## Model Selection Guide

### By VRAM
| VRAM | Recommended Models |
|------|-------------------|
| 4GB | Phi-3-mini, Gemma 2B Q8, Llama 3.2 1B |
| 8GB | Llama 3.2 3B, Mistral 7B Q4, Phi-3-small |
| 16GB | Llama 3.1 8B, Mistral 7B Q8, Gemma 9B |
| 24GB | Llama 3.1 70B Q4, Qwen 14B, DeepSeek 16B |
| 48GB+ | Llama 3.1 70B Q8, Mixtral 8x7B |

### By Use Case
| Use Case | Recommended Model Family |
|----------|------------------------|
| Code | DeepSeek-Coder, Qwen-Coder, CodeLlama |
| Chat | Llama 3.1, Mistral, Gemma |
| RAG | Nomic-embed-text, BGE-M3 |
| Vision | LLaVA, Phi-3-vision, InternVL |

## Quantization Notes
- **Q4_K_M**: Best balance of size and quality
- **Q5_K_M**: Slightly better quality, 20% larger
- **Q8_0**: Near-lossless, needs 2x RAM vs Q4
- **F16**: Full precision, for GPU with enough VRAM

## Integration with Ollama
```bash
# Pull recommended model
ollama pull llama3.2:3b
ollama pull nomic-embed-text

# Test performance
ollama run llama3.2:3b "Write a bubble sort in Python"
```
