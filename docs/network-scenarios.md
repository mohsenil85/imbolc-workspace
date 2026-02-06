# Network Deployment Scenarios

> **Status: Planned Feature - Not Yet Implemented**
>
> This document describes the planned network/collaboration feature.
> The imbolc-net crate exists but is not yet functional.

This document describes common deployment scenarios for Imbolc, from solo laptop use to multi-node jam sessions.

## Core Principles

1. **One render server** — SuperCollider and all audio processing runs on exactly one machine
2. **Audio never traverses the network** — physical cables only (audio interface to server)
3. **LAN only** — we assume a local, low-latency network (ideally dedicated or well-managed)
4. **MIDI/control data over network** — actions, state sync, and metering travel over TCP
5. **Server is authoritative** — all state lives on the server; clients hold mirrors

## Scenarios

### 1. Solo Local (Laptop Only)

The default mode. No network, no imbolc-net.

```
┌─────────────────────────────────────┐
│           Your Laptop               │
│                                     │
│  imbolc (TUI)                       │
│     ↓                               │
│  LocalDispatcher (imbolc-core)      │
│     ↓                               │
│  SuperCollider                      │
│     ↓                               │
│  Built-in speakers / headphones     │
└─────────────────────────────────────┘
```

**Launch:** `imbolc`

**Use case:** Composing, arranging, sound design — anything where you're working alone.

---

### 2. Solo Local + MIDI Controller

Same as above, but with a USB MIDI keyboard or controller attached.

```
┌─────────────────────────────────────┐
│           Your Laptop               │
│                                     │
│  USB MIDI keyboard ──┐              │
│                      ↓              │
│  imbolc (TUI) ← MIDI events         │
│     ↓                               │
│  LocalDispatcher                    │
│     ↓                               │
│  SuperCollider                      │
│     ↓                               │
│  Audio interface / headphones       │
└─────────────────────────────────────┘
```

**Launch:** `imbolc`

MIDI is handled locally by the OS. Imbolc polls for MIDI events and converts them to actions (note triggers, CC mappings). No network involved.

**Use case:** Playing live, recording MIDI, tweaking parameters with hardware knobs.

---

### 3. Solo Local + Audio Input (Guitar/Mic)

Recording or processing external audio through the server.

```
┌──────────────────────────────────────────────┐
│              Your Laptop                     │
│                                              │
│  Guitar/Mic ─→ Audio Interface ─→ SC input   │
│                                              │
│  imbolc (TUI)                                │
│     ↓                                        │
│  LocalDispatcher                             │
│     ↓                                        │
│  SuperCollider (AudioIn source)              │
│     ↓                                        │
│  Audio Interface ─→ monitors/headphones      │
└──────────────────────────────────────────────┘
```

**Launch:** `imbolc`

The guitar signal goes into your audio interface, SuperCollider reads it via `AudioIn`, processes it through your effect chain, and outputs to monitors. All local.

**Use case:** Guitar/vocal processing, sampling, live looping.

---

### 4. Two Laptops (Casual Jam)

One laptop runs the server and acts as the privileged node. The second joins as a client.

```
┌─────────────────────────────┐          ┌─────────────────────────────┐
│  Laptop A (Server + Priv)   │          │  Laptop B (Client)          │
│                             │          │                             │
│  Guitar ─→ Audio Interface  │          │  MIDI keyboard (USB)        │
│                  ↓          │   LAN    │         ↓                   │
│  imbolc --server --tui      │ ←──────→ │  imbolc --connect <A's IP>  │
│  LocalDispatcher            │  Actions │  RemoteDispatcher           │
│  SuperCollider              │  State   │  (no SC, no audio)          │
│         ↓                   │          │                             │
│  Monitors / PA              │          │                             │
└─────────────────────────────┘          └─────────────────────────────┘
              ↑
       All audio output
```

**Launch:**
- Laptop A: `imbolc --server --tui`
- Laptop B: `imbolc --connect 192.168.1.100:9999`

**Audio routing:** Laptop A's audio interface handles all I/O. Laptop B has no audio — it's purely a control surface. If Laptop B's player has a guitar, they cable it to Laptop A's interface.

**MIDI:** Each laptop handles its own USB MIDI locally. MIDI events become actions, which travel over the network to the server.

**Privilege:** Laptop A is privileged (transport, save/load). Laptop B can only modify instruments it owns.

**Use case:** Jamming with a friend, each on their own screen.

---

### 5. Pro Setup (Dedicated Server)

A dedicated machine (headless laptop, Mac Mini, or similar) runs the server. All players are LAN clients, including the "host" who has privileged status.

```
                           ┌──────────────────────────────────┐
                           │  Dedicated Server (headless)    │
                           │                                 │
  All audio inputs ──────→ │  Audio Interface (multi-in)     │
  (guitars, mics,          │         ↓                       │
   synths, etc.)           │  imbolc --server                │
                           │  LocalDispatcher                │
                           │  SuperCollider                  │
                           │         ↓                       │
                           │  Audio Interface ─→ PA/monitors │
                           └────────────┬────────────────────┘
                                        │
                          LAN (dedicated switch recommended)
                                        │
         ┌──────────────────────────────┼──────────────────────────────┐
         │                              │                              │
         ↓                              ↓                              ↓
┌─────────────────────┐    ┌─────────────────────┐    ┌─────────────────────┐
│  Client A (Priv)    │    │  Client B           │    │  Client C           │
│  imbolc --connect   │    │  imbolc --connect   │    │  imbolc --connect   │
│  MIDI keyboard      │    │  MIDI controller    │    │  Just laptop        │
│  (transport ctrl)   │    │  (owns synth 2)     │    │  (owns drums)       │
└─────────────────────┘    └─────────────────────┘    └─────────────────────┘
```

**Launch:**
- Server: `imbolc --server` (headless, or with `--tui` if you want to monitor)
- Clients: `imbolc --connect <server-ip>:9999`

**Audio routing:**
- All instruments (guitars, mics, hardware synths) cable directly to the server's audio interface
- SuperCollider on the server does all mixing and processing
- Main output goes to PA/monitors
- Headphone mixes can be routed via SuperCollider's cue bus system (requires multi-output interface)

**Network:**
- Dedicated LAN switch recommended (avoid sharing with general internet traffic)
- Wired ethernet preferred; WiFi works but adds latency variance
- Typical action latency: 1-5ms on a clean LAN

**Privilege:**
- One client is designated privileged (can control transport, save/load project)
- Other clients can only modify instruments they own
- Ownership assigned on connect (requested by client, granted by server)

**MIDI flow:**
- Each client's USB MIDI devices stay local to that client
- MIDI events → Action → Network → Server dispatch → SuperCollider
- For latency-sensitive control (knobs), this adds ~5-15ms over local
- For most use cases (playing notes, tweaking effects), this is imperceptible

---

## What Travels Over the Network

| Data | Direction | Notes |
|------|-----------|-------|
| Actions | Client → Server | User intent (play note, change param, etc.) |
| State | Server → Clients | Full state snapshot after each action |
| Metering | Server → Clients | Playhead, BPM, peak levels (~30Hz) |
| Ownership | Both | Instrument ownership table |

**NOT sent over network:**
- Raw audio (always via cables)
- Raw MIDI (handled locally, converted to actions)
- Undo history (per-client)
- Clipboard (per-client)
- UI navigation state (per-client)

---

## Hardware Recommendations

### Audio Interface (Server)

For multi-player setups, the server needs enough inputs:

| Players | Minimum Inputs | Example |
|---------|----------------|---------|
| 2 | 4 inputs | Focusrite Scarlett 4i4 |
| 3-4 | 8 inputs | MOTU 8A, Focusrite 18i8 |
| 5+ | 16+ inputs | MOTU 16A, RME Fireface |

Outputs: At minimum, stereo main. For individual headphone mixes, you need additional output pairs.

### Network

| Setup | Recommendation |
|-------|----------------|
| 2 laptops | Direct ethernet cable, or shared home WiFi |
| 3+ nodes | Dedicated gigabit switch, wired connections |
| Pro/live | Dedicated VLAN or isolated network segment |

Avoid: Congested WiFi, networks with high latency/jitter, anything traversing the internet.

---

## Latency Considerations

### Local (no network)
- MIDI → action: <1ms
- Action → audio: ~3-10ms (depends on SC buffer size)

### Networked
- MIDI → action: <1ms (local)
- Action → server → dispatch: +1-10ms (LAN)
- State sync back: +1-10ms (LAN)

For **note triggering**, this is usually fine — you're playing to what you hear, not to visual feedback.

For **continuous control** (turning a knob), the extra latency is noticeable if you're watching the UI, but the audio response is what matters, and that's only delayed by the first hop (action → server).

If latency becomes an issue for continuous control, future options include:
- UDP fast path for `RecordValue` actions
- Direct OSC passthrough to SuperCollider (bypassing action/dispatch)

---

## Quick Reference

| Mode | Command | Audio | SuperCollider | Use Case |
|------|---------|-------|---------------|----------|
| Local | `imbolc` | Local | Local | Solo work |
| Server (headless) | `imbolc --server` | Local | Local | Dedicated render box |
| Server (with UI) | `imbolc --server --tui` | Local | Local | Host is also playing |
| Client | `imbolc --connect <ip>` | None | None | Remote player |
