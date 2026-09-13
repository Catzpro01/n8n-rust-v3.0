---
name: autoscraper
description: >-
  Smart and lightweight automatic web scraper (alirezamika/autoscraper) that learns scraping rules from examples.
---

# AutoScraper — Example-Based Web Scraping

## Usage
```python
from autoscraper import AutoScraper
scraper = AutoScraper()
results = scraper.build(url, wanted_list)
```

## Guardrails
- Validate extracted data against target schemas
- Respect robots.txt and apply request rate limits
