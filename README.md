# Relieve 🕊️⚡

> **Speech-Driven Church Scripture & Media Presentation Desktop Application**  
> Powered by **Tauri v2**, **Rust**, **Whisper AI**, and **SvelteKit 5**.

---

## 🌟 Overview

**Relieve** is an intelligent, real-time desktop presentation system built specifically for houses of worship and live church services. By listening directly to spoken sermon audio through local high-performance speech-to-text (**whisper.cpp**), Relieve automatically detects spoken Scripture citations (such as *"John chapter 3 verse 16"*, *"Romans 8:28"*, or *"Psalm 23"*), fetches the verses instantly from an offline SQLite multi-translation Bible database, and projects beautifully styled slides onto external sanctuary projectors and screens with zero manual operator intervention.

---

## ✨ Key Features

- **🎙️ Real-time Audio Speech-to-Text**:
  - Powered by embedded local `whisper-rs` (`whisper.cpp`) running on Metal / Apple Silicon / x86_64 SIMD.
  - 100% offline & private — no audio leaves the local sanctuary computer.
  - Energy-gated dynamic range normalization and voice activity detection to prevent ambient noise triggers.

- **📖 Intelligent Scripture Citation Parser**:
  - Recognizes standard citations (*"Matthew 6:33"*), natural spoken variants (*"First Corinthians chapter thirteen verse four to eight"*), book aliases, and conversational speech patterns.
  - Automatic continuation detection (*"and verse nine"*, *"next verse"*).

- **⚡ Dual-Window Presentation Architecture**:
  - **Operator Console**: Interactive control center with live audio visualizer, transcript stream, instant passage search, manual override, typography controls, and monitor previews.
  - **Projector Output Window**: Clean, borderless secondary window designed for sanctuary beamers, LED walls, and confidence monitors.
  - Instant **Blackout** (`B`) and **Clear** (`C`) hotkey controls.

- **🎨 Dynamic Projector Engine & Typography**:
  - Auto-shrinking & auto-growing typography engine to ensure Scripture text always fits the screen without clipping.
  - High-contrast themes: Pure Black Darkroom, Deep Midnight, Cathedral Gold, Warm Parchment, and Modern Light.
  - Configurable citation placement (Top-Left, Top-Center, Bottom-Left, Bottom-Center, Bottom-Right).

- **📚 Multi-Translation Offline Bible Database**:
  - High-performance local SQLite database (`bible.db`) supporting KJV, ASV, BBE, and additional translations.
  - Sub-millisecond indexed verse queries.

---

## 🚀 Installation & Setup

### Requirements
- **macOS** 11+ (Intel or Apple Silicon)
- **Node.js** 18+ and `npm`
- **Rust** 1.75+ and `cargo`
- CMake (for compiling whisper.cpp)

### Development
```bash
# Clone repository
git clone https://github.com/terryekoe/Relieve.git
cd Relieve

# Install frontend dependencies
npm install

# Run application in development mode
npm run tauri dev
```

### Production Build
```bash
npm run tauri build
```
The compiled macOS `.dmg` and `.app` bundles will be located in:
`src-tauri/target/release/bundle/dmg/Relieve_1.0.0_x64.dmg`

---

## ⌨️ Keyboard Shortcuts

| Key | Action |
|---|---|
| `Space` | Toggle Audio Listening (Mic on/off) |
| `P` | Toggle Projector Window Display |
| `B` | Blackout Projector (Display pure black screen) |
| `C` | Clear Projector (Hide current passage) |
| `A` | Toggle Auto-Project Mode |
| `Esc` | Return Focus to Live Feed |

---

## 🛡️ Privacy & Permissions

Relieve runs entirely on your local machine:
- Audio streams are processed in memory and never transmitted over the internet.
- SQLite Scripture databases are bundled locally.
- Requires macOS Microphone permission (`NSMicrophoneUsageDescription`) for audio capture.

---

## 📄 License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
