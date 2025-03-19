# mnemnk-application

`mnemnk-application` is one of [Mnemnk](https://github.com/mnemnk/mnemnk-app/) agents, which reports your application usage to Mnemnk.

## Installation

1. Create a directory named `mnemnk-application` under `${mnemnk_dir}/agents/`. `${mnemnk_dir}` is the directory specified in the Mnemnk App settings, and the `agents` directory should already be automatically created.
2. Download the binary from the release page and place it under the newly created `mnemnk-application` directory. When doing so, remove the suffix like `-v0.5.0-macos-arm64` or `-v0.5.0-win-amd64` from the file name, and rename it to `mnemnk-application` for mac or `mnemnk-application.exe` for Windows.
3. Download `mnemnk.mac.json` or `mnemnk.win.json`, rename it to `mnemnk.json`, and place it in the same `mnemnk-application` directory.

After installation, restart Mnemnk and `Application` should be appear in Agents page.

## License

MIT
