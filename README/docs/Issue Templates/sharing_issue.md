---
name: Sharing / Tunnel Issue
about: Report issues with database sharing or remote connections
title: '[SHARE] '
labels: sharing, tunnel
triage: true
---

## Sharing Mode

- [ ] LAN Sharing (same network)
- [ ] Remote Sharing (reverse tunnel)

## Host Environment

- **OS:** [e.g., macOS 14.5]
- **Bennett Studio Version:** [e.g., v0.3.2]
- **Network:** [e.g., Home WiFi, Corporate VPN, University]
- **Firewall:** [e.g., None, Corporate, Custom]

## Guest Environment

- **OS:** [e.g., Windows 11]
- **Browser/App:** [e.g., Chrome 125, Desktop App v0.3.2]
- **Network:** [e.g., Different city, same office]

## Steps to Reproduce

1. Host creates share for [database type]
2. Guest opens URL: `https://share.bennett.studio/db/...`
3. [Describe what happens]

## Expected Behavior

[What should happen]

## Actual Behavior

[What actually happens]

## Error Messages

```
[Paste any error messages from host or guest]
```

## Diagnostics

### Host Side

Run this in your terminal and paste output:

```bash
bennett-cli diagnose --sharing
```

### Guest Side

Open browser DevTools (F12) → Network tab → reproduce issue → export HAR or screenshot.

## Additional Context

- Does LAN sharing work but remote doesn't?
- Does it work for some databases but not others?
- Any corporate proxies or VPNs involved?
