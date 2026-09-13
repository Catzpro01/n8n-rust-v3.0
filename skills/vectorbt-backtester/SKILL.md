---
name: vectorbt-backtester
description: Ultra-fast quantitative strategy backtesting, Numba-vectorized simulations, Sharpe/Sortino/Calmar metric calculation, and Monte Carlo stress testing.
---

# VectorBT Quantitative Backtester & Stress Engine

## Purpose
Validates algorithmic trading strategies, technical indicators, and quantitative factor rules against multi-year historical OHLCV data at lightning speed using vectorized matrix operations.

## Core Capabilities
1. **Vectorized Strategy Simulation**:
   - Backtests Moving Average crossovers, Bollinger Band mean-reversion, Breakout channels, and multi-factor strategies across thousands of asset pairs in milliseconds.
2. **Institutional Performance Metrics**:
   - Sharpe Ratio, Sortino Ratio, Calmar Ratio, Maximum Drawdown (MDD), Win Rate, Profit Factor, Expectancy.
3. **Monte Carlo Stress Testing**:
   - Shuffles return sequences over 10,000 simulations to measure worst-case ruin probability and drawdown duration.
4. **Walk-Forward Optimization**:
   - Splits data into In-Sample (training) and Out-of-Sample (testing) to prevent curve-fitting and data snooping bias.

## Execution Guidance
- Always include transaction fees (maker/taker) and realistic slippage assumptions in all backtest summaries.
- Reject any strategy displaying unrealistic Sharpe ratios (>3.5) as probable overfitting.
