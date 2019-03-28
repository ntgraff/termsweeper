# termsweeper

terminal minesweeper

Works on Linux, Redox OS, and MacOS. _Should_ also work in any ANSI terminal.

(only tested on linux)

## usage

`$ cargo run -- <args>`

ex:

```
$ cargo run -- --help
minesweeper - little terminal minesweeper

flags:
    -w | --width N           ~ set the horizontal count of tiles
    -h | --height N          ~ set the vertical count of tiles
    -d | --difficulty [0, 2] ~ set the difficulty of the game

controls:
    space: reveal cell
    up/down/left/right: move cursor in direction
    f: flag cell
    q: quit
    r: restart
```
