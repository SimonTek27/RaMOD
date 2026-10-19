# RaMOD Event Editor — UI Reference

**Status:** Target layout (not implemented)  
**Source:** FMOD Studio Event Editor (reference screenshot, 2026-10-19)  
**Aligned with:** [DESIGN.md](../DESIGN.md) · [ARCHITECTURE.md](../ARCHITECTURE.md) · ROADMAP Phase 2–3

This document freezes the **visual and structural target** for `ramod_editor`.
We do **not** copy proprietary assets or code — only the interaction model
that sound designers already know.

---

## 1. Overall chrome (3-pane + chrome)

```
┌─ Menu bar ─────────────────────────────────────────────── toolbar ─┐
│ File Edit Create View Window Scripts    [project] [?] [⚙] [─ □ ×] │
├─ Tabs ─────────────────────────────────────────────────────────────┤
│ [Events] [Banks] [Assets]                                          │
├──────────┬──────────────────────────────────────────┬──────────────┤
│          │  Editor tab bar  [Editor] [+]            │              │
│ BROWSER  │  TRANSPORT  ■ ▶ ❚❚  TIME 00:00.000       │  OVERVIEW    │
│ (left)   │                                          │  (right)     │
│          │  MAIN CANVAS                             │              │
│          │  (timeline / tracks / instruments)       │              │
│          │                                          │              │
├──────────┤──────────────────────────────────────────┤──────────────┤
│ Props    │  Inspector (track / instrument)          │              │
│ (bottom) │                                          │              │
├──────────┴──────────────────────────────────────────┴──────────────┤
│ Status: Live Update · Platform · Locale                            │
└────────────────────────────────────────────────────────────────────┘
```

Dark neutral theme (near-black panels, light-gray text). Primary accent on
transport Play (and selected tab).

---

## 2. Region map → RaMOD features

| Region (from screenshot) | FMOD behaviour | RaMOD target |
|--------------------------|----------------|--------------|
| **Menu** File/Edit/Create/View/Window/Scripts | Project I/O, create events/folders | Same; Scripts optional later |
| **Tabs** Events · Banks · Assets | Switch browser mode | Events = event tree; Banks = FSB5/native banks; Assets = samples/WAV/Ogg |
| **Left browser** + search | Tree of events/folders | `New Event` / `New Folder` / Flatten |
| **Left bottom** properties | Selected item metadata | Name, path, 3D flags, parameters |
| **Center transport** | Stop / Play / Pause, time, loop, view modes | Drive `ramod_runtime` preview |
| **Center canvas** | Tracks, instruments, automation | Event Designer timeline (Phase 3) |
| **Center bottom** | “Select a track or instrument…” | Inspector for instrument/DSP/params |
| **Right Overview** | Event overview / routing | Buses, parameters, 3D, snapshots summary |
| **Status bar** | Live Update Off · Platform · Locale | Live Update client state; target platform; locale bank |

---

## 3. Transport bar (detail)

From the screenshot, left → right:

1. **Stop** (square)
2. **Play** (triangle) — accent color
3. **Pause**
4. **Time** display `00:00.000` + state `STOPPED`
5. Seek / loop toggles
6. View mode icons (list / grid / mixer-style)

RaMOD: same controls bound to a local `RamodRuntime` instance embedded in the editor (no FMOD DLL).

---

## 4. Empty states (copy cues)

Keep familiar empty-state strings (or close equivalents):

- Left: *Select an item to view its properties*
- Center: *Select a track or instrument to view its properties*
- Canvas: blank until an event is opened

---

## 5. Implementation notes (egui / iced)

| Choice | Recommendation |
|--------|----------------|
| Toolkit | **egui** first (fast iteration, embeds in Bevy tools later) |
| Layout | Immediate-mode panels: left `SidePanel`, right `SidePanel`, bottom `TopBottomPanel`, center `CentralPanel` |
| Theme | Custom dark visuals matching screenshot contrast |
| Data | Browser reads project folder + loaded banks via `ramod` crate |
| Preview | `ramod_runtime` (Phase 1) when available; until then play decoded PCM/Ogg via a thin backend |

**Crate layout (future):**

```
ramod_editor/
  src/
    main.rs          # window + app loop
    ui/
      browser.rs     # Events / Banks / Assets
      transport.rs
      timeline.rs    # Phase 3
      overview.rs
      inspector.rs
    project.rs       # open/save project
    preview.rs       # runtime bridge
```

---

## 6. Out of scope for v1 UI

- Pixel-perfect FMOD cloning / proprietary icons
- Full multi-user collaboration
- Shipping FMOD `.bank` as *authoritative* format (FSB5 + native RaMOD banks first)
- Feature parity with every FMOD Studio panel on day one

---

## 7. Roadmap link

| Phase | UI work |
|-------|--------|
| **0** | None (CLI only) — current |
| **1** | Runtime preview backend (headless) |
| **2** | Waveform editor MVP (can share chrome) |
| **3** | This Event Editor layout + Live Update status |

---

*Reference screenshot stored by the project owner; this file is the textual contract for implementers.*
