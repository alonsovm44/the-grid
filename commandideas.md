```bash
grid reward <program> <program2> ... -int=<intensity> -d=<duration> #gratification from 0=nothing to 1=bliss, duration in seconds
grid punish <program> <program2> ... -int=<intensity> -d=<duration> #pain from 0=nothing to 1=torture # duration in seconds
grid status [DONE] # shows a dashboard of the grid, active projects, invoked programs, active programs, their stats, tasks assigned, etc
grid gag <program> -d=<duration> # mutes a program and makes it unable to speak, duration in seconds
grid start-adversarial-network <program> <program2> ... -vs <program3> <program4> ... arena=[disk-wars | light-cycles | -melee | -tanks] --death-match 
#programs fight each other in realtime based games, in each turn a program uses the AI to calculate its next action. 
# -vs flag is optional, if not passed it is free for all

# disk wars: 

# light cicles: each player gets a lightcycle of its color/team color. each turn the AI gets the state of the light cycle grid (127x127 pixels) and returns a value that updates its position. If the AI crashes into a wall it loses.

# tanks: allowed moves per turn:
     #-move turret NSEW
     # move one space NSEW
     # shoot
grid pop <path> --list #  scans how many programs live in that directory and subdirectory, optional --list flag gives a list of programs

grid give-title <program> <title> # gives a program a title or alias e.g: grid give-title git "baron"

grid revoke-title <program <title> # revokes a title from a program

grid <program> task ["task" |  --spec=<pathtospec>] [DONE] # assigns a specific task to a program, either a textual description or a path to a spec file

grid run bin/LLLaserControl -ok 1 --target="Userfile@username" # Digitizes an external user into The Grid from a profile URL.
# IDEAS:
# - Fetches a GitHub or social media profile URL using `reqwest`.
# - Uses an LLM to analyze the profile's bio, top languages, repos, and commit message tone.
# - Generates a custom personality prompt and stats (IQ, formality, mood) based on the profile data.
# - Spawns a new autonomous agent named after the profile handle (e.g., "torvalds") that acts as a digital clone interacting with the other programs.
#proposed workflow
    # fetch url with user data,
    # feed it to an LLM
    # ask LLM t make Userfile@username blueprint
    # parse Userfile and produce binary 

grid run bin/LLLaserControl -ok 1 --dna="<path_to_fasta_or_id>" # Synthesizes a program directly from raw biological DNA!
# IDEAS (DNA -> .exe):
# - Parses a local `.fasta` file or fetches from a public genome database (like NCBI).
# - Calculates biological metrics: GC-content dictates the program's emotional stability/formality (high GC = rigid, low GC = volatile).
# - Sequence length dictates IQ and memory allocation.
# - Feeds these biological stats to the LLM to generate a unique, biologically-seeded digital entity.
# - Result: A living `.exe` whose behavior is fundamentally dictated by real-world genetic code.

## Userfile Template
```

```yaml


```