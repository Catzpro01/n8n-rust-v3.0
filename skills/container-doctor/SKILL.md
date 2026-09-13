---
name: container-doctor
description: Docker & container lifecycle troubleshooter. Diagnoses OOMKilled errors, port/networking conflicts, permission issues, and slims Docker images from 1GB to 50MB.
---

# Container Doctor & Docker Optimizer

## Purpose
Fixes daily container headaches: crashes, network unreachable bugs, volume permission mismatches, and bloated multi-gigabyte container images.

## Core Diagnostics:
1. **Crash & OOMKilled Triage**:
   - Inspects exit codes (137 = Out of Memory, 1 = Application throw, 127 = Missing binary/path).
2. **Multi-Stage Build Slimming**:
   - Strips build dependencies, leverages Alpine/Distroless bases, and organizes layer caching for instant builds.
3. **Host-to-Container Networking**:
   - Resolves localhost vs host.docker.internal vs bridge network resolution bugs.
