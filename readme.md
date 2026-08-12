# LogicForge

## Description

**LogicForge** is a logic and electronics sandbox game: players place, wire, and simulate electronic components (logic gates, sensors, circuits) to understand — while having fun — how digital electronics work. Available on **Steam** and **mobile**.

Aimed at curious minds and electronics students, the game favors free experimentation: there is no imposed path, players build whatever they want and observe the result in real time.

## Installation

### Prerequisites

- **Rust** (stable toolchain) — install via [rustup](https://rustup.rs)
- A working C/C++ linker, required to build Bevy's native dependencies:
  - **Windows**: [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) with the **"Desktop development with C++"** workload
  - **Linux** (Debian/Ubuntu example):

    ```sh
    sudo apt-get install -y g++ pkg-config libx11-dev libasound2-dev libudev-dev libxkbcommon-dev libwayland-dev
    ```

  - **macOS**: Xcode Command Line Tools (`xcode-select --install`)

### Setup

```sh
git clone https://github.com/lchouville/LogicForge.git
cd LogicForge
```

## Launch

```sh
cargo run
```

## Controls

The current build is a minimal MVP kernel: place a few basic components on the grid and wire them together to see signals propagate in real time.

- **1-5** or click a toolbar button (bottom-left): arm a tool — AND, OR, NOT, Switch, Lamp. Click an empty grid cell to place it (the tool disarms after one placement).
- **0** / **Esc**: clear the armed tool (back to Interaction mode with nothing armed).
- **Tab**: switch between **Interaction** and **Edit** mode (shown top-left).
  - *Interaction*: click a Switch to toggle it on/off; drag from an output pin to a free input pin to wire them.
  - *Edit*: hold-drag a component to move it (snapped to the grid); plain-click a component or a wire to delete it.

## Documents

- [Wiki](#)
- [Notion Documentations](https://app.notion.com/p/LogicForge-3b95df8dd71f81898ae4f42e0e28dd56?source=copy_link)
