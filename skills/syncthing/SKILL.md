---
name: syncthing
description: >-
  Decentralized P2P file synchronization without third-party servers. 
  MPL-2.0 licensed, actively maintained. Files stay on your machines only.
---

# Syncthing — P2P File Sync

## Architecture
- **No central server** — files sync directly between your devices
- **Encrypted in transit** — TLS with certificate pinning between peers
- **No third-party access** — only you and your trusted devices
- **Conflict resolution** — automatic with manual override option

## Setup

### Install
```bash
# Linux
apt install syncthing

# macOS  
brew install syncthing

# Docker
docker run -d -p 8384:8384 -p 22000:22000 syncthing/syncthing
```

### First-Time Config
1. Open Web UI: `http://localhost:8384`
2. **IMMEDIATELY** set a Web UI password (Settings → General)
3. Add remote devices using their Device ID
4. Create shared folders and select which devices sync

### Agent OS Integration
```bash
# Check sync status
curl http://127.0.0.1:8384/rest/db/status?folder=default   -H "X-API-Key: $(cat ~/.config/syncthing/api-key)"

# Trigger sync
curl -X POST http://127.0.0.1:8384/rest/db/scan?folder=default   -H "X-API-Key: $(cat ~/.config/syncthing/api-key)"
```

## Security Configuration
```xml
<!-- ~/.config/syncthing/config.xml - key settings -->
<gui>
  <address>127.0.0.1:8384</address>  <!-- localhost only -->
  <user>admin</user>
  <password>[hashed]</password>
  <useTLS>true</useTLS>
</gui>
```

## Security Rules
✅ Always bind Web GUI to `127.0.0.1`, never `0.0.0.0`
✅ Always set Web UI password before first use
✅ Use `ignorePerms` for cross-OS syncing
⚠️ Don't sync folders containing secrets/credentials without reviewing .stignore
⚠️ Any device added to sync has full folder access — trust carefully
