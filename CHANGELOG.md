# Changelog

All notable changes to Rustblox are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [1.0.0] - 2026-11-15

First public release.

### Added

- Procedural voxel worlds in three sizes (128², 192², 256²; height 64) with
  value-noise terrain, beaches, lakes, snow peaks and trees.
- World list with seeds, save/load, delete with confirmation and recreate.
- Creative mode: flight, instant mining, infinite blocks, full block palette,
  middle-click block pick.
- Survival mode: 20 HP, fall and drowning damage, slow regeneration, death
  screen and respawn, apples from leaves.
- Hardness-based block breaking with progress bar, item drops.
- 36-slot inventory with hotbar, split/merge slot handling and stack cursor.
- Crafting: 2×2 hand grid, 3×3 crafting table, furnace smelting.
- Recipe book with per-station lists and one-click auto-fill of the craft grid.
- Ores: coal, iron, gold and diamond veins at different depths.
- Chunked meshes, water pass, alpha-tested glass pass, block outline.
- Menu, pause, settings (sensitivity, FOV, view distance, fullscreen),
  HUD with FPS, seed, position, block info, hearts and air.
- Procedural 16×16 texture atlas generated in code (no third-party assets).
- Save format `RBW3` with world name, player state, mode, health, inventory and
  RLE-compressed blocks; `RBW2` files still load.

[1.0.0]: https://github.com/barq3026-dot/rustblox/releases/tag/v1.0.0
