# mnemnk-application

`mnemnk-application` is one of [Mnemnk](https://github.com/mnemnk/mnemnk-app/) agents, which reports your application usage to Mnemnk.

## Installation

```shell
cargo install --git https://github.com/mnemnk/mnemnk-application
```

## Setup

`mnemnk-application` is enabled by default. After installation, restart Mnemnk and it should be running.

If it is not enabled, please edit Settings in Mnemnk as follows

```json
  "agents": {
    "application": {
      "enabled": true
    },
    ...
```

Save the settings and restart Mnemnk.

## Development

```shell
> cargo run

CONFIG {"interval":10,"ignore":["LockApp.exe"]}
STORE application {"t":1739435029568,"name":"Visual Studio Code","title":"README.md - mnemnk-application - Visual Studio Code","x":1136,"y":152,"width":2733,"height":1737,"text":"Visual Studio Code README.md - mnemnk-application - Visual Studio Code"}
...
```

## License

MIT
