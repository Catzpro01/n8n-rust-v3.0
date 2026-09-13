---
name: excalidraw
description: >-
  Virtual collaborative whiteboard for creating diagrams, sketches, and 
  wireframes. MIT licensed, end-to-end encrypted for collaboration.
  Best for system design diagrams and architecture sketches.
---

# Excalidraw — Collaborative Diagramming

## When to Use Excalidraw
- System architecture diagrams
- Database schema sketches
- UI wireframes (low fidelity)
- Flow charts and sequence diagrams
- Brainstorming boards

## Diagram Types

### System Architecture
Use rectangles for services, cylinders for databases, clouds for external services, arrows for data flow:
```
[Client Browser] → [Load Balancer] → [API Gateway]
                                           ↓
                                    [Auth Service] [Product Service]
                                           ↓              ↓
                                    [Users DB]      [Products DB]
```

### Sequence Diagrams
Use vertical lines for participants, horizontal arrows for messages:
```
Browser → Server: POST /login {credentials}
Server → DB: SELECT user WHERE email=?
DB → Server: {user_record}
Server → Browser: 200 {jwt_token}
```

## Agent Integration
When the user asks for a diagram, the agent should:
1. Describe the diagram structure in text first
2. Offer to generate Excalidraw JSON format
3. Provide the URL: `https://excalidraw.com/#json=...`

## Excalidraw JSON Format
```json
{
  "type": "excalidraw",
  "version": 2,
  "elements": [
    {
      "type": "rectangle",
      "x": 100, "y": 100,
      "width": 200, "height": 80,
      "text": "API Server"
    }
  ]
}
```

## Privacy Notes
- Online version: drawings encrypted E2E — link sharing is safe
- Self-hostable via `docker run excalidraw/excalidraw`
- No server stores drawing content (only you have the decryption key)
- For confidential architecture: always use self-hosted instance
