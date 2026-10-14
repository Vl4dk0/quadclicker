mod navigation;
mod routing;

#[cfg(target_os = "macos")]
#[allow(unsafe_code)] // The native boundary owns and audits all AppKit/Core Graphics FFI.
mod macos;

fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    return macos::run();

    #[cfg(not(target_os = "macos"))]
    anyhow::bail!("QuadClicker requires macOS")
}
