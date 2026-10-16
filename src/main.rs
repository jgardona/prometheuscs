// SPDX-License-Identifier: GPL-2.0-or-later
//! prometheuscs application entry point.

fn main() {
    #[cfg(feature = "gui")]
    prometheuscs::app::run();
    #[cfg(not(feature = "gui"))]
    eprintln!("prometheuscs was built without the `gui` feature; rebuild with --features gui");
}
