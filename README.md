# Clear Test Interface

This repository contains integration and verification tests for the `clear` desktop environment programs, specifically:
- `clear-computing-environment-server` (Wayland compositor with background blur support)
- `ccec` (window manager / layout agent)
- `clear-ui` (WebGPU-based desktop widget system)

The tests run in a **headless Wayland session** using the `headless` backend of `wlroots` to ensure portability and automated execution.

## Features tested
1. **Window Opacity**: Verifies that client windows (such as `clear-ui`) render with expected transparency/alpha settings.
2. **Background Blur**: Confirms the compositor's background blur is successfully initialized and enabled for mapped window surface trees.
3. **Window Layout and Tiling**: Verifies that IPC commands sent via `clearctl` dynamically update tiling layouts (Cascade, Grid, Vsplit, etc.) and window geometries as expected.

## Requirements
- Python 3.x
- `Pillow` (for screenshot color analysis)
- `grim` (for Wayland screenshot capture)
- `clear-computing-environment-server` and `ccec` built/installed

## Running the Suite

You can execute the whole test suite using the orchestration script:
```bash
python3 run_suite.py
```

Or run individual tests with python unit tests or pytest if desired:
```bash
python3 -m unittest discover tests
```
