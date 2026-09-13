---
name: chandra-ocr
description: >-
  Advanced multilingual OCR engine for extracting text from documents, tables,
  and formulas across 90+ languages. Outputs Markdown and structured JSON.
---

# Chandra OCR — Multilingual Document Extraction

## Supported Input Formats
- PDF (native text + scanned)
- PNG, JPEG, TIFF, WebP
- Word documents (.docx)
- Excel with embedded images
- Scanned business cards, receipts, invoices

## Capabilities
- Text extraction from 90+ languages
- Table recognition → Markdown/CSV/JSON
- Mathematical formula detection → LaTeX
- Handwriting recognition (English, simplified Chinese)
- Layout-aware parsing (columns, headers, footnotes)

## Usage

### Basic OCR
```python
from chandra_ocr import OCREngine

ocr = OCREngine(language="auto")  # Auto-detect language

# From file
result = ocr.extract("document.pdf")
print(result.text)         # Plain text
print(result.markdown)     # Formatted Markdown
print(result.json)         # Structured JSON with positions

# From URL
result = ocr.extract_url("https://example.com/report.pdf")
```

### Table Extraction
```python
result = ocr.extract("invoice.pdf", extract_tables=True)
for table in result.tables:
    df = table.to_dataframe()
    print(df.to_csv())
```

### Batch Processing
```python
import glob
files = glob.glob("./documents/*.pdf")
results = ocr.batch_extract(files, parallel=4)
```

## Performance Tips
- PDF with embedded text: instant (no OCR needed)
- Scanned PDF: ~2-5 seconds per page
- High-resolution images: higher accuracy, slower
- Use `dpi=150` for speed vs `dpi=300` for accuracy

## Security Notes
⚠️ Decompression bomb risk: Large PDFs/images can exhaust memory
✅ Set file size limit: `ocr = OCREngine(max_file_size_mb=50)`
✅ No network calls for local processing
✅ Sensitive documents processed locally — no cloud upload
