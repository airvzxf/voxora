// SPDX-License-Identifier: Apache-2.0
//
// Backend dispatch placeholder. Engine-specific helpers live in
// submodules that are each compile-gated by their own feature flag;
// the CLI's `select()` / `run()` paths live in `crate::engine` and
// dispatch into the appropriate submodule when the feature is enabled.
//
// We keep this file as a single named module so future shared helpers
// (e.g. a streaming ASR server) have an obvious home without having
// to churn the file layout.
