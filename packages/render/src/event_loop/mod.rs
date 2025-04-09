mod builder;
mod event_loop;

pub use builder::Builder;
pub use event_loop::EventLoop;

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub(crate) enum Size {
    Fixed(u16, u16),
    FullScreen,

    // unimplemented, bc panel have fixed size
    Auto,
}

// unimplemented
// this is useful, when want to use Render with other things
// like put a clock on the right top corner in shell
#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub(crate) enum Position {
    LeftTop,
    RightTop,
    LeftBottom,
    RightBottom,
    Center,
}
