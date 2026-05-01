# Zed HTTP Client

⚠️ This project is under active development.

A Zed extension for `.http` files, inspired by the HTTP Client in IntelliJ IDEA.

## Features

- Syntax highlighting for `.http` and `.rest` files
- Gutter run button on each request block, which executes the request via a task and shows the response in the integrated terminal

## Installation

To use this extension, complete the following two steps. Due to current Zed extension API limitations, the runner binary and task definition cannot be shipped with the extension itself.

### 1. Install the `httpc` binary

Download the prebuilt binary for your platform from [Releases](https://github.com/hainet50b/http-client/releases) and place it on your `PATH`. `~/.local/bin/` is recommended (no `sudo` required); make sure the directory is on your `PATH`.

```bash
mkdir -p ~/.local/bin
tar -xzf httpc-x86_64-unknown-linux-gnu.tar.gz -C ~/.local/bin
```

### 2. Register the runnable task

Add the following to your global `~/.config/zed/tasks.json` (or per-project `.zed/tasks.json`):

```json
[
  {
    "label": "$ZED_CUSTOM_method $ZED_STEM | $ZED_CUSTOM_title",
    "command": "httpc",
    "args": ["--file", "$ZED_FILE", "--line", "$ZED_ROW"],
    "tags": ["http-request"],
    "reveal": "no_focus",
    "use_new_terminal": false,
    "allow_concurrent_runs": false
  }
]
```

Once both are in place, click the ▶ button next to any request to execute it.

## Related Projects

- [tie304/zed-http](https://github.com/tie304/zed-http) — alternative Zed extension that bridges to the [httpYac](https://httpyac.github.io) CLI.

## Acknowledgments

This extension uses the following third-party tree-sitter grammars:

- [`rest-nvim/tree-sitter-http`](https://github.com/rest-nvim/tree-sitter-http) for parsing `.http` files — MIT License, © 2021 NTBBloodbath.
- [`tree-sitter-grammars/tree-sitter-xml`](https://github.com/tree-sitter-grammars/tree-sitter-xml) for highlighting XML request bodies — MIT License, © 2023 ObserverOfTime.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
