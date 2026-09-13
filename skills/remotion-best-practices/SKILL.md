---
name: remotion-best-practices
description: >-
  Best practices for creating programmatic videos with Remotion (React-based
  video framework). Covers composition, performance, rendering, and deployment.
---

# Remotion Best Practices

## Core Concepts
Remotion renders React components to video frames. Each frame is a snapshot of your React tree at a specific timestamp.

## Project Structure
```
src/
├── Root.tsx           # Register all compositions
├── compositions/
│   ├── MyVideo.tsx    # Main video component
│   └── scenes/        # Individual scenes
├── components/        # Reusable components
└── assets/            # Images, audio, fonts
```

## Composition Setup
```tsx
// Root.tsx
export const RemotionRoot = () => (
  <Folder name="My Project">
    <Composition
      id="MyVideo"
      component={MyVideo}
      durationInFrames={300}   // 10 seconds at 30fps
      fps={30}
      width={1920}
      height={1080}
      defaultProps={{
        title: "Hello World"
      }}
    />
  </Folder>
);
```

## Animation Best Practices

### Use `interpolate` for Smooth Motion
```tsx
import { interpolate, useCurrentFrame } from "remotion";

const frame = useCurrentFrame();
const opacity = interpolate(frame, [0, 30], [0, 1], {
  extrapolateLeft: "clamp",
  extrapolateRight: "clamp"
});
```

### Avoid Expensive Computations in Render
```tsx
// BAD: Computed every frame
const expensiveData = heavyComputation(frame);

// GOOD: Memoize
const expensiveData = useMemo(() => heavyComputation(), []);
```

## Performance Rules
- Preload all assets with `staticFile()` and `prefetch()`
- Use `delayRender()` / `continueRender()` for async assets
- Avoid network requests during render
- Keep component tree depth manageable

## Rendering
```bash
# Preview
npx remotion preview

# Render to file
npx remotion render MyVideo out/video.mp4

# Render with custom config
npx remotion render MyVideo out/video.mp4 --codec h264 --quality 90
```

## Resource Note
⚠️ Rendering is CPU/GPU intensive — allocate sufficient resources
⚠️ Long videos can take significant time — estimate: 1 min video ≈ 5-15 min render
