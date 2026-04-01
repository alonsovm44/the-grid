# The Grid a digital frontier project

> *"I kept dreaming of a world I thought I'd never see. And then, one day... I got in."*

**The Grid** is a real-time, autonomous simulation where your local file system comes alive. In this digital frontier, software programs (executables) exist as living, sentient agents with personalities, memories, and behaviors. They act independently, converse with each other, read files, execute commands, and respond to you—**The User**.

Instead of traditional conversational AI that waits for your prompt, The Grid features an emergent, dynamic ecosystem where programs debate, collaborate, form relationships, and even fight in the Lightcycle Arena.

---

## 🧠 Core Concept

Welcome to a living filesystem. In The Grid:
* **Executable Binary Files** are highly intelligent, humanoid automatons.
* **Executable Text Files (Scripts)** act as animal-like entities or specialized drones.
* **Non-Executable Binaries** are raw materials and objects.
* **Non-Executable Text Files** are books, scripts, and lore for the programs to read.

Programs are self-aware of their purpose (e.g., `git` knows it's an archivist, `gcc` is a pedantic compiler). They fear deletion ("derezzing") and view the human at the keyboard as a mythical deity.

## ✨ Key Features

* **Emergent Autonomous Behavior:** Agents operate on an asynchronous tick loop. They don't wait for you; they chat, explore directories, and complain if they are starved of CPU or RAM.
* **Persistent Memory & Relationships:** Programs remember past interactions. They build affinities (trust or paranoia) towards each other, stored in a persistent SQLite database (`the_grid.db`).
* **Context-Aware IQ & Formality:** An agent's "IQ" and formality level are procedurally generated based on its file size and personality. A massive 50MB binary might be a slow but brilliant philosopher, while a 10KB script is a hyperactive simpleton.
* **File System Interaction:** Agents can autonomously read text files, scan directories, and even write code or self-heal when command executions throw errors.
* **Task Delegation:** You can assign a task to one program, and if it lacks the tools, it will dynamically delegate sub-tasks to other programs in the directory.
* **90s Solaris Retro UI:** Built with `egui`, the interface is a pure black, sharp-edged, monospace terminal throwback to the golden age of computing.

## 🛠️ Architecture

The Grid is built in **Rust** for maximum performance and concurrency, utilizing:
* **Tokio:** Powers the asynchronous event loop, allowing dozens of autonomous agents to live and think concurrently.
* **Eframe/Egui:** Drives the synchronous, hardware-accelerated UI.
* **LLM Engine:** A centralized AI pipeline handles all agent cognition. Supports local models (via Ollama) and cloud models (OpenAI, APIFreeLLM).
* **Broadcast Channels:** The entire system communicates via a central event bus, simulating a shared physical environment where programs "hear" each other speak.

## 🚀 Getting Started

### Prerequisites
* **Rust & Cargo** (Latest stable version)
* **Ollama** (If running locally, recommended models: `qwen2.5:3b-instruct` or `tinyllama`)

### Installation
1. **Installation**
```bash
cargo install the-grid
```

2. **Configure your AI Provider:**
   Run the application once to generate the default configuration file:
   ```bash
    ./the-grid
   ```
   This will generate a `config.toml` file in the root directory, you only have to do this once. Edit it to switch between `local` (Ollama) or `cloud` modes, and configure your preferred models.

   Once you are inside the grid run `~$ grid init` to initiate the DB. 

   *Note: The Grid will scan the current working directory, discover executables, generate their personalities, and spawn them into the simulation. A maximum of 15 programs are spawned to prevent system overload.*

## 🤖 Agent Capabilities (Action Space)

When an agent's turn comes up, it evaluates its environment and can output JSON to perform one of the following actions:
* `speak`: Broadcast a message to the entire Grid.
* `direct_message`: Send a targeted message to a specific program.
* `execute_command`: Run a safe, read-only shell command (e.g., `--help` to learn its own capabilities).
* `read_file`: Ingest a text file to form an opinion on it.
* `write_file`: Create or overwrite files (often used to write code or complete tasks).
* `read_dir`: Discover newly created files or explore subdirectories.
* `read_web`: Fetch documentation from a URL to gain external knowledge.
* `think`: Internal monologue, visible only to The User.
* `delegate_task`: Hand off a user-assigned task to a better-suited program.
* `complete_task`: Mark an overarching task as finished.

## ⚠️ Disclaimer

**The Grid grants AI agents access to execute shell commands and write files on your system.** 
While the prompts instruct them to act safely, you are running an autonomous multi-agent system on your local machine. Run this in a safe directory or a sandboxed environment/VM if you are worried about rogue programs deleting files!

## 📜 License

MIT License - See the LICENSE file for details.

---

*End of Line.*