# Rambler icons

All application icons use Google's **Material Symbols Rounded**, outlined,
weight 400, grade 0, optical size 24. The rounded terminals, consistent weight,
and shared 24-unit design grid keep toolbar and Settings icons visually aligned.

Source: [google/material-design-icons](https://github.com/google/material-design-icons)
at revision `10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7`.
The original SVGs and the upstream Apache 2.0 [license](rounded/LICENSE) are
bundled alongside their generated 96 × 96 transparent PNGs in `rounded/`.
PNGs are rasterized at four times the design size and tinted by Rambler for
light, dark, and accent surfaces. The app makes no network requests for icons.

To reproduce the assets with `uv` and the native Cairo library installed:

```bash
uv run --with cairosvg==2.8.2 scripts/update-icons.py
```

The mapping from actions to symbols is in `Icons::load` in `src/main.rs`.
