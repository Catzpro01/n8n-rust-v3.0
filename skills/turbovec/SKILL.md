---
name: turbovec
description: >-
  High-performance vector indexing using Rust-based TurboQuant algorithm.
  Enables efficient semantic similarity search for RAG pipelines with
  minimal memory footprint.
---

# TurboVec — High-Performance Vector Index

## What Is It
TurboVec provides extremely fast approximate nearest-neighbor (ANN) search for embedding vectors, using Google's TurboQuant compression algorithm implemented in Rust.

## Performance
- 10-100x faster than naive brute-force search
- 60-80% memory reduction vs. full-precision float32
- Sub-millisecond queries on 1M vector datasets

## Usage

### Build Index
```python
from turbovec import VectorIndex

index = VectorIndex(
    dimensions=1536,    # OpenAI ada-002 dimension
    quantization="Q4"   # Compression level
)

# Add vectors
for doc_id, embedding in enumerate(embeddings):
    index.add(doc_id, embedding)

index.build()          # Build the ANN structure
index.save("./index")  # Persist to disk
```

### Query
```python
index = VectorIndex.load("./index")

# Search
query_embedding = embed("What is Python?")
results = index.search(query_embedding, top_k=10)

for doc_id, score in results:
    print(f"Doc {doc_id}: similarity={score:.4f}")
    print(documents[doc_id])
```

### RAG Integration
```python
def rag_query(question: str, top_k: int = 5) -> str:
    # 1. Embed question
    q_embedding = embed(question)
    
    # 2. Find similar docs
    results = index.search(q_embedding, top_k=top_k)
    
    # 3. Build context
    context = "

".join(documents[doc_id] for doc_id, _ in results)
    
    # 4. Generate answer
    return llm.complete(f"Context:
{context}

Question: {question}")
```

## Security Notes
⚠️ FFI/C-bindings: Update to latest version to patch potential memory issues
✅ Purely local — no network access
✅ Vectors are encoded data, not executable — safe to store
