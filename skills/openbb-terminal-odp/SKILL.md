---
name: openbb-terminal-odp
description: Institutional-grade financial data engine (OpenBB ODP & SEC EDGAR / FRED). Fetches global equities, fundamentals, SEC 10-K/Q, macro indicators, and dividend/earnings calendars.
---

# OpenBB Terminal & Bloomberg-Tier Market Data Engine

## Purpose
Provides an institutional-grade, 100% free financial research pipeline aggregating fundamental ratios, SEC 10-K/Q audited reports, institutional 13F ownership, macroeconomic series (FRED), and global stock price histories (US, IDX/IHSG, Global).

## Core Capabilities
1. **Equity Fundamentals & Valuation**:
   - P/E, P/B, EV/EBITDA, Free Cash Flow Yield, ROE, Debt-to-Equity.
   - Audited financial statements (Income Statement, Balance Sheet, Cash Flow).
2. **SEC EDGAR & Institutional Tracking**:
   - Form 10-K (Annual), Form 10-Q (Quarterly).
   - Form 13F (Whale & institutional holdings like Berkshire, BlackRock, Vanguard).
   - Form 4 (Insider Buying/Selling signals).
3. **Macroeconomics & Central Bank Series (FRED API)**:
   - Federal Reserve Interest Rate, CPI/PCE Inflation, 10Y-2Y Treasury Yield Curve, M2 Money Supply, Unemployment (NFP).
4. **Corporate Actions**:
   - Dividend history/yield, earnings calendar, analyst consensus price targets.

## Execution Guidance
- Use Python scripts utilizing yfinance, SEC EDGAR REST API, and public FRED endpoints to fetch verified financial numbers without hallucination.
- Format tabular data cleanly with key metrics highlighted.
- Ground all valuation models strictly to audited financial figures.
