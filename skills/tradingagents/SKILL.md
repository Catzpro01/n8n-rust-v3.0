---
name: tradingagents
description: >-
  Multi-agent LLM financial trading simulation framework. Strictly operates in
  SIMULATION / PAPER-TRADING mode only to prevent monetary loss.
---

# TradingAgents — Financial Trading Simulation

> ⚠️ MANDATORY MONETARY GUARDRAIL:
> 1. STRICTLY PAPER TRADING ONLY. Live trading API keys are FORBIDDEN.
> 2. All orders must be human-in-the-loop verified before execution.

## Safety Rules
- Mode MUST be set to `SIMULATION` or `PAPER_TRADING`.
- Do NOT connect real broker API secret keys with trade execution privileges.
- Use historical or delayed market feeds for backtesting.

## Architecture
- Analyst Agent: Evaluates market indicators.
- Risk Manager Agent: Enforces maximum drawdown thresholds.
- Executor Agent: Generates simulated trade proposals (NO live execution).
