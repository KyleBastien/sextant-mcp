# Changelog

## [0.1.13](https://github.com/KyleBastien/sextant-mcp/compare/v0.1.12...v0.1.13) (2026-05-14)


### Features

* remove file-exclusion knobs from sextant config ([#40](https://github.com/KyleBastien/sextant-mcp/issues/40)) ([f478ce0](https://github.com/KyleBastien/sextant-mcp/commit/f478ce05b190ecc568cfdb9c9c922a256ad8c310))

## [0.1.12](https://github.com/KyleBastien/sextant-mcp/compare/v0.1.11...v0.1.12) (2026-05-13)


### Bug Fixes

* **action:** drop expression contexts from composite-action input fields ([#38](https://github.com/KyleBastien/sextant-mcp/issues/38)) ([477fc14](https://github.com/KyleBastien/sextant-mcp/commit/477fc14ed0fdc07f0e1424a7a3ff517907ed18f2))

## [0.1.11](https://github.com/KyleBastien/sextant-mcp/compare/v0.1.10...v0.1.11) (2026-05-11)


### Bug Fixes

* drop retired macos-13 + use sha256sum on Windows ([#36](https://github.com/KyleBastien/sextant-mcp/issues/36)) ([bae2fa2](https://github.com/KyleBastien/sextant-mcp/commit/bae2fa20245185aabc640df962ee0d1a26f7c349))

## [0.1.10](https://github.com/KyleBastien/sextant-mcp/compare/v0.1.9...v0.1.10) (2026-05-11)


### Bug Fixes

* dispatch release.yml from auto-backfill so binaries actually build ([#34](https://github.com/KyleBastien/sextant-mcp/issues/34)) ([909e936](https://github.com/KyleBastien/sextant-mcp/commit/909e936c076b03054e4a70ffb4d296370fee1239))

## [0.1.9](https://github.com/KyleBastien/sextant-mcp/compare/v0.1.8...v0.1.9) (2026-05-11)


### Features

* auto-backfill missing root release after every main push ([#31](https://github.com/KyleBastien/sextant-mcp/issues/31)) ([6486adf](https://github.com/KyleBastien/sextant-mcp/commit/6486adf078b50fe4407c9d964d7d51af9669144b))

## [0.1.8](https://github.com/KyleBastien/sextant-mcp/compare/v0.1.7...v0.1.8) (2026-05-11)


### Bug Fixes

* drop package-name from root config so component matches empty body prefix ([#29](https://github.com/KyleBastien/sextant-mcp/issues/29)) ([ec10d58](https://github.com/KyleBastien/sextant-mcp/commit/ec10d58cc2a50f97a7109721324ec83bd8363df3))

## [0.1.7](https://github.com/KyleBastien/sextant-mcp/compare/v0.1.6...v0.1.7) (2026-05-11)


### Bug Fixes

* set empty component on root package so release tagging matches ([#27](https://github.com/KyleBastien/sextant-mcp/issues/27)) ([6a551aa](https://github.com/KyleBastien/sextant-mcp/commit/6a551aa643c1cc9d02c7d74438122a0eaa1796c0))

## [0.1.6](https://github.com/KyleBastien/sextant-mcp/compare/v0.1.5...v0.1.6) (2026-05-11)


### Bug Fixes

* drop ${scope} from title pattern to avoid duplicate capture group ([#25](https://github.com/KyleBastien/sextant-mcp/issues/25)) ([b70d999](https://github.com/KyleBastien/sextant-mcp/commit/b70d999af2e0fe03354f13e4fee016e8ee962594))
* pin release-please title pattern so root tagging stops failing ([#24](https://github.com/KyleBastien/sextant-mcp/issues/24)) ([2a503b7](https://github.com/KyleBastien/sextant-mcp/commit/2a503b722673a3b714fce3fb4132e3058262d22e))

## [0.1.5](https://github.com/KyleBastien/sextant-mcp/compare/v0.1.4...v0.1.5) (2026-05-10)


### Features

* 5 strict TS pack rules for loose-typing escape hatches ([#21](https://github.com/KyleBastien/sextant-mcp/issues/21)) ([98823e5](https://github.com/KyleBastien/sextant-mcp/commit/98823e575e3fd00d2455ed165b7a9a8ed4e8353d))
* propose-only autofix patches across all rule types ([#19](https://github.com/KyleBastien/sextant-mcp/issues/19)) ([9a66e71](https://github.com/KyleBastien/sextant-mcp/commit/9a66e71bb7f79e49ef12785213d8d6626a3c89e1))


### Bug Fixes

* roll release-please manifest back to unblock workflow ([#22](https://github.com/KyleBastien/sextant-mcp/issues/22)) ([d4505c9](https://github.com/KyleBastien/sextant-mcp/commit/d4505c97c9dd992b9f83565984e07a7762d79fe4))

## [0.1.5](https://github.com/KyleBastien/sextant-mcp/compare/v0.1.4...v0.1.5) (2026-05-10)


### Features

* propose-only autofix patches across all rule types ([#19](https://github.com/KyleBastien/sextant-mcp/issues/19)) ([9a66e71](https://github.com/KyleBastien/sextant-mcp/commit/9a66e71bb7f79e49ef12785213d8d6626a3c89e1))

## [0.1.4](https://github.com/KyleBastien/sextant-mcp/compare/v0.1.3...v0.1.4) (2026-05-10)


### Features

* rule packs + strict TypeScript pack ([#17](https://github.com/KyleBastien/sextant-mcp/issues/17)) ([029cd87](https://github.com/KyleBastien/sextant-mcp/commit/029cd87ea66fee6cfe79bfd7a0778483d576b0db))

## [0.1.3](https://github.com/KyleBastien/sextant-mcp/compare/v0.1.2...v0.1.3) (2026-05-10)


### Bug Fixes

* **vscode:** rename to sextant-mcp and drop AI-themed metadata ([#15](https://github.com/KyleBastien/sextant-mcp/issues/15)) ([98b4744](https://github.com/KyleBastien/sextant-mcp/commit/98b4744c2ad6218397f46b027863eb5fa04dfacb))

## [0.1.2](https://github.com/KyleBastien/sextant-mcp/compare/v0.1.1...v0.1.2) (2026-05-10)


### Bug Fixes

* ship MIT LICENSE files so vsce publish stops being flagged as suspicious ([#13](https://github.com/KyleBastien/sextant-mcp/issues/13)) ([1f0b0b1](https://github.com/KyleBastien/sextant-mcp/commit/1f0b0b10d3f32913e699ed21f66b30e4b39a29f8))

## [0.1.1](https://github.com/KyleBastien/sextant-mcp/compare/v0.1.0...v0.1.1) (2026-05-10)


### Features

* add sextant-lsp crate and VS Code extension for in-editor grading ([#8](https://github.com/KyleBastien/sextant-mcp/issues/8)) ([2a8a3e0](https://github.com/KyleBastien/sextant-mcp/commit/2a8a3e0e5e1815ec48680cdf7cf6a968ddb377c3))


### Bug Fixes

* drop workspace.package.version so release-please skips root manifest ([#10](https://github.com/KyleBastien/sextant-mcp/issues/10)) ([c276add](https://github.com/KyleBastien/sextant-mcp/commit/c276addb68fdd39f0e5f1c56f0706e908b5e1676))
* make workspace root a hybrid package so release-please can parse it ([#11](https://github.com/KyleBastien/sextant-mcp/issues/11)) ([97cd768](https://github.com/KyleBastien/sextant-mcp/commit/97cd768ebd7f0ae76d350b592f78667cde192ba8))
* pin literal versions on member crates so release-please can bump them ([#9](https://github.com/KyleBastien/sextant-mcp/issues/9)) ([96c7c1a](https://github.com/KyleBastien/sextant-mcp/commit/96c7c1a29d7841fae3048e3848c0b1a9be1829d2))
