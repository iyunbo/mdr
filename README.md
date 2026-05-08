# mdr — markdown reader

A terminal markdown reader with vi-style keybindings.

## Features

- Browse a directory tree, preview markdown files in a side panel
- Pretty rendering: headings with underlines, styled inline code, aligned tables with borders, italics/bold
- Vi-style keys: `j`/`k`, count prefixes (`5j`, `12k`), `/` and `?` search, `n`/`N` repeat
- Configurable theme and key bindings via `~/.config/mdr/config.toml`
- Async file loading with `tokio`

## Install

```sh
cargo install --path .
```

Or download a prebuilt binary from the [Releases](https://github.com/iyunbo/mdr/releases) page.

## Usage

```sh
mdr                  # browse the current directory
mdr path/to/dir      # browse a directory
mdr file.md          # open a file directly
```

## Keys

| Key                | Action                                        |
|--------------------|-----------------------------------------------|
| `j` / `Down`       | move down (accepts count, e.g. `5j`)          |
| `k` / `Up`         | move up                                       |
| `g`                | jump to first line                            |
| `Ng`               | jump to line N (e.g. `42g`)                   |
| `G`                | jump to last line                             |
| `Ctrl+d` / `Ctrl+u` | half-page down / up                          |
| `Enter` / `l` / `Right` | activate (open file or expand directory) |
| `Esc` / `h` / `Left`    | back / collapse directory                |
| `/` / `?`          | search forward / backward                     |
| `n` / `N`          | repeat last search same / opposite direction  |
| `q`, `Ctrl+C`      | quit                                          |

## Config

`~/.config/mdr/config.toml` — partial overrides are merged with defaults.

```toml
[theme]
heading_color = "blue"
code_color = "green"

[keys]
quit = "Q"               # string form
down = ["j", "Down"]     # array form (multiple bindings)
```

## License

Licensed under either of [MIT](https://opensource.org/licenses/MIT) or [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0) at your option.
