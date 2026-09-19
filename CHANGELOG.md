# Changelog

## [1.1.3](https://github.com/jrjohn/arcana-cloud-rust/compare/v1.1.2...v1.1.3) (2026-09-19)


### Bug Fixes

* **deps:** update rust crate futures to v0.3.34 ([#107](https://github.com/jrjohn/arcana-cloud-rust/issues/107)) ([0866124](https://github.com/jrjohn/arcana-cloud-rust/commit/0866124ed793718a23ceeb749f0e66859c2cec6e))

## [1.1.2](https://github.com/jrjohn/arcana-cloud-rust/compare/v1.1.1...v1.1.2) (2026-09-18)


### Bug Fixes

* **deps:** update rust crate deadpool-redis to v0.23.1 ([#105](https://github.com/jrjohn/arcana-cloud-rust/issues/105)) ([b32bde4](https://github.com/jrjohn/arcana-cloud-rust/commit/b32bde4244f407671f694d650ae9b091f7f57f9f))
* **deps:** update rust crate wasmtime to v48 ([#101](https://github.com/jrjohn/arcana-cloud-rust/issues/101)) ([d035b4b](https://github.com/jrjohn/arcana-cloud-rust/commit/d035b4b3530c14f1b1e4babe13bcf5e6a2a67bdd))

## [1.1.1](https://github.com/jrjohn/arcana-cloud-rust/compare/v1.1.0...v1.1.1) (2026-09-16)


### Bug Fixes

* **deps:** update rust crate wasmtime-wasi to v48 ([#102](https://github.com/jrjohn/arcana-cloud-rust/issues/102)) ([dd5bc21](https://github.com/jrjohn/arcana-cloud-rust/commit/dd5bc21ad865d972a1815a46b3090ae91beb567a))

## [1.1.0](https://github.com/jrjohn/arcana-cloud-rust/compare/v1.0.1...v1.1.0) (2026-09-03)


### Features

* **deploy:** 把 pod 真的切開 —— worker/scheduler 成為獨立角色 ([#99](https://github.com/jrjohn/arcana-cloud-rust/issues/99)) ([8fed441](https://github.com/jrjohn/arcana-cloud-rust/commit/8fed441fe9d2d133c7ab6f5d9f024de6547c28b1))

## [1.0.1](https://github.com/jrjohn/arcana-cloud-rust/compare/v1.0.0...v1.0.1) (2026-08-03)


### Bug Fixes

* **ci:** derive :1.0.0 from build-N image instead of exporting static tag ([3923b15](https://github.com/jrjohn/arcana-cloud-rust/commit/3923b157252364a93bd147783f14e3c166c1207e))
* **ci:** harden Coverage (llvm-cov) gate, drop DinD bind-mount + swallow ([e12589f](https://github.com/jrjohn/arcana-cloud-rust/commit/e12589f2ac40f6ca7f62166acf965b2250ed0f3d))
* **deps:** update rust crate reqwest to 0.13 ([#56](https://github.com/jrjohn/arcana-cloud-rust/issues/56)) ([925d609](https://github.com/jrjohn/arcana-cloud-rust/commit/925d6095c0670b96f74404cb47784de8cef0bf0d))
