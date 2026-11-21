//! rg52-info: informacoes do sistema na tela do aparelho (substitui o `System Info.sh`).
pub mod gfx;
pub mod sysinfo;
pub mod ui;

pub use sysinfo::{Battery, Disk, Info, Sys};
