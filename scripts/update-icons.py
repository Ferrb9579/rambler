#!/usr/bin/env python3
"""Fetch pinned Material Symbols Rounded sources and render bundled PNGs.

Development-only requirement: CairoSVG 2.8.2 and the native Cairo library.
Run with uv run --with cairosvg==2.8.2 scripts/update-icons.py.
The app needs neither Python nor CairoSVG.
"""

from pathlib import Path
from urllib.request import urlopen

import cairosvg


REVISION = "10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7"
BASE_URL = f"https://raw.githubusercontent.com/google/material-design-icons/{REVISION}"
DESTINATION = Path(__file__).resolve().parents[1] / "assets/icons/rounded"
ICONS = (
    "drag_indicator", "content_copy", "check", "mic", "stop", "settings",
    "close", "hourglass_empty", "notes", "download", "info", "save",
    "keyboard", "palette", "arrow_back", "refresh",
)
PIXELS = 96


def download(url: str, destination: Path) -> None:
    with urlopen(url, timeout=30) as response:
        destination.write_bytes(response.read())


def main() -> None:
    DESTINATION.mkdir(parents=True, exist_ok=True)
    download(f"{BASE_URL}/LICENSE", DESTINATION / "LICENSE")
    for name in ICONS:
        source = DESTINATION / f"{name}.svg"
        download(f"{BASE_URL}/symbols/web/{name}/materialsymbolsrounded/{name}_24px.svg", source)
        cairosvg.svg2png(url=str(source), write_to=str(DESTINATION / f"{name}.png"),
                        output_width=PIXELS, output_height=PIXELS)
        print(name)


if __name__ == "__main__":
    main()
