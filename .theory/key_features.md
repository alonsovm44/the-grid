# The Grid: Killer Features to Prioritize
Based on the theory docs and what you already have working, here's my ranking — ordered by impact-to-effort ratio and how novel they are compared to what exists in the industry.

## Tier 1: Build These First (Transcendental)
1. Semantic Piping — The Core Differentiator
Why: Nothing else in the AI agent space does this. LangChain chains are rigid DAGs. CrewAI is task-based delegation. Nobody has a live, composable shell where you pipe mental state between agents in real-time.

### Minimum viable version:

`~$ think "analyze this code" | implement "fix the bug"`
Two agents, one pipe, context flows from left to right. That's it. No ?? gates, no & parallel, no variables — just the basic | operator working with two agents and proving the concept.

Why it's transcendental: This is the equivalent of Unix pipes in 1973. Simple mechanism, infinite composability. Once this works, everything else (parallel pipes, conditionals, scripting) is iteration.

2. Inference as a Schedulable Resource

Why: You already have the iq_level field on agents and different AI providers. The leap is tiny: make the system dynamically allocate model quality based on agent priority and system load.

### Minimum viable version:

`Background agents → local/cheap model (e.g., Ollama/TinyLlama)`
`Foreground agent (user-invoked via GridShell) → GPT-4-class`
`System under load → demote all background agents to cheaper inference`
Why it's transcendental: This is the "CPU scheduler for intelligence" idea from Core.md. Nobody is doing this. Every multi-agent framework gives every agent the same model. You'd be the first to treat inference tokens like CPU cycles and schedule them.

Tier 2: Build These Second (High Value)
3. The Semantic Bus (Already 80% Done)
You already have broadcast_channel in Rust. What's missing:

Typed events (not just strings) — agents react to semantic content, not raw text
Event filtering — agents subscribe to topics they care about, ignore the rest
This is infrastructure, not flashy — but it's the nervous system that makes piping and scheduling possible.

4. Agent Blueprints + summon Command
Why: The .ag files are a great idea. The killer UX is:

~$ summon security-auditor
A new agent materializes from a blueprint, inherits the right model tier, and starts working. This is what makes The Grid feel like an OS instead of a chat interface. Combined with semantic piping, you get:

~$ summon code-reviewer | summon security-auditor | summarize
Tier 3: Build Later (Cool but Not Essential Yet)
5. SKFS Spatial Organization
The self-organizing 3D filesystem is visually spectacular and conceptually beautiful, but it's not what makes the system useful. Start with simple tag-based file metadata and find_near queries. Add physics simulation and 3D visualization after the core agent pipeline is solid.

6. Spatial-SQL
Full query language is over-engineering at this stage. A handful of built-in commands (find_by_tag, find_near, find_related) covers 90% of use cases.

7. GridShell Scripting / Variables / Conditionals
This is language design work that can wait until the basic pipe operator is proven.

The One-Sentence Strategy
Get one semantic pipe between two agents with dynamic model allocation working end-to-end. Everything else is expansion.

If you demo think "analyze X" | implement "fix Y" where the first agent gets GPT-4 and the second gets a local model based on system load — that's a thing nobody has ever seen before. That's your proof of concept, your pitch, and your north star.