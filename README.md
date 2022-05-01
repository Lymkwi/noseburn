# Noseburn

A visualization tool for a custom flavour of Brainfuck called Moostar.

## Installation

Using `cargo install .` in this repository will install `noseburn` to your local cargo directory.

## Usage

Simply call `noseburn` with the path to the moostar file you want to load.

Once the program is loaded, you can Reset (`r`) the simulation, move one step ahead (`s`), pause/start the simulation (`space`) or change the interpreter's frequency (`up`/`down`).

The simulator also represents a portion of the memory ribbon where your cursor currently is (depending on window size), so you can see the movement of data as the program unfolds.
