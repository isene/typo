# typo

<img src="img/typo.svg" align="right" width="150">

**The terminal touch-typing tutor. Written in Rust.**

![Rust](https://img.shields.io/badge/language-Rust-f74c00) ![License](https://img.shields.io/badge/license-Unlicense-green) ![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS-blue) ![Stay Amazing](https://img.shields.io/badge/Stay-Amazing-important)

Strict touch-typing tutor for the terminal. Fixed lessons take you from the home row to full sentences, and a gradual mode hands out one new key at a time as you earn it. Built on [Crust](https://github.com/isene/crust), part of the [Fe2O3 suite](https://github.com/isene/fe2o3).

![Typo screenshot](img/screenshot.png)

## Features

- **Fixed lessons**: home row, top row, bottom row, capitals, numbers, symbols, sentences. The Norwegian set adds AltGr (`@ $ { [ ] } \ |`)
- **Gradual mode**: starts on four keys and unlocks the next one after three clean rounds. Drill words are built only from keys you have
- **Accuracy before speed**: your speed stays hidden until three rounds in a row clear 97%
- **Weak-key drills**: every keystroke is timed, and a drill can be built from your three slowest keys and two slowest pairs
- **Rhythm score**: one number for how even your keystrokes are, beside WPM. An even beat beats bursts and stalls
- **Eyes ahead**: the word after the cursor is emphasised, so you read ahead of your fingers
- **Daily test**: the same text every time, logged per layout, with a trend you can read
- **Two keyboard layouts**: US and Norwegian, with layout-specific drills (æ ø å, Norwegian shift pairings, AltGr)
- **Strict mode**: the drill only advances on the correct key; wrong keys count as errors and flash red
- **Zero idle cost**: fully event-driven, no timers, no polling. All timing is bookkeeping on the keypress already being handled
- **Single binary**: one dependency (crust), instant startup

## Install

Download the prebuilt binary from [Releases](https://github.com/isene/typo/releases), or build from source:

```bash
cargo build --release
cp target/release/typo ~/.local/bin/
```

## Key Bindings

| Key | Action |
|-----|--------|
| j/k, UP/DOWN | Select lesson |
| 1-9 | Jump straight into a lesson |
| ENTER | Start selected lesson |
| g | Gradual mode |
| w | Drill my weak keys |
| t | Daily test |
| T | Trend |
| l | Toggle keyboard layout (US / Norwegian) |
| q, ESC | Quit |

In a drill: type what you see. `⏎` means press ENTER. `ESC` returns to the menu. After a result, `r` retries.

## How to use it

Fifteen to twenty minutes a day beats one long session. Work for accuracy, not speed; slow deliberate keystrokes are the method, not a failure. Take the daily test at the same time each day so the trend line means something.

Expect to be slower than your old habit for two to three weeks. Then you pass it, usually within a month. That dip is why most people quit in week two.

## Layouts

The tutor checks the character you produce, so any keyboard works. The lessons themselves are layout-specific: the Norwegian set puts ø and æ on the home row, å with the top row, drills the Norwegian shift pairings (`s"`, `f¤`, `j/`, `ø=`), and has its own AltGr lesson. The layout choice persists across sessions.

## Files

`~/.typo` holds the chosen layout, your personal bests, unlocked keys, per-key and per-pair timings, and the daily-test log. Plain tab-separated text, one tagged record per line; delete a line to reset it. Test dates are UTC.

## License

Public domain (Unlicense). Created by [Geir Isene](https://isene.com).
