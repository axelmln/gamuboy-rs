# gamuboy-rs
A Game Boy emulation core written in Rust

## Motivation

Learning about emulation and rust

## Usage

Add to your `Cargo.toml`:

```toml
gamuboy = { git = "https://github.com/your-username/gamuboy" }
```

### *Example*

```rust
use gamuboy::{
    config::Config,
    gameboy::GameBoy,
    lcd::{self},
    saver::FileSaver,
};

fn main() {
    let cfg = Config {
        rom: load_rom(), // load a rom file
        headless_mode: false,
        bootrom: load_bootrom(), // optionally load a bootrom (boot sequence is skipped if not provided)
        log_file_path: None,
    };

    let (event_tx, event_rx) = std::sync::mpsc::channel::<Event>(); // init an event channel to send joyoad events

    let mut gb = GameBoy::new(
        &cfg,
        Gui::new(), // inject your LCD implementation
        Stereo::new(), // inject your sound implementation
        EventsHandler::new(controller), // Inject your joypad event handler implementation, where your key bindings happen
        FileSaver::new(), // Inject your game saver implementation
        &event_rx, // inject event receiver
    );

    let my_event_poller = EventPoller::new(); // init your event poller

    loop {
        for event in event_pump.poll() {
            event_tx.send(event).unwrap(); // handle event polling as you need before sending it via the event channel
        }

        gb.step(); // advance the gameboy state
    }
}
```


## 🚧 Status

### Working
- CPU
- PPU
- APU
- Interrupts
- Joypad
- Timer
- MBC 1 and 2
- Game saves


### Test suite

#### Blargg's test roms [https://github.com/retrio/gb-test-roms]

**Every dmg** tests except:
- [ ] oam_bug
- [ ] halt_bug

#### Mooneye test suite [https://github.com/Gekkio/mooneye-test-suite]

Tested and working:
- [x] MBC1 tests
- [x] MBC2 tests


### Todo
- [ ] Support other MBCs
- [ ] Color support
- [ ] Serial

