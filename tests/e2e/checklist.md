# E2E Checklist — CodeAtlas v1 MVP

## Pre-flight

- [ ] `cargo build --release` compila sin errores
- [ ] `npm run build` compila sin errores
- [ ] `cargo test` pasa (32 tests)
- [ ] `npm run test` pasa (7+ tests)
- [ ] `cargo clippy -- -D warnings` limpio
- [ ] `npm run lint` limpio

## Functional E2E Tests

### 1. Project Opening

- [ ] User opens Tauri app → empty state shown
- [ ] User clicks "Open project" → native folder dialog opens
- [ ] User selects a folder with TS/JS/Rust files → scan starts
- [ ] Scan progress indicator shown during scan
- [ ] Scan completes → graph appears (or error if empty)

### 2. Graph Visualization

- [ ] Graph renders with nodes coloured by type
- [ ] Zoom in/out with mouse wheel works
- [ ] Pan by dragging works
- [ ] Minimap shows overview
- [ ] Controls (zoom fit, zoom in/out) work
- [ ] Nodes can be clicked → detail panel updates

### 3. Explorer Sync

- [ ] Sidebar shows file tree from scan
- [ ] Clicking file in sidebar selects node in graph
- [ ] Node selection in graph highlights file in sidebar

### 4. Detail Panel

- [ ] Selecting node shows file metadata (path, lines, symbols)
- [ ] Loading state shown while fetching details
- [ ] Error state shown if fetch fails
- [ ] Symbol list shows exported symbols

### 5. AI Explanation

- [ ] With API key configured: selecting node triggers AI explanation
- [ ] Explanation renders as markdown (headings, lists, code)
- [ ] Without API key: error state with guidance
- [ ] Loading spinner shown during AI request

### 6. Chat

- [ ] Chat input accepts text
- [ ] Sending message shows user message
- [ ] AI response appears (if API key set)
- [ ] AI error shows user-friendly message
- [ ] Suggestions appear in chat input

### 7. Error Handling

- [ ] Non-existent project path → PATH_NOT_FOUND error shown
- [ ] Invalid API key → INVALID_KEY error shown
- [ ] Rate limit → RATE_LIMITED message with retry guidance
- [ ] Network unreachable → UNREACHABLE message

### 8. Tab Navigation

- [ ] Detail tab shows file details
- [ ] AI tab shows explanation
- [ ] Chat tab shows chat panel
- [ ] Tab state persists when switching

## Performance Targets (informative)

- [ ] Scan of 100-file project < 5s
- [ ] Graph render after scan < 2s
- [ ] Node click → detail panel update < 500ms
- [ ] AI response < 10s (after network latency)

## Manual QA Sign-off

- [ ] App starts without crash
- [ ] No console errors in DevTools
- [ ] All panels render in all states (loading/empty/error/ready)
- [ ] Window resize handled gracefully
- [ ] Dark/light theme consistent (currently dark)

---

_Documento E2E — PR6 Hardening v1-mvp-core_
