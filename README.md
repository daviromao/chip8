# Chip-8 Emulator

## Introduction
This project is a Chip-8 emulator written in Rust, utilizing the SDL2 library for rendering and handling input. Chip-8 is a simple, interpreted programming language developed in the 1970s for creating games on a variety of hardware. This emulator replicates the Chip-8 environment, allowing you to run and interact with Chip-8 programs.

## Table of Contents
- [Introduction](#introduction)
- [Table of Contents](#table-of-contents)
- [Installation](#installation)
- [Usage](#usage)
- [Features](#features)
- [Dependencies](#dependencies)
- [Configuration](#configuration)
- [Documentation](#documentation)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)
- [Contributors](#contributors)
- [License](#license)

## Installation
1. **Prerequisites**: 
   Ensure you have Rust and Cargo installed. Follow the instructions [here](https://www.rust-lang.org/tools/install) if you need to install them. Also ensure you have SDL2 installed on your system.
   
2. **Clone the repository**:
    ```sh
    git clone <repository-url>
    cd chip-8-emulator
    ```
3. **Install dependencies**:
    ```sh
    cargo build
    ```

## Usage
1. **Running the emulator**:
    ```sh
    cargo run --release -- path/to/rom.ch8
    ```
   Replace `path/to/rom.ch8` with the path to your Chip-8 ROM file.

## Features
- Emulates the Chip-8 CPU and memory
- Supports Chip-8 timers and input
- Renders graphics using SDL2
- Loads fonts and ROMs
- Supports key input for Chip-8 programs
- Dont support sound yet

## Dependencies
- **Rust**: Programming language used for developing the emulator.
- **SDL2**: Simple DirectMedia Layer used for rendering and input handling.

## Configuration
You can configure various aspects of the emulator such as the display scale by modifying the constants in the source code.

## Documentation
- **Chip-8 Specification**: Refer to [this documentation](http://devernay.free.fr/hacks/chip8/C8TECH10.HTM) for the Chip-8 technical reference.
- **SDL2 Documentation**: Refer to the [official SDL2 documentation](https://wiki.libsdl.org/FrontPage) for more information on using SDL2 with Rust.
- **Guide**: For a detailed guide on how to build a Chip-8 emulator, refer to [this tutorial](https://tobiasvl.github.io/blog/write-a-chip-8-emulator/).

## Examples
Include examples of Chip-8 programs that can be run with this emulator. For instance:
```sh
cargo run --release -- examples/IBMLogo.ch8
```

## Troubleshooting
If you encounter issues, consider the following steps:
- Ensure all dependencies are correctly installed.
- Verify that the path to the ROM file is correct.
- Check the console output for any error messages.

## Authors
- Davi Romão
    - Email: dsr@ic.ufal.br
    - Computer Science Undergrad Student

## License
This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
