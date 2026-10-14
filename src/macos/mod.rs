mod app;
mod display;
mod input;
mod overlay;
mod permissions;
mod scheduling;

pub fn run() -> anyhow::Result<()> {
    objc2::rc::autoreleasepool(|_| app::run())
}
