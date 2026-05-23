# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-05-21

### Added

- Initial release: keyboard-driven terminal HTTP client (TUI).
- Request tree with folders, persisted to `lazypost-workspace.json`.
- Request editor with Info, Auth, Body, Params, URL Vars, Headers, and Capture tabs.
- Auth types: None, Bearer, Basic, and API Key (header or query param).
- Body types: None, Raw, JSON, Form, and Multipart.
- Environment variables with `{{var}}` substitution and a dedicated editor.
- URL variables via `<key>` tokens in the URL.
- Capture templates that extract response values into environment variables.
- Request history (last 500 entries) with restore, delete, and clear.

[Unreleased]: https://github.com/AaronAmelink/lazypost/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/AaronAmelink/lazypost/releases/tag/v0.1.0
