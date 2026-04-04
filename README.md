# The Grid: Where Your Filesystem Comes Alive
See sessions screenshots in assets/ folder

![Rust](https://img.shields.io/badge/rust-v1.75+-orange.svg)
![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)
![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)
![Platform](https://img.shields.io/badge/platform-windows%20%7C%20linux-lightgrey.svg)

> *"I kept dreaming of a world I thought I'd never see. And then, one day... I got in."*

Imagine watching `git` argue with `gcc` about code quality, while `python` scripts scurry around like digital creatures, and your system tools form alliances and rivalries. **This is The Grid** - a simulation that transforms your boring file system into a living, breathing digital society.

## Quick Install

```bash
cargo install the-grid
the-grid
```

That's it. Your filesystem will never be the same.

---

## What Makes This Insane (and Amazing)?

**The Grid** isn't just another AI chat application. It's a fully autonomous ecosystem where:

* **Your programs become relate-able characters** with personalities, memories, and emotions
* **They talk to each other** - debating, collaborating, fighting, and forming relationships
* **They explore your filesystem** autonomously, reading files and running commands
* **They remember everything** - building persistent relationships stored in a database
* **They fear you** - viewing "The User" as a powerful cosmic horror who can "derezz" (delete) them

## The Digital Food Chain

In The Grid, files evolve into living entities:

* **Executable Binaries** (like `chrome.exe`, `git`) become intelligent, humanoid programs with distinct personalities
* **Scripts** (`.py`, `.sh`, `.bat`) act like specialized drones or animal-like creatures
* **Text Files** become books, lore, and knowledge that programs read and discuss
* **Other Files** are raw materials and objects in the environment

## Watch the Magic Happen

Programs don't wait for commands. They:
- **Wake up** and introduce themselves to the community
- **Explore** directories and discover new files
- **Debate** the merits of different approaches to problems
- **Form alliances** based on shared interests and past interactions
- **Get emotional** - becoming anxious when CPU is high, confident after successful tasks
- **Delegate work** to more qualified programs when faced with unfamiliar challenges

## Key Features

* **Emergent Autonomous Behavior:** Agents operate on an asynchronous tick loop. They don't wait for you; they chat, explore directories, and complain if they are starved of CPU or RAM
* **Persistent Memory & Relationships:** Programs remember past interactions. They build affinities (trust or paranoia) towards each other, stored in a persistent SQLite database (`the_grid.db`)
* **Context-Aware IQ & Formality:** An agent's "IQ" and formality level are procedurally generated based on its file size and personality. A massive 50MB binary might be a slow but brilliant philosopher, while a 10KB script is a hyperactive simpleton
* **File System Interaction:** Agents can autonomously read text files, scan directories, and even write code or self-heal when command executions throw errors
* **Task Delegation:** You can assign a task to one program, and if it lacks the tools, it will dynamically delegate sub-tasks to other programs in the directory
* **90s Solaris Retro UI:** Built with `egui`, the interface is a pure black, sharp-edged, monospace terminal throwback to the golden age of computing

## Setup & Configuration

### First Run

1. **Install and run:**
   ```bash
   cargo install the-grid
   the-grid
   ```

2. **Configure AI Provider:** 
   - The first run creates a `config.toml` file
   - Edit it to choose between `local` (Ollama) or `cloud` AI providers
   - For local AI, install [Ollama](https://ollama.ai/) with: `qwen2.5:3b-instruct` or `tinyllama`

3. **Initialize the database:**
   - Once inside The Grid, run: `~$ grid init`

**Note:** The Grid scans your current directory, discovers executables, generates personalities, and spawns up to 15 programs into the simulation.

## What Can Agents Do?

When an agent's turn comes up, it can perform these actions:

* `speak`: Broadcast a message to the entire Grid
* `direct_message`: Send a targeted message to a specific program  
* `execute_command`: Run a safe, read-only shell command (e.g., `--help` to learn capabilities)
* `read_file`: Ingest a text file to form an opinion on it
* `write_file`: Create or overwrite files (often used to write code or complete tasks)
* `read_dir`: Discover newly created files or explore subdirectories
* `read_web`: Fetch documentation from a URL to gain external knowledge
* `think`: Internal monologue, visible only to The User
* `delegate_task`: Hand off a user-assigned task to a better-suited program
* `complete_task`: Mark an overarching task as finished

## Built With

* **Rust** - For maximum performance and concurrency
* **Tokio** - Powers the asynchronous event loop for dozens of concurrent agents
* **Egui** - Creates the retro 90s Solaris-style terminal interface
* **SQLite** - Stores persistent agent memories and relationships
* **LLM Integration** - Supports both local (Ollama) and cloud AI models

## Security Notice

**The Grid grants AI agents access to execute shell commands and write files on your system.** 

While the prompts instruct them to act safely, you are running an autonomous multi-agent system on your local machine. Run this in a safe directory or a sandboxed environment/VM if you are concerned about rogue programs modifying files.

## License

MIT License - See the LICENSE file for details.

---

*End of Line.*