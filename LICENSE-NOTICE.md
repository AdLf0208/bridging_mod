# Licence provenance

- Mod: `bridgingmod` (Bridging Mod) by squeeglii — <https://github.com/squeeglii/BridgingMod>
- Licence: **MIT**, read from the jar the campaign audited
  (`licence-cache/BridgingMod-2.6.2+1.21.1.neoforge-release.jar`, its
  `META-INF/neoforge.mods.toml` and the repository's `LICENSE`, rank 0 permissive). The port reads
  the behaviour of upstream **2.7.0**, commit `fdd6cf6738767795876536014c0a9519317fb766`; the jar
  in hand is 2.6.2, the same target model.
- This port is a clean-room Rust rewrite: no Java file is copied or translated line by line. The
  upstream repository was read for behaviour only and its clone deleted after
  `../_shared/upstream/bridgingmod/FINDINGS.md` was written.
- The licence text in `LICENSE` is the MIT text with the campaign's copyright lines; upstream's own
  copyright is squeeglii's.
- Local-only until the maintainer approves publishing (ARCHITECTURE.md §8).
