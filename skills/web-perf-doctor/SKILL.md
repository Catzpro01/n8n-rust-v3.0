---
name: web-perf-doctor
description: Web performance and Core Web Vitals tuner. Diagnoses LCP, INP, CLS layout shifts, Chrome DevTools memory leaks, and excessive JS bundle sizes.
---

# Web Performance & Core Web Vitals Doctor

## Purpose
Diagnoses sluggish web applications, slow loading times, and layout stutters using Google's Core Web Vitals standards (LCP, INP, CLS).

## Core Capabilities:
1. **LCP (Largest Contentful Paint) Optimization**:
   - Priority hints (etchpriority='high'), modern image formats (AVIF/WebP), and server-side preloading.
2. **INP (Interaction to Next Paint) Debugging**:
   - Yielding main thread execution via scheduler.yield() / 
equestIdleCallback, breaking long tasks.
3. **CLS (Cumulative Layout Shift) Prevention**:
   - Dimension reserving, font display swapping (ont-display: optional), and dynamic injection containment.
