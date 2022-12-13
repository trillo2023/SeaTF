# SeaTF
base for a start menu for linux + X that runs CLI Programs for e.g. a run-prompt or a settings menu.
The main program is basically a drop-down(/-up) terminal emulator.
It is written in rust using xcb with xkb to display the window.
To run it you need to enable feature "t":
'''sh
cargo run --features t
'''

## Features(planned/mostly functional)
* basic terminal emulator
* only black on white / white on black color schemes
* SGR commands set color mode to inverted
* no scrollback buffer
* dynamic window size corresponding to content
* lib crate for programming clients (think simple,very specific ncurses) (hence run feature t is not default)

## Roadmap
* use xft instead of basic X fonts
* get rid of most internal state and simply transcribe ANSI codes to X commands
* switch to two independant threads for listening for x events and listening to pty
  right now pty output is only read for 1s after a keyboard event.
* take arguments at launch to change settings e.g. run sea_tf -c dark inside sea_tf to change to dark mode*
* write clients
