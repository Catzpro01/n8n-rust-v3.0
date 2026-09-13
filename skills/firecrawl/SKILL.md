---
name: firecrawl
description: >-
  Web crawling/scraping engine that converts websites into clean Markdown for LLM 
  consumption. Handles JavaScript-rendered pages, PDFs, and structured data extraction.
  AGPL-3.0. Use responsibly and legally.
---

# Firecrawl — Web to LLM Pipeline

## Core Capabilities
- Convert any URL to clean Markdown (removes nav, ads, boilerplate)
- Full-site crawling with configurable depth
- JavaScript-rendered page support (headless browser)
- Structured data extraction via schemas
- PDF and document parsing

## Usage Patterns

### Single Page Scrape
```python
from firecrawl import FirecrawlApp
app = FirecrawlApp(api_key="your-key")
result = app.scrape_url("https://docs.example.com/api")
print(result['markdown'])
```

### Site Crawl
```python
result = app.crawl_url(
    "https://docs.example.com",
    params={"crawlerOptions": {"maxDepth": 2, "limit": 50}}
)
```

### Structured Extraction
```python
result = app.scrape_url(
    "https://example.com/product",
    params={
        "extractorOptions": {
            "extractionSchema": {
                "type": "object",
                "properties": {
                    "price": {"type": "string"},
                    "name": {"type": "string"}
                }
            }
        }
    }
)
```

## Security Rules
⚠️ **SSRF Risk**: Never point Firecrawl at internal/intranet URLs (192.168.x.x, 10.x.x.x, localhost)
⚠️ **Legal**: Only scrape sites where you have permission or ToS allows it
⚠️ **Rate Limiting**: Implement delays between requests to avoid DoS
✅ Always check `robots.txt` before crawling
✅ Respect `Crawl-delay` directives
