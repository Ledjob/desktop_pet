# Desktop Pet

A cute animated desktop parrot for Windows, written in Rust. The parrot walks, idles, flies, and displays speech bubbles with messages. Inspired by desktop pets and Japanese learning tools.

## Features

- Animated parrot with walking, idle, and flying states
- Drag and drop the parrot anywhere on your desktop
- Speech bubble with customizable messages (supports Japanese and English)
- Custom font rendering for Japanese text
- Physics-based movement and random behaviors
- Easily customizable images and messages

## Installation

### Prerequisites

- Windows 10 or later
- [Rust toolchain](https://www.rust-lang.org/tools/install)
- [Fontdue](https://crates.io/crates/fontdue) and [image](https://crates.io/crates/image) crates (handled by Cargo)
- Japanese font file: `NotoSansCJKjp-Regular.otf` (see below)

### Steps

1. Clone this repository:
   ```sh
   git clone <repo-url>
   cd parrot-pet
   ```
2. Download and install the font `NotoSansCJKjp-Regular.otf` from [Google Fonts](https://fonts.google.com/noto/specimen/Noto+Sans+JP) or your preferred source.
   - Place the font at: `C:/Users/<YourUsername>/AppData/Local/Microsoft/Windows/Fonts/NotoSansCJKjp-Regular.otf`
   - Or update the path in `src/main.rs` to match your font location.
3. Build and run:
   ```sh
   cargo run --release
   ```

## Usage

- The parrot will appear on your desktop.
- **Left-click and drag** to move the parrot.
- **Release left-click** to make the parrot fly.
- **Right-click** the parrot to show/hide a speech bubble with a random message.
- Messages are loaded from `messages.txt` (one message per line).

## Customization

- **Images:** Replace the PNG files in the `assets/` folder to change the parrot or bubble appearance.
- **Messages:** Edit `messages.txt` to add or change the messages. Supports Japanese and English.
- **Font:** Change the font file path in `src/main.rs` if you want to use a different font.
- **Animation/Physics:** Tweak parameters in `src/main.rs` for speed, gravity, animation timing, etc.
- **Variables:** Replace important variables like ALWAYS_ON_TOP, BUBBLE_SCALE in `src/utils.rs`

## Dependencies

- [windows](https://crates.io/crates/windows)
- [image](https://crates.io/crates/image)
- [fontdue](https://crates.io/crates/fontdue)

## License

MIT
