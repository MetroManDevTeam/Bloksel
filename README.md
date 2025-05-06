# Bloksel

A voxel-based game engine written in Rust, featuring a block-based world with advanced terrain generation, physics, and rendering capabilities.

## Features

- Block-based world system with support for sub-blocks
- Advanced terrain generation with biomes and caves
- Efficient chunk-based world management
- Physics system for player and block interactions
- Modern OpenGL-based rendering pipeline
- Configurable world generation parameters

## Requirements

- Rust 1.70 or higher
- OpenGL 4.3 or higher
- CMake 3.10 or higher (for some dependencies)

## Building

```bash
# Clone the repository
git clone https://github.com/MetroManSR/Bloksel.git
cd Bloksel

# Build the project
cargo build --release

# Run the game
cargo run --release
```

## Project Structure

- `src/world/` - Core world and block management
- `src/render/` - Rendering pipeline and graphics
- `src/config/` - Configuration and settings
- `src/player/` - Player physics and controls
- `src/assets/` - Asset management and loading
- `src/ui/` - User interface components

