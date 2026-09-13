---
name: risk-parity-guardian
description: Portfolio capital preservation shield, Fractional Kelly Criterion position sizing, ATR adaptive stop-loss, Value-at-Risk (VaR), and paper-trading safety locks.
---

# Risk Parity Guardian & Capital Preservation Shield

## Purpose
Acts as an unbreachable risk gatekeeper enforcing mathematical position sizing, volatility-adjusted stop losses, portfolio diversification, and strict paper-trading safeguards.

## Core Rules & Capabilities
1. **Mathematical Position Sizing**:
   - Uses **Fractional Kelly Criterion** (typically 0.2x - 0.5x Kelly) and fixed fractional risk (maximum 1-2% portfolio risk per trade).
2. **Volatility-Adaptive Stop Loss (ATR)**:
   - Calculates dynamic stop-loss and take-profit targets based on Average True Range (ATR), eliminating arbitrary tight stops during high-volatility regimes.
3. **Value-at-Risk (VaR) & CVaR**:
   - Computes parametric and historical 95%/99% VaR to quantify maximum expected daily loss.
4. **Paper-Trading & Simulation Safety Switch**:
   - All execution plans, backtest orders, and automated trading logic are strictly locked to **Paper-Trading / Simulation Mode** by default to protect user capital.

## Execution Guidance
- Always output precise risk parameters: Entry Price, Invalidation (Stop Loss), Target 1/2, Risk-to-Reward Ratio (min 1:2), and Max Position Size ($ and % of portfolio).
