//! breadsearch/config.toml — semantic search indexer (breadmill) + GUI.
//! Schema mirrors breadsearch-shared::Config ([index], [search], [model], [power]).

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::Box as GBox;

use crate::config;
use crate::ui::widgets as w;

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("breadsearch/config.toml")
}

pub fn build() -> GBox {
    let path = config_path();
    let doc = Rc::new(RefCell::new(config::load_doc(&path)));

    let (outer, c) = w::view_scaffold("File Search");
    c.append(&w::service_control("breadmill.service", false, true));

    c.append(&w::section("Power"));
    c.append(&w::hint(
        "breadmill's embedding step is CPU/NPU/GPU-heavy. Turn it off entirely, \
         or just pause it on battery — it resumes automatically on AC power.",
    ));
    c.append(&w::switch_row("Enabled", &doc, &["power", "enabled"], true));
    c.append(&w::switch_row(
        "Index while on battery",
        &doc,
        &["power", "run_on_battery"],
        false,
    ));

    c.append(&w::section("Model"));
    // breadsearch v0.2.3+ publishes breadmill built with --features full
    // (npu + rocm + cuda + openvino all in one binary, ort load-dynamic/
    // dlopen — no separate build needed per backend). Selecting a GPU/NPU
    // backend here only takes effect if the matching ONNX Runtime is
    // actually present on this machine; breadmill logs which EP really
    // registered at startup.
    c.append(&w::dropdown_row(
        "Compute backend",
        &doc,
        &["model", "backend"],
        &["cpu", "npu", "rocm", "cuda", "openvino"],
        "cpu",
    ));
    c.append(&w::hint(
        "npu = AMD Ryzen AI (XDNA), rocm = AMD GPU (via MIGraphX), cuda = \
         NVIDIA GPU, openvino = Intel iGPU/Arc GPU. Each needs the matching \
         ONNX Runtime installed and visible to breadmill (system package, \
         or ORT_DYLIB_PATH) — check \
         `journalctl --user -u breadmill` after restarting for a \
         \"Successfully registered\" line confirming it actually took. \
         ROCm additionally needs ORT_MIGRAPHX_MODEL_CACHE_PATH set (bakery's \
         breadmill.service sets this by default) or the first query after \
         each backend/model change costs a one-time ~1-2 minute GPU compile.",
    ));

    c.append(&w::section("Index"));
    c.append(&w::csv_row(
        "Roots",
        &doc,
        &["index", "roots"],
        "~/Documents, ~/Projects",
    ));
    c.append(&w::csv_row(
        "Excludes",
        &doc,
        &["index", "excludes"],
        "~/Projects/some-noisy-repo",
    ));
    c.append(&w::csv_row(
        "Extensions",
        &doc,
        &["index", "extensions"],
        "md, txt, org, pdf, odt, docx",
    ));
    c.append(&w::spin_f64_row(
        "Max file size (MB)",
        &doc,
        &["index", "max_file_mb"],
        0.1,
        500.0,
        0.5,
        1,
        10.0,
    ));

    c.append(&w::section("Search"));
    c.append(&w::spin_row(
        "Result limit",
        &doc,
        &["search", "limit"],
        1.0,
        100.0,
        1.0,
        10,
    ));
    c.append(&w::spin_row(
        "Snippet length",
        &doc,
        &["search", "snippet_len"],
        20.0,
        2000.0,
        20.0,
        200,
    ));

    outer.append(&w::save_button(&doc, path));
    outer
}
