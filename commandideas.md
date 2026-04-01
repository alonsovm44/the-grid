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


```

## Userfile Template

```yaml


```