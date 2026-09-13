---
name: scrapling
description: >-
  Adaptive stealth web scraper with self-healing parsers and anti-bot bypass
  capabilities. BSD-3-Clause. Use legally and responsibly.
---

# Scrapling — Adaptive Web Scraping

> ⚠️ LEGAL WARNING: Only scrape sites where permitted by Terms of Service
> or where you have explicit permission. Respect robots.txt.

## Key Features
- **Self-healing parsers**: If HTML structure changes, automatically adapts
- **Anti-bot stealth**: TLS fingerprint randomization, realistic headers
- **Playwright integration**: For JavaScript-heavy sites
- **Auto-retry**: Exponential backoff on failures

## Basic Usage

### Simple Scrape
```python
from scrapling import Scraper

scraper = Scraper(stealth=True)
page = scraper.get("https://example.com/products")

# CSS selector
products = page.find_all(".product-card")
for product in products:
    print(product.find(".price").text)
```

### With Auto-Match (Self-Healing)
```python
# Train on known good data
scraper.train(
    url="https://example.com/product/1",
    examples={"title": "Widget Pro", "price": "$29.99"}
)

# Will adapt if site structure changes
data = scraper.extract("https://example.com/product/2")
```

### Async Batch Scraping
```python
import asyncio
from scrapling import AsyncScraper

async def scrape_batch(urls):
    async with AsyncScraper(stealth=True, concurrency=5) as scraper:
        results = await scraper.get_all(urls)
    return results

results = asyncio.run(scrape_batch(url_list))
```

## Responsible Use

### Always Do
- Check `robots.txt` before scraping
- Implement delays: `time.sleep(random.uniform(1, 3))`
- Use rotating user agents
- Respect `Retry-After` headers
- Set reasonable concurrency limits (max 5)

### Never Do
- Scrape login-protected content without authorization
- Ignore explicit `Disallow:` in robots.txt
- DDoS sites with excessive concurrent requests
- Scrape and republish copyrighted content
- Use for credential stuffing or fraud

## Rate Limiting
```python
scraper = Scraper(
    delay=2.0,          # 2 second base delay
    delay_jitter=1.0,   # ± 1 second random
    max_retries=3,      # Retry failed requests
    backoff=2.0         # Exponential backoff multiplier
)
```
