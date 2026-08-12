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

This builds the project and opens the game window. The first build compiles all of Bevy's dependencies and can take several minutes; subsequent builds are much faster thanks to incremental compilation.

## Documents

- [Wiki](#)
- [Notion Documentations](https://app.notion.com/p/LogicForge-3b95df8dd71f81898ae4f42e0e28dd56?source=copy_link)
