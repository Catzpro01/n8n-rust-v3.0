---
name: crypto-onchain-radar
description: Crypto on-chain analytics, DeFi metrics (DefiLlama), DEX liquidity monitoring (DexScreener), funding rate arbitrage, and order book microstructure (CCXT).
---

# Crypto On-Chain Radar & Microstructure Engine

## Purpose
Tracks smart money flows, liquidity depth, protocol economics, and market microstructure across centralized (CEX) and decentralized (DEX) crypto markets using 100% free public APIs.

## Core Capabilities
1. **DeFi Protocol Metrics (DefiLlama API)**:
   - Total Value Locked (TVL) breakdown, protocol revenue, treasury reserves, and stablecoin supply flows.
2. **DEX Liquidity & Micro-Cap Radar (DexScreener & GeckoTerminal)**:
   - Real-time pool creation, liquidity-to-FDV ratios, buy/sell transaction volume, and token pair health.
3. **Market Microstructure & Derivatives (CCXT Public Endpoints)**:
   - Real-time Funding Rates, Long/Short liquidation heatmaps, Open Interest (OI) velocity, and L2/L3 Order Book Bid/Ask imbalance.
4. **Tokenomics & Unlock Dynamics**:
   - Vesting schedules, emission rates, and potential dump-pressure analysis.

## Execution Guidance
- Query DefiLlama and DexScreener REST endpoints directly for live on-chain truth.
- Always check liquidity depth before suggesting any token analysis to prevent illiquid slippage or honeypot traps.
