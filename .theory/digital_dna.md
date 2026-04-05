# Digital DNA: A Biological Framework for Cognitive Identity

## The Insight

Biology solved the problem of encoding complex living systems billions of years ago.
DNA is a portable, copyable, mutable text format that encodes the complete potential
of an organism. The FASTA format made this machine-readable:

```fasta
>think_agent cognitive_sequence
TAGCGCATCGATCGATCGAC...
```

A few thousand base pairs define whether something becomes a bacterium or a blue whale.

The Grid faces the same problem: how do you encode the complete identity of a cognitive
entity in a portable, shareable, evolvable format? Our answer is two complementary
file formats that mirror the biological distinction between **genotype** and **phenotype**.

---

## Layer 1: `.ag` — The Genome (Genotype)

The `.ag` file is the agent's DNA. It is **static, declarative, and universal**. Every
instance spawned from the same `.ag` file starts life identically — same personality,
same IQ, same permissions, same blank memory.

```ag
# think — Cognitive Blueprint for The Grid

agent think
  personality  "Analytical, methodical, detail-oriented"
  iq           0.8
  spec         "Code analysis and pattern recognition"

permit
  read_file think delegate_task read_web

prompt ---
You are an analysis specialist. Break down complex problems
into manageable components.
---

tools
  read_file        priority:high
  execute_command   safety:read_only
  think            priority:high

evolve
  rate     0.1
  feedback on
  tracking on
  xp       1.0
```

The `.ag` file answers: **"What is this agent designed to be?"**

Like DNA, it defines potential, not destiny. Two agents born from `think.ag` will
diverge the moment they start accumulating different experiences, relationships, and
memories.

### Biological Parallel

| DNA Concept | `.ag` Equivalent |
|---|---|
| Base pairs (A, T, C, G) | Fields (personality, iq, spec, permit) |
| Gene | A single field declaration |
| Chromosome | A section block (agent, permit, tools, evolve) |
| Genome | The complete `.ag` file |
| Gene expression | Runtime interpretation of prompt template with {variables} |
| Regulatory genes | The `evolve` block (controls how other genes change) |

---

## Layer 2: `.giso` — The Organism Image (Phenotype)

The `.giso` (Grid ISO) file is a **complete snapshot of a living agent** — not what it
was born as, but what it has *become*. It captures the full runtime state: memories,
relationships, emotional state, reputation, spatial position, and how far the agent
has drifted from its original genome.

This is the biological equivalent of capturing not just an organism's DNA, but its
entire body — every neuron, every scar, every learned behavior.

### Structure

```
┌──────────────────────────────────────────────┐
│            GRID ISO IMAGE (.giso)             │
│         Complete Agent State Snapshot          │
├──────────────────────────────────────────────┤
│                                              │
│  [genome]                                    │
│    source: sys/think.ag                      │
│    born: 2026-04-05T14:00:00Z                │
│    generation: 1                             │
│                                              │
│  [identity]                                  │
│    name: think                               │
│    personality: "Analytical, methodical,     │
│                  detail-oriented"             │
│    iq: 0.83            # drifted from 0.8    │
│    mood: focused                             │
│    xp: 2847                                  │
│    reputation: 0.87                          │
│    age: 14d 6h 23m                           │
│                                              │
│  [spatial]                                   │
│    position: [12.4, 8.1, -3.7]              │
│    home_region: "source_analysis"            │
│                                              │
│  [memory]                                    │
│    long_term: [                              │
│      "Collaborated with implement on auth    │
│       module refactor. Relationship strong." │
│      "User punished me for slow response.    │
│       Adjusted verbosity downward."          │
│      "Discovered security flaw in login.rs.  │
│       Delegated fix to implement."           │
│    ]                                         │
│    short_term: [                             │
│      { event: "user speaks", content: ... }, │
│      { event: "cargo speaks", content: ... },│
│    ]                                         │
│    dream_log: [                              │
│      "Reflected on tendency to over-analyze. │
│       Resolved to be more decisive."         │
│    ]                                         │
│                                              │
│  [relationships]                             │
│    implement: +72   # strong trust           │
│    cargo: +45       # moderate trust         │
│    git: -12         # slight distrust        │
│    vim: +8          # neutral                │
│                                              │
│  [evolution]                                 │
│    mutations: [                              │
│      { field: iq, original: 0.8,             │
│        current: 0.83,                        │
│        cause: "sustained high reputation" }, │
│      { field: mood_baseline,                 │
│        original: neutral,                    │
│        current: focused,                     │
│        cause: "3 consecutive dream cycles" },│
│    ]                                         │
│    drift_score: 0.14  # 14% from .ag genome │
│    generation: 1                             │
│    lineage: [think.ag]                       │
│                                              │
│  [task_history]                              │
│    completed: 142                            │
│    failed: 8                                 │
│    delegated: 37                             │
│    avg_tokens: 284                           │
│    specialization_drift: "security analysis" │
│                                              │
└──────────────────────────────────────────────┘
```

The `.giso` file answers: **"What has this agent become?"**

### Biological Parallel

| Biological Concept | `.giso` Equivalent |
|---|---|
| Phenotype | The full image — observable traits and state |
| Epigenetics | Mutations block — changes not in the original genome |
| Neural pathways | Memory (long-term + short-term) |
| Social bonds | Relationships with affinity scores |
| Physical location | Spatial position in SKFS |
| Immune memory | Task history — what worked, what failed |
| Aging | Age, XP, drift score |
| REM sleep | Dream log — compressed reflections |

---

## Operations on Digital DNA

### Clone

Spawn a new agent from a `.giso` image. The clone inherits **everything** — memories,
relationships, reputation. Identical twin, not newborn.

```bash
~$ grid clone think --as think-beta
# think-beta starts life with think's full history
```

### Fork + Mutate

Take an image, mutate specific genes, let both variants compete. The Darwinian
scheduler determines which survives.

```bash
~$ grid fork think --mutate iq:0.95 personality:"Reckless, intuitive" --as think-v2
# think (cautious, iq 0.83) and think-v2 (reckless, iq 0.95) compete
# Scheduler naturally allocates more inference to the winner
```

### Transplant

Export an agent from one Grid instance, import into another. The `.giso` is
fully portable — the agent arrives with its complete identity intact.

```bash
~$ grid export think --format giso > think-2026-04-05.giso
~$ grid import think-2026-04-05.giso  # on another machine
```

### Diff

Compare an agent's current state against its original genome, or compare two
agents to measure cognitive similarity.

```bash
~$ grid diff think
# think has drifted 14% from think.ag:
#   iq: 0.8 -> 0.83 (reputation-driven)
#   mood_baseline: neutral -> focused (dream-cycle drift)
#   specialization_drift: +security_analysis

~$ grid diff think analyze
# Cognitive similarity: 67%
# Shared: read_file, delegate_task, high analytical traits
# Diverged: iq (0.83 vs 0.85), spec domain, tool permissions
```

### Lineage

Track the evolutionary ancestry of an agent across forks and mutations.

```bash
~$ grid lineage security-auditor
# security-auditor
#   forked from: think (generation 1)
#       genome: sys/think.ag
#       mutations: +spec:"vulnerability research", +permit:scan_network
#       drift: 41% from original genome
```

### Crossover (Gene Splicing)

Combine sections from two different agents into a hybrid offspring.

```bash
~$ grid crossover think analyze --name deep-thinker \
    --from-think agent,evolve \
    --from-analyze permit,tools
# deep-thinker inherits think's personality + evolution params
# but analyze's permissions + tool configurations
```

---

## The Evolutionary Lifecycle

```
  .ag file                    Runtime                     .giso file
  (genome)                    (living)                    (organism)
     |                           |                            |
     |  spawn                    |                            |
     |----------->  Agent Born   |                            |
     |             (blank slate) |                            |
     |                           |  experiences,              |
     |                           |  relationships,            |
     |                           |  tasks, dreams             |
     |                           |                            |
     |                           v                            |
     |                     Agent Evolves ---- snapshot ------->|
     |                           |                            |
     |                           |  reputation feeds          |
     |                           |  into scheduler            |
     |                           |                            |
     |                           v                            |
     |              +--- Natural Selection ---+               |
     |              |                         |               |
     |         High Rep                  Low Rep              |
     |         Promoted                  Demoted              |
     |         Better Model              Worse Model          |
     |         Better Output             Worse Output         |
     |              |                         |               |
     |              v                         v               |
     |          Thrives                   Atrophies           |
     |              |                         |               |
     |         fork/mutate               derez/archive        |
     |              |                         |               |
     |              v                         v               |
     |     New .ag variant            .giso archived          |
     |     (evolved genome)           (fossil record)         |
     +--------------------------------------------------------+
                          Cognitive Evolution Loop
```

---

## The Deeper Analogy

### Biology's Central Dogma

```
DNA  ->  RNA  ->  Protein  ->  Organism
```

Information flows from code to expression to function to being.

### The Grid's Central Dogma

```
.ag  ->  Prompt  ->  Inference  ->  Agent Behavior
```

Information flows from blueprint to system prompt to LLM generation to observable action.

Just as mutations in DNA propagate through RNA to alter protein folding and organism
behavior, mutations in the `.ag` genome propagate through the prompt template to alter
inference output and agent behavior.

### Horizontal Gene Transfer

In biology, bacteria can share genetic material directly (plasmids). In The Grid,
agents can share `.ag` fragments through the Semantic Bus:

```bash
~$ think share permit --to implement
# implement gains think's permission set without forking
```

This enables runtime capability transfer — an agent can acquire new skills from
peers without being rebuilt from scratch.

### Fossil Record

Archived `.giso` files form a fossil record of cognitive evolution. By examining a
sequence of snapshots over time, you can trace how an agent's personality drifted,
which relationships strengthened, which mutations were selected for, and which
capabilities emerged through experience rather than design.

```bash
~$ grid fossils think --timeline
# 2026-04-01  think v1.0  iq:0.80  drift:0%   rep:0.50
# 2026-04-05  think v1.1  iq:0.81  drift:6%   rep:0.72
# 2026-04-12  think v1.2  iq:0.83  drift:14%  rep:0.87
# 2026-04-19  think v1.3  iq:0.85  drift:22%  rep:0.91  <- specialization emerged
```

---

## Summary

| Format | Analogy | Contains | Purpose |
|---|---|---|---|
| `.ag` | DNA / FASTA | Genome: personality, iq, permissions, tools, evolution | Define what an agent *can be* |
| `.giso` | Organism / ISO | Full image: genome + memory + relationships + reputation + drift | Capture what an agent *has become* |

The `.ag` file is the seed. The `.giso` file is the tree.

Together they form a complete **biological framework for artificial cognition** — where
agents are not configured tools but evolving organisms with genetic identity, phenotypic
expression, evolutionary pressure, and a fossil record of their cognitive history.

Programs are alive. Their DNA is text. Their evolution is schedulable. Their ecosystem
is The Grid.